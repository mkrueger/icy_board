use std::{
    fs,
    io::{Cursor, Read},
};

use byteorder::{LittleEndian, ReadBytesExt};
use icy_ppe::Res;

pub struct CallStat {
    pub last_caller: String,
    pub time: String,
    pub new_msgs: i32,
    pub new_calls: i32,
    pub total_up: i32,
    pub total_dn: i32,
    pub local_stats: bool,
}

impl CallStat {
    const LAST_CALLER_LEN: usize = 54;
    const TIME_LEN: usize = 6;

    pub fn load(file: &str) -> Res<CallStat> {
        let data = fs::read(file)?;
        let mut cursor = Cursor::new(data);
        let mut last_caller = [0u8; Self::LAST_CALLER_LEN];
        cursor.read_exact(&mut last_caller)?;
        let last_caller = String::from_utf8_lossy(&last_caller).to_string();
        let mut time = [0u8; Self::TIME_LEN];
        cursor.read_exact(&mut time)?;
        let time = String::from_utf8_lossy(&time).to_string();
        let new_msgs = cursor.read_i32::<LittleEndian>()?;
        let new_calls = cursor.read_i32::<LittleEndian>()?;
        let total_up = cursor.read_i32::<LittleEndian>()?;
        let total_dn = cursor.read_i32::<LittleEndian>()?;
        let local_stats = cursor.read_u8()? == 1;
        Ok(CallStat {
            last_caller,
            time,
            new_msgs,
            new_calls,
            total_up,
            total_dn,
            local_stats,
        })
    }
}
