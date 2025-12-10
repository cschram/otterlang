//! Target platform configuration for cross-compilation
//!
//! Supports multiple target architectures including native, WebAssembly, and embedded targets

use std::str::FromStr;

/// Target architecture/OS configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTriple {
    /// Architecture (e.g., x86_64, arm, wasm32)
    pub arch: String,
    /// Vendor (e.g., unknown, apple, pc)
    pub vendor: String,
    /// Operating system (e.g., linux, darwin, windows, none, wasi)
    pub os: String,
    /// OS version suffix (e.g., 19.6.0 for darwin)
    pub os_version: Option<String>,
    /// ABI/environment (e.g., gnu, msvc, eabi, elf)
    pub env: Option<String>,
}

impl TargetTriple {
    /// Create a new target triple
    pub fn new(
        arch: impl Into<String>,
        vendor: impl Into<String>,
        os: impl Into<String>,
        env: Option<impl Into<String>>,
    ) -> Self {
        Self {
            arch: arch.into(),
            vendor: vendor.into(),
            os: os.into(),
            os_version: None,
            env: env.map(|e| e.into()),
        }
    }

    /// Parse a target triple string (e.g., "x86_64-unknown-linux-gnu")
    pub fn parse(triple: &str) -> Result<Self, String> {
        let parts: Vec<&str> = triple.split('-').collect();

        if parts.len() < 3 {
            return Err(format!("Invalid target triple format: {}", triple));
        }

        let mut arch = parts[0].to_string();
        // Normalize arm64 to aarch64 for LLVM compatibility
        if arch == "arm64" {
            arch = "aarch64".to_string();
        }

        let vendor = parts[1].to_string();
        let raw_os = parts[2];

        // Separate OS base (alphabetic prefix) from version suffix (digits or dots)
        let mut split_index = raw_os.len();
        for (idx, ch) in raw_os.char_indices() {
            if ch.is_ascii_digit() || ch == '.' {
                split_index = idx;
                break;
            }
        }

        let (os_base, os_suffix) = raw_os.split_at(split_index);
        let os = if os_base.is_empty() {
            raw_os.to_string()
        } else {
            os_base.to_string()
        };

        let os_version = if !os_suffix.is_empty() {
            Some(os_suffix.to_string())
        } else {
            None
        };

        // Only include parts[3..] as env (e.g., "gnu", "musl", "eabi")
        // Do NOT inject version suffixes or darwin-specific strings
        let env = if parts.len() > 3 {
            let env_str = parts[3..].join("-");
            // Filter out empty or invalid env strings
            if env_str.is_empty() {
                None
            } else {
                Some(env_str)
            }
        } else {
            None
        };

        Ok(Self {
            arch,
            vendor,
            os,
            os_version,
            env,
        })
    }

    /// Convert to LLVM target triple string
    pub fn to_llvm_triple(&self) -> String {
        let os_part = if let Some(ver) = &self.os_version {
            format!("{}{}", self.os, ver)
        } else {
            self.os.clone()
        };

        match &self.env {
            Some(env) => format!("{}-{}-{}-{}", self.arch, self.vendor, os_part, env),
            None => format!("{}-{}-{}", self.arch, self.vendor, os_part),
        }
    }

    /// Check if this is a WebAssembly target
    pub fn is_wasm(&self) -> bool {
        self.arch == "wasm32" || self.arch == "wasm64"
    }

    /// Check if this is an embedded target (no OS)
    pub fn is_embedded(&self) -> bool {
        self.os == "none" || self.os == "elf"
    }

    /// Check if this is a Windows target
    pub fn is_windows(&self) -> bool {
        self.os == "windows"
    }

    /// Check if this is a Unix-like target
    pub fn is_unix(&self) -> bool {
        matches!(
            self.os.as_str(),
            "linux" | "darwin" | "freebsd" | "openbsd" | "netbsd"
        )
    }

    /// Get the appropriate C compiler for this target
    pub fn c_compiler(&self) -> String {
        if self.is_wasm() || self.is_windows() {
            // Prefer clang so we have a consistent driver that accepts Unix-style flags
            "clang".to_string()
        } else {
            "cc".to_string()
        }
    }

    /// Get the appropriate linker driver for this target
    pub fn linker(&self) -> String {
        if self.is_wasm() {
            "wasm-ld".to_string()
        } else if self.is_windows() {
            // Use clang as the linker driver so we can keep passing POSIX-style flags
            "clang".to_string()
        } else {
            "cc".to_string()
        }
    }

    /// Get linker flags for this target
    pub fn linker_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();

        if self.is_wasm() {
            flags.push("--no-entry".to_string());
            flags.push("--export-dynamic".to_string());
            if self.os == "wasi" {
                flags.push("--allow-undefined".to_string());
            }
        } else if self.is_windows() {
            // Pass subsystem settings through clang to the MSVC linker
            flags.push("-Wl,/SUBSYSTEM:CONSOLE".to_string());
        } else if self.is_embedded() {
            // Embedded targets typically use custom link scripts
            flags.push("-nostdlib".to_string());
        }

        if self.os == "darwin" {
            // sysinfo and other runtime bits rely on CoreFoundation + IOKit
            flags.push("-framework".to_string());
            flags.push("CoreFoundation".to_string());
            flags.push("-framework".to_string());
            flags.push("IOKit".to_string());
        }

        flags
    }

    /// Check if this target needs position-independent code
    pub fn needs_pic(&self) -> bool {
        self.is_wasm() || matches!(self.os.as_str(), "linux" | "freebsd" | "openbsd" | "netbsd")
    }
}

impl FromStr for TargetTriple {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Default for TargetTriple {
    fn default() -> Self {
        // Get native target from LLVM
        let llvm_triple = inkwell::targets::TargetMachine::get_default_triple();
        let triple_str = llvm_triple
            .as_str()
            .to_str()
            .unwrap_or("unknown-unknown-unknown")
            .to_string();

        // Normalize common macOS triples
        // Convert "arm64" to "aarch64" for LLVM compatibility
        // Force macOS 11.0 for compatibility
        if triple_str.starts_with("arm64-apple-darwin")
            || triple_str.starts_with("aarch64-apple-darwin")
        {
            Self::new("aarch64", "apple", "darwin11.0", None::<String>)
        } else if triple_str.starts_with("x86_64-apple-darwin") {
            Self::new("x86_64", "apple", "darwin11.0", None::<String>)
        } else {
            Self::parse(&triple_str)
                .unwrap_or_else(|_| Self::new("x86_64", "unknown", "linux", Some("gnu")))
        }
    }
}

/// Predefined target triples
impl TargetTriple {
    /// WebAssembly target (wasm32-unknown-unknown)
    pub fn wasm32_unknown_unknown() -> Self {
        Self::new("wasm32", "unknown", "unknown", None::<String>)
    }

    /// WebAssembly System Interface target (wasm32-wasi)
    pub fn wasm32_wasi() -> Self {
        Self::new("wasm32", "unknown", "wasi", None::<String>)
    }

    /// ARM Cortex-M0 target (thumbv6m-none-eabi)
    pub fn thumbv6m_none_eabi() -> Self {
        Self::new("thumbv6m", "none", "none", Some("eabi"))
    }

    /// ARM Cortex-M3 target (thumbv7m-none-eabi)
    pub fn thumbv7m_none_eabi() -> Self {
        Self::new("thumbv7m", "none", "none", Some("eabi"))
    }

    /// ARM Cortex-M4 target (thumbv7em-none-eabi)
    pub fn thumbv7em_none_eabi() -> Self {
        Self::new("thumbv7em", "none", "none", Some("eabi"))
    }

    /// ARM Cortex-A9 target (armv7-none-eabi)
    pub fn armv7_none_eabi() -> Self {
        Self::new("armv7", "none", "none", Some("eabi"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_triple() {
        let triple = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(triple.arch, "x86_64");
        assert_eq!(triple.vendor, "unknown");
        assert_eq!(triple.os, "linux");
        assert_eq!(triple.env, Some("gnu".to_string()));
    }

    #[test]
    fn test_wasm_triple() {
        let triple = TargetTriple::wasm32_unknown_unknown();
        assert!(triple.is_wasm());
        assert_eq!(triple.to_llvm_triple(), "wasm32-unknown-unknown");
    }

    #[test]
    fn test_embedded_triple() {
        let triple = TargetTriple::thumbv7m_none_eabi();
        assert!(triple.is_embedded());
    }
}
