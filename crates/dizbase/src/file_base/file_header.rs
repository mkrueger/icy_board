use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
};

use chrono::{DateTime, MappedLocalTime, TimeZone, Utc};

#[derive(Clone)]
pub struct FileHeader {
    //name_len: u8,
    /// File name (up to 255 bytes long)
    pub name: String,
    /// unix utc timestamp
    pub date: DateTime<Utc>,
    /// size of the file in bytes
    pub size: u64,
    /// # times of download.self.attribute & attributes::PASSWORD != 0
    pub dl_counter: u64,
    /// Offset of the metadata
    pub metadata_offset: u64,
    /// Attributes of the file
    pub attribute: FileAttributes,
}

bitflags::bitflags! {
    #[derive(Copy, Clone)]
    pub struct FileAttributes : u8 {
        const NONE = 0b0000_0000;

        /// File is free - no dl costs
        const FREE = 0b0000_0001;
        /// File has tags to scan for
        const HAS_TAGS = 0b0000_0010;
        /// PW protected - it's in the metadata
        const PASSWORD = 0b0001_0000;
        /// File can't be deleted
        const LOCKED = 0b0100_0000;
        /// File is deleted
        const DELETED = 0b1000_0000;
    }
}

impl FileHeader {
    pub const HEADER_SIZE: usize = 256 + 8 + 8 + 8 + 4 + 8 + 8 + 1;
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn date(&self) -> DateTime<Utc> {
        self.date
    }
    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn dl_counter(&self) -> u64 {
        self.dl_counter
    }
    pub fn metadata_offset(&self) -> u64 {
        self.metadata_offset
    }

    pub fn is_free(&self) -> bool {
        self.attribute.contains(FileAttributes::FREE)
    }
    pub fn needs_password(&self) -> bool {
        self.attribute.contains(FileAttributes::PASSWORD)
    }
    pub fn is_locked(&self) -> bool {
        self.attribute.contains(FileAttributes::LOCKED)
    }
    pub fn is_deleted(&self) -> bool {
        self.attribute.contains(FileAttributes::DELETED)
    }

    pub fn set_free(&mut self, free: bool) {
        if free {
            self.attribute |= FileAttributes::FREE;
        } else {
            self.attribute &= !FileAttributes::FREE;
        }
    }

    pub fn set_password(&mut self, password: bool) {
        if password {
            self.attribute |= FileAttributes::PASSWORD;
        } else {
            self.attribute &= !FileAttributes::PASSWORD;
        }
    }

    pub fn set_locked(&mut self, locked: bool) {
        if locked {
            self.attribute |= FileAttributes::LOCKED;
        } else {
            self.attribute &= !FileAttributes::LOCKED;
        }
    }

    pub fn set_deleted(&mut self, deleted: bool) {
        if deleted {
            self.attribute |= FileAttributes::DELETED;
        } else {
            self.attribute &= !FileAttributes::DELETED;
        }
    }

    pub fn read(file: &mut BufReader<File>) -> crate::Result<Self> {
        let byte = &mut [0; 1];
        let byte8 = &mut [0; 8];

        file.read_exact(byte)?;
        let name_len = byte[0] as usize;

        let mut data = vec![0; name_len];
        file.read_exact(&mut data)?;
        let name = std::str::from_utf8(&mut data).unwrap().to_string();
        file.read_exact(byte8)?;

        let date = if let MappedLocalTime::Single(dt) = Utc.timestamp_millis_opt(i64::from_le_bytes(*byte8)) {
            dt
        } else {
            Utc::now()
        };
        file.read_exact(byte8)?;
        let size = u64::from_le_bytes(*byte8);
        file.read_exact(byte8)?;
        let dl_counter = u64::from_le_bytes(*byte8);
        file.read_exact(byte8)?;
        let metadata_offset = u64::from_le_bytes(*byte8);
        file.read_exact(byte)?;
        let attribute = byte[0];

        Ok(Self {
            name,
            date,
            size,
            dl_counter,
            metadata_offset,
            attribute: FileAttributes::from_bits_truncate(attribute),
        })
    }
    pub fn write(&self, file: &mut BufWriter<File>) -> crate::Result<()> {
        let mut data = Vec::with_capacity(Self::HEADER_SIZE);
        if self.name.len() > 255 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Name too long")));
        }
        data.push(self.name.len() as u8);
        data.extend(self.name.as_bytes());
        data.extend(&self.date.timestamp_millis().to_le_bytes());

        data.extend(&self.size.to_le_bytes());
        data.extend(&self.dl_counter.to_le_bytes());
        data.extend(&self.metadata_offset.to_le_bytes());
        data.push(self.attribute.bits());
        file.write_all(&data)?;
        Ok(())
    }
}
