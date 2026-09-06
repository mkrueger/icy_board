//! Exercise the real headless worker, not an in-process substitute for its VM.
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::Executable,
    parser::{Encoding, ErrorReporter, UserTypeRegistry},
};

const ORIGINAL_NAME: &str = "original upload;one.ZIP";
const AD_NAME: &str = "board ad;generated.txt";
// DiskIO's existing text writer emits UTF-8 with a BOM on the first FPUT.
const AD_CONTENT: &[u8] = b"\xef\xbb\xbfgenerated for [original upload;one.ZIP]";
const GENERATOR: &str = r#"
STRING outdir, original
GETTOKEN outdir
original = GETTOKEN()
PRINT "This terminal output must be discarded."
FCREATE 1, outdir + "board ad;generated.txt", O_WR, S_DN
FPUT 1, "generated for [", original, "]"
FCLOSE 1
EXIT
"#;

fn check_compile_errors(errors: &Arc<Mutex<ErrorReporter>>) {
    let errors = errors.lock().unwrap();
    assert!(
        !errors.has_errors(),
        "PPS errors: {:?}",
        errors.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
    );
}

fn compile_ppe(directory: &Path, source: &str) -> PathBuf {
    let path = directory.join("generator with spaces;one.ppe");
    let registry = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let ast = icy_board_engine::parser::parse_ast(path.with_extension("pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    check_compile_errors(&errors);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    check_compile_errors(&errors);
    let executable = compiler.create_executable().unwrap();
    // Match the engine VM harness: serialization initializes the variable table.
    let mut bytes = executable.to_buffer().unwrap();
    let executable = Executable::from_buffer(&mut bytes, false).unwrap();
    fs::write(&path, executable.to_buffer().unwrap()).unwrap();
    path
}

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

fn captured(file: &mut File) -> Vec<u8> {
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    file.take(64 * 1024).read_to_end(&mut bytes).unwrap();
    bytes
}

fn bounded_output(command: &mut Command, timeout: Duration) -> Output {
    // Files rather than pipes: a verbose or broken child cannot block on output,
    // and reading diagnostics never waits for a descendant to close a pipe.
    let mut stdout = tempfile::tempfile().unwrap();
    let mut stderr = tempfile::tempfile().unwrap();
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout.try_clone().unwrap()))
        .stderr(Stdio::from(stderr.try_clone().unwrap()));
    let mut child = KillOnDrop(command.spawn().unwrap());
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            return Output {
                status,
                stdout: captured(&mut stdout),
                stderr: captured(&mut stderr),
            };
        }
        if Instant::now() >= deadline {
            let _ = child.0.kill();
            // Reap when possible, but even cleanup must never wait indefinitely.
            let cleanup_deadline = Instant::now() + Duration::from_secs(2);
            while matches!(child.0.try_wait(), Ok(None)) && Instant::now() < cleanup_deadline {
                thread::sleep(Duration::from_millis(10));
            }
            panic!(
                "subprocess exceeded {timeout:?}: {command:?}\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&captured(&mut stdout)),
                String::from_utf8_lossy(&captured(&mut stderr))
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {}; stdout: {}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn worker(root: &Path, ppe: &Path) -> (PathBuf, Output) {
    let output_dir = root.join("output directory;literal");
    let cwd = root.join("empty working directory");
    fs::create_dir(&output_dir).unwrap();
    fs::create_dir(&cwd).unwrap();
    let board_config = root.join("invalid board.toml");
    fs::write(&board_config, b"this is deliberately not valid TOML = [").unwrap();
    let output = bounded_output(
        Command::new(env!("CARGO_BIN_EXE_icboard"))
            .arg("--upload-advertisement-ppe")
            .arg(ppe)
            .arg(&output_dir)
            .arg(ORIGINAL_NAME)
            // Even an explicit, broken board configuration must be bypassed.
            .arg(&board_config)
            .current_dir(&cwd)
            .env_remove("ICB_PATH")
            .env("LANG", "en")
            .env("LC_ALL", "en")
            .env("LANGUAGE", "en")
            .env("TERM", "dumb"),
        Duration::from_secs(20),
    );
    assert!(output.stdout.is_empty(), "worker must not print PPE output or start the UI");
    assert!(!board_config.with_extension("log").exists(), "worker must bypass board logging setup");
    assert_eq!(fs::read_dir(&cwd).unwrap().count(), 0, "worker must not initialize a board in its cwd");
    (output_dir, output)
}

#[test]
fn worker_gettoken_and_file_io_preserve_literal_arguments() {
    let directory = tempfile::tempdir().unwrap();
    let ppe = compile_ppe(directory.path(), GENERATOR);
    let (output_dir, output) = worker(directory.path(), &ppe);
    assert_success(&output);
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read_dir(&output_dir).unwrap().count(), 1);
    // This also checks the trailing separator on GETTOKEN's output-directory
    // argument and that the generator's UTF-8 BOM is preserved without a newline.
    assert_eq!(fs::read(output_dir.join(AD_NAME)).unwrap(), AD_CONTENT);
}

#[test]
fn worker_print_is_permitted_but_discarded() {
    let directory = tempfile::tempdir().unwrap();
    let ppe = compile_ppe(directory.path(), "PRINT \"discarded headless output\"\nEXIT\n");
    let (output_dir, output) = worker(directory.path(), &ppe);
    assert_success(&output);
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read_dir(output_dir).unwrap().count(), 0);
}

#[test]
fn worker_can_succeed_without_generating_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let ppe = compile_ppe(directory.path(), "EXIT\n");
    let (output_dir, output) = worker(directory.path(), &ppe);
    assert_success(&output);
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read_dir(output_dir).unwrap().count(), 0);
}

#[test]
fn worker_stop_is_an_error() {
    let directory = tempfile::tempdir().unwrap();
    let ppe = compile_ppe(directory.path(), "STOP\n");
    let (output_dir, output) = worker(directory.path(), &ppe);
    assert!(output.status.code().is_some_and(|code| code != 0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("advertisement PPE aborted (STOP)"), "{stderr}");
    assert_eq!(fs::read_dir(output_dir).unwrap().count(), 0);
}

#[test]
fn documented_generator_creates_one_advertisement() {
    let directory = tempfile::tempdir().unwrap();
    let ppe = compile_ppe(directory.path(), include_str!("../../../assets/upload_advertisement.pps"));
    let (output_dir, output) = worker(directory.path(), &ppe);
    assert_success(&output);
    assert_eq!(fs::read_dir(&output_dir).unwrap().count(), 1);
    assert_eq!(
        fs::read(output_dir.join("BOARD.AD")).unwrap(),
        b"\xef\xbb\xbfAvailable from Example BBS\nArchive: original upload;one.ZIP\n"
    );
}

#[test]
fn worker_invalid_ppe_exits_nonzero() {
    let directory = tempfile::tempdir().unwrap();
    let ppe = directory.path().join("invalid.ppe");
    fs::write(&ppe, b"not a PPE executable").unwrap();
    let (output_dir, output) = worker(directory.path(), &ppe);
    assert!(output.status.code().is_some_and(|code| code != 0));
    assert!(!output.stderr.is_empty());
    assert_eq!(fs::read_dir(output_dir).unwrap().count(), 0);
}

// Only the scanner launcher needs Unix: it uses /bin/sh builtins and execs this
// test executable. No Python, grep, zip/unzip, real scanner, or new dependency.
#[cfg(unix)]
mod processor {
    use super::*;
    use dizbase::file_base_scanner::{
        bbstro_fingerprint::FingerprintData,
        repack::{RepackOptions, Repacked, repack_file},
    };
    use icy_board_engine::icy_board::{
        icb_config::{UploadProcessingConfig, UploadPublishPolicy, UploadScannerConfig},
        upload_processor::UploadProcessor,
        upload_quarantine::{QuarantineStatus, UploadQuarantine},
    };

    const DIZ: &[u8] = b"Original description\r\nPreserve spaces here:  \r\n\x1a";
    const PAYLOAD: &[u8] = b"original upload payload\r\n";
    const CHILD_ROOT: &str = "ICBOARD_AD_PROCESSOR_TEST_ROOT";
    const SCANNER_FILE: &str = "ICBOARD_AD_SCANNER_TEST_FILE";
    const SCANNER_MARKER: &str = "ICBOARD_AD_SCANNER_TEST_MARKER";
    const SCANNER_TEST: &str = "processor::scanner_checks_generated_archive";
    const PROCESSOR_TEST: &str = "processor::compiled_ppe_is_added_before_scanning_and_original_diz_is_preserved";

    fn put16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_input_zip(path: &Path) {
        // Minimal, unencrypted ZIP32 fixture with two stored members. icboard
        // has no direct zip dependency; CRC32 is already available in icy_net.
        let mut bytes = Vec::new();
        let mut central = Vec::new();
        for (name, content) in [("FILE_ID.DIZ", DIZ), ("PAYLOAD.TXT", PAYLOAD)] {
            let offset = u32::try_from(bytes.len()).unwrap();
            let size = u32::try_from(content.len()).unwrap();
            let crc = icy_net::crc::get_crc32(content);
            let name_len = u16::try_from(name.len()).unwrap();
            let mut local = [0u8; 30];
            put32(&mut local, 0, 0x0403_4b50);
            put16(&mut local, 4, 20); // version needed
            put16(&mut local, 12, 0x0021); // 1980-01-01
            put32(&mut local, 14, crc);
            put32(&mut local, 18, size);
            put32(&mut local, 22, size);
            put16(&mut local, 26, name_len);
            bytes.extend_from_slice(&local);
            bytes.extend_from_slice(name.as_bytes());
            bytes.extend_from_slice(content);

            let mut header = [0u8; 46];
            put32(&mut header, 0, 0x0201_4b50);
            put16(&mut header, 4, 20); // version made by
            put16(&mut header, 6, 20); // version needed
            put16(&mut header, 14, 0x0021);
            put32(&mut header, 16, crc);
            put32(&mut header, 20, size);
            put32(&mut header, 24, size);
            put16(&mut header, 28, name_len);
            put32(&mut header, 42, offset);
            central.extend_from_slice(&header);
            central.extend_from_slice(name.as_bytes());
        }
        let mut end = [0u8; 22];
        put32(&mut end, 0, 0x0605_4b50);
        put16(&mut end, 8, 2);
        put16(&mut end, 10, 2);
        put32(&mut end, 12, u32::try_from(central.len()).unwrap());
        put32(&mut end, 16, u32::try_from(bytes.len()).unwrap());
        bytes.extend_from_slice(&central);
        bytes.extend_from_slice(&end);
        fs::write(path, bytes).unwrap();
    }

    fn assert_archive(path: &Path, expected: &[(&str, &[u8])]) {
        let mut actual: Vec<_> = dizbase::scan_file_contents(&path.to_path_buf())
            .unwrap()
            .into_iter()
            .map(|entry| (entry.name, entry.size))
            .collect();
        let mut names: Vec<_> = expected.iter().map(|(name, data)| (name.to_string(), data.len() as u64)).collect();
        actual.sort();
        names.sort();
        assert_eq!(actual, names, "archive member names and sizes");

        // Verify decoded bytes, not just filenames or compressed-byte substrings.
        // A single exact SHA-256+size fingerprint must match only its named member.
        // dry_run keeps the archive (including FILE_ID.DIZ) completely untouched.
        let before = fs::read(path).unwrap();
        let fingerprints = tempfile::tempdir().unwrap();
        for (name, data) in expected {
            fs::write(fingerprints.path().join("expected.bin"), data).unwrap();
            let rules = FingerprintData::scan_fingerprint_dir(&fingerprints.path()).unwrap();
            let options = RepackOptions {
                lowercase_names: false,
                recompress: false,
                dry_run: true,
                ..Default::default()
            };
            match repack_file(path, &rules, &options).unwrap() {
                Repacked::Converted { removed, .. } => assert_eq!(removed, vec![name.to_string()], "exact bytes of {name}"),
                _ => panic!("archive does not contain the exact expected bytes for {name}"),
            }
        }
        assert_eq!(fs::read(path).unwrap(), before, "verification must not alter the scanner input");
    }

    #[test]
    #[ignore = "internal fake-scanner subprocess, launched by the processor test"]
    fn scanner_checks_generated_archive() {
        let path = PathBuf::from(std::env::var_os(SCANNER_FILE).expect("scanner input"));
        assert_archive(&path, &[("FILE_ID.DIZ", DIZ), ("PAYLOAD.TXT", PAYLOAD), (AD_NAME, AD_CONTENT)]);
        let marker = PathBuf::from(std::env::var_os(SCANNER_MARKER).expect("scanner marker"));
        fs::write(marker, path.to_str().unwrap()).unwrap();
    }

    #[test]
    fn compiled_ppe_is_added_before_scanning_and_original_diz_is_preserved() {
        let Some(root) = std::env::var_os(CHILD_ROOT) else {
            // Isolate the entire async processor, including blocking archive work
            // and runtime shutdown, behind the same hard subprocess deadline.
            let directory = tempfile::tempdir().unwrap();
            let output = bounded_output(
                Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", PROCESSOR_TEST, "--nocapture", "--test-threads=1"])
                    .env(CHILD_ROOT, directory.path())
                    .env_remove("ICB_PATH")
                    .current_dir(directory.path()),
                Duration::from_secs(60),
            );
            assert_success(&output);
            assert!(
                directory.path().join("processor passed").is_file(),
                "processor child must actually run the test"
            );
            return;
        };
        let root = PathBuf::from(root);
        let ppe = compile_ppe(&root, GENERATOR);
        let input = root.join("input.zip");
        write_input_zip(&input);
        assert_archive(&input, &[("FILE_ID.DIZ", DIZ), ("PAYLOAD.TXT", PAYLOAD)]);
        let marker = root.join("scanner saw generated archive;marker");
        let test_binary = std::env::current_exe().unwrap();

        // Negative control: this scanner must reject the original archive; a
        // successful scanner exit cannot merely mean it was invoked too early.
        let before_generation = bounded_output(
            Command::new(&test_binary)
                .args(["--ignored", "--exact", SCANNER_TEST, "--nocapture", "--test-threads=1"])
                .env(SCANNER_FILE, &input)
                .env(SCANNER_MARKER, &marker),
            Duration::from_secs(10),
        );
        assert!(before_generation.status.code().is_some_and(|code| code != 0));
        assert!(!marker.exists());

        let config = UploadProcessingConfig {
            publish_policy: UploadPublishPolicy::AfterProcessing,
            quarantine_path: root.join("quarantine with spaces;one"),
            advertisement_file: ppe,
            scanner: UploadScannerConfig {
                enabled: true,
                executable: PathBuf::from("/bin/sh"),
                arguments: vec![
                    "-c".into(),
                    format!("{SCANNER_FILE}=\"$2\" {SCANNER_MARKER}=\"$3\" exec \"$1\" --ignored --exact {SCANNER_TEST} --nocapture --test-threads=1"),
                    "advertisement-scanner".into(),
                    test_binary.to_str().unwrap().into(),
                    "{file}".into(),
                    marker.to_str().unwrap().into(),
                ],
                timeout_seconds: 10,
                ..Default::default()
            },
            ..Default::default()
        };
        let quarantine = UploadQuarantine::new(config.quarantine_path.clone());
        let queued = quarantine
            .enqueue(
                &input,
                ORIGINAL_NAME.into(),
                root.join("public"),
                root.join("metadata"),
                "ALICE".into(),
                vec!["User supplied description".into()],
            )
            .unwrap();
        let original_payload = quarantine.payload_path(&queued);
        let processor = UploadProcessor::new(config);
        let processed = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(processor.process(&queued.id))
            .unwrap();
        assert_eq!(processed.status, QuarantineStatus::ReadyToPublish, "{:?}", processed.processing_report);
        assert_eq!(processed.original_name, ORIGINAL_NAME);
        assert_eq!(processed.description, queued.description);
        assert_eq!(processed, quarantine.load(&queued.id).unwrap());
        let payload = quarantine.payload_path(&processed);
        assert_ne!(payload, original_payload);
        assert!(!original_payload.exists());
        assert_eq!(fs::read_to_string(&marker).unwrap(), payload.to_str().unwrap());
        let added = processed
            .processing_report
            .iter()
            .position(|entry| entry == &format!("added member: {AD_NAME}"))
            .unwrap();
        let scanned = processed.processing_report.iter().position(|entry| entry == "virus scanner: clean").unwrap();
        assert!(added < scanned, "{:?}", processed.processing_report);
        assert_archive(&payload, &[("FILE_ID.DIZ", DIZ), ("PAYLOAD.TXT", PAYLOAD), (AD_NAME, AD_CONTENT)]);
        assert!(!root.join("public").exists(), "processing must not bypass publication");
        fs::write(root.join("processor passed"), b"ok").unwrap();
    }
}
