use std::{
    collections::HashMap,
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
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    IcyBoardSerializer, PcbUser,
    icb_config::DEFAULT_PCBOARD_DATE_FORMAT,
    is_false, is_null_16, is_null_64, is_null_i64,
    user_inf::{AccountUserInf, BankUserInf, QwkConfigUserInf},
};

#[derive(Clone)]
pub enum Password {
    PlainText(String),
    BCrypt(String),
    Argon2(String),
    /// A secret that may be compared but never shown, like a door password a PPE reads.
    Protected(String),
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Whatever is behind it, a secret has no business in a log line.
        match self {
            Password::PlainText(_) => write!(f, "PlainText(******)"),
            Password::BCrypt(_) => write!(f, "BCrypt(******)"),
            Password::Argon2(_) => write!(f, "Argon2(******)"),
            Password::Protected(_) => write!(f, "Protected(******)"),
        }
    }
}

impl PartialEq for Password {
    fn eq(&self, other: &Self) -> bool {
        match (self.unhashed(), other.unhashed()) {
            (Some(left), Some(right)) => left == right,
            (Some(plain), None) => other.verify(plain),
            (None, Some(plain)) => self.verify(plain),
            // A hash carries its own salt, so only an identical bcrypt record matches.
            (None, None) => match (self, other) {
                (Self::BCrypt(l), Self::BCrypt(r)) => l == r,
                _ => false,
            },
        }
    }
}

impl Default for Password {
    fn default() -> Self {
        Password::PlainText(String::new())
    }
}

impl Password {
    pub fn new_plaintext(str: impl Into<String>) -> Res<Self> {
        Ok(Password::PlainText(str.into().to_lowercase()))
    }

    /// A secret kept as it stands so it can be compared, and which never shows itself.
    pub fn new_protected(str: impl Into<String>) -> Password {
        Password::Protected(str.into().to_lowercase())
    }

    /// The same secret in a form a PPE may compare against but never print.
    #[must_use]
    pub fn protected(&self) -> Password {
        match self {
            Password::PlainText(s) => Password::Protected(s.clone()),
            other => other.clone(),
        }
    }

    /// The secret behind the values that are not hashed.
    fn unhashed(&self) -> Option<&str> {
        match self {
            Password::PlainText(s) | Password::Protected(s) => Some(s),
            Password::Argon2(_) | Password::BCrypt(_) => None,
        }
    }

    fn verify(&self, plain: &str) -> bool {
        match self {
            Password::Argon2(hash) => {
                if let Ok(parsed_hash) = PasswordHash::new(hash) {
                    Argon2::default().verify_password(plain.as_bytes(), &parsed_hash).is_ok()
                } else {
                    false
                }
            }
            Password::BCrypt(hash) => bcrypt::verify(plain, hash).unwrap_or(false),
            Password::PlainText(s) | Password::Protected(s) => s == plain,
        }
    }

    pub fn new_argon2(str: impl Into<String>) -> Password {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);

        let password_hash = argon2.hash_password(str.into().to_lowercase().as_bytes(), &salt).unwrap().to_string();
        Password::Argon2(password_hash)
    }

    pub fn new_bcrypt(str: impl Into<String>) -> Password {
        let hashed = bcrypt::hash(str.into().to_lowercase(), bcrypt::DEFAULT_COST).unwrap();
        Password::BCrypt(hashed)
    }
}

impl std::fmt::Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Password::PlainText(s) => write!(f, "{s}"),
            Password::Argon2(_) | Password::BCrypt(_) | Password::Protected(_) => write!(f, "******"),
        }
    }
}

impl Password {
    pub fn is_empty(&self) -> bool {
        match self {
            Password::PlainText(s) | Password::Argon2(s) | Password::BCrypt(s) | Password::Protected(s) => s.is_empty(),
        }
    }

    pub fn is_valid(&self, pwd: &str) -> bool {
        self == &Password::PlainText(pwd.to_lowercase().clone())
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|p| {
            if let Some(rest) = p.strip_prefix("bcrypt:") {
                return Password::BCrypt(rest.to_string());
            }
            if p.starts_with("$argon2") {
                Password::Argon2(p)
            } else if p.len() >= 2 && p.starts_with('"') && p.ends_with('"') {
                Password::PlainText(p[1..p.len() - 1].to_string())
            } else {
                // Plain text password without quotes (legacy)
                Password::PlainText(p)
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
            // A protected secret is only ever built for a PPE to compare against, it is
            // stored the way a plain one is on the chance it reaches a record at all.
            Password::PlainText(key) | Password::Protected(key) => format!("\"{key}\"").serialize(serializer),
            Password::Argon2(key) => key.serialize(serializer),
            Password::BCrypt(key) => format!("bcrypt:{key}").serialize(serializer),
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
}

/// What `PCBoard`'s `checkpassword` makes of a password the caller wants to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordVerdict {
    Ok,
    /// The one they already have - the record is left alone.
    Unchanged,
    TooShort,
    PartOfName,
    PreviouslyUsed,
}

/// `PCBoard` never let a password be longer than this, so a longer minimum is
/// one nobody could ever satisfy.
const MAX_PASSWORD_LEN: usize = 12;

/// How many old passwords are remembered and refused.
pub const PASSWORD_HISTORY_LEN: usize = 3;

impl PasswordInfo {
    /// The rules `PCBoard`'s `checkpassword` applied: long enough, not a piece of
    /// the caller's own name and not one of the last three.
    ///
    /// The original also refused a password sharing the first `min_len - 2`
    /// characters with an old one. That needs the old passwords in the clear,
    /// which is exactly what hashing them takes away, so it is not checked here.
    pub fn check_new_password(&self, name: &str, candidate: &str, min_len: u8) -> PasswordVerdict {
        let candidate = candidate.trim_end();
        if candidate.len() < (min_len as usize).min(MAX_PASSWORD_LEN) {
            return PasswordVerdict::TooShort;
        }

        let upper_name = name.to_uppercase();
        let upper_candidate = candidate.to_uppercase();
        if upper_name.contains(&upper_candidate) {
            return PasswordVerdict::PartOfName;
        }
        for part in upper_name.split_whitespace() {
            if part.contains(&upper_candidate) || upper_candidate.contains(part) {
                return PasswordVerdict::PartOfName;
            }
        }

        if self.password.is_valid(candidate) {
            return PasswordVerdict::Unchanged;
        }
        if self.prev_pwd.iter().any(|previous| previous.is_valid(candidate)) {
            return PasswordVerdict::PreviouslyUsed;
        }
        PasswordVerdict::Ok
    }

    /// Takes the new password on, pushing the old one onto the history.
    ///
    /// `PCBoard` rotated the history at most once a day so that changing the
    /// password repeatedly could not flush out what came before.
    pub fn accept_new_password(&mut self, password: Password, now: DateTime<Utc>, expire_days: u16) {
        if self.last_change.date_naive() != now.date_naive() {
            self.last_change = now;
            self.times_changed = self.times_changed.wrapping_add(1);
            self.prev_pwd.push(self.password.clone());
            while self.prev_pwd.len() > PASSWORD_HISTORY_LEN {
                self.prev_pwd.remove(0);
            }
            self.expire_date = if expire_days == 0 {
                DateTime::default()
            } else {
                // The date only ever moves further out.
                let next = now + chrono::Duration::days(expire_days as i64);
                if next > self.expire_date { next } else { self.expire_date }
            };
        }
        self.password = password;
    }
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
    #[serde(skip_serializing_if = "is_null_i64")]
    /// Goes negative when an upload earns more credit than the caller has spent today.
    pub today_dnld_bytes: i64,

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
            FSEMode::Ask => 'A',
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
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
            ChatStatus::Available => 'A',
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
pub struct UserContact {
    pub service: String,
    pub account: String,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contacts: Vec<UserContact>,

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

    #[serde(default)]
    pub birth_date: DateTime<Utc>,

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
    pub expiration_date: DateTime<Utc>,

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

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tpa_records: Vec<TpaRecord>,
}

/// Storage a third party application keeps next to a user, one record per
/// keyword, plus one per conference the application wrote for.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TpaRecord {
    pub keyword: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub data: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conferences: Vec<TpaConferenceRecord>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TpaConferenceRecord {
    pub conference: usize,
    pub data: String,
}

#[bitflag(u8)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum ConferenceFlags {
    None = 0x00,
    Registered = 0x01,
    Expired = 0x02,
    Selected = 0x04,
    Sysop = 0x08,
    MailWaiting = 0x10,
    NetStatus = 0x20,
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
    use std::fmt::Write as _;

    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::LastReadStatus;

    pub fn serialize<S>(date: &HashMap<(usize, usize), LastReadStatus>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = String::new();
        for ((conf, area), v) in date {
            // Only these flags get stored in PCBoard - rest is for use at runtime.
            let _ = write!(s, "{},{},{},{},{};", conf, area, v.last_read, v.highest_msg_read, i32::from(v.include_qwk));
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
            if let (Some(c), Some(a), Some(lr), Some(hr), Some(flags)) = (iter.next(), iter.next(), iter.next(), iter.next(), iter.next())
                && let (Ok(c), Ok(a), Ok(lr), Ok(hr), Ok(flags)) = (
                    c.parse::<usize>(),
                    a.parse::<usize>(),
                    lr.parse::<usize>(),
                    hr.parse::<usize>(),
                    flags.parse::<usize>(),
                )
            {
                map.insert(
                    (c, a),
                    LastReadStatus {
                        last_read: lr,
                        highest_msg_read: hr,
                        include_qwk: flags == 1,
                    },
                );
            }
        });
        Ok(map)
    }
}

mod conference_flags_format {
    use std::collections::HashMap;
    use std::fmt::Write as _;

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
            let v = *v & (ConferenceFlags::Selected | ConferenceFlags::Registered | ConferenceFlags::Expired);
            let _ = write!(s, "{}:{};", k, v.bits());
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
            if let (Some(k), Some(v)) = (iter.next(), iter.next())
                && let (Ok(k), Ok(v)) = (k.parse::<usize>(), v.parse::<u8>())
            {
                map.insert(k, ConferenceFlags::from_bits_truncate(v));
            }
        });
        Ok(map)
    }
}

impl User {
    pub fn get_name(&self) -> &String {
        &self.name
    }

    /// A keyword a third party application never wrote reads back as empty.
    pub fn get_tpa(&self, keyword: &str) -> &str {
        match self.find_tpa(keyword) {
            Some(record) => &record.data,
            None => "",
        }
    }

    pub fn set_tpa(&mut self, keyword: &str, data: &str) {
        self.tpa_record_mut(keyword).data = data.to_string();
    }

    pub fn get_conference_tpa(&self, keyword: &str, conference: usize) -> &str {
        let Some(record) = self.find_tpa(keyword) else {
            return "";
        };
        match record.conferences.iter().find(|c| c.conference == conference) {
            Some(entry) => &entry.data,
            None => "",
        }
    }

    pub fn set_conference_tpa(&mut self, keyword: &str, conference: usize, data: &str) {
        let record = self.tpa_record_mut(keyword);
        match record.conferences.iter_mut().find(|c| c.conference == conference) {
            Some(entry) => entry.data = data.to_string(),
            None => record.conferences.push(TpaConferenceRecord {
                conference,
                data: data.to_string(),
            }),
        }
    }

    fn find_tpa(&self, keyword: &str) -> Option<&TpaRecord> {
        self.tpa_records.iter().find(|r| r.keyword.eq_ignore_ascii_case(keyword))
    }

    fn tpa_record_mut(&mut self, keyword: &str) -> &mut TpaRecord {
        if let Some(index) = self.tpa_records.iter().position(|r| r.keyword.eq_ignore_ascii_case(keyword)) {
            return &mut self.tpa_records[index];
        }
        self.tpa_records.push(TpaRecord {
            keyword: keyword.to_string(),
            ..Default::default()
        });
        self.tpa_records.last_mut().unwrap()
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
                personal.birth_date.to_utc_date_time(),
                personal.email.clone(),
                personal.web.clone(),
            )
        } else {
            (String::new(), IcbDate::new(0, 0, 0).to_utc_date_time(), String::new(), String::new())
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
                    .map(|pwd| Password::new_plaintext(pwd).unwrap())
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
            custom_comment1.clone_from(notes.notes.first().unwrap_or(&String::new()));
            custom_comment2.clone_from(notes.notes.get(1).unwrap_or(&String::new()));
            custom_comment3.clone_from(notes.notes.get(2).unwrap_or(&String::new()));
            custom_comment4.clone_from(notes.notes.get(3).unwrap_or(&String::new()));
            custom_comment5.clone_from(notes.notes.get(4).unwrap_or(&String::new()));
        }

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
                    flag |= ConferenceFlags::Selected;
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
            birth_date,
            email,
            web,
            contacts: Vec::new(),

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
                password: Password::new_plaintext(&u.user.password).unwrap(),
                prev_pwd,
                last_change: last_change.to_utc_date_time(),
                times_changed: times_changed as u64,
                expire_date: expire_date.to_utc_date_time(),
            },

            qwk_config,
            account,
            bank,
            tpa_records: Vec::new(),

            bus_data_phone: u.user.bus_data_phone.clone(),
            home_voice_phone: u.user.home_voice_phone.clone(),
            user_comment: u.user.user_comment.clone(),
            sysop_comment: u.user.sysop_comment.clone(),
            security_level: u.user.security_level,
            expiration_date: u.user.exp_date.to_utc_date_time(),
            exp_security_level: u.user.exp_security_level,
            flags: UserFlags {
                expert_mode: u.user.expert_mode,
                is_dirty: u.user.is_dirty,
                msg_clear: u.user.msg_clear,
                has_mail: u.user.has_mail,
                fse_mode: if u.user.use_fsedefault {
                    FSEMode::Yes
                } else if u.user.dont_ask_fse {
                    FSEMode::No
                } else {
                    FSEMode::Ask
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
                today_dnld_bytes: u.user.daily_downloaded_bytes as i64,
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
            super::write_atomic(path, user_txt.as_bytes())?;
        }
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
            security_level: self.security_level,
            exp_date: IcbDate::from_utc(&self.expiration_date),
            exp_security_level: self.exp_security_level,
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
            date_last_dir_read: IcbDate::from_utc(&self.date_last_dir_read),
            is_chat_available: matches!(self.chat_status, ChatStatus::Available),
            last_date_on: IcbDate::from_utc(&self.stats.last_on),
            num_times_on: self.stats.num_times_on as usize,
            num_uploads: self.stats.num_uploads as i32,
            num_downloads: self.stats.num_downloads as i32,
            ul_tot_dnld_bytes: self.stats.total_dnld_bytes,
            ul_tot_upld_bytes: self.stats.total_upld_bytes,
            daily_downloaded_bytes: self.stats.today_dnld_bytes.max(0) as usize,
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
            if flags.contains(ConferenceFlags::Selected) {
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
        if !(self.gender.is_empty() && self.email.is_empty() && self.web.is_empty()) {
            let birth_date = IcbDate::from_utc(&self.birth_date);

            inf.personal = Some(PersonalUserInf {
                gender: self.gender.clone(),
                birth_date,
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
                last_change: IcbDate::from_utc(&self.password.last_change),
                times_changed: self.password.times_changed as usize,
                expire_date: IcbDate::from_utc(&self.password.expire_date),
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
                first_date_on: IcbDate::from_utc(&self.stats.first_date_on),
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
        inf.qwk_config.clone_from(&self.qwk_config);
        inf.account.clone_from(&self.account);
        inf.bank.clone_from(&self.bank);

        PcbUser { user: rec, inf }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserBase {
    users: Vec<User>,
}

impl UserBase {
    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        let lookup = name.trim();
        if lookup.is_empty() {
            return None;
        }

        self.users.iter().position(|u| u.is_valid_loginname(lookup))
    }

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
            pcb.inf.name.clone_from(&pcb.user.name);
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

#[cfg(test)]
mod password_tests {
    use super::*;
    use chrono::TimeZone;

    fn info(current: &str, previous: &[&str]) -> PasswordInfo {
        PasswordInfo {
            password: Password::new_plaintext(current).unwrap(),
            prev_pwd: previous.iter().map(|p| Password::new_plaintext(*p).unwrap()).collect(),
            ..Default::default()
        }
    }

    fn day(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
    }

    #[test]
    fn no_expiry_period_clears_the_date() {
        let mut info = info("one", &[]);
        info.accept_new_password(Password::new_plaintext("two").unwrap(), day(1), 30);
        info.accept_new_password(Password::new_plaintext("three").unwrap(), day(2), 0);

        assert_eq!(info.expire_date, DateTime::<Utc>::default());
    }

    #[test]
    fn a_good_password_is_taken() {
        assert_eq!(info("old", &[]).check_new_password("JOHN DOE", "kaleidoscope", 6), PasswordVerdict::Ok);
    }

    #[test]
    fn a_short_password_is_refused() {
        assert_eq!(info("old", &[]).check_new_password("JOHN DOE", "abc", 6), PasswordVerdict::TooShort);
    }

    /// `PCBoard` capped the minimum at the 12 characters a password could hold,
    /// so a larger setting cannot lock everyone out.
    #[test]
    fn the_minimum_cannot_exceed_the_field() {
        assert_eq!(info("old", &[]).check_new_password("JOHN DOE", "123456789012", 20), PasswordVerdict::Ok);
    }

    #[test]
    fn a_password_out_of_the_callers_name_is_refused() {
        let info = info("old", &[]);
        assert_eq!(info.check_new_password("JOHN DOE", "JOHN", 0), PasswordVerdict::PartOfName);
        // A name that is part of the password is refused just as well.
        assert_eq!(info.check_new_password("JOHN DOE", "DOEDOEDOE", 0), PasswordVerdict::PartOfName);
        assert_eq!(info.check_new_password("JOHN DOE", "john doe", 0), PasswordVerdict::PartOfName);
    }

    #[test]
    fn the_password_they_already_have_counts_as_unchanged() {
        assert_eq!(info("secret", &[]).check_new_password("JOHN DOE", "secret", 0), PasswordVerdict::Unchanged);
    }

    #[test]
    fn an_old_password_is_refused() {
        let info = info("current", &["former", "ancient"]);
        assert_eq!(info.check_new_password("JOHN DOE", "former", 0), PasswordVerdict::PreviouslyUsed);
        assert_eq!(info.check_new_password("JOHN DOE", "ancient", 0), PasswordVerdict::PreviouslyUsed);
    }

    #[test]
    fn accepting_pushes_the_old_password_onto_the_history() {
        let mut info = info("first", &[]);
        info.accept_new_password(Password::new_plaintext("second").unwrap(), day(1), 0);

        assert!(info.password.is_valid("second"));
        assert!(info.prev_pwd[0].is_valid("first"));
        assert_eq!(info.times_changed, 1);
    }

    #[test]
    fn the_history_keeps_the_last_three() {
        let mut info = info("one", &[]);
        for (no, pwd) in ["two", "three", "four", "five"].iter().enumerate() {
            info.accept_new_password(Password::new_plaintext(*pwd).unwrap(), day(no as u32 + 1), 0);
        }

        assert_eq!(info.prev_pwd.len(), PASSWORD_HISTORY_LEN);
        assert!(info.prev_pwd[0].is_valid("two"));
        assert!(info.prev_pwd[2].is_valid("four"));
    }

    /// Changing twice in one day must not push the history along, otherwise the
    /// old passwords could be flushed out and used again right away.
    #[test]
    fn the_history_moves_once_a_day() {
        let mut info = info("one", &[]);
        info.accept_new_password(Password::new_plaintext("two").unwrap(), day(1), 0);
        info.accept_new_password(Password::new_plaintext("three").unwrap(), day(1), 0);

        assert!(info.password.is_valid("three"));
        assert_eq!(info.prev_pwd.len(), 1);
        assert!(info.prev_pwd[0].is_valid("one"));
        assert_eq!(info.times_changed, 1);
    }

    #[test]
    fn the_expiry_date_only_moves_further_out() {
        let mut info = info("one", &[]);
        info.accept_new_password(Password::new_plaintext("two").unwrap(), day(1), 30);
        let far = info.expire_date;

        info.accept_new_password(Password::new_plaintext("three").unwrap(), day(2), 1);
        assert_eq!(info.expire_date, far, "a shorter period pulled the expiry date back");
    }
}
