use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn missing_ppe_reports_the_resolved_path_on_stderr() {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("ppld-missing-{}-{nonce}", std::process::id()));

    let output = Command::new(env!("CARGO_BIN_EXE_ppld")).arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stdout.contains("PPLD v"));
    assert!(stderr.contains("ERROR: Can't read"));
    assert!(stderr.contains(&format!("{}.ppe", path.display())));
    assert!(!stderr.contains("panicked"));
}
