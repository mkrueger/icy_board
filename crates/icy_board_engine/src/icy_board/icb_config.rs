use std::path::PathBuf;

use crate::datetime::{IcbDoW, IcbTime};
use icy_engine::Color;
use serde::{Deserialize, Serialize};

use super::{is_false, is_null_16, is_null_32, is_null_8, login_server::LoginServer, user_base::Password, IcyBoardSerializer};

#[derive(Serialize, Deserialize)]
pub struct SysopCommandLevels {
    /// Sysop Security Level
    pub sysop: u8,

    pub read_all_comments: u8,
    pub read_all_mail: u8,
    pub copy_move_messages: u8,
    pub enter_color_codes_in_messages: u8,

    pub edit_any_message: u8,
    pub not_update_msg_read: u8,
    pub use_broadcast_command: u8,
    pub view_private_uploads: u8,
    pub enter_generic_messages: u8,

    pub edit_message_headers: u8,
    pub protect_unprotect_messages: u8,
    pub overwrite_files_on_uploads: u8,
    pub set_pack_out_date_on_messages: u8,
    pub see_all_return_receipts: u8,

    /// Sysop commands
    pub sec_1_view_caller_log: u8,
    pub sec_2_view_usr_list: u8,
    pub sec_3_pack_renumber_msg: u8,
    pub sec_4_recover_deleted_msg: u8,
    pub sec_5_list_message_hdr: u8,
    pub sec_6_view_any_file: u8,
    pub sec_7_user_maint: u8,
    pub sec_8_pack_usr_file: u8,
    pub sec_9_exit_to_dos: u8,
    pub sec_10_shelled_dos_func: u8,
    pub sec_11_view_other_nodes: u8,
    pub sec_12_logoff_alt_node: u8,
    pub sec_13_view_alt_node_callers: u8,
    pub sec_14_drop_alt_node_to_dos: u8,
}

#[derive(Serialize, Deserialize)]
pub struct UserCommandLevels {
    pub cmd_a: u8,
    pub cmd_b: u8,
    pub cmd_c: u8,
    pub cmd_d: u8,
    pub cmd_e: u8,
    pub cmd_f: u8,
    // pub cmd_g: u8, No longer used by PCBoard
    pub cmd_h: u8,
    pub cmd_i: u8,
    pub cmd_j: u8,
    pub cmd_k: u8,
    pub cmd_l: u8,
    pub cmd_m: u8,
    pub cmd_n: u8,
    pub cmd_o: u8,
    pub cmd_p: u8,

    pub cmd_q: u8,
    pub cmd_r: u8,
    pub cmd_s: u8,
    pub cmd_t: u8,
    pub cmd_u: u8,
    pub cmd_v: u8,
    pub cmd_w: u8,
    pub cmd_x: u8,
    pub cmd_y: u8,
    pub cmd_z: u8,
    pub cmd_chat: u8,
    pub cmd_open_door: u8,
    pub cmd_test_file: u8,
    pub cmd_show_user_list: u8,
    pub cmd_who: u8,

    pub batch_file_transfer: u8,
    pub edit_own_messages: u8,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionMode {
    /// run in subscription mode
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub is_enabled: bool,

    /// default days in new subscription period (v14.5)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_32")]
    pub subscription_length: u32,

    /// default expired security level (v14.5)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub default_expired_level: u8,

    /// days prior to subscription expiration (v14.5)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_32")]
    pub warning_days: u32,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardInformation {
    ///  name of board
    pub name: String,

    /// Allow IEMSI logins
    #[serde(default)]
    pub allow_iemsi: bool,

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

    #[serde(default)]
    pub who_include_city: bool,

    #[serde(default)]
    pub who_show_alias: bool,
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

    pub external_editor: String,

    pub config_color_theme: String,
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

impl IcbColor {
    pub fn dos_black() -> Self {
        IcbColor::Dos(0x00)
    }
    pub fn dos_blue() -> Self {
        IcbColor::Dos(0x00)
    }

    pub fn dos_light_blue() -> Self {
        IcbColor::Dos(0x09)
    }

    pub fn dos_light_green() -> Self {
        IcbColor::Dos(0x0A)
    }

    pub fn dos_cyan() -> Self {
        IcbColor::Dos(0x0B)
    }

    pub fn dos_light_red() -> Self {
        IcbColor::Dos(0x0C)
    }

    pub fn dos_magenta() -> Self {
        IcbColor::Dos(0x0D)
    }

    pub fn dos_yellow() -> Self {
        IcbColor::Dos(0x0E)
    }

    pub fn dos_white() -> Self {
        IcbColor::Dos(0x0F)
    }
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
    pub chat_intro_file: PathBuf,
    /// name and location of CHAT menu (v15.0)
    pub chat_menu: PathBuf,
    /// name and location of CHAT ACTIONS menu (v15.4)
    pub chat_actions_menu: PathBuf,

    /// name and location of NOANSI Warning
    pub no_ansi: PathBuf,

    /// name and location of trashcan files
    pub trashcan_upload_files: PathBuf,

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

    pub caller_log: PathBuf,

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

    pub ask_business_phone: bool,
    pub ask_home_phone: bool,
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
pub struct MessageOptions {
    /// max number of lines in a message
    pub max_msg_lines: u16,
    pub scan_all_mail_at_login: bool,

    pub disable_message_scan_prompt: bool,
    pub allow_esc_codes: bool,
    pub allow_carbon_copy: bool,
    pub validate_to_name: bool,
    pub default_quick_personal_scan: bool,
    pub default_scan_all_selected_confs_at_login: bool,
    pub prompt_to_read_mail: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileTransferOptions {
    pub disallow_batch_uploads: bool,
    pub promote_to_batch_transfers: bool,

    pub upload_credit_time: u32,
    pub upload_credit_bytes: u32,

    pub verify_files_uploaded: bool,
    pub upload_descr_lines: u8,
    pub display_uploader: bool,

    pub disable_drive_size_check: bool,
    pub stop_uploads_free_space: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SystemControlOptions {
    pub disable_ns_logon: bool,

    /// Only allow pw change in 'w' command.
    pub disable_full_record_updating: bool,

    pub allow_alias_change: bool,

    pub is_multi_lingual: bool,

    /// Run in NewAsk mode.
    pub is_closed_board: bool,

    /// Switch between daily and session limits
    pub enforce_daily_time_limit: bool,

    pub allow_password_failure_comment: bool,

    /// G command will ask for logoff (bye will skip that)
    pub guard_logoff: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConfigSwitches {
    #[serde(default)]
    pub default_graphics_at_login: bool,

    // disable colors
    #[serde(default)]
    pub non_graphics: bool,

    /// Exclude local calls from all statistics
    #[serde(default)]
    pub exclude_local_calls_stats: bool,

    /// DisplayNewsBehavior
    pub display_news_behavior: DisplayNewsBehavior,

    #[serde(default)]
    pub display_userinfo_at_login: bool,

    #[serde(default)]
    pub force_intro_on_join: bool,

    #[serde(default)]
    pub scan_new_blt: bool,

    #[serde(default)]
    pub capture_grp_chat_session: bool,

    #[serde(default)]
    pub allow_handle_in_grpchat: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LimitOptions {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub keyboard_timeout: u16,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub max_number_upload_descr_lines: u16,

    /// Minimum Password Length (0=disable)
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub min_pwd_length: u8,

    /// Number of days PWRD is valid before expiring
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub password_expire_days: u16,

    /// Number of days prior to WARN of PWRD expiring
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub password_expire_warn_days: u16,

    ///  start time to allow sysop page
    #[serde(default)]
    #[serde(skip_serializing_if = "IcbTime::is_empty")]
    pub sysop_start: IcbTime,

    ///  stop  time to allow sysop page
    #[serde(default)]
    #[serde(skip_serializing_if = "IcbTime::is_empty")]
    pub sysop_stop: IcbTime,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BoardOptions {
    #[serde(default)]
    pub give_user_password_to_doors: bool,

    #[serde(default)]
    pub call_log: bool,

    #[serde(default)]
    pub page_bell: bool,

    #[serde(default)]
    pub alarm: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EventOptions {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub event_dat_path: PathBuf,

    #[serde(default)]
    pub suspend_minutes: u16,

    #[serde(default)]
    pub disallow_uploads: bool,

    #[serde(default)]
    pub minutes_uploads_disallowed: u16,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AccountingOptions {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub use_money: bool,

    #[serde(default)]
    pub concurrent_tracking: bool,

    #[serde(default)]
    pub ignore_empty_sec_level: bool,

    #[serde(default)]
    pub peak_usage_start: IcbTime,

    #[serde(default)]
    pub peak_usage_end: IcbTime,

    #[serde(default)]
    pub peak_days_of_week: IcbDoW,

    #[serde(default)]
    pub peak_holiday_list_file: PathBuf,

    #[serde(default)]
    pub cfg_file: PathBuf,

    #[serde(default)]
    pub tracking_file: PathBuf,

    #[serde(default)]
    pub info_file: PathBuf,

    #[serde(default)]
    pub warning_file: PathBuf,

    #[serde(default)]
    pub logoff_file: PathBuf,
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
impl DisplayNewsBehavior {
    pub fn to_pcb_char(&self) -> char {
        match self {
            DisplayNewsBehavior::OnlyNewer => 'Y',
            DisplayNewsBehavior::OncePerDay => 'N',
            DisplayNewsBehavior::Always => 'A',
        }
    }

    pub fn from_pcb_char(c: char) -> Self {
        match c {
            'Y' => DisplayNewsBehavior::OnlyNewer,
            'N' => DisplayNewsBehavior::OncePerDay,
            'A' => DisplayNewsBehavior::Always,
            _ => {
                log::warn!("Invalid DisplayNewsBehavior char: {}", c);
                DisplayNewsBehavior::OnlyNewer
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct IcbConfig {
    pub board: BoardInformation,
    pub sysop: SysopInformation,

    pub new_user_settings: NewUserSettings,

    pub message: MessageOptions,
    pub file_transfer: FileTransferOptions,
    pub system_control: SystemControlOptions,
    pub switches: ConfigSwitches,
    pub limits: LimitOptions,
    pub options: BoardOptions,
    pub event: EventOptions,
    pub accounting: AccountingOptions,

    pub login_server: LoginServer,

    #[serde(rename = "sysop_sec")]
    pub sysop_command_level: SysopCommandLevels,

    #[serde(rename = "user_sec")]
    pub user_command_level: UserCommandLevels,

    #[serde(rename = "paths")]
    pub paths: ConfigPaths,

    #[serde(rename = "colors")]
    pub color_configuration: ColorConfiguration,

    ///  function key definitions
    pub func_keys: [String; 10],

    #[serde(rename = "subs")]
    pub subscription_info: SubscriptionMode,
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
                allow_iemsi: true,
                who_include_city: true,
                who_show_alias: true,
            },

            sysop: SysopInformation {
                name: "SYSOP".to_string(),
                password: Password::PlainText(String::new()),
                require_password_to_exit: false,
                use_real_name: false,
                external_editor: "nano".to_string(),
                config_color_theme: "DEFAULT".to_string(),
            },
            login_server: LoginServer::default(),
            sysop_command_level: SysopCommandLevels {
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
                see_all_return_receipts: 110,
                overwrite_files_on_uploads: 110,
                not_update_msg_read: 110,
                enter_generic_messages: 110,
                edit_any_message: 110,

                sec_1_view_caller_log: 110,
                sec_2_view_usr_list: 110,
                sec_3_pack_renumber_msg: 110,
                sec_4_recover_deleted_msg: 110,
                sec_5_list_message_hdr: 110,
                sec_6_view_any_file: 110,
                sec_7_user_maint: 110,
                sec_8_pack_usr_file: 110,
                sec_9_exit_to_dos: 110,
                sec_10_shelled_dos_func: 110,
                sec_11_view_other_nodes: 110,
                sec_12_logoff_alt_node: 110,
                sec_13_view_alt_node_callers: 110,
                sec_14_drop_alt_node_to_dos: 110,
            },
            user_command_level: UserCommandLevels {
                cmd_a: 10,
                cmd_b: 10,
                cmd_c: 10,
                cmd_d: 10,
                cmd_e: 10,
                cmd_f: 10,
                cmd_h: 10,
                cmd_i: 10,
                cmd_j: 10,
                cmd_k: 10,
                cmd_l: 10,
                cmd_m: 10,
                cmd_n: 10,
                cmd_o: 10,
                cmd_p: 10,
                cmd_q: 10,
                cmd_r: 10,
                cmd_s: 10,
                cmd_t: 10,
                cmd_u: 10,
                cmd_v: 10,
                cmd_w: 10,
                cmd_x: 10,
                cmd_y: 10,
                cmd_z: 10,
                cmd_chat: 10,
                cmd_open_door: 10,
                cmd_test_file: 10,
                cmd_show_user_list: 10,
                cmd_who: 10,
                batch_file_transfer: 10,
                edit_own_messages: 10,
            },
            color_configuration: ColorConfiguration::default(),
            func_keys: Default::default(),
            subscription_info: SubscriptionMode {
                is_enabled: false,
                subscription_length: 365,
                default_expired_level: 10,
                warning_days: 30,
            },
            paths: ConfigPaths {
                help_path: PathBuf::from("art/help/"),
                tmp_work_path: PathBuf::from("tmp/"),
                icbtext: PathBuf::from("config/icbtext.toml"),
                conferences: PathBuf::from("config/conferences.toml"),
                security_file_path: PathBuf::from("art/secmsgs/"),
                command_display_path: PathBuf::from("art/cmd_display/"),
                home_dir: PathBuf::from("home/"),
                caller_log: PathBuf::from("caller.log"),

                welcome: PathBuf::new(),
                newuser: PathBuf::new(),
                closed: PathBuf::new(),
                expire_warning: PathBuf::new(),
                expired: PathBuf::new(),
                conf_join_menu: PathBuf::new(),
                chat_intro_file: PathBuf::new(),
                chat_menu: PathBuf::new(),
                chat_actions_menu: PathBuf::new(),
                no_ansi: PathBuf::new(),

                trashcan_upload_files: PathBuf::new(),
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
                ask_address: false,
                ask_verification: false,
                ask_business_phone: true,
                ask_home_phone: true,
                ask_comment: true,
                ask_clr_msg: true,
                ask_date_format: false,
                ask_xfer_protocol: true,
                ask_alias: false,
                ask_gender: false,
                ask_birthdate: false,
                ask_email: false,
                ask_web_address: false,
                ask_use_short_descr: false,
                ask_fse: false,
            },
            message: MessageOptions {
                max_msg_lines: 100,
                scan_all_mail_at_login: true,
                prompt_to_read_mail: true,
                disable_message_scan_prompt: true,
                allow_esc_codes: false,
                allow_carbon_copy: true,
                validate_to_name: true,
                default_quick_personal_scan: true,
                default_scan_all_selected_confs_at_login: true,
            },
            file_transfer: FileTransferOptions {
                display_uploader: false,
                upload_descr_lines: 20,
                disallow_batch_uploads: false,
                promote_to_batch_transfers: true,
                upload_credit_time: 100,
                upload_credit_bytes: 0,
                verify_files_uploaded: true,
                disable_drive_size_check: false,
                stop_uploads_free_space: 1024,
            },
            system_control: SystemControlOptions {
                disable_ns_logon: false,
                disable_full_record_updating: false,
                is_closed_board: false,
                guard_logoff: false,
                enforce_daily_time_limit: false,
                allow_alias_change: false,
                is_multi_lingual: false,
                allow_password_failure_comment: false,
            },
            switches: ConfigSwitches {
                display_news_behavior: DisplayNewsBehavior::OnlyNewer,
                display_userinfo_at_login: false,
                exclude_local_calls_stats: true,
                non_graphics: false,
                default_graphics_at_login: true,
                force_intro_on_join: false,
                scan_new_blt: true,
                capture_grp_chat_session: false,
                allow_handle_in_grpchat: false,
            },
            limits: LimitOptions {
                keyboard_timeout: 5,
                min_pwd_length: 0,
                password_expire_days: 0,
                password_expire_warn_days: 0,

                sysop_start: IcbTime::default(),
                sysop_stop: IcbTime::default(),
                max_number_upload_descr_lines: 20,
            },
            options: BoardOptions {
                give_user_password_to_doors: false,
                page_bell: true,
                alarm: false,
                call_log: true,
            },
            event: EventOptions {
                enabled: false,
                event_dat_path: PathBuf::new(),
                suspend_minutes: 0,
                disallow_uploads: false,
                minutes_uploads_disallowed: 0,
            },
            accounting: AccountingOptions {
                enabled: false,
                use_money: false,
                concurrent_tracking: false,
                ignore_empty_sec_level: false,
                peak_usage_start: IcbTime::default(),
                peak_usage_end: IcbTime::default(),
                peak_days_of_week: IcbDoW::default(),
                peak_holiday_list_file: PathBuf::new(),
                cfg_file: PathBuf::new(),
                tracking_file: PathBuf::new(),
                info_file: PathBuf::new(),
                warning_file: PathBuf::new(),
                logoff_file: PathBuf::new(),
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
