use std::{
    collections::HashMap,
    fs,
    ops::{Deref, DerefMut, Index, IndexMut},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    Res,
    datetime::IcbDate,
    icy_board::{
        user_inf::{AddressUserInf, AliasUserInf, CallStatsUserInf, NotesUserInf, PasswordUserInf, PcbUserInf, PersonalUserInf, VerifyUserInf},
        users::PcbUserRecord,
    },
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use bitflag::bitflag;
use chrono::{DateTime, Utc, format};
use serde::{Deserialize, Serialize};

use super::{
    IcyBoardSerializer, PcbUser,
    icb_config::{DEFAULT_PCBOARD_DATE_FORMAT, PasswordStorageMethod},
    is_false, is_null_16, is_null_64,
    user_inf::{AccountUserInf, BankUserInf, QwkConfigUserInf},
};

#[derive(Clone, Debug)]
pub enum Password {
    PlainText(String),
    Argon2(String),
}

impl PartialEq for Password {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PlainText(l0), Self::PlainText(r0)) => l0 == r0,
            // Argon2 to Argon2: exact hash comparison
            (Self::Argon2(l0), Self::Argon2(r0)) => {
                log::warn!("Comparing two Argon2 hashes directly. This is not recommended.");
                l0 == r0
            }

            // Plain to Argon2: verify the plain password against the hash
            (Self::PlainText(plain), Self::Argon2(hash)) | (Self::Argon2(hash), Self::PlainText(plain)) => {
                if let Ok(parsed_hash) = PasswordHash::new(hash) {
                    Argon2::default().verify_password(plain.as_bytes(), &parsed_hash).is_ok()
                } else {
                    false
                }
            }
        }
    }
}

impl Default for Password {
    fn default() -> Self {
        Password::PlainText(String::new())
    }
}

impl Password {
    pub fn from_str(s: &str) -> Res<Self> {
        Ok(Password::PlainText(s.to_string()))
    }

    pub fn to_argon2(&self) -> Self {
        match self {
            Password::PlainText(text) => {
                let argon2 = Argon2::default();
                let salt = SaltString::generate(&mut OsRng);

                let password_hash = argon2.hash_password(text.to_lowercase().as_bytes(), &salt).unwrap().to_string();
                Password::Argon2(password_hash)
            }

            Password::Argon2(x) => Password::Argon2(x.clone()),
        }
    }

    pub fn new_argon2(str: impl Into<String>) -> Password {
        Password::PlainText(str.into()).to_argon2()
    }
}

impl std::fmt::Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Password::PlainText(s) => write!(f, "{}", s),
            Password::Argon2(_s) => write!(f, "******"),
        }
    }
}

impl Password {
    pub fn is_empty(&self) -> bool {
        match self {
            Password::PlainText(s) => s.is_empty(),
            Password::Argon2(s) => s.is_empty(),
        }
    }

    pub fn is_valid(&self, pwd: &str) -> bool {
        self == &Password::PlainText(pwd.to_lowercase().to_string())
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|p| {
            if p.starts_with("$argon2") {
                Password::Argon2(p)
            } else {
                if p.len() >= 2 && p.starts_with('"') && p.ends_with('"') {
                    Password::PlainText(p[1..p.len() - 1].to_string())
                } else {
                    // Plain text password without quotes (legacy)
                    Password::PlainText(p)
                }
            }
        })
    }
}

impl serde::Serialize for Password {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Password::PlainText(key) => format!("\"{key}\"").serialize(serializer),
            Password::Argon2(key) => key.serialize(serializer),
        }
    }
}

impl FromStr for Password {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Password::PlainText(s.to_string()))
    }
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct PasswordInfo {
    pub password: Password,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prev_pwd: Vec<Password>,

    #[serde(default)]
    pub last_change: DateTime<Utc>,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub times_changed: u64,

    #[serde(default)]
    pub expire_date: DateTime<Utc>,

    #[serde(default)]
    #[serde(skip_serializing_if = "PasswordStorageMethod::is_default")]
    pub password_storage_method: PasswordStorageMethod,
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserStats {
    /// First date on
    #[serde(default)]
    pub first_date_on: DateTime<Utc>,

    #[serde(default)]
    pub last_on: DateTime<Utc>,

    /// Number of times the caller has connected
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_times_on: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub messages_read: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub messages_left: u64,

    /// Number of security violations
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_sec_viol: u64,
    /// Number of unregistered conference attempts
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_not_reg: u64,
    /// # Download limit reached
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_reach_dnld_lim: u64,
    /// # Download file not found
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_file_not_found: u64,
    /// # Password failures
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_password_failures: u64,
    /// # Upload verification failed
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_verify_errors: u64,

    /// Times of paged sysop
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_sysop_pages: u64,
    /// Times of group chat
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_group_chats: u64,
    /// Times of comments to sysop
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_comments: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_uploads: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub num_downloads: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub total_dnld_bytes: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub total_upld_bytes: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub today_num_downloads: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub today_num_uploads: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub today_dnld_bytes: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub today_upld_bytes: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub total_doors_executed: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub minutes_today: u16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum FSEMode {
    #[default]
    Yes,
    No,
    Ask,
}

impl FSEMode {
    pub fn from_pcboard(s: &str) -> Self {
        match s {
            "Y" => FSEMode::Yes,
            "N" => FSEMode::No,
            _ => FSEMode::Ask,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            FSEMode::Yes => 'Y',
            FSEMode::No => 'N',
            _ => 'A',
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum ChatStatus {
    #[default]
    Available,
    Unavailable,
}

impl ChatStatus {
    pub fn from_pcboard(s: &str) -> Self {
        match s {
            "U" => ChatStatus::Unavailable,
            _ => ChatStatus::Available,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            ChatStatus::Unavailable => 'U',
            _ => 'A',
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct UserFlags {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub expert_mode: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub is_dirty: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub msg_clear: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_mail: bool,

    #[serde(default)]
    pub fse_mode: FSEMode,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub scroll_msg_body: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_short_filedescr: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub long_msg_header: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub wide_editor: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub delete_flag: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub disabled_flag: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_graphics: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_alias: bool,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Path to the user file
    pub path: Option<PathBuf>,

    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub alias: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub verify_answer: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub city_or_state: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub city: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub state: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub street1: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub street2: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub zip: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub country: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub gender: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub email: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub web: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date_format: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub language: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bus_data_phone: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub home_voice_phone: String,

    pub birth_day: Option<IcbDate>,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub user_comment: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sysop_comment: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub custom_comment1: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub custom_comment2: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub custom_comment3: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub custom_comment4: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub custom_comment5: String,

    pub password: PasswordInfo,

    pub security_level: u8,

    #[serde(default)]
    pub exp_date: IcbDate,

    /// Expired security level
    pub exp_security_level: u8,

    pub flags: UserFlags,

    /// Protocol (A->Z)
    pub protocol: String,

    /// Page length when display data on the screen
    pub page_len: u16,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub last_conference: u16,

    /// Number of minutes online
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub elapsed_time_on: u16,

    /// Date for last DIR Scan (most recent file)
    #[serde(default)]
    pub date_last_dir_read: DateTime<Utc>,

    pub qwk_config: Option<QwkConfigUserInf>,
    pub account: Option<AccountUserInf>,
    pub bank: Option<BankUserInf>,

    pub stats: UserStats,

    #[serde(default)]
    pub chat_status: ChatStatus,

    #[serde(default)]
    #[serde(with = "conference_flags_format")]
    pub conference_flags: HashMap<usize, ConferenceFlags>,

    #[serde(default)]
    #[serde(with = "lastread_ptr_flags")]
    pub lastread_ptr_flags: HashMap<(usize, usize), LastReadStatus>,
}

#[bitflag(u8)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum ConferenceFlags {
    None = 0b0000_0000,
    UserSelected = 0b0000_0001,
    Registered = 0b0000_0010,
    Expired = 0b0000_0100,

    CON = 0b0000_1000,
    MFL = 0b0001_0000,
    NET = 0b0010_0000,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct LastReadStatus {
    pub last_read: usize,
    pub highest_msg_read: usize,
    pub include_qwk: bool,
}

impl Default for LastReadStatus {
    fn default() -> Self {
        Self {
            last_read: 0,
            highest_msg_read: 0,
            include_qwk: true,
        }
    }
}

mod lastread_ptr_flags {
    use std::collections::HashMap;

    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::LastReadStatus;

    pub fn serialize<S>(date: &HashMap<(usize, usize), LastReadStatus>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = String::new();
        for ((conf, area), v) in date {
            // Only these flags get stored in PCBoard - rest is for use at runtime.
            s.push_str(&format!(
                "{},{},{},{},{};",
                conf,
                area,
                v.last_read,
                v.highest_msg_read,
                if v.include_qwk { 1 } else { 0 }
            ));
        }
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<(usize, usize), LastReadStatus>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut map = HashMap::new();
        s.split(';').for_each(|item| {
            if item.is_empty() {
                return;
            }
            let mut iter = item.split(',');
            if let (Some(c), Some(a), Some(lr), Some(hr), Some(flags)) = (iter.next(), iter.next(), iter.next(), iter.next(), iter.next()) {
                if let (Ok(c), Ok(a), Ok(lr), Ok(hr), Ok(flags)) = (
                    c.parse::<usize>(),
                    a.parse::<usize>(),
                    lr.parse::<usize>(),
                    hr.parse::<usize>(),
                    flags.parse::<usize>(),
                ) {
                    map.insert(
                        (c, a),
                        LastReadStatus {
                            last_read: lr,
                            highest_msg_read: hr,
                            include_qwk: flags == 1,
                        },
                    );
                }
            }
        });
        Ok(map)
    }
}

mod conference_flags_format {
    use std::collections::HashMap;

    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::ConferenceFlags;

    pub fn serialize<S>(date: &HashMap<usize, ConferenceFlags>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = String::new();
        for (k, v) in date {
            if v.is_empty() {
                continue;
            }
            // Only these flags get stored in PCBoard - rest is for use at runtime.
            let v = *v & (ConferenceFlags::UserSelected | ConferenceFlags::Registered | ConferenceFlags::Expired);
            s.push_str(&format!("{}:{};", k, v.bits()));
        }
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<usize, ConferenceFlags>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut map = HashMap::new();
        s.split(';').for_each(|item| {
            if item.is_empty() {
                return;
            }
            let mut iter = item.split(':');
            if let (Some(k), Some(v)) = (iter.next(), iter.next()) {
                if let (Ok(k), Ok(v)) = (k.parse::<usize>(), v.parse::<u8>()) {
                    map.insert(k, ConferenceFlags::from_bits_truncate(v));
                }
            }
        });
        Ok(map)
    }
}

impl User {
    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_first_name(&self) -> String {
        if let Some(idx) = self.name.find(' ') {
            self.name[..idx].to_string()
        } else {
            self.name.clone()
        }
    }

    pub fn get_last_name(&self) -> String {
        if let Some(idx) = self.name.find(' ') {
            self.name[idx + 1..].to_string()
        } else {
            String::new()
        }
    }

    fn import_pcb(u: &PcbUser) -> Self {
        let alias = if let Some(alias) = &u.inf.alias { alias.alias.clone() } else { String::new() };
        let verify = if let Some(verify) = &u.inf.verify {
            verify.verify.clone()
        } else {
            String::new()
        };

        let (gender, birth_date, email, web) = if let Some(personal) = &u.inf.personal {
            (
                personal.gender.clone(),
                personal.birth_date.clone(),
                personal.email.clone(),
                personal.web.clone(),
            )
        } else {
            (String::new(), IcbDate::new(0, 0, 0), String::new(), String::new())
        };

        let (street1, street2, city, state, zip, country) = if let Some(address) = &u.inf.address {
            (
                address.street1.clone(),
                address.street2.clone(),
                address.city.clone(),
                address.state.clone(),
                address.zip.clone(),
                address.country.clone(),
            )
        } else {
            (String::new(), String::new(), String::new(), String::new(), String::new(), String::new())
        };

        let (prev_pwd, last_change, times_changed, expire_date) = if let Some(password) = &u.inf.password {
            (
                password
                    .prev_pwd
                    .iter()
                    .filter(|s| !s.is_empty())
                    .map(|pwd| Password::from_str(pwd).unwrap())
                    .collect(),
                password.last_change.clone(),
                password.times_changed,
                password.expire_date.clone(),
            )
        } else {
            (Vec::new(), IcbDate::new(0, 0, 0), 0, IcbDate::new(0, 0, 0))
        };

        let (
            first_date_on,
            num_sysop_pages,
            num_group_chats,
            num_comments,
            num_sec_viol,
            num_not_reg,
            num_reach_dnld_lim,
            num_file_not_found,
            num_pwrd_errors,
            num_verify_errors,
        ) = if let Some(stats) = &u.inf.call_stats {
            (
                stats.first_date_on.clone(),
                stats.num_sysop_pages,
                stats.num_group_chats,
                stats.num_comments,
                stats.num_sec_viol,
                stats.num_not_reg,
                stats.num_reach_dnld_lim,
                stats.num_file_not_found,
                stats.num_pwrd_errors,
                stats.num_verify_errors,
            )
        } else {
            // Fake creation date. IcyBoard sorts users by this date. This should mimic the order from pcboard.
            (IcbDate::new(1, 1, 1980 + u.user.rec_num as u16), 0, 0, 0, 0, 0, 0, 0, 0, 0)
        };
        let mut custom_comment1 = String::new();
        let mut custom_comment2 = String::new();
        let mut custom_comment3 = String::new();
        let mut custom_comment4 = String::new();
        let mut custom_comment5 = String::new();

        if let Some(notes) = &u.inf.notes {
            custom_comment1 = notes.notes.get(0).unwrap_or(&String::new()).clone();
            custom_comment2 = notes.notes.get(1).unwrap_or(&String::new()).clone();
            custom_comment3 = notes.notes.get(2).unwrap_or(&String::new()).clone();
            custom_comment4 = notes.notes.get(3).unwrap_or(&String::new()).clone();
            custom_comment5 = notes.notes.get(4).unwrap_or(&String::new()).clone();
        };

        let qwk_config = u.inf.qwk_config.clone();
        let account = u.inf.account.clone();
        let bank = u.inf.bank.clone();

        let mut conference_flags = HashMap::new();

        for i in 0..5 {
            for j in 0..8 {
                let reg = u.user.conf_reg_flags[i] & (1 << j) != 0;
                let exp = u.user.conf_exp_flags[i] & (1 << j) != 0;
                let usr = u.user.conf_usr_flags[i] & (1 << j) != 0;

                let mut flag = ConferenceFlags::None;
                if exp {
                    flag |= ConferenceFlags::Expired;
                }
                if reg {
                    flag |= ConferenceFlags::Registered;
                }
                if usr {
                    flag |= ConferenceFlags::UserSelected;
                }

                if !flag.is_empty() {
                    conference_flags.insert(i * 8 + j, flag);
                }
            }
        }

        let mut lastread_ptr_flags = HashMap::new();
        for (i, lmr) in u.user.last_message_read_ptr.iter().enumerate() {
            if *lmr == 0 {
                continue;
            }
            lastread_ptr_flags.insert(
                (i, 0),
                LastReadStatus {
                    last_read: *lmr as usize,
                    highest_msg_read: *lmr as usize,
                    include_qwk: true,
                },
            );
        }

        // for x in 0..u.user.

        Self {
            path: None,
            name: u.user.name.clone(),
            alias,
            verify_answer: verify,
            city_or_state: u.user.city.clone(),

            date_format: DEFAULT_PCBOARD_DATE_FORMAT.to_string(),
            gender,
            birth_day: Some(birth_date),
            email,
            web,

            city,
            street1,
            street2,
            state,
            zip,
            country,

            custom_comment1,
            custom_comment2,
            custom_comment3,
            custom_comment4,
            custom_comment5,

            password: PasswordInfo {
                password: Password::from_str(&u.user.password).unwrap(),
                prev_pwd,
                last_change: last_change.to_utc_date_time(),
                times_changed: times_changed as u64,
                expire_date: expire_date.to_utc_date_time(),
                password_storage_method: PasswordStorageMethod::PlainText,
            },

            qwk_config,
            account,
            bank,

            bus_data_phone: u.user.bus_data_phone.clone(),
            home_voice_phone: u.user.home_voice_phone.clone(),
            user_comment: u.user.user_comment.clone(),
            sysop_comment: u.user.sysop_comment.clone(),
            security_level: u.user.security_level as u8,
            exp_date: u.user.exp_date.clone(),
            exp_security_level: u.user.exp_security_level as u8,
            flags: UserFlags {
                expert_mode: u.user.expert_mode,
                is_dirty: u.user.is_dirty,
                msg_clear: u.user.msg_clear,
                has_mail: u.user.has_mail,
                fse_mode: if u.user.use_fsedefault {
                    FSEMode::Yes
                } else {
                    if u.user.dont_ask_fse { FSEMode::No } else { FSEMode::Ask }
                },
                scroll_msg_body: u.user.scroll_msg_body,
                use_short_filedescr: u.user.short_file_descr,
                long_msg_header: u.user.long_msg_header,
                wide_editor: u.user.wide_editor,
                delete_flag: u.user.delete_flag,
                use_graphics: true,
                disabled_flag: false,
                use_alias: false,
            },
            protocol: u.user.protocol.to_string(),
            page_len: u.user.page_len as u16,
            last_conference: u.user.last_conference,
            elapsed_time_on: u.user.elapsed_time_on,
            date_last_dir_read: u.user.date_last_dir_read.to_utc_date_time(),
            chat_status: if u.user.is_chat_available {
                ChatStatus::Available
            } else {
                ChatStatus::Unavailable
            },
            language: String::new(),
            stats: UserStats {
                first_date_on: first_date_on.to_utc_date_time(),
                last_on: u.user.last_date_on.to_utc_date_time(),
                num_times_on: u.user.num_times_on as u64,
                messages_read: u.user.num_times_on as u64,
                messages_left: u.user.num_times_on as u64,
                num_sysop_pages: num_sysop_pages as u64,
                num_group_chats: num_group_chats as u64,
                num_comments: num_comments as u64,
                num_sec_viol: num_sec_viol as u64,
                num_not_reg: num_not_reg as u64,
                num_reach_dnld_lim: num_reach_dnld_lim as u64,
                num_file_not_found: num_file_not_found as u64,
                num_password_failures: num_pwrd_errors as u64,
                num_verify_errors: num_verify_errors as u64,
                num_uploads: u.user.num_uploads as u64,
                num_downloads: u.user.num_downloads as u64,
                total_dnld_bytes: u.user.ul_tot_dnld_bytes,
                total_upld_bytes: u.user.ul_tot_upld_bytes,
                today_dnld_bytes: u.user.daily_downloaded_bytes as u64,
                today_upld_bytes: 0,
                today_num_downloads: 0,
                today_num_uploads: 0,
                total_doors_executed: 0,
                minutes_today: 0,
            },
            conference_flags,
            lastread_ptr_flags,
        }
    }

    pub fn is_valid_loginname(&self, name: &str) -> bool {
        let name = name.trim();
        self.name.eq_ignore_ascii_case(name) || self.alias.eq_ignore_ascii_case(name)
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn save(&self, _home_dir: &Path) -> Res<()> {
        let user_txt = toml::to_string(self)?;
        if let Some(path) = self.path.as_ref() {
            fs::write(path, user_txt)?;
        } /* else {
        let home_dir = UserBase::get_user_home_dir(home_dir, self.get_name());
        std::fs::create_dir_all(&home_dir)?;
        fs::write(home_dir.join("user.toml"), user_txt)?;
        }*/
        Ok(())
    }

    fn to_pcboard(&self) -> PcbUser {
        // Build base PCBoard user record
        let mut rec = PcbUserRecord {
            name: self.name.clone(),
            password: self.password.password.to_string(),
            city: self.city_or_state.clone(),
            bus_data_phone: self.bus_data_phone.clone(),
            home_voice_phone: self.home_voice_phone.clone(),
            user_comment: self.user_comment.clone(),
            sysop_comment: self.sysop_comment.clone(),
            security_level: self.security_level as u8,
            exp_date: self.exp_date.clone(),
            exp_security_level: self.exp_security_level as u8,
            expert_mode: self.flags.expert_mode,
            is_dirty: self.flags.is_dirty,
            msg_clear: self.flags.msg_clear,
            has_mail: self.flags.has_mail,
            use_fsedefault: matches!(self.flags.fse_mode, FSEMode::Yes),
            dont_ask_fse: matches!(self.flags.fse_mode, FSEMode::No),
            scroll_msg_body: self.flags.scroll_msg_body,
            short_file_descr: self.flags.use_short_filedescr,
            long_msg_header: self.flags.long_msg_header,
            wide_editor: self.flags.wide_editor,
            delete_flag: self.flags.delete_flag,
            protocol: self.protocol.chars().next().unwrap_or('Z'),
            page_len: self.page_len as u8,
            last_conference: self.last_conference,
            elapsed_time_on: self.elapsed_time_on,
            date_last_dir_read: IcbDate::from_utc(self.date_last_dir_read),
            is_chat_available: matches!(self.chat_status, ChatStatus::Available),
            last_date_on: IcbDate::from_utc(self.stats.last_on),
            num_times_on: self.stats.num_times_on as usize,
            num_uploads: self.stats.num_uploads as i32,
            num_downloads: self.stats.num_downloads as i32,
            ul_tot_dnld_bytes: self.stats.total_dnld_bytes,
            ul_tot_upld_bytes: self.stats.total_upld_bytes,
            daily_downloaded_bytes: self.stats.today_dnld_bytes as usize,
            // Arrays and remaining fields defaulted:
            last_message_read_ptr: [0; 40].to_vec(),
            ..Default::default()
        };

        // Pack conference flags (0..40) into 5 bytes each
        for (conf, flags) in &self.conference_flags {
            if *conf >= 40 {
                continue;
            }
            let byte = conf / 8;
            let bit = conf % 8;
            if flags.contains(ConferenceFlags::Registered) {
                rec.conf_reg_flags[byte] |= 1 << bit;
            }
            if flags.contains(ConferenceFlags::Expired) {
                rec.conf_exp_flags[byte] |= 1 << bit;
            }
            if flags.contains(ConferenceFlags::UserSelected) {
                rec.conf_usr_flags[byte] |= 1 << bit;
            }
        }

        // Last message read pointers
        for ((conf, _area), status) in &self.lastread_ptr_flags {
            if *conf >= 40 {
                continue;
            }
            rec.last_message_read_ptr[*conf] = status.last_read as i32;
            //            rec.last_read_high_msg_read[*conf] = status.highest_msg_read as u32;
        }

        // Build extended INF structure
        let mut inf = PcbUserInf { ..Default::default() };

        if !self.alias.is_empty() {
            inf.alias = Some(AliasUserInf { alias: self.alias.clone() });
        }
        if !self.verify_answer.is_empty() {
            inf.verify = Some(VerifyUserInf {
                verify: self.verify_answer.clone(),
            });
        }
        if !(self.street1.is_empty()
            && self.street2.is_empty()
            && self.city.is_empty()
            && self.state.is_empty()
            && self.zip.is_empty()
            && self.country.is_empty())
        {
            inf.address = Some(AddressUserInf {
                street1: self.street1.clone(),
                street2: self.street2.clone(),
                city: self.city.clone(),
                state: self.state.clone(),
                zip: self.zip.clone(),
                country: self.country.clone(),
            });
        }
        if !(self.gender.is_empty() && self.birth_day.is_none() && self.email.is_empty() && self.web.is_empty()) {
            inf.personal = Some(PersonalUserInf {
                gender: self.gender.clone(),
                birth_date: self.birth_day.clone().unwrap_or_else(|| IcbDate::new(0, 0, 0)),
                email: self.email.clone(),
                web: self.web.clone(),
            });
        }
        if (!self.password.prev_pwd.is_empty()) || self.password.times_changed > 0 {
            // PCBoard expects exactly 3 previous passwords, pad with empty strings if needed
            let mut prev = Vec::new();
            for i in 0..3 {
                if i < self.password.prev_pwd.len() {
                    prev.push(self.password.prev_pwd[i].to_string());
                } else {
                    prev.push(String::new());
                }
            }
            inf.password = Some(PasswordUserInf {
                prev_pwd: prev.try_into().expect("prev should have exactly 3 elements"),
                last_change: IcbDate::from_utc(self.password.last_change),
                times_changed: self.password.times_changed as usize,
                expire_date: IcbDate::from_utc(self.password.expire_date),
            });
        }

        // Call statistics (mirror fields used on import)
        let any_stats = self.stats.first_date_on.timestamp() != 0
            || self.stats.num_sysop_pages > 0
            || self.stats.num_group_chats > 0
            || self.stats.num_comments > 0
            || self.stats.num_sec_viol > 0
            || self.stats.num_not_reg > 0
            || self.stats.num_reach_dnld_lim > 0
            || self.stats.num_file_not_found > 0
            || self.stats.num_password_failures > 0
            || self.stats.num_verify_errors > 0;

        if any_stats {
            inf.call_stats = Some(CallStatsUserInf {
                first_date_on: IcbDate::from_utc(self.stats.first_date_on),
                num_sysop_pages: self.stats.num_sysop_pages as usize,
                num_group_chats: self.stats.num_group_chats as usize,
                num_comments: self.stats.num_comments as usize,
                num_sec_viol: self.stats.num_sec_viol as usize,
                num_not_reg: self.stats.num_not_reg as usize,
                num_reach_dnld_lim: self.stats.num_reach_dnld_lim as usize,
                num_file_not_found: self.stats.num_file_not_found as usize,
                num_pwrd_errors: self.stats.num_password_failures as usize,
                num_verify_errors: self.stats.num_verify_errors as usize,
                ..Default::default()
            });
        }

        if !(self.custom_comment1.is_empty()
            && self.custom_comment2.is_empty()
            && self.custom_comment3.is_empty()
            && self.custom_comment4.is_empty()
            && self.custom_comment5.is_empty())
        {
            inf.notes = Some(NotesUserInf {
                notes: vec![
                    self.custom_comment1.clone(),
                    self.custom_comment2.clone(),
                    self.custom_comment3.clone(),
                    self.custom_comment4.clone(),
                    self.custom_comment5.clone(),
                ],
            });
        }

        // Direct passthrough optional sections
        inf.qwk_config = self.qwk_config.clone();
        inf.account = self.account.clone();
        inf.bank = self.bank.clone();

        PcbUser { user: rec, inf }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserBase {
    users: Vec<User>,
}

impl UserBase {
    pub fn len(&self) -> usize {
        self.users.len()
    }

    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    pub fn import_pcboard(pcb_user: &[PcbUser]) -> Self {
        let mut users = Vec::new();
        for u in pcb_user {
            users.push(User::import_pcb(u));
        }
        Self { users }
    }

    pub fn new_user(&mut self, new_user: User) -> usize {
        self.users.push(new_user);
        self.users.len() - 1
    }

    pub fn export_pcboard(&self, users_file: &PathBuf, users_inf_file: &PathBuf) -> Res<()> {
        use std::fs::File;
        use std::io::BufWriter;

        // Convert all users to PCBoard format
        let mut pcb_users = Vec::new();
        let mut pcb_infs = Vec::new();

        for (idx, user) in self.users.iter().enumerate() {
            let mut pcb = user.to_pcboard();
            // Set record number based on position in the database (1-based)
            pcb.user.rec_num = idx as u32;

            // Also set the name in the inf structure
            pcb.inf.name = pcb.user.name.clone();
            pcb.inf.messages_read = user.stats.messages_read as usize;
            pcb.inf.messages_left = user.stats.messages_left as usize;

            pcb_users.push(pcb.user);
            pcb_infs.push(pcb.inf);
        }

        // Write USERS file (main user records)
        {
            let file = File::create(users_file)?;
            let mut writer = BufWriter::new(file);

            for pcb_user in &pcb_users {
                pcb_user.write(&mut writer)?;
            }
        }

        // Write USERS.INF file using the static write_users method
        {
            let file = File::create(users_inf_file)?;
            let mut writer = BufWriter::new(file);

            PcbUserInf::write_users(&pcb_infs, &mut writer)?;
        }
        Ok(())
    }
    /*
    pub fn get_user_home_dir(home_dir: &Path, user_name: &str) -> PathBuf {
        home_dir.join(user_name.to_ascii_lowercase().replace(' ', "_"))
    }*/
}

impl IcyBoardSerializer for UserBase {
    const FILE_TYPE: &'static str = "user base";
}

impl Index<usize> for UserBase {
    type Output = User;
    fn index(&self, i: usize) -> &User {
        &self.users[i]
    }
}
impl IndexMut<usize> for UserBase {
    fn index_mut(&mut self, i: usize) -> &mut User {
        &mut self.users[i]
    }
}

impl Deref for UserBase {
    type Target = Vec<User>;

    fn deref(&self) -> &Self::Target {
        &self.users
    }
}

impl DerefMut for UserBase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.users
    }
}
