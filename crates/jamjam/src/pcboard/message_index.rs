use std::{
    fs::File,
    io::{BufReader, Read},
};

use bstr::BString;

use crate::pcboard::FROM_TO_LEN;

#[derive(Clone, Debug)]
pub struct PCBoardMessageIndex {
    pub offset: u32,
    pub num: u32,
    pub to: BString,
    pub from: BString,
    pub status: u8,
    pub date: u16,
    pub reserved: [u8; 3],
}

impl PCBoardMessageIndex {
    pub const HEADER_SIZE: usize = 4 + 4 + 25 + 25 + 1 + 2 + 3;

    pub fn read(file: &mut BufReader<File>) -> crate::Result<Self> {
        let data = &mut [0; Self::HEADER_SIZE];
        file.read_exact(data)?;
        let mut data = &data[..];

        convert_u32!(offset, data);
        convert_u32!(num, data);
        convert_to_string!(to, data, FROM_TO_LEN);
        convert_to_string!(from, data, FROM_TO_LEN);
        convert_u8!(status, data);
        convert_u16!(date, data);
        let reserved = [data[0], data[1], data[2]];

        Ok(Self {
            offset,
            num,
            to,
            from,
            status,
            date,
            reserved,
        })
    }

    /*
    pub fn load(file: &str) -> crate::Result<Vec<Self>> {
        let buf = fs::read(file)?;
        let mut cursor = Cursor::new(&buf);
        let mut messages = Vec::new();
        while cursor.position() < cursor.get_ref().len() as u64 {
            messages.push(Self::deserialize(&mut cursor)?);
        }
        Ok(messages)
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        buf.extend(&self.offset.to_le_bytes());
        buf.extend(&self.num.to_le_bytes());

        buf.extend(&gen_string(&self.to, FROM_TO_LEN));
        buf.extend(&gen_string(&self.from, FROM_TO_LEN));
        buf.push(self.status);
        buf.extend(&self.date.to_le_bytes());
        buf.extend(&self.reserved);
    }*/
}
