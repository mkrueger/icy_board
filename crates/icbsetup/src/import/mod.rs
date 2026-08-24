use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
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
        events::{BoardEvent, EventList, EventMode},
        file_directory::DirectoryList,
        ftn::{FtnConfig, FtnOptions},
        group_list::GroupList,
        icb_config::{
            BoardInformation, BoardOptions, ColorConfiguration, ConfigPaths, DEFAULT_PCBOARD_DATE_FORMAT, DisplayNewsBehavior, IcbColor, IcbConfig,
            NewUserSettings, PasswordStorageMethod, QwkSettings, SubscriptionMode, SysopCommandLevels, SysopInformation,
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
        lookup_case_insensitive,
        security_expr::SecurityExpression,
        user_base::{PasswordInfo, User},
    },
};
use jamjam::util::echomail::EchomailAddress;
use relative_path::{PathExt, RelativePathBuf};
use walkdir::WalkDir;

use self::import_log::ImportLog;
use std::fmt::Write as _;

pub mod console_logger;
pub mod import_log;

pub trait OutputLogger {
    fn start_action(&self, message: String);
    fn check_error(&self, res: Option<std::io::Error>) -> Res<()>;
    fn warning(&self, message: String);
}

/// Art and RIP lines start with the same character as a PPE call, so only a plausible name is followed.
fn is_dos_path(file: &str) -> bool {
    !file.is_empty() && file.len() <= 128 && !file.chars().any(|ch| ch.is_control() || matches!(ch, '|' | '<' | '>' | '"' | '*' | '?' | '@'))
}

fn imported_metadata_path(output: &str, index: usize) -> PathBuf {
    PathBuf::from(output).join(format!("dir{index:02}"))
}

fn unresolved_configured_path_warning(file: &str, resolved_file: &Path) -> String {
    format!(
        "Can't resolve configured path '{}' (looked for '{}'); keeping the original value.",
        file,
        resolved_file.display()
    )
}

/// A `X:\…` path starting at `start`, ending where a delimiter or the line does.
fn dos_path_at(data: &[u8], start: usize) -> Option<usize> {
    if start + 3 > data.len() || !data[start].is_ascii_alphabetic() || data[start + 1] != b':' || data[start + 2] != b'\\' {
        return None;
    }
    if start > 0 && data[start - 1].is_ascii_alphanumeric() {
        return None;
    }
    let mut end = start + 3;
    while end < data.len() {
        let ch = data[end];
        if !(0x21..=0x7e).contains(&ch) || matches!(ch, b'"' | b'\'' | b',' | b';' | b'|' | b'*' | b'?' | b'<' | b'>') {
            break;
        }
        end += 1;
    }
    Some(end)
}

/// A whole PCBoard installation may be given instead of its PCBOARD.DAT.
fn locate_pcboard_dat(path: &Path) -> Res<PathBuf> {
    if !path.is_dir() {
        return Ok(path.to_path_buf());
    }
    let in_root = lookup_case_insensitive(&path.join("PCBOARD.DAT"));
    if in_root.is_file() {
        return Ok(in_root);
    }
    let found = WalkDir::new(path)
        .max_depth(2)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file() && entry.file_name().to_string_lossy().eq_ignore_ascii_case("pcboard.dat"))
        .map(|entry| entry.into_path())
        .next();
    match found {
        Some(found) => Ok(found),
        None => Err(Box::new(IcyBoardError::Error(format!(
            "No PCBOARD.DAT found in {} - point the import at the file itself.",
            path.display()
        )))),
    }
}

#[derive(Default)]
pub struct ImportStats {
    pub conferences: usize,
    pub users: usize,
    pub users_without_inf: usize,
    pub message_bases: usize,
    pub ppes: usize,
    pub converted_files: usize,
}

pub struct PCBoardImporter {
    pub output: Box<dyn OutputLogger>,
    pub data: PcbBoardData,
    pub output_directory: PathBuf,
    pub logger: ImportLog,

    /// Directory PCBOARD.DAT was read from.
    pub source_directory: PathBuf,

    /// Contains paths to map dos paths to unix paths
    /// For example:
    /// 'C:\' -> '/home/user/pcboard'
    /// Difference to map_paths is that this maps source paths to other source paths.
    pub resolve_paths: HashMap<String, String>,

    pub converted_files: HashMap<String, String>,

    pub stats: ImportStats,

    /// DOS paths nothing was found for - resolving happens behind &self.
    unresolved: RefCell<BTreeSet<String>>,
}

fn read_mapped_source_directory(parent: &Path, configured_path: &str, resolved_path: &Path) -> Res<fs::ReadDir> {
    fs::read_dir(parent).map_err(|err| {
        Box::new(IcyBoardError::Error(format!(
            "Can't read the PCBoard source directory '{}' while resolving '{}' to '{}': {err}. Verify that --map points to the existing PCBoard installation, for example --map 'D:\\PCB=/path/to/pcb'.",
            parent.display(),
            configured_path,
            resolved_path.display()
        ))) as _
    })
}

impl PCBoardImporter {
    pub fn new(file_name: &Path, output: Box<dyn OutputLogger>, output_directory: PathBuf, mappings: &[(String, String)]) -> Res<Self> {
        let file_name = locate_pcboard_dat(file_name)?;
        match PcbBoardData::import_pcboard(&file_name) {
            Ok(data) => {
                let mut paths = HashMap::new();
                for (dos_path, local_path) in mappings {
                    let mut dos_path = dos_path.replace('\\', "/");
                    while dos_path.ends_with('/') {
                        dos_path.pop();
                    }
                    paths.insert(dos_path, local_path.trim_end_matches('/').to_string());
                }

                let file_path = file_name.clone();
                let mut help = data.path.help_loc.clone();
                if help.ends_with('\\') {
                    help.pop();
                }

                help = help.replace('\\', "/");

                let help_loc = PathBuf::from(&help);
                let path = file_path.parent().unwrap().to_path_buf();

                let upper = path.to_string_lossy().to_ascii_uppercase();

                let source_directory = path.clone();
                output.start_action(format!(
                    "Importing PCBoard {} from base path {}\n",
                    file_path.file_name().unwrap_or_default().to_string_lossy(),
                    source_directory.display()
                ));

                if !lookup_case_insensitive(&path.join(help_loc.file_name().unwrap_or_default())).is_dir() {
                    output.warning(format!(
                        "'{}' not found below {} - assuming the installation was copied into one directory.",
                        data.path.help_loc,
                        source_directory.display()
                    ));
                }

                //let len = to_str().unwrap().len();
                if let Some(k) = help_loc.parent()
                    && let Some(v) = file_path.parent()
                {
                    let k = k.to_str().unwrap_or_default().to_string();
                    let v = v.to_path_buf().to_str().unwrap_or_default().to_string();
                    paths.entry(k).or_insert(v);
                }

                // A board below C:\PCB has the rest of the drive next to it, so C: is worth a guess.
                if let Some((drive, _)) = help.split_once(':')
                    && let Some(drive_root) = source_directory.parent()
                {
                    paths.entry(format!("{}:", drive)).or_insert(drive_root.to_string_lossy().to_string());
                }

                let mut map_paths = HashMap::new();
                map_paths.insert(upper.clone() + "\\PPE", output_directory.join("ppe"));
                map_paths.insert(upper.clone(), output_directory.clone());

                Ok(Self {
                    output,
                    data,
                    output_directory,
                    source_directory,
                    resolve_paths: paths,
                    logger: ImportLog::default(),
                    converted_files: HashMap::new(),
                    stats: ImportStats::default(),
                    unresolved: RefCell::new(BTreeSet::new()),
                })
            }
            Err(err) => Err(Box::new(IcyBoardError::Error(format!("Error reading PCBoard data: {}", err)))),
        }
    }

    pub fn resolve_file(&self, file: &str) -> PathBuf {
        let resolved = self.try_resolve_file(file);
        if !resolved.exists() && !file.trim().is_empty() {
            self.unresolved.borrow_mut().insert(file.to_string());
        }
        resolved
    }

    /// Conference lists name their files with the DOS path they had - if that leads nowhere the name alone still may.
    pub fn resolve_listed_file(&self, file: &Path) -> PathBuf {
        let full = self.try_resolve_file(&file.to_string_lossy());
        if full.exists() {
            return full;
        }
        if let Some(name) = file.file_name() {
            let by_name = self.try_resolve_file(&name.to_string_lossy());
            if by_name.exists() {
                return by_name;
            }
        }
        if !file.as_os_str().is_empty() {
            self.unresolved.borrow_mut().insert(file.to_string_lossy().to_string());
        }
        full
    }

    pub fn unresolved_paths(&self) -> Vec<String> {
        self.unresolved.borrow().iter().cloned().collect()
    }

    fn try_resolve_file(&self, file: &str) -> PathBuf {
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
            if let Some(drive) = self.resolve_paths.keys().find(|key| key.len() == 2 && key.ends_with(':')) {
                s = format!("{}{}", drive, s);
            } else if let Some(path) = self.resolve_paths.values().next() {
                s = format!("{}/..{}", path, s);
            }
        }

        // C:\PCB and C: both map somewhere, the longer one is the one that was meant.
        let mapping = self
            .resolve_paths
            .iter()
            .filter(|(k, _)| s.to_ascii_uppercase().starts_with(&k.to_ascii_uppercase()))
            .max_by_key(|(k, _)| k.len());
        if let Some((k, v)) = mapping {
            let rest = &s[k.len()..];
            s = if rest.is_empty() || rest.starts_with('/') || v.ends_with('/') {
                v.clone() + rest
            } else {
                v.clone() + "/" + rest
            };
        }

        let mapped = PathBuf::from(&s);
        let resolved = lookup_case_insensitive(&mapped);
        if resolved.exists() {
            return resolved;
        }
        for candidate in self.name_variants(&mapped) {
            let candidate = lookup_case_insensitive(&candidate);
            if candidate.exists() {
                return candidate;
            }
        }
        resolved
    }

    /// Where PCBoard names a file it may mean the same name with a display extension, a prompt
    /// suffix like `_` or `~`, or - on installations copied into one directory - the board directory.
    fn name_variants(&self, mapped: &Path) -> Vec<PathBuf> {
        let Some(name) = mapped.file_name().map(|n| n.to_string_lossy().to_string()) else {
            return Vec::new();
        };
        let parent = mapped.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let trimmed = name.trim_end_matches(['_', '~', '.', ' ']).to_string();

        let mut names = vec![name.clone()];
        if trimmed != name && !trimmed.is_empty() {
            names.push(trimmed.clone());
        }
        if !trimmed.contains('.') {
            for ext in ["pcb", "ans", "asc", "rip"] {
                names.push(format!("{}.{}", trimmed, ext));
            }
        }

        let mut variants = Vec::new();
        // Relative names are relative to the board directory, not to where icbsetup was started.
        if mapped.is_relative() {
            variants.push(self.source_directory.join(mapped));
            if let Some(dir) = mapped.parent() {
                variants.push(self.source_directory.join(dir).join(&trimmed));
            }
        }
        for name in names {
            if parent != self.source_directory {
                variants.push(parent.join(&name));
            }
            variants.push(self.source_directory.join(&name));
        }
        variants.retain(|variant| variant != mapped);
        variants
    }

    pub fn start_import(&mut self) -> Res<()> {
        self.create_directories()?;

        self.copy_display_directory("help files", &self.data.path.help_loc.clone(), "art/help", Some("HLP"), |_| true)?;
        self.copy_display_directory(
            "commmand display files",
            &self.data.path.cmd_display_files_loc.clone(),
            "art/cmd_display",
            None,
            |_| true,
        )?;
        self.copy_display_directory("security files", &self.data.path.sec_loc.clone(), "art/secmsgs", None, |p| {
            let file_name = p.file_name().unwrap().to_str().unwrap();
            file_name.chars().next().unwrap_or_default().is_ascii_digit()
        })?;

        let icbtext = self.convert_pcbtext(&(self.data.path.text_loc.clone() + "/PCBTEXT"), "main/icbtext")?;
        let trashcan_user = self.convert_trashcan(&self.data.path.tcan_file.clone(), "main/tcan_user.txt")?;
        let trashcan_upload_files = self.convert_trashcan(&self.data.path.file_tcan.clone(), "main/tcan_uploads.txt")?;
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
        let transfer_log = self.convert_display_file(&self.data.path.download_file.clone(), "transfer.log")?;

        let accounting_config_file = self.convert_accounting_cfg(&self.data.account_config.clone(), "main/accounting_cfg.toml")?;
        let accounting_holiday_list_file = self.convert_display_file(&self.data.holidays_file.clone(), "art/acc_holidays")?;
        let accounting_info_file = self.convert_display_file(&self.data.account_info.clone(), "art/acc_info")?;
        let accounting_warning_file = self.convert_display_file(&self.data.account_warn.clone(), "art/acc_warn")?;
        let accounting_logoff_file = self.convert_display_file(&self.data.account_logoff.clone(), "art/acc_logoff")?;

        let conf_join_menu = self.convert_display_file(&self.data.path.conf_menu.clone(), "art/cnfn")?;
        let group_chat = self.convert_display_file(&self.data.path.group_chat.clone(), "art/group")?;
        let chat_menu = self.convert_display_file(&self.data.path.chat_menu.clone(), "art/chtm")?;
        let chat_actions_menu = self.convert_display_file(&self.data.path.chat_actions.clone(), "art/chatactm")?;

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
            color_configuration.file_duplicate = IcbColor::Dos(color_file[start + 12]);
            color_configuration.file_deleted = IcbColor::Dos(color_file[start + 14]);
            color_configuration.file_offline = IcbColor::Dos(color_file[start + 16]);
            color_configuration.file_new_file = IcbColor::Dos(color_file[start + 18]);
        }

        let mut icb_cfg = IcbConfig {
            sysop: SysopInformation {
                name: self.data.sysop_info.sysop.clone(),
                password: Password::new_argon2(self.data.sysop_info.password.as_str()),
                require_password_to_exit: self.data.sysop_info.require_pwrd_to_exit,
                use_real_name: self.data.sysop_info.use_real_name,
                external_editor: "nano".to_string(),
                graphics_editor: "icy_draw".to_string(),
                config_color_theme: "DEFAULT1".to_string(),
                config_color_configuration: Default::default(),
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
                sec_13_view_alt_node_callers: SecurityExpression::from_req_security(self.data.sysop_security.sec_13_view_alt_node_callers as u8),
                sec_14_drop_alt_node_to_dos: SecurityExpression::from_req_security(self.data.sysop_security.sec_14_drop_alt_node_to_dos as u8),
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
                web_admin: Default::default(),
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
                transfer_log,
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
                ftn_file: if self.data.enable_fido {
                    PathBuf::from("main/ftn.toml")
                } else {
                    PathBuf::new()
                },

                trashcan_upload_files,
                trashcan_user,
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
                auto_register_conferences: self.data.auto_reg_conf,
            },
            message: MessageOptions {
                max_msg_lines: self.data.max_msg_lines as u16,
                scan_all_mail_at_login: self.data.scan_all,
                prompt_to_read_mail: self.data.prompt_to_read_mail,
                disable_message_scan_prompt: self.data.disable_scan,
                allow_esc_codes: self.data.allow_esc_codes,
                allow_carbon_copy: self.data.allow_ccs,
                validate_to_name: self.data.validate_to,
                default_quick_personal_scan: self.data.quick_scan,
                default_scan_all_selected_confs_at_login: self.data.scan_all,
                force_comments_to_main: self.data.force_main,
                update_last_read_pointer: self.data.last_read_update,
            },
            file_transfer: FileTransferOptions {
                verify_files_uploaded: self.data.test_uploads,
                display_uploader: self.data.upload_by,
                upload_descr_lines: self.data.num_ul_desc_lines as u8,
                // PCBoard listed whatever the DIR file held, so an import keeps it.
                strip_colors_in_descriptions: false,
                disallow_batch_uploads: self.data.no_batch_up,
                promote_to_batch_transfers: self.data.promote_batch,
                upload_credit_time: self.data.upload_credit.max(0) as u32,
                upload_credit_bytes: self.data.byte_credit.max(0) as u32,
                disable_drive_size_check: self.data.disable_drive_check,
                stop_uploads_free_space: self.data.stop_free_space.max(0) as u32,
            },
            system_control: SystemControlOptions {
                disable_ns_logon: self.data.disable_quick,
                disable_full_record_updating: self.data.allow_pwrd_only,
                is_closed_board: self.data.closed_board,
                guard_logoff: self.data.guard_logoff,
                allow_alias_change: self.data.allow_alias_change,
                is_multi_lingual: self.data.multi_lingual,
                enforce_daily_time_limit: self.data.enforce_time,
                allow_password_failure_comment: self.data.allow_pwrd_comment,
                password_storage_method: PasswordStorageMethod::default(),
                confirm_caller_name: self.data.confirm_caller,
                reread_sec_level_on_join: self.data.conf_pwrd_adjust,
                // The limits come across from PWRD, but the sysop has to turn enforcement on.
                enforce_transfer_limits: false,
            },
            switches: ConfigSwitches {
                display_news_behavior: DisplayNewsBehavior::from_pcb_char(self.data.display_news),
                exclude_local_calls_stats: self.data.exclude_locals,
                display_userinfo_at_login: self.data.display_userinfo_at_login,
                non_graphics: self.data.non_graphics,
                disable_registration_edits: self.data.disable_edits,
                disable_high_ascii_filter: self.data.disable_filter,
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
                log_caller_number: self.data.log_caller_number,
                log_connect_string: self.data.log_connect_str,
                log_security_level: self.data.log_sec_level,
            },
            event: EventOptions {
                enabled: self.data.event_active,
                event_file: PathBuf::from("main/events.toml"),
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
            qwk_settings: QwkSettings {
                bbs_name: self.data.board_name.clone(),
                bbs_sysop_name: self.data.sysop_info.sysop.clone(),
                // PCBoard falls back to CapFile when QwkFile is empty, see getqwkroot().
                bbs_id: if self.data.qwk_file.trim().is_empty() {
                    self.data.cap_file.trim().to_string()
                } else {
                    self.data.qwk_file.trim().to_string()
                },
                max_msgs: self.data.max_total_msgs.clamp(0, u16::MAX as i32) as u16,
                max_msgs_per_conf: self.data.max_conf_msgs.clamp(0, u16::MAX as i32) as u16,
                ..QwkSettings::default()
            },
        };
        icb_cfg.board.allow_iemsi = false;
        icb_cfg.login_server.telnet.port = 1337;
        icb_cfg.login_server.ssh.port = 1338;

        // PCBOARD.DAT only knows the one daily event, EVENT.DAT is not read.
        let mut events = EventList::default();
        if !self.data.event_time.trim().is_empty() {
            events.push(BoardEvent {
                description: "Imported from PCBOARD.DAT".to_string(),
                time: IcbTime::parse(&format!("{}:00", self.data.event_time.trim())),
                mode: if self.data.event_slide { EventMode::Slide } else { EventMode::Fixed },
                ..Default::default()
            });
        }
        let event_path = self.output_directory.join(&icb_cfg.event.event_file);
        if let Some(parent) = event_path.parent() {
            fs::create_dir_all(parent)?;
        }
        events.save(&event_path)?;

        // The addresses and the links live in the files under FidoLoc, which
        // are in no documented format; what PCBOARD.DAT holds is the set of
        // decisions the tosser makes, and those carry over.
        if self.data.enable_fido {
            let ftn = FtnConfig {
                options: FtnOptions {
                    process_in: self.data.fido_process_in,
                    process_out: self.data.fido_process_out,
                    process_orphan: self.data.fido_process_orphan,
                    dial_out: self.data.fido_dial_out,
                    import_after_xfer: self.data.fido_import_after_xfer,
                    check_dupe_msg_id: self.data.fido_check_dupe_msg_id,
                    check_dupe_path: self.data.fido_check_dupe_path,
                    msgs_to_track: self.data.fido_num_msgs_to_track.max(0) as u32,
                    secure: self.data.fido_secure,
                    sysop_change: self.data.fido_sysop_change,
                    auto_add: self.data.fido_auto_add,
                    auto_add_conference: 0,
                    pass_thru: self.data.fido_enable_pass_thru,
                    default_zone: self.data.fido_default_zone.clamp(0, u16::MAX as i32) as u16,
                    default_net: self.data.fido_default_net.clamp(0, u16::MAX as i32) as u16,
                    verbose_log: self.data.fido_log_level != 0,
                },
                ..FtnConfig::default()
            };
            let ftn_path = self.output_directory.join(&icb_cfg.paths.ftn_file);
            if let Some(parent) = ftn_path.parent() {
                fs::create_dir_all(parent)?;
            }
            ftn.save(&ftn_path)?;
        }

        let destination = self.output_directory.join(icy_board_engine::DEFAULT_ICYBOARD_FILE);
        self.output.start_action(format!("Create main configuration {}…", destination.display()));
        if let Err(err) = icb_cfg.save(&destination) {
            self.logger.log_boxed_error(&*err);
        }
        self.output.start_action("done.".into());
        self.logger.log("done.");

        let destination = self.output_directory.join("importlog.txt");
        fs::write(destination, &self.logger.output)?;

        self.write_report()?;

        Ok(())
    }

    fn write_report(&mut self) -> Res<()> {
        let unresolved = self.unresolved_paths();
        let mut report = String::new();
        let _ = write!(
            report,
            "Imported {} to {}\n\n",
            self.source_directory.display(),
            self.output_directory.display()
        );
        let _ = writeln!(report, "Conferences   {}", self.stats.conferences);
        let _ = writeln!(report, "Users         {}", self.stats.users);
        let _ = writeln!(report, "Message bases {}", self.stats.message_bases);
        let _ = writeln!(report, "PPEs          {}", self.stats.ppes);
        let _ = writeln!(report, "Files         {}", self.stats.converted_files);
        if self.stats.users_without_inf > 0 {
            let _ = writeln!(report, "Users without USERS.INF record {}", self.stats.users_without_inf);
        }
        let _ = write!(report, "\nUnresolved paths ({}):\n", unresolved.len());
        for path in &unresolved {
            let _ = writeln!(report, "  {}", path);
        }
        report.push_str("\nPaths are looked up through:\n");
        let mut mappings: Vec<_> = self.resolve_paths.iter().collect();
        mappings.sort();
        for (dos, local) in mappings {
            let _ = writeln!(report, "  {} -> {}", dos, local);
        }
        report.push_str("\nAdd --map 'D:\\FILES=/path/to/files' for paths that point to another drive.\n");

        let destination = self.output_directory.join("import_report.txt");
        fs::write(&destination, &report)?;

        self.output.start_action(format!(
            "\nConferences {}, users {}, message bases {}, PPEs {}, files {}, unresolved paths {}.\nReport written to {}",
            self.stats.conferences,
            self.stats.users,
            self.stats.message_bases,
            self.stats.ppes,
            self.stats.converted_files,
            unresolved.len(),
            destination.display()
        ));
        Ok(())
    }

    fn convert_conferences(&mut self, conference_file: &str, new_rel_name: &str) -> Res<PathBuf> {
        self.output.start_action("Convert conferences…".into());

        let conf = self.resolve_file(conference_file);
        if !conf.exists() {
            self.output
                .warning(format!("Can't find conference file {}/{}", conf.display(), conference_file));
            self.logger
                .log(format!("Can't find conference file {}/{}", conf.display(), conference_file).as_str());
            return Ok(PathBuf::new());
        }
        let conferences = PcbConferenceHeader::import_pcboard(&conf, self.data.num_conf as usize)?;

        let conf_add = self.resolve_file(&(conference_file.to_string() + ".ADD"));
        let conf_old_add = self.resolve_file(&(conference_file.to_string() + ".@@@"));

        let add_conferences = if conf_add.exists() {
            self.logger.log(format!("import conference header {}", conf_add.display()).as_str());
            PcbAdditionalConferenceHeader::import_pcboard(&conf_add)?
        } else if conf_old_add.exists() {
            self.logger.log(format!("read old conference header {}", conf_old_add.display()).as_str());
            PcbAdditionalConferenceHeader::import_old_pcboard(&conf_old_add)?
        } else {
            self.output.warning(format!("Can't find conference add file {}", conf_add.display()));
            self.logger.log(format!("Can't find conference add file {}", conf_add.display()).as_str());
            vec![PcbAdditionalConferenceHeader::default(); conferences.len()]
        };
        self.logger.log("imported conference headers, converting...");
        self.stats.conferences = conferences.len();

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
            // The metadata path anchors the file base index, so it has to follow the new
            // upload directory instead of staying on the sysop's old DOS drive.
            conf.pub_upload_location = PathBuf::from(output.to_string() + "/pub_up");
            conf.private_upload_location = PathBuf::from(output.to_string() + "/priv_up");
            let _ = fs::create_dir_all(self.output_directory.join(&conf.pub_upload_location));
            let _ = fs::create_dir_all(self.output_directory.join(&conf.private_upload_location));
            conf.pub_upload_metadata = conf.pub_upload_location.join("dir");
            conf.private_upload_metadata = conf.private_upload_location.join("dir");
            conf.doors_menu = self.convert_conference_display_file(&output, &conf.doors_menu)?;
            conf.doors_file = self.convert_doors_file(&destination, &output, &conf.doors_file)?;

            conf.blt_menu = self.convert_conference_display_file(&output, &conf.blt_menu)?;
            conf.blt_file = self.convert_bullettin_file(&destination, &output, &conf.blt_file)?;

            self.logger.log(format!("convert survey menus {}", conf.survey_menu.display()).as_str());
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
                    // An area without a message base is one the board can only stumble over.
                    list.retain(|area| !area.path.as_os_str().is_empty());
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

    fn convert_conference_display_file(&mut self, output: &str, file_path: &Path) -> Res<PathBuf> {
        let Some(file_name) = file_path.file_name() else {
            return Ok(PathBuf::new());
        };

        let resolved_file = self.resolve_listed_file(file_path);

        let name = resolved_file.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
        let new_name = format!("{}/{}", output, &name);
        if resolved_file.exists() {
            return self.convert_display_file(&resolved_file.to_string_lossy(), &new_name);
        }
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
            for entry in read_mapped_source_directory(parent, pcb_text_file, &resolved_file)?.flatten() {
                if entry.path().is_dir() {
                    continue;
                }
                if entry
                    .file_name()
                    .to_str()
                    .unwrap()
                    .to_ascii_uppercase()
                    .starts_with(&resolved_file.file_name().unwrap().to_str().unwrap().to_ascii_uppercase())
                    && let Ok(mut text_file) = IcbTextFile::load(&entry.path())
                {
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

        Ok(PathBuf::from(new_rel_name.to_string() + ".toml"))
    }

    pub fn scan_pcb_text_line_for_commands(&mut self, line: &str, i: usize) -> Res<String> {
        if let Some(call) = PPECall::try_parse_line(line) {
            if !is_dos_path(&call.file) {
                return Ok(line.to_string());
            }
            // '_' ends the record and '~' is a space - both belong to the entry, not to the file name.
            let file = call.file.trim_end_matches(['_', '~']).to_string();
            self.logger
                .log(&format!("Found {:?} in entry {} : {} arguments :{:?}", call.call_type, i, file, call.arguments));
            let resolved_file = self.resolve_file(&file);
            if !resolved_file.exists() {
                let warning = unresolved_configured_path_warning(&file, &resolved_file);
                self.output.warning(warning.clone());
                self.logger.log(&format!("Warning: {warning}"));
                return Ok(line.to_string());
            }
            let new_name = self.convert_file(resolved_file)?;

            let mut new_line = String::new();
            for (i, ch) in line.chars().enumerate() {
                if i == 1 {
                    new_line.push_str(&new_name);
                }
                if i >= 1 && i <= file.len() {
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
            if !is_dos_path(&call.file) {
                return Ok(line.to_string());
            }
            let file = call.file.trim_end_matches(['_', '~']).to_string();
            self.logger
                .log(&format!("Found {:?} in line {} : {} arguments :{:?}", call.call_type, i, file, call.arguments));
            let resolved_file = self.resolve_file(&file);
            if !resolved_file.exists() {
                let warning = unresolved_configured_path_warning(&file, &resolved_file);
                self.output.warning(warning.clone());
                self.logger.log(&format!("Warning: {warning}"));
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
                if i >= 1 && i <= file.len() {
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
                    let new_name = self.copy_ppe(&resolved_file)?;
                    self.converted_files.insert(upper_file_name.clone(), new_name.clone());
                    return Ok(new_name);
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
        if let Err(err) = self.import_and_scan_file(&resolved_file, &output_path) {
            self.logger.log_boxed_error(&*err);
            return Ok(resolved_file.to_str().unwrap().to_string());
        }
        Ok(rel_name)

        /*
        let rel_name = format!("gen/{}", resolved_file.file_name().unwrap().to_ascii_lowercase().to_string_lossy());
        let output_path = self.output_directory.join(&rel_name);
        convert_to_utf8(&resolved_file, &output_path)?;
        self.converted_files.insert(upper_file_name.clone(), rel_name);
        let out_path = output_path.file_name().unwrap().to_str().unwrap().to_string();
        self.logger.converted_file(&resolved_file, &output_path, true);
        Ok(out_path)*/
    }

    /// A PPE is useless without the files next to it, so its own directory comes over as a whole.
    fn copy_ppe(&mut self, ppe: &Path) -> Res<String> {
        let stem = ppe.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let file_name = ppe.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
        let parent = ppe.parent().unwrap_or(&self.source_directory).to_path_buf();
        let own_directory = self.is_ppe_directory(&parent);

        let dir_name = if own_directory {
            parent.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase()
        } else {
            stem.to_ascii_lowercase()
        };
        let dest_dir = self.output_directory.join("ppe").join(&dir_name);

        self.output
            .start_action(format!("\t copy PPE '{}' to '{}'…", ppe.display(), dest_dir.display()));
        fs::create_dir_all(&dest_dir)?;

        for entry in WalkDir::new(&parent).max_depth(if own_directory { usize::MAX } else { 1 }) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            if !own_directory && !entry.path().file_stem().is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(&stem)) {
                continue;
            }
            let rel_path: RelativePathBuf = entry.path().relative_to(&parent).unwrap();
            let to = RelativePathBuf::from_path(rel_path.as_str().to_lowercase()).unwrap().to_logical_path(&dest_dir);
            if let Some(parent_dir) = to.parent() {
                fs::create_dir_all(parent_dir)?;
            }
            self.copy_ppe_file(entry.path(), &to)?;
            self.logger.copy_file(entry.path(), &to);
        }
        self.stats.ppes += 1;
        Ok(format!("ppe/{}/{}", dir_name, file_name))
    }

    /// A directory holding a handful of PPEs is one PPE's own; a bucket like PPE\ is not.
    fn is_ppe_directory(&self, dir: &Path) -> bool {
        if dir == self.source_directory || !dir.is_dir() {
            return false;
        }
        let ppes = fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|entry| entry.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("ppe")))
                    .count()
            })
            .unwrap_or(0);
        (1..=3).contains(&ppes)
    }

    /// Config files of a PPE carry the DOS paths of the installation they were written for.
    /// The bytes are kept as they are - only the paths themselves are replaced.
    fn copy_ppe_file(&mut self, from: &Path, to: &Path) -> Res<()> {
        let is_documentation = from.extension().is_some_and(|ext| {
            ["doc", "nfo", "diz", "ans", "pcb", "asc", "rip"]
                .iter()
                .any(|kind| ext.eq_ignore_ascii_case(kind))
        });
        let Ok(data) = fs::read(from) else {
            self.output.check_error(fs::copy(from, to).err())?;
            return Ok(());
        };
        if is_documentation || data.contains(&0) || !data.windows(2).any(|w| w[1] == b':' && w[0].is_ascii_alphabetic()) {
            self.output.check_error(fs::copy(from, to).err())?;
            return Ok(());
        }

        let mut out = Vec::with_capacity(data.len());
        let mut i = 0;
        let mut rewrote = 0;
        while i < data.len() {
            let Some(end) = dos_path_at(&data, i) else {
                out.push(data[i]);
                i += 1;
                continue;
            };
            let dos_path = String::from_utf8_lossy(&data[i..end]).to_string();
            match self.map_ppe_path(&dos_path) {
                Some(new_path) => {
                    out.extend(new_path.as_bytes());
                    rewrote += 1;
                    self.logger.log(&format!("{}: {} -> {}", from.display(), dos_path, new_path));
                }
                None => out.extend(&data[i..end]),
            }
            i = end;
        }

        if rewrote > 0 {
            self.output.start_action(format!("\t adjust {} path(s) in '{}'…", rewrote, to.display()));
        }
        fs::write(to, &out)?;
        Ok(())
    }

    /// A DOS path that leads into a PPE directory gets the place that directory has now.
    fn map_ppe_path(&self, dos_path: &str) -> Option<String> {
        let resolved = self.try_resolve_file(dos_path);
        if resolved.exists() {
            if let Some(mapped) = self.ppe_target_path(&resolved) {
                return Some(mapped);
            }
            // Anything the import already brought over is named by the place it went to.
            return self.converted_files.get(&resolved.to_string_lossy().to_ascii_uppercase()).cloned();
        }
        // Log files and the like are only created while the board runs, so the directory decides.
        let path = PathBuf::from(dos_path.replace('\\', "/"));
        let dir = self.try_resolve_file(&path.parent()?.to_string_lossy());
        if !dir.is_dir() {
            return None;
        }
        let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
        self.ppe_target_path(&dir.join(name))
    }

    /// Where a file below a PPE directory ends up - the place copy_ppe() gives it.
    fn ppe_target_path(&self, resolved: &Path) -> Option<String> {
        let mut rel = PathBuf::new();
        let mut dir = resolved.to_path_buf();
        loop {
            if self.is_ppe_directory(&dir) {
                let name = dir.file_name()?.to_string_lossy().to_ascii_lowercase();
                let rel = rel.to_string_lossy().to_ascii_lowercase();
                return Some(if rel.is_empty() {
                    format!("ppe/{}", name)
                } else {
                    format!("ppe/{}/{}", name, rel)
                });
            }
            let name = dir.file_name()?.to_os_string();
            dir = dir.parent()?.to_path_buf();
            rel = PathBuf::from(name).join(rel);
        }
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
        // A prompt file ends where the cursor has to stay, so a newline is not added to one that had none.
        if !in_string.ends_with('\n') && !in_string.ends_with('\r') {
            import.pop();
        }

        write_utf8_with_bom(to, &import)?;
        self.stats.converted_files += 1;
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
            fs::write(&dest, trashcan_header)?;
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

            let mut missing_inf = 0;
            let pcb_users = users
                .iter()
                .map(|u| PcbUser {
                    user: u.clone(),
                    // A packed or crashed board can leave USERS.INF shorter than USERS.
                    inf: user_inf.get(u.rec_num as usize).cloned().unwrap_or_else(|| {
                        missing_inf += 1;
                        PcbUserInf::default()
                    }),
                })
                .collect::<Vec<PcbUser>>();
            if missing_inf > 0 {
                self.output
                    .warning(format!("{} users have no USERS.INF record - imported with defaults.", missing_inf));
                self.logger.log(&format!("{} users without USERS.INF record", missing_inf));
                self.stats.users_without_inf = missing_inf;
            }

            UserBase::import_pcboard(&pcb_users)
        };
        let destination: PathBuf = self.output_directory.join(new_rel_name);
        if user_base.is_empty() {
            self.logger.log("User base empty, generating sysop.");

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
        self.stats.users = user_base.len();
        user_base.save(&destination)?;
        Ok(PathBuf::from(new_rel_name))
    }

    fn copy_display_directory<F: Fn(&Path) -> bool>(
        &mut self,
        category: &str,
        dir_loc: &str,
        rel_output: &str,
        flat_prefix: Option<&str>,
        filter: F,
    ) -> Res<()> {
        self.logger.log(&format!("\n=== Converting {} ===", category));
        if dir_loc.is_empty() {
            self.logger.log("\ndir wasn't set.");
            return Ok(());
        }
        let help_loc = self.resolve_file(dir_loc);
        let mut help_loc = PathBuf::from(&help_loc);
        let mut flat_prefix = flat_prefix.map(|p| p.to_ascii_uppercase());
        if !help_loc.is_dir() {
            // On a flat installation the files sit next to PCBOARD.DAT and only the name tells them apart.
            match &flat_prefix {
                Some(_) if self.source_directory.is_dir() => {
                    self.logger
                        .log(&format!("\ndir {} doesn't exist - taking them from the board directory", help_loc.display()));
                    help_loc = self.source_directory.clone();
                }
                _ => {
                    self.logger.log(&format!("\ndir {} doesn't exist", help_loc.display()));
                    return Ok(());
                }
            }
        } else {
            flat_prefix = None;
        }

        let o = self.output_directory.join(rel_output);
        if help_loc.exists() {
            self.output
                .start_action(format!("Copy {} from {} to {}…", category, help_loc.display(), o.display()));
            for entry in WalkDir::new(&help_loc).max_depth(if flat_prefix.is_some() { 1 } else { usize::MAX }) {
                let entry = entry?;
                if entry.path().is_dir() {
                    continue;
                }
                if let Some(prefix) = &flat_prefix
                    && !entry.file_name().to_string_lossy().to_ascii_uppercase().starts_with(prefix)
                {
                    continue;
                }
                if !filter(entry.path()) {
                    continue;
                }
                let rel_path: RelativePathBuf = entry.path().relative_to(&help_loc).unwrap();
                let lower_case = RelativePathBuf::from_path(rel_path.as_str().to_lowercase()).unwrap();
                let to = lower_case.to_logical_path(&o);
                if let Some(parent_dir) = to.parent()
                    && !parent_dir.exists()
                    && let Err(err) = fs::create_dir(parent_dir)
                {
                    self.logger.log(&format!("Can't create directory {}:", parent_dir.display()));
                    self.logger.log_error(Some(err))?;
                    self.output.warning(format!("Can't create directory {}", parent_dir.display()));
                    continue;
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
                if let Some(parent_dir) = to.parent()
                    && !parent_dir.exists()
                {
                    fs::create_dir(parent_dir).unwrap();
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
        let resolved_file = self.resolve_listed_file(src_file);
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
        self.stats.message_bases += 1;

        self.logger
            .log(&format!("Converted message base {} -> {}", resolved_file.display(), destination.display()));
        let new_rel_name = PathBuf::from(output.to_string().to_lowercase()).join(resolved_file.file_name().unwrap().to_ascii_lowercase());
        self.converted_files.insert(upper_file_name.clone(), new_rel_name.to_str().unwrap().to_string());
        Ok(new_rel_name)
    }

    /// A conference without a usable list still gets an empty one, so the board has something to load.
    fn write_empty_list<T: Default + IcyBoardSerializer>(&mut self, dest_path: &Path, output: &str, name: &str) -> Res<PathBuf> {
        let destination = dest_path.join(name);
        T::default().save(&destination)?;
        self.logger.log(&format!("Wrote empty list {}", destination.display()));
        Ok(PathBuf::from(format!("{}/{}", output, name)))
    }

    fn convert_bullettin_file(&mut self, dest_path: &Path, output: &str, src_file: &Path) -> Res<PathBuf> {
        self.logger.log(&format!("\n=== Converting BLT.LST {} ===", src_file.display()));

        if src_file.to_str().unwrap().is_empty() {
            return self.write_empty_list::<BullettinList>(dest_path, output, "blt.toml");
        }
        let resolved_file = self.resolve_listed_file(src_file);
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let Ok(mut list) = BullettinList::import_pcboard(&resolved_file) else {
            self.logger.log(&format!("Warning, can't import bulletin  {}", resolved_file.display()));
            self.output.warning(format!("Warning, can't import bulletin {}", resolved_file.display()));
            return self.write_empty_list::<BullettinList>(dest_path, output, "blt.toml");
        };
        let resolved_file = resolved_file.with_extension("toml");

        let destination = PathBuf::from(dest_path).join(resolved_file.file_name().unwrap().to_ascii_lowercase());

        for entry in list.iter_mut() {
            let new_entry = self.resolve_file(entry.path.to_str().unwrap());
            if !new_entry.exists() {
                self.logger
                    .log(&format!("Warning, can't import bulletin  {}, doesn't exist.", new_entry.display()));
                continue;
            }

            let name = new_entry.file_name().unwrap().to_str().unwrap().to_string().to_ascii_lowercase();
            let new_name = format!("{}/{}", output, &name);

            match self.convert_display_file(new_entry.to_str().unwrap(), &new_name) {
                Ok(new_name) => {
                    entry.path = new_name;
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
            return self.write_empty_list::<SurveyList>(dest_path, output, "survey.toml");
        }
        let resolved_file = self.resolve_listed_file(src_file);
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let Ok(mut list) = SurveyList::import_pcboard(&resolved_file) else {
            self.logger
                .log(&format!("Warning, can't import script questionnaires {}", resolved_file.display()));
            self.output
                .warning(format!("Warning, can't import script questionnaires {}", resolved_file.display()));
            return self.write_empty_list::<SurveyList>(dest_path, output, "survey.toml");
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
            return self.write_empty_list::<DirectoryList>(dest_path, output, "dir.toml");
        }
        let resolved_file = self.resolve_listed_file(src_file);
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let Ok(mut list) = DirectoryList::import_pcboard(&resolved_file) else {
            self.logger.log(&format!("Warning, can't import dir.lst file {}", resolved_file.display()));
            self.output.warning(format!("Warning, can't import dir.lst file {}", resolved_file.display()));
            return self.write_empty_list::<DirectoryList>(dest_path, output, "dir.toml");
        };
        let resolved_file = resolved_file.with_extension("toml");

        let destination = PathBuf::from(dest_path).join(resolved_file.file_name().unwrap().to_ascii_lowercase());

        for (i, entry) in list.iter_mut().enumerate() {
            let configured_path = entry.path.clone();
            entry.path = self.resolve_file(entry.path.to_str().unwrap());
            if !entry.path.exists() {
                let warning = format!(
                    "File area directory '{}' does not exist (looked for '{}'); keeping it for manual correction.",
                    configured_path.display(),
                    entry.path.display()
                );
                self.output.warning(warning.clone());
                self.logger.log(&format!("Warning: {warning}"));
            }
            entry.metadata_path = imported_metadata_path(output, i);
            let base_path = self.output_directory.join(&entry.metadata_path);
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
            return self.write_empty_list::<DoorList>(dest_path, output, "doors.toml");
        }
        let resolved_file = self.resolve_listed_file(src_file);
        let upper_file_name = resolved_file.to_str().unwrap().to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let Ok(list) = DoorList::import_pcboard(&resolved_file) else {
            self.logger.log(&format!("Warning, can't import bulletin  {}", resolved_file.display()));
            self.output.warning(format!("Warning, can't import bulletin {}", resolved_file.display()));
            return self.write_empty_list::<DoorList>(dest_path, output, "doors.toml");
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
        let resolved_file = self.resolve_listed_file(&src_file);
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

#[cfg(test)]
mod import_error_tests {
    use super::*;

    #[test]
    fn imported_file_area_metadata_stays_inside_the_board() {
        assert_eq!(imported_metadata_path("conferences/main", 0), PathBuf::from("conferences/main/dir00"));
        assert_eq!(imported_metadata_path("conferences/12", 3), PathBuf::from("conferences/12/dir03"));
    }

    #[test]
    fn unresolved_configured_path_message_explains_that_the_value_is_retained() {
        let warning = unresolved_configured_path_warning(r"D:\PCB\PPL\LORD\LORD.PPE", Path::new("/import/pcb/PPL/LORD/LORD.PPE"));

        assert!(warning.contains(r"D:\PCB\PPL\LORD\LORD.PPE"));
        assert!(warning.contains("/import/pcb/PPL/LORD/LORD.PPE"));
        assert!(warning.contains("keeping the original value"));
    }

    #[test]
    fn missing_mapped_source_directory_explains_the_mapping() {
        let parent = std::env::temp_dir().join(format!("icbsetup-missing-map-{}", std::process::id()));
        let resolved = parent.join("GEN/PCBTEXT");

        let error = read_mapped_source_directory(&parent.join("GEN"), r"D:\PCB\GEN\PCBTEXT", &resolved).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(r"D:\PCB\GEN\PCBTEXT"));
        assert!(message.contains(&resolved.display().to_string()));
        assert!(message.contains("Verify that --map points to the existing PCBoard installation"));
    }
}
