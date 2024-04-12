use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Result, Seek, SeekFrom, Write},
    path::Path,
    time::SystemTime,
};

use icy_ppe::Res;

use crate::vm::VMError;

const O_RD: i32 = 0;
const O_RW: i32 = 2;
const O_WR: i32 = 1;
const O_APPEND: i32 = 4;

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

impl DiskIO {
    pub fn fappend(&mut self, channel: usize, file: &str, _am: i32, sm: i32) {
        let _ = self.fopen(channel, file, O_APPEND, sm);
    }

    pub fn fcreate(&mut self, channel: usize, file: &str, _am: i32, sm: i32) {
        let _ = self.fopen(channel, file, O_WR, sm);
    }

    pub fn delete(&mut self, file: &str) -> std::io::Result<()> {
        fs::remove_file(file)
    }

    pub fn rename(&mut self, old: &str, new: &str) -> std::io::Result<()> {
        fs::rename(old, new)
    }

    pub fn copy(&mut self, from: &str, to: &str) -> std::io::Result<()> {
        fs::copy(from, to)?;
        Ok(())
    }

    pub fn fopen(&mut self, channel: usize, file_name: &str, mode: i32, _sm: i32) -> Res<()> {
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

    pub fn ferr(&self, channel: usize) -> bool {
        self.channels[channel].err
    }

    pub fn fput(&mut self, channel: usize, text: String) -> Res<()> {
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

    pub fn fget(&mut self, channel: usize) -> Res<String> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };

        if let Some(f) = chan.file.take() {
            chan.reader = Some(BufReader::new(*f));
        }
        if let Some(reader) = &mut chan.reader {
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                chan.err = true;
                Ok(String::new())
            } else {
                chan.err = false;
                Ok(line.trim_end_matches(|c| c == '\r' || c == '\n').to_string())
            }
        } else {
            log::error!("no file!");
            chan.err = true;
            Ok(String::new())
        }
    }

    pub fn fread(&mut self, channel: usize, size: usize) -> Res<Vec<u8>> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };
        if let Some(f) = chan.file.take() {
            chan.reader = Some(BufReader::new(*f));
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

    pub fn fseek(&mut self, channel: usize, pos: i32, seek_pos: i32) -> Res<()> {
        let Some(chan) = self.channels.get_mut(channel) else {
            return Err(Box::new(VMError::FileChannelNotOpen(channel)));
        };

        match &mut chan.file {
            Some(f) => match seek_pos {
                0 => {
                    f.seek(SeekFrom::Start(pos as u64)).expect("seek error");
                }
                1 => {
                    f.seek(SeekFrom::Current(pos as i64)).expect("seek error");
                }
                2 => {
                    f.seek(SeekFrom::End(-pos as i64)).expect("seek error");
                }
                _ => return Err(Box::new(VMError::InvalidSeekPosition(seek_pos))),
            },
            _ => {
                if let Some(reader) = &mut chan.reader {
                    match seek_pos {
                        0 => {
                            reader.seek(SeekFrom::Start(pos as u64)).expect("seek error");
                        }
                        1 => {
                            reader.seek(SeekFrom::Current(pos as i64)).expect("seek error");
                        }
                        2 => {
                            reader.seek(SeekFrom::End(-pos as i64)).expect("seek error");
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

    pub fn frewind(&mut self, channel: usize) -> Res<()> {
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

    pub fn fclose(&mut self, channel: usize) -> Res<()> {
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

    pub fn file_exists(&self, file: &str) -> bool {
        fs::metadata(file).is_ok()
    }

    pub fn get_file_date(&self, file: &str) -> Result<SystemTime> {
        let metadata = fs::metadata(file)?;
        metadata.accessed()
    }

    pub fn get_file_size(&self, file: &str) -> u64 {
        if let Ok(metadata) = fs::metadata(file) {
            metadata.len()
        } else {
            0
        }
    }
}
