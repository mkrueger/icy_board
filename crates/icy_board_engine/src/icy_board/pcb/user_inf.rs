use crate::{Res, datetime::IcbDate};
use byteorder::{LittleEndian, ReadBytesExt};
use codepages::tables::CP437_TO_UNICODE;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    path::Path,
};

use crate::icy_board::IcyBoardError;
use crate::icy_board::user_base::ConferenceFlags;

#[derive(Debug, Clone)]
struct UserInfApplication {
    /// Name of application
    pub name: String,
    /// Version number
    pub version: u16,
    /// Size of application record in bytes
    pub size_of_rec: u16,
    /// Size of each application conference record
    pub size_of_conf_rec: u16,
    pub conference_count: usize,
    /// Keyword for executing the application
    pub keyword: String,
    /// Offset in user record where the data is stored
    pub offset: u32,
}

impl UserInfApplication {
    pub fn read(cursor: &mut Cursor<Vec<u8>>) -> Res<UserInfApplication> {
        let mut buf = [0; 15];
        cursor.read_exact(&mut buf)?;
        let name = convert_str(&buf);
        let version = cursor.read_u16::<LittleEndian>()?;
        let size_of_rec = cursor.read_u16::<LittleEndian>()?;
        let size_of_conf_rec = cursor.read_u16::<LittleEndian>()?;

        let mut buf = [0; 9];
        cursor.read_exact(&mut buf)?;
        let keyword = convert_str(&buf);
        let offset = cursor.read_u32::<LittleEndian>()?;

        Ok(UserInfApplication {
            name,
            version,
            size_of_rec,
            size_of_conf_rec,
            conference_count: 0,
            keyword,
            offset,
        })
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PcbTpaInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub keyword: String,
    pub version: u16,
    pub record_size: u16,
    pub conference_record_size: u16,
}

#[derive(Clone, Debug, Default)]
pub struct PcbTpaRecord {
    pub info: PcbTpaInfo,
    pub keyword: String,
    pub data: Vec<u8>,
    pub conferences: Vec<Vec<u8>>,
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
    pub conference_flags: HashMap<usize, ConferenceFlags>,
    pub lastread_ptrs: HashMap<usize, u32>,
    pub tpa_records: Vec<PcbTpaRecord>,
}

impl PcbUserInf {
    const REC_SIZE: usize = 33;
    fn read(data: &[u8]) -> Res<Self> {
        if data.len() < Self::REC_SIZE {
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
        let num_ext_conf = cursor.read_u16::<LittleEndian>()? as usize;
        let size_of_rec = cursor.read_u16::<LittleEndian>()? as usize;
        let conf_size = cursor.read_u32::<LittleEndian>()? as usize;
        let app_num = cursor.read_u16::<LittleEndian>()?;
        let rec_size = cursor.read_u32::<LittleEndian>()? as usize;

        let mut apps = Vec::new();
        for _ in 0..app_num {
            let inf = UserInfApplication::read(&mut cursor)?;
            apps.push(inf);
        }

        let records = &cursor.get_ref()[cursor.position() as usize..];
        if rec_size == 0 || size_of_rec < Self::REC_SIZE || size_of_rec > rec_size || records.len() % rec_size != 0 {
            return Err("Invalid USERS.INF record sizes or truncated user record".into());
        }
        let num_areas = 40 + num_ext_conf;
        let conference_end = size_of_rec.checked_add(conf_size).ok_or("Invalid USERS.INF conference size")?;
        if conference_end > rec_size {
            return Err("Truncated USERS.INF conference data".into());
        }
        for record in records.chunks_exact(rec_size) {
            let mut user = PcbUserInf::read(&record[..size_of_rec])?;
            let conference_data = &record[size_of_rec..conference_end];
            user.read_conferences(conference_data, num_ext_conf)?;

            for app in &apps {
                let start = app.offset as usize;
                if start < conference_end {
                    return Err(format!("USERS.INF application {} overlaps the PCBoard record", app.name).into());
                }
                let end = start.checked_add(app.size_of_rec as usize).ok_or("Invalid USERS.INF application size")?;
                let data = record.get(start..end).ok_or_else(|| format!("Truncated USERS.INF application {}", app.name))?;
                let boundary = apps
                    .iter()
                    .filter(|other| other.offset > app.offset)
                    .map(|other| other.offset as usize)
                    .min()
                    .unwrap_or(rec_size);
                let allocation_size = boundary
                    .checked_sub(end)
                    .ok_or_else(|| format!("Overlapping USERS.INF application {}", app.name))?;
                // Below 40 conferences, only allocation offsets expose the TPA conference count.
                let allocation_end = if app.size_of_conf_rec == 0 {
                    end
                } else {
                    let size = app.size_of_conf_rec as usize;
                    let count = allocation_size / size;
                    if allocation_size % size != 0 || count > num_areas || (num_ext_conf != 0 && count != num_areas) {
                        return Err(format!("Invalid USERS.INF conference allocation for {}", app.name).into());
                    }
                    boundary
                };
                let conference_data = record
                    .get(end..allocation_end)
                    .ok_or_else(|| format!("Truncated USERS.INF conference allocation for {}", app.name))?;
                let is_tpa = !matches!(
                    app.name.as_str(),
                    AliasUserInf::NAME
                        | VerifyUserInf::NAME
                        | AddressUserInf::NAME
                        | PasswordUserInf::NAME
                        | CallStatsUserInf::NAME
                        | NotesUserInf::NAME
                        | QwkConfigUserInf::NAME
                        | AccountUserInf::NAME
                        | PersonalUserInf::NAME
                        | BankUserInf::NAME
                );
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
                    _ => {}
                }
                if is_tpa || app.size_of_conf_rec != 0 {
                    let keyword = if app.keyword.is_empty() {
                        log::warn!("USERS.INF application {} has no keyword; preserving it under its application name", app.name);
                        app.name.clone()
                    } else {
                        app.keyword.clone()
                    };
                    user.tpa_records.push(PcbTpaRecord {
                        info: PcbTpaInfo {
                            name: app.name.clone(),
                            keyword: app.keyword.clone(),
                            version: app.version,
                            record_size: app.size_of_rec,
                            conference_record_size: app.size_of_conf_rec,
                        },
                        keyword,
                        data: data.to_vec(),
                        conferences: if app.size_of_conf_rec == 0 {
                            Vec::new()
                        } else {
                            conference_data.chunks_exact(app.size_of_conf_rec as usize).map(<[u8]>::to_vec).collect()
                        },
                    });
                }
            }
            users.push(user);
        }

        Ok(users)
    }

    fn read_conferences(&mut self, data: &[u8], num_ext_conf: usize) -> Res<()> {
        if data.is_empty() && num_ext_conf == 0 {
            return Ok(());
        }
        let num_areas = 40 + num_ext_conf;
        let bitmap_size = num_areas.div_ceil(8);
        let extended_size = bitmap_size - 5;
        let pointers_start = 2 * bitmap_size + 3 * extended_size;
        if data.len() < pointers_start + 4 * num_ext_conf {
            return Err("Truncated USERS.INF conference bitmaps or last-read pointers".into());
        }
        // USERSYS.DOC: full mail/sysop bitmaps, extended registration/expiry/scan bitmaps, then longs.
        for conference in 0..num_areas {
            let byte = conference / 8;
            let bit = 1 << (conference % 8);
            let mut flags = ConferenceFlags::None;
            if data[byte] & bit != 0 {
                flags |= ConferenceFlags::MailWaiting;
            }
            if data[bitmap_size + byte] & bit != 0 {
                flags |= ConferenceFlags::Sysop;
            }
            if conference >= 40 {
                for (index, flag) in [ConferenceFlags::Registered, ConferenceFlags::Expired, ConferenceFlags::Selected]
                    .into_iter()
                    .enumerate()
                {
                    if data[2 * bitmap_size + index * extended_size + byte - 5] & bit != 0 {
                        flags |= flag;
                    }
                }
                let offset = pointers_start + 4 * (conference - 40);
                let pointer = u32::from_le_bytes(data[offset..offset + 4].try_into()?);
                if pointer != 0 {
                    self.lastread_ptrs.insert(conference, pointer);
                }
            }
            if !flags.is_empty() {
                self.conference_flags.insert(conference, flags);
            }
        }
        Ok(())
    }

    fn write_with_apps(&self, writer: &mut impl std::io::Write, apps: &[UserInfApplication], num_ext_conf: usize) -> Res<()> {
        use crate::tables::export_cp437_string;
        use byteorder::{LittleEndian, WriteBytesExt};

        // Write base user info record (33 bytes)
        writer.write_all(&export_cp437_string(&self.name, 25, b' '))?;
        writer.write_u32::<LittleEndian>(self.messages_read as u32)?;
        writer.write_u32::<LittleEndian>(self.messages_left as u32)?;
        let bitmap_size = (40 + num_ext_conf).div_ceil(8);
        let extended_size = bitmap_size - 5;
        let mut conferences = vec![0; 2 * bitmap_size + 3 * extended_size];
        for (&conference, flags) in &self.conference_flags {
            let byte = conference / 8;
            let bit = 1 << (conference % 8);
            for (index, flag) in [ConferenceFlags::MailWaiting, ConferenceFlags::Sysop].into_iter().enumerate() {
                if flags.contains(flag) {
                    conferences[index * bitmap_size + byte] |= bit;
                }
            }
            if conference >= 40 {
                for (index, flag) in [ConferenceFlags::Registered, ConferenceFlags::Expired, ConferenceFlags::Selected]
                    .into_iter()
                    .enumerate()
                {
                    if flags.contains(flag) {
                        conferences[2 * bitmap_size + index * extended_size + byte - 5] |= bit;
                    }
                }
            }
        }
        writer.write_all(&conferences)?;
        for conference in 40..40 + num_ext_conf {
            writer.write_u32::<LittleEndian>(*self.lastread_ptrs.get(&conference).unwrap_or(&0))?;
        }

        // Write application sections in the order declared in the header
        for app in apps {
            let app_size = app.size_of_rec as usize;
            match app.name.as_str() {
                "PCBALIAS" => {
                    if let Some(alias) = &self.alias {
                        alias.write(writer)?;
                    } else {
                        // Write empty alias record
                        writer.write_all(&vec![b' '; app_size])?;
                    }
                }
                "PCBVERIFY" => {
                    if let Some(verify) = &self.verify {
                        verify.write(writer)?;
                    } else {
                        writer.write_all(&vec![b' '; app_size])?;
                    }
                }
                "PCBADDRESS" => {
                    if let Some(address) = &self.address {
                        address.write(writer)?;
                    } else {
                        writer.write_all(&vec![b' '; app_size])?;
                    }
                }
                "PCBPASSWORD" => {
                    if let Some(password) = &self.password {
                        password.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; app_size])?;
                    }
                }
                "PCBSTATS" => {
                    if let Some(call_stats) = &self.call_stats {
                        call_stats.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; app_size])?;
                    }
                }
                "PCBNOTES" => {
                    if let Some(notes) = &self.notes {
                        notes.write(writer)?;
                    } else {
                        writer.write_all(&vec![b' '; app_size])?;
                    }
                }
                QwkConfigUserInf::NAME => {
                    if let Some(qwk_config) = &self.qwk_config {
                        qwk_config.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; app_size])?;
                    }
                }
                "PCBACCOUNT" => {
                    if let Some(account) = &self.account {
                        account.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; app_size])?;
                    }
                }
                "PCBPERSONAL" => {
                    if let Some(personal) = &self.personal {
                        personal.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; app_size])?;
                    }
                }
                "PCBBANK" => {
                    if let Some(bank) = &self.bank {
                        bank.write(writer)?;
                    } else {
                        writer.write_all(&vec![0u8; app_size])?;
                    }
                }
                _ => {
                    let tpa = self.tpa_records.iter().find(|tpa| tpa.info.name == app.name);
                    let mut data = tpa.map_or_else(Vec::new, |tpa| tpa.data.clone());
                    data.resize(app_size, 0);
                    writer.write_all(&data)?;
                }
            }
            if app.size_of_conf_rec != 0 {
                let tpa = self.tpa_records.iter().find(|tpa| tpa.info.name == app.name);
                for conference in 0..app.conference_count {
                    let mut data = tpa.and_then(|tpa| tpa.conferences.get(conference)).cloned().unwrap_or_default();
                    data.resize(app.size_of_conf_rec as usize, 0);
                    writer.write_all(&data)?;
                }
            }
        }

        Ok(())
    }

    pub(crate) fn write_users(users: &[PcbUserInf], writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;
        use byteorder::{LittleEndian, WriteBytesExt};

        // Determine which applications are present across all users
        let mut num_areas = 40;
        for user in users {
            for conference in user.conference_flags.keys().chain(user.lastread_ptrs.keys()) {
                num_areas = num_areas.max(conference.checked_add(1).ok_or("Invalid PCBoard conference number")?);
            }
            for tpa in &user.tpa_records {
                num_areas = num_areas.max(tpa.conferences.len());
            }
        }
        if num_areas > u16::MAX as usize {
            return Err("Too many conferences for a PCBoard user file".into());
        }
        let num_ext_conf = num_areas - 40;
        let bitmap_size = num_areas.div_ceil(8);
        let conf_size = 2 * bitmap_size + 3 * (bitmap_size - 5) + 4 * num_ext_conf;
        let mut apps = Vec::new();
        let mut offset = (Self::REC_SIZE + conf_size) as u32;

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
        }

        let mut apps: Vec<_> = apps
            .into_iter()
            .map(|(name, size, offset)| UserInfApplication {
                name,
                version: 1,
                size_of_rec: size as u16,
                size_of_conf_rec: 0,
                conference_count: 0,
                keyword: String::new(),
                offset,
            })
            .collect();
        let psa_count = apps.len();
        for user in users {
            for tpa in &user.tpa_records {
                if tpa.info.name.chars().count() > 15 || tpa.keyword.chars().count() > 9 {
                    return Err("PCBoard application name or keyword is too long".into());
                }
                let size_of_rec = tpa.info.record_size.max(u16::try_from(tpa.data.len())?);
                let mut size_of_conf_rec = tpa.info.conference_record_size;
                for data in &tpa.conferences {
                    size_of_conf_rec = size_of_conf_rec.max(u16::try_from(data.len())?);
                }
                if let Some(index) = apps.iter().position(|app| app.name == tpa.info.name) {
                    let app = &mut apps[index];
                    if index < psa_count && size_of_rec != app.size_of_rec {
                        return Err(format!("Invalid fixed record size for supported allocation {}", tpa.info.name).into());
                    }
                    if index >= psa_count && (app.version != tpa.info.version || app.keyword != tpa.keyword) {
                        return Err(format!("Conflicting USERS.INF allocation for {}", tpa.info.name).into());
                    }
                    app.version = tpa.info.version;
                    app.keyword.clone_from(&tpa.keyword);
                    app.size_of_rec = app.size_of_rec.max(size_of_rec);
                    app.size_of_conf_rec = app.size_of_conf_rec.max(size_of_conf_rec);
                } else {
                    apps.push(UserInfApplication {
                        name: tpa.info.name.clone(),
                        version: tpa.info.version,
                        size_of_rec,
                        size_of_conf_rec,
                        conference_count: 0,
                        keyword: tpa.keyword.clone(),
                        offset: 0,
                    });
                }
            }
        }
        offset = (Self::REC_SIZE + conf_size) as u32;
        for app in &mut apps {
            app.conference_count = if num_ext_conf == 0 {
                users
                    .iter()
                    .flat_map(|user| &user.tpa_records)
                    .filter(|tpa| tpa.info.name == app.name)
                    .map(|tpa| tpa.conferences.len())
                    .max()
                    .unwrap_or(0)
            } else {
                num_areas
            };
            app.offset = offset;
            offset = offset
                .checked_add(u32::from(app.size_of_rec) + u32::from(app.size_of_conf_rec) * app.conference_count as u32)
                .ok_or("USERS.INF record is too large")?;
        }
        let total_record_size = offset;

        // Write header (16 bytes)
        writer.write_u16::<LittleEndian>(6)?; // version
        writer.write_u16::<LittleEndian>(num_ext_conf as u16)?;
        writer.write_u16::<LittleEndian>(Self::REC_SIZE as u16)?; // base record size
        writer.write_u32::<LittleEndian>(conf_size as u32)?;
        writer.write_u16::<LittleEndian>(u16::try_from(apps.len())?)?;
        writer.write_u32::<LittleEndian>(total_record_size)?; // total record size

        // Application descriptors are 34 bytes, including the conference allocation.
        for app in &apps {
            writer.write_all(&export_cp437_string(&app.name, 15, 0))?;
            writer.write_u16::<LittleEndian>(app.version)?;
            writer.write_u16::<LittleEndian>(app.size_of_rec)?;
            writer.write_u16::<LittleEndian>(app.size_of_conf_rec)?;
            writer.write_all(&export_cp437_string(&app.keyword, 9, 0))?;
            writer.write_u32::<LittleEndian>(app.offset)?;
        }

        // Write user records
        for user in users {
            user.write_with_apps(writer, &apps, num_ext_conf)?;
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
        let last_change = IcbDate::from_pcboard_full(u16::from_le_bytes([data[i], data[i + 1]]) as u32);
        i += 2;
        let times_changed = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        i += 2;
        let expire_date = u16::from_le_bytes([data[i], data[i + 1]]) as u32;

        Ok(Self {
            prev_pwd: [pwd1, pwd2, pwd3],
            last_change,
            times_changed,
            expire_date: IcbDate::from_pcboard_full(expire_date),
        })
    }

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use crate::tables::export_cp437_string;
        use byteorder::{LittleEndian, WriteBytesExt};

        // Write 3 previous passwords
        for i in 0..3 {
            let pwd = self.prev_pwd.get(i).map_or("", std::string::String::as_str);
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
        let first_date_on = cursor.read_u16::<LittleEndian>()? as u32;
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
            first_date_on: IcbDate::from_pcboard_full(first_date_on),
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

        writer.write_u16::<LittleEndian>(self.first_date_on.to_pcboard_date() as u16)?;
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
            let note = self.notes.get(i).map_or("", std::string::String::as_str);
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
        writer.write_u8(u8::from(self.new_files))?;

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

    /// Validate all monetary storage, including the two balance snapshots.
    pub fn validate(&self) -> Res<()> {
        let values = [
            self.starting_balance,
            self.start_this_session,
            self.debit_call,
            self.debit_time,
            self.debit_msg_read,
            self.debit_msg_read_capture,
            self.debit_msg_write,
            self.debit_msg_write_echoed,
            self.debit_msg_write_private,
            self.debit_download_file,
            self.debit_download_bytes,
            self.debit_group_chat,
            self.debit_tpu,
            self.debit_special,
            self.credit_upload_file,
            self.credit_upload_bytes,
            self.credit_special,
        ];
        for (field, value) in values.into_iter().enumerate() {
            if !value.is_finite() {
                return Err(format!("Accounting field {field} must be finite").into());
            }
        }
        Ok(())
    }

    /// Remaining balance, including unposted global time charges.
    /// Concurrent tracking bills the largest debit category, not concurrent sessions.
    /// Invalid storage/pending time or arithmetic overflow returns NaN;
    /// callers should validate at input boundaries.
    pub fn balance(&self, concurrent: bool, pending_time: f64) -> f64 {
        if self.validate().is_err() || !pending_time.is_finite() {
            return f64::NAN;
        }
        let debits = [
            self.debit_time + pending_time,
            self.debit_call,
            self.debit_msg_read,
            self.debit_msg_read_capture,
            self.debit_msg_write,
            self.debit_msg_write_echoed,
            self.debit_msg_write_private,
            self.debit_download_file,
            self.debit_download_bytes,
            self.debit_group_chat,
            self.debit_tpu,
            self.debit_special,
        ];
        // Check after adding pending time: max could hide negative overflow.
        if debits.iter().any(|debit| !debit.is_finite()) {
            return f64::NAN;
        }
        let spending = if concurrent {
            debits.into_iter().fold(f64::NEG_INFINITY, f64::max)
        } else {
            debits.into_iter().sum()
        };
        if !spending.is_finite() {
            return f64::NAN;
        }
        let balance = self.starting_balance - (spending - self.credit_upload_file - self.credit_upload_bytes - self.credit_special);
        if balance.is_finite() { balance } else { f64::NAN }
    }

    /// Add a signed amount to a PPL accounting debit/credit field (2 through 16).
    /// Invalid fields, invalid existing storage and overflow leave the account unchanged.
    pub fn apply_charge(&mut self, field: usize, amount: f64) -> Res<()> {
        self.validate()?;
        if !amount.is_finite() {
            return Err("Accounting charge must be finite".into());
        }
        let storage = match field {
            2 => &mut self.debit_call,
            3 => &mut self.debit_time,
            4 => &mut self.debit_msg_read,
            5 => &mut self.debit_msg_read_capture,
            6 => &mut self.debit_msg_write,
            7 => &mut self.debit_msg_write_echoed,
            8 => &mut self.debit_msg_write_private,
            9 => &mut self.debit_download_file,
            10 => &mut self.debit_download_bytes,
            11 => &mut self.debit_group_chat,
            12 => &mut self.debit_tpu,
            13 => &mut self.debit_special,
            14 => &mut self.credit_upload_file,
            15 => &mut self.credit_upload_bytes,
            16 => &mut self.credit_special,
            _ => return Err(format!("Accounting charge field {field} is outside 2..=16").into()),
        };
        let total = *storage + amount;
        if !total.is_finite() {
            return Err("Accounting charge overflows its storage field".into());
        }
        *storage = total;
        Ok(())
    }

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

        let account = Self {
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
        };
        account.validate()?;
        Ok(account)
    }

    fn write(&self, writer: &mut impl std::io::Write) -> Res<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        self.validate()?;
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

        // Birth date - 9 bytes, MMDDYYYY so the century survives the field width
        let birth_str = if self.birth_date == IcbDate::default() {
            String::new()
        } else {
            format!("{:02}{:02}{:04}", self.birth_date.month(), self.birth_date.day(), self.birth_date.year())
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
            last_deposite_date: IcbDate::from_pcboard_full(last_deposite_date),
            last_withdraw_date: IcbDate::from_pcboard_full(last_withdraw_date),
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
