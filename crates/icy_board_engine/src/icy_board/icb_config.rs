use std::path::PathBuf;

use icy_engine::Color;
use icy_ppe::datetime::IcbTime;
use serde::{Deserialize, Serialize};

use super::{
    is_false, is_null_16, is_null_8, is_null_i32, user_base::Password, IcyBoardSerializer,
};

#[derive(Serialize, Deserialize)]
pub struct SysopSecurityLevels {
    /// Sysop Security Level
    pub sysop: u8,

    pub read_all_comments: u8,
    pub read_all_mail: u8,
    pub copy_move_messages: u8,
    pub use_broadcast_command: u8,
    pub view_private_uploads: u8,
    pub edit_message_headers: u8,
    pub protect_unprotect_messages: u8,
}

#[derive(Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum PasswordStorageMethod {
    ///  Passwords are stored in plain text
    #[default]
    #[serde(rename = "plain")]
    PlainText,
}

impl PasswordStorageMethod {
    pub fn is_default(&self) -> bool {
        *self == PasswordStorageMethod::PlainText
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserPasswordPolicy {
    /// Minimum Password Length (0=disable)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub min_length: u8,
    /// Number of days PWRD is valid before expiring
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub password_expire_days: u16,
    /// Number of days prior to WARN of PWRD expiring
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub password_expire_warn_days: u16,

    #[serde(default)]
    #[serde(skip_serializing_if = "PasswordStorageMethod::is_default")]
    pub password_storage_method: PasswordStorageMethod,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionMode {
    /// run in subscription mode
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub is_enabled: bool,

    /// default days in new subscription period (v14.5)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_i32")]
    pub subscription_length: i32,

    /// default expired security level (v14.5)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub default_expired_level: u8,

    /// days prior to subscription expiration (v14.5)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_i32")]
    pub warning_days: i32,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardInformation {
    ///  name of board
    pub name: String,

    /// Location of the board (used in EmsiISI)
    pub location: String,

    /// Operator of the board (used in EmsiISI)
    pub operator: String,

    /// Notice for the board (used in EmsiISI)
    pub notice: String,

    /// Capabilities of the board (used in EmsiISI)
    pub capabilities: String,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SysopInformation {
    /// Sysop Dislay Name
    pub name: String,
    /// Sysop local password
    #[serde(default)]
    #[serde(skip_serializing_if = "Password::is_empty")]
    pub password: Password,

    ///  Require Local Password to drop PCBoard to DOS (v15.0)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub require_password_to_exit: bool,

    /// Use sysop real name instead of 'SYSOP'
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_real_name: bool,

    ///  start time to allow sysop page
    #[serde(default)]
    #[serde(skip_serializing_if = "IcbTime::is_empty")]
    pub sysop_start: IcbTime,
    ///  stop  time to allow sysop page
    #[serde(default)]
    #[serde(skip_serializing_if = "IcbTime::is_empty")]
    pub sysop_stop: IcbTime,
}

#[derive(PartialEq, Serialize, Deserialize)]
pub struct ColorConfiguration {
    ///  color code for default color
    pub default: IcbColor,
    ///  color for DATE line of message header
    pub msg_hdr_date: IcbColor,
    ///  color for TO   line of message header
    pub msg_hdr_to: IcbColor,
    ///  color for FROM line of message header
    pub msg_hdr_from: IcbColor,
    ///  color for SUBJ line of message header
    pub msg_hdr_subj: IcbColor,
    ///  color for READ line of message header
    pub msg_hdr_read: IcbColor,
    ///  color for CONF line of message header
    pub msg_hdr_conf: IcbColor,
}

#[derive(PartialEq, Clone)]
pub enum IcbColor {
    None,
    Dos(u8),
    IcyEngine(Color),
}

impl From<u8> for IcbColor {
    fn from(color: u8) -> Self {
        IcbColor::Dos(color)
    }
}

impl<'de> Deserialize<'de> for IcbColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|s| {
            if s.starts_with('@') {
                IcbColor::Dos(u8::from_str_radix(&s[2..], 16).unwrap())
            } else {
                IcbColor::IcyEngine(Color::from_hex(&s).unwrap())
            }
        })
    }
}

impl serde::Serialize for IcbColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            IcbColor::None => "".serialize(serializer),
            IcbColor::Dos(u8) => format!("@X{:02X}", u8).serialize(serializer),
            IcbColor::IcyEngine(color) => color.to_hex().serialize(serializer),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ConfigPaths {
    pub help_path: PathBuf,

    /// Shown during login process
    pub security_file_path: PathBuf,

    /// Command display files are shown to the user before a command is executed
    /// file name == command name
    pub command_display_path: PathBuf,

    pub tmp_path: PathBuf,

    pub icbtext: PathBuf,

    pub user_base: PathBuf,

    pub conferences: PathBuf,

    /// name and location of welcome file
    pub welcome: PathBuf,
    /// name and location of newuser file
    pub newuser: PathBuf,
    /// name and location of closed file
    pub closed: PathBuf,
    /// name and location of warning file
    pub warning: PathBuf,
    /// name and location of expired file
    pub expired: PathBuf,

    /// name and location of conference join menu
    pub conf_join_menu: PathBuf,

    /// name and loc of group chat Intro file
    pub group_chat: PathBuf,
    /// name and location of CHAT menu (v15.0)
    pub chat_menu: PathBuf,
    /// name and location of NOANSI Warning
    pub no_ansi: PathBuf,

    /// Bad users file
    pub bad_users: PathBuf,
    /// Bad email file
    pub bad_email: PathBuf,
    /// Bad passwords file
    pub bad_passwords: PathBuf,
    /// VIP users file
    pub vip_users: PathBuf,

    /// name and location of protocol data file
    pub protocol_data_file: PathBuf,

    /// name and location of security level config file
    pub security_level_file: PathBuf, // *

    /// name and location of command file
    pub command_file: PathBuf,

    /// name and location of command file
    pub statistics_file: PathBuf,

    /// name and location of multi language definitions
    pub language_file: PathBuf,

    /// name and location of multi language definitions
    pub group_file: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct IcbConfig {
    pub board: BoardInformation,

    pub sysop: SysopInformation,

    #[serde(rename = "sysop_sec")]
    pub sysop_security_level: SysopSecurityLevels,

    #[serde(rename = "paths")]
    pub paths: ConfigPaths,

    #[serde(rename = "colors")]
    pub color_configuration: ColorConfiguration,

    ///  function key definitions
    pub func_keys: [String; 10],

    #[serde(rename = "subs")]
    pub subscription_info: SubscriptionMode,

    #[serde(rename = "user_pwrd")]
    pub user_password_policy: UserPasswordPolicy,
}

impl IcbConfig {
    pub fn new() -> Self {
        Self {
            board: BoardInformation {
                name: "IcyBoard".to_string(),
                location: String::new(),
                operator: String::new(),
                notice: String::new(),
                capabilities: String::new(),
            },

            sysop: SysopInformation {
                name: "SYSOP".to_string(),
                password: Password::PlainText(String::new()),
                require_password_to_exit: false,
                use_real_name: false,
                sysop_start: IcbTime::default(),
                sysop_stop: IcbTime::default(),
            },
            sysop_security_level: SysopSecurityLevels {
                sysop: 100,
                read_all_comments: 110,
                read_all_mail: 110,
                copy_move_messages: 110,
                use_broadcast_command: 110,
                view_private_uploads: 110,
                edit_message_headers: 110,
                protect_unprotect_messages: 110,
            },
            color_configuration: ColorConfiguration {
                default: IcbColor::Dos(0x07),
                msg_hdr_date: IcbColor::Dos(0x1F),
                msg_hdr_to: IcbColor::Dos(0x3F),
                msg_hdr_from: IcbColor::Dos(0x3F),
                msg_hdr_subj: IcbColor::Dos(0x3F),
                msg_hdr_read: IcbColor::Dos(0x3E),
                msg_hdr_conf: IcbColor::Dos(0x3E),
            },
            func_keys: Default::default(),
            subscription_info: SubscriptionMode {
                is_enabled: false,
                subscription_length: 0,
                default_expired_level: 0,
                warning_days: 0,
            },
            user_password_policy: UserPasswordPolicy {
                min_length: 0,
                password_expire_days: 0,
                password_expire_warn_days: 0,
                password_storage_method: PasswordStorageMethod::PlainText,
            },
            paths: ConfigPaths {
                help_path: PathBuf::from("art/help/"),
                tmp_path: PathBuf::from("tmp/"),
                icbtext: PathBuf::from("data/icbtext.toml"),
                user_base: PathBuf::from("data/user_base.toml"),
                conferences: PathBuf::from("config/conferences.toml"),
                security_file_path: PathBuf::from("art/secmsgs/"),
                command_display_path: PathBuf::from("art/cmd_display/"),

                welcome: PathBuf::new(),
                newuser: PathBuf::new(),
                closed: PathBuf::new(),
                warning: PathBuf::new(),
                expired: PathBuf::new(),
                conf_join_menu: PathBuf::new(),
                group_chat: PathBuf::new(),
                chat_menu: PathBuf::new(),
                no_ansi: PathBuf::new(),

                bad_users: PathBuf::new(),
                bad_email: PathBuf::new(),
                bad_passwords: PathBuf::new(),
                vip_users: PathBuf::new(),

                protocol_data_file: PathBuf::new(),
                security_level_file: PathBuf::new(),
                language_file: PathBuf::new(),
                command_file: PathBuf::new(),
                statistics_file: PathBuf::new(),
                group_file: PathBuf::new(),
            },
        }
    }
}

impl IcyBoardSerializer for IcbConfig {
    const FILE_TYPE: &'static str = "icyboard";
}

impl Default for IcbConfig {
    fn default() -> Self {
        Self::new()
    }
}
