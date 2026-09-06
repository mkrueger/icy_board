use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use icy_board_engine::executable::{Executable, FuncOpCode, OpCode, PPECommand, PPEScript};

fn ppld() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ppld"));
    command
        .env("LANG", "en_US.UTF-8")
        .env("LC_ALL", "en_US.UTF-8")
        .env("LANGUAGE", "en_US.UTF-8")
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("PPL_LANG_VERSION");
    command
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data").join(name)
}

struct TempPpe(PathBuf);

impl TempPpe {
    fn new(executable: &Executable) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!("ppld-cli-{}-{nonce}-{id}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let result = Self(directory.join("input.ppe"));
        fs::write(&result.0, executable.to_buffer().unwrap()).unwrap();
        result
    }
}

impl Drop for TempPpe {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.0.parent().unwrap());
    }
}

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

#[test]
fn check_is_informational_by_default_and_strict_rejects_findings() {
    for (name, finding) in [
        ("dointr.ppe", "STATEMENT DOINTR"),
        ("sound.ppe", "STATEMENT SOUND"),
        ("modem.ppe", "FUNCTION MODEM"),
        ("reg.ppe", "FUNCTION REGAX"),
    ] {
        let mut previous = None;
        for strict in [false, true, true] {
            let mut command = ppld();
            command.arg("--check");
            if strict {
                command.arg("--strict");
            }
            let output = command.arg(fixture(name)).output().unwrap();
            let text = String::from_utf8_lossy(&output.stdout);
            assert_eq!(output.status.code(), Some(i32::from(strict)), "{name}: {text}");
            assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
            assert!(text.starts_with("PPLD v") && text.contains("Compatibility Report"), "{text}");
            assert!(text.contains(finding) && text.contains("Summary:"), "{text}");
            assert!(!text.contains("ERROR"), "findings are not analysis errors: {text}");
            if let Some(previous) = previous {
                assert_eq!(output.stdout, previous, "strict only changes exit status; reports must be deterministic");
            }
            previous = Some(output.stdout);
        }
    }
}

#[test]
fn check_strict_accepts_clean_ppe() {
    for args in [vec!["--check"], vec!["--check", "--strict"]] {
        let output = ppld().args(args).arg(fixture("beep.ppe")).output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("No unsupported / unimplemented features detected."), "{text}");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn check_strict_rejects_partial_support_alone() {
    let mut executable = Executable::read_file(&fixture("dointr.ppe"), false).unwrap();
    let script = PPEScript::from_ppe_file(&executable).unwrap();
    let PPECommand::PredefinedCall(_, args) = &script.statements[0].command else {
        panic!("expected DOINTR")
    };
    executable.script_buffer.clear();
    PPECommand::PredefinedCall(OpCode::DLOCK.get_definition(), vec![args[0].clone()]).serialize(&mut executable.script_buffer);
    PPECommand::End.serialize(&mut executable.script_buffer);
    let temp = TempPpe::new(&executable);

    for strict in [false, true] {
        let mut command = ppld();
        command.arg("--check");
        if strict {
            command.arg("--strict");
        }
        let output = command.arg(&temp.0).output().unwrap();
        let text = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(i32::from(strict)), "{text}");
        assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
        for expected in ["Partially Implemented:", "STATEMENT DLOCK", "0 unimplemented", "0 unsupported", "1 partial"] {
            assert!(text.contains(expected), "{text}");
        }
    }
    assert!(!temp.0.with_extension("ppd").exists(), "checks must not write source files");
}

#[test]
fn check_analysis_errors_fail_in_both_modes_and_use_stderr() {
    let mut executable = Executable::read_file(&fixture("beep.ppe"), false).unwrap();
    executable.script_buffer = vec![OpCode::LET as i16];
    let temp = TempPpe::new(&executable);
    for args in [vec!["--check"], vec!["--check", "--strict"]] {
        let output = ppld().args(args).arg(&temp.0).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("ERROR during compatibility check: Failed to deserialize PPE"), "{stderr}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("ERROR"));
        assert!(!stderr.contains("panicked"));
    }
}

#[test]
fn output_stdout_is_exactly_source_even_when_decompilation_warns() {
    for warnings in [false, true] {
        let mut executable = Executable::read_file(&fixture(if warnings { "dointr.ppe" } else { "beep.ppe" }), false).unwrap();
        if warnings {
            // A legacy obfuscator's unary operator with no operand is recoverable:
            // the engine skips it and reports an issue at the DOINTR statement.
            assert_eq!(executable.script_buffer[0], OpCode::DOINTR as i16);
            executable.script_buffer.insert(1, FuncOpCode::UPLUS as i16);
        }
        let temp = TempPpe::new(&executable);
        let file_output = ppld().arg(&temp.0).output().unwrap();
        assert_eq!(
            file_output.status.code(),
            Some(i32::from(warnings)),
            "{}",
            String::from_utf8_lossy(&file_output.stderr)
        );
        let expected = format!("{}\n", fs::read_to_string(temp.0.with_extension("ppd")).unwrap());
        fs::remove_file(temp.0.with_extension("ppd")).unwrap();
        for flag in ["--output", "-o"] {
            let output = ppld().arg(flag).arg(&temp.0).output().unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(i32::from(warnings)), "{stderr}");
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                expected,
                "stdout must contain only the source, including source comments"
            );
            assert!(stderr.starts_with("PPLD v"), "{stderr}");
            assert_eq!(stderr.contains("WARNING:"), warnings, "{stderr}");
            assert_eq!(stderr.contains("1 issues found during decompilation"), warnings, "{stderr}");
            assert!(!temp.0.with_extension("ppd").exists());
        }
    }
}

#[test]
fn output_combined_with_check_moves_the_report_to_stderr() {
    for strict in [false, true] {
        let mut command = ppld();
        command.args(["--output", "--check"]);
        if strict {
            command.arg("--strict");
        }
        let output = command.arg(fixture("dointr.ppe")).output().unwrap();
        assert_eq!(output.status.code(), Some(i32::from(strict)));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.starts_with("PPLD v") && stderr.contains("STATEMENT DOINTR"), "{stderr}");
    }
}

#[test]
fn output_read_errors_leave_stdout_empty() {
    let executable = Executable::read_file(&fixture("beep.ppe"), false).unwrap();
    let temp = TempPpe::new(&executable);
    fs::remove_file(&temp.0).unwrap();
    for args in [vec!["--output"], vec!["--output", "--check"], vec!["--output", "--check", "--strict"]] {
        let output = ppld().args(args).arg(&temp.0).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.starts_with("PPLD v") && stderr.contains("ERROR: Can't read"), "{stderr}");
    }
}
