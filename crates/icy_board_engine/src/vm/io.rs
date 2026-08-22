use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, Cursor, Read, Result, Seek, SeekFrom, Write},
    path::Path,
    time::SystemTime,
};

use crate::{Res, executable::PPEExpr, icy_board::read_data_with_encoding_detection, vm::VirtualMachine};

use crate::vm::VMError;

const O_RD: i32 = 0;
const O_WR: i32 = 1;
/// Not a PPL access mode; FAPPEND is a statement of its own in `PCBoard`.
const O_APPEND: i32 = 4;
pub const MAX_FILE_CHANNELS: i32 = 8;

pub trait PCBoardIO: Send {
    /// Open a file for append access
    /// channel - integer expression with the channel to use for the file
    /// file - file name to open
    /// am - desired access mode for the file
    /// sm - desired share mode for the file
    fn fappend(&mut self, channel: i32, file: &str);

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
    /// `PCBoard` cleared `errStat` when FERR was read (EVALP.CPP `TOK_OP_FERR`).
    /// Returns true if an error occured on the specified channel since the last check.
    fn ferr(&mut self, channel: i32) -> bool;

    fn fput(&mut self, channel: i32, text: String) -> Res<()>;

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
    fn fget(&mut self, channel: i32) -> Res<String>;

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

    /// .
    ///
    /// # Examples
    ///
    /// ```
    /// // Example template not implemented for trait functions
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn get_file_date(&self, file: &str) -> Result<SystemTime>;
    fn get_file_size(&self, file: &str) -> u64;

    fn is_open(&self, channel: i32) -> bool;

    /// What the last operation that really failed was, for `ERR()`. Reading to the end
    /// of a file is not one of those, so it is not reported here.
    fn take_failure(&mut self) -> Option<(i32, String)> {
        None
    }
}

struct FileChannel {
    file: Option<Box<File>>,
    reader: Option<Cursor<String>>,
    _content: Vec<u8>,
    err: bool,
    /// Set alongside `err` for a real failure, left alone at end of file.
    failure: Option<String>,
}

impl FileChannel {
    fn new() -> Self {
        FileChannel {
            file: None,
            reader: None,
            _content: Vec::new(),
            err: false,
            failure: None,
        }
    }

    fn fail(&mut self, message: impl Into<String>) {
        self.err = true;
        self.failure = Some(message.into());
    }
}

pub struct DiskIO {
    _path: String, // use that as
    channels: HashMap<i32, FileChannel>,
}

impl DiskIO {
    #[must_use]
    pub fn new(path: &str, answer_file: Option<&Path>) -> Self {
        let mut first_chan = FileChannel::new();

        if let Some(answer_file) = answer_file {
            match File::create(answer_file) {
                Ok(file) => {
                    first_chan.file = Some(Box::new(file));
                }
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
        }
    }

    /// The channel a PPE names, or `None` when nothing is open on it - `PCBoard` remembered
    /// an error flag for every channel and returned instead of ending the PPE.
    fn open_channel(&mut self, channel: i32) -> Option<&mut FileChannel> {
        let chan = self.channels.entry(channel).or_insert_with(FileChannel::new);
        if chan.file.is_none() && chan.reader.is_none() {
            chan.fail(format!("no file open on channel {channel}"));
            return None;
        }
        Some(chan)
    }

    fn set_channel_error(&mut self, channel: i32, message: impl Into<String>) {
        self.channels.entry(channel).or_insert_with(FileChannel::new).fail(message);
    }
}

impl PCBoardIO for DiskIO {
    fn fappend(&mut self, channel: i32, file_name: &str) {
        if let Err(err) = self.fopen(channel, file_name, O_APPEND, 0) {
            log::error!("error appending file: {err}");
        }
    }

    fn fcreate(&mut self, channel: i32, file_name: &str, _am: i32, sm: i32) {
        if let Err(err) = self.fopen(channel, file_name, O_WR, sm) {
            log::error!("error creating file: {err}");
        }
    }

    fn delete(&mut self, file: &str) -> std::io::Result<()> {
        fs::remove_file(file)
    }

    fn rename(&mut self, old: &str, new: &str) -> std::io::Result<()> {
        fs::rename(old, new)
    }
    fn copy(&mut self, from: &str, to: &str) -> std::io::Result<()> {
        fs::copy(from, to)?;
        Ok(())
    }

    fn is_open(&self, channel: i32) -> bool {
        self.channels.get(&channel).is_some_and(|chan| chan.file.is_some() || chan.reader.is_some())
    }

    fn take_failure(&mut self) -> Option<(i32, String)> {
        self.channels
            .iter_mut()
            .find_map(|(channel, chan)| chan.failure.take().map(|message| (*channel, message)))
    }

    fn fopen(&mut self, channel: i32, file_name: &str, mode: i32, _sm: i32) -> Res<()> {
        // PCBoard's openChan set an error flag and carried on - a channel already in use or a
        // file that would not open never stopped a PPE. See SCREXEC.CPP.
        if self.is_open(channel) {
            self.set_channel_error(channel, format!("channel {channel} is already in use"));
            return Ok(());
        }

        // PCBoard masks the access mode to two bits, and dosfopen creates a missing file for any write mode.
        let file = if mode == O_APPEND {
            OpenOptions::new().append(true).create(true).open(file_name)
        } else {
            match mode & 0x03 {
                O_RD => File::open(file_name),
                O_WR => File::create(file_name),
                // Read-write mode: preserve existing content, only create if missing.
                _ => OpenOptions::new().read(true).write(true).create(true).truncate(false).open(file_name),
            }
        };
        match file {
            Ok(handle) => {
                self.channels.insert(
                    channel,
                    FileChannel {
                        file: Some(Box::new(handle)),
                        reader: None,
                        _content: Vec::new(),
                        err: false,
                        failure: None,
                    },
                );
            }
            Err(err) => {
                log::error!("error opening file {file_name}: {err}");
                self.set_channel_error(channel, format!("can't open {file_name}: {err}"));
            }
        }

        Ok(())
    }

    fn ferr(&mut self, channel: i32) -> bool {
        // PCBoard (EVALP.CPP): result = fileArr[c].errStat; fileArr[c].errStat = FALSE;
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

    fn fput(&mut self, channel: i32, text: String) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };

        if let Some(f) = &mut chan.file {
            if let Ok(md) = f.metadata()
                && md.len() == 0
            {
                const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
                let _ = f.write(&UTF8_BOM);
            }
            let _ = f.write(text.as_bytes());
            chan.err = false;
        } else {
            log::error!("channel {channel} not found");
            chan.fail(format!("no file open on channel {channel}"));
        }
        Ok(())
    }

    fn fget(&mut self, channel: i32) -> Res<String> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(String::new());
        };

        if let Some(mut f) = chan.file.take() {
            let mut buf = Vec::new();
            let _ = f.read_to_end(&mut buf);
            match read_data_with_encoding_detection(&buf) {
                Ok(str) => {
                    chan.reader = Some(Cursor::new(str));
                }
                // Whatever a PPE opened, failing to decode it is the channel's error, not the end of the PPE.
                Err(err) => {
                    log::error!("can't decode channel {channel}: {err}");
                    chan.fail(format!("can't decode channel {channel}: {err}"));
                    return Ok(String::new());
                }
            }
        }
        if let Some(reader) = &mut chan.reader {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(size) => {
                    chan.err = size == 0;
                    Ok(line.trim_end_matches(['\r', '\n']).to_string())
                }
                Err(err) => {
                    log::error!("error reading line: {err}");
                    chan.fail(format!("error reading channel {channel}: {err}"));
                    Ok(String::new())
                }
            }
        } else {
            log::error!("no file!");
            chan.fail(format!("no file open on channel {channel}"));
            Ok(String::new())
        }
    }

    fn fread(&mut self, channel: i32, size: usize) -> Res<Vec<u8>> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(Vec::new());
        };
        let mut buf = vec![0; size];
        let read = if let Some(f) = &mut chan.file {
            f.read_exact(&mut buf)
        } else if let Some(reader) = &mut chan.reader {
            reader.read_exact(&mut buf)
        } else {
            chan.fail(format!("no file open on channel {channel}"));
            return Ok(Vec::new());
        };
        if read.is_err() {
            chan.fail(format!("can't read {size} bytes from channel {channel}"));
            return Ok(Vec::new());
        }
        Ok(buf)
    }
    fn fwrite(&mut self, channel: i32, data: &[u8]) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };

        if let Some(f) = &mut chan.file {
            let _ = f.write(data);
            chan.err = false;
        } else {
            log::error!("fwrite channel {channel} not found");
            chan.fail(format!("no file open on channel {channel}"));
        }
        Ok(())
    }

    fn ftell(&mut self, channel: i32) -> Res<u64> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(0);
        };

        match &mut chan.file {
            Some(f) => Ok(f.stream_position().unwrap_or_default()),
            _ => {
                if let Some(reader) = &mut chan.reader {
                    Ok(reader.position())
                } else {
                    chan.fail(format!("no file open on channel {channel}"));
                    Ok(0)
                }
            }
        }
    }

    fn fseek(&mut self, channel: i32, pos: i32, seek_pos: i32) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };

        let seek_to = match seek_pos {
            0 => SeekFrom::Start(pos as u64),
            1 => SeekFrom::Current(pos as i64),
            2 => SeekFrom::End(-pos as i64),
            _ => return Err(Box::new(VMError::InvalidSeekPosition(seek_pos))),
        };
        let sought = match &mut chan.file {
            Some(f) => f.seek(seek_to).map(|_| ()),
            _ => {
                if let Some(reader) = &mut chan.reader {
                    reader.seek(seek_to).map(|_| ())
                } else {
                    chan.fail(format!("no file open on channel {channel}"));
                    return Ok(());
                }
            }
        };
        if sought.is_err() {
            chan.fail(format!("can't seek channel {channel}"));
        }

        Ok(())
    }

    fn frewind(&mut self, channel: i32) -> Res<()> {
        let Some(chan) = self.open_channel(channel) else {
            return Ok(());
        };

        match &mut chan.file {
            Some(f) => {
                if f.seek(SeekFrom::Start(0)).is_err() {
                    chan.fail(format!("can't rewind channel {channel}"));
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
                if f.flush().is_err() {
                    chan.fail(format!("can't flush channel {channel}"));
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
            Some(chan) if chan.file.is_some() || chan.reader.is_some() => {
                chan.file = None;
                chan.reader = None;
                chan.err = false;
            }
            _ => self.set_channel_error(channel, format!("channel {channel} was not open")),
        }

        Ok(())
    }

    fn get_file_date(&self, file: &str) -> Result<SystemTime> {
        let metadata = fs::metadata(file)?;
        metadata.accessed()
    }

    fn get_file_size(&self, file: &str) -> u64 {
        if let Ok(metadata) = fs::metadata(file) { metadata.len() } else { 0 }
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

    /// `PCBoard`'s openChan set the error flag when the file would not open, and the PPE
    /// carried on to look at FERR itself.
    #[test]
    fn test_fopen_o_rd_missing_file_reports_through_ferr() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("missing.dat");
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.fopen(1, path.to_str().unwrap(), 0, 0).unwrap();
        assert!(io.ferr(1));
        // FERR clears the sticky flag on read (EVALP.CPP).
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
        assert_eq!(io.fget(6).unwrap(), "");
        assert_eq!(io.fread(6, 4).unwrap(), Vec::<u8>::new());
        io.fput(6, "x".to_string()).unwrap();
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
        io.fput(0, "answer".to_string()).unwrap();
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
    fn test_fopen_o_append_creates_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new_append.dat");
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.fopen(1, path.to_str().unwrap(), 4, 0).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_fopen_o_append_preserves_existing_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("existing.dat");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"hello").unwrap();
        }
        let mut io = DiskIO::new(tmp.path().to_str().unwrap(), None);
        io.fopen(1, path.to_str().unwrap(), 4, 0).unwrap();
        io.fput(1, " world".to_string()).unwrap();
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
