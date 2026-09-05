use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::Duration,
};

use tempfile::TempPath;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    time::Instant,
};

use crate::{
    Connection, NetError,
    binkp::{BinkpCommand, Frame, FrameReader},
};
use std::fmt::Write as _;

/// How much of a file goes into one data frame.
pub const DATA_BLOCK_SIZE: usize = 16384;

/// What a file being received is called until it is all there.
const PARTIAL_PREFIX: &str = "binkp-";
const PARTIAL_SUFFIX: &str = ".tmp";

/// How long such a file is left alone before it counts as what a session that
/// was killed off left behind. A transfer running right now is far younger.
const PARTIAL_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

/// The characters FTS-1026 5.4 lets a file name carry unescaped.
const SAFE: &str = "!\"#$%&'()*+,-./:;<=>?@[]^_`{|}~";

#[derive(Clone, Debug, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    /// Modification time in seconds since the epoch, which is how binkp tells
    /// two files of the same name apart.
    pub time: u64,
}

impl FileInfo {
    /// File identity and, for M_FILE/M_GET, the required offset. None denotes
    /// the NR offset request (-1), which is valid only in M_FILE (FTS-1028).
    fn parse(argument: &str, command: BinkpCommand) -> Option<(FileInfo, Option<u64>)> {
        let mut fields = argument.split_whitespace();
        let name = unescape_filename(fields.next()?);
        let size = fields.next()?.parse().ok()?;
        let time = fields.next()?.parse().ok()?;
        let offset = if matches!(command, BinkpCommand::File | BinkpCommand::Get) {
            match fields.next()? {
                "-1" if command == BinkpCommand::File => None,
                value if value.bytes().all(|byte| byte.is_ascii_digit()) => Some(value.parse().ok()?),
                _ => return None,
            }
        } else {
            Some(0)
        };
        Some((FileInfo { name, size, time }, offset))
    }

    fn to_argument(&self, offset: Option<u64>) -> String {
        match offset {
            Some(offset) => format!("{} {} {} {}", escape_filename(&self.name), self.size, self.time, offset),
            None => format!("{} {} {}", escape_filename(&self.name), self.size, self.time),
        }
    }
}

/// A file waiting in the outbound for the next session.
#[derive(Clone, Debug, PartialEq)]
pub struct OutboundFile {
    pub path: PathBuf,
    pub info: FileInfo,
}

impl OutboundFile {
    pub async fn open(path: &Path) -> crate::Result<Self> {
        let metadata = tokio::fs::metadata(path).await?;
        let time = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or(0);
        Ok(Self {
            path: path.to_path_buf(),
            info: FileInfo {
                name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                size: metadata.len(),
                time,
            },
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BatchResult {
    /// Files the remote acknowledged (M_GOT or M_GET at EOF), safe to delete.
    pub sent: Vec<PathBuf>,
    /// Files the remote asked us to offer again next time.
    pub skipped: Vec<PathBuf>,
    pub received: Vec<PathBuf>,
}

struct Sending {
    file: OutboundFile,
    handle: File,
    offset: u64,
}

struct Receiving {
    info: FileInfo,
    /// The name the file is meant to end up under once it is all there.
    target: PathBuf,
    /// Where the bytes go until then. Dropping this removes the partial file.
    partial: TempPath,
    handle: File,
    written: u64,
}

impl Receiving {
    /// A file is written under a name of its own until the last octet has
    /// arrived, so a session that breaks off cannot leave half a bundle where
    /// the tosser would read it as whole.
    async fn start(inbound: &Path, info: FileInfo, name: &str) -> crate::Result<Self> {
        let partial = tempfile::Builder::new().prefix(PARTIAL_PREFIX).suffix(PARTIAL_SUFFIX).tempfile_in(inbound)?;
        let handle = File::from_std(partial.reopen()?);
        Ok(Self {
            info,
            target: inbound.join(name),
            partial: partial.into_temp_path(),
            handle,
            written: 0,
        })
    }

    /// Puts the finished file under the name it was offered as, which is where
    /// the tosser looks for it.
    async fn store(mut self) -> crate::Result<PathBuf> {
        self.handle.flush().await?;
        let Some(target) = free_name(&self.target) else {
            return Err(NetError::BinkpNoFreeName(self.target.display().to_string()).into());
        };
        drop(self.handle);
        self.partial.persist(&target).map_err(|error| error.error)?;
        Ok(target)
    }
}

/// Clears away the working files of sessions that were killed off. Nothing
/// here is worth failing a transfer over, so what cannot be read is left.
async fn remove_stale_partials(inbound: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(inbound).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if !is_partial(&path) {
            continue;
        }
        let stale = match entry.metadata().await.and_then(|metadata| metadata.modified()) {
            Ok(modified) => modified.elapsed().is_ok_and(|age| age >= PARTIAL_LIFETIME),
            Err(_) => false,
        };
        if stale {
            log::info!("binkp: removing {}, which a session that did not finish left behind", path.display());
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}

fn is_partial(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(PARTIAL_PREFIX) && name.ends_with(PARTIAL_SUFFIX))
}

/// A name the inbound does not hold yet. Mail still waiting to be tossed must
/// not be replaced by a file that happens to be called the same, and the
/// extension is kept so that what arrives is still recognised as a bundle.
fn free_name(target: &Path) -> Option<PathBuf> {
    if !target.exists() {
        return Some(target.to_path_buf());
    }
    let stem = target.file_stem()?.to_string_lossy().to_string();
    let extension = target.extension().map(|extension| extension.to_string_lossy().to_string());
    (1..100).find_map(|counter| {
        let name = match &extension {
            Some(extension) => format!("{stem}-{counter}.{extension}"),
            None => format!("{stem}-{counter}"),
        };
        let candidate = target.with_file_name(name);
        (!candidate.exists()).then_some(candidate)
    })
}

/// Runs the file transfer stage of FTS-1026 6.2 until both sides have sent
/// their end of batch and every file has been accounted for.
pub async fn transfer_batch(connection: &mut dyn Connection, outbound: Vec<OutboundFile>, inbound: &Path, timeout: Duration) -> crate::Result<BatchResult> {
    tokio::fs::create_dir_all(inbound).await?;
    remove_stale_partials(inbound).await;

    let mut reader = FrameReader::new();
    let mut result = BatchResult::default();
    let mut waiting = outbound.into_iter();
    let mut sending: Option<Sending> = None;
    let mut unacknowledged: Vec<OutboundFile> = Vec::new();
    // Finish any other active file before announcing a retry: interleaving
    // M_FILE/data would otherwise abandon that file at the receiver.
    let mut retries: VecDeque<(OutboundFile, u64)> = VecDeque::new();
    let mut receiving: Option<Receiving> = None;
    let mut sent_eob = false;
    let mut got_eob = false;
    let mut deadline = Instant::now() + timeout;

    loop {
        if got_eob && sent_eob && sending.is_none() && retries.is_empty() && unacknowledged.is_empty() && receiving.is_none() {
            return Ok(result);
        }
        let mut worked = false;

        // One frame in and one block out per turn, so neither side can starve the other.
        if let Some(frame) = reader.poll(connection).await? {
            worked = true;
            match frame {
                Frame::Data(data) => {
                    if let Some(current) = &mut receiving {
                        if data.len() as u64 > current.info.size - current.written {
                            return abort(
                                connection,
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("binkp: data for {} exceeds advertised size {}", current.info.name, current.info.size),
                                ),
                            )
                            .await;
                        }
                        current.handle.write_all(&data).await?;
                        current.written += data.len() as u64;
                        if current.written == current.info.size {
                            // The remote may only be told the file arrived once
                            // it is somewhere the next run will find it.
                            let current = receiving.take().unwrap();
                            let info = current.info.clone();
                            let path = current.store().await?;
                            Frame::command(BinkpCommand::Got, info.to_argument(None)).send(connection).await?;
                            result.received.push(path);
                        }
                    }
                    // With no receiving file, discard data already in flight
                    // after M_SKIP/M_GET (FTS-1026, RxWaitF).
                }

                Frame::Command(BinkpCommand::File, argument) => {
                    let Some((info, offset)) = FileInfo::parse(&argument, BinkpCommand::File) else {
                        return abort(connection, NetError::BinkpBadArgument("M_FILE".to_string(), argument)).await;
                    };
                    if offset.is_some_and(|offset| offset > info.size) {
                        return abort(connection, NetError::BinkpBadArgument("M_FILE".to_string(), argument)).await;
                    }
                    // An unfinished file is dropped rather than mixed with the next one.
                    receiving = None;
                    if offset != Some(0) {
                        Frame::command(BinkpCommand::Get, info.to_argument(Some(0))).send(connection).await?;
                        continue;
                    }
                    let Some(name) = safe_name(&info.name) else {
                        log::warn!("binkp: refusing file name '{}'", info.name);
                        Frame::command(BinkpCommand::Skip, info.to_argument(None)).send(connection).await?;
                        continue;
                    };
                    let current = Receiving::start(inbound, info, &name).await?;
                    if current.info.size == 0 {
                        let info = current.info.clone();
                        let path = current.store().await?;
                        Frame::command(BinkpCommand::Got, info.to_argument(None)).send(connection).await?;
                        result.received.push(path);
                    } else {
                        receiving = Some(current);
                    }
                }

                Frame::Command(BinkpCommand::Got, argument) => {
                    let Some((info, _)) = FileInfo::parse(&argument, BinkpCommand::Got) else {
                        return abort(connection, NetError::BinkpBadArgument("M_GOT".to_string(), argument)).await;
                    };
                    // Arriving mid file this is a destructive skip, so stop sending either way.
                    if sending.as_ref().is_some_and(|current| current.file.info == info) {
                        let current = sending.take().unwrap();
                        result.sent.push(current.file.path);
                    } else if let Some(index) = unacknowledged.iter().position(|file| file.info == info) {
                        result.sent.push(unacknowledged.remove(index).path);
                    } else if let Some(index) = retries.iter().position(|(file, _)| file.info == info) {
                        result.sent.push(retries.remove(index).unwrap().0.path);
                    }
                }

                Frame::Command(BinkpCommand::Skip, argument) => {
                    let Some((info, _)) = FileInfo::parse(&argument, BinkpCommand::Skip) else {
                        return abort(connection, NetError::BinkpBadArgument("M_SKIP".to_string(), argument)).await;
                    };
                    if sending.as_ref().is_some_and(|current| current.file.info == info) {
                        let current = sending.take().unwrap();
                        result.skipped.push(current.file.path);
                    } else if let Some(index) = unacknowledged.iter().position(|file| file.info == info) {
                        result.skipped.push(unacknowledged.remove(index).path);
                    } else if let Some(index) = retries.iter().position(|(file, _)| file.info == info) {
                        result.skipped.push(retries.remove(index).unwrap().0.path);
                    }
                }

                Frame::Command(BinkpCommand::Get, argument) => {
                    let Some((info, Some(offset))) = FileInfo::parse(&argument, BinkpCommand::Get) else {
                        return abort(connection, NetError::BinkpBadArgument("M_GET".to_string(), argument)).await;
                    };
                    if offset > info.size {
                        return abort(connection, NetError::BinkpBadArgument("M_GET".to_string(), argument)).await;
                    }
                    if offset == info.size {
                        // FTS-1026, table 6: an EOF request finalizes the file,
                        // just like a destructive skip via M_GOT.
                        if sending.as_ref().is_some_and(|current| current.file.info == info) {
                            result.sent.push(sending.take().unwrap().file.path);
                        } else if let Some(index) = unacknowledged.iter().position(|file| file.info == info) {
                            result.sent.push(unacknowledged.remove(index).path);
                        } else if let Some(index) = retries.iter().position(|(file, _)| file.info == info) {
                            result.sent.push(retries.remove(index).unwrap().0.path);
                        }
                    } else if let Some(current) = &mut sending
                        && current.file.info == info
                    {
                        current.handle.seek(std::io::SeekFrom::Start(offset)).await?;
                        current.offset = offset;
                        Frame::command(BinkpCommand::File, info.to_argument(Some(offset))).send(connection).await?;
                    } else if let Some((_, queued_offset)) = retries.iter_mut().find(|(file, _)| file.info == info) {
                        *queued_offset = offset;
                    } else if let Some(index) = unacknowledged.iter().position(|file| file.info == info) {
                        retries.push_back((unacknowledged.remove(index), offset));
                        // EOB does not prevent retransmission of an unacknowledged
                        // file. Send a fresh EOB once the retries have drained.
                        sent_eob = false;
                    }
                }

                Frame::Command(BinkpCommand::Eob, _) => got_eob = true,
                Frame::Command(BinkpCommand::Nul, _) => {}
                Frame::Command(BinkpCommand::Err, argument) => return Err(NetError::BinkpRemoteError(argument).into()),
                Frame::Command(BinkpCommand::Bsy, argument) => return Err(NetError::BinkpRemoteBusy(argument).into()),
                Frame::Command(command, _) => {
                    return abort(connection, NetError::BinkpUnexpectedFrame(command.to_string())).await;
                }
            }
        }

        if !sent_eob {
            worked = true;
            if sending.is_none() {
                match retries.pop_front().or_else(|| waiting.next().map(|file| (file, 0))) {
                    Some((file, offset)) => {
                        let mut handle = File::open(&file.path).await?;
                        handle.seek(std::io::SeekFrom::Start(offset)).await?;
                        Frame::command(BinkpCommand::File, file.info.to_argument(Some(offset))).send(connection).await?;
                        sending = Some(Sending { file, handle, offset });
                    }
                    None => {
                        Frame::command(BinkpCommand::Eob, "").send(connection).await?;
                        sent_eob = true;
                    }
                }
            }
            if let Some(current) = &mut sending {
                let mut block = vec![0u8; (current.file.info.size - current.offset).min(DATA_BLOCK_SIZE as u64) as usize];
                let read = current.handle.read(&mut block).await?;
                if read > 0 {
                    block.truncate(read);
                    current.offset += read as u64;
                    Frame::Data(block).send(connection).await?;
                }
                if read == 0 || current.offset >= current.file.info.size {
                    unacknowledged.push(sending.take().unwrap().file);
                }
            }
        }

        if worked {
            deadline = Instant::now() + timeout;
        } else {
            if Instant::now() >= deadline {
                return Err(NetError::BinkpTimeout.into());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

async fn abort(connection: &mut dyn Connection, error: impl std::error::Error + Send + Sync + 'static) -> crate::Result<BatchResult> {
    let _ = Frame::command(BinkpCommand::Err, error.to_string()).send(connection).await;
    Err(error.into())
}

/// Refuses everything a remote could use to write outside the inbound. A binkp
/// file name has no directory part to begin with, so anything that looks like
/// one is a reason to refuse rather than to repair.
fn safe_name(name: &str) -> Option<String> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\', ':']) {
        return None;
    }
    Some(name.to_string())
}

pub fn escape_filename(name: &str) -> String {
    let mut escaped = String::new();
    for byte in name.bytes() {
        let character = byte as char;
        if character.is_ascii_alphanumeric() || SAFE.contains(character) {
            escaped.push(character);
        } else {
            let _ = write!(escaped, "\\x{:02x}", byte);
        }
    }
    escaped
}

pub fn unescape_filename(name: &str) -> String {
    let bytes = name.as_bytes();
    let mut plain = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            // FSP-1011 mailers left out the x, so both spellings are decoded.
            let digits = if index + 1 < bytes.len() && bytes[index + 1] | 0x20 == b'x' {
                index + 2
            } else {
                index + 1
            };
            if digits + 2 <= bytes.len()
                && let Some(value) = std::str::from_utf8(&bytes[digits..digits + 2])
                    .ok()
                    .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            {
                plain.push(value);
                index = digits + 2;
                continue;
            }
        }
        plain.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&plain).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::ChannelConnection;

    fn timeout() -> Duration {
        Duration::from_secs(5)
    }

    async fn outbound_file(directory: &Path, name: &str, contents: &[u8]) -> OutboundFile {
        let path = directory.join(name);
        tokio::fs::write(&path, contents).await.unwrap();
        OutboundFile::open(&path).await.unwrap()
    }

    async fn peer_frame(peer: &mut ChannelConnection) -> Frame {
        tokio::time::timeout(timeout(), Frame::read(peer)).await.expect("peer timed out").unwrap()
    }

    /// Check both the restart announcement and the exact remaining bytes; an
    /// unexpected command, truncated file, or extra data fails at the boundary.
    async fn peer_file(peer: &mut ChannelConnection, info: &FileInfo, offset: u64, contents: &[u8]) {
        assert_eq!(peer_frame(peer).await, Frame::command(BinkpCommand::File, info.to_argument(Some(offset))));
        let expected = &contents[offset as usize..];
        let mut received = Vec::new();
        while received.len() < expected.len() {
            let Frame::Data(data) = peer_frame(peer).await else {
                panic!("expected file data");
            };
            received.extend(data);
            assert!(received.len() <= expected.len(), "sender overran the file");
        }
        assert_eq!(received, expected);
    }

    #[tokio::test]
    async fn test_late_get_after_eob_restarts_an_unacknowledged_file() {
        // The small case reproduces the original race; the multi-block case
        // also catches premature termination while a retry is still sending.
        for contents in [b"0123456789".to_vec(), vec![0x5a; DATA_BLOCK_SIZE * 2 + 17]] {
            for offset in [0, 3] {
                let directory = tempfile::tempdir().unwrap();
                let inbound = directory.path().join("in");
                let file = outbound_file(directory.path(), "mail.su0", &contents).await;
                let (mut ours, mut peer) = ChannelConnection::create_pair();
                let (result, ()) = tokio::join!(transfer_batch(&mut ours, vec![file.clone()], &inbound, timeout()), async {
                    Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
                    peer_file(&mut peer, &file.info, 0, &contents).await;
                    assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::Eob, ""));
                    // No sleeps: the request is sent only after the actual EOB.
                    Frame::command(BinkpCommand::Get, file.info.to_argument(Some(offset)))
                        .send(&mut peer)
                        .await
                        .unwrap();
                    peer_file(&mut peer, &file.info, offset, &contents).await;
                    assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::Eob, ""));
                    Frame::command(BinkpCommand::Got, file.info.to_argument(None)).send(&mut peer).await.unwrap();
                });
                assert_eq!(
                    result.unwrap(),
                    BatchResult {
                        sent: vec![file.path],
                        ..BatchResult::default()
                    }
                );
            }
        }
    }

    #[tokio::test]
    async fn test_get_at_eof_after_eob_finalizes_without_another_ack() {
        for contents in [b"".as_slice(), b"complete".as_slice()] {
            let directory = tempfile::tempdir().unwrap();
            let inbound = directory.path().join("in");
            let file = outbound_file(directory.path(), "mail.su0", contents).await;
            let (mut ours, mut peer) = ChannelConnection::create_pair();
            let (result, ()) = tokio::join!(transfer_batch(&mut ours, vec![file.clone()], &inbound, timeout()), async {
                Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
                peer_file(&mut peer, &file.info, 0, contents).await;
                assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::Eob, ""));
                Frame::command(BinkpCommand::Get, file.info.to_argument(Some(file.info.size)))
                    .send(&mut peer)
                    .await
                    .unwrap();
            });
            assert_eq!(result.unwrap().sent, vec![file.path]);
            assert_eq!(FrameReader::new().poll(&mut peer).await.unwrap(), None, "EOF GET must not send more bytes");
        }
    }

    #[tokio::test]
    async fn test_get_for_an_active_file_restarts_at_a_nonzero_offset() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        let contents: Vec<u8> = (0..DATA_BLOCK_SIZE * 2 + 17).map(|index| (index % 251) as u8).collect();
        let file = outbound_file(directory.path(), "mail.su0", &contents).await;
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        // Script one input per transfer turn: offer/send a block, then GET.
        // Preloading makes the active-file state independent of task scheduling.
        Frame::command(BinkpCommand::Nul, "first turn").send(&mut peer).await.unwrap();
        Frame::command(BinkpCommand::Get, file.info.to_argument(Some(7))).send(&mut peer).await.unwrap();
        let (result, ()) = tokio::join!(transfer_batch(&mut ours, vec![file.clone()], &inbound, timeout()), async {
            assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::File, file.info.to_argument(Some(0))));
            assert_eq!(peer_frame(&mut peer).await, Frame::Data(contents[..DATA_BLOCK_SIZE].to_vec()));
            peer_file(&mut peer, &file.info, 7, &contents).await;
            assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::Eob, ""));
            Frame::command(BinkpCommand::Got, file.info.to_argument(None)).send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
        });
        assert_eq!(result.unwrap().sent, vec![file.path]);
    }

    #[tokio::test]
    async fn test_queued_get_preserves_the_active_file_and_updates_the_retry_offset() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        let first_bytes = b"0123456789";
        let active_bytes = vec![0x6b; DATA_BLOCK_SIZE * 4 + 17];
        let first = outbound_file(directory.path(), "first.su0", first_bytes).await;
        let active = outbound_file(directory.path(), "active.su0", &active_bytes).await;
        let last = outbound_file(directory.path(), "last.su0", b"last").await;
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        // Turn 1 finishes first; turn 2 starts active; turns 3/4 request first.
        // The active file must finish intact, not be overwritten or restarted.
        for frame in [
            Frame::command(BinkpCommand::Nul, "first"),
            Frame::command(BinkpCommand::Nul, "active"),
            Frame::command(BinkpCommand::Get, first.info.to_argument(Some(1))),
            Frame::command(BinkpCommand::Get, first.info.to_argument(Some(3))),
        ] {
            frame.send(&mut peer).await.unwrap();
        }
        let (result, ()) = tokio::join!(
            transfer_batch(&mut ours, vec![first.clone(), active.clone(), last.clone()], &inbound, timeout()),
            async {
                peer_file(&mut peer, &first.info, 0, first_bytes).await;
                peer_file(&mut peer, &active.info, 0, &active_bytes).await;
                peer_file(&mut peer, &first.info, 3, first_bytes).await;
                peer_file(&mut peer, &last.info, 0, b"last").await;
                assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::Eob, ""));
                for file in [&first, &active, &last] {
                    Frame::command(BinkpCommand::Got, file.info.to_argument(None)).send(&mut peer).await.unwrap();
                }
                Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            }
        );
        assert_eq!(
            result.unwrap(),
            BatchResult {
                sent: vec![first.path, active.path, last.path],
                ..BatchResult::default()
            }
        );
    }

    #[tokio::test]
    async fn test_queued_retry_can_be_acknowledged_skipped_or_finalized_at_eof() {
        for command in [BinkpCommand::Got, BinkpCommand::Skip, BinkpCommand::Get] {
            let directory = tempfile::tempdir().unwrap();
            let inbound = directory.path().join("in");
            let first = outbound_file(directory.path(), "first.su0", b"first").await;
            let active_bytes = vec![0x6b; DATA_BLOCK_SIZE * 5 + 17];
            let active = outbound_file(directory.path(), "active.su0", &active_bytes).await;
            let (mut ours, mut peer) = ChannelConnection::create_pair();
            let action = Frame::command(command, first.info.to_argument((command == BinkpCommand::Get).then_some(first.info.size)));
            for frame in [
                Frame::command(BinkpCommand::Nul, "first"),
                Frame::command(BinkpCommand::Nul, "active"),
                Frame::command(BinkpCommand::Get, first.info.to_argument(Some(1))),
                action.clone(),
                action,
            ] {
                frame.send(&mut peer).await.unwrap();
            }
            let (result, ()) = tokio::join!(transfer_batch(&mut ours, vec![first.clone(), active.clone()], &inbound, timeout()), async {
                peer_file(&mut peer, &first.info, 0, b"first").await;
                peer_file(&mut peer, &active.info, 0, &active_bytes).await;
                assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::Eob, ""));
                Frame::command(BinkpCommand::Got, active.info.to_argument(None)).send(&mut peer).await.unwrap();
                Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            });
            let result = result.unwrap();
            if command == BinkpCommand::Skip {
                assert_eq!(result.sent, vec![active.path]);
                assert_eq!(result.skipped, vec![first.path]);
            } else {
                assert_eq!(result.sent, vec![first.path, active.path]);
                assert!(result.skipped.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_invalid_file_and_get_offsets_send_err() {
        for command in [BinkpCommand::File, BinkpCommand::Get] {
            for offset in ["", "invalid", "-2", "+1", "1.5", "18446744073709551616", "6", "-1"] {
                if command == BinkpCommand::File && offset == "-1" {
                    continue; // The NR request is tested separately below.
                }
                let directory = tempfile::tempdir().unwrap();
                let inbound = directory.path().join("in");
                let file = outbound_file(directory.path(), "mail.su0", b"hello").await;
                let outbound = if command == BinkpCommand::Get { vec![file.clone()] } else { Vec::new() };
                let argument = format!("{} {offset}", file.info.to_argument(None));
                let (mut ours, mut peer) = ChannelConnection::create_pair();
                let (result, ()) = tokio::join!(transfer_batch(&mut ours, outbound, &inbound, timeout()), async {
                    if command == BinkpCommand::Get {
                        Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
                        peer_file(&mut peer, &file.info, 0, b"hello").await;
                        assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::Eob, ""));
                    }
                    Frame::command(command, &argument).send(&mut peer).await.unwrap();
                    loop {
                        match peer_frame(&mut peer).await {
                            Frame::Command(BinkpCommand::Eob, _) => {}
                            Frame::Command(BinkpCommand::Err, message) => {
                                assert!(message.contains(&command.to_string()));
                                break;
                            }
                            frame => panic!("invalid offset {argument:?} produced {frame:?}"),
                        }
                    }
                });
                let error = result.unwrap_err();
                assert!(
                    matches!(error.downcast_ref::<NetError>(), Some(NetError::BinkpBadArgument(name, args)) if name == &command.to_string() && args == &argument)
                );
                assert_eq!(std::fs::read_dir(&inbound).unwrap().count(), 0);
            }
        }
    }

    #[tokio::test]
    async fn test_receive_overrun_sends_err_without_ack_or_persisting_a_file() {
        for chunks in [vec![b"hello!".to_vec()], vec![b"he".to_vec(), b"llo!".to_vec()]] {
            let directory = tempfile::tempdir().unwrap();
            let inbound = directory.path().join("in");
            let info = FileInfo {
                name: "mail.su0".to_string(),
                size: 5,
                time: 1234,
            };
            let (mut ours, mut peer) = ChannelConnection::create_pair();
            let (result, ()) = tokio::join!(transfer_batch(&mut ours, Vec::new(), &inbound, timeout()), async {
                Frame::command(BinkpCommand::File, info.to_argument(Some(0))).send(&mut peer).await.unwrap();
                for chunk in chunks {
                    Frame::Data(chunk).send(&mut peer).await.unwrap();
                }
                loop {
                    match peer_frame(&mut peer).await {
                        Frame::Command(BinkpCommand::Eob, _) => {}
                        Frame::Command(BinkpCommand::Err, message) => {
                            assert!(message.contains("exceeds advertised size"));
                            break;
                        }
                        frame => panic!("overrun must not be acknowledged: {frame:?}"),
                    }
                }
            });
            assert!(result.unwrap_err().to_string().contains("exceeds advertised size"));
            assert_eq!(
                std::fs::read_dir(&inbound).unwrap().count(),
                0,
                "no complete or partial corrupt file may survive"
            );
            assert_eq!(FrameReader::new().poll(&mut peer).await.unwrap(), None, "no ACK may follow M_ERR");
        }
    }

    #[tokio::test]
    async fn test_nr_and_nonzero_file_offers_request_zero_and_discard_in_flight_data() {
        for (offset, contents) in [
            ("-1", b"hello".as_slice()),
            ("3", b"hello".as_slice()),
            ("5", b"hello".as_slice()),
            ("-1", b"".as_slice()),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let inbound = directory.path().join("in");
            let info = FileInfo {
                name: "mail.su0".to_string(),
                size: contents.len() as u64,
                time: 1234,
            };
            let (mut ours, mut peer) = ChannelConnection::create_pair();
            let (result, ()) = tokio::join!(transfer_batch(&mut ours, Vec::new(), &inbound, timeout()), async {
                Frame::command(BinkpCommand::File, format!("{} {offset}", info.to_argument(None)))
                    .send(&mut peer)
                    .await
                    .unwrap();
                Frame::Data(b"old queued bytes exceed the advertised size".to_vec())
                    .send(&mut peer)
                    .await
                    .unwrap();
                let mut got_request = false;
                let mut got_eob = false;
                while !got_request || !got_eob {
                    match peer_frame(&mut peer).await {
                        Frame::Command(BinkpCommand::Eob, _) => got_eob = true,
                        Frame::Command(BinkpCommand::Get, argument) => {
                            assert_eq!(argument, info.to_argument(Some(0)));
                            got_request = true;
                        }
                        frame => panic!("expected offset negotiation, not {frame:?}"),
                    }
                }
                Frame::command(BinkpCommand::File, info.to_argument(Some(0))).send(&mut peer).await.unwrap();
                if !contents.is_empty() {
                    Frame::Data(contents.to_vec()).send(&mut peer).await.unwrap();
                }
                Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
                assert_eq!(peer_frame(&mut peer).await, Frame::command(BinkpCommand::Got, info.to_argument(None)));
            });
            assert_eq!(result.unwrap().received, vec![inbound.join("mail.su0")]);
            assert_eq!(tokio::fs::read(inbound.join("mail.su0")).await.unwrap(), contents);
            assert_eq!(std::fs::read_dir(&inbound).unwrap().count(), 1);
        }
    }

    #[tokio::test]
    async fn test_skipped_file_data_is_discarded_before_the_next_offer() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        let refused = FileInfo {
            name: "../escaped".to_string(),
            size: 1,
            time: 1234,
        };
        let accepted = FileInfo {
            name: "mail.su0".to_string(),
            size: 5,
            time: 1234,
        };
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        let (result, ()) = tokio::join!(transfer_batch(&mut ours, Vec::new(), &inbound, timeout()), async {
            Frame::command(BinkpCommand::File, refused.to_argument(Some(0))).send(&mut peer).await.unwrap();
            Frame::Data(b"queued before SKIP was seen".to_vec()).send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::File, accepted.to_argument(Some(0))).send(&mut peer).await.unwrap();
            Frame::Data(b"hello".to_vec()).send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            let mut skipped = false;
            let mut ended = false;
            let mut received = false;
            while !skipped || !ended || !received {
                match peer_frame(&mut peer).await {
                    Frame::Command(BinkpCommand::Skip, argument) => {
                        assert_eq!(argument, refused.to_argument(None));
                        skipped = true;
                    }
                    Frame::Command(BinkpCommand::Got, argument) => {
                        assert_eq!(argument, accepted.to_argument(None));
                        received = true;
                    }
                    Frame::Command(BinkpCommand::Eob, _) => ended = true,
                    frame => panic!("unexpected response after SKIP: {frame:?}"),
                }
            }
        });
        assert_eq!(result.unwrap().received, vec![inbound.join("mail.su0")]);
        assert_eq!(tokio::fs::read(inbound.join("mail.su0")).await.unwrap(), b"hello");
        assert!(!directory.path().join("escaped").exists());
    }

    #[test]
    fn test_offset_parser_preserves_u64_range_and_optional_extension_fields() {
        let argument = "mail.su0 18446744073709551615 1234 18446744073709551615 extra";
        for command in [BinkpCommand::File, BinkpCommand::Get] {
            let (info, offset) = FileInfo::parse(argument, command).unwrap();
            assert_eq!(info.size, u64::MAX);
            assert_eq!(offset, Some(u64::MAX));
        }
        for command in [BinkpCommand::Got, BinkpCommand::Skip] {
            assert!(FileInfo::parse("mail.su0 5 1234", command).is_some());
        }
    }

    /// Reads until the batch has ended and the awaited command has arrived, which
    /// is not the same moment: end of batch only means the far side is done sending.
    async fn read_until(peer: &mut ChannelConnection, awaited: BinkpCommand) -> Vec<Frame> {
        let mut seen = Vec::new();
        loop {
            seen.push(Frame::read(peer).await.unwrap());
            let ended = seen.iter().any(|frame| matches!(frame, Frame::Command(BinkpCommand::Eob, _)));
            let arrived = seen.iter().any(|frame| matches!(frame, Frame::Command(command, _) if *command == awaited));
            if ended && arrived {
                return seen;
            }
        }
    }

    /// Plays the far end of a batch: takes everything offered, offers nothing back.
    fn accept_everything(mut peer: ChannelConnection, into: PathBuf) -> tokio::task::JoinHandle<Vec<PathBuf>> {
        tokio::spawn(async move {
            let mut received = Vec::new();
            let mut current: Option<(FileInfo, PathBuf, Vec<u8>)> = None;
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            loop {
                match Frame::read(&mut peer).await.unwrap() {
                    Frame::Command(BinkpCommand::File, argument) => {
                        let (info, _) = FileInfo::parse(&argument, BinkpCommand::File).unwrap();
                        let path = into.join(&info.name);
                        current = Some((info, path, Vec::new()));
                    }
                    Frame::Data(data) => {
                        let Some((info, path, bytes)) = &mut current else { continue };
                        bytes.extend_from_slice(&data);
                        if bytes.len() as u64 >= info.size {
                            tokio::fs::write(&path, &bytes).await.unwrap();
                            Frame::command(BinkpCommand::Got, info.to_argument(None)).send(&mut peer).await.unwrap();
                            received.push(path.clone());
                            current = None;
                        }
                    }
                    Frame::Command(BinkpCommand::Eob, _) => return received,
                    _ => {}
                }
            }
        })
    }

    #[tokio::test]
    async fn test_a_file_arrives_with_the_bytes_it_left_with() {
        let directory = tempfile::tempdir().unwrap();
        let far_side = tempfile::tempdir().unwrap();
        let contents = vec![0x5a; DATA_BLOCK_SIZE * 2 + 17];
        let file = outbound_file(directory.path(), "mail.su0", &contents).await;

        let (mut ours, peer) = ChannelConnection::create_pair();
        let peer = accept_everything(peer, far_side.path().to_path_buf());
        let result = transfer_batch(&mut ours, vec![file.clone()], &directory.path().join("in"), timeout())
            .await
            .unwrap();

        assert_eq!(result.sent, vec![file.path]);
        assert_eq!(peer.await.unwrap(), vec![far_side.path().join("mail.su0")]);
        assert_eq!(tokio::fs::read(far_side.path().join("mail.su0")).await.unwrap(), contents);
    }

    #[tokio::test]
    async fn test_a_batch_with_nothing_to_send_still_ends() {
        let directory = tempfile::tempdir().unwrap();
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            read_until(&mut peer, BinkpCommand::Eob).await;
        });
        let result = transfer_batch(&mut ours, Vec::new(), &directory.path().join("in"), timeout()).await.unwrap();
        assert_eq!(result, BatchResult::default());
    }

    #[tokio::test]
    async fn test_an_offered_file_is_written_into_the_inbound() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            let info = FileInfo {
                name: "mail bundle.su0".to_string(),
                size: 5,
                time: 1234,
            };
            Frame::command(BinkpCommand::File, info.to_argument(Some(0))).send(&mut peer).await.unwrap();
            Frame::Data(b"hello".to_vec()).send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            read_until(&mut peer, BinkpCommand::Got).await;
        });
        let result = transfer_batch(&mut ours, Vec::new(), &inbound, timeout()).await.unwrap();

        assert_eq!(result.received, vec![inbound.join("mail bundle.su0")]);
        assert_eq!(tokio::fs::read(inbound.join("mail bundle.su0")).await.unwrap(), b"hello");
    }

    #[tokio::test]
    async fn test_a_file_that_never_arrived_in_full_leaves_nothing_behind() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            let info = FileInfo {
                name: "mail.su0".to_string(),
                size: 5000,
                time: 1234,
            };
            Frame::command(BinkpCommand::File, info.to_argument(Some(0))).send(&mut peer).await.unwrap();
            Frame::Data(b"half of it".to_vec()).send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            // Hold the connection so the batch has to give up on its own.
            tokio::time::sleep(Duration::from_secs(30)).await;
        });

        let result = transfer_batch(&mut ours, Vec::new(), &inbound, Duration::from_millis(200)).await;

        assert!(result.is_err(), "a file that stopped half way cannot end the batch");
        assert!(!inbound.join("mail.su0").exists(), "half a bundle must not be left for the tosser");
        assert_eq!(std::fs::read_dir(&inbound).unwrap().count(), 0, "and the working file must be gone too");
    }

    /// The name keeps its extension, so what arrives is still read as a bundle.
    #[tokio::test]
    async fn test_a_file_does_not_replace_one_that_is_still_waiting_to_be_tossed() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        std::fs::create_dir_all(&inbound).unwrap();
        std::fs::write(inbound.join("mail.su0"), b"still waiting to be tossed").unwrap();

        let (mut ours, mut peer) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            let info = FileInfo {
                name: "mail.su0".to_string(),
                size: 5,
                time: 1234,
            };
            Frame::command(BinkpCommand::File, info.to_argument(Some(0))).send(&mut peer).await.unwrap();
            Frame::Data(b"fresh".to_vec()).send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            read_until(&mut peer, BinkpCommand::Got).await;
        });

        let result = transfer_batch(&mut ours, Vec::new(), &inbound, timeout()).await.unwrap();

        assert_eq!(std::fs::read(inbound.join("mail.su0")).unwrap(), b"still waiting to be tossed");
        assert_eq!(result.received, vec![inbound.join("mail-1.su0")]);
        assert_eq!(std::fs::read(inbound.join("mail-1.su0")).unwrap(), b"fresh");
    }

    /// A working file is only rubbish once no session can still be writing to it.
    #[tokio::test]
    async fn test_the_working_file_of_a_session_that_was_killed_is_cleared_away() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        std::fs::create_dir_all(&inbound).unwrap();

        let stale = inbound.join("binkp-fromlastweek.tmp");
        std::fs::write(&stale, b"half a bundle").unwrap();
        let times = std::fs::FileTimes::new().set_modified(std::time::SystemTime::now() - Duration::from_secs(48 * 60 * 60));
        std::fs::File::options().write(true).open(&stale).unwrap().set_times(times).unwrap();

        let running = inbound.join("binkp-rightnow.tmp");
        std::fs::write(&running, b"a session may still be writing this").unwrap();
        let waiting = inbound.join("mail.su0");
        std::fs::write(&waiting, b"waiting to be tossed").unwrap();

        let (mut ours, mut peer) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            read_until(&mut peer, BinkpCommand::Eob).await;
        });
        transfer_batch(&mut ours, Vec::new(), &inbound, timeout()).await.unwrap();

        assert!(!stale.exists(), "what a session long gone left behind is rubbish");
        assert!(running.exists(), "what one may still be writing to is not");
        assert!(waiting.exists(), "and mail waiting to be tossed is never touched");
    }

    #[tokio::test]
    async fn test_a_file_the_remote_skipped_is_not_reported_as_sent() {
        let directory = tempfile::tempdir().unwrap();
        let file = outbound_file(directory.path(), "mail.su0", b"whatever").await;
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        tokio::spawn(async move {
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            loop {
                match Frame::read(&mut peer).await.unwrap() {
                    Frame::Command(BinkpCommand::File, argument) => {
                        let (info, _) = FileInfo::parse(&argument, BinkpCommand::File).unwrap();
                        Frame::command(BinkpCommand::Skip, info.to_argument(None)).send(&mut peer).await.unwrap();
                    }
                    Frame::Command(BinkpCommand::Eob, _) => return,
                    _ => {}
                }
            }
        });
        let result = transfer_batch(&mut ours, vec![file.clone()], &directory.path().join("in"), timeout())
            .await
            .unwrap();

        assert!(result.sent.is_empty());
        assert_eq!(result.skipped, vec![file.path]);
    }

    #[tokio::test]
    async fn test_a_name_that_climbs_out_of_the_inbound_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let inbound = directory.path().join("in");
        let (mut ours, mut peer) = ChannelConnection::create_pair();
        let refused = tokio::spawn(async move {
            let info = FileInfo {
                name: "../escaped".to_string(),
                size: 5,
                time: 1234,
            };
            Frame::command(BinkpCommand::File, info.to_argument(Some(0))).send(&mut peer).await.unwrap();
            Frame::command(BinkpCommand::Eob, "").send(&mut peer).await.unwrap();
            read_until(&mut peer, BinkpCommand::Skip).await
        });
        let result = transfer_batch(&mut ours, Vec::new(), &inbound, timeout()).await.unwrap();

        assert!(!refused.await.unwrap().is_empty());
        assert!(result.received.is_empty());
        assert!(!directory.path().join("escaped").exists());
    }

    #[test]
    fn test_a_space_does_not_survive_unescaped_in_a_name() {
        assert_eq!(escape_filename("abcd e.0f@"), "abcd\\x20e.0f@");
        assert_eq!(escape_filename("back\\slash"), "back\\x5cslash");
    }

    #[test]
    fn test_both_spellings_of_an_escape_are_understood() {
        assert_eq!(unescape_filename("abcd\\x20e.0f@"), "abcd e.0f@");
        assert_eq!(unescape_filename("abcd\\20e.0f@"), "abcd e.0f@");
        assert_eq!(unescape_filename("nothing to undo"), "nothing to undo");
    }

    #[test]
    fn test_a_name_that_points_at_another_directory_is_not_a_name() {
        assert_eq!(safe_name("mail.su0"), Some("mail.su0".to_string()));
        assert_eq!(safe_name("../../etc/passwd"), None);
        assert_eq!(safe_name("/etc/passwd"), None);
        assert_eq!(safe_name(".."), None);
        assert_eq!(safe_name(".hidden"), None);
        assert_eq!(safe_name("..\\windows"), None);
    }
}
