use std::{
    fs::File,
    io::{BufWriter, Read, Write},
};

use crate::util::basic_real::BasicReal;

pub struct PCBoardMessageBaseHeader {
    /// Highest message number in index file
    pub high_msg_num: u32,
    /// Lowest message number in index file
    pub low_msg_num: u32,
    /// Active (non deleted) messages
    pub active_msgs: u32,

    /// Number of callers for this conference.
    /// Note that PCBoard only allows 1 message base per conference.
    pub callers: u32,

    pub lock_status: [u8; 6],
}

pub const UNLOCKED: [u8; 6] = *b"      ";
pub const LOCKED: [u8; 6] = *b"LOCKED";

impl PCBoardMessageBaseHeader {
    pub const HEADER_SIZE: usize = 4 * 4 + 6;

    pub fn load(file: &mut File) -> crate::Result<Self> {
        let data = &mut [0; Self::HEADER_SIZE];
        file.read_exact(data)?;
        let mut data = &data[..];

        convert_u32!(high_msg_num, data);
        let high_msg_num = BasicReal::from(high_msg_num.to_le_bytes()).into();
        convert_u32!(low_msg_num, data);
        let low_msg_num = BasicReal::from(low_msg_num.to_le_bytes()).into();
        convert_u32!(num_active_msgs, data);
        let num_active_msgs = BasicReal::from(num_active_msgs.to_le_bytes()).into();
        convert_u32!(num_callers, data);
        let num_callers = BasicReal::from(num_callers.to_le_bytes()).into();
        let lock_status = [data[0], data[1], data[2], data[3], data[4], data[5]];

        Ok(Self {
            high_msg_num,
            low_msg_num,
            active_msgs: num_active_msgs,
            callers: num_callers,
            lock_status,
        })
    }

    pub(crate) fn write_header_to(&self, file: &mut BufWriter<File>) -> crate::Result<()> {
        let mut data = Vec::with_capacity(Self::HEADER_SIZE);
        data.extend(&self.high_msg_num.to_le_bytes());
        data.extend(&self.low_msg_num.to_le_bytes());
        data.extend(&self.active_msgs.to_le_bytes());
        data.extend(&self.callers.to_le_bytes());
        data.extend(&self.lock_status);
        file.write_all(&data)?;
        Ok(())
    }

    pub fn lock(&mut self) {
        self.lock_status = LOCKED;
    }

    pub fn unlock(&mut self) {
        self.lock_status = UNLOCKED;
    }

    pub(crate) fn is_locked(&self) -> bool {
        self.lock_status != UNLOCKED
    }
}
