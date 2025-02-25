use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
};

use chrono::{DateTime, Utc};

use crate::{convert_u8, convert_u32, convert_u64};

#[derive(Clone)]
pub struct FileHeader {
    //name_len: u8,
    /// File name (up to 255 bytes long)
    pub name: String,
    /// unix utc timestamp
    pub file_date: u64,
    /// size of the file in bytes
    pub size: u64,
    /// crc32 hash of the file
    pub hash: u32,
    /// # times of download.self.attribute & attributes::PASSWORD != 0
    pub dl_counter: u64,
    /// Offset of the metadata
    pub metadata_offset: u64,
    /// Long description offset
    pub long_description_offset: u64,
    /// Attributes of the file
    pub attribute: u8,
}

pub mod attributes {
    /// File is free - no dl costs
    pub const FREE: u8 = 0b0000_0001;
    /// File has tags to scan for
    pub const HAS_TAGS: u8 = 0b0000_0010;
    /// PW protected - it's in the metadata
    pub const PASSWORD: u8 = 0b0001_0000;
    /// File can't be deleted
    pub const LOCKED: u8 = 0b0100_0000;
    /// File is deleted
    pub const DELETED: u8 = 0b1000_0000;
}

impl FileHeader {
    pub const HEADER_SIZE: usize = 256 + 8 + 8 + 8 + 4 + 8 + 8 + 1;
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn file_date(&self) -> Option<DateTime<Utc>> {
        chrono::DateTime::from_timestamp(self.file_date as i64, 0)
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn hash(&self) -> u32 {
        self.hash
    }
    pub fn dl_counter(&self) -> u64 {
        self.dl_counter
    }
    pub fn metadata_offset(&self) -> u64 {
        self.metadata_offset
    }

    pub fn is_free(&self) -> bool {
        self.attribute & attributes::FREE != 0
    }
    pub fn needs_password(&self) -> bool {
        self.attribute & attributes::PASSWORD != 0
    }
    pub fn is_locked(&self) -> bool {
        self.attribute & attributes::LOCKED != 0
    }
    pub fn is_deleted(&self) -> bool {
        self.attribute & attributes::DELETED != 0
    }

    pub fn set_free(&mut self, free: bool) {
        if free {
            self.attribute |= attributes::FREE;
        } else {
            self.attribute &= !attributes::FREE;
        }
    }

    pub fn set_password(&mut self, password: bool) {
        if password {
            self.attribute |= attributes::PASSWORD;
        } else {
            self.attribute &= !attributes::PASSWORD;
        }
    }

    pub fn set_locked(&mut self, locked: bool) {
        if locked {
            self.attribute |= attributes::LOCKED;
        } else {
            self.attribute &= !attributes::LOCKED;
        }
    }

    pub fn set_deleted(&mut self, deleted: bool) {
        if deleted {
            self.attribute |= attributes::DELETED;
        } else {
            self.attribute &= !attributes::DELETED;
        }
    }

    pub fn read(file: &mut BufReader<File>) -> crate::Result<Self> {
        let data = &mut [0; Self::HEADER_SIZE];
        file.read_exact(data)?;

        let name_len = data[0] as usize;
        let name = std::str::from_utf8(&data[1..name_len + 1]).unwrap().to_string();
        let mut data = &data[256..];
        convert_u64!(file_date, data);
        convert_u64!(size, data);
        convert_u32!(hash, data);
        convert_u64!(dl_counter, data);
        convert_u64!(metadata_offset, data);
        convert_u64!(long_description_offset, data);
        convert_u8!(attribute, data);

        Ok(Self {
            name,
            file_date,
            size,
            hash,
            dl_counter,
            metadata_offset,
            long_description_offset,
            attribute,
        })
    }
    pub fn write(&self, file: &mut BufWriter<File>) -> crate::Result<()> {
        let mut data = Vec::with_capacity(Self::HEADER_SIZE);
        if self.name.len() > 255 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Name too long")));
        }
        data.push(self.name.len() as u8);
        data.extend(self.name.as_bytes());
        data.resize(256, 0);

        data.extend(&self.file_date.to_le_bytes());
        data.extend(&self.size.to_le_bytes());
        data.extend(&self.hash.to_le_bytes());
        data.extend(&self.dl_counter.to_le_bytes());
        data.extend(&self.metadata_offset.to_le_bytes());
        data.extend(&self.long_description_offset.to_le_bytes());
        data.push(self.attribute);
        file.write_all(&data)?;
        Ok(())
    }
}
