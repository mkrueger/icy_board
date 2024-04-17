use bstr::BString;
use chrono::{Datelike, Local, NaiveTime};

use super::{convert_pcboard_str, PCBoardError};
use crate::{
    pcboard::{DATE_LEN, FROM_TO_LEN, PASSWORD_LEN, TIME_LEN},
    util::basic_real::basicreal_to_u32,
};
use std::{
    fs::File,
    io::{BufReader, Read},
};

pub enum MessageType {
    Public,
    Receiver,
    Comment,
    SenderPassword,
    GroupPassword,
}

#[derive(Clone, Debug)]
pub struct PCBoardMessageHeader {
    /// Message status flags
    /// # Remarks
    /// Not Read | Read | Type
    /// ---------|------|---------------
    /// '*'      | '+'  | Private
    /// '~'      | '`'  | Comment to Sysop
    /// '%'      | '^'  | Sender password
    /// '!'      | '#'  | Group password
    /// ' '      | '-'  | Public
    /// '$'      | NA   | Group password, public
    pub status: u8,

    /// Message number
    pub msg_number: u32,

    /// Message replied to (or referenced?)
    pub reply_to: u32,

    /// Number of 128 byte blocks including message header
    pub num_blocks: u8,

    /// Date & time (MM-DD-YYHH:MM)
    pub date_time: String,

    /// 25 character "To" field
    pub to_field: BString,

    /// Date as number YYMMDD
    pub reply_date: u32,

    /// Reply Time (HH:MM)
    pub reply_time: String,

    /// 'R' == repliey, ' ' == no reply
    pub reply_status: u8,

    /// 25 character "From" field
    pub from_field: BString,

    /// 25 character "Subj" field
    pub subj_field: BString,

    /// 12 character "Password" in plain text
    pub password: BString,

    /// 225 == active, 226 == deleted
    pub active_flag: u8,

    /// Echo flag == 'E'
    pub echo_flag: u8,
    pub reserved: [u8; 4],

    /// Flags indicating extended header status
    ///
    pub extended_status: u8,

    /// Either '*' which means the message is a netmail message or 0
    /// Haven't encountered ' ' yet
    pub net_tag: u8,
}

mod extended_status {
    /// Extended header 'TO' is defined
    pub const TO: u8 = 1;
    /// Extended header 'FROM' is defined
    pub const FROM: u8 = 2;
    /// Extended header 'SUBJECT' is defined
    pub const SUBJ: u8 = 4;
    /// Extended header 'LIST' is defined
    pub const LIST: u8 = 8;
    /// Extended header 'ATTACH' is defined
    pub const ATTACH: u8 = 16;
    /// Extended header 'REQRR' is defined
    pub const REQRR: u8 = 64;

    // 128 is undefined ?
}

pub const MSG_ACTIVE: u8 = 225;
pub const MSG_INACTIVE: u8 = 226;

pub const ECHO: u8 = b'E';
pub const NOECHO: u8 = b' ';

pub const REPLIED: u8 = b'R';
pub const NOT_REPLIED: u8 = b' ';

impl PCBoardMessageHeader {
    pub const HEADER_SIZE: usize = 128;
    /*
    pub get_status(&self) -> MessageType {
        match self.status {
            b'~' | b'*' => MessageStatus::PrivateUnread,
            b'`' | b'+' => MessageStatus::PrivateRead,
            b' ' => MessageStatus::PublicRead,
            b'-' => MessageStatus::PublicUnread,
            _ => MessageStatus::Unknown,
        }
    }*/

    /// Returns the date and time of the message
    ///
    /// # Remarks
    /// Returns default, in case of incorrect date
    pub fn date_time(&self) -> chrono::naive::NaiveDateTime {
        chrono::NaiveDateTime::parse_from_str(&self.date_time, "%m-%d-%y%H:%M").unwrap_or_default()
    }

    /// Returns the date and time of the reply
    /// The century is guessed from the current one. If the year is greater than the current year, it is assumed to be the previous century.
    ///
    /// # Remarks
    /// Returns default, in case of incorrect date
    pub fn reply_date_time(&self) -> chrono::naive::NaiveDateTime {
        let time = NaiveTime::parse_from_str(&self.reply_time, "%H:%M").unwrap_or_default();

        let now = Local::now();
        let century = (now.year() / 100) * 100;
        let mut year = (self.reply_date / 10000) as i32 + century;
        if year > now.year() {
            year -= 100;
        }

        let month = (self.reply_date % 10000) / 100;
        let day = self.reply_date % 100;

        let date = chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap_or_default();
        chrono::NaiveDateTime::new(date, time)
    }

    pub fn has_to(&self) -> bool {
        self.extended_status & extended_status::TO != 0
    }

    pub fn has_from(&self) -> bool {
        self.extended_status & extended_status::FROM != 0
    }

    pub fn has_subject(&self) -> bool {
        self.extended_status & extended_status::SUBJ != 0
    }

    pub fn has_list(&self) -> bool {
        self.extended_status & extended_status::LIST != 0
    }

    pub fn has_attach(&self) -> bool {
        self.extended_status & extended_status::ATTACH != 0
    }

    pub fn has_reqrr(&self) -> bool {
        self.extended_status & extended_status::REQRR != 0
    }

    pub fn replied(&self) -> bool {
        self.reply_status == REPLIED
    }

    pub fn read(file: &mut BufReader<File>) -> crate::Result<Self> {
        let data = &mut [0; Self::HEADER_SIZE];
        file.read_exact(data)?;
        let mut data = &data[..];

        convert_u8!(status, data);
        convert_u32!(msg_number, data);
        let msg_number = basicreal_to_u32(msg_number);
        convert_u32!(ref_number, data);
        let ref_number = basicreal_to_u32(ref_number);
        convert_u8!(num_blocks, data);
        convert_to_string!(date_time, data, DATE_LEN + TIME_LEN);
        convert_to_string!(to_field, data, FROM_TO_LEN);
        convert_u32!(reply_date, data);
        let reply_date = basicreal_to_u32(reply_date);

        convert_to_string!(reply_time, data, TIME_LEN);
        convert_u8!(reply_status, data);
        convert_to_string!(from_field, data, FROM_TO_LEN);
        convert_to_string!(subj_field, data, FROM_TO_LEN);
        convert_to_string!(password, data, PASSWORD_LEN);
        convert_u8!(active_flag, data);
        convert_u8!(echo_flag, data);
        let reserved = [data[0], data[1], data[2], data[3]];
        data = &data[4..];
        convert_u8!(extended_status, data);
        convert_u8!(net_tag, data);

        Ok(Self {
            status,
            msg_number,
            reply_to: ref_number,
            num_blocks,
            date_time: date_time.to_string(),
            to_field,
            reply_date,
            reply_time: reply_time.to_string(),
            reply_status,
            from_field,
            subj_field,
            password,
            active_flag,
            echo_flag,
            reserved,
            extended_status,
            net_tag,
        })
    }

    pub(crate) fn is_deleted(&self) -> bool {
        self.active_flag != MSG_ACTIVE
    }
}

#[derive(Debug)]
pub enum ExtendedHeaderInformation {
    To,
    From,
    Subject,
    Attach,
    List,
    Route,
    Origin,
    Reqrr,
    Ackrr,
    Ackname,
    Packout,
    To2,
    From2,
    Forward,
    Ufollow,
    Unewsgr,
}
const TO: &[u8; 7] = b"TO     ";
const FROM: &[u8; 7] = b"FROM   ";
const SUBJECT: &[u8; 7] = b"SUBJECT";
const ATTACH: &[u8; 7] = b"ATTACH ";
const LIST: &[u8; 7] = b"LIST   ";
const ROUTE: &[u8; 7] = b"ROUTE  ";
const ORIGIN: &[u8; 7] = b"ORIGIN ";
const REQRR: &[u8; 7] = b"REQRR  ";
const ACKRR: &[u8; 7] = b"ACKRR  ";
const ACKNAME: &[u8; 7] = b"ACKNAME";

/// Date when this message is deleted automatically
/// Note: All messages with packout date are public messages
const PACKOUT: &[u8; 7] = b"PACKOUT";
const TO2: &[u8; 7] = b"TO2    ";
const FROM2: &[u8; 7] = b"FROM2  ";
const FORWARD: &[u8; 7] = b"FORWARD";
const UFOLLOW: &[u8; 7] = b"UFOLLOW";
const UNEWSGR: &[u8; 7] = b"UNEWSGR";

impl ExtendedHeaderInformation {
    fn from_data(data: &[u8]) -> crate::Result<Self> {
        if *data == *TO {
            return Ok(Self::To);
        }
        if *data == *FROM {
            return Ok(Self::From);
        }
        if *data == *SUBJECT {
            return Ok(Self::Subject);
        }
        if *data == *ATTACH {
            return Ok(Self::Attach);
        }
        if *data == *LIST {
            return Ok(Self::List);
        }
        if *data == *ROUTE {
            return Ok(Self::Route);
        }
        if *data == *ORIGIN {
            return Ok(Self::Origin);
        }
        if *data == *REQRR {
            return Ok(Self::Reqrr);
        }
        if *data == *ACKRR {
            return Ok(Self::Ackrr);
        }
        if *data == *ACKNAME {
            return Ok(Self::Ackname);
        }
        if *data == *PACKOUT {
            return Ok(Self::Packout);
        }
        if *data == *TO2 {
            return Ok(Self::To2);
        }
        if *data == *FROM2 {
            return Ok(Self::From2);
        }
        if *data == *FORWARD {
            return Ok(Self::Forward);
        }
        if *data == *UFOLLOW {
            return Ok(Self::Ufollow);
        }
        if *data == *UNEWSGR {
            return Ok(Self::Unewsgr);
        }

        Err(PCBoardError::UnknownExtendedHeader(BString::new(data.to_vec())).into())
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::To => "TO     ",
            Self::From => "FROM   ",
            Self::Subject => "SUBJECT",
            Self::Attach => "ATTACH ",
            Self::List => "LIST   ",
            Self::Route => "ROUTE  ",
            Self::Origin => "ORIGIN ",
            Self::Reqrr => "REQRR  ",
            Self::Ackrr => "ACKRR  ",
            Self::Ackname => "ACKNAME",
            Self::Packout => "PACKOUT",
            Self::To2 => "TO2    ",
            Self::From2 => "FROM2  ",
            Self::Forward => "FORWARD",
            Self::Ufollow => "UFOLLOW",
            Self::Unewsgr => "UNEWSGR",
        }
    }
}

pub struct PCBoardExtendedHeader {
    pub info: ExtendedHeaderInformation,
    pub content: BString,
    /// 'N' == none, 'R' == read
    pub status: u8,
}

impl PCBoardExtendedHeader {
    // const ID:u16 = 0x40FF;
    const FUNC_LEN: usize = 7;
    const DESC_LEN: usize = 60;

    pub fn read(&self) -> bool {
        self.status == b'R'
    }

    pub fn deserialize(buf: &[u8]) -> crate::Result<Self> {
        // let _id = u16::from_le_bytes([buf[0], buf[1]]);
        let mut i = 2;
        let function = ExtendedHeaderInformation::from_data(&buf[i..i + Self::FUNC_LEN]).unwrap();
        i += Self::FUNC_LEN + 1; // skip ':'

        let content = convert_pcboard_str(&buf[i..i + Self::DESC_LEN]);
        i += Self::DESC_LEN;

        let status = buf[i];
        Ok(Self {
            info: function,
            content,
            status,
        })
    }
}
