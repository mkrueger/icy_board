use std::{
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
    /// The first three fields of M_FILE, M_GOT, M_GET and M_SKIP.
    fn parse(argument: &str) -> Option<(FileInfo, u64)> {
        let mut fields = argument.split_whitespace();
        let name = unescape_filename(fields.next()?);
        let size = fields.next()?.parse().ok()?;
        let time = fields.next()?.parse().ok()?;
        let offset = fields.next().unwrap_or("0").parse().unwrap_or(0);
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
    /// Files the remote acknowledged, which is what makes them safe to delete.
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
        let partial = tempfile::Builder::new().prefix("binkp-").suffix(".tmp").tempfile_in(inbound)?;
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

    let mut reader = FrameReader::new();
    let mut result = BatchResult::default();
    let mut waiting = outbound.into_iter();
    let mut sending: Option<Sending> = None;
    let mut unacknowledged: Vec<OutboundFile> = Vec::new();
    let mut receiving: Option<Receiving> = None;
    let mut sent_eob = false;
    let mut got_eob = false;
    let mut deadline = Instant::now() + timeout;

    loop {
        if got_eob && sent_eob && unacknowledged.is_empty() && receiving.is_none() {
            return Ok(result);
        }
        let mut worked = false;

        // One frame in and one block out per turn, so neither side can starve the other.
        if let Some(frame) = reader.poll(connection).await? {
            worked = true;
            match frame {
                Frame::Data(data) => {
                    if let Some(current) = &mut receiving {
                        current.handle.write_all(&data).await?;
                        current.written += data.len() as u64;
                        if current.written >= current.info.size {
                            // The remote may only be told the file arrived once
                            // it is somewhere the next run will find it.
                            let current = receiving.take().unwrap();
                            let info = current.info.clone();
                            let path = current.store().await?;
                            Frame::command(BinkpCommand::Got, info.to_argument(None)).send(connection).await?;
                            result.received.push(path);
                        }
                    }
                }

                Frame::Command(BinkpCommand::File, argument) => {
                    let Some((info, offset)) = FileInfo::parse(&argument) else {
                        return abort(connection, NetError::BinkpBadArgument("M_FILE".to_string(), argument)).await;
                    };
                    // An unfinished file is dropped rather than mixed with the next one.
                    receiving = None;
                    if offset != 0 {
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
                    let Some((info, _)) = FileInfo::parse(&argument) else {
                        return abort(connection, NetError::BinkpBadArgument("M_GOT".to_string(), argument)).await;
                    };
                    // Arriving mid file this is a destructive skip, so stop sending either way.
                    if sending.as_ref().is_some_and(|current| current.file.info == info) {
                        let current = sending.take().unwrap();
                        result.sent.push(current.file.path);
                    } else if let Some(index) = unacknowledged.iter().position(|file| file.info == info) {
                        result.sent.push(unacknowledged.remove(index).path);
                    }
                }

                Frame::Command(BinkpCommand::Skip, argument) => {
                    let Some((info, _)) = FileInfo::parse(&argument) else {
                        return abort(connection, NetError::BinkpBadArgument("M_SKIP".to_string(), argument)).await;
                    };
                    if sending.as_ref().is_some_and(|current| current.file.info == info) {
                        let current = sending.take().unwrap();
                        result.skipped.push(current.file.path);
                    } else if let Some(index) = unacknowledged.iter().position(|file| file.info == info) {
                        result.skipped.push(unacknowledged.remove(index).path);
                    }
                }

                Frame::Command(BinkpCommand::Get, argument) => {
                    let Some((info, offset)) = FileInfo::parse(&argument) else {
                        return abort(connection, NetError::BinkpBadArgument("M_GET".to_string(), argument)).await;
                    };
                    if let Some(current) = &mut sending
                        && current.file.info == info
                        && offset <= info.size
                    {
                        current.handle.seek(std::io::SeekFrom::Start(offset)).await?;
                        current.offset = offset;
                        Frame::command(BinkpCommand::File, info.to_argument(Some(offset))).send(connection).await?;
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
                match waiting.next() {
                    Some(file) => {
                        let handle = File::open(&file.path).await?;
                        Frame::command(BinkpCommand::File, file.info.to_argument(Some(0))).send(connection).await?;
                        sending = Some(Sending { file, handle, offset: 0 });
                    }
                    None => {
                        Frame::command(BinkpCommand::Eob, "").send(connection).await?;
                        sent_eob = true;
                    }
                }
            }
            if let Some(current) = &mut sending {
                let mut block = vec![0u8; DATA_BLOCK_SIZE];
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

async fn abort(connection: &mut dyn Connection, error: NetError) -> crate::Result<BatchResult> {
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
                        let (info, _) = FileInfo::parse(&argument).unwrap();
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
                        let (info, _) = FileInfo::parse(&argument).unwrap();
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
