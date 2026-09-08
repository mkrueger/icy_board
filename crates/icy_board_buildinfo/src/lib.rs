//! Shared build-script helper so every tool's `--version` reports the exact
//! commit it was built from. Called from each tool's `build.rs`.
use std::process::Command;

/// Emit `cargo:rustc-env=GIT_HASH=<short-hash-or-empty>` and rerun triggers.
/// A release tarball has no checkout, so an empty hash is a normal result,
/// never a build failure. Set the `GIT_HASH` environment variable to override
/// the detected commit (e.g. for reproducible packaging builds).
pub fn emit_git_hash() {
    println!("cargo:rerun-if-env-changed=GIT_HASH");
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        let git_dir = std::path::Path::new(&git_dir);
        for path in ["HEAD", "refs/heads"] {
            println!("cargo:rerun-if-changed={}", git_dir.join(path).display());
        }
    }
    let hash = std::env::var("GIT_HASH")
        .ok()
        .or_else(|| git(&["rev-parse", "--short", "HEAD"]))
        .unwrap_or_default();
    println!("cargo:rustc-env=GIT_HASH={hash}");
}

fn git(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
