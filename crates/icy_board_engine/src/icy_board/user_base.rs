use std::{
    fs,
    ops::{Deref, Index, IndexMut},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{datetime::IcbDate, Res};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    icb_config::{PasswordStorageMethod, DEFAULT_PCBOARD_DATE_FORMAT},
    is_false, is_null_16, is_null_64,
    user_inf::{AccountUserInf, BankUserInf, QwkConfigUserInf},
    PcbUser,
};

#[derive(Clone, PartialEq)]
pub enum Password {
    PlainText(String),
}

impl std::fmt::Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Password::PlainText(s) => write!(f, "{}", s),
        }
    }
}

impl Default for Password {
    fn default() -> Self {
        Password::PlainText(String::new())
    }
}

impl Password {
    pub fn is_empty(&self) -> bool {
        match self {
            Password::PlainText(s) => s.is_empty(),
        }
    }

    pub fn is_valid(&self, pwd: &str) -> bool {
        match self {
            Password::PlainText(s) => !pwd.is_empty() && s.eq_ignore_ascii_case(pwd),
        }
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Password::PlainText)
    }
}

impl serde::Serialize for Password {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Password::PlainText(key) => key.serialize(serializer),
        }
    }
}

impl FromStr for Password {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Password::PlainText(s.to_string()))
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
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

#[derive(Default, Clone, Serialize, Deserialize)]
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
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum FSEMode {
    #[default]
    Yes,
    No,
    Ask,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct User {
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
    pub exp_date: DateTime<Utc>,
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

        Self {
            name: u.user.name.clone(),
            alias,
            verify_answer: verify,
            city_or_state: u.user.city.clone(),

            date_format: DEFAULT_PCBOARD_DATE_FORMAT.to_string(),
            gender,
            birth_date: birth_date.to_utc_date_time(),
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
            exp_date: u.user.exp_date.to_utc_date_time(),
            exp_security_level: u.user.exp_security_level as u8,
            flags: UserFlags {
                expert_mode: u.user.expert_mode,
                is_dirty: u.user.is_dirty,
                msg_clear: u.user.msg_clear,
                has_mail: u.user.has_mail,
                fse_mode: if u.user.use_fsedefault {
                    FSEMode::Yes
                } else {
                    if u.user.dont_ask_fse {
                        FSEMode::No
                    } else {
                        FSEMode::Ask
                    }
                },
                scroll_msg_body: u.user.scroll_msg_body,
                use_short_filedescr: u.user.short_header,
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
            },
        }
    }

    pub fn is_valid_loginname(&self, name: &str) -> bool {
        let name = name.trim();
        self.name.eq_ignore_ascii_case(name) || self.alias.eq_ignore_ascii_case(name)
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct UserBase {
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
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

    pub fn load_users(&mut self, home_dir: &Path) -> Res<()> {
        let read_dir = fs::read_dir(&home_dir).map_err(|e| {
            log::error!("Error loading home directories {} from {}", e, home_dir.display());
            e
        })?;
        for name in read_dir.flatten() {
            if !name.path().is_dir() {
                continue;
            }
            let user_file = name.path().join("user.toml");
            if !user_file.exists() {
                log::error!("Can't read user file for {} ", name.path().display());
                continue;
            }
            let user_txt = fs::read_to_string(user_file)?;
            let user: User = toml::from_str(&user_txt)?;
            self.users.push(user);
        }
        self.users.sort_by(|a, b| a.stats.first_date_on.cmp(&b.stats.first_date_on));

        Ok(())
    }

    pub fn save_users(&self, home_dir: &Path) -> Res<()> {
        std::fs::create_dir_all(&home_dir)?;
        for user in &self.users {
            let home_dir = Self::get_user_home_dir(home_dir, user.get_name());
            std::fs::create_dir_all(&home_dir)?;
            let user_txt = toml::to_string(user)?;
            fs::write(home_dir.join("user.toml"), user_txt)?;
        }
        Ok(())
    }

    pub fn get_user_home_dir(home_dir: &Path, user_name: &str) -> PathBuf {
        home_dir.join(user_name.to_ascii_lowercase().replace(' ', "_"))
    }
}
/*
impl IcyBoardSerializer for UserBase {
    const FILE_TYPE: &'static str = "user base";

    fn load<P: AsRef<std::path::Path>>(path: &P) -> crate::Res<Self> {
        let mut res = load_internal::<Self, P>(path)?;
        Ok(res)
    }

    fn save<P: AsRef<std::path::Path>>(&self, path: &P) -> crate::Res<()> {
        std::fs::create_dir_all(&self.home_dir)?;
        for user in &self.users {
            let home_dir = self.home_dir.join(user.get_name().to_ascii_lowercase().replace(' ', "_"));
            std::fs::create_dir_all(&home_dir)?;
            let user_txt = toml::to_string(user)?;
            fs::write(home_dir.join("user.toml"), user_txt)?;
        }
        save_internal::<Self, P>(self, path)?;
        Ok(())
    }
}
*/
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
