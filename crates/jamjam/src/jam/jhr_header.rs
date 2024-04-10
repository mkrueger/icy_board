use std::{
    fs::{self, File},
    io::{BufWriter, Read, Seek, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::jam::{JamError, JAM_SIGNATURE};

/// This isthe 1024-byte record at the beginning of all
/// .JHR files.
///
/// The first actual message header starts at offset 1024 in the .JHR file.
#[derive(Debug, Default)]
pub struct JHRHeaderInfo {
    /// <J><A><M> followed by <NUL>
    //pub signature: u32,

    /// Creation date
    pub date_created: u32,
    /// Update counter
    pub mod_counter: u32,
    /// Number of active (not deleted) msgs  
    pub active_msgs: u32,
    /// CRC-32 of password to access
    /// Set to CRC_SEED (0xFFFFFFFF) for no password.
    pub password_crc: u32,

    /// Lowest message number in index file
    ///
    /// # Remarks
    /// This field determines the lowest message number in the index file.
    /// The value for this field is one (1) when a message area is first
    /// created. By using this field, a message area can be packed (deleted
    /// messages are removed) without renumbering it. If BaseMsgNum contains
    /// 500, the first index record points to message number 500.
    ///
    /// BaseMsgNum has to be taken into account when an application
    /// calculates the next available message number (for creating new
    /// messages) as well as the highest and lowest message number in a
    /// message area.
    pub base_msg_num: u32,
    // Reserved space (currently unused)
    // pub reserved: [u8; 1000]
}

impl JHRHeaderInfo {
    const JHR_USED_HEADER_SIZE: usize = 24;
    pub const JHR_HEADER_SIZE: u64 = 1024;

    pub fn load(file: &mut File) -> crate::Result<Self> {
        let data = &mut [0; Self::JHR_USED_HEADER_SIZE];
        file.read_exact(data)?;
        if !data.starts_with(&JAM_SIGNATURE) {
            return Err(Box::new(JamError::InvalidHeaderSignature));
        }
        let mut data = &data[4..];
        convert_u32!(datecreated, data);
        convert_u32!(modcounter, data);
        convert_u32!(activemsgs, data);
        convert_u32!(passwordcrc, data);
        convert_u32!(basemsgnum, data);
        Ok(Self {
            date_created: datecreated,
            mod_counter: modcounter,
            active_msgs: activemsgs,
            password_crc: passwordcrc,
            base_msg_num: basemsgnum,
        })
    }

    pub(crate) fn create<P: AsRef<Path>>(file_name: &P, passwordcrc: u32) -> crate::Result<()> {
        let now = SystemTime::now();
        let unix_time = now.duration_since(UNIX_EPOCH)?;
        let datecreated = unix_time.as_secs() as u32;

        let mut result = Vec::with_capacity(Self::JHR_HEADER_SIZE as usize);
        result.extend(JAM_SIGNATURE);
        result.extend(&datecreated.to_le_bytes());
        result.extend(&0u32.to_le_bytes());
        result.extend(&0u32.to_le_bytes());
        result.extend(&passwordcrc.to_le_bytes());
        result.extend(&1u32.to_le_bytes());
        result.resize(Self::JHR_HEADER_SIZE as usize, 0);
        fs::write(file_name, result)?;
        Ok(())
    }

    pub(crate) fn update(&mut self, file: &mut BufWriter<File>) -> crate::Result<()> {
        file.seek(std::io::SeekFrom::Start(8))?;
        self.mod_counter = self.mod_counter.wrapping_add(1);
        file.write_all(&self.mod_counter.to_le_bytes())?;
        file.write_all(&self.active_msgs.to_le_bytes())?;
        file.write_all(&self.password_crc.to_le_bytes())?;
        file.write_all(&self.base_msg_num.to_le_bytes())?;
        Ok(())
    }
}
