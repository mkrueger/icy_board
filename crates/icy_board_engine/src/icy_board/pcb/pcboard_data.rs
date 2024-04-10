use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use icy_ppe::{
    parser::Encoding,
    tables::{CP437_TO_UNICODE, UNICODE_TO_CP437},
    Res,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcbSysopInfo {
    /// Sysop Dislay Name
    pub sysop: String,
    /// Sysop local password
    pub password: String,
    ///  Require Local Password to drop PCBoard to DOS (v15.0)
    pub require_pwrd_to_exit: bool,
    /// Use sysop real name instead of 'SYSOP'
    pub use_real_name: bool,
    /// Use local graphics
    pub use_local_graphics: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcbSubscriptionMode {
    /// run in subscription mode
    pub is_enabled: bool,
    /// default days in new subscription period (v14.5)
    pub subscription_length: i32,
    /// default expired security level (v14.5)
    pub default_expired_level: u8,
    /// days prior to subscription expiration (v14.5)
    pub warning_days: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcbBoardData {
    /// PCBoard version
    pub version: String,

    pub sysop_info: PcbSysopInfo,

    /// Sysop function security levels
    pub sysop_security: PCBSysopSecurityLevels,
    /// paths
    pub path: FileLocations,

    pub colors: PCBColorConfiguration,
    pub modem: ModemInformation,

    pub subscription_info: PcbSubscriptionMode,

    ///  disable message base scan prompt
    pub disable_scan: bool,
    ///  disable screen clear after 3 minutes
    pub disable_cls: bool,
    ///  disable registration edits
    pub disable_edits: bool,
    ///  disable high order ascii filter
    pub disable_filter: bool,
    ///  disable quick logon
    pub disable_quick: bool,
    ///  disable password check on return from DOS
    pub disable_password: bool,
    ///  support multi-lingual operation
    pub multi_lingual: bool,
    ///  allow only password changes
    pub allow_pwrd_only: bool,
    ///  run system as a closed board
    pub closed_board: bool,
    ///  run in non-graphics mode only
    pub non_graphics: bool,
    ///  exit to dos after each caller
    pub exit_to_dos: bool,
    ///  include user's city/state in node chat display
    pub include_city: bool,
    ///  eliminate screen snow
    pub eliminate_snow: bool,

    ///  allow ESC codes in messages
    pub allow_esc_codes: bool,
    /// : bool,allow CC: messages
    pub allow_ccs: bool,
    /// : bool,valide TO: name
    pub validate_to: bool,
    ///  update Last Read pointer afer new msg
    pub last_read_update: bool,
    ///  enforce daily time limit
    pub enforce_time: bool,
    ///  display news only if changed since last call
    pub display_news: char,
    ///  disable CTS drop during disk writes
    pub disable_ctsdrop: bool,
    ///  num of minutes for keyboard timeout
    pub kbd_timeout: i32,
    ///  maximum lines in scrollback buffer (v14.5)
    pub max_scroll_back: i32,

    ///  pre-load pcbtext file (v14.5)
    pub fast_text: bool,
    ///  pre-load cnames file (v14.5)
    pub fast_cnames: bool,
    ///  include Upload By: in upload description (v14.5)
    pub upload_by: bool,
    ///  test files after upload
    pub test_uploads: bool,
    ///  include settings display on login (v14.5)
    pub show_settings: bool,
    ///  allow shelling to DOS (v14.5)
    pub allow_shell: bool,
    ///  use Slave Card updating (v14.5)
    pub slaves: bool,
    ///  running under a network
    pub network: bool,
    ///  node number of this node
    pub node_num: i32,
    ///  network timeout delay (20-99 seconds)
    pub net_timeout: i32,
    ///  node chat delay inveral when idle (5-99 sec)
    pub chat_delay: i32,
    ///  name of board
    pub board_name: String,
    ///  name of message capture file
    pub cap_file: String,
    ///  name and loc of batch file to view archives
    pub view_batch: String,
    ///  archive extension
    pub view_ext: String,
    ///  function key definitions
    pub func_keys: [String; 10],
    /// public conference string
    pub pub_conf: String,
    ///  number of conferences (1 thru 65534)
    pub num_conf: i32,
    ///  number of conference areas (Main=1 thru 65535)
    pub num_areas: i32,
    ///  maximum message lines allowed (1-400)
    pub max_msg_lines: i32,
    ///  default foreground color
    pub default_color: i32,
    ///  default intensity (0 or 1)
    pub default_intensity: i32,
    ///  timed event active
    pub event_active: bool,
    ///  time to activate event '00:00'
    pub event_time: String,
    /// minutes to suspend board activity before event
    pub event_suspend: i32,
    ///  stop uploads initiation during suspense time
    pub event_stop_uplds: bool,
    ///  slide event if user online
    pub event_slide: bool,
    ///  upload buffer size (4-32)
    pub upload_buf_size: i32,
    /// disable upload drive size check
    pub disable_drive_check: bool,
    ///  parallel port number (1-3 or 0)
    pub parallel_port_num: i32,
    /// stop uploads when free space < #K
    pub stop_free_space: i32,
    /// total maximum messages in a capture file (v14.5)
    pub max_total_msgs: i32,
    /// maximum messages per conference in capture (v14.5)
    pub max_conf_msgs: i32,
    ///  default to 'Y Q' message scanning (v14.5)
    pub quick_scan: bool,
    ///  default to 'Y A' message scanning (v14.5)
    pub scan_all: bool,
    /// stop upload minutes prior to an event (v14.5)
    pub min_prior_to_event: i32,
    ///  use NEWASK plus built-in questions (v14.5)
    pub use_new_ask_file: bool,
    ///  allow one-name users to log into system (v14.5)
    pub allow_one_name: bool,
    /// stop the clock during capture file download (v14.5)
    pub stop_clock_on_cap: bool,
    ///  start time to allow sysop page (v14.5)
    pub sysop_start: String,
    ///  stop  time to allow sysop page (v14.5)
    pub sysop_stop: String,
    ///  log caller number to disk (v14.5)
    pub log_caller_number: bool,
    ///  log connect string to disk (v14.5)
    pub log_connect_str: bool,
    ///  log security level to disk (v14.5)
    pub log_sec_level: bool,
    ///  re-read PWRD upon joining a conference (v14.5)
    pub conf_pwrd_adjust: bool,
    ///  confirm caller name (v14.5)
    pub confirm_caller: bool,
    ///  allow password failure comment (v14.5)
    pub allow_pwrd_comment: bool,
    ///  guard logoff command (v14.5)
    pub guard_logoff: bool,
    ///  number of upload description lines (v14.5)
    pub num_desc_lines: i32,
    ///  user security levels (v15.0)
    pub user_levels: UserSecurityLevels,

    ///  drive letters for slow drives (v15.0)
    pub slow_drives: String,
    ///  swap out during normal shell (v15.0)
    pub swap: bool,
    ///  don't allow batch uploads / force first name (v15.0)
    pub no_batch_up: bool,
    ///  force comments to main board (v15.0)
    pub force_main: bool,
    ///  LineSeparator and tilde changes (v15.0)
    pub foreign: bool,
    ///  watch for incompleted connections (v15.0)
    pub monitor_modem: bool,
    ///  force 16550 to be treated as a 16450 (v15.0)
    pub no16550: bool,
    ///  force 16450 to be treated as a 16550 (v15.0)
    pub force16550_a: bool,
    ///  reset modem every 5 minutes while waiting (v15.0)
    pub auto_reset: bool,
    ///  float node number (v15.0)
    pub float_node_number: bool,
    ///  promote D and U commands to DB and UB (v15.0)
    pub promote_batch: bool,
    ///  automatically register new user in all confs (v15.0)
    pub auto_reg_conf: bool,
    ///  scan bulletins when joining a conference (v15.0)
    pub scan_blts: bool,
    ///  create msgs file if missing (v15.0)
    pub auto_make_msgs: bool,
    ///  verify cd loss before recycling (v15.0)
    pub verify_cdloss: bool,
    ///  encrypt users file (v15.0)
    pub encrypt: bool,
    ///  time credit (times 10, i.e. 25 = 2.5 credit (v15.0)
    pub upload_credit: i32,
    ///  byte credit (times 10, i.e. 25 = 2.5 credit (v15.0)
    pub byte_credit: i32,

    ///  ORIGIN information (v15.0)
    pub origin: String,

    ///  requested size for DOS's environment (v15.0)
    pub env_size: i32,
    ///  Security Level to override low baud limit (v15.0)
    pub low_baud_sec_override: i32,
    ///  enable FIDO setup and processing (v15.0)
    pub enable_fido: bool,
    ///  TRUE allows the node to process incoming PKT
    pub fido_process_in: bool,
    ///  TRUE allows the node to process orphan PKT
    pub fido_process_orphan: bool,
    ///  TRUE allows the node to export mail
    pub fido_process_out: bool,
    ///  TRUE allows the node to dial out
    pub fido_dial_out: bool,
    ///  Minutes between processing incoming PKT
    pub fido_pkt_freq: i32,
    ///  Minutes between checking to export mail
    pub fido_export_freq: i32,
    ///  Minutes between checking for need to dial out
    pub fido_mail_freq: i32,
    pub fido_default_zone: i32,
    pub fido_default_net: i32,
    ///  FIDO import/export configuration file (v15.0) */ /* no longer used as of v15.22
    pub fido_config: String,
    ///  FIDO queue filename (v15.0) */ /* no longer used as of v15.22
    pub fido_queue: String,
    ///  ask to read mail waiting (v15.0)
    pub prompt_to_read_mail: bool,

    ///  name of qwk packet file (v15.0)
    pub qwk_file: String,
    ///  TRUE if $$logon or $$logoff.bat should swap (v15.0)
    pub swap_during_bat: bool,
    ///  TRUE if users.sys should be made for $$logon (v15.0)
    pub user_sys_during_bat: bool,
    ///  TRUE to default to graphics mode on login (v15.0)
    pub default_graphics: bool,
    ///  batch file to run after slow-drive copy (v15.0)
    pub slow_drive_bat: String,
    ///  TRUE if ALIAS can be changed after setting it (v15.0)
    pub allow_alias_change: bool,
    ///  TRUE if local logins should be ignored (v15.0)
    pub exclude_locals: bool,
    ///  Number of days PWRD is valid before expiring (v15.0)
    pub pwrd_update: i32,
    ///  Number of days prior to WARN of PWRD expiring (v15.0)
    pub pwrd_warn: i32,
    ///  TRUE if group chat should be captured to disk (v15.0)
    pub record_group_chat: bool,
    ///  TRUE if sysop wants ALIAS shown in USERNET (v15.0)
    pub show_alias: bool,
    ///  Minimum Password Length when PSA is installed (v15.0)
    pub min_pwrd_len: i32,
    ///  TRUE if handles are allowed in Group Chat (v15.0)
    pub allow_handles: bool,
    ///  TRUE if forcing INTRO to be displayed (v15.1)
    pub force_intro: bool,
    ///  TRUE to skip protocol selection for new users (v15.1)
    pub skip_protocol: bool,
    ///  TRUE to skip alias selection for new users
    pub skip_alias: bool,
    ///  TRUE to exit to DOS if no carrier after connect*/ /* v15.1
    pub no_carrier_exit: bool,
    ///  TRUE if set to use OS/2 COMM Driver
    pub os2_driver: bool,
    ///  TRUE if accounting features are enabled (v15.2)
    pub enable_accounting: bool,
    ///  TRUE if currency values should be shown (v15.2)
    pub show_currency: bool,
    ///  TRUE if charges run concurrently (v15.2)
    pub concurrent: bool,
    ///  TRUE if the DropSecLevel should not be used (v15.2)
    pub ignore_drop_sec_level: bool,
    ///  Start of peak usage hours (v15.2)
    pub peak_start: String,
    ///  End of peak usage hours (v15.2)
    pub peak_end: String,
    ///  bit encoded flags (bit0=sun, bit1=mon, etc (v15.2)
    pub peak_days: String,
    ///  name/loc of Peak Holidays File (v15.2)
    pub holidays_file: String,
    ///  name/loc of Account Rate File (v15.2)
    pub account_config: String,
    ///  name/loc of Account Info File (v15.2)
    pub account_info: String,
    ///  name/loc of Account Warning File (v15.2)
    pub account_warn: String,
    ///  name/loc of Account Tracking File (v15.2)
    pub account_track: String,
    ///  name/loc of Account Logoff Info File (v15.2)
    pub account_logoff: String,
    ///  location for UUCP files (v15.2)
    pub uucp_path: String,
    ///  location for UUCP Spool files (v15.2)
    pub uucp_spool_path: String,
    ///  location for UUCP Log files (v15.2)
    pub uucp_log_path: String,
    ///  name/loc of compress batch file for UUCP (v15.2)
    pub comp_bat_file: String,
    ///  name/loc of decompress batch file for UUCP (v15.2)
    pub de_comp_bat_file: String,
    ///  UUCP Name (v15.2)
    pub uucp_name: String,
    ///  UUCP Domain Name (v15.2)
    pub uucp_domain_name: String,
    ///  UUCP Email Host Name (v15.2)
    pub uucp_email_host: String,
    ///  UUCP News Host Name (v15.2)
    pub uucp_news_host: String,
    ///  UUCP Default News Distribution (v15.2)
    pub uucp_def_dist: String,
    ///  Time Zone - offset from GMT (v15.2)
    pub uucp_time_zone: String,
    ///  UUCP Name Separator (v15.2)
    pub uucp_separator: String,
    ///  Organization Name (v15.2)
    pub organization: String,
    ///  S = Strip, R = Replace, C = Convert (v15.2)
    pub uucp_high_ascii: char,
    ///  number of UUCP Email Conference (v15.2)
    pub uucp_email_conf: i32,
    ///  number of UUCP Junk Conference (v15.2)
    pub uucp_junk_conf: i32,
    ///  Bang Domain (v15.2)
    pub uucp_bang_domain: bool,
    ///  Sub Domain (v15.2)
    pub uucp_sub_domain: bool,
    ///  List of UUCP Moderators (v15.2)
    pub uucp_mod_file: String,
    ///  TRUE if import takes place after file xfer (v15.21)
    pub fido_import_after_xfer: bool,
    ///  security level required for crash mail (v15.21)
    pub fido_crash_sec: i32,
    ///  controls the amount of logging performed (v15.21)
    pub fido_log_level: i32,
    ///  create MSG files (v15.21)
    pub fido_create_msg: bool,
    ///  enable fido routing (v15.21)
    pub fido_enable_routing: bool,
    /// FIDO configuration files location (v15.22)
    pub fido_loc: String,
    ///  Store msgs to unknown users in secure location (v15.22)
    pub fido_secure: bool,
    ///  Change sysop to FIDO_SYSOP on import (v15.22)
    pub fido_sysop_change: bool,
    ///  Check for dupes using message path (v15.22)
    pub fido_check_dupe_path: bool,
    ///  Check for dupes using msg id (v15.22)
    pub fido_check_dupe_msg_id: bool,
    ///  Number of messages per conference to track (v15.22)
    pub fido_num_msgs_to_track: i32,
    ///  Generate response messages (v15.22)
    pub fido_make_response: bool,
    ///  Enable pass throughs (v15.22)
    pub fido_enable_pass_thru: bool,
    ///  Enable areafix forwarding (v15.22)
    pub fido_enable_area_fix: bool,
    ///  Automatically add fido areas (v15.22)
    pub fido_auto_add: bool,
    ///  Re-Address Routed Packets (v15.22)
    pub fido_re_address: bool,
    ///  Route Echo Mail (v15.22)
    pub fido_route_echo_mail: bool,
    /// 0=Nothing, 1=Minimize Self, 2=Minimize Shell    $$LOGON/LOGOFF batch file (v15.22)
    pub minimize_log_on_off: i32,
    ///  External Protocols (v15.22)
    pub minimize_protocols: i32,
    ///  Doors (v15.22)
    pub minimize_doors: i32,
    ///  Upload verification process (v15.22)
    pub minimize_file_test: i32,
    ///  File View (v15.22)
    pub minimize_file_view: i32,
    ///  PCBQWK and PCBCMPRS batch files (v15.22)
    pub minimize_compress: i32,
    ///  All Other Shells (v15.22)
    pub minimize_shells: i32,
    ///  Priority for regular processing (v15.22)
    pub priority_normal: i32,
    ///  Priority for external protocols (v15.22)
    pub priority_protocols: i32,
    ///  Priority for PCBQWK and PCBCMPRS batch files (v15.22)
    pub priority_compress: i32,
    ///  Priority for all other shells (v15.22)
    pub priority_shells: i32,
    ///  Priority for fido import (v15.22)
    pub priority_fido_in: i32,
    ///  Priority for fido export (v15.22)
    pub priority_fido_out: i32,
    ///  command for performing a network file copy (v15.3)
    pub net_copy: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModemInformation {
    /// Seconds to wait for connect
    pub seconds: i32,
    /// Device of serial port (COM1, /dev/ttyS0, etc.)
    pub modem_serial_device: String,
    /// Modem bps speed
    pub modem_speed: i32,
    /// Lock modem speed
    pub lock_speed: bool,
    /// Modem initialization string
    pub modem_init: String,
    /// Modem initialization string 2
    pub modem_init2: String,
    /// Modem off hook string
    pub modem_off_hook: String,
    /// Modem answer string
    pub modem_answer: String,
    /// Modem dial string
    pub modem_dial: String,
    /// Numer of redials before abortingi dial sequence
    pub num_redials: i32,
    /// Maximum number of tries to connect
    pub max_tries: i32,
    /// Disable CTS/RTS check
    pub disable_cts: bool,
    pub use_hayes_9600: bool,
    pub use_fast_comm: bool,
    /// Reset modem during recycle
    pub reset_modem: bool,
    /// Take modem off hook during recycle
    pub take_modem_off_hook: bool,
    /// Runing under a packet switch network (???)
    pub packed: bool,

    pub answer_ring: bool,
    pub allow_7e1: bool,
    pub allow_low_baud: bool,
    pub allow_low_start_time: String,
    pub allow_low_stop_time: String,
    pub low_baud_limit: i32,
    pub modem_delay: i32,
    pub rings_require: i32,
    pub irq_num: i32,
    pub base_address: i32,
    /// leave DTR up on exit to DOS
    pub leave_dtr_up: bool,
    pub share_irqs: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PCBColorConfiguration {
    ///  color code for default color (v15.0)
    pub default: i32,
    ///  color for DATE line of message header (v15.0)
    pub msg_hdr_date: i32,
    ///  color for TO   line of message header (v15.0)
    pub msg_hdr_to: i32,
    ///  color for FROM line of message header (v15.0)
    pub msg_hdr_from: i32,
    ///  color for SUBJ line of message header (v15.0)
    pub msg_hdr_subj: i32,
    ///  color for READ line of message header (v15.0)
    pub msg_hdr_read: i32,
    ///  color for CONF line of message header (v15.0)
    pub msg_hdr_conf: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PCBSysopSecurityLevels {
    pub sysop: i32,
    pub read_all_comments: i32,
    pub read_all_mail: i32,
    pub copy_move_messages: i32,
    pub enter_at_vars_in_messages: i32,
    pub edit_any_message: i32,
    pub not_update_msg_read_status: i32,
    pub use_broadcast_command: i32,
    pub view_private_uploads: i32,
    pub enter_generic_message: i32,
    pub edit_message_headers: i32,
    pub protect_unprotect_messages: i32,
    pub overwrite_uploads: i32,
    pub set_pack_out_date_on_messages: i32,
    pub see_all_return_receipt_messages: i32,

    pub subs: i32,
    pub edit_all: i32,
    pub read_only: i32,
    pub sec_15: i32,
    pub unused0: i32,
    pub keep_msg: i32,
    pub seeretrcpt: i32,

    pub sec_1_view_caller_log: i32,
    pub sec_2_view_usr_list: i32,
    pub sec_3_pack_renumber_msg: i32,
    pub sec_4_recover_deleted_msg: i32,
    pub sec_5_list_message_hdr: i32,
    pub sec_6_view_any_file: i32,
    pub sec_7_user_maint: i32,
    pub sec_8_pack_usr_file: i32,
    pub sec_9_exit_to_dos: i32,
    pub sec_10_shelled_dos_func: i32,
    pub sec_11_view_other_nodes: i32,
    pub sec_12_logoff_alt_node: i32,
    pub sec_13_drop_alt_node_to_dos: i32,
    pub sec_14_drop_to_dos: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserSecurityLevels {
    pub cmd_a: i32,
    pub cmd_b: i32,
    pub cmd_c: i32,
    pub cmd_d: i32,
    pub cmd_e: i32,
    pub cmd_f: i32,
    pub cmd_g: i32,
    pub cmd_h: i32,
    pub cmd_i: i32,
    pub cmd_j: i32,
    pub cmd_k: i32,
    pub cmd_l: i32,
    pub cmd_m: i32,
    pub cmd_n: i32,
    pub cmd_o: i32,
    pub cmd_p: i32,

    pub cmd_q: i32,
    pub cmd_r: i32,
    pub cmd_s: i32,
    pub cmd_t: i32,
    pub cmd_u: i32,
    pub cmd_v: i32,
    pub cmd_w: i32,
    pub cmd_x: i32,
    pub cmd_y: i32,
    pub cmd_z: i32,
    pub cmd_chat: i32,
    pub cmd_open_door: i32,
    pub cmd_test_file: i32,
    pub cmd_show_user_list: i32,
    pub cmd_who: i32,

    pub batch_file_transfer: i32,
    pub edit_own_messages: i32,
    pub agree_to_register: i32,
    pub refuse_to_register: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileLocations {
    /// location of help files
    pub help_loc: String,
    /// location of security message files
    pub sec_loc: String,
    /// location of chat files
    pub chat_loc: String,
    /// location of pcbtext files
    pub text_loc: String,
    /// location of index files
    pub index_loc: String,
    /// location of temporary files
    pub tmp_loc: String,
    /// name of users file
    pub usr_file: String,
    /// name of users info file
    pub inf_file: String,
    /// name and location of callers file
    pub clr_file: String,
    /// name and location of conference data file
    pub conference_file: String,
    /// name and location of pwrd file
    pub pwd_file: String,
    /// name and location of fsec file
    pub fsec_file: String,
    /// name and location of upsec file
    pub upsec_file: String,
    /// name and location of tcan file
    pub tcan_file: String,

    ///  location of command display files (v15.1)
    pub cmd_display_files_loc: String,

    /// name and location of welcome file
    pub welcome_file: String,
    /// name and location of newuser file
    pub newuser_file: String,
    /// name and location of closed file
    pub closed_file: String,
    /// name and location of warning file
    pub warning_file: String,
    /// name and location of expired file
    pub expired_file: String,

    /// name and location of USERNET.DAT file
    pub usernet_file: String,
    /// name and location of conference join menu
    pub conf_menu: String,
    /// name and location of newreg file questions
    pub newreg_file: String,
    /// name and location of non-reg user's answer file
    pub answer_file: String,
    /// name and location of protocol data file
    pub protocol_data_file: String,
    /// name and location of download summary file
    pub download_file: String,
    /// name and loc of logoff script questionnaire
    pub logoff_script: String,
    /// name and loc of logoff script answers
    pub logoff_answer: String,
    /// name and loc of pcbml.dat file
    pub pcml_dat_file: String,
    /// name and loc of group chat topic/intro file
    pub group_chat: String,
    /// name and loc of PCBFILER.DEF file
    pub color_file: String,

    ///  path to swap file (v15.0)
    pub swap_path: String,
    ///  name and location of EVENT.DAT (v15.0)
    pub event_dat_file: String,
    ///  location of event files (v15.0)
    pub event_files: String,
    ///  name and location of all files DLPATH.LST (v15.0)
    pub all_files_list: String,

    ///  name/location of CMD.LST to use for defaults (v15.0)
    pub cmd_lst: String,
    ///  name and loc of logon script questionnaire (v15.0)
    pub log_on_scr: String,
    ///  name and loc of logon script answers (v15.0)
    pub log_on_ans: String,
    ///  name of File Trashcan for stopping uploads (v15.0)
    pub file_tcan: String,

    ///  name and location of chat.dat (v15.0) Levels
    pub chat_file: String,
    ///  name and location of pcbstats.dat (v15.0)
    pub stats_file: String,
    ///  name and location of CHAT menu (v15.0)
    pub chat_menu: String,
    ///  name and location of NOANSI file (v15.0)
    pub no_ansi: String,
}

/// # Errors
pub fn read_line(reader: &mut BufReader<File>, encoding: Encoding) -> Res<String> {
    let mut buf = Vec::new();
    reader.read_until(b'\n', &mut buf)?;
    while buf.ends_with(&[b'\r']) || buf.ends_with(&[b'\n']) {
        buf.pop();
    }
    let res = if encoding == Encoding::CP437 {
        buf.into_iter()
            .map(|b| CP437_TO_UNICODE[b as usize])
            .collect::<String>()
    } else {
        String::from_utf8_lossy(&buf).to_string()
    };
    Ok(res)
}

/// # Errors
pub fn read_bool(reader: &mut BufReader<File>, encoding: Encoding) -> Res<bool> {
    Ok(&read_line(reader, encoding)? == "-1")
}

/// # Errors
pub fn read_int(reader: &mut BufReader<File>, encoding: Encoding) -> Res<i32> {
    let line = read_line(reader, encoding)?;
    Ok(line.parse::<i32>()?)
}

/// # Errors
pub fn read_hex(reader: &mut BufReader<File>, encoding: Encoding) -> Res<i32> {
    let line = read_line(reader, encoding)?;
    Ok(i32::from_str_radix(&line, 16)?)
}

/// # Panics
pub fn append_line(writer: &mut Vec<u8>, encoding: Encoding, s: &str) {
    if encoding == Encoding::CP437 {
        for c in s.chars() {
            writer.push(*UNICODE_TO_CP437.get(&c).unwrap());
        }
    } else {
        writer.extend_from_slice(s.as_bytes());
    };
    writer.extend(b"\r\n");
}

pub fn append_bool(writer: &mut Vec<u8>, encoding: Encoding, b: bool) {
    append_line(writer, encoding, if b { "-1" } else { "0" });
}

pub fn append_int(writer: &mut Vec<u8>, encoding: Encoding, i: i32) {
    append_line(writer, encoding, &i.to_string());
}

pub fn append_hex(writer: &mut Vec<u8>, encoding: Encoding, i: i32) {
    append_line(writer, encoding, &i.to_string());
}

impl PcbBoardData {
    /// .
    ///
    /// # Errors
    /// # Panics
    ///
    /// Panics if .
    #[allow(clippy::field_reassign_with_default)]
    pub fn import_pcboard(filename: &str) -> Res<Self> {
        let mut reader = BufReader::new(File::open(filename)?);

        let version = read_line(&mut reader, Encoding::Utf8)?;
        let encoding = if version.starts_with("*** PCBoard") {
            Encoding::CP437
        } else {
            Encoding::Utf8
        };

        let mut ret = Self::default();
        ret.sysop_info.sysop = read_line(&mut reader, encoding)?;
        ret.sysop_info.password = read_line(&mut reader, encoding)?;
        ret.sysop_info.use_real_name = read_bool(&mut reader, encoding)?;
        ret.sysop_info.use_local_graphics = read_bool(&mut reader, encoding)?;

        ret.sysop_security.read_all_comments = read_int(&mut reader, encoding)?;
        ret.sysop_security.read_all_mail = read_int(&mut reader, encoding)?;
        ret.sysop_security.sysop = read_int(&mut reader, encoding)?;
        ret.sysop_security.copy_move_messages = read_int(&mut reader, encoding)?;

        ret.sysop_security.sec_1_view_caller_log = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_2_view_usr_list = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_3_pack_renumber_msg = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_4_recover_deleted_msg = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_5_list_message_hdr = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_6_view_any_file = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_7_user_maint = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_8_pack_usr_file = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_9_exit_to_dos = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_10_shelled_dos_func = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_11_view_other_nodes = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_12_logoff_alt_node = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_13_drop_alt_node_to_dos = read_int(&mut reader, encoding)?;
        ret.sysop_security.sec_14_drop_to_dos = read_int(&mut reader, encoding)?;

        ret.path.help_loc = read_line(&mut reader, encoding)?;
        ret.path.sec_loc = read_line(&mut reader, encoding)?;
        ret.path.chat_loc = read_line(&mut reader, encoding)?;
        ret.path.text_loc = read_line(&mut reader, encoding)?;
        ret.path.index_loc = read_line(&mut reader, encoding)?;
        ret.path.usr_file = read_line(&mut reader, encoding)?;
        ret.path.clr_file = read_line(&mut reader, encoding)?;
        ret.path.conference_file = read_line(&mut reader, encoding)?;
        ret.path.pwd_file = read_line(&mut reader, encoding)?;
        ret.path.fsec_file = read_line(&mut reader, encoding)?;
        ret.path.upsec_file = read_line(&mut reader, encoding)?;
        ret.path.tcan_file = read_line(&mut reader, encoding)?;
        ret.path.welcome_file = read_line(&mut reader, encoding)?;
        ret.path.newuser_file = read_line(&mut reader, encoding)?;
        ret.path.closed_file = read_line(&mut reader, encoding)?;
        ret.path.warning_file = read_line(&mut reader, encoding)?;
        ret.path.expired_file = read_line(&mut reader, encoding)?;
        ret.path.usernet_file = read_line(&mut reader, encoding)?;
        ret.path.conf_menu = read_line(&mut reader, encoding)?;
        ret.path.newreg_file = read_line(&mut reader, encoding)?;
        ret.path.answer_file = read_line(&mut reader, encoding)?;
        ret.path.protocol_data_file = read_line(&mut reader, encoding)?;
        ret.path.download_file = read_line(&mut reader, encoding)?;
        ret.path.logoff_script = read_line(&mut reader, encoding)?;
        ret.path.logoff_answer = read_line(&mut reader, encoding)?;
        ret.path.pcml_dat_file = read_line(&mut reader, encoding)?;
        ret.path.group_chat = read_line(&mut reader, encoding)?;

        ret.modem.seconds = read_int(&mut reader, encoding)?;
        ret.modem.modem_serial_device = read_line(&mut reader, encoding)?;
        ret.modem.modem_speed = read_int(&mut reader, encoding)?;
        ret.modem.lock_speed = read_bool(&mut reader, encoding)?;
        ret.modem.modem_init = read_line(&mut reader, encoding)?;
        ret.modem.modem_off_hook = read_line(&mut reader, encoding)?;
        ret.modem.disable_cts = read_bool(&mut reader, encoding)?;
        ret.modem.use_hayes_9600 = read_bool(&mut reader, encoding)?;
        ret.modem.use_fast_comm = read_bool(&mut reader, encoding)?;
        ret.modem.reset_modem = read_bool(&mut reader, encoding)?;
        ret.modem.take_modem_off_hook = read_bool(&mut reader, encoding)?;
        ret.modem.packed = read_bool(&mut reader, encoding)?;
        ret.modem.answer_ring = read_bool(&mut reader, encoding)?;
        ret.modem.allow_7e1 = read_bool(&mut reader, encoding)?;
        ret.modem.allow_low_baud = read_bool(&mut reader, encoding)?;
        ret.modem.allow_low_start_time = read_line(&mut reader, encoding)?;
        ret.modem.allow_low_stop_time = read_line(&mut reader, encoding)?;

        ret.disable_scan = read_bool(&mut reader, encoding)?;
        ret.disable_cls = read_bool(&mut reader, encoding)?;
        ret.disable_edits = read_bool(&mut reader, encoding)?;
        ret.disable_filter = read_bool(&mut reader, encoding)?;
        ret.disable_quick = read_bool(&mut reader, encoding)?;
        ret.disable_password = read_bool(&mut reader, encoding)?;
        ret.multi_lingual = read_bool(&mut reader, encoding)?;
        ret.allow_pwrd_only = read_bool(&mut reader, encoding)?;
        ret.closed_board = read_bool(&mut reader, encoding)?;
        ret.non_graphics = read_bool(&mut reader, encoding)?;
        ret.exit_to_dos = read_bool(&mut reader, encoding)?;
        ret.include_city = read_bool(&mut reader, encoding)?;
        ret.eliminate_snow = read_bool(&mut reader, encoding)?;
        ret.subscription_info.is_enabled = read_bool(&mut reader, encoding)?;
        ret.allow_esc_codes = read_bool(&mut reader, encoding)?;
        ret.allow_ccs = read_bool(&mut reader, encoding)?;
        ret.validate_to = read_bool(&mut reader, encoding)?;
        ret.last_read_update = read_bool(&mut reader, encoding)?;
        ret.enforce_time = read_bool(&mut reader, encoding)?;
        ret.display_news = match read_int(&mut reader, encoding)? {
            -1 => 'Y',
            1 => 'A',
            _ => 'N',
        };
        ret.disable_ctsdrop = read_bool(&mut reader, encoding)?;
        ret.kbd_timeout = read_int(&mut reader, encoding)?;
        ret.network = read_bool(&mut reader, encoding)?;
        ret.node_num = read_int(&mut reader, encoding)?;
        ret.net_timeout = read_int(&mut reader, encoding)?;
        ret.chat_delay = read_int(&mut reader, encoding)?;

        ret.board_name = read_line(&mut reader, encoding)?;
        ret.view_batch = read_line(&mut reader, encoding)?;
        ret.view_ext = read_line(&mut reader, encoding)?;
        for i in 0..10 {
            ret.func_keys[i] = read_line(&mut reader, encoding)?;
        }
        ret.pub_conf = read_line(&mut reader, encoding)?;
        ret.num_conf = read_int(&mut reader, encoding)?;
        ret.max_msg_lines = read_int(&mut reader, encoding)?;
        ret.default_color = read_int(&mut reader, encoding)?;
        ret.default_intensity = read_int(&mut reader, encoding)?;
        ret.event_active = read_bool(&mut reader, encoding)?;
        ret.event_time = read_line(&mut reader, encoding)?;
        ret.event_suspend = read_int(&mut reader, encoding)?;
        ret.event_stop_uplds = read_bool(&mut reader, encoding)?;
        ret.event_slide = read_bool(&mut reader, encoding)?;
        ret.upload_buf_size = read_int(&mut reader, encoding)?;
        ret.disable_drive_check = read_bool(&mut reader, encoding)?;
        ret.parallel_port_num = read_int(&mut reader, encoding)?;
        ret.stop_free_space = read_int(&mut reader, encoding)?;

        ret.user_levels.cmd_a = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_b = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_c = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_d = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_e = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_f = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_g = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_h = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_i = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_j = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_k = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_l = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_m = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_n = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_o = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_p = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_q = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_r = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_s = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_t = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_u = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_v = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_w = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_x = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_y = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_z = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_open_door = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_chat = read_int(&mut reader, encoding)?;
        ret.user_levels.agree_to_register = read_int(&mut reader, encoding)?;
        ret.user_levels.refuse_to_register = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_show_user_list = read_int(&mut reader, encoding)?;
        ret.user_levels.cmd_who = read_int(&mut reader, encoding)?;
        ret.user_levels.batch_file_transfer = read_int(&mut reader, encoding)?;
        //ret.user_levels.cmd_edit = read_int(&mut reader, encoding)?;

        ret.path.color_file = read_line(&mut reader, encoding)?;
        ret.modem.low_baud_limit = read_int(&mut reader, encoding)?;
        ret.modem.modem_delay = read_int(&mut reader, encoding)?;
        ret.modem.rings_require = read_int(&mut reader, encoding)?;
        ret.modem.irq_num = read_int(&mut reader, encoding)?;
        ret.modem.base_address = read_hex(&mut reader, encoding)?;
        ret.modem.leave_dtr_up = read_bool(&mut reader, encoding)?;
        ret.max_scroll_back = read_int(&mut reader, encoding)?;
        ret.fast_text = read_bool(&mut reader, encoding)?;
        ret.fast_cnames = read_bool(&mut reader, encoding)?;
        ret.sysop_security.subs = read_int(&mut reader, encoding)?;
        ret.upload_by = read_bool(&mut reader, encoding)?;
        ret.show_settings = read_bool(&mut reader, encoding)?;
        ret.allow_shell = read_bool(&mut reader, encoding)?;
        ret.slaves = read_bool(&mut reader, encoding)?;
        ret.subscription_info.subscription_length = read_int(&mut reader, encoding)?;

        ret.max_total_msgs = read_int(&mut reader, encoding)?;
        ret.max_conf_msgs = read_int(&mut reader, encoding)?;
        ret.min_prior_to_event = read_int(&mut reader, encoding)?;

        ret.sysop_security.edit_all = read_int(&mut reader, encoding)?;
        ret.user_levels.edit_own_messages = read_int(&mut reader, encoding)?;
        ret.modem.share_irqs = read_bool(&mut reader, encoding)?;
        ret.scan_all = read_bool(&mut reader, encoding)?;
        ret.sysop_security.read_only = read_int(&mut reader, encoding)?;
        ret.use_new_ask_file = read_bool(&mut reader, encoding)?;

        ret.path.tmp_loc = read_line(&mut reader, encoding)?;
        ret.path.inf_file = read_line(&mut reader, encoding)?;

        ret.quick_scan = read_bool(&mut reader, encoding)?;
        ret.subscription_info.warning_days = read_int(&mut reader, encoding)?;
        ret.allow_one_name = read_bool(&mut reader, encoding)?;
        ret.subscription_info.default_expired_level = read_int(&mut reader, encoding)? as u8;

        ret.user_levels.cmd_test_file = read_int(&mut reader, encoding)?;
        ret.cap_file = read_line(&mut reader, encoding)?;
        ret.test_uploads = read_bool(&mut reader, encoding)?;
        ret.stop_clock_on_cap = read_bool(&mut reader, encoding)?;
        ret.sysop_start = read_line(&mut reader, encoding)?;
        ret.sysop_stop = read_line(&mut reader, encoding)?;

        ret.log_caller_number = read_bool(&mut reader, encoding)?;
        ret.log_connect_str = read_bool(&mut reader, encoding)?;
        ret.log_sec_level = read_bool(&mut reader, encoding)?;
        ret.conf_pwrd_adjust = read_bool(&mut reader, encoding)?;
        ret.confirm_caller = read_bool(&mut reader, encoding)?;
        ret.allow_pwrd_comment = read_bool(&mut reader, encoding)?;
        ret.guard_logoff = read_bool(&mut reader, encoding)?;
        ret.num_desc_lines = read_int(&mut reader, encoding)?;

        // End of 14.5 files
        let Ok(chat_file) = read_line(&mut reader, encoding) else {
            return Ok(ret);
        };
        ret.path.chat_file = chat_file;
        ret.path.stats_file = read_line(&mut reader, encoding)?;
        ret.path.chat_menu = read_line(&mut reader, encoding)?;
        ret.path.no_ansi = read_line(&mut reader, encoding)?;
        ret.slow_drives = read_line(&mut reader, encoding)?;
        ret.path.swap_path = read_line(&mut reader, encoding)?;

        ret.swap = read_bool(&mut reader, encoding)?;
        ret.no_batch_up = read_bool(&mut reader, encoding)?;
        ret.force_main = read_bool(&mut reader, encoding)?;
        ret.foreign = read_bool(&mut reader, encoding)?;
        ret.monitor_modem = read_bool(&mut reader, encoding)?;
        ret.no16550 = read_bool(&mut reader, encoding)?;
        ret.force16550_a = read_bool(&mut reader, encoding)?;
        ret.auto_reset = read_bool(&mut reader, encoding)?;
        ret.float_node_number = read_bool(&mut reader, encoding)?;
        ret.promote_batch = read_bool(&mut reader, encoding)?;
        ret.auto_reg_conf = read_bool(&mut reader, encoding)?;
        ret.scan_blts = read_bool(&mut reader, encoding)?;
        ret.auto_make_msgs = read_bool(&mut reader, encoding)?;
        ret.verify_cdloss = read_bool(&mut reader, encoding)?;
        ret.encrypt = read_bool(&mut reader, encoding)?;
        ret.upload_credit = read_int(&mut reader, encoding)?;
        ret.byte_credit = read_int(&mut reader, encoding)?;

        ret.colors.default = read_int(&mut reader, encoding)?;
        ret.colors.msg_hdr_date = read_int(&mut reader, encoding)?;
        ret.colors.msg_hdr_to = read_int(&mut reader, encoding)?;
        ret.colors.msg_hdr_from = read_int(&mut reader, encoding)?;
        ret.colors.msg_hdr_subj = read_int(&mut reader, encoding)?;
        ret.colors.msg_hdr_read = read_int(&mut reader, encoding)?;
        ret.colors.msg_hdr_conf = read_int(&mut reader, encoding)?;
        ret.origin = read_line(&mut reader, encoding)?;
        ret.path.event_dat_file = read_line(&mut reader, encoding)?;
        ret.path.event_files = read_line(&mut reader, encoding)?;

        ret.path.cmd_lst = read_line(&mut reader, encoding)?;
        ret.sysop_info.require_pwrd_to_exit = read_bool(&mut reader, encoding)?;

        ret.sysop_security.sec_15 = read_int(&mut reader, encoding)?;
        ret.sysop_security.use_broadcast_command = read_int(&mut reader, encoding)?;
        ret.sysop_security.view_private_uploads = read_int(&mut reader, encoding)?;
        ret.sysop_security.enter_generic_message = read_int(&mut reader, encoding)?;
        ret.sysop_security.edit_message_headers = read_int(&mut reader, encoding)?;
        ret.sysop_security.protect_unprotect_messages = read_int(&mut reader, encoding)?;
        ret.sysop_security.overwrite_uploads = read_int(&mut reader, encoding)?;
        ret.sysop_security.unused0 = read_int(&mut reader, encoding)?;

        ret.env_size = read_int(&mut reader, encoding)?;
        ret.low_baud_sec_override = read_int(&mut reader, encoding)?;

        ret.path.all_files_list = read_line(&mut reader, encoding)?;
        ret.enable_fido = read_bool(&mut reader, encoding)?;
        ret.fido_config = read_line(&mut reader, encoding)?;
        ret.fido_queue = read_line(&mut reader, encoding)?;
        ret.prompt_to_read_mail = read_bool(&mut reader, encoding)?;
        ret.path.log_on_scr = read_line(&mut reader, encoding)?;
        ret.path.log_on_ans = read_line(&mut reader, encoding)?;
        ret.qwk_file = read_line(&mut reader, encoding)?;
        ret.path.file_tcan = read_line(&mut reader, encoding)?;

        ret.sysop_security.keep_msg = read_int(&mut reader, encoding)?;
        ret.sysop_security.seeretrcpt = read_int(&mut reader, encoding)?;

        ret.swap_during_bat = read_bool(&mut reader, encoding)?;
        ret.user_sys_during_bat = read_bool(&mut reader, encoding)?;
        ret.default_graphics = read_bool(&mut reader, encoding)?;

        ret.slow_drive_bat = read_line(&mut reader, encoding)?;

        ret.allow_alias_change = read_bool(&mut reader, encoding)?;
        ret.exclude_locals = read_bool(&mut reader, encoding)?;
        ret.pwrd_update = read_int(&mut reader, encoding)?;
        ret.pwrd_warn = read_int(&mut reader, encoding)?;
        ret.record_group_chat = read_bool(&mut reader, encoding)?;
        ret.show_alias = read_bool(&mut reader, encoding)?;
        ret.min_pwrd_len = read_int(&mut reader, encoding)?;
        ret.allow_handles = read_bool(&mut reader, encoding)?;

        ret.path.cmd_display_files_loc = read_line(&mut reader, encoding)?;
        ret.force_intro = read_bool(&mut reader, encoding)?;
        ret.skip_protocol = read_bool(&mut reader, encoding)?;
        ret.skip_alias = read_bool(&mut reader, encoding)?;
        ret.no_carrier_exit = read_bool(&mut reader, encoding)?;
        ret.os2_driver = read_bool(&mut reader, encoding)?;

        ret.enable_accounting = read_bool(&mut reader, encoding)?;
        ret.show_currency = read_bool(&mut reader, encoding)?;
        ret.concurrent = read_bool(&mut reader, encoding)?;

        ret.peak_start = read_line(&mut reader, encoding)?;
        ret.peak_end = read_line(&mut reader, encoding)?;

        ret.account_config = read_line(&mut reader, encoding)?;
        ret.account_info = read_line(&mut reader, encoding)?;
        ret.account_warn = read_line(&mut reader, encoding)?;
        ret.account_track = read_line(&mut reader, encoding)?;
        ret.account_logoff = read_line(&mut reader, encoding)?;

        ret.uucp_path = read_line(&mut reader, encoding)?;
        ret.uucp_spool_path = read_line(&mut reader, encoding)?;
        ret.uucp_log_path = read_line(&mut reader, encoding)?;
        ret.comp_bat_file = read_line(&mut reader, encoding)?;
        ret.de_comp_bat_file = read_line(&mut reader, encoding)?;
        ret.uucp_name = read_line(&mut reader, encoding)?;
        ret.uucp_domain_name = read_line(&mut reader, encoding)?;
        ret.uucp_email_host = read_line(&mut reader, encoding)?;
        ret.uucp_news_host = read_line(&mut reader, encoding)?;
        ret.uucp_def_dist = read_line(&mut reader, encoding)?;
        ret.uucp_time_zone = read_line(&mut reader, encoding)?;
        ret.uucp_separator = read_line(&mut reader, encoding)?;
        ret.organization = read_line(&mut reader, encoding)?;
        ret.uucp_high_ascii = read_line(&mut reader, encoding)?.chars().next().unwrap();
        ret.uucp_email_conf = read_int(&mut reader, encoding)?;
        ret.uucp_junk_conf = read_int(&mut reader, encoding)?;
        ret.uucp_bang_domain = read_bool(&mut reader, encoding)?;
        ret.uucp_sub_domain = read_bool(&mut reader, encoding)?;

        ret.fido_process_in = read_bool(&mut reader, encoding)?;
        ret.fido_process_orphan = read_bool(&mut reader, encoding)?;
        ret.fido_process_out = read_bool(&mut reader, encoding)?;
        ret.fido_dial_out = read_bool(&mut reader, encoding)?;
        ret.fido_pkt_freq = read_int(&mut reader, encoding)?;
        ret.fido_export_freq = read_int(&mut reader, encoding)?;
        ret.fido_mail_freq = read_int(&mut reader, encoding)?;
        ret.fido_default_zone = read_int(&mut reader, encoding)?;
        ret.fido_default_net = read_int(&mut reader, encoding)?;

        ret.peak_days = read_line(&mut reader, encoding)?; // TODO?

        ret.holidays_file = read_line(&mut reader, encoding)?;
        ret.uucp_mod_file = read_line(&mut reader, encoding)?;

        ret.modem.modem_init2 = read_line(&mut reader, encoding)?;
        ret.modem.modem_answer = read_line(&mut reader, encoding)?;
        ret.ignore_drop_sec_level = read_bool(&mut reader, encoding)?;
        ret.modem.modem_dial = read_line(&mut reader, encoding)?;
        ret.modem.num_redials = read_int(&mut reader, encoding)?;
        ret.modem.max_tries = read_int(&mut reader, encoding)?;

        ret.fido_import_after_xfer = read_bool(&mut reader, encoding)?;
        ret.fido_crash_sec = read_int(&mut reader, encoding)?;
        ret.fido_log_level = read_int(&mut reader, encoding)?;
        ret.fido_create_msg = read_bool(&mut reader, encoding)?;
        ret.fido_enable_routing = read_bool(&mut reader, encoding)?;

        ret.fido_loc = read_line(&mut reader, encoding)?;
        ret.fido_secure = read_bool(&mut reader, encoding)?;
        ret.fido_sysop_change = read_bool(&mut reader, encoding)?;
        ret.fido_check_dupe_path = read_bool(&mut reader, encoding)?;
        ret.fido_check_dupe_msg_id = read_bool(&mut reader, encoding)?;
        ret.fido_num_msgs_to_track = read_int(&mut reader, encoding)?;
        ret.fido_make_response = read_bool(&mut reader, encoding)?;
        ret.fido_enable_pass_thru = read_bool(&mut reader, encoding)?;
        ret.fido_enable_area_fix = read_bool(&mut reader, encoding)?;
        ret.fido_auto_add = read_bool(&mut reader, encoding)?;
        ret.fido_re_address = read_bool(&mut reader, encoding)?;
        ret.fido_route_echo_mail = read_bool(&mut reader, encoding)?;

        ret.minimize_log_on_off = read_int(&mut reader, encoding)?;
        ret.minimize_protocols = read_int(&mut reader, encoding)?;
        ret.minimize_doors = read_int(&mut reader, encoding)?;
        ret.minimize_file_test = read_int(&mut reader, encoding)?;
        ret.minimize_file_view = read_int(&mut reader, encoding)?;
        ret.minimize_compress = read_int(&mut reader, encoding)?;
        ret.minimize_shells = read_int(&mut reader, encoding)?;

        ret.priority_normal = read_int(&mut reader, encoding)?;
        ret.priority_protocols = read_int(&mut reader, encoding)?;
        ret.priority_compress = read_int(&mut reader, encoding)?;
        ret.priority_shells = read_int(&mut reader, encoding)?;
        ret.priority_fido_in = read_int(&mut reader, encoding)?;
        ret.priority_fido_out = read_int(&mut reader, encoding)?;

        ret.net_copy = read_line(&mut reader, encoding)?;

        Ok(ret)
    }

    pub fn serialize(&self, encoding: Encoding) -> Vec<u8> {
        let mut res = Vec::new();
        append_line(
            &mut res,
            encoding,
            "*** PCBoard Version 14.5 & 15.0 data file ***",
        );
        append_line(&mut res, encoding, &self.sysop_info.sysop);
        append_line(&mut res, encoding, &self.sysop_info.password);
        append_bool(&mut res, encoding, self.sysop_info.use_real_name);
        append_bool(&mut res, encoding, self.sysop_info.use_local_graphics);

        append_int(&mut res, encoding, self.sysop_security.read_all_comments);
        append_int(&mut res, encoding, self.sysop_security.read_all_mail);
        append_int(&mut res, encoding, self.sysop_security.sysop);
        append_int(&mut res, encoding, self.sysop_security.copy_move_messages);

        append_int(
            &mut res,
            encoding,
            self.sysop_security.sec_1_view_caller_log,
        );
        append_int(&mut res, encoding, self.sysop_security.sec_2_view_usr_list);
        append_int(
            &mut res,
            encoding,
            self.sysop_security.sec_3_pack_renumber_msg,
        );
        append_int(
            &mut res,
            encoding,
            self.sysop_security.sec_4_recover_deleted_msg,
        );
        append_int(
            &mut res,
            encoding,
            self.sysop_security.sec_5_list_message_hdr,
        );
        append_int(&mut res, encoding, self.sysop_security.sec_6_view_any_file);
        append_int(&mut res, encoding, self.sysop_security.sec_7_user_maint);
        append_int(&mut res, encoding, self.sysop_security.sec_8_pack_usr_file);
        append_int(&mut res, encoding, self.sysop_security.sec_9_exit_to_dos);
        append_int(
            &mut res,
            encoding,
            self.sysop_security.sec_10_shelled_dos_func,
        );
        append_int(
            &mut res,
            encoding,
            self.sysop_security.sec_11_view_other_nodes,
        );
        append_int(
            &mut res,
            encoding,
            self.sysop_security.sec_12_logoff_alt_node,
        );
        append_int(
            &mut res,
            encoding,
            self.sysop_security.sec_13_drop_alt_node_to_dos,
        );
        append_int(&mut res, encoding, self.sysop_security.sec_14_drop_to_dos);

        append_line(&mut res, encoding, &self.path.help_loc);
        append_line(&mut res, encoding, &self.path.sec_loc);
        append_line(&mut res, encoding, &self.path.chat_loc);
        append_line(&mut res, encoding, &self.path.text_loc);
        append_line(&mut res, encoding, &self.path.index_loc);
        append_line(&mut res, encoding, &self.path.usr_file);
        append_line(&mut res, encoding, &self.path.clr_file);
        append_line(&mut res, encoding, &self.path.conference_file);
        append_line(&mut res, encoding, &self.path.pwd_file);
        append_line(&mut res, encoding, &self.path.fsec_file);
        append_line(&mut res, encoding, &self.path.upsec_file);
        append_line(&mut res, encoding, &self.path.tcan_file);
        append_line(&mut res, encoding, &self.path.welcome_file);
        append_line(&mut res, encoding, &self.path.newuser_file);
        append_line(&mut res, encoding, &self.path.closed_file);
        append_line(&mut res, encoding, &self.path.warning_file);
        append_line(&mut res, encoding, &self.path.expired_file);
        append_line(&mut res, encoding, &self.path.usernet_file);
        append_line(&mut res, encoding, &self.path.conf_menu);
        append_line(&mut res, encoding, &self.path.newreg_file);
        append_line(&mut res, encoding, &self.path.answer_file);
        append_line(&mut res, encoding, &self.path.protocol_data_file);
        append_line(&mut res, encoding, &self.path.download_file);
        append_line(&mut res, encoding, &self.path.logoff_script);
        append_line(&mut res, encoding, &self.path.logoff_answer);
        append_line(&mut res, encoding, &self.path.pcml_dat_file);
        append_line(&mut res, encoding, &self.path.group_chat);

        append_int(&mut res, encoding, self.modem.seconds);
        append_line(&mut res, encoding, &self.modem.modem_serial_device);
        append_int(&mut res, encoding, self.modem.modem_speed);
        append_bool(&mut res, encoding, self.modem.lock_speed);
        append_line(&mut res, encoding, &self.modem.modem_init);
        append_line(&mut res, encoding, &self.modem.modem_off_hook);
        append_bool(&mut res, encoding, self.modem.disable_cts);
        append_bool(&mut res, encoding, self.modem.use_hayes_9600);
        append_bool(&mut res, encoding, self.modem.use_fast_comm);
        append_bool(&mut res, encoding, self.modem.reset_modem);
        append_bool(&mut res, encoding, self.modem.take_modem_off_hook);
        append_bool(&mut res, encoding, self.modem.packed);
        append_bool(&mut res, encoding, self.modem.answer_ring);
        append_bool(&mut res, encoding, self.modem.allow_7e1);
        append_bool(&mut res, encoding, self.modem.allow_low_baud);
        append_line(&mut res, encoding, &self.modem.allow_low_start_time);
        append_line(&mut res, encoding, &self.modem.allow_low_stop_time);

        append_bool(&mut res, encoding, self.disable_scan);
        append_bool(&mut res, encoding, self.disable_cls);
        append_bool(&mut res, encoding, self.disable_edits);
        append_bool(&mut res, encoding, self.disable_filter);
        append_bool(&mut res, encoding, self.disable_quick);
        append_bool(&mut res, encoding, self.disable_password);
        append_bool(&mut res, encoding, self.multi_lingual);
        append_bool(&mut res, encoding, self.allow_pwrd_only);
        append_bool(&mut res, encoding, self.closed_board);
        append_bool(&mut res, encoding, self.non_graphics);
        append_bool(&mut res, encoding, self.exit_to_dos);
        append_bool(&mut res, encoding, self.include_city);
        append_bool(&mut res, encoding, self.eliminate_snow);
        append_bool(&mut res, encoding, self.subscription_info.is_enabled);
        append_bool(&mut res, encoding, self.allow_esc_codes);
        append_bool(&mut res, encoding, self.allow_ccs);
        append_bool(&mut res, encoding, self.validate_to);
        append_bool(&mut res, encoding, self.last_read_update);
        append_bool(&mut res, encoding, self.enforce_time);
        append_line(&mut res, encoding, &self.display_news.to_string());
        append_bool(&mut res, encoding, self.disable_ctsdrop);
        append_int(&mut res, encoding, self.kbd_timeout);
        append_bool(&mut res, encoding, self.network);
        append_int(&mut res, encoding, self.node_num);
        append_int(&mut res, encoding, self.net_timeout);
        append_int(&mut res, encoding, self.chat_delay);

        append_line(&mut res, encoding, &self.board_name);
        append_line(&mut res, encoding, &self.view_batch);
        append_line(&mut res, encoding, &self.view_ext);
        for i in 0..10 {
            append_line(&mut res, encoding, &self.func_keys[i]);
        }
        append_line(&mut res, encoding, &self.pub_conf);
        append_int(&mut res, encoding, self.num_conf);
        append_int(&mut res, encoding, self.max_msg_lines);
        append_int(&mut res, encoding, self.default_color);
        append_int(&mut res, encoding, self.default_intensity);
        append_bool(&mut res, encoding, self.event_active);
        append_line(&mut res, encoding, &self.event_time);
        append_int(&mut res, encoding, self.event_suspend);
        append_bool(&mut res, encoding, self.event_stop_uplds);
        append_bool(&mut res, encoding, self.event_slide);
        append_int(&mut res, encoding, self.upload_buf_size);
        append_bool(&mut res, encoding, self.disable_drive_check);
        append_int(&mut res, encoding, self.parallel_port_num);

        append_int(&mut res, encoding, self.stop_free_space);

        append_int(&mut res, encoding, self.user_levels.cmd_a);
        append_int(&mut res, encoding, self.user_levels.cmd_b);
        append_int(&mut res, encoding, self.user_levels.cmd_c);
        append_int(&mut res, encoding, self.user_levels.cmd_d);
        append_int(&mut res, encoding, self.user_levels.cmd_e);
        append_int(&mut res, encoding, self.user_levels.cmd_f);
        append_int(&mut res, encoding, self.user_levels.cmd_g);
        append_int(&mut res, encoding, self.user_levels.cmd_h);
        append_int(&mut res, encoding, self.user_levels.cmd_i);
        append_int(&mut res, encoding, self.user_levels.cmd_j);
        append_int(&mut res, encoding, self.user_levels.cmd_k);
        append_int(&mut res, encoding, self.user_levels.cmd_l);
        append_int(&mut res, encoding, self.user_levels.cmd_m);
        append_int(&mut res, encoding, self.user_levels.cmd_n);
        append_int(&mut res, encoding, self.user_levels.cmd_o);
        append_int(&mut res, encoding, self.user_levels.cmd_p);
        append_int(&mut res, encoding, self.user_levels.cmd_q);
        append_int(&mut res, encoding, self.user_levels.cmd_r);
        append_int(&mut res, encoding, self.user_levels.cmd_s);
        append_int(&mut res, encoding, self.user_levels.cmd_t);
        append_int(&mut res, encoding, self.user_levels.cmd_u);
        append_int(&mut res, encoding, self.user_levels.cmd_v);
        append_int(&mut res, encoding, self.user_levels.cmd_w);
        append_int(&mut res, encoding, self.user_levels.cmd_x);
        append_int(&mut res, encoding, self.user_levels.cmd_y);
        append_int(&mut res, encoding, self.user_levels.cmd_z);

        append_int(&mut res, encoding, self.user_levels.cmd_open_door);
        append_int(&mut res, encoding, self.user_levels.cmd_chat);
        append_int(&mut res, encoding, self.user_levels.agree_to_register);
        append_int(&mut res, encoding, self.user_levels.refuse_to_register);
        append_int(&mut res, encoding, self.user_levels.cmd_show_user_list);
        append_int(&mut res, encoding, self.user_levels.cmd_who);
        append_int(&mut res, encoding, self.user_levels.batch_file_transfer);
        //append_int(&mut res, encoding, self.user_levels.cmd_edit);

        append_line(&mut res, encoding, &self.path.color_file);
        append_int(&mut res, encoding, self.modem.low_baud_limit);
        append_int(&mut res, encoding, self.modem.modem_delay);
        append_int(&mut res, encoding, self.modem.rings_require);
        append_int(&mut res, encoding, self.modem.irq_num);
        append_hex(&mut res, encoding, self.modem.base_address);
        append_bool(&mut res, encoding, self.modem.leave_dtr_up);
        append_int(&mut res, encoding, self.max_scroll_back);
        append_bool(&mut res, encoding, self.fast_text);
        append_bool(&mut res, encoding, self.fast_cnames);
        append_int(&mut res, encoding, self.sysop_security.subs);
        append_bool(&mut res, encoding, self.upload_by);
        append_bool(&mut res, encoding, self.show_settings);
        append_bool(&mut res, encoding, self.allow_shell);

        append_bool(&mut res, encoding, self.slaves);
        append_int(
            &mut res,
            encoding,
            self.subscription_info.subscription_length,
        );
        append_int(&mut res, encoding, self.max_total_msgs);
        append_int(&mut res, encoding, self.max_conf_msgs);
        append_int(&mut res, encoding, self.min_prior_to_event);
        append_int(&mut res, encoding, self.sysop_security.edit_all);
        append_int(&mut res, encoding, self.user_levels.edit_own_messages);
        append_bool(&mut res, encoding, self.modem.share_irqs);
        append_bool(&mut res, encoding, self.scan_all);
        append_int(&mut res, encoding, self.sysop_security.read_only);
        append_bool(&mut res, encoding, self.use_new_ask_file);

        append_line(&mut res, encoding, &self.path.tmp_loc);
        append_line(&mut res, encoding, &self.path.inf_file);
        append_bool(&mut res, encoding, self.quick_scan);
        append_int(&mut res, encoding, self.subscription_info.warning_days);
        append_bool(&mut res, encoding, self.allow_one_name);
        append_int(
            &mut res,
            encoding,
            self.subscription_info.default_expired_level as i32,
        );
        append_int(&mut res, encoding, self.user_levels.cmd_test_file);
        append_line(&mut res, encoding, &self.cap_file);
        append_bool(&mut res, encoding, self.test_uploads);
        append_bool(&mut res, encoding, self.stop_clock_on_cap);
        append_line(&mut res, encoding, &self.sysop_start);
        append_line(&mut res, encoding, &self.sysop_stop);

        append_bool(&mut res, encoding, self.log_caller_number);
        append_bool(&mut res, encoding, self.log_connect_str);
        append_bool(&mut res, encoding, self.log_sec_level);
        append_bool(&mut res, encoding, self.conf_pwrd_adjust);
        append_bool(&mut res, encoding, self.confirm_caller);
        append_bool(&mut res, encoding, self.allow_pwrd_comment);
        append_bool(&mut res, encoding, self.guard_logoff);
        append_int(&mut res, encoding, self.num_desc_lines);

        append_line(&mut res, encoding, &self.path.chat_file);
        append_line(&mut res, encoding, &self.path.stats_file);
        append_line(&mut res, encoding, &self.path.chat_menu);
        append_line(&mut res, encoding, &self.path.no_ansi);
        append_line(&mut res, encoding, &self.slow_drives);
        append_line(&mut res, encoding, &self.path.swap_path);

        append_bool(&mut res, encoding, self.swap);
        append_bool(&mut res, encoding, self.no_batch_up);
        append_bool(&mut res, encoding, self.force_main);
        append_bool(&mut res, encoding, self.foreign);
        append_bool(&mut res, encoding, self.monitor_modem);
        append_bool(&mut res, encoding, self.no16550);
        append_bool(&mut res, encoding, self.force16550_a);
        append_bool(&mut res, encoding, self.auto_reset);
        append_bool(&mut res, encoding, self.float_node_number);
        append_bool(&mut res, encoding, self.promote_batch);
        append_bool(&mut res, encoding, self.auto_reg_conf);
        append_bool(&mut res, encoding, self.scan_blts);
        append_bool(&mut res, encoding, self.auto_make_msgs);
        append_bool(&mut res, encoding, self.verify_cdloss);
        append_bool(&mut res, encoding, self.encrypt);
        append_int(&mut res, encoding, self.upload_credit);
        append_int(&mut res, encoding, self.byte_credit);

        append_int(&mut res, encoding, self.colors.default);
        append_int(&mut res, encoding, self.colors.msg_hdr_date);
        append_int(&mut res, encoding, self.colors.msg_hdr_to);
        append_int(&mut res, encoding, self.colors.msg_hdr_from);
        append_int(&mut res, encoding, self.colors.msg_hdr_subj);
        append_int(&mut res, encoding, self.colors.msg_hdr_read);
        append_int(&mut res, encoding, self.colors.msg_hdr_conf);
        append_line(&mut res, encoding, &self.origin);
        append_line(&mut res, encoding, &self.path.event_dat_file);
        append_line(&mut res, encoding, &self.path.event_files);

        append_line(&mut res, encoding, &self.path.cmd_lst);
        append_bool(&mut res, encoding, self.sysop_info.require_pwrd_to_exit);

        append_int(&mut res, encoding, self.sysop_security.sec_15);
        append_int(
            &mut res,
            encoding,
            self.sysop_security.use_broadcast_command,
        );
        append_int(&mut res, encoding, self.sysop_security.view_private_uploads);
        append_int(
            &mut res,
            encoding,
            self.sysop_security.enter_generic_message,
        );
        append_int(&mut res, encoding, self.sysop_security.edit_message_headers);
        append_int(
            &mut res,
            encoding,
            self.sysop_security.protect_unprotect_messages,
        );
        append_int(&mut res, encoding, self.sysop_security.overwrite_uploads);
        append_int(&mut res, encoding, self.sysop_security.unused0);

        append_int(&mut res, encoding, self.env_size);
        append_int(&mut res, encoding, self.low_baud_sec_override);

        append_line(&mut res, encoding, &self.path.all_files_list);
        append_bool(&mut res, encoding, self.enable_fido);
        append_line(&mut res, encoding, &self.fido_config);
        append_line(&mut res, encoding, &self.fido_queue);
        append_bool(&mut res, encoding, self.prompt_to_read_mail);
        append_line(&mut res, encoding, &self.path.log_on_scr);
        append_line(&mut res, encoding, &self.path.log_on_ans);
        append_line(&mut res, encoding, &self.qwk_file);
        append_line(&mut res, encoding, &self.path.file_tcan);

        append_int(&mut res, encoding, self.sysop_security.keep_msg);
        append_int(&mut res, encoding, self.sysop_security.seeretrcpt);
        append_bool(&mut res, encoding, self.swap_during_bat);
        append_bool(&mut res, encoding, self.user_sys_during_bat);
        append_bool(&mut res, encoding, self.default_graphics);
        append_line(&mut res, encoding, &self.slow_drive_bat);
        append_bool(&mut res, encoding, self.allow_alias_change);
        append_bool(&mut res, encoding, self.exclude_locals);
        append_int(&mut res, encoding, self.pwrd_update);
        append_int(&mut res, encoding, self.pwrd_warn);
        append_bool(&mut res, encoding, self.record_group_chat);
        append_bool(&mut res, encoding, self.show_alias);
        append_int(&mut res, encoding, self.min_pwrd_len);
        append_bool(&mut res, encoding, self.allow_handles);

        append_line(&mut res, encoding, &self.path.cmd_display_files_loc);
        append_bool(&mut res, encoding, self.force_intro);
        append_bool(&mut res, encoding, self.skip_protocol);
        append_bool(&mut res, encoding, self.skip_alias);
        append_bool(&mut res, encoding, self.no_carrier_exit);
        append_bool(&mut res, encoding, self.os2_driver);

        append_bool(&mut res, encoding, self.enable_accounting);
        append_bool(&mut res, encoding, self.show_currency);
        append_bool(&mut res, encoding, self.concurrent);

        append_line(&mut res, encoding, &self.peak_start);
        append_line(&mut res, encoding, &self.peak_end);

        append_line(&mut res, encoding, &self.account_config);
        append_line(&mut res, encoding, &self.account_info);
        append_line(&mut res, encoding, &self.account_warn);
        append_line(&mut res, encoding, &self.account_track);
        append_line(&mut res, encoding, &self.account_logoff);

        append_line(&mut res, encoding, &self.uucp_path);
        append_line(&mut res, encoding, &self.uucp_spool_path);
        append_line(&mut res, encoding, &self.uucp_log_path);
        append_line(&mut res, encoding, &self.comp_bat_file);
        append_line(&mut res, encoding, &self.de_comp_bat_file);
        append_line(&mut res, encoding, &self.uucp_name);
        append_line(&mut res, encoding, &self.uucp_domain_name);
        append_line(&mut res, encoding, &self.uucp_email_host);
        append_line(&mut res, encoding, &self.uucp_news_host);
        append_line(&mut res, encoding, &self.uucp_def_dist);
        append_line(&mut res, encoding, &self.uucp_time_zone);
        append_line(&mut res, encoding, &self.uucp_separator);
        append_line(&mut res, encoding, &self.organization);
        append_line(&mut res, encoding, &self.uucp_high_ascii.to_string());
        append_int(&mut res, encoding, self.uucp_email_conf);
        append_int(&mut res, encoding, self.uucp_junk_conf);
        append_bool(&mut res, encoding, self.uucp_bang_domain);
        append_bool(&mut res, encoding, self.uucp_sub_domain);

        append_bool(&mut res, encoding, self.fido_process_in);
        append_bool(&mut res, encoding, self.fido_process_orphan);
        append_bool(&mut res, encoding, self.fido_process_out);
        append_bool(&mut res, encoding, self.fido_dial_out);
        append_int(&mut res, encoding, self.fido_pkt_freq);
        append_int(&mut res, encoding, self.fido_export_freq);
        append_int(&mut res, encoding, self.fido_mail_freq);
        append_int(&mut res, encoding, self.fido_default_zone);
        append_int(&mut res, encoding, self.fido_default_net);

        append_line(&mut res, encoding, &self.peak_days);
        append_line(&mut res, encoding, &self.holidays_file);
        append_line(&mut res, encoding, &self.uucp_mod_file);

        append_line(&mut res, encoding, &self.modem.modem_init2);
        append_line(&mut res, encoding, &self.modem.modem_answer);
        append_bool(&mut res, encoding, self.ignore_drop_sec_level);
        append_line(&mut res, encoding, &self.modem.modem_dial);
        append_int(&mut res, encoding, self.modem.num_redials);
        append_int(&mut res, encoding, self.modem.max_tries);

        append_bool(&mut res, encoding, self.fido_import_after_xfer);
        append_int(&mut res, encoding, self.fido_crash_sec);
        append_int(&mut res, encoding, self.fido_log_level);
        append_bool(&mut res, encoding, self.fido_create_msg);
        append_bool(&mut res, encoding, self.fido_enable_routing);

        append_line(&mut res, encoding, &self.fido_loc);
        append_bool(&mut res, encoding, self.fido_secure);
        append_bool(&mut res, encoding, self.fido_sysop_change);
        append_bool(&mut res, encoding, self.fido_check_dupe_path);
        append_bool(&mut res, encoding, self.fido_check_dupe_msg_id);
        append_int(&mut res, encoding, self.fido_num_msgs_to_track);
        append_bool(&mut res, encoding, self.fido_make_response);
        append_bool(&mut res, encoding, self.fido_enable_pass_thru);
        append_bool(&mut res, encoding, self.fido_enable_area_fix);
        append_bool(&mut res, encoding, self.fido_auto_add);
        append_bool(&mut res, encoding, self.fido_re_address);
        append_bool(&mut res, encoding, self.fido_route_echo_mail);

        append_int(&mut res, encoding, self.minimize_log_on_off);
        append_int(&mut res, encoding, self.minimize_protocols);
        append_int(&mut res, encoding, self.minimize_doors);
        append_int(&mut res, encoding, self.minimize_file_test);
        append_int(&mut res, encoding, self.minimize_file_view);
        append_int(&mut res, encoding, self.minimize_compress);
        append_int(&mut res, encoding, self.minimize_shells);

        append_int(&mut res, encoding, self.priority_normal);
        append_int(&mut res, encoding, self.priority_protocols);
        append_int(&mut res, encoding, self.priority_compress);
        append_int(&mut res, encoding, self.priority_shells);
        append_int(&mut res, encoding, self.priority_fido_in);
        append_int(&mut res, encoding, self.priority_fido_out);

        append_line(&mut res, encoding, &self.net_copy);

        res
    }
}

fn _convert_path(c_drive: &str, path: &str) -> String {
    let mut path = path;
    let mut res = if path.starts_with("C:") {
        path = &path[2..];
        c_drive.to_string()
    } else {
        String::new()
    };

    for c in path.chars() {
        if c == '\\' {
            res.push('/');
        } else {
            res.push(c);
        }
    }
    res
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub status: char,
    pub mail_waiting: bool,
    pub pager: u32,
    pub name: String,
    pub city: String,
    pub operation: String,
    pub message: String,
    pub channel: u8,
    pub last_update: String,
}
