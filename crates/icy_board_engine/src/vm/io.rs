use std::{
    borrow::Cow,
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Result, Seek, SeekFrom, Write},
    path::Path,
    sync::{Arc, Mutex, Weak},
};

use codepages::tables::UNICODE_TO_CP437;

use crate::{Res, executable::PPEExpr, icy_board::read_data_with_encoding_detection, vm::VirtualMachine};

use crate::vm::VMError;

const O_RD: i32 = 0;
const O_WR: i32 = 1;
const ACCESS_READ: i32 = 1;
const ACCESS_WRITE: i32 = 2;
pub const MAX_FILE_CHANNELS: i32 = 8;
const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const DOS_EOF: u8 = 0x1A;

/// How a channel was opened; `PCBoard`'s FOPEN, FCREATE and FAPPEND only differ in this.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenKind {
    /// Keeps the contents; a missing file is created when the mode allows writing.
    Open,
    /// Creates the file or truncates it.
    Create,
    /// Always read/write, creates a missing file and starts at its end. Writes follow FSEEK.
    Append,
}

pub trait PCBoardIO: Send {
    /// Open a file for append access
    /// channel - integer expression with the channel to use for the file
    /// file - file name to open
    /// am - desired access mode for the file
    /// sm - desired share mode for the file
    fn fappend(&mut self, channel: i32, file: &str);

    fn fappend_with_share(&mut self, channel: i32, file: &str, _share_mode: i32) {
        self.fappend(channel, file);
    }

    /// Creates a new file
    /// channel - integer expression with the channel to use for the file
    /// file - file name to open
    /// am - desired access mode for the file
    /// sm - desired share mode for the file
    fn fcreate(&mut self, channel: i32, file: &str, am: i32, sm: i32);

    /// Opens a new file
    /// channel - integer expression with the channel to use for the file
    /// file - file name to open
    /// am - desired access mode for the file
    /// sm - desired share mode for the file
    /// # Errors
    fn fopen(&mut self, channel: i32, file: &str, am: i32, sm: i32) -> Res<()>;

    /// Determine if a file error has occured on a channel since last check.
    /// channel - integer expression with the channel to use for the file
    ///
    /// `PCBoard` cleared the error when FERR was read.
    /// Returns true if an error occured on the specified channel since the last check.
    fn ferr(&mut self, channel: i32) -> bool;

    /// Writes `text` at the current position without truncating the file.
    ///
    /// The bytes follow the file: a UTF-8 file gets UTF-8, a CP437 file CP437. A new or empty
    /// file is CP437 for legacy PPEs, as in `PCBoard`, and UTF-8 with a BOM when `utf8` (runtime 400).
    fn fput(&mut self, channel: i32, text: &str, utf8: bool) -> Res<()>;

    /// Read a line from an open file
    /// channel - integer expression with the channel to use for the file
    /// # Returns
    /// The line read or "", on error
    ///
    /// # Example
    /// INTEGER i
    /// STRING s
    /// FOPEN 1,"FILE.DAT",ORD,S DW
    /// IF (FERR(1)) THEN
    ///   PRINTLN "Error on opening..."
    ///   END
    /// ENDIF
    ///
    /// FGET 1, s
    /// WHILE (!FERR(1)) DO
    ///   INC i
    ///   PRINTLN "Line ", RIGHT(i, 3), ": ", s
    ///   FGET 1, s
    /// ENDWHILE
    /// FCLOSE 1
    /// `detect_utf8` also reads files without a BOM as UTF-8 when they are valid UTF-8 (runtime 400).
    ///
    /// CR, LF and CRLF end a line, and a DOS EOF marker ends the text. `PCBoard` only knew CR
    /// and also flagged a last line without a terminator; here only the end of the file does.
    fn fget(&mut self, channel: i32, detect_utf8: bool) -> Res<String>;

    /// Reads up to `size` bytes. A short read returns what was there and sets the error flag.
    fn fread(&mut self, channel: i32, size: usize) -> Res<Vec<u8>>;
    fn fwrite(&mut self, channel: i32, data: &[u8]) -> Res<()>;

    fn fseek(&mut self, channel: i32, pos: i32, seek_pos: i32) -> Res<()>;

    fn ftell(&mut self, channel: i32) -> Res<u64>;

    fn fflush(&mut self, channel: i32) -> Res<()>;

    /// channel - integer expression with the channel to use for the file
    /// #Example
    /// STRING s
    /// FAPPEND `1,"C:\PCB\MAIN\PPE.LOG",O_RW,S_DN`
    /// FPUTLN 1, `U_NAME`()
    /// FREWIND 1
    /// WHILE (!FERR(1)) DO
    /// FGET 1,s
    /// PRINTLN s
    /// ENDWHILE
    /// FCLOSE 1
    fn frewind(&mut self, channel: i32) -> Res<()>;

    fn fclose(&mut self, channel: i32) -> Res<()>;

    fn close_all(&mut self) {
        for channel in 0..MAX_FILE_CHANNELS {
            if self.is_open(channel) {
                let _ = self.fclose(channel);
            }
        }
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn delete(&mut self, file: &str) -> std::io::Result<()>;

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn rename(&mut self, old: &str, new: &str) -> std::io::Result<()>;

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn copy(&mut self, from: &str, to: &str) -> std::io::Result<()>;

    fn is_open(&self, channel: i32) -> bool;

    /// Records a structured-data failure on an otherwise usable file channel.
    fn record_error(&mut self, channel: i32, message: String);

    /// What the last fallible operation did. EOF does not count as an operation result.
    fn take_operation_result(&mut self) -> Option<std::result::Result<(), (i32, String)>> {
        None
    }
}

struct FileShare {
    identity: same_file::Handle,
    access: i32,
    denied: i32,
}

static FILE_SHARES: Mutex<Vec<Weak<FileShare>>> = Mutex::new(Vec::new());

/// Opens `file_name` and registers its share. `mode` is the PPL access mode (`O_RD`, `O_WR`,
/// `O_RW`); FAPPEND ignores it. Returns the handle, its share and whether it may be read and written.
fn open_shared(file_name: &Path, kind: OpenKind, mode: i32, share_mode: i32) -> Result<(File, Arc<FileShare>, bool, bool)> {
    let access = if kind == OpenKind::Append {
        ACCESS_READ | ACCESS_WRITE
    } else {
        match mode & 0x03 {
            O_RD => ACCESS_READ,
            O_WR => ACCESS_WRITE,
            _ => ACCESS_READ | ACCESS_WRITE,
        }
    };
    let readable = access & ACCESS_READ != 0;
    let writable = access & ACCESS_WRITE != 0;
    // Truncating needs a writable handle even for `FCREATE ..., O_RD`; the channel stays read-only.
    let needs_write = writable || kind == OpenKind::Create;
    let open = |read: bool| {
        OpenOptions::new()
            .read(read)
            .write(needs_write)
            .create(needs_write)
            .truncate(false)
            .open(file_name)
    };
    // A write-only channel still reads its own file to match the text encoding, where allowed.
    let file = match open(true) {
        Err(err) if !readable && err.kind() == std::io::ErrorKind::PermissionDenied => open(false)?,
        result => result?,
    };
    let share = Arc::new(FileShare {
        identity: same_file::Handle::from_file(file.try_clone()?)?,
        access,
        denied: share_mode & 0x03,
    });
    let mut shares = FILE_SHARES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    shares.retain(|share| share.strong_count() > 0);
    if shares
        .iter()
        .filter_map(Weak::upgrade)
        .any(|existing| existing.identity == share.identity && (existing.access & share.denied != 0 || share.access & existing.denied != 0))
    {
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "file sharing conflict"));
    }
    shares.push(Arc::downgrade(&share));
    drop(shares);
    let mut file = file;
    match kind {
        OpenKind::Create => file.set_len(0)?,
        OpenKind::Append => {
            file.seek(SeekFrom::End(0))?;
        }
        OpenKind::Open => {}
    }
    Ok((file, share, readable, writable))
}

#[derive(Clone, Copy)]
enum TextEncoding {
    Cp437,
    Utf8,
    Utf8Bom,
}

/// Detects the text encoding from the start of the file, or `None` when the file is empty.
fn detect_text_encoding(file: &mut BufReader<File>, detect_utf8: bool) -> Result<Option<TextEncoding>> {
    let position = file.stream_position()?;
    file.seek(SeekFrom::Start(0))?;
    let mut data = Vec::new();
    let read = if detect_utf8 {
        file.read_to_end(&mut data)
    } else {
        Read::by_ref(file).take(UTF8_BOM.len() as u64).read_to_end(&mut data)
    };
    let restore = file.seek(SeekFrom::Start(position));
    read?;
    restore?;
    if data.is_empty() {
        Ok(None)
    } else if data.starts_with(&UTF8_BOM) {
        Ok(Some(TextEncoding::Utf8Bom))
    } else {
        let end = data.iter().position(|&byte| byte == 0 || byte == DOS_EOF).unwrap_or(data.len());
        if detect_utf8 && std::str::from_utf8(&data[..end]).is_ok() {
            Ok(Some(TextEncoding::Utf8))
        } else {
            Ok(Some(TextEncoding::Cp437))
        }
    }
}

/// Reads one line without its terminator. CR, LF and CRLF end a line; the DOS EOF marker
/// ends the text of a non-BOM file and is not consumed. Returns whether a terminator was found.
fn read_text_line(file: &mut BufReader<File>, encoding: TextEncoding) -> Result<(Vec<u8>, bool)> {
    let dos_eof = !matches!(encoding, TextEncoding::Utf8Bom);
    let mut line = Vec::new();
    loop {
        let buffer = file.fill_buf()?;
        if buffer.is_empty() {
            return Ok((line, false));
        }
        let Some(index) = buffer.iter().position(|&byte| byte == b'\r' || byte == b'\n' || (dos_eof && byte == DOS_EOF)) else {
            let count = buffer.len();
            line.extend_from_slice(buffer);
            file.consume(count);
            continue;
        };
        let terminator = buffer[index];
        line.extend_from_slice(&buffer[..index]);
        if terminator == DOS_EOF {
            file.consume(index);
            return Ok((line, false));
        }
        file.consume(index + 1);
        if terminator == b'\r' && file.fill_buf()?.first() == Some(&b'\n') {
            file.consume(1);
        }
        return Ok((line, true));
    }
}

fn encode_text(text: &str, encoding: TextEncoding) -> Cow<'_, [u8]> {
    match encoding {
        TextEncoding::Cp437 => Cow::Owned(text.chars().map(|c| UNICODE_TO_CP437.get(&c).copied().unwrap_or(b'?')).collect()),
        TextEncoding::Utf8 | TextEncoding::Utf8Bom => Cow::Borrowed(text.as_bytes()),
    }
}

/// Moves the OS file position back to the logical one before writing, dropping read-ahead.
fn discard_read_ahead(file: &mut BufReader<File>) -> Result<()> {
    if !file.buffer().is_empty() {
        // Unlike `stream_position`, seeking drops the buffer.
        #[allow(clippy::seek_from_current)]
        file.seek(SeekFrom::Current(0))?;
    }
    Ok(())
}

struct FileChannel {
    file: Option<BufReader<File>>,
    readable: bool,
    writable: bool,
    /// FAPPEND writes go to the current end until the PPE positions the channel itself,
    /// so two nodes appending to one log do not overwrite each other.
    append_at_end: bool,
    /// The text encoding last detected, keyed by whether UTF-8 detection was on.
    text_encoding: Option<(bool, TextEncoding)>,
    share: Option<Arc<FileShare>>,
    err: bool,
    /// Set alongside `err` for a real failure, left alone at end of file.
    failure: Option<String>,
}

impl FileChannel {
    fn new() -> Self {
        FileChannel {
            file: None,
            readable: false,
            writable: false,
            append_at_end: false,
            text_encoding: None,
            share: None,
            err: false,
            failure: None,
        }
    }

    fn opened(file: File, share: Arc<FileShare>, readable: bool, writable: bool, append_at_end: bool) -> Self {
        FileChannel {
            file: Some(BufReader::new(file)),
            readable,
            writable,
            append_at_end,
            share: Some(share),
            ..Self::new()
        }
    }

    fn fail(&mut self, message: impl Into<String>) {
        self.err = true;
        self.failure = Some(message.into());
    }

    /// The encoding of the file's text, or `None` while the file is empty.
    fn text_encoding(&mut self, detect_utf8: bool) -> Result<Option<TextEncoding>> {
        if let Some((detected_with, encoding)) = self.text_encoding
            && detected_with == detect_utf8
        {
            return Ok(Some(encoding));
        }
        let Some(file) = &mut self.file else {
            return Err(std::io::Error::other("no file open"));
        };
        let encoding = detect_text_encoding(file, detect_utf8)?;
        self.text_encoding = encoding.map(|encoding| (detect_utf8, encoding));
        Ok(encoding)
    }

    /// The handle positioned for the next write.
    fn write_handle(&mut self) -> Result<&mut File> {
        let Some(file) = &mut self.file else {
            return Err(std::io::Error::other("no file open"));
        };
        if self.append_at_end {
            file.seek(SeekFrom::End(0))?;
        } else {
            discard_read_ahead(file)?;
        }
        Ok(file.get_mut())
    }

    // Keep encoding detection, EOF positioning and writing together across nodes and processes.
    fn with_append_lock(&mut self, write: impl FnOnce(&mut Self) -> Result<()>) -> Result<()> {
        if !self.append_at_end {
            return write(self);
        }
        let Some(file) = &self.file else {
            return Err(std::io::Error::other("no file open"));
        };
        let lock = file.get_ref().try_clone()?;
        lock.lock()?;
        let result = write(self);
        if let Err(err) = lock.unlock() {
            return Err(std::io::Error::other(format!("can't unlock append file: {err}; write result: {result:?}")));
        }
        result
    }

    fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.with_append_lock(|channel| channel.write_handle()?.write_all(data))
    }

    fn write_text(&mut self, text: &str, utf8: bool) -> Result<()> {
        self.with_append_lock(|channel| channel.write_text_unlocked(text, utf8))
    }

    fn write_text_unlocked(&mut self, text: &str, utf8: bool) -> Result<()> {
        let detected = match self.text_encoding(utf8) {
            Ok(detected) => detected,
            // A write-only handle the OS would not let us read: assume the file matches the PPE.
            Err(_) if !self.readable => Some(if utf8 { TextEncoding::Utf8 } else { TextEncoding::Cp437 }),
            Err(err) => return Err(err),
        };
        let (encoding, write_bom) = match detected {
            Some(encoding) => (encoding, false),
            None => (if utf8 { TextEncoding::Utf8Bom } else { TextEncoding::Cp437 }, utf8),
        };
        self.text_encoding = Some((utf8, encoding));
        let file = self.write_handle()?;
        if write_bom {
            file.write_all(&UTF8_BOM)?;
        }
        file.write_all(&encode_text(text, encoding))
    }
}

pub struct DiskIO {
    _path: String, // use that as
    channels: HashMap<i32, FileChannel>,
    operation_succeeded: bool,
    operation_failure: Option<(i32, String)>,
}

impl DiskIO {
    #[must_use]
    pub fn new(path: &str, answer_file: Option<&Path>) -> Self {
        let mut first_chan = FileChannel::new();

        if let Some(answer_file) = answer_file {
            match open_shared(answer_file, OpenKind::Create, O_WR, 0) {
                Ok((file, share, readable, writable)) => first_chan = FileChannel::opened(file, share, readable, writable, false),
                // A PPE that cannot record its answers still runs; channel 0 reports the error.
                Err(err) => {
                    log::error!("Can't create answer file {}: {err}", answer_file.display());
                    first_chan.fail(format!("can't create answer file: {err}"));
                }
            }
        }
        let mut channels = HashMap::new();
        channels.insert(0, first_chan);

        DiskIO {
            _path: path.to_string(),
            channels,
            operation_succeeded: false,
            operation_failure: None,
        }
    }

    /// The channel a PPE names, or `None` when nothing is open on it - `PCBoard` remembered
    /// an error flag for every channel and returned instead of ending the PPE.
    fn open_channel(&mut self, channel: i32) -> Option<&mut FileChannel> {
        let chan = self.channels.entry(channel).or_insert_with(FileChannel::new);
        if chan.file.is_none() {
            chan.fail(format!("no file open on channel {channel}"));
            return None;
        }
        Some(chan)
    }

    /// Like `open_channel`, but also fails a channel that was opened write-only.
    fn readable_channel(&mut self, channel: i32) -> Option<&mut FileChannel> {
        let chan = self.open_channel(channel)?;
        if !chan.readable {
            chan.fail(format!("channel {channel} is not open for reading"));
            return None;
        }
        Some(chan)
    }

    fn set_channel_error(&mut self, channel: i32, message: impl Into<String>) {
        self.channels.entry(channel).or_insert_with(FileChannel::new).fail(message);
    }

    fn open_file(&mut self, channel: i32, file_name: &str, kind: OpenKind, mode: i32, share_mode: i32) {
        // PCBoard set an error flag and carried on - a channel already in use or a
        // file that would not open never stopped a PPE.
        if self.is_open(channel) {
            self.set_channel_error(channel, format!("channel {channel} is already in use"));
            return;
        }
        match open_shared(Path::new(file_name), kind, mode, share_mode) {
            Ok((file, share, readable, writable)) => {
                let append_at_end = kind == OpenKind::Append;
                self.channels
                    .insert(channel, FileChannel::opened(file, share, readable, writable, append_at_end));
                self.operation_succeeded = true;
            }
            Err(err) => {
                log::error!("error opening file {file_name}: {err}");
                self.set_channel_error(channel, format!("can't open {file_name}: {err}"));
            }
        }
    }
}

impl PCBoardIO for DiskIO {
    fn fappend(&mut self, channel: i32, file_name: &str) {
        self.fappend_with_share(channel, file_name, 0);
    }

    fn fappend_with_share(&mut self, channel: i32, file_name: &str, share_mode: i32) {
        self.open_file(channel, file_name, OpenKind::Append, O_RD, share_mode);
    }

    fn close_all(&mut self) {
        self.channels.clear();
        self.operation_succeeded = false;
        self.operation_failure = None;
    }

    fn fcreate(&mut self, channel: i32, file_name: &str, am: i32, sm: i32) {
        self.open_file(channel, file_name, OpenKind::Create, am, sm);
    }

    fn delete(&mut self, file: &str) -> std::io::Result<()> {
        match fs::remove_file(file) {
            Ok(()) => {
                self.operation_succeeded = true;
                Ok(())
            }
            Err(err) => {
                self.operation_failure = Some((-1, format!("can't delete {file}: {err}")));
                Err(err)
            }
        }
    }

    fn rename(&mut self, old: &str, new: &str) -> std::io::Result<()> {
        match fs::rename(old, new) {
            Ok(()) => {
                self.operation_succeeded = true;
                Ok(())
            }
            Err(err) => {
                self.operation_failure = Some((-1, format!("can't rename {old} to {new}: {err}")));
                Err(err)
            }
        }
    }
    fn copy(&mut self, from: &str, to: &str) -> std::io::Result<()> {
        match fs::copy(from, to) {
            Ok(_) => {
                self.operation_succeeded = true;
                Ok(())
            }
            Err(err) => {
                self.operation_failure = Some((-1, format!("can't copy {from} to {to}: {err}")));
                Err(err)
            }
        }
    }

    fn is_open(&self, channel: i32) -> bool {
        self.channels.get(&channel).is_some_and(|chan| chan.file.is_some())
    }

    fn record_error(&mut self, channel: i32, message: String) {
        let channel = self.channels.entry(channel).or_insert_with(FileChannel::new);
        channel.err = true;
        log::error!("structured record error on channel: {message}");
    }

    fn take_operation_result(&mut self) -> Option<std::result::Result<(), (i32, String)>> {
        let failure = self.operation_failure.take().or_else(|| {
            self.channels
                .iter_mut()
                .find_map(|(channel, chan)| chan.failure.take().map(|message| (*channel, message)))
        });
        let succeeded = std::mem::take(&mut self.operation_succeeded);
        failure.map_or_else(|| succeeded.then_some(Ok(())), |failure| Some(Err(failure)))
    }

    fn fopen(&mut self, channel: i32, file_name: &str, mode: i32, sm: i32) -> Res<()> {
        self.open_file(channel, file_name, OpenKind::Open, mode, sm);
        Ok(())
    }

    fn ferr(&mut self, channel: i32) -> bool {
        // Reading FERR clears the sticky error. A channel that was never opened has no
        // sticky error yet — PCBoard's array starts false — but any prior failed op set it.
        if let Some(chan) = self.channels.get_mut(&channel) {
            let err = chan.err;
            chan.err = false;
            err
        } else {
            // No slot yet: nothing has set an error on this channel.
            false
        }
    }

    fn fput(&mut self, channel: i32, text: &str, utf8: bool) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };
        if !chan.writable {
            chan.fail(format!("channel {channel} is not open for writing"));
            return Ok(());
        }
        if text.is_empty() {
            chan.err = false;
            self.operation_succeeded = true;
            return Ok(());
        }
        if let Err(err) = chan.write_text(text, utf8) {
            chan.fail(format!("can't write channel {channel}: {err}"));
        } else {
            chan.err = false;
            self.operation_succeeded = true;
        }
        Ok(())
    }

    fn fget(&mut self, channel: i32, detect_utf8: bool) -> Res<String> {
        let Some(chan) = self.readable_channel(channel) else {
            return Ok(String::new());
        };

        let read_result = (|| -> Res<(String, bool)> {
            let encoding = chan.text_encoding(detect_utf8)?.unwrap_or(TextEncoding::Cp437);
            let Some(file) = &mut chan.file else {
                return Err(format!("no file open on channel {channel}").into());
            };
            let position = file.stream_position()?;
            let (line, terminated) = read_text_line(file, encoding)?;
            let bytes = if position == 0 { icy_board_ppl::io::strip_utf8_bom(&line) } else { &line };
            let decoded = match encoding {
                TextEncoding::Cp437 => read_data_with_encoding_detection(bytes)?,
                TextEncoding::Utf8 | TextEncoding::Utf8Bom => String::from_utf8_lossy(bytes).into_owned(),
            };
            Ok((decoded, terminated || !line.is_empty()))
        })();
        let (result, succeeded) = match read_result {
            Ok((line, read_something)) => {
                // PCBoard also flagged a last line without CR/LF, which made the usual
                // `WHILE (!FERR)` loop drop it; only the end of the file sets the flag here.
                chan.err = !read_something;
                (line, read_something)
            }
            Err(err) => {
                log::error!("error reading channel {channel}: {err}");
                chan.fail(format!("can't read channel {channel}: {err}"));
                (String::new(), false)
            }
        };
        self.operation_succeeded |= succeeded;
        Ok(result)
    }

    fn fread(&mut self, channel: i32, size: usize) -> Res<Vec<u8>> {
        let Some(chan) = self.readable_channel(channel) else {
            return Ok(Vec::new());
        };
        let Some(file) = &mut chan.file else {
            return Ok(Vec::new());
        };
        let mut buf = Vec::new();
        match Read::by_ref(file).take(size as u64).read_to_end(&mut buf) {
            Ok(read) if read == size => self.operation_succeeded = true,
            // A short read is the end of the file, not a failure.
            Ok(_) => chan.err = true,
            Err(err) => chan.fail(format!("can't read {size} bytes from channel {channel}: {err}")),
        }
        Ok(buf)
    }

    fn fwrite(&mut self, channel: i32, data: &[u8]) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };
        if !chan.writable {
            chan.fail(format!("channel {channel} is not open for writing"));
            return Ok(());
        }
        // Raw bytes can change what the text encoding detection would see.
        chan.text_encoding = None;
        if let Err(err) = chan.write_bytes(data) {
            chan.fail(format!("can't write channel {channel}: {err}"));
        } else {
            chan.err = false;
            self.operation_succeeded = true;
        }
        Ok(())
    }

    fn ftell(&mut self, channel: i32) -> Res<u64> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(0);
        };

        let result = match &mut chan.file {
            Some(f) => match f.stream_position() {
                Ok(position) => Some(position),
                Err(err) => {
                    chan.fail(format!("can't tell channel {channel}: {err}"));
                    None
                }
            },
            _ => {
                chan.fail(format!("no file open on channel {channel}"));
                None
            }
        };
        self.operation_succeeded |= result.is_some();
        Ok(result.unwrap_or_default())
    }

    fn fseek(&mut self, channel: i32, pos: i32, seek_pos: i32) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };

        let seek_to = match seek_pos {
            0 => SeekFrom::Start(pos as u64),
            1 => SeekFrom::Current(pos as i64),
            2 => SeekFrom::End(pos as i64),
            _ => return Err(Box::new(VMError::InvalidSeekPosition(seek_pos))),
        };
        chan.append_at_end = false;
        let sought = match &mut chan.file {
            Some(f) => f.seek(seek_to).map(|_| ()),
            _ => {
                chan.fail(format!("no file open on channel {channel}"));
                return Ok(());
            }
        };
        if let Err(err) = sought {
            chan.fail(format!("can't seek channel {channel}: {err}"));
        } else {
            self.operation_succeeded = true;
        }

        Ok(())
    }

    fn frewind(&mut self, channel: i32) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };

        chan.append_at_end = false;
        match &mut chan.file {
            Some(f) => {
                if let Err(err) = f.seek(SeekFrom::Start(0)) {
                    chan.fail(format!("can't rewind channel {channel}: {err}"));
                } else {
                    self.operation_succeeded = true;
                }
            }
            _ => {
                chan.fail(format!("no file open on channel {channel}"));
            }
        }
        Ok(())
    }

    fn fflush(&mut self, channel: i32) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };

        match &mut chan.file {
            Some(f) => {
                if let Err(err) = f.get_mut().flush() {
                    chan.fail(format!("can't flush channel {channel}: {err}"));
                } else {
                    self.operation_succeeded = true;
                }
            }
            _ => {
                chan.fail(format!("no file open on channel {channel}"));
            }
        }
        Ok(())
    }

    fn fclose(&mut self, channel: i32) -> Res<()> {
        // A channel keeps its place after it was closed, so FERR still answers for it.
        match self.channels.get_mut(&channel) {
            Some(chan) if chan.file.is_some() => {
                chan.file = None;
                chan.readable = false;
                chan.writable = false;
                chan.append_at_end = false;
                chan.text_encoding = None;
                chan.share = None;
                chan.err = false;
                self.operation_succeeded = true;
            }
            _ => self.set_channel_error(channel, format!("channel {channel} was not open")),
        }

        Ok(())
    }
}

pub async fn get_file_channel(vm: &mut VirtualMachine<'_>, args: &[PPEExpr]) -> Res<i32> {
    let channel = vm.eval_expr(&args[0]).await?.as_int();
    Ok(channel % MAX_FILE_CHANNELS)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{DiskIO, PCBoardIO};
    use tempfile::TempDir;

    const SEED: &[u8] = b"12345\r\nABCDE\r\n";

    fn seeded(name: &str) -> (TempDir, String) {
        let root = TempDir::new().unwrap();
        let path = root.path().join(name);
        std::fs::write(&path, SEED).unwrap();
        let path = path.to_str().unwrap().to_string();
        (root, path)
    }

    /// `PCBoard` 15.4/M: FOPEN `O_WR` on an existing file overwrites in place and never truncates.
    #[test]
    fn fopen_for_writing_keeps_the_rest_of_the_file() {
        let (_root, path) = seeded("owr.dat");
        let mut io = DiskIO::new(".", None);
        io.fopen(2, &path, 1, 0).unwrap();
        io.fput(2, "xy", false).unwrap();
        assert!(!io.ferr(2));
        assert_eq!(io.fget(2, false).unwrap(), "");
        assert!(io.ferr(2));
        assert_eq!(io.fread(2, 1).unwrap(), b"");
        assert!(io.ferr(2));
        io.fclose(2).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"xy345\r\nABCDE\r\n");
    }

    /// `PCBoard` 15.4/M: FCREATE truncates and honours `O_RW`, so the channel reads back what it wrote.
    #[test]
    fn fcreate_truncates_and_keeps_its_access_mode() {
        let (_root, path) = seeded("crw.dat");
        let mut io = DiskIO::new(".", None);
        io.fcreate(2, &path, 2, 0);
        io.fput(2, "back\r\n", false).unwrap();
        io.frewind(2).unwrap();
        assert_eq!(io.fget(2, false).unwrap(), "back");
        assert!(!io.ferr(2));
        io.fclose(2).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"back\r\n");

        io.fcreate(2, &path, 0, 0);
        assert!(!io.ferr(2));
        io.fput(2, "denied", false).unwrap();
        assert!(io.ferr(2));
        io.fclose(2).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"");
    }

    /// `PCBoard` 15.4/M opens FAPPEND read/write at the end; FSEEK and FREWIND move reads and writes.
    #[test]
    fn fappend_is_read_write_and_follows_fseek() {
        let (_root, path) = seeded("app.dat");
        let mut io = DiskIO::new(".", None);
        io.fappend(2, &path);
        io.fput(2, "end\r\n", false).unwrap();
        io.frewind(2).unwrap();
        assert_eq!(io.fget(2, false).unwrap(), "12345");
        io.fseek(2, 0, 0).unwrap();
        io.fput(2, "xy", false).unwrap();
        assert!(!io.ferr(2));
        io.fclose(2).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"xy345\r\nABCDE\r\nend\r\n");
    }

    #[test]
    fn file_io_regression_concurrent_appends_keep_every_record() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("append.dat");
        std::fs::write(&path, b"").unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        std::thread::scope(|scope| {
            for writer in 0..8 {
                let path = &path;
                let barrier = barrier.clone();
                scope.spawn(move || {
                    let mut io = DiskIO::new(".", None);
                    io.fappend(1, path.to_str().unwrap());
                    assert!(!io.ferr(1));
                    for record in 0..128 {
                        let text = format!("{writer}:{record:03}{}\n", "x".repeat(58));
                        assert_eq!(text.len(), 64);
                        barrier.wait();
                        if writer % 2 == 0 {
                            io.fput(1, &text, false).unwrap();
                        } else {
                            io.fwrite(1, text.as_bytes()).unwrap();
                        }
                        assert!(!io.ferr(1));
                    }
                    io.fclose(1).unwrap();
                });
            }
        });
        let data = std::fs::read(&path).unwrap();
        assert_eq!(data.len(), 8 * 128 * 64);
        let records: std::collections::HashSet<_> = data.chunks_exact(64).collect();
        assert_eq!(records.len(), 8 * 128);
        for writer in 0..8 {
            for record in 0..128 {
                let text = format!("{writer}:{record:03}{}\n", "x".repeat(58));
                assert!(records.contains(text.as_bytes()));
            }
        }
    }

    #[test]
    fn file_io_regression_concurrent_utf8_appends_write_one_bom() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("append.dat");
        std::fs::write(&path, b"").unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        std::thread::scope(|scope| {
            for writer in 0..8 {
                let path = &path;
                let barrier = barrier.clone();
                scope.spawn(move || {
                    let mut io = DiskIO::new(".", None);
                    io.fappend(1, path.to_str().unwrap());
                    assert!(!io.ferr(1));
                    barrier.wait();
                    io.fput(1, &format!("{writer}\n"), true).unwrap();
                    assert!(!io.ferr(1));
                });
            }
        });
        let data = std::fs::read(&path).unwrap();
        assert!(data.starts_with(&super::UTF8_BOM));
        assert_eq!(data.len(), 3 + 8 * 2);
        let records: std::collections::HashSet<_> = data[3..].chunks_exact(2).collect();
        for writer in 0..8 {
            assert!(records.contains(format!("{writer}\n").as_bytes()));
        }
    }

    #[test]
    fn file_io_regression_appends_across_processes() {
        const CHILD_PATH: &str = "ICY_BOARD_APPEND_TEST_PATH";
        const CHILD_WRITER: &str = "ICY_BOARD_APPEND_TEST_WRITER";
        if let Some(path) = std::env::var_os(CHILD_PATH) {
            let writer: usize = std::env::var(CHILD_WRITER).unwrap().parse().unwrap();
            let mut io = DiskIO::new(".", None);
            io.fappend(1, path.to_str().unwrap());
            assert!(!io.ferr(1));
            for record in 0..128 {
                let text = format!("{writer}:{record:03}{}\n", "x".repeat(58));
                if writer % 2 == 0 {
                    io.fput(1, &text, true).unwrap();
                } else {
                    io.fwrite(1, text.as_bytes()).unwrap();
                }
                assert!(!io.ferr(1));
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            return;
        }

        let root = TempDir::new().unwrap();
        let path = root.path().join("append.dat");
        std::fs::write(&path, b"seed").unwrap();
        let children: Vec<_> = (0..4)
            .map(|writer| {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "vm::io::tests::file_io_regression_appends_across_processes", "--test-threads=1"])
                    .env(CHILD_PATH, &path)
                    .env(CHILD_WRITER, writer.to_string())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect();
        for child in children {
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "append child failed: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let data = std::fs::read(&path).unwrap();
        assert!(data.starts_with(b"seed"));
        assert_eq!(data.len(), 4 + 4 * 128 * 64);
        let records: std::collections::HashSet<_> = data[4..].chunks_exact(64).collect();
        assert_eq!(records.len(), 4 * 128);
        for writer in 0..4 {
            for record in 0..128 {
                let text = format!("{writer}:{record:03}{}\n", "x".repeat(58));
                assert!(records.contains(text.as_bytes()));
            }
        }
    }

    /// `PCBoard` 15.4/M: `FSEEK ch, n, SEEK_END` moves to the end plus `n`.
    #[test]
    fn fseek_from_the_end_adds_the_offset() {
        let (_root, path) = seeded("sk.dat");
        let mut io = DiskIO::new(".", None);
        io.fopen(2, &path, 0, 0).unwrap();
        io.fseek(2, -2, 2).unwrap();
        assert_eq!(io.fread(2, 1).unwrap(), b"\r");
        assert!(!io.ferr(2));
        io.fseek(2, 2, 2).unwrap();
        assert!(!io.ferr(2));
        assert_eq!(io.fread(2, 1).unwrap(), b"");
        assert!(io.ferr(2));
    }

    /// CR, LF and CRLF end a line; a LF directly after CR belongs to it, as in `dosfgets`.
    /// `PCBoard` only treats CR as a terminator; LF also counts here so Unix files keep working.
    /// A DOS EOF marker ends the text. A last line without a terminator is returned without
    /// the error flag; the read after it reports the end of the file.
    #[test]
    fn fget_line_ends_and_unterminated_text() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("eol.dat");
        std::fs::write(&path, b"a\nb\rc\r\nd\x1ae\r\n").unwrap();
        let mut io = DiskIO::new(".", None);
        io.fopen(2, path.to_str().unwrap(), 0, 0).unwrap();
        for (text, err) in [("a", false), ("b", false), ("c", false), ("d", false), ("", true), ("", true)] {
            assert_eq!(io.fget(2, false).unwrap(), text);
            assert_eq!(io.ferr(2), err, "{text}");
        }

        std::fs::write(&path, b"one\r\nlast").unwrap();
        io.fclose(2).unwrap();
        io.fopen(2, path.to_str().unwrap(), 0, 0).unwrap();
        assert_eq!(io.fget(2, false).unwrap(), "one");
        assert!(!io.ferr(2));
        assert_eq!(io.fget(2, false).unwrap(), "last");
        assert!(!io.ferr(2));
        assert_eq!(io.fget(2, false).unwrap(), "");
        assert!(io.ferr(2));
    }

    /// A NUL inside a line truncates the text, as a C string did, without stopping the reader.
    #[test]
    fn fget_moves_past_a_nul_byte() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("nul.dat");
        std::fs::write(&path, b"ab\0cd\r\nnext\r\n").unwrap();
        let mut io = DiskIO::new(".", None);
        io.fopen(2, path.to_str().unwrap(), 0, 0).unwrap();
        assert_eq!(io.fget(2, false).unwrap(), "ab");
        assert_eq!(io.fget(2, false).unwrap(), "next");
        assert!(!io.ferr(2));
    }

    #[test]
    fn text_writes_follow_the_encoding_of_the_file() {
        let root = TempDir::new().unwrap();
        let cases: [(&str, Option<&[u8]>, bool, &[u8]); 7] = [
            // PCBoard 15.4/M wrote CP437 without a BOM.
            ("new-legacy", None, false, b"A\x81"),
            ("new-400", None, true, b"\xef\xbb\xbfA\xc3\xbc"),
            ("empty-400", Some(b""), true, b"\xef\xbb\xbfA\xc3\xbc"),
            ("cp437-400", Some(b"\x81\r\n"), true, b"\x81\r\nA\x81"),
            ("utf8-400", Some("ü\r\n".as_bytes()), true, "ü\r\nAü".as_bytes()),
            ("bom-legacy", Some(b"\xef\xbb\xbfx\r\n"), false, "\u{feff}x\r\nAü".as_bytes()),
            ("ascii-legacy", Some(b"x\r\n"), false, b"x\r\nA\x81"),
        ];
        for (name, original, utf8, expected) in cases {
            let path = root.path().join(name);
            if let Some(original) = original {
                std::fs::write(&path, original).unwrap();
            }
            let mut io = DiskIO::new(".", None);
            io.fappend(1, path.to_str().unwrap());
            io.fput(1, "A", utf8).unwrap();
            io.fput(1, "ü", utf8).unwrap();
            assert!(!io.ferr(1), "{name}");
            io.fclose(1).unwrap();
            assert_eq!(std::fs::read(&path).unwrap(), expected, "{name}");
        }
    }

    #[test]
    fn unmappable_characters_become_question_marks_in_cp437_files() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("cp437.dat");
        let mut io = DiskIO::new(".", None);
        io.fcreate(1, path.to_str().unwrap(), 1, 0);
        io.fput(1, "Ł░\r\n", false).unwrap();
        io.fclose(1).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"?\xb0\r\n");
    }

    /// `PCBoard` 15.4/M stored the bytes of a short FREAD, so the caller gets them and the error flag.
    #[test]
    fn fread_returns_the_bytes_of_a_short_read() {
        let (_root, path) = seeded("short.dat");
        let mut io = DiskIO::new(".", None);
        io.fopen(1, &path, 0, 0).unwrap();
        io.fseek(1, -2, 2).unwrap();
        assert_eq!(io.fread(1, 4).unwrap(), b"\r\n");
        assert!(io.ferr(1));
        assert_eq!(io.fread(1, usize::MAX).unwrap(), b"");
        assert!(io.ferr(1));
    }

    #[test]
    fn text_read_rewind_write_matches_pcboard_without_truncating() {
        for original in [
            None,
            Some(b"".as_slice()),
            Some(b"old text\r\nsecond line\r\n"),
            Some(b"old content\r\nsecond line\r\n"),
            Some(b"old\r\nsecond line\r\n"),
        ] {
            let root = TempDir::new().unwrap();
            let path = root.path().join("test.dat");
            if let Some(original) = original {
                std::fs::write(&path, original).unwrap();
            }
            let mut io = DiskIO::new(root.path().to_str().unwrap(), None);
            io.fopen(1, path.to_str().unwrap(), 2, 2).unwrap();
            assert!(!io.ferr(1));
            let _ = io.fget(1, false).unwrap();
            io.frewind(1).unwrap();
            io.fput(1, "new line", false).unwrap();
            assert!(!io.ferr(1));
            io.fclose(1).unwrap();

            let mut expected = original.unwrap_or_default().to_vec();
            if expected.is_empty() {
                expected.extend_from_slice(b"new line");
            } else {
                expected[..8].copy_from_slice(b"new line");
            }
            assert_eq!(std::fs::read(&path).unwrap(), expected, "{original:?}");
        }
    }

    #[test]
    fn text_and_binary_io_share_one_byte_position() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("mixed.dat");
        std::fs::write(&path, b"one\r\ntwo\r\nthree").unwrap();
        let mut io = DiskIO::new(".", None);
        io.fopen(1, path.to_str().unwrap(), 2, 0).unwrap();
        assert_eq!(io.fget(1, false).unwrap(), "one");
        assert_eq!(io.ftell(1).unwrap(), 5);
        assert_eq!(io.fread(1, 3).unwrap(), b"two");
        assert_eq!(io.ftell(1).unwrap(), 8);
        io.fseek(1, 0, 1).unwrap();
        io.fwrite(1, b"!!").unwrap();
        io.fflush(1).unwrap();
        assert!(!io.ferr(1));
        assert_eq!(io.ftell(1).unwrap(), 10);
        assert_eq!(io.fget(1, false).unwrap(), "three");
        io.fseek(1, 5, 0).unwrap();
        assert_eq!(io.fget(1, false).unwrap(), "two!!three");
        io.frewind(1).unwrap();
        assert_eq!(io.fget(1, false).unwrap(), "one");
        io.fseek(1, -5, 2).unwrap();
        assert_eq!(io.fread(1, 5).unwrap(), b"three");
        assert!(!io.ferr(1));
        assert_eq!(std::fs::read(path).unwrap(), b"one\r\ntwo!!three");
    }

    #[test]
    fn text_positions_count_encoded_bytes_and_binary_reads_stay_raw() {
        for (original, first, offset) in [
            (b"\xef\xbb\xbfGr\xc3\xbc\xc3\x9fe\r\nnext".as_slice(), "Grüße", 12),
            (b"Gr\xc3\xbc\xc3\x9fe\r\nnext".as_slice(), "Grüße", 9),
            (b"Gr\x81\xe1e\r\nnext".as_slice(), "Grüße", 7),
        ] {
            let root = TempDir::new().unwrap();
            let path = root.path().join("encoded.dat");
            std::fs::write(&path, original).unwrap();
            let mut io = DiskIO::new(".", None);
            io.fopen(1, path.to_str().unwrap(), 0, 0).unwrap();
            assert_eq!(io.fget(1, true).unwrap(), first);
            assert_eq!(io.ftell(1).unwrap(), offset);
            assert_eq!(io.fread(1, 4).unwrap(), b"next");
            io.frewind(1).unwrap();
            assert_eq!(io.fread(1, original.len()).unwrap(), original);
            assert!(!io.ferr(1));
        }
    }

    #[test]
    fn text_writes_after_reads_use_the_current_position_and_invalidate_read_ahead() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("text.dat");
        std::fs::write(&path, b"one\r\nold text\r\nlast").unwrap();
        let mut io = DiskIO::new(".", None);
        io.fopen(1, path.to_str().unwrap(), 2, 0).unwrap();
        assert_eq!(io.fget(1, true).unwrap(), "one");
        io.fput(1, "new line", true).unwrap();
        assert_eq!(io.ftell(1).unwrap(), 13);
        assert_eq!(io.fget(1, true).unwrap(), "");
        assert!(!io.ferr(1));
        assert_eq!(io.fget(1, true).unwrap(), "last");
        io.fseek(1, 5, 0).unwrap();
        assert_eq!(io.fget(1, true).unwrap(), "new line");
        assert_eq!(std::fs::read(path).unwrap(), b"one\r\nnew line\r\nlast");
    }

    #[test]
    fn text_encoding_detection_is_file_wide_and_dos_eof_leaves_binary_bytes_intact() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("cp437.dat");
        std::fs::write(&path, b"Gr\xc3\xbc\xc3\x9fe\r\n\x81\r\n\x1aSAUCE").unwrap();
        let mut io = DiskIO::new(".", None);
        io.fopen(1, path.to_str().unwrap(), 0, 0).unwrap();
        assert_eq!(io.fget(1, true).unwrap(), "Gr├╝├ƒe");
        assert_eq!(io.fget(1, true).unwrap(), "ü");
        assert_eq!(io.fget(1, true).unwrap(), "");
        assert!(io.ferr(1));
        assert_eq!(io.ftell(1).unwrap(), 12);
        assert_eq!(io.fget(1, true).unwrap(), "");
        assert!(io.ferr(1));
        assert_eq!(io.fread(1, 6).unwrap(), b"\x1aSAUCE");
        io.frewind(1).unwrap();
        assert_eq!(io.fget(1, true).unwrap(), "Gr├╝├ƒe");
        assert!(!io.ferr(1));
    }

    #[test]
    fn failed_writes_on_read_only_channels_leave_them_readable() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("readonly.dat");
        std::fs::write(&path, b"one\r\ntwo").unwrap();
        let mut io = DiskIO::new(".", None);
        io.fopen(1, path.to_str().unwrap(), 0, 0).unwrap();
        assert_eq!(io.fget(1, false).unwrap(), "one");
        io.fput(1, "not written", false).unwrap();
        assert!(io.ferr(1));
        assert!(matches!(io.take_operation_result(), Some(Err((1, _)))));
        assert_eq!(io.ftell(1).unwrap(), 5);
        assert_eq!(io.fget(1, false).unwrap(), "two");
        io.frewind(1).unwrap();
        io.fwrite(1, b"not written").unwrap();
        assert!(io.ferr(1));
        assert!(matches!(io.take_operation_result(), Some(Err((1, _)))));
        assert_eq!(io.fread(1, 3).unwrap(), b"one");
        assert_eq!(std::fs::read(path).unwrap(), b"one\r\ntwo");
    }

    // APFS stores only UTF-8 names, so macOS cannot create the file this needs.
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a6_answer_file_preserves_native_path_and_participates_in_sharing() {
        use std::os::unix::ffi::OsStringExt;
        let root = TempDir::new().unwrap();
        let path = root.path().join(std::ffi::OsString::from_vec(b"answers-\xff".to_vec()));
        let alias = root.path().join("answers");
        let mut answers = DiskIO::new(".", Some(&path));
        assert!(!answers.ferr(0));
        answers.fwrite(0, b"kept").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"kept");
        std::os::unix::fs::symlink(&path, &alias).unwrap();
        let mut other = DiskIO::new(".", None);
        other.fopen(1, alias.to_str().unwrap(), 0, 3).unwrap();
        assert!(other.ferr(1));
        answers.close_all();
        other.fopen(1, alias.to_str().unwrap(), 0, 3).unwrap();
        assert!(!other.ferr(1));
    }

    #[test]
    fn a6_share_lifetime_covers_cached_reads_close_and_drop() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("shared.dat");
        let path = path.to_str().unwrap();
        std::fs::write(path, b"one\ntwo\n").unwrap();
        let mut first = DiskIO::new(".", None);
        let mut second = DiskIO::new(".", None);
        let mut writer = DiskIO::new(".", None);
        first.fopen(1, path, 0, 2).unwrap();
        second.fopen(1, path, 0, 2).unwrap();
        assert!(!first.ferr(1));
        assert!(!second.ferr(1));
        assert_eq!(first.fget(1, false).unwrap(), "one");
        assert_eq!(first.fget(1, false).unwrap(), "two");
        assert_eq!(first.fget(1, false).unwrap(), "");
        assert!(first.ferr(1));
        writer.fcreate(1, path, 1, 0);
        assert!(writer.ferr(1));
        first.fclose(1).unwrap();
        writer.fcreate(1, path, 1, 0);
        assert!(writer.ferr(1));
        drop(second);
        writer.fcreate(1, path, 1, 3);
        assert!(!writer.ferr(1));
        writer.fwrite(1, b"unchanged").unwrap();
        writer.fget(1, false).unwrap();
        assert!(writer.ferr(1));
        first.fopen(1, path, 0, 0).unwrap();
        assert!(first.ferr(1));
        writer.fclose(1).unwrap();
        first.fopen(1, path, 0, 0).unwrap();
        assert!(!first.ferr(1));
        assert_eq!(first.fget(1, false).unwrap(), "unchanged");
    }

    #[test]
    fn a6_share_modes_apply_to_append_and_failed_opens() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("append.dat");
        let path = path.to_str().unwrap();
        let mut owner = DiskIO::new(".", None);
        let mut other = DiskIO::new(".", None);
        owner.fappend_with_share(1, path, 3);
        owner.fwrite(1, b"original").unwrap();
        other.fappend(1, path);
        assert!(other.ferr(1));
        assert!(!other.is_open(1));
        assert_eq!(std::fs::read(path).unwrap(), b"original");
        owner.fclose(1).unwrap();
        // FAPPEND opens read/write like PCBoard's OPEN_RDWR, so concurrent appenders must not deny reading.
        other.fappend_with_share(1, path, 1);
        assert!(!other.ferr(1));
        owner.fappend_with_share(1, path, 1);
        assert!(owner.ferr(1));
        other.fclose(1).unwrap();
        other.fappend_with_share(1, path, 0);
        assert!(!other.ferr(1));
        owner.fappend_with_share(1, path, 0);
        assert!(!owner.ferr(1));
        owner.fwrite(1, b"-a").unwrap();
        other.fwrite(1, b"-b").unwrap();
        owner.fput(1, "-c", false).unwrap();
        other.fopen(2, path, 0, 3).unwrap();
        assert!(other.ferr(2));
        assert_eq!(std::fs::read(path).unwrap(), b"original-a-b-c");
        owner.close_all();
        other.close_all();
        owner.fopen(1, path, 0, 3).unwrap();
        assert!(!owner.ferr(1));
    }

    #[cfg(unix)]
    #[test]
    fn a6_share_modes_follow_symlinks_hardlinks_and_relative_components() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("data");
        std::fs::write(&path, b"preserved").unwrap();
        let symlink = root.path().join("symbolic");
        let hardlink = root.path().join("hard");
        std::os::unix::fs::symlink(&path, &symlink).unwrap();
        std::fs::hard_link(&path, &hardlink).unwrap();
        let mut owner = DiskIO::new(".", None);
        owner.fopen(1, path.to_str().unwrap(), 0, 3).unwrap();
        for alias in [symlink, hardlink, root.path().join(".").join("data")] {
            let mut other = DiskIO::new(".", None);
            other.fcreate(1, alias.to_str().unwrap(), 1, 0);
            assert!(other.ferr(1), "{}", alias.display());
            assert_eq!(std::fs::read(&path).unwrap(), b"preserved");
        }
    }

    #[test]
    fn a6_share_acquisition_is_atomic_between_threads() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("lock");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let handles = (0..8)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut io = DiskIO::new(".", None);
                    barrier.wait();
                    io.fopen(1, path.to_str().unwrap(), 2, 3).unwrap();
                    let acquired = !io.ferr(1);
                    barrier.wait();
                    acquired
                })
            })
            .collect::<Vec<_>>();
        let acquired = handles.into_iter().map(|handle| usize::from(handle.join().unwrap())).sum::<usize>();
        assert_eq!(acquired, 1);
        let mut io = DiskIO::new(".", None);
        io.fopen(1, path.to_str().unwrap(), 2, 3).unwrap();
        assert!(!io.ferr(1));
    }

    #[test]
    fn a6_share_modes_check_both_access_directions_before_truncating() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("shared.dat");
        let path = path.to_str().unwrap();
        for first_mode in 0..=2 {
            for first_share in 0..=3 {
                for second_mode in 0..=2 {
                    for second_share in 0..=3 {
                        std::fs::write(path, b"preserved").unwrap();
                        let mut first = DiskIO::new(root.path().to_str().unwrap(), None);
                        let mut second = DiskIO::new(root.path().to_str().unwrap(), None);
                        first.fopen(1, path, first_mode, first_share).unwrap();
                        assert!(!first.ferr(1));
                        let before = std::fs::read(path).unwrap();
                        let access = |mode| match mode {
                            0 => 1,
                            1 => 2,
                            _ => 3,
                        };
                        let denied = access(first_mode) & second_share != 0 || access(second_mode) & first_share != 0;
                        second.fopen(1, path, second_mode, second_share).unwrap();
                        let context = format!("first={first_mode}/{first_share}, second={second_mode}/{second_share}");
                        assert_eq!(second.ferr(1), denied, "{context}");
                        assert_eq!(second.is_open(1), !denied, "{context}");
                        if denied {
                            assert_eq!(std::fs::read(path).unwrap(), before, "{context}");
                        }
                    }
                }
            }
        }
    }

    /// `PCBoard`'s openChan set the error flag when the file would not open, and the PPE
    /// carried on to look at FERR itself.
    #[test]
    fn test_fopen_o_rd_missing_file_reports_through_ferr() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("missing.dat");
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.fopen(1, path.to_str().unwrap(), 0, 0).unwrap();
        assert!(io.ferr(1));
        // FERR clears the sticky flag on read.
        assert!(!io.ferr(1));
        assert!(!io.is_open(1));
    }

    #[test]
    fn test_ferr_clears_sticky_error() {
        let tmp = TempDir::new().unwrap();
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.frewind(3).unwrap();
        assert!(io.ferr(3));
        assert!(!io.ferr(3));
    }

    /// Every operation on a channel that is not open answers the same way - `PCBoard`
    /// never let one end a PPE.
    #[test]
    fn test_a_closed_channel_only_sets_the_error_flag() {
        let tmp = TempDir::new().unwrap();
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);

        io.frewind(6).unwrap();
        assert!(io.ferr(6));
        assert_eq!(io.fget(6, false).unwrap(), "");
        assert_eq!(io.fread(6, 4).unwrap(), Vec::<u8>::new());
        io.fput(6, "x", false).unwrap();
        io.fwrite(6, b"x").unwrap();
        io.fseek(6, 0, 0).unwrap();
        io.fflush(6).unwrap();
        assert_eq!(io.ftell(6).unwrap(), 0);
    }

    /// An answer file that cannot be created leaves channel 0 in error rather than
    /// taking the whole PPE down with it.
    #[test]
    fn an_answer_file_that_cannot_be_created_only_fails_its_channel() {
        let tmp = TempDir::new().unwrap();
        let unusable = tmp.path().join("no-such-directory").join("answers.txt");

        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), Some(&unusable));

        assert!(io.ferr(0));
        assert!(!io.is_open(0));
        io.fput(0, "answer", false).unwrap();
    }

    /// A file that was read to the end and closed can be reopened on the same channel.
    #[test]
    fn test_a_channel_is_free_again_after_it_was_closed() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("data.dat");
        std::fs::write(&path, b"hello").unwrap();
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);

        io.fopen(6, path.to_str().unwrap(), 0, 0).unwrap();
        assert!(io.is_open(6));
        io.fclose(6).unwrap();
        assert!(!io.is_open(6));
        io.fopen(6, path.to_str().unwrap(), 0, 0).unwrap();
        assert!(!io.ferr(6));
    }

    #[test]
    fn test_fopen_o_wr_creates_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new_wr.dat");
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.fopen(1, path.to_str().unwrap(), 1, 0).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_fopen_o_rw_creates_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new_rw.dat");
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.fopen(1, path.to_str().unwrap(), 2, 0).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_fappend_creates_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new_append.dat");
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.fappend(1, path.to_str().unwrap());
        assert!(!io.ferr(1));
        assert!(path.exists());
    }

    #[test]
    fn test_fappend_preserves_existing_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("existing.dat");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"hello").unwrap();
        }
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.fappend(1, path.to_str().unwrap());
        io.fput(1, " world", false).unwrap();
        io.fflush(1).unwrap();
        io.fclose(1).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello world");
    }

    /// `PCBoard` masks the mode to two bits, so 3 is read/write and 8 is plain read.
    #[test]
    fn test_fopen_masks_the_access_mode_to_two_bits() {
        let tmp = TempDir::new().unwrap();
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);

        let read_write = tmp.path().join("mode3.dat");
        io.fopen(1, read_write.to_str().unwrap(), 3, 0).unwrap();
        assert!(read_write.exists());

        io.fopen(2, tmp.path().join("mode8.dat").to_str().unwrap(), 8, 0).unwrap();
        assert!(io.ferr(2));
    }
}
