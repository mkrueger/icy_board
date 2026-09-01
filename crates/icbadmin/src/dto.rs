use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct OverviewDto {
    pub board_file: String,
    pub root_path: String,
    pub tool_version: String,
    pub config_loaded: bool,
    pub load_error: Option<String>,
    pub board_name: Option<String>,
    pub sysop_name: Option<String>,
    pub num_nodes: Option<u16>,
    pub counts: Option<CountsDto>,
    pub statistics: Option<StatisticsDto>,
    pub paths: Vec<PathCheckDto>,
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct CountsDto {
    pub conferences: usize,
    pub users: usize,
    pub security_levels: usize,
    pub commands: usize,
    pub languages: usize,
    pub protocols: usize,
}

#[derive(Serialize)]
pub struct StatisticsDto {
    pub today_calls: u64,
    pub today_messages: u64,
    pub today_uploads: u64,
    pub today_downloads: u64,
    pub total_calls: u64,
    pub total_messages: u64,
    pub total_uploads: u64,
    pub total_downloads: u64,
}

#[derive(Serialize)]
pub struct PathCheckDto {
    pub label: String,
    pub path: String,
    pub exists: bool,
    pub expected: PathKind,
}

#[derive(Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PathKind {
    File,
    Directory,
    Unset,
}

#[derive(Serialize, Debug)]
pub struct FieldChangeDto {
    pub field: String,
    pub old: String,
    pub new: String,
}

#[derive(Serialize, Debug)]
pub struct DiffDto {
    pub changes: Vec<FieldChangeDto>,
}

#[derive(Serialize, Debug)]
pub struct ApplyResultDto {
    pub changed_fields: Vec<String>,
    pub backup: Option<String>,
    pub fingerprint: String,
}

pub const DATE_FORMATS: &[(&str, &str)] = &[
    ("%m/%d/%y", "MM/DD/YY"),
    ("%d/%m/%y", "DD/MM/YY"),
    ("%y/%m/%d", "YY/MM/DD"),
    ("%m.%d.%y", "MM.DD.YY"),
    ("%d.%m.%y", "DD.MM.YY"),
    ("%y.%m.%d", "YY.MM.DD"),
    ("%m-%d-%y", "MM-DD-YY"),
    ("%d-%m-%y", "DD-MM-YY"),
    ("%y-%m-%d", "YY-MM-DD"),
];

pub const PASSWORD_STORAGE_METHODS: &[(&str, &str)] = &[("bcrypt", "bcrypt (recommended)"), ("argon2", "Argon2"), ("plain", "Plain text (legacy only)")];

pub const DISPLAY_NEWS_BEHAVIORS: &[(&str, &str)] = &[("Y", "Only newer news"), ("N", "Once per day"), ("A", "Always"), ("X", "Never")];

/// Board identity + sysop display options. The sysop password is never part of this DTO.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct GeneralSettingsDto {
    pub board_name: String,
    pub location: String,
    pub operator: String,
    pub notice: String,
    pub capabilities: String,
    pub date_format: String,
    pub num_nodes: u16,
    #[serde(default)]
    pub allow_iemsi: bool,
    #[serde(default)]
    pub who_include_city: bool,
    #[serde(default)]
    pub who_show_alias: bool,
    pub sysop_name: String,
    #[serde(default)]
    pub sysop_use_real_name: bool,
    #[serde(default)]
    pub sysop_require_password_to_exit: bool,
    #[serde(default)]
    pub sysop_external_editor: String,
    #[serde(default)]
    pub sysop_config_color_theme: String,
    #[serde(default)]
    pub web_admin_enabled: bool,
    #[serde(default)]
    pub web_admin_address: String,
    #[serde(default)]
    pub web_admin_port: u16,
    #[serde(default)]
    pub web_admin_allow_remote: bool,
}

#[derive(Serialize, Debug)]
pub struct GeneralSettingsResponse {
    #[serde(flatten)]
    pub settings: GeneralSettingsDto,
    pub sysop_password_set: bool,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct GeneralSettingsPatch {
    #[serde(flatten)]
    pub settings: GeneralSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct MessageSettingsDto {
    pub max_msg_lines: u16,
    #[serde(default)]
    pub scan_all_mail_at_login: bool,
    #[serde(default)]
    pub disable_message_scan_prompt: bool,
    #[serde(default)]
    pub allow_esc_codes: bool,
    #[serde(default)]
    pub allow_carbon_copy: bool,
    #[serde(default)]
    pub validate_to_name: bool,
    #[serde(default)]
    pub default_quick_personal_scan: bool,
    #[serde(default)]
    pub default_scan_all_selected_confs_at_login: bool,
    #[serde(default)]
    pub prompt_to_read_mail: bool,
    #[serde(default)]
    pub force_comments_to_main: bool,
    #[serde(default)]
    pub update_last_read_pointer: bool,
}

#[derive(Serialize, Debug)]
pub struct MessageSettingsResponse {
    #[serde(flatten)]
    pub settings: MessageSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct MessageSettingsPatch {
    #[serde(flatten)]
    pub settings: MessageSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct FileTransferSettingsDto {
    #[serde(default)]
    pub disallow_batch_uploads: bool,
    #[serde(default)]
    pub promote_to_batch_transfers: bool,
    pub upload_credit_time: u32,
    pub upload_credit_bytes: u32,
    #[serde(default)]
    pub verify_files_uploaded: bool,
    pub upload_descr_lines: u8,
    #[serde(default)]
    pub display_uploader: bool,
    #[serde(default)]
    pub disable_drive_size_check: bool,
    pub stop_uploads_free_space: u32,
}

#[derive(Serialize, Debug)]
pub struct FileTransferSettingsResponse {
    #[serde(flatten)]
    pub settings: FileTransferSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct FileTransferSettingsPatch {
    #[serde(flatten)]
    pub settings: FileTransferSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SystemControlSettingsDto {
    #[serde(default)]
    pub disable_ns_logon: bool,
    #[serde(default)]
    pub disable_full_record_updating: bool,
    #[serde(default)]
    pub allow_alias_change: bool,
    #[serde(default)]
    pub is_multi_lingual: bool,
    #[serde(default)]
    pub is_closed_board: bool,
    #[serde(default)]
    pub enforce_daily_time_limit: bool,
    #[serde(default)]
    pub allow_password_failure_comment: bool,
    #[serde(default)]
    pub guard_logoff: bool,
    /// One of: bcrypt, argon2, plain
    pub password_storage_method: String,
    #[serde(default)]
    pub confirm_caller_name: bool,
    #[serde(default)]
    pub reread_sec_level_on_join: bool,
}

#[derive(Serialize, Debug)]
pub struct SystemControlSettingsResponse {
    #[serde(flatten)]
    pub settings: SystemControlSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct SystemControlSettingsPatch {
    #[serde(flatten)]
    pub settings: SystemControlSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SwitchesSettingsDto {
    #[serde(default)]
    pub default_graphics_at_login: bool,
    #[serde(default)]
    pub non_graphics: bool,
    #[serde(default)]
    pub exclude_local_calls_stats: bool,
    /// One of: Y, N, A, X
    pub display_news_behavior: String,
    #[serde(default)]
    pub disable_registration_edits: bool,
    #[serde(default)]
    pub disable_high_ascii_filter: bool,
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
    #[serde(default)]
    pub give_user_password_to_doors: bool,
    #[serde(default)]
    pub call_log: bool,
    #[serde(default)]
    pub page_bell: bool,
    #[serde(default)]
    pub page_notification_command: String,
    #[serde(default)]
    pub alarm: bool,
    #[serde(default)]
    pub log_caller_number: bool,
    #[serde(default)]
    pub log_connect_string: bool,
    #[serde(default)]
    pub log_security_level: bool,
}

#[derive(Serialize, Debug)]
pub struct SwitchesSettingsResponse {
    #[serde(flatten)]
    pub settings: SwitchesSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct SwitchesSettingsPatch {
    #[serde(flatten)]
    pub settings: SwitchesSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct LimitsSettingsDto {
    pub keyboard_timeout: u16,
    pub max_number_upload_descr_lines: u16,
    pub min_pwd_length: u8,
    pub password_expire_days: u16,
    pub password_expire_warn_days: u16,
    /// HH:MM:SS or empty
    pub sysop_start: String,
    /// HH:MM:SS or empty
    pub sysop_stop: String,
}

#[derive(Serialize, Debug)]
pub struct LimitsSettingsResponse {
    #[serde(flatten)]
    pub settings: LimitsSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct LimitsSettingsPatch {
    #[serde(flatten)]
    pub settings: LimitsSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct NewUserSettingsDto {
    pub sec_level: u8,
    pub new_user_groups: String,
    #[serde(default)]
    pub allow_one_name_users: bool,
    #[serde(default)]
    pub use_newask_and_builtin: bool,
    #[serde(default)]
    pub ask_city_or_state: bool,
    #[serde(default)]
    pub ask_address: bool,
    #[serde(default)]
    pub ask_verification: bool,
    #[serde(default)]
    pub ask_business_phone: bool,
    #[serde(default)]
    pub ask_home_phone: bool,
    #[serde(default)]
    pub ask_comment: bool,
    #[serde(default)]
    pub ask_clr_msg: bool,
    #[serde(default)]
    pub ask_xfer_protocol: bool,
    #[serde(default)]
    pub ask_date_format: bool,
    #[serde(default)]
    pub ask_fse: bool,
    #[serde(default)]
    pub ask_alias: bool,
    #[serde(default)]
    pub ask_gender: bool,
    #[serde(default)]
    pub ask_birthdate: bool,
    #[serde(default)]
    pub ask_email: bool,
    #[serde(default)]
    pub ask_web_address: bool,
    #[serde(default)]
    pub ask_use_short_descr: bool,
    #[serde(default)]
    pub auto_register_conferences: bool,
}

#[derive(Serialize, Debug)]
pub struct NewUserSettingsResponse {
    #[serde(flatten)]
    pub settings: NewUserSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct NewUserSettingsPatch {
    #[serde(flatten)]
    pub settings: NewUserSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct EventSettingsDto {
    #[serde(default)]
    pub enabled: bool,
    pub event_file: String,
    pub suspend_minutes: u16,
    #[serde(default)]
    pub disallow_uploads: bool,
    pub minutes_uploads_disallowed: u16,
}

#[derive(Serialize, Debug)]
pub struct EventSettingsResponse {
    #[serde(flatten)]
    pub settings: EventSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct EventSettingsPatch {
    #[serde(flatten)]
    pub settings: EventSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SubscriptionSettingsDto {
    #[serde(default)]
    pub is_enabled: bool,
    pub subscription_length: u32,
    pub default_expired_level: u8,
    pub warning_days: u32,
}

#[derive(Serialize, Debug)]
pub struct SubscriptionSettingsResponse {
    #[serde(flatten)]
    pub settings: SubscriptionSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct SubscriptionSettingsPatch {
    #[serde(flatten)]
    pub settings: SubscriptionSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ListenerDto {
    #[serde(default)]
    pub is_enabled: bool,
    pub port: u16,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub display_file: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SecureWebsocketDto {
    #[serde(default)]
    pub is_enabled: bool,
    pub port: u16,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub display_file: String,
    #[serde(default)]
    pub cert_pem: String,
    #[serde(default)]
    pub key_pem: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ConnectionSettingsDto {
    pub telnet: ListenerDto,
    pub ssh: ListenerDto,
    pub secure_websocket: SecureWebsocketDto,
}

#[derive(Serialize, Debug)]
pub struct ConnectionSettingsResponse {
    #[serde(flatten)]
    pub settings: ConnectionSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct ConnectionSettingsPatch {
    #[serde(flatten)]
    pub settings: ConnectionSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PathsSettingsDto {
    pub help_path: String,
    pub security_file_path: String,
    pub email_msgbase: String,
    pub command_display_path: String,
    pub tmp_work_path: String,
    pub icbtext: String,
    pub conferences: String,
    pub welcome: String,
    pub newuser: String,
    pub closed: String,
    pub expire_warning: String,
    pub expired: String,
    pub conf_join_menu: String,
    pub chat_intro_file: String,
    pub chat_menu: String,
    pub chat_actions_menu: String,
    pub no_ansi: String,
    pub trashcan_upload_files: String,
    pub trashcan_user: String,
    pub trashcan_email: String,
    pub trashcan_passwords: String,
    pub vip_users: String,
    pub protocol_data_file: String,
    pub pwrd_sec_level_file: String,
    pub command_file: String,
    pub statistics_file: String,
    pub language_file: String,
    pub group_file: String,
    pub ftn_file: String,
    pub user_file: String,
    pub caller_log: String,
    pub transfer_log: String,
    pub logon_survey: String,
    pub logon_answer: String,
    pub logoff_survey: String,
    pub logoff_answer: String,
    pub newask_survey: String,
    pub newask_answer: String,
}

#[derive(Serialize, Debug)]
pub struct PathsSettingsResponse {
    #[serde(flatten)]
    pub settings: PathsSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct PathsSettingsPatch {
    #[serde(flatten)]
    pub settings: PathsSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AccountingSettingsDto {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub use_money: bool,
    #[serde(default)]
    pub concurrent_tracking: bool,
    #[serde(default)]
    pub ignore_empty_sec_level: bool,
    pub peak_usage_start: String,
    pub peak_usage_end: String,
    /// Seven Y/N characters, Sunday first
    pub peak_days_of_week: String,
    pub peak_holiday_list_file: String,
    pub cfg_file: String,
    pub tracking_file: String,
    pub info_file: String,
    pub warning_file: String,
    pub logoff_file: String,
}

#[derive(Serialize, Debug)]
pub struct AccountingSettingsResponse {
    #[serde(flatten)]
    pub settings: AccountingSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct AccountingSettingsPatch {
    #[serde(flatten)]
    pub settings: AccountingSettingsDto,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct FunctionKeysSettingsDto {
    pub keys: [String; 10],
}

#[derive(Serialize, Debug)]
pub struct FunctionKeysSettingsResponse {
    #[serde(flatten)]
    pub settings: FunctionKeysSettingsDto,
    pub fingerprint: String,
}

#[derive(Deserialize)]
pub struct FunctionKeysSettingsPatch {
    #[serde(flatten)]
    pub settings: FunctionKeysSettingsDto,
    pub fingerprint: String,
}

// ---------------------------------------------------------------- conferences

pub const CONFERENCE_TYPES: &[(&str, &str)] = &[
    ("Normal", "Normal"),
    ("InternetEmail", "Internet e-mail"),
    ("InternetUsenet", "Internet usenet"),
    ("UsnetModeratedNewsgroup", "Usenet moderated newsgroup"),
    ("UsnetPublicNewsgroup", "Usenet public newsgroup"),
    ("FidoConference", "FidoNet conference"),
];

pub const SORT_ORDERS: &[(&str, &str)] = &[
    ("0", "No sorting"),
    ("1", "Name ascending"),
    ("2", "Name descending"),
    ("3", "Date ascending"),
    ("4", "Date descending"),
];

#[derive(Serialize, Debug)]
pub struct ConferenceSummaryDto {
    pub index: usize,
    pub name: String,
    pub is_public: bool,
    pub is_read_only: bool,
    pub conference_type: String,
    pub required_security: String,
    pub password_set: bool,
}

#[derive(Serialize, Debug)]
pub struct ConferenceListResponse {
    pub conferences: Vec<ConferenceSummaryDto>,
    pub file: String,
    pub fingerprint: String,
}

/// One conference. The join password is never sent to the client; `new_password`
/// and `clear_password` are write-only fields.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct ConferenceDto {
    pub name: String,
    pub conference_type: String,

    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub is_read_only: bool,
    #[serde(default)]
    pub echo_mail_in_conference: bool,
    #[serde(default)]
    pub force_echomail: bool,
    #[serde(default)]
    pub auto_rejoin: bool,
    #[serde(default)]
    pub allow_view_conf_members: bool,
    #[serde(default)]
    pub private_uploads: bool,
    #[serde(default)]
    pub private_msgs: bool,
    #[serde(default)]
    pub disallow_private_msgs: bool,
    #[serde(default)]
    pub allow_aliases: bool,
    #[serde(default)]
    pub show_intro_in_scan: bool,
    #[serde(default)]
    pub use_main_commands: bool,
    #[serde(default)]
    pub record_origin: bool,
    #[serde(default)]
    pub prompt_for_routing: bool,
    #[serde(default)]
    pub long_to_names: bool,

    pub required_security: String,
    pub sec_attachments: String,
    pub sec_write_message: String,
    pub sec_request_rr: String,
    pub sec_carbon_copy: String,

    pub carbon_list_limit: u8,
    pub add_conference_security: i32,
    pub add_conference_time: u16,
    pub pub_upload_sort: u8,
    pub private_upload_sort: u8,

    pub charge_time: f64,
    pub charge_msg_read: f64,
    pub charge_msg_write: f64,

    pub users_menu: String,
    pub sysop_menu: String,
    pub news_file: String,
    pub intro_file: String,
    pub attachment_location: String,
    pub command_file: String,
    pub pub_upload_location: String,
    pub pub_upload_metadata: String,
    pub private_upload_location: String,
    pub private_upload_metadata: String,
    pub doors_menu: String,
    pub doors_file: String,
    pub blt_menu: String,
    pub blt_file: String,
    pub survey_menu: String,
    pub survey_file: String,
    pub dir_menu: String,
    pub dir_file: String,
    pub area_menu: String,
    pub area_file: String,

    /// Write only: a non empty value replaces the join password.
    #[serde(default, skip_serializing)]
    pub new_password: String,
    /// Write only: removes the join password.
    #[serde(default, skip_serializing)]
    pub clear_password: bool,
}

#[derive(Serialize, Debug)]
pub struct ConferenceResponse {
    pub index: usize,
    pub settings: ConferenceDto,
    pub password_set: bool,
    pub file: String,
    pub fingerprint: String,
}
