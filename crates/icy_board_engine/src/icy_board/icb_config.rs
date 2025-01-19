use std::path::PathBuf;

use crate::datetime::IcbTime;
use icy_engine::Color;
use serde::{Deserialize, Serialize};

use super::{is_false, is_null_16, is_null_8, is_null_i32, login_server::LoginServer, user_base::Password, IcyBoardSerializer};

#[derive(Serialize, Deserialize)]
pub struct SysopSecurityLevels {
    /// Sysop Security Level
    pub sysop: u8,

    pub read_all_comments: u8,
    pub read_all_mail: u8,
    pub copy_move_messages: u8,
    pub enter_color_codes_in_messages: u8,
    pub use_broadcast_command: u8,
    pub view_private_uploads: u8,
    pub edit_message_headers: u8,
    pub protect_unprotect_messages: u8,
    pub set_pack_out_date_on_messages: u8,
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

    /// Local date format
    pub date_format: String,

    /// Maximum number of active nodes
    pub num_nodes: u16,
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
    pub sysop_start: IcbTime,

    ///  stop  time to allow sysop page
    #[serde(default)]
    #[serde(skip_serializing_if = "IcbTime::is_empty")]
    pub sysop_stop: IcbTime,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

    pub file_name: IcbColor,
    pub file_size: IcbColor,
    pub file_date: IcbColor,
    pub file_description: IcbColor,
    pub file_head: IcbColor,
    pub file_text: IcbColor,
    pub file_description_low: IcbColor,
    pub file_deleted: IcbColor,
    pub file_offline: IcbColor,
    pub file_new_file: IcbColor,
}

impl Default for ColorConfiguration {
    fn default() -> Self {
        Self {
            default: IcbColor::Dos(0x07),
            msg_hdr_date: IcbColor::Dos(0x1F),
            msg_hdr_to: IcbColor::Dos(0x3F),
            msg_hdr_from: IcbColor::Dos(0x3F),
            msg_hdr_subj: IcbColor::Dos(0x3F),
            msg_hdr_read: IcbColor::Dos(0x3E),
            msg_hdr_conf: IcbColor::Dos(0x3E),

            file_name: IcbColor::Dos(0x0E),
            file_size: IcbColor::Dos(0x02),
            file_date: IcbColor::Dos(0x04),
            file_description: IcbColor::Dos(0x0B),
            file_head: IcbColor::Dos(0x06),
            file_text: IcbColor::Dos(0x06),
            file_description_low: IcbColor::Dos(0x03),
            file_deleted: IcbColor::Dos(0x0F),
            file_offline: IcbColor::Dos(0x05),
            file_new_file: IcbColor::Dos(0x8F),
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum IcbColor {
    None,
    Dos(u8), // Color Code with fg + bg color
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

    pub tmp_work_path: PathBuf,

    pub icbtext: PathBuf,

    pub conferences: PathBuf,

    /// name and location of welcome file
    pub welcome: PathBuf,
    /// name and location of newuser file
    pub newuser: PathBuf,
    /// name and location of closed file
    pub closed: PathBuf,
    /// name and location of warning file
    pub expire_warning: PathBuf,
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
    pub trashcan_user: PathBuf,
    /// Bad email file
    pub trashcan_email: PathBuf,
    /// Bad passwords file
    pub trashcan_passwords: PathBuf,
    /// VIP users file
    pub vip_users: PathBuf,

    /// name and location of protocol data file
    pub protocol_data_file: PathBuf,

    /// name and location of security level config file
    pub pwrd_sec_level_file: PathBuf, // *

    /// name and location of command file
    pub command_file: PathBuf,

    /// name and location of command file
    pub statistics_file: PathBuf,

    /// name and location of multi language definitions
    pub language_file: PathBuf,

    /// name and location of multi language definitions
    pub group_file: PathBuf,

    /// home directory for user files
    pub home_dir: PathBuf,

    pub logon_survey: PathBuf,
    pub logon_answer: PathBuf,

    pub logoff_survey: PathBuf,
    pub logoff_answer: PathBuf,

    pub newask_survey: PathBuf,
    pub newask_answer: PathBuf,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NewUserSettings {
    pub sec_level: u8,

    pub new_user_groups: String,
    pub allow_one_name_users: bool,

    /// if true, then the logon survey will be asked in ADDITION to the built in questions
    pub use_newask_and_builtin: bool,

    pub ask_city_or_state: bool,

    pub ask_address: bool,
    pub ask_verification: bool,

    pub ask_bus_data_phone: bool,
    pub ask_voice_phone: bool,
    pub ask_comment: bool,
    pub ask_clr_msg: bool,

    pub ask_xfer_protocol: bool,
    pub ask_date_format: bool,
    pub ask_fse: bool,

    pub ask_alias: bool,
    pub ask_gender: bool,
    pub ask_birthdate: bool,
    pub ask_email: bool,
    pub ask_web_address: bool,
    pub ask_use_short_descr: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct BoardOptions {
    /// Only allow pw change in 'w' command.
    pub disable_full_record_updating: bool,

    /// Run in NewAsk mode.
    pub is_closed_board: bool,

    /// Exclude local calls from all statistics
    pub exclude_local_calls: bool,

    /// DisplayNewsBehavior
    pub display_news_behavior: DisplayNewsBehavior,

    pub display_userinfo_at_login: bool,

    /// max number of lines in a message
    pub max_msg_lines: u16,
    pub scan_all_mail_at_login: bool,
    pub prompt_to_read_mail: bool,

    // keyboard timeout in minutes
    pub keyboard_timeout: u16,

    pub check_files_uploaded: bool,
    pub upload_descr_lines: u8,
    pub display_uploader: bool,

    // disable colors
    pub non_graphics: bool,

    pub give_user_password_to_doors: bool,

    pub call_log: bool,
    pub page_bell: bool,
    pub alarm: bool,

    pub allow_iemsi: bool,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]

pub enum DisplayNewsBehavior {
    /// Display news on login
    #[serde(rename = "Y")]
    OnlyNewer,
    /// Display news on command
    #[serde(rename = "N")]
    OncePerDay,
    /// Display news on command if news is available
    #[serde(rename = "A")]
    Always,
}

#[derive(Serialize, Deserialize)]
pub struct IcbConfig {
    pub board: BoardInformation,
    pub sysop: SysopInformation,

    pub new_user_settings: NewUserSettings,

    pub options: BoardOptions,

    pub login_server: LoginServer,

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

pub const DEFAULT_PCBOARD_DATE_FORMAT: &str = "%m/%d/%y";

impl IcbConfig {
    pub fn new() -> Self {
        Self {
            board: BoardInformation {
                name: "IcyBoard".to_string(),
                location: String::new(),
                operator: String::new(),
                notice: String::new(),
                capabilities: String::new(),
                date_format: DEFAULT_PCBOARD_DATE_FORMAT.to_string(),
                num_nodes: 4,
            },

            sysop: SysopInformation {
                name: "SYSOP".to_string(),
                password: Password::PlainText(String::new()),
                require_password_to_exit: false,
                use_real_name: false,
                sysop_start: IcbTime::default(),
                sysop_stop: IcbTime::default(),
            },
            login_server: LoginServer::default(),
            sysop_security_level: SysopSecurityLevels {
                sysop: 100,
                read_all_comments: 110,
                read_all_mail: 110,
                copy_move_messages: 110,
                enter_color_codes_in_messages: 110,
                use_broadcast_command: 110,
                view_private_uploads: 110,
                edit_message_headers: 110,
                protect_unprotect_messages: 110,
                set_pack_out_date_on_messages: 110,
            },
            color_configuration: ColorConfiguration::default(),
            func_keys: Default::default(),
            subscription_info: SubscriptionMode {
                is_enabled: false,
                subscription_length: 365,
                default_expired_level: 10,
                warning_days: 30,
            },
            user_password_policy: UserPasswordPolicy {
                min_length: 0,
                password_expire_days: 0,
                password_expire_warn_days: 0,
                password_storage_method: PasswordStorageMethod::PlainText,
            },
            paths: ConfigPaths {
                help_path: PathBuf::from("art/help/"),
                tmp_work_path: PathBuf::from("tmp/"),
                icbtext: PathBuf::from("config/icbtext.toml"),
                conferences: PathBuf::from("config/conferences.toml"),
                security_file_path: PathBuf::from("art/secmsgs/"),
                command_display_path: PathBuf::from("art/cmd_display/"),
                home_dir: PathBuf::from("home/"),

                welcome: PathBuf::new(),
                newuser: PathBuf::new(),
                closed: PathBuf::new(),
                expire_warning: PathBuf::new(),
                expired: PathBuf::new(),
                conf_join_menu: PathBuf::new(),
                group_chat: PathBuf::new(),
                chat_menu: PathBuf::new(),
                no_ansi: PathBuf::new(),

                trashcan_user: PathBuf::new(),
                trashcan_email: PathBuf::new(),
                trashcan_passwords: PathBuf::new(),
                vip_users: PathBuf::new(),

                protocol_data_file: PathBuf::new(),
                pwrd_sec_level_file: PathBuf::new(),
                language_file: PathBuf::new(),
                command_file: PathBuf::new(),
                statistics_file: PathBuf::new(),
                group_file: PathBuf::new(),

                logon_survey: PathBuf::new(),
                logon_answer: PathBuf::new(),

                logoff_survey: PathBuf::new(),
                logoff_answer: PathBuf::new(),

                newask_survey: PathBuf::new(),
                newask_answer: PathBuf::new(),
            },
            new_user_settings: NewUserSettings {
                sec_level: 10,
                new_user_groups: "new_users".to_string(),
                allow_one_name_users: false,
                use_newask_and_builtin: false,
                ask_city_or_state: true,
                ask_address: true,
                ask_verification: true,
                ask_bus_data_phone: true,
                ask_voice_phone: true,
                ask_comment: true,
                ask_clr_msg: true,
                ask_date_format: true,
                ask_xfer_protocol: true,
                ask_alias: true,
                ask_gender: true,
                ask_birthdate: true,
                ask_email: true,
                ask_web_address: true,
                ask_use_short_descr: true,
                ask_fse: true,
            },
            options: BoardOptions {
                disable_full_record_updating: false,
                is_closed_board: false,
                display_news_behavior: DisplayNewsBehavior::OnlyNewer,
                display_userinfo_at_login: false,
                exclude_local_calls: true,
                max_msg_lines: 100,
                scan_all_mail_at_login: true,
                prompt_to_read_mail: true,
                check_files_uploaded: true,
                display_uploader: false,
                non_graphics: false,
                keyboard_timeout: 5,
                upload_descr_lines: 20,
                give_user_password_to_doors: false,
                page_bell: true,
                alarm: false,
                call_log: true,
                allow_iemsi: true,
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
