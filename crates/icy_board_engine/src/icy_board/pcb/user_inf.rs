use crate::{Res, datetime::IcbDate};
use byteorder::{LittleEndian, ReadBytesExt};
use codepages::tables::CP437_TO_UNICODE;
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize("USER.INF", Self::REC_SIZE, data.len())));
        }
        let mut i = 0;
        let name = convert_str(&data[i..i + 25]);
        i += 25;
        let messages_read = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        i += 4;
        let messages_left = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;

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
                let data = &record[app.offset as usize..app.offset as usize + app.size_of_rec as usize];
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

    pub(crate) fn write_with_apps(&self, writer: &mut impl std::io::Write, apps: &[(String, usize, u32)]) -> Res<()> {
        use crate::tables::export_cp437_string;
        use byteorder::{LittleEndian, WriteBytesExt};

        // Write base user info record (33 bytes)
        writer.write_all(&export_cp437_string(&self.name, 25, b' '))?;
        writer.write_u32::<LittleEndian>(self.messages_read as u32)?;
        writer.write_u32::<LittleEndian>(self.messages_left as u32)?;

        // Write application sections in the order declared in the header
        for (app_name, app_size, _) in apps {
            match app_name.as_str() {
                "PCBALIAS" => {
                    if let Some(alias) = &self.alias {
                        alias.write(writer)?;
                    } else {
                        // Write empty alias record
                        writer.write_all(&vec![b' '; *app_size])?;
                    }
                }
                "PCBVERIFY" => {
                    if let Some(verify) = &self.verify {
                        verify.write(writer)?;
                    } else {
                        writer.write_all(&vec![b' '; *app_size])?;
                    }
                }
                "PCBADDRESS" => {
                    if let Some(address) = &self.address {
                        address.write(writer)?;
                    } else {
                        writer.write_all(&vec![b' '; *app_size])?;
                    }
                }
                "PCBPASSWORD" => {
                    if let Some(password) = &self.password {
                        password.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; *app_size])?;
                    }
                }
                "PCBSTATS" => {
                    if let Some(call_stats) = &self.call_stats {
                        call_stats.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; *app_size])?;
                    }
                }
                "PCBNOTES" => {
                    if let Some(notes) = &self.notes {
                        notes.write(writer)?;
                    } else {
                        writer.write_all(&vec![b' '; *app_size])?;
                    }
                }
                "PCBQWKCONFIG" => {
                    if let Some(qwk_config) = &self.qwk_config {
                        qwk_config.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; *app_size])?;
                    }
                }
                "PCBACCOUNT" => {
                    if let Some(account) = &self.account {
                        account.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; *app_size])?;
                    }
                }
                "PCBPERSONAL" => {
                    if let Some(personal) = &self.personal {
                        personal.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; *app_size])?;
                    }
                }
                "PCBBANK" => {
                    if let Some(bank) = &self.bank {
                        bank.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; *app_size])?;
                    }
                }
                _ => {
                    // Unknown app, write zeros
                    writer.write_all(&vec![0u8; *app_size])?;
                }
            }
        }

        Ok(())
    }

    pub(crate) fn write_users(users: &[PcbUserInf], writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;
        use byteorder::{LittleEndian, WriteBytesExt};

        // Determine which applications are present across all users
        let mut apps = Vec::new();
        let mut offset = Self::REC_SIZE as u32;

        // Check if any user has each application type and build app list
        if users.iter().any(|u| u.alias.is_some()) {
            apps.push((AliasUserInf::NAME.to_string(), AliasUserInf::REC_SIZE, offset));
            offset += AliasUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.verify.is_some()) {
            apps.push((VerifyUserInf::NAME.to_string(), VerifyUserInf::REC_SIZE, offset));
            offset += VerifyUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.address.is_some()) {
            apps.push((AddressUserInf::NAME.to_string(), AddressUserInf::REC_SIZE, offset));
            offset += AddressUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.password.is_some()) {
            apps.push((PasswordUserInf::NAME.to_string(), PasswordUserInf::REC_SIZE, offset));
            offset += PasswordUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.call_stats.is_some()) {
            apps.push((CallStatsUserInf::NAME.to_string(), CallStatsUserInf::REC_SIZE, offset));
            offset += CallStatsUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.notes.is_some()) {
            apps.push((NotesUserInf::NAME.to_string(), NotesUserInf::REC_SIZE, offset));
            offset += NotesUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.qwk_config.is_some()) {
            apps.push((QwkConfigUserInf::NAME.to_string(), QwkConfigUserInf::REC_SIZE, offset));
            offset += QwkConfigUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.account.is_some()) {
            apps.push((AccountUserInf::NAME.to_string(), AccountUserInf::REC_SIZE, offset));
            offset += AccountUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.personal.is_some()) {
            apps.push((PersonalUserInf::NAME.to_string(), PersonalUserInf::REC_SIZE, offset));
            offset += PersonalUserInf::REC_SIZE as u32;
        }
        if users.iter().any(|u| u.bank.is_some()) {
            apps.push((BankUserInf::NAME.to_string(), BankUserInf::REC_SIZE, offset));
            offset += BankUserInf::REC_SIZE as u32;
        }

        let total_record_size = offset;

        // Write header (16 bytes)
        writer.write_u16::<LittleEndian>(6)?; // version
        writer.write_u16::<LittleEndian>(0)?; // num_conf
        writer.write_u16::<LittleEndian>(Self::REC_SIZE as u16)?; // base record size
        writer.write_u32::<LittleEndian>(0)?; // conf_size
        writer.write_u16::<LittleEndian>(apps.len() as u16)?; // number of applications
        writer.write_u32::<LittleEndian>(total_record_size)?; // total record size

        // Write application headers (32 bytes each)
        for (name, size, offset) in &apps {
            writer.write_all(&export_cp437_string(name, 15, 0))?; // app name
            writer.write_u16::<LittleEndian>(1)?; // version
            writer.write_u16::<LittleEndian>(*size as u16)?; // record size
            writer.write_u16::<LittleEndian>(0)?; // conf record size
            writer.write_all(&[0u8; 9])?; // keyword (unused)
            writer.write_u32::<LittleEndian>(*offset)?; // offset
        }

        // Write user records
        for user in users {
            user.write_with_apps(writer, &apps)?;
        }

        Ok(())
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;
        writer.write_all(&export_cp437_string(&self.alias, Self::REC_SIZE, b' '))?;
        Ok(())
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
        }
        let alias = convert_str(data);
        Ok(Self { verify: alias })
    }

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;
        writer.write_all(&export_cp437_string(&self.verify, Self::REC_SIZE, b' '))?;
        Ok(())
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;
        writer.write_all(&export_cp437_string(&self.street1, 50, b' '))?;
        writer.write_all(&export_cp437_string(&self.street2, 50, b' '))?;
        writer.write_all(&export_cp437_string(&self.city, 25, b' '))?;
        writer.write_all(&export_cp437_string(&self.state, 10, b' '))?;
        writer.write_all(&export_cp437_string(&self.zip, 10, b' '))?;
        writer.write_all(&export_cp437_string(&self.country, 15, b' '))?;
        Ok(())
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;
        use byteorder::{LittleEndian, WriteBytesExt};

        // Write 3 previous passwords
        for i in 0..3 {
            let pwd = self.prev_pwd.get(i).map(|s| s.as_str()).unwrap_or("");
            writer.write_all(&export_cp437_string(pwd, Self::PWD_LEN, b' '))?;
        }

        // Last password change date
        writer.write_u16::<LittleEndian>(self.last_change.to_pcboard_date() as u16)?;

        // Times changed
        writer.write_u16::<LittleEndian>(self.times_changed as u16)?;

        // Password expiration date
        writer.write_u16::<LittleEndian>(self.expire_date.to_pcboard_date() as u16)?;

        Ok(())
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        writer.write_i16::<LittleEndian>(self.first_date_on.to_pcboard_date() as i16)?;
        writer.write_u16::<LittleEndian>(self.num_sysop_pages as u16)?;
        writer.write_u16::<LittleEndian>(self.num_group_chats as u16)?;
        writer.write_u16::<LittleEndian>(self.num_comments as u16)?;
        writer.write_u16::<LittleEndian>(self.num300 as u16)?;
        writer.write_u16::<LittleEndian>(self.num1200 as u16)?;
        writer.write_u16::<LittleEndian>(self.num2400 as u16)?;
        writer.write_u16::<LittleEndian>(self.num9600 as u16)?;
        writer.write_u16::<LittleEndian>(self.num14400 as u16)?;
        writer.write_u16::<LittleEndian>(self.num_sec_viol as u16)?;
        writer.write_u16::<LittleEndian>(self.num_not_reg as u16)?;
        writer.write_u16::<LittleEndian>(self.num_reach_dnld_lim as u16)?;
        writer.write_u16::<LittleEndian>(self.num_file_not_found as u16)?;
        writer.write_u16::<LittleEndian>(self.num_pwrd_errors as u16)?;
        writer.write_u16::<LittleEndian>(self.num_verify_errors as u16)?;
        Ok(())
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;

        for i in 0..Self::NOTE_COUNT {
            let note = self.notes.get(i).map(|s| s.as_str()).unwrap_or("");
            writer.write_all(&export_cp437_string(note, Self::NOTE_SIZE, b' '))?;
        }
        Ok(())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        writer.write_u16::<LittleEndian>(self.max_msgs)?;
        writer.write_u16::<LittleEndian>(self.max_msgs_per_conf)?;
        writer.write_i16::<LittleEndian>(self.personal_attach_limit as i16)?;
        writer.write_i16::<LittleEndian>(self.public_attach_limit as i16)?;
        writer.write_i16::<LittleEndian>(self.new_blt_limit as i16)?;
        writer.write_u8(if self.new_files { 1 } else { 0 })?;

        // Pad to REC_SIZE (30 bytes total, we've written 11 bytes)
        writer.write_all(&[0u8; 19])?;

        Ok(())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        writer.write_f64::<LittleEndian>(self.starting_balance)?;
        writer.write_f64::<LittleEndian>(self.start_this_session)?;
        writer.write_f64::<LittleEndian>(self.debit_call)?;
        writer.write_f64::<LittleEndian>(self.debit_time)?;
        writer.write_f64::<LittleEndian>(self.debit_msg_read)?;
        writer.write_f64::<LittleEndian>(self.debit_msg_read_capture)?;
        writer.write_f64::<LittleEndian>(self.debit_msg_write)?;
        writer.write_f64::<LittleEndian>(self.debit_msg_write_echoed)?;
        writer.write_f64::<LittleEndian>(self.debit_msg_write_private)?;
        writer.write_f64::<LittleEndian>(self.debit_download_file)?;
        writer.write_f64::<LittleEndian>(self.debit_download_bytes)?;
        writer.write_f64::<LittleEndian>(self.debit_group_chat)?;
        writer.write_f64::<LittleEndian>(self.debit_tpu)?;
        writer.write_f64::<LittleEndian>(self.debit_special)?;
        writer.write_f64::<LittleEndian>(self.credit_upload_file)?;
        writer.write_f64::<LittleEndian>(self.credit_upload_bytes)?;
        writer.write_f64::<LittleEndian>(self.credit_special)?;
        writer.write_u8(self.drop_sec_level)?;
        Ok(())
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
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;

        // Gender - 1 byte
        let gender_byte = if self.gender.is_empty() {
            b' '
        } else {
            self.gender.chars().next().unwrap_or(' ') as u8
        };
        writer.write_all(&[gender_byte])?;

        // Birth date - 9 bytes
        let birth_str = if self.birth_date == IcbDate::default() {
            String::new()
        } else {
            self.birth_date.to_string()
        };
        writer.write_all(&export_cp437_string(&birth_str, 9, b' '))?;

        // Email - 60 bytes
        writer.write_all(&export_cp437_string(&self.email, 60, b' '))?;

        // Unknown - 61 bytes
        writer.write_all(&[0u8; 61])?;

        // Web - 60 bytes
        writer.write_all(&export_cp437_string(&self.web, 60, b' '))?;

        // Unknown - 61 bytes
        writer.write_all(&[0u8; 61])?;

        Ok(())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        writer.write_u32::<LittleEndian>(self.last_deposite_date.to_pcboard_date() as u32)?;
        writer.write_u32::<LittleEndian>(self.last_withdraw_date.to_pcboard_date() as u32)?;
        writer.write_u32::<LittleEndian>(self.last_transaction_amount)?;
        writer.write_u32::<LittleEndian>(self.amount_saved)?;
        writer.write_u32::<LittleEndian>(self.max_withdrawl_per_day)?;
        writer.write_u32::<LittleEndian>(self.max_stored_amount)?;
        Ok(())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankUserInf {
    pub time_info: BankInfo,
    pub byte_info: BankInfo,
}

impl BankUserInf {
    pub const NAME: &'static str = "PCBBANK";
    const REC_SIZE: usize = 48;

    pub fn read(data: &[u8]) -> Res<Self> {
        if Self::REC_SIZE != data.len() {
            return Err(Box::new(IcyBoardError::InvalidUserInfRecordSize(Self::NAME, Self::REC_SIZE, data.len())));
        }

        let mut cursor = Cursor::new(data);
        let time_info = BankInfo::read(&mut cursor)?;
        let byte_info = BankInfo::read(&mut cursor)?;

        Ok(Self { time_info, byte_info })
    }

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        self.time_info.write(writer)?;
        self.byte_info.write(writer)?;
        Ok(())
    }
}
