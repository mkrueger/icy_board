use std::{
    fs::{self, File},
    io::{BufReader, Seek},
    path::{Path, PathBuf},
};

use bstr::BString;
use thiserror::Error;

use crate::util::basic_real::BasicReal;

use self::{
    control::{Conference, ControlDat},
    qwk_message::QWKMessage,
};

pub mod control;
pub mod qwk_message;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum QwkError {
    #[error("Invalid conference number ({0})")]
    CantParseConferenceNumber(BString),

    #[error("Invalid message number ({0})")]
    CantParseMessageNumbers(BString),

    #[error("Invalid message block number ({0})")]
    CantParseMessageBlockNumber(BString),

    #[error("Message number in mail header invalid.")]
    InvalidMessageNumber,
}

pub struct QwkMessageBase {
    path: PathBuf,
    control_dat: ControlDat,
    is_extended: bool,

    pub index_offset_bug: bool,
}

impl QwkMessageBase {
    pub fn get_bbs_name(&self) -> &BString {
        &self.control_dat.bbs_name
    }

    pub fn get_bbs_city_and_state(&self) -> &BString {
        &self.control_dat.bbs_city_and_state
    }

    pub fn get_bbs_phone_number(&self) -> &BString {
        &self.control_dat.bbs_phone_number
    }

    pub fn get_bbs_sysop_name(&self) -> &BString {
        &self.control_dat.bbs_sysop_name
    }

    pub fn get_bbs_id(&self) -> &BString {
        &self.control_dat.bbs_id
    }

    pub fn get_creation_time(&self) -> &BString {
        &self.control_dat.creation_time
    }

    pub fn get_qmail_user_name(&self) -> &BString {
        &self.control_dat.qmail_user_name
    }

    pub fn get_qmail_menu_name(&self) -> &BString {
        &self.control_dat.qmail_menu_name
    }

    pub fn get_message_count(&self) -> u32 {
        self.control_dat.message_count
    }

    pub fn get_welcome_screen(&self) -> &BString {
        &self.control_dat.welcome_screen
    }

    pub fn get_news_screen(&self) -> &BString {
        &self.control_dat.news_screen
    }

    pub fn get_logoff_screen(&self) -> &BString {
        &self.control_dat.logoff_screen
    }

    /// opens an existing message base with base path (without any extension)
    /// extended flag for setting if it's a qwke base
    /// should be safe to always have this enabled.
    pub fn open<P: AsRef<Path>>(path: P, is_extended: bool) -> crate::Result<Self> {
        let control_dat_path = fs::read(path.as_ref().join("control.dat"))?;
        let control_dat = ControlDat::read(&control_dat_path)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            is_extended,
            control_dat,
            index_offset_bug: false,
        })
    }

    pub fn get_conferences(&self) -> &[Conference] {
        &self.control_dat.conferences
    }

    pub fn read_qwk_index<P: AsRef<Path>>(path: P) -> crate::Result<Vec<u32>> {
        let data = fs::read(path)?;
        Self::convert_qwk_index(data.as_slice())
    }

    pub fn convert_qwk_index(mut data: &[u8]) -> crate::Result<Vec<u32>> {
        let mut res = Vec::with_capacity(data.len() / 5);
        while !data.is_empty() {
            convert_qwk_ndx!(conference, data);
            res.push(BasicReal::from(conference.to_le_bytes()).into());
        }
        Ok(res)
    }

    pub fn read_conference_mail(&self, conference: u16) -> crate::Result<Vec<QWKMessage>> {
        let file_name = format!("{:03}.ndx", conference);
        let index = Self::read_qwk_index(self.path.join(file_name))?;
        let mut res = Vec::with_capacity(index.len());

        let msg_file_name = self.path.join("messages.dat");
        let mut reader = BufReader::new(File::open(msg_file_name)?);
        for block in index {
            // First block is the header block, so we need to subtract 1
            // Unfortunately not all index files require that.
            let mut block = block - 1;

            // easier to remove that way - hopefully there is a way to detect that which I overlooked.
            if self.index_offset_bug {
                block += 1;
            }

            reader.seek(std::io::SeekFrom::Start(block as u64 * 128))?;
            let mail = QWKMessage::read(&mut reader, self.is_extended)?;
            res.push(mail);
        }

        Ok(res)
    }

    pub fn iter(&self) -> impl Iterator<Item = crate::Result<QWKMessage>> {
        let idx_file_name = self.path.join("messages.dat");
        let mut f = File::open(idx_file_name).unwrap();
        let size = f.metadata().unwrap().len();
        f.seek(std::io::SeekFrom::Start(128)).unwrap();
        QWKMessageIter {
            reader: BufReader::new(f),
            size,
        }
    }
}

struct QWKMessageIter {
    reader: BufReader<File>,
    size: u64,
}

impl Iterator for QWKMessageIter {
    type Item = crate::Result<QWKMessage>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(pos) = self.reader.stream_position() {
            if pos >= self.size {
                return None;
            }
            Some(QWKMessage::read(&mut self.reader, true))
        } else {
            None
        }
    }
}
