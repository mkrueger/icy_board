use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek},
    path::{Path, PathBuf},
};

use bstr::BString;
use thiserror::Error;

use crate::util::basic_real::BasicReal;

use self::{
    base_header::PCBoardMessageBaseHeader,
    message_header::{PCBoardExtendedHeader, PCBoardMessageHeader},
    message_index::PCBoardMessageIndex,
};

mod base_header;
pub mod message_header;
mod message_index;

#[cfg(test)]
mod tests;

const FROM_TO_LEN: usize = 25;
const PASSWORD_LEN: usize = 12;
const DATE_LEN: usize = 8;
const TIME_LEN: usize = 5;

#[derive(Error, Debug)]
pub enum PCBoardError {
    #[error("Message number {0} out of range. Valid range is {1}..={2}")]
    MessageNumberOutOfRange(u32, u32, u32),

    #[error("Unknown extended header: {0}")]
    UnknownExtendedHeader(BString),
}

mod extensions {
    /// filename.JHR - Message header data
    pub const INDEX: &str = "idx";

    /// filename.NDX - Old message index (optional)
    pub const OLD_INDEX: &str = "ndx";
}

/// PCBoard strings contain trailing spaces that need to be removed.
pub(crate) fn convert_pcboard_str(buf: &[u8]) -> BString {
    let mut str = BString::from(buf);
    while str.ends_with(&[b' ']) {
        str.pop();
    }
    str
}

fn _gen_string(str: &str, num: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    for c in str.chars().take(num) {
        buf.push(c as u8);
    }
    while buf.len() < num {
        buf.push(b' ');
    }
    buf
}

pub struct PCBoardMessage {
    pub header: PCBoardMessageHeader,
    pub extended_header: Vec<PCBoardExtendedHeader>,
    pub text: BString,
}

impl PCBoardMessage {
    pub fn read(file: &mut BufReader<File>) -> crate::Result<Self> {
        let header = PCBoardMessageHeader::read(file)?;
        let mut buf = vec![0; 128 * ((header.num_blocks as usize).saturating_sub(1))];
        file.read_exact(&mut buf)?;
        let mut i = 0;

        let mut extended_header = Vec::new();
        while i < buf.len() {
            if buf[i] == 0xFF && buf[i + 1] == 0x40 {
                extended_header.push(PCBoardExtendedHeader::deserialize(&buf[i..])?);
                i += 0x48;
                continue;
            }
            let text = convert_msg(&buf[i..]);
            return Ok(PCBoardMessage { header, extended_header, text });
        }

        Ok(PCBoardMessage {
            header,
            extended_header,
            text: BString::default(),
        })
    }

    pub fn get_status(&self) -> MessageStatus {
        match self.header.status {
            b'*' | b'+' => MessageStatus::Private,
            b'~' | b'`' => MessageStatus::CommentToSysop,
            b'%' | b'^' => MessageStatus::SenderPassword,
            b'!' | b'#' => MessageStatus::GroupPassword,
            b' ' | b'-' => MessageStatus::Public,
            b'$' => MessageStatus::GroupPasswordMessageToAll,
            _ => MessageStatus::Public,
        }
    }

    pub fn is_read(&self) -> bool {
        b"+~`^#-".contains(&self.header.status)
    }

    pub fn is_deleted(&self) -> bool {
        self.header.is_deleted()
    }
}

pub const PCB_TXT_EOL: u8 = 0xE3;
pub const PCB_TXT_EOL_PTR: &[u8] = &[PCB_TXT_EOL];

pub(crate) fn convert_msg(buf: &[u8]) -> BString {
    let mut buf = buf.to_vec();
    for b in &mut buf {
        if *b == PCB_TXT_EOL {
            *b = b'\n';
        }
    }
    BString::from(buf)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    Private,
    CommentToSysop,
    Public,

    SenderPassword,

    GroupPassword,
    GroupPasswordMessageToAll,
}

pub struct PCBoardMessageBase {
    file_name: PathBuf,
    header_info: PCBoardMessageBaseHeader,
}

impl PCBoardMessageBase {
    /// opens an existing message base with base path (without any extension)
    pub fn open<P: AsRef<Path>>(file_name: P) -> crate::Result<Self> {
        let header_info = PCBoardMessageBaseHeader::load(&mut File::open(&file_name)?)?;
        Ok(Self {
            file_name: file_name.as_ref().into(),
            header_info,
        })
    }

    fn write_base_header(&self) -> crate::Result<()> {
        let header_file = OpenOptions::new().write(true).open(&self.file_name)?;
        let mut writer = BufWriter::new(header_file);
        self.header_info.write_header_to(&mut writer)?;
        Ok(())
    }

    pub fn is_locked(&mut self) -> crate::Result<bool> {
        self.header_info = PCBoardMessageBaseHeader::load(&mut File::open(&self.file_name)?)?;
        Ok(self.header_info.is_locked())
    }

    pub fn lock(&mut self) -> crate::Result<()> {
        self.header_info.lock();
        self.write_base_header()
    }

    pub fn unlock(&mut self) -> crate::Result<()> {
        self.header_info.unlock();
        self.write_base_header()
    }

    /// Number of active (not deleted) msgs  
    pub fn active_messages(&self) -> u32 {
        self.header_info.active_msgs
    }

    pub fn highest_message_number(&self) -> u32 {
        self.header_info.high_msg_num
    }

    pub fn lowest_message_number(&self) -> u32 {
        self.header_info.low_msg_num
    }

    pub fn callers(&self) -> u32 {
        self.header_info.callers
    }

    pub fn read_message(&self, num: u32) -> crate::Result<PCBoardMessage> {
        if num < self.lowest_message_number() || num > self.highest_message_number() {
            return Err(PCBoardError::MessageNumberOutOfRange(num, self.lowest_message_number(), self.highest_message_number()).into());
        }
        let idx_file_name = self.file_name.with_extension(extensions::INDEX);
        let mut reader = BufReader::new(File::open(idx_file_name)?);
        reader.seek(std::io::SeekFrom::Start((num as u64 - 1) * PCBoardMessageIndex::HEADER_SIZE as u64))?;
        let header = PCBoardMessageIndex::read(&mut reader)?;
        let mut file = BufReader::new(File::open(&self.file_name)?);
        file.seek(std::io::SeekFrom::Start(header.offset as u64))?;
        PCBoardMessage::read(&mut file)
    }

    pub fn read_old_index(&self) -> crate::Result<Vec<u32>> {
        let old_idx_file_name = self.file_name.with_extension(extensions::OLD_INDEX);

        let mut res = Vec::new();
        let bytes = fs::read(old_idx_file_name)?;

        let mut data = &bytes[..];
        while data.len() >= 4 {
            convert_u32!(num, data);
            if num == 0 {
                break;
            }
            let r: u32 = BasicReal::from(num.to_le_bytes()).into();
            let num = (r - 1) * 128;
            res.push(num);
        }

        Ok(res)
    }

    pub fn read_index(&self) -> crate::Result<Vec<PCBoardMessageIndex>> {
        let idx_file_name = self.file_name.with_extension(extensions::INDEX);

        let mut res = Vec::new();
        let mut reader = BufReader::new(File::open(idx_file_name)?);

        while let Ok(header) = PCBoardMessageIndex::read(&mut reader) {
            res.push(header);
        }

        Ok(res)
    }

    pub fn iter(&self) -> impl Iterator<Item = crate::Result<PCBoardMessage>> {
        let idx_file_name = self.file_name.clone();
        let mut f = File::open(idx_file_name).unwrap();
        let size = f.metadata().unwrap().len();

        f.seek(std::io::SeekFrom::Start(128)).unwrap();
        PCBoardMessageIter {
            reader: BufReader::new(f),
            size,
        }
    }
}

struct PCBoardMessageIter {
    reader: BufReader<File>,
    size: u64,
}

impl Iterator for PCBoardMessageIter {
    type Item = crate::Result<PCBoardMessage>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(pos) = self.reader.stream_position() {
            if pos >= self.size {
                return None;
            }
            Some(PCBoardMessage::read(&mut self.reader))
        } else {
            None
        }
    }
}
