use crate::{
    convert_u32, convert_u64,
    file_base::{FileBaseError, HDR_SIGNATURE},
};
use chrono::{DateTime, Utc};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

pub struct FileBaseHeaderInfo {
    /// 'I' 'C' 'F' 'B'
    // pub signature: u32,

    /// Creation date, unix utc timestamp
    pub date_created: u64,
    /// Password for the filebase (Murmur64a hash)
    pub password: u64,
    /// attributes
    pub attributes: u32,
}
pub mod base_header_attributes {
    /// FileBase is password protected
    pub const PASSWORD: u32 = 0b0000_0001;
}

impl FileBaseHeaderInfo {
    const USED_HEADER_SIZE: usize = 24;
    pub const HEADER_SIZE: u64 = 1024;

    pub fn date_created(&self) -> Option<DateTime<Utc>> {
        chrono::DateTime::from_timestamp(self.date_created as i64, 0)
    }
    pub fn password(&self) -> u64 {
        self.password
    }

    pub fn load(file: &mut File) -> crate::Result<Self> {
        let data = &mut [0; Self::USED_HEADER_SIZE];
        file.read_exact(data)?;
        if !data.starts_with(&HDR_SIGNATURE) {
            return Err(Box::new(FileBaseError::InvalidHeaderSignature));
        }
        let mut data = &data[4..];
        convert_u64!(date_created, data);
        convert_u64!(password, data);
        convert_u32!(attributes, data);
        Ok(Self {
            date_created,
            password,
            attributes,
        })
    }

    pub(crate) fn create<P: AsRef<Path>>(file_name: &P, password: u64, attributes: u32) -> crate::Result<()> {
        let now = Utc::now();
        let datecreated = now.timestamp();

        let mut result = Vec::with_capacity(Self::HEADER_SIZE as usize);
        result.extend(HDR_SIGNATURE);
        result.extend(&datecreated.to_le_bytes());
        result.extend(&password.to_le_bytes());
        result.extend(&attributes.to_le_bytes());
        result.resize(Self::HEADER_SIZE as usize, 0);
        fs::write(file_name, result)?;
        Ok(())
    }
}
