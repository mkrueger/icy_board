use std::sync::Arc;
use std::sync::Mutex;

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::user_base::Password;
use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::config_menu::EditMode;
use icy_board_tui::config_menu::Value;
use icy_board_tui::tab_page::TabPage;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    theme::THEME,
};
use ratatui::text::Text;
use ratatui::{
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Padding, Widget},
    Frame,
};

pub struct GeneralTab {
    pub state: ConfigMenuState,

    config: ConfigMenu,
    icy_board: Arc<Mutex<IcyBoard>>,
}

impl GeneralTab {
    pub fn new(lock: Arc<Mutex<IcyBoard>>) -> Self {
        let icy_board = lock.lock().unwrap();
        let sysop_info_width = 16;
        let sysop_info = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "sysop_name",
                    "Sysop's Name".to_string(),
                    ListValue::Text(25, icy_board.config.sysop.name.clone()),
                )
                .with_status("Enter the first name of the sysop.")
                .with_label_width(sysop_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "local_pass",
                    "Local Password".to_string(),
                    ListValue::Text(25, icy_board.config.sysop.password.to_string().clone()),
                )
                .with_status("Call waiting screen password.")
                .with_label_width(sysop_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "local_pass_exit",
                    "Require Password to Exit".to_string(),
                    ListValue::Bool(icy_board.config.sysop.require_password_to_exit),
                )
                .with_status("IcyBoard requires pw to exit the call waiting screen."),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "use_real_name",
                    "Use Real Name".to_string(),
                    ListValue::Bool(icy_board.config.sysop.use_real_name),
                )
                .with_status("Message to sysop with real name?"),
            ),
        ];

        let board_info_width = 13;

        let board_info = vec![
            ConfigEntry::Item(
                ListItem::new("board_name", "Board Name".to_string(), ListValue::Text(54, icy_board.config.board.name.clone()))
                    .with_status("Board name is shown on login to the caller.")
                    .with_label_width(board_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new("location", "Location".to_string(), ListValue::Text(54, icy_board.config.board.location.clone()))
                    .with_status("Board location used in IEMSI")
                    .with_label_width(board_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new("operator", "Operator".to_string(), ListValue::Text(30, icy_board.config.board.operator.clone()))
                    .with_status("Board operator used in IEMSI")
                    .with_label_width(board_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new("notice", "Notice".to_string(), ListValue::Text(30, icy_board.config.board.notice.clone()))
                    .with_status("Board notice used in IEMSI")
                    .with_label_width(board_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "capabilities",
                    "Capabilities".to_string(),
                    ListValue::Text(30, icy_board.config.board.capabilities.clone()),
                )
                .with_status("Board capabilities used in IEMSI")
                .with_label_width(board_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "date_format",
                    "Date Format".to_string(),
                    ListValue::ValueList(
                        icy_board.config.board.date_format.clone(),
                        vec![
                            Value::new("MM/DD/YY", "%m/%d/%y"),
                            Value::new("DD/MM/YY", "%d/%m/%y"),
                            Value::new("YY/MM/DD", "%y/%m/%d"),
                            Value::new("MM.DD.YY", "%m.%d.%y"),
                            Value::new("DD.MM.YY", "%d.%m.%y"),
                            Value::new("YY.MM.DD", "%y.%m.%d"),
                            Value::new("MM-DD-YY", "%m-%d-%y"),
                            Value::new("DD-MM-YY", "%d-%m-%y"),
                            Value::new("YY-MM-DD", "%y-%m-%d"),
                        ],
                    ),
                )
                .with_status("Default date format")
                .with_label_width(board_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "num_nodes",
                    "# Nodes".to_string(),
                    ListValue::U32(icy_board.config.board.num_nodes as u32, 1, 256),
                )
                .with_status("Numer of active nodes")
                .with_label_width(board_info_width),
            ),
        ];
        let new_user_info_width = 14;

        let new_user_info = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "sec_level",
                    "Security Level".to_string(),
                    ListValue::U32(icy_board.config.new_user_settings.sec_level as u32, 0, 255),
                )
                .with_label_width(new_user_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "allow_one_name_users",
                    "Allow One Name Users".to_string(),
                    ListValue::Bool(icy_board.config.new_user_settings.allow_one_name_users),
                )
                .with_label_width(new_user_info_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "new_user_groups",
                    "Groups".to_string(),
                    ListValue::Text(40, icy_board.config.new_user_settings.new_user_groups.clone()),
                )
                .with_label_width(new_user_info_width),
            ),
            ConfigEntry::Table(
                2,
                vec![
                    ConfigEntry::Item(ListItem::new(
                        "ask_city_or_state",
                        "City or State".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_city_or_state),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_address",
                        "Address".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_address),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_verification",
                        "Verification".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_verification),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_bus_data_phone",
                        "Bus Phone".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_bus_data_phone),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_voice_phone",
                        "Home Phone".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_voice_phone),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_comment",
                        "Comment".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_comment),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_clr_msg",
                        "MsgClear".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_clr_msg),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_fse",
                        "Full Screen Editor".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_fse),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_xfer_protocol",
                        "Protocols".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_xfer_protocol),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_date_format",
                        "Date Format".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_date_format),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_alias",
                        "Alias".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_alias),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_gender",
                        "Gender".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_gender),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_birthdate",
                        "Birthdate".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_birthdate),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_email",
                        "Email".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_email),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_web_address",
                        "Web Address".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_web_address),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "ask_use_short_descr",
                        "Short File Descr".to_string(),
                        ListValue::Bool(icy_board.config.new_user_settings.ask_use_short_descr),
                    )),
                    ConfigEntry::Item(ListItem::new(
                        "allow_iemsi",
                        "Allow IEMSI logins".to_string(),
                        ListValue::Bool(icy_board.config.options.allow_iemsi),
                    )),
                ],
            ),
        ];

        let function_keys_width = 10;
        let mut function_keys = Vec::new();
        for i in 0..10 {
            let key = format!("f{}", i + 1);
            function_keys.push(ConfigEntry::Item(
                ListItem::new(
                    &key,
                    format!("F-Key #{}", i + 1),
                    ListValue::Text(50, icy_board.config.func_keys[i].to_string()),
                )
                .with_label_width(function_keys_width),
            ));
        }

        let message_width = 17;
        let message = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "max_lines",
                    "Max Message".to_string(),
                    ListValue::U32(icy_board.config.options.max_msg_lines as u32, 17, 400),
                )
                .with_status("Maximum Lines in the Message Editor (17-400).")
                .with_label_width(message_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "scan_all_mail_at_login",
                    "Scan ALL at Login".to_string(),
                    ListValue::Bool(icy_board.config.options.scan_all_mail_at_login),
                )
                .with_status("Default to Scan ALL Conferences at Login")
                .with_label_width(message_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "prompt_to_read_mail",
                    "Read Mail Prompt".to_string(),
                    ListValue::Bool(icy_board.config.options.prompt_to_read_mail),
                )
                .with_status("Prompt to Read Mail w hen Mail Waiting")
                .with_label_width(message_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "msg_hdr_date",
                    "Header DATE Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_date.clone()),
                )
                .with_status("Color for Message Header DATE Line")
                .with_label_width(message_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "msg_hdr_to",
                    "Header TO Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_to.clone()),
                )
                .with_status("Color for Message Header TO Line")
                .with_label_width(message_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "msg_hdr_from",
                    "Header FROM Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_from.clone()),
                )
                .with_status("Color for Message Header FROM Line")
                .with_label_width(message_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "msg_hdr_subj",
                    "Header SUBJ Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_subj.clone()),
                )
                .with_status("Color for Message Header SUBJ Line")
                .with_label_width(message_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "msg_hdr_read",
                    "Header READ Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_read.clone()),
                )
                .with_status("Color for Message Header READ Line")
                .with_label_width(message_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "msg_hdr_conf",
                    "Header CONF Line".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.msg_hdr_conf.clone()),
                )
                .with_status("Color for Message Header CONF Line")
                .with_label_width(message_width),
            ),
        ];

        let file_transfer_width = 18;
        let file_transfer = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "check_files_uploaded",
                    "Verify File Uploads".to_string(),
                    ListValue::Bool(icy_board.config.options.check_files_uploaded),
                )
                .with_status("Verify/Test uploaded files")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "display_uploader",
                    "Show 'Uploaded By'".to_string(),
                    ListValue::Bool(icy_board.config.options.display_uploader),
                )
                .with_status("Include 'Uploaded By' in Description")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "upload_descr_lines",
                    "Max UL descr Lines".to_string(),
                    ListValue::U32(icy_board.config.options.upload_descr_lines as u32, 1, 60),
                )
                .with_status("Maximum Number of Lines in Upload Description")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "file_head",
                    "File HEAD Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_head.clone()),
                )
                .with_status("Color for File HEAD")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "file_name",
                    "File NAME Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_name.clone()),
                )
                .with_status("Color for File NAME")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "file_size",
                    "File Size Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_size.clone()),
                )
                .with_status("Color for Size of Files")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "file_date",
                    "File DATE Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_date.clone()),
                )
                .with_status("Color for File DATE")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "file_description",
                    "File DESCR1 Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_description.clone()),
                )
                .with_status("Color for File first line of Description")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "file_description_low",
                    "File DESCR2 Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_description_low.clone()),
                )
                .with_status("Color for File Description")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "file_text",
                    "File Text Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_text.clone()),
                )
                .with_status("Color for Text in Files")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "file_deleted",
                    "File Deleted Color".to_string(),
                    ListValue::Color(icy_board.config.color_configuration.file_deleted.clone()),
                )
                .with_status("Color for 'Deleted' in Files")
                .with_label_width(file_transfer_width),
            ),
        ];
        let switches = vec![
            ConfigEntry::Item(
                ListItem::new(
                    "keyboard_timeout",
                    "Keyboard Timeout".to_string(),
                    ListValue::U32(icy_board.config.options.keyboard_timeout as u32, 0, 255),
                )
                .with_status("Keyboard Timeout (in min, 0=off)")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "exclude_local_calls",
                    "Exclude Local".to_string(),
                    ListValue::Bool(icy_board.config.options.exclude_local_calls),
                )
                .with_status("Exclude Local Logins from Statistics")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "disable_full_record_updating",
                    "Disable Fill Record Updating".to_string(),
                    ListValue::Bool(icy_board.config.options.disable_full_record_updating),
                )
                .with_status("Setting 'Y' will only allow pwd change in 'W' command.")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "is_closed_board",
                    "Run System as a Closed Board".to_string(),
                    ListValue::Bool(icy_board.config.options.disable_full_record_updating),
                )
                .with_status("Deny new users (NEWASK Survey works).")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "display_userinfo_at_login",
                    "Display User Info at Login".to_string(),
                    ListValue::Bool(icy_board.config.options.disable_full_record_updating),
                )
                .with_status("Display 'v' command at login.")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(
                ListItem::new(
                    "non_graphics",
                    "Non Graphics BBS".to_string(),
                    ListValue::Bool(icy_board.config.options.disable_full_record_updating),
                )
                .with_status("Disable Colors.")
                .with_label_width(file_transfer_width),
            ),
            ConfigEntry::Item(ListItem::new(
                "give_user_password_to_doors",
                "Doors get Plaintext Passwords".to_string(),
                ListValue::Bool(icy_board.config.options.give_user_password_to_doors),
            )),
        ];

        Self {
            state: ConfigMenuState::default(),
            icy_board: lock.clone(),
            config: ConfigMenu {
                entry: vec![
                    ConfigEntry::Group("Sysop Information".to_string(), sysop_info),
                    ConfigEntry::Group("Board Information".to_string(), board_info),
                    ConfigEntry::Group("New User Settings".to_string(), new_user_info),
                    ConfigEntry::Group("Function Keys".to_string(), function_keys),
                    ConfigEntry::Group("Messages".to_string(), message),
                    ConfigEntry::Group("File Transfer".to_string(), file_transfer),
                    ConfigEntry::Group("Switches".to_string(), switches),
                ],
            },
        }
    }

    fn write_item(&self, item: &ListItem, icy_board: &mut IcyBoard) {
        match &item.value {
            ListValue::Text(_, text) => match item.id.as_str() {
                "board_name" => icy_board.config.board.name = text.clone(),
                "location" => icy_board.config.board.location = text.clone(),
                "operator" => icy_board.config.board.operator = text.clone(),
                "notice" => icy_board.config.board.notice = text.clone(),
                "capabilities" => icy_board.config.board.capabilities = text.clone(),

                "sysop_name" => icy_board.config.sysop.name = text.clone(),
                "local_pass" => icy_board.config.sysop.password = Password::PlainText(text.clone()),
                "new_user_groups" => icy_board.config.new_user_settings.new_user_groups = text.clone(),
                "f1" => icy_board.config.func_keys[0] = text.clone(),
                "f2" => icy_board.config.func_keys[1] = text.clone(),
                "f3" => icy_board.config.func_keys[2] = text.clone(),
                "f4" => icy_board.config.func_keys[3] = text.clone(),
                "f5" => icy_board.config.func_keys[4] = text.clone(),
                "f6" => icy_board.config.func_keys[5] = text.clone(),
                "f7" => icy_board.config.func_keys[6] = text.clone(),
                "f8" => icy_board.config.func_keys[7] = text.clone(),
                "f9" => icy_board.config.func_keys[8] = text.clone(),
                "f10" => icy_board.config.func_keys[9] = text.clone(),
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::U32(i, _, _) => match item.id.as_str() {
                "sec_level" => icy_board.config.new_user_settings.sec_level = *i as u8,
                "num_nodes" => icy_board.config.board.num_nodes = *i as u16,
                "max_lines" => icy_board.config.options.max_msg_lines = *i as u16,
                "keyboard_timeout" => icy_board.config.options.keyboard_timeout = *i as u16,
                "upload_descr_lines" => icy_board.config.options.upload_descr_lines = *i as u8,

                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Bool(b) => match item.id.as_str() {
                "local_pass_exit" => icy_board.config.sysop.require_password_to_exit = *b,
                "use_real_name" => icy_board.config.sysop.use_real_name = *b,
                "allow_one_name_users" => icy_board.config.new_user_settings.allow_one_name_users = *b,
                "ask_city_or_state" => icy_board.config.new_user_settings.ask_city_or_state = *b,
                "ask_address" => icy_board.config.new_user_settings.ask_address = *b,
                "ask_verification" => icy_board.config.new_user_settings.ask_verification = *b,
                "ask_bus_data_phone" => icy_board.config.new_user_settings.ask_bus_data_phone = *b,
                "ask_voice_phone" => icy_board.config.new_user_settings.ask_voice_phone = *b,
                "ask_comment" => icy_board.config.new_user_settings.ask_comment = *b,
                "ask_clr_msg" => icy_board.config.new_user_settings.ask_clr_msg = *b,
                "ask_fse" => icy_board.config.new_user_settings.ask_fse = *b,
                "ask_xfer_protocol" => icy_board.config.new_user_settings.ask_xfer_protocol = *b,
                "ask_date_format" => icy_board.config.new_user_settings.ask_date_format = *b,
                "ask_alias" => icy_board.config.new_user_settings.ask_alias = *b,
                "ask_gender" => icy_board.config.new_user_settings.ask_gender = *b,
                "ask_birthdate" => icy_board.config.new_user_settings.ask_birthdate = *b,
                "ask_email" => icy_board.config.new_user_settings.ask_email = *b,
                "ask_web_address" => icy_board.config.new_user_settings.ask_web_address = *b,
                "ask_use_short_descr" => icy_board.config.new_user_settings.ask_use_short_descr = *b,
                "allow_iemsi" => icy_board.config.options.allow_iemsi = *b,
                "scan_all_mail_at_login" => icy_board.config.options.scan_all_mail_at_login = *b,
                "prompt_to_read_mail" => icy_board.config.options.prompt_to_read_mail = *b,
                "check_files_uploaded" => icy_board.config.options.check_files_uploaded = *b,
                "display_uploader" => icy_board.config.options.display_uploader = *b,
                "exclude_local_calls" => icy_board.config.options.exclude_local_calls = *b,
                "disable_full_record_updating" => icy_board.config.options.disable_full_record_updating = *b,
                "is_closed_board" => icy_board.config.options.is_closed_board = *b,
                "display_userinfo_at_login" => icy_board.config.options.display_userinfo_at_login = *b,
                "non_graphics" => icy_board.config.options.non_graphics = *b,
                "give_user_password_to_doors" => icy_board.config.options.give_user_password_to_doors = *b,
                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::Color(c) => match item.id.as_str() {
                "msg_hdr_date" => icy_board.config.color_configuration.msg_hdr_date = c.clone(),
                "msg_hdr_to" => icy_board.config.color_configuration.msg_hdr_to = c.clone(),
                "msg_hdr_from" => icy_board.config.color_configuration.msg_hdr_from = c.clone(),
                "msg_hdr_subj" => icy_board.config.color_configuration.msg_hdr_subj = c.clone(),
                "msg_hdr_read" => icy_board.config.color_configuration.msg_hdr_read = c.clone(),
                "msg_hdr_conf" => icy_board.config.color_configuration.msg_hdr_conf = c.clone(),
                "file_head" => icy_board.config.color_configuration.file_head = c.clone(),
                "file_name" => icy_board.config.color_configuration.file_name = c.clone(),
                "file_size" => icy_board.config.color_configuration.file_size = c.clone(),
                "file_date" => icy_board.config.color_configuration.file_date = c.clone(),
                "file_description" => icy_board.config.color_configuration.file_description = c.clone(),
                "file_description_low" => icy_board.config.color_configuration.file_description_low = c.clone(),
                "file_text" => icy_board.config.color_configuration.file_text = c.clone(),
                "file_deleted" => icy_board.config.color_configuration.file_deleted = c.clone(),

                _ => panic!("Unknown id: {}", item.id),
            },
            ListValue::ValueList(val, _list) => match item.id.as_str() {
                "date_format" => icy_board.config.board.date_format = val.clone(),
                _ => panic!("Unknown id: {}", item.id),
            },
            _ => panic!("Unknown id: {}", item.id),
        }
    }
}

impl TabPage for GeneralTab {
    fn title(&self) -> String {
        "General".to_string()
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let area = area.inner(Margin { horizontal: 2, vertical: 1 });

        Clear.render(area, frame.buffer_mut());

        let block = Block::new()
            .style(THEME.content_box)
            .padding(Padding::new(2, 2, 1, 1))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);
        block.render(area, frame.buffer_mut());

        let area = area.inner(Margin { vertical: 1, horizontal: 1 });
        self.config.render(area, frame, &mut self.state);

        if self.state.in_edit {
            self.set_cursor_position(frame);
        }
    }

    fn has_control(&self) -> bool {
        self.state.in_edit
    }

    fn set_cursor_position(&self, frame: &mut Frame) {
        self.config.get_item(self.state.selected).unwrap().text_field_state.set_cursor_position(frame);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> ResultState {
        let res = self.config.handle_key_press(key, &mut self.state);
        for item in self.config.iter() {
            self.write_item(item, &mut self.icy_board.lock().unwrap());
        }
        res
    }

    fn request_status(&self) -> ResultState {
        return ResultState {
            edit_mode: EditMode::None,
            status_line: if self.state.selected < self.config.entry.len() {
                "".to_string()
            } else {
                "".to_string()
            },
        };
    }

    fn get_help(&self) -> Text<'static> {
        /*
        if let Some(item) = self.config.get_item(self.state.selected) {
            let hlp = get_help(&item.id);
            return tui_markdown::from_str(&hlp);
        }

        let input = include_str!("../../data/general_help.md");
        tui_markdown::from_str(input)*/

        String::new().into()
    }
}

fn _get_help(help: &str) -> &'static str {
    match help {
        "date_format" => include_str!("../../data/date_format.md"),
        _ => "TODO - please contribute me.",
    }
}
