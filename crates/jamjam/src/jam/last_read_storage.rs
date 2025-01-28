use std::{
    fs::File,
    io::{BufReader, Read, Write},
};

#[derive(Default, Debug)]
pub struct JamLastReadStorage {
    pub user_crc: u32,      // CRC-32 of user name (lowercase)   (1)
    pub user_id: u32,       // Unique UserID
    pub last_read_msg: u32, // Last read message number
    pub high_read_msg: u32, // Highest read message number
}

impl JamLastReadStorage {
    const LAST_READ_SIZE: usize = 16;

    pub fn load(file: &mut BufReader<File>) -> crate::Result<Self> {
        let data = &mut [0; Self::LAST_READ_SIZE];
        file.read_exact(data)?;
        let mut data = &data[..];
        convert_u32!(user_crc, data);
        convert_u32!(user_id, data);
        convert_u32!(last_read_msg, data);
        convert_u32!(high_read_msg, data);
        Ok(Self {
            user_crc,
            user_id,
            last_read_msg,
            high_read_msg,
        })
    }

    pub fn write(&self, file: &mut File) -> crate::Result<()> {
        let mut record = Vec::new();
        record.extend_from_slice(&self.user_id.to_le_bytes());
        record.extend_from_slice(&self.user_crc.to_le_bytes());
        record.extend_from_slice(&self.last_read_msg.to_le_bytes());
        record.extend_from_slice(&self.high_read_msg.to_le_bytes());
        file.write_all(&record)?;
        Ok(())
    }
}
