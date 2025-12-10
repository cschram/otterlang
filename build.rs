use std::env;
use std::fs::copy;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Build the otterc_runtime static library and pull it into the target directory
    // so it's available with the otter binary. The otter compiler will use this static
    // lib when compiling Otter programs.
    let release = if let Ok(profile) = env::var("PROFILE") {
        profile == "release"
    } else {
        false
    };
    let runtime_dir = PathBuf::from("crates/otterc_runtime");
    let mut cmd = Command::new("cargo");
    cmd.arg("rustc");
    if release {
        cmd.arg("--release");
    }
    cmd.args(["--lib", "--crate-type=staticlib", "--target-dir", "target"])
        .current_dir(&runtime_dir)
        .status()
        .expect("Failed to build otterc_runtime");
    let target_dir = PathBuf::from(if release {
        "target/release"
    } else {
        "target/debug"
    });
    #[cfg(target_os = "windows")]
    let runtime_lib = "otterc_runtime.lib";
    #[cfg(not(target_os = "windows"))]
    let runtime_lib = "libotterc_runtime.a";
    copy(
        runtime_dir.join(&target_dir).join(runtime_lib),
        target_dir.join(runtime_lib),
    )
    .expect("Failed to copy otterc_runtime library");
}
