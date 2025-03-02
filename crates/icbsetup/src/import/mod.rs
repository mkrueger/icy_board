use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use codepages::tables::write_utf8_with_bom;
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoardError, IcyBoardSerializer, PCBoardImport, PcbUser,
        bulletins::BullettinList,
        commands::CommandList,
        conferences::ConferenceBase,
        file_directory::DirectoryList,
        group_list::GroupList,
        icb_config::{
            BoardInformation, BoardOptions, ColorConfiguration, ConfigPaths, DEFAULT_PCBOARD_DATE_FORMAT, DisplayNewsBehavior, IcbColor, IcbConfig,
            NewUserSettings, SubscriptionMode, SysopCommandLevels, SysopInformation,
        },
        icb_text::IcbTextFile,
        language::SupportedLanguages,
        login_server::LoginServer,
        menu::Menu,
        message_area::AreaList,
        pcbconferences::{PcbAdditionalConferenceHeader, PcbConferenceHeader},
        pcboard_data::PcbBoardData,
        read_with_encoding_detection,
        sec_levels::SecurityLevelDefinitions,
        state::functions::PPECall,
        statistics::Statistics,
        surveys::SurveyList,
        user_base::{Password, UserBase},
        user_inf::PcbUserInf,
        users::PcbUserRecord,
        xfer_protocols::SupportedProtocols,
    },
};
use icy_board_engine::{
    datetime::{IcbDoW, IcbTime},
    icy_board::{
        PCBoardRecordImporter,
        accounting_cfg::AccountingConfig,
        commands::CommandType,
        doors::DoorList,
        icb_config::{
            AccountingOptions, ConfigSwitches, EventOptions, FileTransferOptions, LimitOptions, MessageOptions, SystemControlOptions, UserCommandLevels,
        },
        security_expr::SecurityExpression,
        user_base::{PasswordInfo, User},
    },
};
use jamjam::util::echmoail::EchomailAddress;
use qfile::{QFilePath, QTraitSync};
use relative_path::{PathExt, RelativePathBuf};
use walkdir::WalkDir;

use self::import_log::ImportLog;

pub mod console_logger;
pub mod import_log;

pub trait OutputLogger {
    fn start_action(&self, message: String);
    fn check_error(&self, res: Option<std::io::Error>) -> Res<()>;
    fn warning(&self, message: String);
}

pub struct PCBoardImporter {
    pub output: Box<dyn OutputLogger>,
    pub data: PcbBoardData,
    pub output_directory: PathBuf,
    pub logger: ImportLog,

    /// Contains paths to map dos paths to unix paths
    /// For example:
    /// 'C:\' -> '/home/user/pcboard'
    /// Difference to map_paths is that this maps source paths to other source paths.
    pub resolve_paths: HashMap<String, String>,

    pub converted_files: HashMap<String, String>,
}

impl PCBoardImporter {
    pub fn new(file_name: &Path, output: Box<dyn OutputLogger>, output_directory: PathBuf) -> Res<Self> {
        match PcbBoardData::import_pcboard(file_name) {
            Ok(data) => {
                let mut paths = HashMap::new();

                let file_path = PathBuf::from(file_name);
                let mut help = data.path.help_loc.clone();
                if help.ends_with('\\') {
                    help.pop();
                }

                help = help.replace('\\', "/");

                let help_loc = PathBuf::from(&help);
                let mut path = file_path.parent().unwrap().to_path_buf();

                let upper = path.to_string_lossy().to_ascii_uppercase();

                let source_directory = path.clone();
                output.start_action(format!("Importing PCBoard from base path {}\n", source_directory.display()));

                path.push(help_loc.file_name().unwrap_or_default());
                if !path.exists() {
                    return Err(Box::new(IcyBoardError::Error("Can't resolve C: file".to_string())));
                }

                //let len = to_str().unwrap().len();
                if let Some(k) = help_loc.parent() {
                    if let Some(v) = file_path.parent() {
                        let k = k.to_str().unwrap_or_default().to_string();
                        let v = v.to_path_buf().to_str().unwrap_or_default().to_string();
                        paths.insert(k, v);
                    }
                }

                let mut map_paths = HashMap::new();
                map_paths.insert(upper.clone() + "\\PPE", output_directory.join("ppe"));
                map_paths.insert(upper.clone(), output_directory.clone());

                Ok(Self {
                    output,
                    data,
                    output_directory,
                    resolve_paths: paths,
                    logger: ImportLog::default(),
                    converted_files: HashMap::new(),
                })
            }
            Err(err) => Err(Box::new(IcyBoardError::Error(format!("Error reading PCBoard data: {}", err.to_string())))),
        }
    }

    pub fn resolve_file(&self, file: &str) -> PathBuf {
        let path = PathBuf::from(file);
        if path.exists() {
            return path;
        }

        let mut s: String = file
            .chars()
            .map(|x| match x {
                '\\' => '/',
                _ => x,
            })
            .collect();
        // hack for "/path" - assume that PCB is on the same drive & top level dir (like C:\PCB)
        if s.starts_with("/") {
            if let Some(path) = self.resolve_paths.values().next() {
                s = format!("{}/..{}", path, s);
            }
        }

        for (k, v) in &self.resolve_paths {
            if s.to_ascii_uppercase().starts_with(&k.to_ascii_uppercase()) {
                if v.ends_with('/') || s.chars().skip(k.len()).next().unwrap() == '/' {
                    s = v.clone() + &s[k.len()..];
                } else {
                    s = v.clone() + "/" + &s[k.len()..];
                }
            }
        }

        if let Ok(mut file_path) = QFilePath::add_path(s.clone()) {
            if let Ok(file) = file_path.get_path_buf() {
                return file;
            }
        }
        let res = PathBuf::from(s);
        res
    }

    pub fn start_import(&mut self) -> Res<()> {
        self.create_directories()?;

        self.copy_display_directory("help files", &self.data.path.help_loc.clone(), "art/help", |_| true)?;
        self.copy_display_directory(
            "commmand display files",
            &self.data.path.cmd_display_files_loc.clone(),
            "art/cmd_display",
            |_| true,
        )?;
        self.copy_display_directory("security files", &self.data.path.sec_loc.clone(), "art/secmsgs", |p| {
            let file_name = p.file_name().unwrap().to_str().unwrap();
            file_name.chars().next().unwrap_or_default().is_ascii_digit()
        })?;

        let icbtext = self.convert_pcbtext(&(self.data.path.text_loc.clone() + "/PCBTEXT"), "main/icbtext")?;
        // let bad_users = self.convert_trashcan(&self.data.path.tcan_file.clone(), "main/tcan_user.txt")?;
        let trashcan_upload_files = self.convert_trashcan(&self.data.path.tcan_file.clone(), "main/tcan_user.txt")?;
        let tcan_email = self.create_file(include_str!("../../data/tcan_email.txt"), "main/tcan_email.txt")?;
        let tcan_passwords = self.create_file(include_str!("../../data/tcan_passwords.txt"), "main/tcan_passwords.txt")?;
        let vip_users = self.create_file(include_str!("../../data/vip_users.txt"), "main/vip_users.txt")?;

        let group_file = self.create_group_file("main/groups")?;

        let welcome = self.convert_display_file(&self.data.path.welcome_file.clone(), "art/welcome")?;
        let newuser = self.convert_display_file(&self.data.path.newuser_file.clone(), "art/newuser")?;
        let closed = self.convert_display_file(&self.data.path.closed_file.clone(), "art/closed")?;
        let warning = self.convert_display_file(&self.data.path.warning_file.clone(), "art/warning")?;
        let expired = self.convert_display_file(&self.data.path.expired_file.clone(), "art/expired")?;
        let caller_log = self.convert_display_file(&self.data.path.clr_file.clone(), "caller.log")?;

        let accounting_config_file = self.convert_accounting_cfg(&self.data.account_config.clone(), "main/accounting_cfg.toml")?;
        let accounting_holiday_list_file = self.convert_display_file(&self.data.holidays_file.clone(), "art/acc_holidays")?;
        let accounting_info_file = self.convert_display_file(&self.data.account_info.clone(), "art/acc_info")?;
        let accounting_warning_file = self.convert_display_file(&&self.data.account_warn.clone(), "art/acc_warn")?;
        let accounting_logoff_file = self.convert_display_file(&&self.data.account_logoff.clone(), "art/acc_logoff")?;

        let conf_join_menu = self.convert_display_file(&self.data.path.conf_menu.clone(), "art/cnfn")?;
        let group_chat = self.convert_display_file(&self.data.path.group_chat.clone(), "art/group")?;
        let chat_menu = self.convert_display_file(&self.data.path.chat_menu.clone(), "art/chtm")?;
        let chat_actions_menu = self.convert_display_file(&self.data.path.chat_menu.clone(), "art/chatactm")?;

        let no_ansi = self.convert_display_file(&self.data.path.no_ansi.clone(), "art/noansi")?;

        let logon_survey = self.convert_logon_surveys(&self.data.path.login_script.clone(), "art/login_survey")?;
        let logon_answer = PathBuf::from("art/login_answer");

        let logoff_survey = self.convert_logon_surveys(&self.data.path.logoff_script.clone(), "art/logoff_survey")?;
        let logoff_answer = PathBuf::from("art/logoff_answer");

        let newask_survey = self.convert_logon_surveys(&self.data.path.newreg_file.clone(), "art/newask_survey")?;
        let newask_answer = PathBuf::from("art/newask_answer");

        self.convert_user_base(&self.data.path.usr_file.clone(), &self.data.path.inf_file.clone(), "main/users.toml")?;

        let protocol_data_file = self.convert_data::<SupportedProtocols>(&self.data.path.protocol_data_file.clone(), "main/protocols.toml")?;
        let language_file = self.convert_data::<SupportedLanguages>(&self.data.path.pcml_dat_file.clone(), "main/languages.toml")?;
        let security_level_file = self.convert_data::<SecurityLevelDefinitions>(&self.data.path.pwrd_file.clone(), "main/security_levels.toml")?;
        let command_file = self.convert_default_cmd_lst(&self.data.path.cmd_lst.clone(), "main/commands.toml")?;
        let statistics_file = self.convert_data::<Statistics>(&self.data.path.stats_file.clone(), "main/statistics.toml")?;

        let conferences = self.convert_conferences(&self.data.path.conference_file.clone(), "main/conferences.toml")?;

        let color_file = self.resolve_file(&self.data.path.color_file);

        let mut color_configuration = ColorConfiguration {
            default: IcbColor::Dos(self.data.colors.default as u8),
            msg_hdr_date: IcbColor::Dos(self.data.colors.msg_hdr_date as u8),
            msg_hdr_to: IcbColor::Dos(self.data.colors.msg_hdr_to as u8),
            msg_hdr_from: IcbColor::Dos(self.data.colors.msg_hdr_from as u8),
            msg_hdr_subj: IcbColor::Dos(self.data.colors.msg_hdr_subj as u8),
            msg_hdr_read: IcbColor::Dos(self.data.colors.msg_hdr_read as u8),
            msg_hdr_conf: IcbColor::Dos(self.data.colors.msg_hdr_conf as u8),
            ..Default::default()
        };

        if color_file.exists() {
            let color_file = fs::read(color_file)?;
            let start = 123;
            color_configuration.file_name = IcbColor::Dos(color_file[start]);
            color_configuration.file_size = IcbColor::Dos(color_file[start + 2]);
            color_configuration.file_date = IcbColor::Dos(color_file[start + 4]);
            color_configuration.file_description = IcbColor::Dos(color_file[start + 6]);
            color_configuration.file_head = IcbColor::Dos(color_file[start + 8]);
            color_configuration.file_text = IcbColor::Dos(color_file[start + 10]);
            color_configuration.file_description_low = IcbColor::Dos(color_file[start + 12]);
            color_configuration.file_deleted = IcbColor::Dos(color_file[start + 14]);
            color_configuration.file_offline = IcbColor::Dos(color_file[start + 16]);
            color_configuration.file_new_file = IcbColor::Dos(color_file[start + 18]);
        }

        let mut icb_cfg = IcbConfig {
            sysop: SysopInformation {
                name: self.data.sysop_info.sysop.clone(),
                password: Password::from_str(self.data.sysop_info.password.as_str()).unwrap(),
                require_password_to_exit: self.data.sysop_info.require_pwrd_to_exit,
                use_real_name: self.data.sysop_info.use_real_name,
                external_editor: "nano".to_string(),
                config_color_theme: "DEFAULT".to_string(),
            },
            sysop_command_level: SysopCommandLevels {
                sysop: self.data.sysop_security.sysop as u8,
                read_all_comments: SecurityExpression::from_req_security(self.data.sysop_security.read_all_comments as u8),
                read_all_mail: SecurityExpression::from_req_security(self.data.sysop_security.read_all_mail as u8),
                copy_move_messages: SecurityExpression::from_req_security(self.data.sysop_security.copy_move_messages as u8),
                enter_color_codes_in_messages: SecurityExpression::from_req_security(self.data.sysop_security.enter_at_vars_in_messages as u8),
                use_broadcast_command: SecurityExpression::from_req_security(self.data.sysop_security.use_broadcast_command as u8),
                view_private_uploads: SecurityExpression::from_req_security(self.data.sysop_security.view_private_uploads as u8),
                edit_message_headers: SecurityExpression::from_req_security(self.data.sysop_security.edit_message_headers as u8),
                protect_unprotect_messages: SecurityExpression::from_req_security(self.data.sysop_security.protect_unprotect_messages as u8),
                set_pack_out_date_on_messages: SecurityExpression::from_req_security(self.data.sysop_security.set_pack_out_date_on_messages as u8),
                edit_any_message: SecurityExpression::from_req_security(self.data.sysop_security.edit_any_message as u8),
                not_update_msg_read: SecurityExpression::from_req_security(self.data.sysop_security.not_update_msg_read_status as u8),
                enter_generic_messages: SecurityExpression::from_req_security(self.data.sysop_security.enter_generic_message as u8),
                overwrite_files_on_uploads: SecurityExpression::from_req_security(self.data.sysop_security.overwrite_uploads as u8),
                see_all_return_receipts: SecurityExpression::from_req_security(self.data.sysop_security.see_all_return_receipt_messages as u8),

                sec_1_view_caller_log: SecurityExpression::from_req_security(self.data.sysop_security.sec_1_view_caller_log as u8),
                sec_2_view_usr_list: SecurityExpression::from_req_security(self.data.sysop_security.sec_2_view_usr_list as u8),
                sec_3_pack_renumber_msg: SecurityExpression::from_req_security(self.data.sysop_security.sec_3_pack_renumber_msg as u8),
                sec_4_recover_deleted_msg: SecurityExpression::from_req_security(self.data.sysop_security.sec_4_recover_deleted_msg as u8),
                sec_5_list_message_hdr: SecurityExpression::from_req_security(self.data.sysop_security.sec_5_list_message_hdr as u8),
                sec_6_view_any_file: SecurityExpression::from_req_security(self.data.sysop_security.sec_6_view_any_file as u8),
                sec_7_user_maint: SecurityExpression::from_req_security(self.data.sysop_security.sec_7_user_maint as u8),
                sec_8_pack_usr_file: SecurityExpression::from_req_security(self.data.sysop_security.sec_8_pack_usr_file as u8),
                sec_9_exit_to_dos: SecurityExpression::from_req_security(self.data.sysop_security.sec_9_exit_to_dos as u8),
                sec_10_shelled_dos_func: SecurityExpression::from_req_security(self.data.sysop_security.sec_10_shelled_dos_func as u8),
                sec_11_view_other_nodes: SecurityExpression::from_req_security(self.data.sysop_security.sec_11_view_other_nodes as u8),
                sec_12_logoff_alt_node: SecurityExpression::from_req_security(self.data.sysop_security.sec_12_logoff_alt_node as u8),
                sec_13_view_alt_node_callers: SecurityExpression::from_req_security(self.data.sysop_security.sec_13_drop_alt_node_to_dos as u8),
                sec_14_drop_alt_node_to_dos: SecurityExpression::from_req_security(self.data.sysop_security.sec_14_drop_to_dos as u8),
            },

            user_command_level: UserCommandLevels {
                cmd_a: SecurityExpression::from_req_security(self.data.user_levels.cmd_a as u8),
                cmd_b: SecurityExpression::from_req_security(self.data.user_levels.cmd_b as u8),
                cmd_c: SecurityExpression::from_req_security(self.data.user_levels.cmd_c as u8),
                cmd_d: SecurityExpression::from_req_security(self.data.user_levels.cmd_d as u8),
                cmd_e: SecurityExpression::from_req_security(self.data.user_levels.cmd_e as u8),
                cmd_f: SecurityExpression::from_req_security(self.data.user_levels.cmd_f as u8),
                cmd_h: SecurityExpression::from_req_security(self.data.user_levels.cmd_h as u8),
                cmd_i: SecurityExpression::from_req_security(self.data.user_levels.cmd_i as u8),
                cmd_j: SecurityExpression::from_req_security(self.data.user_levels.cmd_j as u8),
                cmd_k: SecurityExpression::from_req_security(self.data.user_levels.cmd_k as u8),
                cmd_l: SecurityExpression::from_req_security(self.data.user_levels.cmd_l as u8),
                cmd_m: SecurityExpression::from_req_security(self.data.user_levels.cmd_m as u8),
                cmd_n: SecurityExpression::from_req_security(self.data.user_levels.cmd_n as u8),
                cmd_o: SecurityExpression::from_req_security(self.data.user_levels.cmd_o as u8),
                cmd_p: SecurityExpression::from_req_security(self.data.user_levels.cmd_p as u8),
                cmd_q: SecurityExpression::from_req_security(self.data.user_levels.cmd_q as u8),
                cmd_r: SecurityExpression::from_req_security(self.data.user_levels.cmd_r as u8),
                cmd_s: SecurityExpression::from_req_security(self.data.user_levels.cmd_s as u8),
                cmd_t: SecurityExpression::from_req_security(self.data.user_levels.cmd_t as u8),
                cmd_u: SecurityExpression::from_req_security(self.data.user_levels.cmd_u as u8),
                cmd_v: SecurityExpression::from_req_security(self.data.user_levels.cmd_v as u8),
                cmd_w: SecurityExpression::from_req_security(self.data.user_levels.cmd_w as u8),
                cmd_x: SecurityExpression::from_req_security(self.data.user_levels.cmd_x as u8),
                cmd_y: SecurityExpression::from_req_security(self.data.user_levels.cmd_y as u8),
                cmd_z: SecurityExpression::from_req_security(self.data.user_levels.cmd_z as u8),

                cmd_chat: SecurityExpression::from_req_security(self.data.user_levels.cmd_chat as u8),
                cmd_open_door: SecurityExpression::from_req_security(self.data.user_levels.cmd_open_door as u8),
                cmd_show_user_list: SecurityExpression::from_req_security(self.data.user_levels.cmd_show_user_list as u8),
                cmd_test_file: SecurityExpression::from_req_security(self.data.user_levels.cmd_test_file as u8),
                cmd_who: SecurityExpression::from_req_security(self.data.user_levels.cmd_who as u8),
                batch_file_transfer: SecurityExpression::from_req_security(self.data.user_levels.batch_file_transfer as u8),
                edit_own_messages: SecurityExpression::from_req_security(self.data.user_levels.edit_own_messages as u8),
            },
            color_configuration,
            board: BoardInformation {
                name: self.data.board_name.clone(),
                location: self.data.origin.clone(),
                operator: String::new(),
                notice: String::new(),
                capabilities: String::new(),
                date_format: DEFAULT_PCBOARD_DATE_FORMAT.to_string(),
                num_nodes: 4,
                allow_iemsi: true,
                who_include_city: self.data.who_include_city,
                who_show_alias: self.data.who_show_alias,
            },
            login_server: LoginServer::default(),
            func_keys: self.data.func_keys.clone(),
            subscription_info: SubscriptionMode {
                is_enabled: self.data.subscription_info.is_enabled,
                subscription_length: self.data.subscription_info.subscription_length as u32,
                default_expired_level: self.data.subscription_info.default_expired_level,
                warning_days: self.data.subscription_info.warning_days as u32,
            },
            paths: ConfigPaths {
                help_path: PathBuf::from("art/help"),
                security_file_path: PathBuf::from("art/secmsgs"),
                command_display_path: PathBuf::from("art/cmd_display"),
                tmp_work_path: PathBuf::from("tmp/"),
                user_file: PathBuf::from("main/users.toml"),
                email_msgbase: PathBuf::from("main/email"),
                caller_log,
                icbtext,
                conferences,
                welcome,
                newuser,
                closed,
                expire_warning: warning,
                expired,
                conf_join_menu,
                chat_intro_file: group_chat,
                chat_menu,
                chat_actions_menu,
                no_ansi,
                protocol_data_file,
                pwrd_sec_level_file: security_level_file,
                language_file,
                command_file,
                statistics_file,
                group_file,

                trashcan_upload_files,
                trashcan_user: PathBuf::new(),
                trashcan_email: tcan_email,
                trashcan_passwords: tcan_passwords,
                vip_users,

                logon_survey,
                logon_answer,
                logoff_survey,
                logoff_answer,
                newask_survey,
                newask_answer,
            },
            new_user_settings: NewUserSettings {
                sec_level: self.data.user_levels.agree_to_register as u8,
                new_user_groups: "new_users".to_string(),
                allow_one_name_users: self.data.allow_one_name_users,
                use_newask_and_builtin: self.data.use_new_ask_file,
                ask_city_or_state: true,
                ask_address: false,
                ask_verification: false,
                ask_business_phone: true,
                ask_home_phone: true,
                ask_comment: true,
                ask_clr_msg: true,
                ask_xfer_protocol: !self.data.skip_protocol,
                ask_date_format: true,
                ask_alias: !self.data.skip_alias,
                ask_gender: true,
                ask_birthdate: true,
                ask_email: true,
                ask_web_address: true,
                ask_use_short_descr: true,
                ask_fse: true,
            },
            message: MessageOptions {
                max_msg_lines: self.data.max_msg_lines as u16,
                scan_all_mail_at_login: self.data.scan_all,
                prompt_to_read_mail: self.data.prompt_to_read_mail,
                disable_message_scan_prompt: self.data.disable_scan,
                allow_esc_codes: self.data.allow_esc_codes,
                allow_carbon_copy: true, // TODO
                validate_to_name: self.data.validate_to,
                default_quick_personal_scan: self.data.quick_scan,
                default_scan_all_selected_confs_at_login: self.data.scan_all,
            },
            file_transfer: FileTransferOptions {
                verify_files_uploaded: self.data.test_uploads,
                display_uploader: self.data.upload_by,
                upload_descr_lines: self.data.num_ul_desc_lines as u8,

                disallow_batch_uploads: self.data.no_batch_up,
                promote_to_batch_transfers: self.data.promote_batch,
                upload_credit_time: self.data.upload_credit.max(0) as u32,
                upload_credit_bytes: self.data.byte_credit.max(0) as u32,
                disable_drive_size_check: self.data.disable_drive_check,
                stop_uploads_free_space: self.data.stop_free_space.max(0) as u32,
            },
            system_control: SystemControlOptions {
                disable_ns_logon: false, // TODO
                disable_full_record_updating: self.data.allow_pwrd_only,
                is_closed_board: self.data.closed_board,
                guard_logoff: self.data.guard_logoff,
                allow_alias_change: self.data.allow_alias_change,
                is_multi_lingual: self.data.multi_lingual,
                enforce_daily_time_limit: self.data.enforce_time,
                allow_password_failure_comment: self.data.allow_pwrd_comment,
            },
            switches: ConfigSwitches {
                display_news_behavior: DisplayNewsBehavior::from_pcb_char(self.data.display_news),
                exclude_local_calls_stats: self.data.exclude_locals,
                display_userinfo_at_login: self.data.display_userinfo_at_login,
                non_graphics: self.data.non_graphics,
                default_graphics_at_login: self.data.default_graphics,
                force_intro_on_join: self.data.force_intro,
                scan_new_blt: self.data.scan_blts,
                capture_grp_chat_session: self.data.record_group_chat,
                allow_handle_in_grpchat: self.data.allow_handles,
            },
            limits: LimitOptions {
                keyboard_timeout: self.data.kbd_timeout as u16,
                min_pwd_length: self.data.min_pwrd_len as u8,
                password_expire_days: self.data.pwrd_update as u16,
                password_expire_warn_days: self.data.pwrd_warn as u16,
                sysop_start: IcbTime::parse(&self.data.sysop_start),
                sysop_stop: IcbTime::parse(&self.data.sysop_stop),
                max_number_upload_descr_lines: self.data.num_ul_desc_lines as u16,
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
                suspend_minutes: self.data.event_suspend as u16,
                disallow_uploads: self.data.event_stop_uplds,
                minutes_uploads_disallowed: self.data.min_prior_to_event as u16,
            },
            accounting: AccountingOptions {
                enabled: self.data.enable_accounting,
                use_money: self.data.acc_show_currency,
                concurrent_tracking: self.data.acc_concurrent_tracking,
                ignore_empty_sec_level: self.data.acc_ignore_drop_sec_level,
                peak_usage_start: IcbTime::parse(&self.data.peak_start),
                peak_usage_end: IcbTime::parse(&self.data.peak_end),
                peak_days_of_week: IcbDoW::from_str(&self.data.peak_days).unwrap_or_default(),

                peak_holiday_list_file: accounting_holiday_list_file,
                cfg_file: accounting_config_file,
                tracking_file: PathBuf::new(),
                info_file: accounting_info_file,
                warning_file: accounting_warning_file,
                logoff_file: accounting_logoff_file,
                accounting_config: None,
            },
        };
        icb_cfg.board.allow_iemsi = false;
        icb_cfg.login_server.telnet.port = 1337;
        icb_cfg.login_server.ssh.port = 1338;

        let destination = self.output_directory.join(icy_board_engine::DEFAULT_ICYBOARD_FILE);
        self.output.start_action(format!("Create main configuration {}…", destination.display()));
        if let Err(err) = icb_cfg.save(&destination) {
            self.logger.log_boxed_error(&*err);
        }
        self.output.start_action("done.".into());
        self.logger.log("done.");

        let destination = self.output_directory.join("importlog.txt");
        fs::write(destination, &self.logger.output)?;

        Ok(())
    }

    fn convert_conferences(&mut self, conference_file: &str, new_rel_name: &str) -> Res<PathBuf> {
        self.output.start_action("Convert conferences…".into());

        let conf = self.resolve_file(conference_file);
        if !conf.exists() {
            self.output
                .warning(format!("Can't find conference file {}/{}", conf.display(), conference_file));
            self.logger
                .log(&format!("Can't find conference file {}/{}", conf.display(), conference_file).as_str());
            return Ok(PathBuf::new());
        }
        let conferences = PcbConferenceHeader::import_pcboard(&conf, self.data.num_conf as usize)?;

        let conf_add = self.resolve_file(&(conference_file.to_string() + ".ADD"));
        let conf_old_add = self.resolve_file(&(conference_file.to_string() + ".@@@"));

        let add_conferences = if conf_add.exists() {
            self.logger.log(&format!("import conference header {}", conf_add.display()).as_str());
            PcbAdditionalConferenceHeader::import_pcboard(&conf_add)?
        } else if conf_old_add.exists() {
            self.logger.log(&format!("read old conference header {}", conf_old_add.display()).as_str());
            PcbAdditionalConferenceHeader::import_old_pcboard(&conf_old_add)?
        } else {
            self.output.warning(format!("Can't find conference add file {}", conf_add.display()));
            self.logger.log(&format!("Can't find conference add file {}", conf_add.display()).as_str());
            vec![PcbAdditionalConferenceHeader::default(); conferences.len()]
        };
        self.logger.log("imported conference headers, converting...");

        let mut conf_base = ConferenceBase::import_pcboard(&self.output_directory, &conferences, &add_conferences);
        for (i, conf) in conf_base.iter_mut().enumerate() {
            let output = if i == 0 { "conferences/main".to_string() } else { format!("conferences/{i}") };
            let destination = self.output_directory.join(&output);

            let _ = fs::create_dir(&destination);
            conf.attachment_location = self.copy_attachment_directory(&(output.to_string() + "/attach"), &conf.attachment_location)?;
            conf.intro_file = self.convert_conference_display_file(&output, &conf.intro_file)?;
            conf.users_menu = self.convert_conference_display_file(&output, &conf.users_menu)?;
            conf.sysop_menu = self.convert_conference_display_file(&output, &conf.sysop_menu)?;
            conf.news_file = self.convert_conference_display_file(&output, &conf.news_file)?;
            conf.pub_upload_location = PathBuf::from(output.to_string() + "/pub_up");
            conf.private_upload_location = PathBuf::from(output.to_string() + "/priv_up");
            conf.doors_menu = self.convert_conference_display_file(&output, &conf.doors_menu)?;
            conf.doors_file = self.convert_doors_file(&destination, &output, &conf.doors_file)?;

            conf.blt_menu = self.convert_conference_display_file(&output, &conf.blt_menu)?;
            conf.blt_file = self.convert_bullettin_file(&destination, &output, &conf.blt_file)?;

            self.logger.log(&format!("convert survey menus {}", conf.survey_menu.display()).as_str());
            conf.survey_menu = self.convert_conference_display_file(&output, &conf.survey_menu)?;

            conf.survey_file = self.convert_questionnaires(&destination, &output, &conf.survey_file)?;

            conf.dir_menu = self.convert_conference_display_file(&output, &conf.dir_menu)?;
            conf.dir_file = self.convert_dirlist_file(&destination, &output, &conf.dir_file)?;

            conf.area_menu = PathBuf::from(output.to_string() + "/area");
            conf.area_file = PathBuf::from(output.to_string() + "/area.toml");

            match AreaList::load(&destination.join("area.toml")) {
                Ok(mut list) => {
                    for area in list.iter_mut() {
                        area.path = self.convert_message_base(&destination, &output, &area.path)?;
                    }
                    list.save(&destination.join("area.toml"))?;
                }
                Err(err) => {
                    self.logger.log("Can't load message area file.");
                    self.logger.log_boxed_error(&*err);
                }
            }
        }

        let destination = self.output_directory.join(new_rel_name);
        conf_base.save(&destination)?;
        self.logger.create_new_file(destination.display().to_string());

        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_conference_display_file(&mut self, output: &str, file_name: &Path) -> Res<PathBuf> {
        let Some(file_name) = file_name.file_name() else {
            return Ok(PathBuf::new());
        };

        let resolved_file = self.resolve_file(file_name.to_str().unwrap());

        let name = resolved_file.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
        let new_name = format!("{}/{}", output, &name);
        self.convert_display_file(file_name.to_str().unwrap(), &new_name)
    }

    pub fn create_directories(&mut self) -> Res<()> {
        self.output.start_action(format!("Creating directory '{}'…", self.output_directory.display()));
        self.logger.log_error(fs::create_dir(&self.output_directory).err())?;
        self.logger.created_directory(self.output_directory.clone());

        const REQUIRED_DIRECTORIES: [&str; 11] = [
            "gen",
            "conferences",
            "conferences/main",
            "ppe",
            "main",
            "main/menus",
            "art",
            "art/cmd_display",
            "art/secmsgs",
            "art/help",
            "work",
        ];

        for dir in REQUIRED_DIRECTORIES.iter() {
            let o = self.output_directory.join(dir);
            self.output.start_action(format!("Creating directory '{}'…", o.display()));
            self.output.check_error(fs::create_dir(&o).err())?;
            self.logger.created_directory(o);
        }
        self.logger.log("");

        Ok(())
    }

    fn convert_pcbtext(&mut self, pcb_text_file: &str, new_rel_name: &str) -> Res<PathBuf> {
        let destination = self.output_directory.join(new_rel_name);
        self.output.start_action(format!("Create ICBTEXT {}…", destination.display()));

        let resolved_file = self.resolve_file(pcb_text_file);

        if let Some(parent) = resolved_file.parent() {
            for entry in fs::read_dir(parent)?.flatten() {
                if entry.path().is_dir() {
                    continue;
                }
                if entry
                    .file_name()
                    .to_str()
                    .unwrap()
                    .to_ascii_uppercase()
                    .starts_with(&resolved_file.file_name().unwrap().to_str().unwrap().to_ascii_uppercase())
                {
                    if let Ok(mut text_file) = IcbTextFile::load(&entry.path()) {
                        for (i, entry) in text_file.iter_mut().enumerate() {
                            entry.text = self.scan_pcb_text_line_for_commands(&entry.text, i)?;
                        }
                        let destination: PathBuf = PathBuf::from(
                            destination
                                .with_extension(entry.path().extension().unwrap_or_default().to_ascii_lowercase())
                                .to_string_lossy()
                                .to_string()
                                + ".toml",
                        );
                        self.logger
                            .log(&format!("------------- PCBTEXT import: {} ->{}", entry.path().display(), destination.display()));

                        if let Err(err) = text_file.save(&destination) {
                            self.logger.log_boxed_error(&*err);
                        }
                        self.logger.converted_file(&resolved_file, &destination, true);
                        self.logger.log("");
                    }
                }
            }
        }

        Ok(PathBuf::from(new_rel_name.to_string() + ".toml"))
    }

    pub fn scan_pcb_text_line_for_commands(&mut self, line: &str, i: usize) -> Res<String> {
        if let Some(call) = PPECall::try_parse_line(line) {
            self.logger.log(&format!(
                "Found {:?} in entry {} : {} arguments :{:?}",
                call.call_type, i, call.file, call.arguments
            ));
            let resolved_file = self.resolve_file(&call.file);
            if !resolved_file.exists() {
                self.output.warning(format!("Can't find file {}", resolved_file.display()));
                self.logger.log(&format!("Can't find file {}", resolved_file.display()));
                return Ok(line.to_string());
            }
            let new_name = self.convert_file(resolved_file)?;

            let mut new_line = String::new();
            for (i, ch) in line.chars().enumerate() {
                if i == 1 {
                    new_line.push_str(&new_name);
                }
                if i >= 1 && i <= call.file.len() {
                    continue;
                }
                new_line.push(ch);
            }
            self.logger.log(&format!("Convert to line: {}", new_line));
            return Ok(new_line);
        }
        Ok(line.to_string())
    }

    pub fn scan_line_for_commands(&mut self, line: &str, i: usize) -> Res<String> {
        if let Some(call) = PPECall::try_parse_line(line) {
            self.logger.log(&format!(
                "Found {:?} in line {} : {} arguments :{:?}",
                call.call_type, i, call.file, call.arguments
            ));
            let resolved_file = self.resolve_file(&call.file);
            if !resolved_file.exists() {
                self.output.warning(format!("Can't find file {}", resolved_file.display()));
                self.logger.log(&format!("Can't find file {}", resolved_file.display()));
                return Ok(line.to_string());
            }
            if resolved_file.is_dir() {
                self.logger.log(&format!("{} is a directory, skipping.", resolved_file.display()));
                return Ok(line.to_string());
            }
            let new_name = self.convert_file(resolved_file)?;

            let mut new_line = String::new();
            for (i, ch) in line.chars().enumerate() {
                if i == 1 {
                    new_line.push_str(&new_name);
                }
                if i >= 1 && i <= call.file.len() {
                    continue;
                }
                new_line.push(ch);
            }
            self.logger.log(&format!("Convert to line: {}", new_line));
            return Ok(new_line);
        }
        Ok(line.to_string())
    }

    fn convert_file(&mut self, resolved_file: PathBuf) -> Res<String> {
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(file.clone());
        }

        if let Some(ext) = resolved_file.extension() {
            match ext.to_ascii_uppercase().to_string_lossy().to_string().as_str() {
                "PPE" => {
                    self.converted_files
                        .insert(upper_file_name.clone(), resolved_file.to_str().unwrap().to_string());
                    return Ok(resolved_file.to_str().unwrap().to_string());
                }
                "MNU" => {
                    let imported_menu = Menu::import_pcboard(&resolved_file)?;
                    let fname = format!("main/menus/{}", resolved_file.file_name().unwrap().to_ascii_lowercase().to_string_lossy());
                    let menu_path = self.output_directory.join(&fname);
                    imported_menu.save(&menu_path)?;
                    self.converted_files.insert(upper_file_name.clone(), fname);
                    let out_path = menu_path.file_name().unwrap().to_str().unwrap().to_string();
                    self.logger.translated_file(&resolved_file, &menu_path);
                    return Ok(out_path);
                }
                _ => {}
            }
        }

        let rel_name = format!("gen/{}", resolved_file.file_name().unwrap().to_ascii_lowercase().to_string_lossy());
        self.converted_files.insert(upper_file_name.clone(), rel_name.clone());

        let output_path = self.output_directory.join(&rel_name);
        match self.import_and_scan_file(&resolved_file, &output_path) {
            Err(err) => {
                self.logger.log_boxed_error(&*err);
                return Ok(resolved_file.to_str().unwrap().to_string());
            }
            _ => {}
        }
        return Ok(rel_name);

        /*
        let rel_name = format!("gen/{}", resolved_file.file_name().unwrap().to_ascii_lowercase().to_string_lossy());
        let output_path = self.output_directory.join(&rel_name);
        convert_to_utf8(&resolved_file, &output_path)?;
        self.converted_files.insert(upper_file_name.clone(), rel_name);
        let out_path = output_path.file_name().unwrap().to_str().unwrap().to_string();
        self.logger.converted_file(&resolved_file, &output_path, true);
        Ok(out_path)*/
    }

    pub fn import_and_scan_file<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, from: &P, to: &Q) -> Res<()> {
        let from = from.as_ref();
        if from.is_dir() {
            self.logger.log(&format!("{} is a directory, skipping.", from.display()));
            return Ok(());
        }
        let in_string = read_with_encoding_detection(&from)?;
        self.output
            .start_action(format!("\t convert '{}' to utf8 '{}'…", from.display(), to.as_ref().display()));
        let mut import = String::new();

        for (i, line) in in_string.lines().enumerate() {
            let line_txt = self.scan_line_for_commands(line, i)?;
            import.push_str(&line_txt);
            import.push('\n');
        }

        write_utf8_with_bom(to, &import)?;
        self.logger.converted_file(from.as_ref(), to.as_ref(), true);
        Ok(())
    }

    fn create_group_file(&mut self, new_rel_name: &str) -> Res<PathBuf> {
        let dest = self.output_directory.join(new_rel_name);

        let mut groups = GroupList::default();
        groups.add_group("sysop", "System Operators");
        groups.add_group("users", "Common Users");
        groups.save(&dest)?;
        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_trashcan(&mut self, trashcan_file: &str, new_rel_name: &str) -> Res<PathBuf> {
        if trashcan_file.is_empty() {
            return Ok(PathBuf::new());
        }

        let resolved_file = self.resolve_file(trashcan_file);
        let resolved_file = PathBuf::from(&resolved_file);
        let trashcan_header = include_str!("../../data/tcan_users.txt");

        let dest = self.output_directory.join(new_rel_name);
        self.output.start_action(format!("Convert trashcan -> tcan_users.txt {}…", dest.display()));

        if !resolved_file.exists() {
            fs::write(new_rel_name, trashcan_header)?;
            self.logger.create_new_file(dest.clone().to_string_lossy());
            return Ok(dest);
        }
        let mut trashcan = regex::escape(&read_with_encoding_detection(&resolved_file)?);
        if !trashcan.ends_with('\n') {
            trashcan.push('\n');
        }

        if let Err(err) = fs::write(dest.clone(), trashcan_header.to_string() + trashcan.as_str()) {
            return Err(Box::new(IcyBoardError::ErrorCreatingFile(new_rel_name.to_string(), err.to_string())));
        }
        self.logger.translated_file(&resolved_file, &dest);
        self.logger.log("");
        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_display_file(&mut self, file: &str, new_name: &str) -> Res<PathBuf> {
        self.logger.log(&format!("search for {} ({})", file, new_name));

        if file.is_empty() {
            self.logger.log(&format!("Original file not defined: {}", new_name));
            return Ok(PathBuf::new());
        }
        let resolved_file = self.resolve_file(file);
        if !resolved_file.exists() {
            self.output.warning(format!("File not found {}", resolved_file.display()));
            self.logger.log(&format!("File not found {}", resolved_file.display()));
            return Ok(PathBuf::from(new_name));
        };

        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            self.logger.log(&format!("already converted ({})", file));
            return Ok(PathBuf::from(file));
        }

        let from_file = PathBuf::from(&resolved_file);
        let mut dest_path = self.output_directory.join(new_name);
        dest_path.pop();
        let mut found = false;
        let upper_name = from_file.file_name().unwrap().to_str().unwrap().to_ascii_uppercase();

        if let Some(parent) = from_file.parent() {
            for entry in fs::read_dir(parent)?.flatten() {
                if !entry.path().is_file() {
                    continue;
                }
                let found_name = entry.file_name().to_str().unwrap().to_ascii_uppercase();
                if found_name.starts_with(&upper_name) {
                    found = true;
                    let mut dest = dest_path.to_path_buf();
                    dest.push(entry.file_name().to_ascii_lowercase());
                    if dest.exists() {
                        // already handled.
                        continue;
                    }

                    if !found_name.ends_with(".PPS") && (/*found_name.ends_with(".PPE") ||*/found_name.contains('.')) {
                        if found_name.ends_with(".MNU") {
                            let imported_menu = Menu::import_pcboard(&entry.path())?;
                            if dest.exists() {
                                fs::remove_file(&dest)?;
                            }
                            imported_menu.save(&dest)?;
                            self.converted_files
                                .insert(entry.path().display().to_string().to_ascii_uppercase(), dest.display().to_string());
                            self.logger.translated_file(&entry.path(), &dest);
                        } else {
                            self.output
                                .start_action(format!("\t copy '{}' to '{}'…", entry.path().display(), dest.display()));
                            self.output.check_error(fs::copy(entry.path(), &dest).err())?;
                            self.logger.copy_file(&entry.path(), &dest);
                        }
                    } else {
                        self.import_and_scan_file(&entry.path(), &dest)?;
                    }
                }
            }
        }
        if !found {
            self.logger
                .log(&format!("Warning: Searched for {}, but didn't find any matching file.", upper_name));
        }

        self.converted_files.insert(upper_file_name.clone(), new_name.to_string());
        Ok(PathBuf::from(new_name))
    }

    fn convert_user_base(&mut self, usr_file: &str, inf_file: &str, new_rel_name: &str) -> Res<PathBuf> {
        self.output.start_action("Convert user base…".into());
        let usr_file = self.resolve_file(usr_file);

        let mut user_base = if !usr_file.exists() {
            self.logger.log(&format!("Can't find user file {}", usr_file.display()));
            UserBase::default()
        } else {
            let inf_file = self.resolve_file(inf_file);
            if !inf_file.exists() {
                self.logger.log(&format!("Can't find user information file {}", inf_file.display()));
                return Ok(PathBuf::new());
            }
            let users = PcbUserRecord::read_users(&PathBuf::from(&usr_file))?;
            let user_inf = PcbUserInf::read_users(&PathBuf::from(&inf_file))?;

            UserBase::import_pcboard(
                &users
                    .iter()
                    .map(|u| PcbUser {
                        user: u.clone(),
                        inf: user_inf[u.rec_num as usize].clone(),
                    })
                    .collect::<Vec<PcbUser>>(),
            )
        };
        let destination: PathBuf = self.output_directory.join(new_rel_name);
        if user_base.is_empty() {
            self.logger.log(&format!("User base empty, generating sysop."));

            let mut user = User {
                name: "SYSOP".to_string(),
                password: PasswordInfo {
                    password: Password::PlainText("".to_string()),
                    ..Default::default()
                },
                page_len: 23,
                security_level: 110,
                ..Default::default()
            };
            user.stats.first_date_on = chrono::Utc::now();
            user_base.new_user(user);
        }
        user_base.save(&destination)?;
        Ok(PathBuf::from(new_rel_name))
    }

    fn copy_display_directory<F: Fn(&Path) -> bool>(&mut self, category: &str, dir_loc: &str, rel_output: &str, filter: F) -> Res<()> {
        self.logger.log(&format!("\n=== Converting {} ===", category));
        if dir_loc.is_empty() {
            self.logger.log(&format!("\ndir wasn't set."));
            return Ok(());
        }
        let help_loc = self.resolve_file(dir_loc);
        let help_loc = PathBuf::from(&help_loc);
        if !help_loc.is_dir() {
            self.logger.log(&format!("\ndir {} doesn't exist", help_loc.display()));
            return Ok(());
        }

        let o = self.output_directory.join(rel_output);
        if help_loc.exists() {
            self.output
                .start_action(format!("Copy {} from {} to {}…", category, help_loc.display(), o.display()));
            for entry in WalkDir::new(&help_loc) {
                let entry = entry?;
                if entry.path().is_dir() {
                    continue;
                }
                if !filter(entry.path()) {
                    continue;
                }
                let rel_path: RelativePathBuf = entry.path().relative_to(&help_loc).unwrap();
                let lower_case = RelativePathBuf::from_path(rel_path.as_str().to_lowercase()).unwrap();
                let to = lower_case.to_logical_path(&o);
                if let Some(parent_dir) = to.parent() {
                    if !parent_dir.exists() {
                        if let Err(err) = fs::create_dir(parent_dir) {
                            self.logger.log(&format!("Can't create directory {}:", parent_dir.display()));
                            self.logger.log_error(Some(err))?;
                            self.output.warning(format!("Can't create directory {}", parent_dir.display()));
                            continue;
                        }
                    }
                }
                self.import_and_scan_file(&entry.path(), &to)?;
            }
        }

        Ok(())
    }

    fn copy_attachment_directory(&mut self, output: &str, attachment_location: &Path) -> Res<PathBuf> {
        let new_rel_name = PathBuf::from(output.to_string());

        let Some(attach_dir) = attachment_location.file_name() else {
            return Ok(new_rel_name);
        };

        let resolved_file = self.resolve_file(attach_dir.to_str().unwrap());
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let destination = self.output_directory.join(output);
        if !destination.exists() {
            fs::create_dir(&destination)?;
        }

        self.logger
            .log(&format!("\n=== Copy attachments {} -> {} ===", resolved_file.display(), destination.display()));

        if resolved_file.is_dir() {
            self.output
                .start_action(format!("Copy attachments from {} to {}…", resolved_file.display(), output));
            for entry in WalkDir::new(&resolved_file) {
                let entry = entry?;
                if entry.path().is_dir() {
                    continue;
                }
                let rel_path = entry.path().relative_to(&resolved_file).unwrap();
                let to = rel_path.to_logical_path(&destination);
                if let Some(parent_dir) = to.parent() {
                    if !parent_dir.exists() {
                        fs::create_dir(parent_dir).unwrap();
                    }
                }
                fs::copy(entry.path(), &to)?;
                self.logger.copy_file(entry.path(), &to);
            }
        }
        self.converted_files.insert(upper_file_name.clone(), new_rel_name.to_str().unwrap().to_string());

        Ok(new_rel_name)
    }

    fn convert_data<T: PCBoardImport>(&mut self, file: &str, new_rel_name: &str) -> Res<PathBuf> {
        if file.is_empty() {
            return Ok(PathBuf::from(new_rel_name));
        }

        let resolved_file = self.resolve_file(file);
        self.output.start_action(format!("Convert {}…", resolved_file.display()));
        let resolved_file = PathBuf::from(&resolved_file);
        let res = if resolved_file.exists() {
            T::import_pcboard(&resolved_file)?
        } else {
            T::default()
        };
        let destination = self.output_directory.join(new_rel_name);
        if let Err(err) = res.save(&destination) {
            self.logger.log_boxed_error(&*err);
        }
        self.logger.log("");

        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_default_cmd_lst(&mut self, file: &str, new_rel_name: &str) -> Res<PathBuf> {
        let res = if file.is_empty() {
            CommandList::new()
        } else {
            let resolved_file = self.resolve_file(file);
            let resolved_file = PathBuf::from(&resolved_file);
            if resolved_file.exists() {
                let mut res = CommandList::import_pcboard(&resolved_file)?;

                for cmd in res.iter_mut() {
                    for act in cmd.actions.iter_mut() {
                        if act.command_type == CommandType::RunPPE {
                            let mut line = self.scan_line_for_commands(&format!("!{}", act.parameter), 0).unwrap();
                            line.remove(0);
                            act.parameter = line;
                        }
                    }
                }

                res.commands.extend_from_slice(&CommandList::new().commands);
                res
            } else {
                CommandList::new()
            }
        };

        let destination = self.output_directory.join(new_rel_name);
        if let Err(err) = res.save(&destination) {
            return Err(Box::new(IcyBoardError::ErrorCreatingFile(new_rel_name.to_string(), err.to_string())));
        }
        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_message_base(&mut self, dest_path: &Path, output: &str, src_file: &Path) -> Res<PathBuf> {
        if src_file.to_str().unwrap().is_empty() {
            return Ok(PathBuf::new());
        }
        let resolved_file = self.resolve_file(src_file.file_name().unwrap().to_str().unwrap());
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        if !resolved_file.is_file() {
            self.converted_files.insert(upper_file_name.clone(), String::new());
            self.logger.log(&format!("Can't find message base {}", resolved_file.display()));
            self.output.warning(format!("Can't find message base {}", resolved_file.display()));
            return Ok(PathBuf::new());
        }

        self.output.start_action(format!("Convert message base {}…", resolved_file.display()));
        let destination = dest_path.join(resolved_file.file_name().unwrap().to_ascii_lowercase());

        jamjam::conversion::convert_pcboard_to_jam(&resolved_file, &destination, &EchomailAddress::default())?;

        self.logger
            .log(&format!("Converted message base {} -> {}", resolved_file.display(), destination.display()));
        let new_rel_name = PathBuf::from(output.to_string().to_lowercase()).join(resolved_file.file_name().unwrap().to_ascii_lowercase());
        self.converted_files.insert(upper_file_name.clone(), new_rel_name.to_str().unwrap().to_string());
        Ok(new_rel_name)
    }

    fn convert_bullettin_file(&mut self, dest_path: &Path, output: &str, src_file: &Path) -> Res<PathBuf> {
        self.logger.log(&format!("\n=== Converting BLT.LST {} ===", src_file.display()));

        if src_file.to_str().unwrap().is_empty() {
            return Ok(PathBuf::new());
        }
        let resolved_file = self.resolve_file(src_file.file_name().unwrap().to_str().unwrap());
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let Ok(mut list) = BullettinList::import_pcboard(&resolved_file) else {
            self.logger.log(&format!("Warning, can't import bulletin  {}", resolved_file.display()));
            self.output.warning(format!("Warning, can't import bulletin {}", resolved_file.display()));
            return Ok(resolved_file);
        };
        let resolved_file = resolved_file.with_extension("toml");

        let destination = PathBuf::from(dest_path).join(resolved_file.file_name().unwrap().to_ascii_lowercase());

        for entry in list.iter_mut() {
            let new_entry = self.resolve_file(entry.file.to_str().unwrap());
            if !new_entry.exists() {
                self.logger
                    .log(&format!("Warning, can't import bulletin  {}, doesn't exist.", new_entry.display()));
                continue;
            }

            let name = new_entry.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
            let new_name = format!("{}/{}", output, &name);

            match self.convert_display_file(new_entry.to_str().unwrap(), &new_name) {
                Ok(new_name) => {
                    entry.file = new_name;
                }
                Err(err) => {
                    self.logger.log_boxed_error(&*err);
                }
            } /*
            } else {
            self.logger.log(&format!(
            "Warning, can't resolve bulletin entry {} in file {}",
            entry.file.display(),
            destination.display()
            ));
            self.output.warning(format!("Warning, can't resolve {}", entry.file.display()));
            }*/
        }

        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        list.save(&destination)?;
        self.logger.log(&format!("Wrote bulletin to {}", destination.display()));

        let name = resolved_file.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
        let new_name = PathBuf::from(format!("{}/{}", output, &name));
        self.converted_files.insert(upper_file_name.clone(), new_name.to_string_lossy().to_string());
        Ok(new_name)
    }

    fn convert_questionnaires(&mut self, dest_path: &Path, output: &str, src_file: &Path) -> Res<PathBuf> {
        self.logger.log(&format!("\n=== Converting Script Questionnaires {} ===", src_file.display()));

        if src_file.to_str().unwrap().is_empty() {
            return Ok(PathBuf::new());
        }
        let resolved_file = self.resolve_file(src_file.file_name().unwrap().to_str().unwrap());
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let Ok(mut list) = SurveyList::import_pcboard(&resolved_file) else {
            self.logger
                .log(&format!("Warning, can't import script questionnaires {}", resolved_file.display()));
            self.output
                .warning(format!("Warning, can't import script questionnaires {}", resolved_file.display()));
            return Ok(resolved_file);
        };
        let resolved_file = resolved_file.with_extension("toml");

        let destination = PathBuf::from(dest_path).join(resolved_file.file_name().unwrap().to_ascii_lowercase());

        for entry in list.iter_mut() {
            let new_entry = self.resolve_file(entry.survey_file.to_str().unwrap());
            if !new_entry.exists() {
                self.logger
                    .log(&format!("Warning, can't import questionaire  {}, doesn't exist.", new_entry.display()));
                continue;
            }
            let name = new_entry.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
            let new_name = format!("{}/{}", output, &name);
            match self.convert_display_file(new_entry.to_str().unwrap(), &new_name) {
                Ok(new_rel_name) => {
                    // Add a separator line after the first 5 lines of the question file
                    if new_rel_name.extension().unwrap_or_default().to_string_lossy() != "ppe" {
                        let full_path = self.output_directory.join(&new_rel_name);

                        if let Ok(str) = read_with_encoding_detection(&full_path) {
                            let mut out = String::new();
                            for (i, line) in str.lines().enumerate() {
                                if i == 5 {
                                    out.push_str("**************************************************************");
                                    out.push('\n');
                                }
                                out.push_str(line);
                                out.push('\n');
                            }
                            fs::write(&full_path, out)?;
                        }
                    }

                    entry.survey_file = new_rel_name;
                }
                Err(err) => {
                    self.logger.log_boxed_error(&*err);
                }
            } /*
            } else {
            self.logger.log(&format!(
            "Warning, can't resolve script questionary {} in file {}",
            entry.question_file.display(),
            destination.display()
            ));
            self.output.warning(format!("Warning, can't resolve {}", entry.question_file.display()));
            }
             */
            let new_entry = self.resolve_file(entry.answer_file.to_str().unwrap());
            let name = new_entry.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
            let new_name = format!("{}/{}", output, &name);
            match self.convert_display_file(new_entry.to_str().unwrap(), &new_name) {
                Ok(new_name) => {
                    entry.answer_file = new_name;
                }
                Err(err) => {
                    self.logger.log_boxed_error(&*err);
                }
            }
            /*
            } else {
                self.logger.log(&format!(
                    "Warning, can't resolve script questionary {} in file {}",
                    entry.answer_file.display(),
                    destination.display()
                ));
                self.output.warning(format!("Warning, can't resolve {}", entry.answer_file.display()));
            }*/
        }

        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        list.save(&destination)?;
        self.logger.log(&format!("Wrote survey to {}", destination.display()));

        let name = resolved_file.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
        let new_name = PathBuf::from(format!("{}/{}", output, &name));
        self.converted_files.insert(upper_file_name.clone(), new_name.to_string_lossy().to_string());
        Ok(new_name)
    }

    fn convert_dirlist_file(&mut self, dest_path: &Path, output: &str, src_file: &Path) -> Res<PathBuf> {
        self.logger.log(&format!("\n=== Converting DIR.LST {} ===", src_file.display()));

        if src_file.to_str().unwrap().is_empty() {
            self.logger.log(&format!("Original file not defined: {}", src_file.display()));
            return Ok(PathBuf::new());
        }
        let resolved_file = self.resolve_file(src_file.file_name().unwrap().to_str().unwrap());
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let Ok(mut list) = DirectoryList::import_pcboard(&resolved_file) else {
            self.logger.log(&format!("Warning, can't import dir.lst file {}", resolved_file.display()));
            self.output.warning(format!("Warning, can't import dir.lst file {}", resolved_file.display()));
            return Ok(resolved_file);
        };
        let resolved_file = resolved_file.with_extension("toml");

        let destination = PathBuf::from(dest_path).join(resolved_file.file_name().unwrap().to_ascii_lowercase());

        for (i, entry) in list.iter_mut().enumerate() {
            entry.path = self.resolve_file(entry.path.to_str().unwrap());
            let base_path = PathBuf::from(dest_path).join(format!("dir{:02}", i));
            self.logger
                .log(&format!("Create file base for {} : {}", entry.path.display(), base_path.display()));
        }

        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        list.save(&destination)?;
        self.logger.log(&format!("Wrote file area to {}", destination.display()));

        let name = resolved_file.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
        let new_name = PathBuf::from(format!("{}/{}", output, &name));
        self.converted_files.insert(upper_file_name.clone(), new_name.to_string_lossy().to_string());
        Ok(new_name)
    }

    fn convert_doors_file(&mut self, dest_path: &Path, output: &str, src_file: &Path) -> Res<PathBuf> {
        self.logger.log(&format!("\n=== Converting DOORS.LST {} ===", src_file.display()));

        if src_file.to_str().unwrap().is_empty() {
            return Ok(PathBuf::new());
        }
        let resolved_file = self.resolve_file(src_file.file_name().unwrap().to_str().unwrap());
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let Ok(list) = DoorList::import_pcboard(&resolved_file) else {
            self.logger.log(&format!("Warning, can't import bulletin  {}", resolved_file.display()));
            self.output.warning(format!("Warning, can't import bulletin {}", resolved_file.display()));
            return Ok(resolved_file);
        };
        let resolved_file = resolved_file.with_extension("toml");

        let destination = PathBuf::from(dest_path).join(resolved_file.file_name().unwrap().to_ascii_lowercase());
        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        list.save(&destination)?;
        self.logger.log(&format!("Wrote bulletin to {}", destination.display()));

        let name = resolved_file.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
        let new_name = PathBuf::from(format!("{}/{}", output, &name));
        self.converted_files.insert(upper_file_name.clone(), new_name.to_string_lossy().to_string());
        Ok(new_name)
    }
    fn create_file(&self, include_str: &str, new_name: &str) -> Res<PathBuf> {
        fs::write(self.output_directory.join(new_name), include_str)?;
        Ok(PathBuf::from(new_name))
    }

    fn convert_logon_surveys(&mut self, source: &str, arg: &str) -> Res<PathBuf> {
        if source.is_empty() {
            return Ok(PathBuf::new());
        }

        let new_entry = self.resolve_file(source);
        self.output.start_action(format!("Trying to import questionaire {}", new_entry.display()));

        if !new_entry.exists() {
            self.logger
                .log(&format!("Warning, can't import questionaire  {}, doesn't exist.", new_entry.display()));
            return Ok(PathBuf::from(arg));
        }
        if let Ok(str) = read_with_encoding_detection(&new_entry) {
            let mut out = String::new();
            // Add a separator line at beginning
            // The login/logoff surveys don't seem to have headers in PCBoard but in IcyBoard they do.
            out.push_str("**************************************************************");
            out.push('\n');
            for line in str.lines() {
                out.push_str(line);
                out.push('\n');
            }
            let new_name = self.output_directory.join(arg);
            write_utf8_with_bom(&new_name, &out)?;
            self.logger.log(&format!("Wrote logon survey to {}", new_name.display()));
        }

        Ok(PathBuf::from(arg))
    }

    fn convert_accounting_cfg(&mut self, source: &str, new_rel_name: &str) -> Res<PathBuf> {
        if source.is_empty() {
            return Ok(PathBuf::new());
        }

        let src_file = PathBuf::from(source);
        let resolved_file = self.resolve_file(src_file.file_name().unwrap().to_str().unwrap());
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }
        self.logger.log(&format!("\n=== Converting Accounting {} ===", source));

        let destination = self.output_directory.join(new_rel_name);
        if let Ok(list) = AccountingConfig::import_pcboard(&resolved_file) {
            list.save(&destination)?;
        } else {
            self.logger.log(&format!("Warning, can't import accounting {}", resolved_file.display()));
            self.output.warning(format!("Warning, can't import accounting {}", resolved_file.display()));
            let list = AccountingConfig::default();
            list.save(&destination)?;
        }
        Ok(PathBuf::from(new_rel_name))
    }
}
