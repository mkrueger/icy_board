use std::{
    ops::{Deref, Index},
    str::FromStr,
};

use icy_ppe::datetime::{IcbDate, IcbTime};
use serde::{Deserialize, Serialize};

use super::{
    icb_config::PasswordStorageMethod,
    is_false, is_null_16, is_null_64,
    user_inf::{AccountUserInf, BankUserInf, QwkConfigUserInf},
    IcyBoardSerializer, PcbUser,
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
    #[serde(skip_serializing_if = "IcbDate::is_empty")]
    pub last_change: IcbDate,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub times_changed: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "IcbDate::is_empty")]
    pub expire_date: IcbDate,

    #[serde(default)]
    #[serde(skip_serializing_if = "PasswordStorageMethod::is_default")]
    pub password_storage_method: PasswordStorageMethod,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct UserStats {
    /// First date on
    #[serde(default)]
    #[serde(skip_serializing_if = "IcbDate::is_empty")]
    pub first_date_on: IcbDate,
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
    pub ul_tot_dnld_bytes: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub ul_tot_upld_bytes: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub daily_downloaded_bytes: u64,
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
    #[serde(skip_serializing_if = "is_false")]
    pub dont_ask_fse: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_fsedefault: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub scroll_msg_body: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub short_header: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub wide_editor: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub delete_flag: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_graphics: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct User {
    name: String,
    id: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub alias: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub verify: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub city: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub street1: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub street2: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub state: String,
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
    pub notes: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bus_data_phone: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub home_voice_phone: String,

    pub birth_date: IcbDate,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub user_comment: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sysop_comment: String,

    pub password: PasswordInfo,

    pub security_level: u8,

    #[serde(default)]
    #[serde(skip_serializing_if = "IcbDate::is_empty")]
    pub exp_date: IcbDate,
    /// Expired security level
    pub exp_security_level: u8,

    pub flags: UserFlags,

    #[serde(default)]
    #[serde(skip_serializing_if = "IcbDate::is_empty")]
    pub last_date_on: IcbDate,
    #[serde(default)]
    #[serde(skip_serializing_if = "IcbTime::is_empty")]
    pub last_time_on: IcbTime,

    /// Protocol (A->Z)
    pub protocol: char,

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
    #[serde(skip_serializing_if = "IcbDate::is_empty")]
    pub date_last_dir_read: IcbDate,

    pub qwk_config: Option<QwkConfigUserInf>,
    pub account: Option<AccountUserInf>,
    pub bank: Option<BankUserInf>,

    pub stats: UserStats,
}

impl User {
    pub fn new(id: u64) -> Self {
        User {
            id,
            ..Default::default()
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

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

    fn import_pcb(id: u64, u: &PcbUser) -> Self {
        let alias = if let Some(alias) = &u.inf.alias {
            alias.alias.clone()
        } else {
            String::new()
        };
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
            (
                String::new(),
                IcbDate::new(0, 0, 0),
                String::new(),
                String::new(),
            )
        };

        let (street1, street2, _city2, state, zip, country) = if let Some(address) = &u.inf.address
        {
            (
                address.street1.clone(),
                address.street2.clone(),
                address.city.clone(),
                address.state.clone(),
                address.zip.clone(),
                address.country.clone(),
            )
        } else {
            (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            )
        };

        let (prev_pwd, last_change, times_changed, expire_date) =
            if let Some(password) = &u.inf.password {
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
            (IcbDate::new(0, 0, 0), 0, 0, 0, 0, 0, 0, 0, 0, 0)
        };

        let notes = if let Some(notes) = &u.inf.notes {
            let mut s = String::new();
            for n in &notes.notes {
                s.push_str(n);
                s.push('\n');
            }
            while s.ends_with('\n') {
                s.pop();
            }
            s
        } else {
            String::new()
        };

        let qwk_config = u.inf.qwk_config.clone();
        let account = u.inf.account.clone();
        let bank = u.inf.bank.clone();

        Self {
            name: u.user.name.clone(),
            id,
            alias,
            verify,
            city: u.user.city.clone(),

            gender,
            birth_date,
            email,
            web,

            street1,
            street2,
            state,
            zip,
            country,

            notes,

            password: PasswordInfo {
                password: Password::from_str(&u.user.password).unwrap(),
                prev_pwd,
                last_change,
                times_changed: times_changed as u64,
                expire_date,
                password_storage_method: PasswordStorageMethod::PlainText,
            },

            qwk_config,
            account,
            bank,

            bus_data_phone: u.user.bus_data_phone.clone(),
            home_voice_phone: u.user.home_voice_phone.clone(),
            user_comment: u.user.user_comment.clone(),
            sysop_comment: u.user.sysop_comment.clone(),
            security_level: u.user.security_level,
            exp_date: u.user.exp_date.clone(),
            exp_security_level: u.user.exp_security_level,
            flags: UserFlags {
                expert_mode: u.user.expert_mode,
                is_dirty: u.user.is_dirty,
                msg_clear: u.user.msg_clear,
                has_mail: u.user.has_mail,
                dont_ask_fse: u.user.dont_ask_fse,
                use_fsedefault: u.user.use_fsedefault,
                scroll_msg_body: u.user.scroll_msg_body,
                short_header: u.user.short_header,
                wide_editor: u.user.wide_editor,
                delete_flag: u.user.delete_flag,
                use_graphics: true,
            },
            last_date_on: u.user.last_date_on.clone(),
            last_time_on: IcbTime::parse(&u.user.last_time_on.clone()),
            protocol: u.user.protocol,
            page_len: u.user.page_len as u16,
            last_conference: u.user.last_conference,
            elapsed_time_on: u.user.elapsed_time_on,
            date_last_dir_read: u.user.date_last_dir_read.clone(),

            stats: UserStats {
                first_date_on,
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
                ul_tot_dnld_bytes: u.user.ul_tot_dnld_bytes,
                ul_tot_upld_bytes: u.user.ul_tot_upld_bytes,
                daily_downloaded_bytes: u.user.daily_downloaded_bytes as u64,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct UserBase {
    last_id: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "user")]
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
            users.push(User::import_pcb(users.len() as u64 + 1, u));
        }
        Self {
            last_id: users.len() as u64,
            users,
        }
    }
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

impl Deref for UserBase {
    type Target = Vec<User>;

    fn deref(&self) -> &Self::Target {
        &self.users
    }
}
