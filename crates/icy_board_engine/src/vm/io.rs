use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Result, Seek, SeekFrom, Write},
    path::Path,
    time::SystemTime,
};

use crate::Res;

use crate::vm::VMError;

const O_RD: i32 = 0;
const O_RW: i32 = 2;
const O_WR: i32 = 1;
const O_APPEND: i32 = 4;

pub trait PCBoardIO: Send {
    /// Open a file for append access
    /// channel - integer expression with the channel to use for the file
    /// file - file name to open
    /// am - desired access mode for the file
    /// sm - desired share mode for the file
    fn fappend(&mut self, channel: usize, file: &str, am: i32, sm: i32);

    /// Creates a new file
    /// channel - integer expression with the channel to use for the file
    /// file - file name to open
    /// am - desired access mode for the file
    /// sm - desired share mode for the file
    fn fcreate(&mut self, channel: usize, file: &str, am: i32, sm: i32);

    /// Opens a new file
    /// channel - integer expression with the channel to use for the file
    /// file - file name to open
    /// am - desired access mode for the file
    /// sm - desired share mode for the file
    /// # Errors
    fn fopen(&mut self, channel: usize, file: &str, am: i32, sm: i32) -> Res<()>;

    /// Dermine if a file error has occured on a channel since last check
    /// channel - integer expression with the channel to use for the file
    /// Returns
    /// True, if an error occured on the specified channel, False otherwise
    fn ferr(&self, channel: usize) -> bool;

    fn fput(&mut self, channel: usize, text: String) -> Res<()>;

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
    fn fget(&mut self, channel: usize) -> Res<String>;

    fn fread(&mut self, channel: usize, size: usize) -> Res<Vec<u8>>;
    fn fwrite(&mut self, channel: usize, data: &[u8]) -> Res<()>;

    fn fseek(&mut self, channel: usize, pos: i32, seek_pos: i32) -> Res<()>;

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
    fn frewind(&mut self, channel: usize) -> Res<()>;

    fn fclose(&mut self, channel: usize) -> Res<()>;

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
}

struct FileChannel {
    file: Option<Box<File>>,
    reader: Option<BufReader<File>>,
    _content: Vec<u8>,
    err: bool,
}

impl FileChannel {
    fn new() -> Self {
        FileChannel {
            file: None,
            reader: None,
            _content: Vec::new(),
            err: false,
        }
    }
}

pub struct DiskIO {
    _path: String, // use that as root
    channels: [FileChannel; 8],
}

impl DiskIO {
    #[must_use]
    pub fn new(path: &str, answer_file: Option<&Path>) -> Self {
        let mut first_chan = FileChannel::new();

        if let Some(answer_file) = answer_file {
            let _ = first_chan.file.replace(Box::new(File::create(answer_file).unwrap()));
        }

        DiskIO {
            _path: path.to_string(),
            channels: [
                first_chan,
                FileChannel::new(),
                FileChannel::new(),
                FileChannel::new(),
                FileChannel::new(),
                FileChannel::new(),
                FileChannel::new(),
                FileChannel::new(),
            ],
        }
    }
}

impl PCBoardIO for DiskIO {
    fn fappend(&mut self, channel: usize, file: &str, _am: i32, sm: i32) {
        let _ = self.fopen(channel, file, O_APPEND, sm);
    }

    fn fcreate(&mut self, channel: usize, file: &str, _am: i32, sm: i32) {
        let _ = self.fopen(channel, file, O_WR, sm);
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

    fn fopen(&mut self, channel: usize, file_name: &str, mode: i32, _sm: i32) -> Res<()> {
        log::info!("FOPEN: {}, {}, mode : {}", channel, file_name, mode);

        let file = match mode {
            O_RD => File::open(file_name),
            O_WR => File::create(file_name),
            O_RW => OpenOptions::new().read(true).write(true).open(file_name),
            O_APPEND => OpenOptions::new().append(true).open(file_name),
            _ => panic!("unsupported mode {mode}"),
        };
        match file {
            Ok(handle) => {
                self.channels[channel] = FileChannel {
                    file: Some(Box::new(handle)),
                    reader: None,
                    _content: Vec::new(),
                    err: false,
                };
            }
            Err(err) => {
                log::error!("error opening file: {}", err);
                return Err(Box::new(VMError::FileNotFound(file_name.to_string())));
            }
        }

        Ok(())
    }

    fn ferr(&self, channel: usize) -> bool {
        self.channels[channel].err
    }

    fn fput(&mut self, channel: usize, text: String) -> Res<()> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };

        if let Some(f) = &mut chan.file {
            let _ = f.write(text.as_bytes());
            chan.err = false;
        } else {
            log::error!("channel {} not found", channel);
            chan.err = true;
        }
        Ok(())
    }

    fn fget(&mut self, channel: usize) -> Res<String> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };

        if let Some(f) = chan.file.take() {
            chan.reader = Some(BufReader::new(*f));
        }
        if let Some(reader) = &mut chan.reader {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(size) => {
                    chan.err = size == 0;
                    Ok(line.trim_end_matches(|c| c == '\r' || c == '\n').to_string())
                }
                Err(err) => {
                    log::error!("error reading line: {}", err);
                    chan.err = true;
                    Ok(String::new())
                }
            }
        } else {
            log::error!("no file!");
            chan.err = true;
            Ok(String::new())
        }
    }

    fn fread(&mut self, channel: usize, size: usize) -> Res<Vec<u8>> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };
        if let Some(f) = &mut chan.file {
            let mut buf = vec![0; size];
            f.read_exact(&mut buf)?;
            return Ok(buf);
        }
        if let Some(reader) = &mut chan.reader {
            let mut buf = vec![0; size];
            reader.read_exact(&mut buf)?;
            Ok(buf)
        } else {
            chan.err = true;
            Ok(Vec::new())
        }
    }
    fn fwrite(&mut self, channel: usize, data: &[u8]) -> Res<()> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };

        if let Some(f) = &mut chan.file {
            let _ = f.write(data);
            chan.err = false;
        } else {
            log::error!("fwrite channel {} not found", channel);
            chan.err = true;
        }
        Ok(())
    }

    fn fseek(&mut self, channel: usize, pos: i32, seek_pos: i32) -> Res<()> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };

        match &mut chan.file {
            Some(f) => match seek_pos {
                0 => {
                    f.seek(SeekFrom::Start(pos as u64))?;
                }
                1 => {
                    f.seek(SeekFrom::Current(pos as i64))?;
                }
                2 => {
                    f.seek(SeekFrom::End(-pos as i64))?;
                }
                _ => return Err(Box::new(VMError::InvalidSeekPosition(seek_pos))),
            },
            _ => {
                if let Some(reader) = &mut chan.reader {
                    match seek_pos {
                        0 => {
                            reader.seek(SeekFrom::Start(pos as u64))?;
                        }
                        1 => {
                            reader.seek(SeekFrom::Current(pos as i64))?;
                        }
                        2 => {
                            reader.seek(SeekFrom::End(-pos as i64))?;
                        }
                        _ => return Err(Box::new(VMError::InvalidSeekPosition(seek_pos))),
                    }
                } else {
                    return Err(Box::new(VMError::FileChannelNotOpen(channel)));
                }
            }
        }

        Ok(())
    }

    fn frewind(&mut self, channel: usize) -> Res<()> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };

        match &mut chan.file {
            Some(f) => {
                f.seek(SeekFrom::Start(0)).expect("seek error");
                chan.err = false;
            }
            _ => {
                chan.err = true;
            }
        }
        Ok(())
    }

    fn fclose(&mut self, channel: usize) -> Res<()> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };

        match &mut chan.file {
            Some(_) => {
                self.channels[channel] = FileChannel {
                    file: None,
                    reader: None,
                    _content: Vec::new(),
                    err: false,
                };
            }
            _ => {
                chan.err = true;
            }
        }
        Ok(())
    }

    fn get_file_date(&self, file: &str) -> Result<SystemTime> {
        let metadata = fs::metadata(file)?;
        metadata.accessed()
    }

    fn get_file_size(&self, file: &str) -> u64 {
        if let Ok(metadata) = fs::metadata(file) {
            metadata.len()
        } else {
            0
        }
    }
}
