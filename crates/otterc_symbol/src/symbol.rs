//! A simple symbol type for use in the Otter compiler.

use core::fmt::{Display, Formatter, Result};

/// An encoded representation of an identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol(String);

impl Symbol {
    #[inline]
    pub fn is_builtin(&self) -> bool {
        matches!(
            self.0.as_str(),
            "u8" | "u16" | "u32" | "i32" | "i64" | "f32" | "f64" | "bool" | "str" | "unit"
        )
    }
}

impl Display for Symbol {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Symbol {
    #[inline]
    fn from(val: &str) -> Self {
        Self(val.to_owned())
    }
}

impl From<String> for Symbol {
    #[inline]
    fn from(val: String) -> Self {
        Self(val)
    }
}
