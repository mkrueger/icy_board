use byteorder::{LittleEndian, ReadBytesExt};
use icy_ppe::{datetime::IcbDate, tables::CP437_TO_UNICODE, Res};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Cursor, Read},
    path::Path,
};

use crate::icy_board::IcyBoardError;

#[derive(Debug)]
struct UserInfApplication {
    /// Name of application
    pub name: String,
    /// Version number
    pub _version: u16,
    /// Size of application record in bytes
    pub size_of_rec: u16,
    /// Size of each application conference record
    pub _size_of_conf_rec: u16,
    /// Keyword for executing the application
    pub _keyword: String,
    /// Offset in user record where the data is stored
    pub offset: u32,
}

impl UserInfApplication {
    pub fn read(cursor: &mut Cursor<Vec<u8>>) -> Res<UserInfApplication> {
        let mut buf = [0; 15];
        cursor.read_exact(&mut buf)?;
        let name = convert_str(&buf);
        let _version = cursor.read_u16::<LittleEndian>()?;
        let size_of_rec = cursor.read_u16::<LittleEndian>()?;
        let _size_of_conf_rec = cursor.read_u16::<LittleEndian>()?;

        let mut buf = [0; 9];
        cursor.read_exact(&mut buf)?;
        let _keyword = convert_str(&buf);
        let offset = cursor.read_u32::<LittleEndian>()?;

        Ok(UserInfApplication {
            name,
            _version,
            size_of_rec,
            _size_of_conf_rec,
            _keyword,
            offset,
        })
    }
}

fn convert_str(buf: &[u8]) -> String {
    let mut str = String::new();
    for c in buf {
        if *c == 0 {
            break;
        }
        str.push(CP437_TO_UNICODE[*c as usize]);
    }
    while str.ends_with([' ']) {
        str.pop();
    }
    str
}

#[derive(Default, Clone, Debug)]
pub struct PcbUserInf {
    pub name: String,
    pub messages_read: usize,
    pub messages_left: usize,

    pub alias: Option<AliasUserInf>,
    pub verify: Option<VerifyUserInf>,
    pub address: Option<AddressUserInf>,
    pub password: Option<PasswordUserInf>,
    pub call_stats: Option<CallStatsUserInf>,
    pub notes: Option<NotesUserInf>,
    pub qwk_config: Option<QwkConfigUserInf>,
    pub account: Option<AccountUserInf>,
    pub personal: Option<PersonalUserInf>,
    pub bank: Option<BankUserInf>,
}

impl PcbUserInf {
    const REC_SIZE: usize = 33;
    fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                "USER.INF",
                Self::REC_SIZE,
                data.len(),
            )));
        }
        let mut i = 0;
        let name = convert_str(&data[i..i + 25]);
        i += 25;
        let messages_read =
            u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        i += 4;
        let messages_left =
            u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;

        Ok(Self {
            name,
            messages_read,
            messages_left,
            ..Default::default()
        })
    }

    pub fn read_users(path: &Path) -> Res<Vec<PcbUserInf>> {
        let mut users = Vec::new();

        let data = fs::read(path)?;
        let mut cursor = Cursor::new(data);

        let _version = cursor.read_u16::<LittleEndian>()?;
        let _num_conf = cursor.read_u16::<LittleEndian>()?;
        let size_of_rec = cursor.read_u16::<LittleEndian>()? as usize;
        let _conf_size = cursor.read_u32::<LittleEndian>()?;
        let app_num = cursor.read_u16::<LittleEndian>()?;
        let rec_size = cursor.read_u32::<LittleEndian>()? as u64;

        let mut apps = Vec::new();
        for _ in 0..app_num {
            let inf = UserInfApplication::read(&mut cursor)?;
            apps.push(inf);
        }

        let mut record = vec![0; rec_size as usize];
        while cursor.position() + rec_size <= cursor.get_ref().len() as u64 {
            cursor.read_exact(&mut record)?;
            let mut user = PcbUserInf::read(&record[0..size_of_rec])?;

            for app in &apps {
                let data =
                    &record[app.offset as usize..app.offset as usize + app.size_of_rec as usize];
                match app.name.as_str() {
                    AliasUserInf::NAME => {
                        user.alias = Some(AliasUserInf::read(data)?);
                    }
                    VerifyUserInf::NAME => {
                        user.verify = Some(VerifyUserInf::read(data)?);
                    }
                    AddressUserInf::NAME => {
                        user.address = Some(AddressUserInf::read(data)?);
                    }
                    PasswordUserInf::NAME => {
                        user.password = Some(PasswordUserInf::read(data)?);
                    }
                    CallStatsUserInf::NAME => {
                        user.call_stats = Some(CallStatsUserInf::read(data)?);
                    }
                    NotesUserInf::NAME => {
                        user.notes = Some(NotesUserInf::read(data)?);
                    }
                    QwkConfigUserInf::NAME => {
                        user.qwk_config = Some(QwkConfigUserInf::read(data)?);
                    }
                    AccountUserInf::NAME => {
                        user.account = Some(AccountUserInf::read(data)?);
                    }
                    PersonalUserInf::NAME => {
                        user.personal = Some(PersonalUserInf::read(data)?);
                    }
                    BankUserInf::NAME => {
                        user.bank = Some(BankUserInf::read(data)?);
                    }
                    unkown => {
                        log::error!("Unknown user.inf app: {}", unkown);
                    }
                }
            }
            users.push(user);
        }

        Ok(users)
    }
}

#[derive(Default, Debug, Clone)]
pub struct AliasUserInf {
    pub alias: String,
}

impl AliasUserInf {
    pub const NAME: &'static str = "PCBALIAS";
    const REC_SIZE: usize = 25;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err("Invalid {} record size".into());
        }
        let alias = convert_str(data);
        Ok(Self { alias })
    }
}

#[derive(Default, Debug, Clone)]
pub struct VerifyUserInf {
    pub verify: String,
}

impl VerifyUserInf {
    pub const NAME: &'static str = "PCBVERIFY";
    const REC_SIZE: usize = 25;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }
        let alias = convert_str(data);
        Ok(Self { verify: alias })
    }
}

#[derive(Default, Debug, Clone)]
pub struct AddressUserInf {
    pub street1: String,
    pub street2: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
}

impl AddressUserInf {
    pub const NAME: &'static str = "PCBADDRESS";
    const REC_SIZE: usize = 160;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }
        let mut i = 0;
        let street1 = convert_str(&data[i..i + 50]);
        i += 50;
        let street2 = convert_str(&data[i..i + 50]);
        i += 50;
        let city = convert_str(&data[i..i + 25]);
        i += 25;
        let state = convert_str(&data[i..i + 10]);
        i += 10;
        let zip = convert_str(&data[i..i + 10]);
        i += 10;
        let country = convert_str(&data[i..i + 15]);
        Ok(Self {
            street1,
            street2,
            city,
            state,
            zip,
            country,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct PasswordUserInf {
    pub prev_pwd: [String; 3],
    pub last_change: IcbDate,
    pub times_changed: usize,
    pub expire_date: IcbDate,
}

impl PasswordUserInf {
    pub const NAME: &'static str = "PCBPASSWORD";
    const REC_SIZE: usize = 42;

    const PWD_LEN: usize = 12;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }

        let mut i = 0;

        let pwd1 = convert_str(&data[i..i + Self::PWD_LEN]);
        i += Self::PWD_LEN;
        let pwd2 = convert_str(&data[i..i + Self::PWD_LEN]);
        i += Self::PWD_LEN;
        let pwd3 = convert_str(&data[i..i + Self::PWD_LEN]);
        i += Self::PWD_LEN;
        let last_change = IcbDate::from_pcboard(u16::from_le_bytes([data[i], data[i + 1]]) as u32);
        i += 2;
        let times_changed = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        i += 2;
        let expire_date = u16::from_le_bytes([data[i], data[i + 1]]) as u32;

        Ok(Self {
            prev_pwd: [pwd1, pwd2, pwd3],
            last_change,
            times_changed,
            expire_date: IcbDate::from_pcboard(expire_date),
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct CallStatsUserInf {
    /// First date on
    pub first_date_on: IcbDate,
    /// Times of paged sysop
    pub num_sysop_pages: usize,
    /// Times of group chat
    pub num_group_chats: usize,
    /// Times of comments to sysop
    pub num_comments: usize,

    /// Times on at 300
    pub num300: usize,
    /// Times on at 1200
    pub num1200: usize,
    /// Times on at 2400
    pub num2400: usize,
    /// Times on at 9600
    pub num9600: usize,
    /// Times on at 14400
    pub num14400: usize,
    /// Number of security violations
    pub num_sec_viol: usize,
    /// Number of unregistered conference attempts
    pub num_not_reg: usize,
    /// # Download limit reached
    pub num_reach_dnld_lim: usize,
    /// # Download file not found
    pub num_file_not_found: usize,
    /// # Password failures
    pub num_pwrd_errors: usize,
    /// # Upload verification failed
    pub num_verify_errors: usize,
}

impl CallStatsUserInf {
    pub const NAME: &'static str = "PCBSTATS";
    const REC_SIZE: usize = 30;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }

        let mut cursor = Cursor::new(data);
        let first_date_on = cursor.read_i16::<LittleEndian>()? as u32;
        let num_sysop_pages = cursor.read_u16::<LittleEndian>()? as usize;
        let num_group_chats = cursor.read_u16::<LittleEndian>()? as usize;
        let num_comments = cursor.read_u16::<LittleEndian>()? as usize;
        let num300 = cursor.read_u16::<LittleEndian>()? as usize;
        let num1200 = cursor.read_u16::<LittleEndian>()? as usize;
        let num2400 = cursor.read_u16::<LittleEndian>()? as usize;
        let num9600 = cursor.read_u16::<LittleEndian>()? as usize;
        let num14400 = cursor.read_u16::<LittleEndian>()? as usize;
        let num_sec_viol = cursor.read_u16::<LittleEndian>()? as usize;
        let num_not_reg = cursor.read_u16::<LittleEndian>()? as usize;
        let num_reach_dnld_lim = cursor.read_u16::<LittleEndian>()? as usize;
        let num_file_not_found = cursor.read_u16::<LittleEndian>()? as usize;
        let num_pwrd_errors = cursor.read_u16::<LittleEndian>()? as usize;
        let num_verify_errors = cursor.read_u16::<LittleEndian>()? as usize;

        Ok(Self {
            first_date_on: IcbDate::from_pcboard(first_date_on),
            num_sysop_pages,
            num_group_chats,
            num_comments,
            num300,
            num1200,
            num2400,
            num9600,
            num14400,
            num_sec_viol,
            num_not_reg,
            num_reach_dnld_lim,
            num_file_not_found,
            num_pwrd_errors,
            num_verify_errors,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct NotesUserInf {
    pub notes: Vec<String>,
}

impl NotesUserInf {
    pub const NAME: &'static str = "PCBNOTES";
    const REC_SIZE: usize = 300;

    const NOTE_COUNT: usize = 5;
    const NOTE_SIZE: usize = 60;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }

        let mut i = 0;
        let mut notes = Vec::new();
        for _ in 0..Self::NOTE_COUNT {
            let note = convert_str(&data[i..i + Self::NOTE_SIZE]);
            i += Self::NOTE_SIZE;
            notes.push(note);
        }
        Ok(Self { notes })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct QwkConfigUserInf {
    pub max_msgs: u16,
    pub max_msgs_per_conf: u16,
    pub personal_attach_limit: i32,
    pub public_attach_limit: i32,
    pub new_blt_limit: i32,
    pub new_files: bool,
}

impl QwkConfigUserInf {
    pub const NAME: &'static str = "PCBQWKNET";
    const REC_SIZE: usize = 30;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }

        let mut cursor = Cursor::new(data);

        let max_msgs = cursor.read_u16::<LittleEndian>()?;
        let max_msgs_per_conf = cursor.read_u16::<LittleEndian>()?;
        let personal_attach_limit = cursor.read_i16::<LittleEndian>()? as i32;
        let public_attach_limit = cursor.read_i16::<LittleEndian>()? as i32;
        let new_blt_limit = cursor.read_i16::<LittleEndian>()? as i32;
        let new_files = cursor.read_u8()? != 0;

        Ok(Self {
            max_msgs,
            max_msgs_per_conf,
            personal_attach_limit,
            public_attach_limit,
            new_blt_limit,
            new_files,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AccountUserInf {
    pub starting_balance: f64,
    pub start_this_session: f64,
    pub debit_call: f64,
    pub debit_time: f64,
    pub debit_msg_read: f64,
    pub debit_msg_read_capture: f64,
    pub debit_msg_write: f64,
    pub debit_msg_write_echoed: f64,
    pub debit_msg_write_private: f64,
    pub debit_download_file: f64,
    pub debit_download_bytes: f64,
    pub debit_group_chat: f64,
    pub debit_tpu: f64,
    pub debit_special: f64,
    pub credit_upload_file: f64,
    pub credit_upload_bytes: f64,
    pub credit_special: f64,
    pub drop_sec_level: u8,
}

impl AccountUserInf {
    pub const NAME: &'static str = "PCBACCOUNT";
    const REC_SIZE: usize = 137;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }

        let mut cursor = Cursor::new(data);

        let starting_balance = cursor.read_f64::<LittleEndian>()?;
        let start_this_session = cursor.read_f64::<LittleEndian>()?;
        let debit_call = cursor.read_f64::<LittleEndian>()?;
        let debit_time = cursor.read_f64::<LittleEndian>()?;
        let debit_msg_read = cursor.read_f64::<LittleEndian>()?;
        let debit_msg_read_capture = cursor.read_f64::<LittleEndian>()?;
        let debit_msg_write = cursor.read_f64::<LittleEndian>()?;
        let debit_msg_write_echoed = cursor.read_f64::<LittleEndian>()?;
        let debit_msg_write_private = cursor.read_f64::<LittleEndian>()?;
        let debit_download_file = cursor.read_f64::<LittleEndian>()?;
        let debit_download_bytes = cursor.read_f64::<LittleEndian>()?;
        let debit_group_chat = cursor.read_f64::<LittleEndian>()?;
        let debit_tpu = cursor.read_f64::<LittleEndian>()?;
        let debit_special = cursor.read_f64::<LittleEndian>()?;
        let credit_upload_file = cursor.read_f64::<LittleEndian>()?;
        let credit_upload_bytes = cursor.read_f64::<LittleEndian>()?;
        let credit_special = cursor.read_f64::<LittleEndian>()?;
        let drop_sec_level = cursor.read_u8()?;

        Ok(Self {
            starting_balance,
            start_this_session,
            debit_call,
            debit_time,
            debit_msg_read,
            debit_msg_read_capture,
            debit_msg_write,
            debit_msg_write_echoed,
            debit_msg_write_private,
            debit_download_file,
            debit_download_bytes,
            debit_group_chat,
            debit_tpu,
            debit_special,
            credit_upload_file,
            credit_upload_bytes,
            credit_special,
            drop_sec_level,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct PersonalUserInf {
    pub gender: String,
    pub birth_date: IcbDate,
    pub email: String,
    pub web: String,
}

impl PersonalUserInf {
    pub const NAME: &'static str = "PCBPERSONAL";
    const REC_SIZE: usize = 252;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }

        let mut i = 0;
        let gender = CP437_TO_UNICODE[data[i] as usize].to_string();
        i += 1;
        let birth_date = convert_str(&data[i..i + 9]);
        i += 9;
        let email = convert_str(&data[i..i + 60]);
        i += 60;
        // unknown ?
        i += 61;
        let web = convert_str(&data[i..i + 60]);
        // i += 60;
        // unknown?
        // i += 61;
        Ok(Self {
            gender,
            birth_date: IcbDate::parse(&birth_date),
            email,
            web,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BankInfo {
    pub last_deposite_date: IcbDate,
    pub last_withdraw_date: IcbDate,
    pub last_transaction_amount: u32,
    pub amount_saved: u32,
    pub max_withdrawl_per_day: u32,
    pub max_stored_amount: u32,
}
impl BankInfo {
    fn read(cursor: &mut Cursor<&[u8]>) -> Res<BankInfo> {
        let last_deposite_date = cursor.read_u32::<LittleEndian>()?;
        let last_withdraw_date = cursor.read_u32::<LittleEndian>()?;
        let last_transaction_amount = cursor.read_u32::<LittleEndian>()?;
        let amount_saved = cursor.read_u32::<LittleEndian>()?;
        let max_withdrawl_per_day = cursor.read_u32::<LittleEndian>()?;
        let max_stored_amount = cursor.read_u32::<LittleEndian>()?;

        Ok(Self {
            last_deposite_date: IcbDate::from_pcboard(last_deposite_date),
            last_withdraw_date: IcbDate::from_pcboard(last_withdraw_date),
            last_transaction_amount,
            amount_saved,
            max_withdrawl_per_day,
            max_stored_amount,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BankUserInf {
    pub time_info: BankInfo,
    pub byte_info: BankInfo,
}

impl BankUserInf {
    pub const NAME: &'static str = "PCBBANK";
    const REC_SIZE: usize = 48;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(
                Self::NAME,
                Self::REC_SIZE,
                data.len(),
            )));
        }

        let mut cursor = Cursor::new(data);
        let time_info = BankInfo::read(&mut cursor)?;
        let byte_info = BankInfo::read(&mut cursor)?;

        Ok(Self {
            time_info,
            byte_info,
        })
    }
}
