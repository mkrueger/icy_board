//! Records the commit the binary was built from. A release tarball has no
//! checkout, so an empty hash is a normal result, never a build failure.
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=ICBOARD_GIT_HASH");
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        let git_dir = std::path::Path::new(&git_dir);
        for path in ["HEAD", "refs/heads"] {
            println!("cargo:rerun-if-changed={}", git_dir.join(path).display());
        }
    }
    let hash = std::env::var("ICBOARD_GIT_HASH")
        .ok()
        .or_else(|| git(&["rev-parse", "--short", "HEAD"]))
        .unwrap_or_default();
    println!("cargo:rustc-env=ICBOARD_GIT_HASH={hash}");
}

fn git(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}
