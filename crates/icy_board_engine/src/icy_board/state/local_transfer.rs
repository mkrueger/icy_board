//! Host filesystem transfers are a capability of the local console, not SysOp privileges.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use icy_net::protocol::TransferState;
use tempfile::{NamedTempFile, TempPath};
use tokio::sync::{mpsc, oneshot};

use super::IcyBoardState;
use crate::Res;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFilePickerKind {
    UploadFile,
    DownloadDirectory,
}

#[derive(Debug)]
pub struct LocalFilePickerRequest {
    pub kind: LocalFilePickerKind,
    pub response: oneshot::Sender<Option<PathBuf>>,
}

impl IcyBoardState {
    async fn local_picker_sender(&self) -> Option<mpsc::Sender<LocalFilePickerRequest>> {
        if !self.session.is_local || self.session.request_logoff {
            return None;
        }
        self.node_state
            .lock()
            .await
            .get(self.node)?
            .as_ref()?
            .local_file_picker
            .clone()
            .filter(|sender| !sender.is_closed())
    }

    /// Cancellation, a missing capability, and a disconnected UI all return `None`.
    /// No node/board lock is held while the host UI is awaiting input.
    pub async fn request_local_path(&mut self, kind: LocalFilePickerKind) -> Res<Option<PathBuf>> {
        let Some(sender) = self.local_picker_sender().await else { return Ok(None) };
        let (response, receiver) = oneshot::channel();
        if sender.send(LocalFilePickerRequest { kind, response }).await.is_err() {
            return Ok(None);
        }
        tokio::select! {
            result = receiver => Ok(result.unwrap_or(None)),
            _ = sender.closed() => Ok(None),
        }
    }

    /// Copy only the command's offered files. Accounting retains ORIGINAL source paths.
    pub async fn local_download_files(&mut self, files: &[PathBuf]) -> Res<Option<TransferState>> {
        let Some(directory) = self.request_local_path(LocalFilePickerKind::DownloadDirectory).await? else {
            return Ok(None);
        };
        // Recheck capability after the UI await, before any filesystem writes.
        if self.local_picker_sender().await.is_none() {
            return Ok(None);
        }
        let files = files.to_vec();
        Ok(Some(tokio::task::spawn_blocking(move || copy_downloads(&directory, &files)).await??))
    }
}

fn open_regular(path: &Path) -> io::Result<fs::File> {
    // Reject symlinks and special files, not just directories.
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Select a regular file (not a symlink)"));
    }
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Source is not a regular file"));
    }
    Ok(file)
}

/// The returned temporary file is owned by the caller; the selected original is never removed.
pub fn stage_local_upload(path: &Path) -> Res<TempPath> {
    let mut source = open_regular(path)?;
    let mut staged = NamedTempFile::new()?;
    io::copy(&mut source, staged.as_file_mut())?;
    staged.as_file().sync_all()?;
    Ok(staged.into_temp_path())
}

fn ensure_unused(directory: &Path, name: &std::ffi::OsStr) -> io::Result<()> {
    let folded = name.to_string_lossy().to_lowercase();
    for entry in fs::read_dir(directory)? {
        // read_dir sees dangling symlinks too. Lossy comparison is conservative for
        // non-UTF8 names; paths themselves always retain their original OS encoding.
        if entry?.file_name().to_string_lossy().to_lowercase() == folded {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "Destination already exists (case insensitive)"));
        }
    }
    Ok(())
}

fn copy_one(directory: &Path, source: &Path) -> Res<u64> {
    let name = source
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing source filename"))?;
    let mut input = open_regular(source)?;
    ensure_unused(directory, name)?; // Also explicitly skips the same source/destination.
    let mut staged = NamedTempFile::new_in(directory)?;
    let bytes = io::copy(&mut input, staged.as_file_mut())?;
    staged.as_file().sync_all()?;
    ensure_unused(directory, name)?;
    staged.persist_noclobber(directory.join(name))?;
    Ok(bytes)
}

fn copy_downloads(directory: &Path, files: &[PathBuf]) -> Res<TransferState> {
    let mut transfer = TransferState::new("Local copy".into());
    // Freeze relative paths and resolve the selected directory without changing cwd.
    let directory = fs::canonicalize(directory)?;
    if !directory.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Destination is not a directory").into());
    }
    for source in files {
        match copy_one(&directory, source) {
            Ok(bytes) => {
                transfer.send_state.total_bytes_transfered += bytes;
                transfer
                    .send_state
                    .finished_files
                    .push((source.file_name().unwrap().to_string_lossy().into_owned(), source.clone()));
            }
            Err(error) => {
                let message = format!("Local download skipped {}: {error}", source.display());
                log::warn!("{message}");
                transfer.send_state.log_error(message);
            }
        }
    }
    transfer.is_finished = true;
    transfer.current_state = "Local copy finished";
    Ok(transfer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::{IcyBoard, bbs::BBS};
    use icy_net::{ConnectionType, channel::ChannelConnection};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn state() -> (IcyBoardState, ChannelConnection) {
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        (
            IcyBoardState::new(bbs, Arc::new(Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await,
            peer,
        )
    }

    #[tokio::test]
    async fn capability_local_only_cancel_and_disconnect() {
        let (mut state, _peer) = state().await;
        state.session.is_local = true;
        assert!(state.request_local_path(LocalFilePickerKind::UploadFile).await.unwrap().is_none());
        assert!(state.local_download_files(&[PathBuf::from("no-capability")]).await.unwrap().is_none());
        let (tx, mut rx) = mpsc::channel(1);
        state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(tx);
        state.session.is_local = false;
        state.session.is_sysop = true;
        assert!(state.request_local_path(LocalFilePickerKind::UploadFile).await.unwrap().is_none());
        assert!(state.local_download_files(&[PathBuf::from("must-not-be-read")]).await.unwrap().is_none());
        assert!(rx.try_recv().is_err());
        state.session.is_local = true;
        state.session.is_sysop = false;
        let (result, ()) = tokio::join!(state.request_local_path(LocalFilePickerKind::UploadFile), async {
            let request = rx.recv().await.unwrap();
            assert_eq!(request.kind, LocalFilePickerKind::UploadFile);
            request.response.send(None).unwrap();
        });
        assert!(result.unwrap().is_none());
        let (result, ()) = tokio::join!(state.request_local_path(LocalFilePickerKind::UploadFile), async {
            drop(rx.recv().await.unwrap());
        });
        assert!(result.unwrap().is_none());
        let (result, ()) = tokio::join!(state.request_local_path(LocalFilePickerKind::UploadFile), async {
            let request = rx.recv().await.unwrap();
            rx.close();
            // Even a retained response sender must not keep a disconnected UI alive.
            tokio::task::yield_now().await;
            drop(request);
        });
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn download_bridge_success_and_cancel() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let file = source.path().join("offered.bin");
        fs::write(&file, b"offered").unwrap();
        let files = [file.clone()];
        let (mut state, _peer) = state().await;
        state.session.is_local = true;
        let (tx, mut rx) = mpsc::channel(1);
        state.node_state.lock().await[state.node].as_mut().unwrap().local_file_picker = Some(tx);
        let (result, ()) = tokio::join!(state.local_download_files(&files), async {
            let request = rx.recv().await.unwrap();
            assert_eq!(request.kind, LocalFilePickerKind::DownloadDirectory);
            request.response.send(None).unwrap();
        });
        assert!(result.unwrap().is_none());
        assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 0);
        let (result, ()) = tokio::join!(state.local_download_files(&files), async {
            rx.recv().await.unwrap().response.send(Some(destination.path().to_path_buf())).unwrap();
        });
        let transfer = result.unwrap().unwrap();
        assert_eq!(transfer.send_state.total_bytes_transfered, 7);
        assert_eq!(transfer.send_state.finished_files, vec![("offered.bin".into(), file.clone())]);
        assert_eq!(fs::read(&file).unwrap(), b"offered");
        assert_eq!(fs::read(destination.path().join("offered.bin")).unwrap(), b"offered");
    }

    #[test]
    fn staging_preserves_source_and_rejects_directories() {
        let source = tempfile::tempdir().unwrap();
        let path = source.path().join("source.bin");
        fs::write(&path, b"original").unwrap();
        let staged = stage_local_upload(&path).unwrap();
        assert_eq!(fs::read(&staged).unwrap(), b"original");
        drop(staged);
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert!(stage_local_upload(source.path()).is_err());
    }

    #[test]
    fn downloads_skip_existing_case_insensitive_same_path_and_bad_sources() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let path = source.path().join("File.bin");
        fs::write(&path, b"original").unwrap();
        fs::write(destination.path().join("FILE.BIN"), b"existing").unwrap();
        let transfer = copy_downloads(destination.path(), &[path.clone(), source.path().to_path_buf()]).unwrap();
        assert!(transfer.send_state.finished_files.is_empty());
        assert_eq!(transfer.send_state.total_bytes_transfered, 0);
        assert_eq!(transfer.send_state.errors, 2);
        assert!(
            copy_downloads(source.path(), std::slice::from_ref(&path))
                .unwrap()
                .send_state
                .finished_files
                .is_empty()
        );
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read(destination.path().join("FILE.BIN")).unwrap(), b"existing");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_never_clobbered_or_staged() {
        use std::os::unix::fs::symlink;
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let path = source.path().join("source.bin");
        fs::write(&path, b"original").unwrap();
        let link = source.path().join("link.bin");
        symlink(&path, &link).unwrap();
        assert!(stage_local_upload(&link).is_err());
        symlink(source.path().join("missing"), destination.path().join("source.bin")).unwrap();
        let transfer = copy_downloads(destination.path(), &[path.clone(), link]).unwrap();
        assert!(transfer.send_state.finished_files.is_empty());
        assert!(fs::symlink_metadata(destination.path().join("source.bin")).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(path).unwrap(), b"original");
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_remain_os_paths_and_partial_batches_count_only_successes() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let path = source.path().join(OsStr::from_bytes(b"file-\xff.bin"));
        fs::write(&path, b"raw name").unwrap();
        let staged = stage_local_upload(&path).unwrap();
        assert_eq!(fs::read(&staged).unwrap(), b"raw name");
        let transfer = copy_downloads(destination.path(), &[source.path().join("missing"), path.clone(), path.clone()]).unwrap();
        assert_eq!(transfer.send_state.finished_files.len(), 1);
        assert_eq!(transfer.send_state.finished_files[0].1, path);
        assert_eq!(transfer.send_state.total_bytes_transfered, 8);
        assert_eq!(transfer.send_state.errors, 2);
        assert_eq!(fs::read(destination.path().join(path.file_name().unwrap())).unwrap(), b"raw name");
        assert_eq!(fs::read(path).unwrap(), b"raw name");
    }
}
