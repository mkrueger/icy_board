//! One optional static advertisement, or a trusted, headless PPE generator.
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use dizbase::file_base_scanner::repack::ArchiveAddition;
use tokio::{io::AsyncReadExt, process::Command, sync::Mutex};

use super::{IcyBoard, bbs::BBS, state::IcyBoardState};
use crate::{
    Res,
    executable::Executable,
    vm::{self, io::DiskIO},
};

const MAX_ADVERTISEMENT_SIZE: u64 = 16 * 1024 * 1024;
const GENERATOR_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PPE_SIZE: u64 = 16 * 1024 * 1024;

struct GeneratorConnection;

#[async_trait::async_trait]
impl icy_net::Connection for GeneratorConnection {
    fn get_connection_type(&self) -> icy_net::ConnectionType {
        icy_net::ConnectionType::Channel
    }
    async fn read(&mut self, _buffer: &mut [u8]) -> icy_net::Result<usize> {
        Err("advertisement PPE cannot request interactive input".into())
    }
    async fn try_read(&mut self, _buffer: &mut [u8]) -> icy_net::Result<usize> {
        Ok(0)
    }
    async fn send(&mut self, _buffer: &[u8]) -> icy_net::Result<()> {
        Ok(())
    }
}

/// No shell expansion or list parsing: a semicolon is part of the filename.
pub async fn load_advertisement(source: &Path, original_name: &str, max_member_size: u64) -> Res<Vec<ArchiveAddition>> {
    if source.as_os_str().is_empty() {
        return Ok(Vec::new());
    }
    if !source.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("ppe")) {
        return Ok(vec![read_advertisement(source, max_member_size)?]);
    }
    let source = fs::canonicalize(source)?;
    if !source.is_file() || fs::metadata(&source)?.len() > MAX_PPE_SIZE {
        return Err("advertisement PPE must be a regular file of at most 16 MiB".into());
    }
    let runner = generator_runner()?;
    generate_advertisement(&runner, &source, original_name, max_member_size, GENERATOR_TIMEOUT).await
}

fn generator_runner() -> Res<PathBuf> {
    let executable = std::env::current_exe()?;
    let directory = executable.parent().ok_or("cannot locate advertisement PPE runner")?;
    let name = format!("icboard{}", std::env::consts::EXE_SUFFIX);
    let sibling = directory.join(&name);
    if sibling.is_file() {
        return Ok(sibling);
    }
    // Cargo examples and integration tests run one level below the binaries.
    if matches!(directory.file_name().and_then(|name| name.to_str()), Some("deps" | "examples")) {
        let sibling = directory.parent().ok_or("missing binary directory")?.join(name);
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    Err("advertisement PPE runner unavailable: install icboard beside the running program".into())
}

async fn generate_advertisement(runner: &Path, ppe: &Path, original_name: &str, max_member_size: u64, timeout: Duration) -> Res<Vec<ArchiveAddition>> {
    let work = tempfile::tempdir()?;
    let output = work.path().join("output");
    fs::create_dir(&output)?;
    let output = fs::canonicalize(output)?;
    let mut child = Command::new(runner)
        .arg("--upload-advertisement-ppe")
        .arg(ppe)
        .arg(&output)
        .arg(original_name)
        .current_dir(&output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut stderr = child.stderr.take().ok_or("PPE runner stderr unavailable")?;
    let mut diagnostics = Vec::new();
    let result = tokio::time::timeout(timeout, async {
        tokio::try_join!(child.wait(), async {
            let mut buffer = [0; 4096];
            loop {
                let count = stderr.read(&mut buffer).await?;
                if count == 0 {
                    break;
                }
                let retained = count.min(16 * 1024 - diagnostics.len());
                diagnostics.extend_from_slice(&buffer[..retained]);
            }
            Ok::<_, std::io::Error>(())
        })
    })
    .await;
    let status = match result {
        Ok(Ok((status, ()))) => status,
        other => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(match other {
                Err(_) => format!("advertisement PPE timed out after {} seconds", timeout.as_secs()),
                Ok(Err(error)) => format!("advertisement PPE process failed: {error}"),
                _ => unreachable!(),
            }
            .into());
        }
    };
    if !status.success() {
        return Err(format!("advertisement PPE failed ({status}): {:?}", String::from_utf8_lossy(&diagnostics)).into());
    }
    collect_generated_advertisement(&output, max_member_size)
}

fn collect_generated_advertisement(output: &Path, max_member_size: u64) -> Res<Vec<ArchiveAddition>> {
    if !fs::symlink_metadata(output)?.file_type().is_dir() {
        return Err("advertisement PPE replaced its output directory".into());
    }
    let mut entries = fs::read_dir(output)?;
    let Some(entry) = entries.next().transpose()? else {
        return Ok(Vec::new());
    };
    if entries.next().transpose()?.is_some() {
        return Err("advertisement PPE must produce at most one file".into());
    }
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_file() {
        return Err("advertisement PPE output must be a regular file, not a directory or symlink".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err("advertisement PPE output must not be a hard link".into());
        }
    }
    Ok(vec![read_advertisement(&path, max_member_size)?])
}

fn read_advertisement(path: &Path, max_member_size: u64) -> Res<ArchiveAddition> {
    let name = path.file_name().and_then(|name| name.to_str()).ok_or("advertisement has no usable filename")?;
    if name.contains(['/', '\\']) || name.chars().any(char::is_control) {
        return Err("advertisement filename contains a separator or control character".into());
    }
    if ["file_id.diz", "file_id.ans", "file_id.pcb", "desc.sdi"]
        .iter()
        .any(|protected| name.eq_ignore_ascii_case(protected))
    {
        return Err("advertisement is a protected description file; adding or replacing descriptions is not supported".into());
    }
    let limit = max_member_size.min(MAX_ADVERTISEMENT_SIZE);
    if !fs::metadata(path)?.is_file() || fs::metadata(path)?.len() > limit {
        return Err(format!("advertisement must be a regular file no larger than {limit} bytes").into());
    }
    let mut content = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut content)?;
    if content.len() as u64 > limit {
        return Err(format!("advertisement exceeds {limit} bytes").into());
    }
    Ok(ArchiveAddition {
        name: name.to_string(),
        content,
    })
}

/// Internal icboard worker entry point. This is process isolation for timeouts,
/// NOT a filesystem sandbox: only run SysOp-installed, trusted PPEs.
pub async fn run_advertisement_ppe(ppe: &Path, output: &Path, original_name: &str) -> Res<()> {
    let output = fs::canonicalize(output)?;
    if !output.is_dir() || fs::read_dir(&output)?.next().is_some() {
        return Err("advertisement output directory must exist and be empty".into());
    }
    let ppe = fs::canonicalize(ppe)?;
    if fs::metadata(&ppe)?.len() > MAX_PPE_SIZE {
        return Err("advertisement PPE exceeds 16 MiB".into());
    }
    let executable = Executable::read_file(&ppe, false)?;
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    // Do not load the live board, acquire its lock or grant a logged-in SysOp.
    let mut board = IcyBoard::new();
    board.root_path = output.clone();
    let board = Arc::new(Mutex::new(board));
    let node = bbs.lock().await.create_new_node(icy_net::ConnectionType::Channel).await;
    let node_states = bbs.lock().await.open_connections.clone();
    let mut state = IcyBoardState::new(bbs, board, node_states, node, Box::new(GeneratorConnection)).await;
    // Push literal tokens: parsing a command line would split paths containing
    // spaces or semicolons. A trailing separator makes simple concatenation safe.
    state
        .session
        .tokens
        .push_back(format!("{}{sep}", output.display(), sep = std::path::MAIN_SEPARATOR));
    state.session.tokens.push_back(original_name.to_string());
    let mut io = DiskIO::new(&output.to_string_lossy(), None);
    if !vm::run(&ppe, &executable, &mut io, &mut state).await? {
        return Err("advertisement PPE aborted (STOP)".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_output_is_optional_but_must_be_one_regular_non_description_file() {
        let work = tempfile::tempdir().unwrap();
        assert!(collect_generated_advertisement(work.path(), 100).unwrap().is_empty());
        let file = work.path().join("BOARD.AD");
        fs::write(&file, b"hello").unwrap();
        assert_eq!(b"hello", collect_generated_advertisement(work.path(), 100).unwrap()[0].content.as_slice());
        assert!(collect_generated_advertisement(work.path(), 4).is_err());
        fs::write(work.path().join("SECOND.AD"), b"hello").unwrap();
        assert!(collect_generated_advertisement(work.path(), 100).is_err());
        fs::remove_file(work.path().join("SECOND.AD")).unwrap();
        fs::rename(&file, work.path().join("FiLe_Id.DiZ")).unwrap();
        assert!(collect_generated_advertisement(work.path(), 100).is_err());
    }

    #[test]
    fn generated_directories_are_rejected() {
        let work = tempfile::tempdir().unwrap();
        fs::create_dir(work.path().join("nested")).unwrap();
        assert!(collect_generated_advertisement(work.path(), 100).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn generated_links_are_rejected() {
        let work = tempfile::tempdir().unwrap();
        let external = tempfile::NamedTempFile::new().unwrap();
        let path = work.path().join("BOARD.AD");
        std::os::unix::fs::symlink(external.path(), &path).unwrap();
        assert!(collect_generated_advertisement(work.path(), 100).is_err());
        fs::remove_file(&path).unwrap();
        fs::hard_link(external.path(), &path).unwrap();
        assert!(collect_generated_advertisement(work.path(), 100).is_err());
    }

    #[tokio::test]
    async fn empty_and_literal_static_paths() {
        assert!(load_advertisement(Path::new(""), "UPLOAD.ZIP", 100).await.unwrap().is_empty());
        let work = tempfile::tempdir().unwrap();
        let path = work.path().join("board ad;one.txt");
        fs::write(&path, b"board").unwrap();
        let result = load_advertisement(&path, "UPLOAD.ZIP", 100).await.unwrap();
        assert_eq!(1, result.len());
        assert_eq!("board ad;one.txt", result[0].name);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn generator_process_timeout_is_enforced() {
        use std::os::unix::fs::PermissionsExt;
        let work = tempfile::tempdir().unwrap();
        let runner = work.path().join("runner");
        fs::write(&runner, "#!/bin/sh\nwhile :; do :; done\n").unwrap();
        fs::set_permissions(&runner, fs::Permissions::from_mode(0o700)).unwrap();
        let error = generate_advertisement(&runner, Path::new("unused.ppe"), "UPLOAD.ZIP", 100, Duration::from_millis(100))
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("timed out"));
    }
}
