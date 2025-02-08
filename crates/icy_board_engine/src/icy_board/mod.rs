use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::Res;
use codepages::tables::write_with_bom;
use qfile::QTraitSync;
use thiserror::Error;

use crate::vm::errors::IcyError;

use self::{
    commands::CommandList,
    conferences::ConferenceBase,
    doors::DoorList,
    file_directory::DirectoryList,
    group_list::GroupList,
    icb_config::IcbConfig,
    icb_text::IcbTextFile,
    language::SupportedLanguages,
    message_area::AreaList,
    pcbconferences::{PcbAdditionalConferenceHeader, PcbConferenceHeader, PcbLegacyConferenceHeader},
    pcboard_data::PcbBoardData,
    sec_levels::SecurityLevelDefinitions,
    statistics::Statistics,
    user_base::{User, UserBase},
    xfer_protocols::SupportedProtocols,
};

pub mod bbs;
pub mod bulletins;
pub mod commands;
pub mod conferences;
pub mod doors;
pub mod file_directory;
pub mod group_list;
pub mod icb_config;
pub mod icb_text;
pub mod language;
pub mod login_server;
pub mod macro_parser;
pub mod menu;
pub mod message_area;
pub mod pcb;
pub mod sec_levels;
pub mod security_expr;
pub mod state;
pub mod statistics;
pub mod surveys;
pub mod user_base;
pub mod xfer_protocols;

pub use pcb::*;

pub mod accounting_cfg;

#[derive(Error, Debug)]
pub enum IcyBoardError {
    #[error("Error: {0}")]
    Error(String),

    #[error("invalid user.inf record size: '{0}' expected {1} got {2}")]
    InvalidUserInfRecordSize(&'static str, usize, usize),

    #[error("Can't run action ({0})")]
    UnknownAction(String),

    #[error("Thread crashed. See output.log for details.")]
    ThreadCrashed,

    #[error("Can't read file {0} ({1})")]
    FileError(PathBuf, String),

    #[error("Can't write file {0} ({1})")]
    ErrorCreatingFile(String, String),

    #[error("Loading file {0} invalid record size ({1}:{2})")]
    InvalidRecordSize(String, usize, usize),

    #[error("Importing file {0} parsing record error ({1})")]
    ImportRecordErorr(String, String),

    #[error("Error loading PCBoard DIR.LIST file invalid sort order ({0})")]
    InvalidDirListSortOrder(u8),

    #[error("User number invalid: {0}")]
    UserNumberInvalid(usize),

    #[error("Internal board lock error (report!).")]
    ErrorLockingBoard,

    #[error("Error opening home directory ({0})")]
    HomeDirMissing(String),
}

pub struct IcyBoard {
    pub file_name: PathBuf,
    pub root_path: PathBuf,
    pub users: UserBase,
    pub config: IcbConfig,
    pub conferences: ConferenceBase,
    pub default_display_text: IcbTextFile,

    pub languages: SupportedLanguages,
    pub protocols: SupportedProtocols,
    pub sec_levels: SecurityLevelDefinitions,
    pub groups: GroupList,
    pub statistics: Statistics,
    pub commands: CommandList,
}

impl IcyBoard {
    pub fn new() -> Self {
        let default_display_text = IcbTextFile::default();

        IcyBoard {
            default_display_text,
            file_name: PathBuf::new(),
            root_path: PathBuf::new(),
            users: UserBase::default(),
            config: IcbConfig::new(),
            conferences: ConferenceBase::default(),
            languages: SupportedLanguages::default(),
            protocols: SupportedProtocols::default(),
            sec_levels: SecurityLevelDefinitions::default(),
            commands: CommandList::default(),
            statistics: Statistics::default(),
            groups: GroupList::default(),
        }
    }

    pub fn resolve_paths(&mut self) {
        self.config.paths.statistics_file = self.resolve_file(&self.config.paths.statistics_file);
    }

    pub fn resolve_file<P: AsRef<Path>>(&self, file: &P) -> PathBuf {
        if file.as_ref().as_os_str().is_empty() {
            return PathBuf::new();
        }
        let mut s = PathBuf::from(file.as_ref());
        if !s.is_absolute() {
            s = self.root_path.join(s);
        }
        if s.exists() {
            return s;
        }
        /*
                let mut s: String = file
                .as_ref()
                .to_string_lossy()
                .to_string()
                .chars()
                .map(|x| match x {
                    '\\' => '/',
                    _ => x,
                })
                .collect();
        */
        return case_insitive_lookup(s);
    }

    pub fn load<P: AsRef<Path>>(path: &P) -> Res<Self> {
        let config = IcbConfig::load(path).map_err(|e| {
            log::error!("Error loading icy board config file: {} from {}", e, path.as_ref().display());
            e
        })?;

        let mut p = PathBuf::from(path.as_ref());
        if !p.is_absolute() {
            if let Ok(cur) = std::env::current_dir() {
                p = cur.join(path.as_ref())
            } else {
                p = p.canonicalize().unwrap();
            }
        }
        let parent_path = p.parent().unwrap();

        /*
        let load_path = &RelativePath::from_path(&config.paths.user_base)?.to_path(parent_path);
        let mut users = UserBase::load(&load_path).map_err(|e| {
            log::error!("Error loading user base: {} from {}", e, load_path.display());
            println!("Error loading user base: {} from {}", e, load_path.display());
            e
        })?;*/
        let mut users = UserBase::default();

        let load_path = get_path(parent_path, &config.paths.home_dir);
        if !load_path.exists() {
            fs::create_dir_all(&load_path).map_err(|e| {
                log::error!("Error creating home directory: {} from {}", e, load_path.display());
                e
            })?;
        }
        users.load_users(&load_path).map_err(|e| {
            log::error!("Error loading users: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = get_path(parent_path, &config.paths.conferences);
        let conferences = ConferenceBase::load(&load_path).map_err(|e| {
            log::error!("Error loading conference base: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = get_path(parent_path, &config.paths.icbtext);
        let default_display_text = IcbTextFile::load(&load_path).map_err(|e| {
            log::error!("Error loading display text: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = get_path(parent_path, &config.paths.language_file);
        let languages = SupportedLanguages::load(&load_path).map_err(|e| {
            log::error!("Error loading languages: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = get_path(parent_path, &config.paths.protocol_data_file);
        let protocols = SupportedProtocols::load(&load_path).map_err(|e| {
            log::error!("Error loading protocols: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = get_path(parent_path, &config.paths.pwrd_sec_level_file);
        let sec_levels = SecurityLevelDefinitions::load(&load_path).map_err(|e| {
            log::error!("Error loading security levels: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = get_path(parent_path, &config.paths.command_file);
        let commands = CommandList::load(&load_path).map_err(|e| {
            log::error!("Error loading commands: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = get_path(parent_path, &config.paths.statistics_file);
        let statistics = match Statistics::load(&load_path) {
            Ok(stat) => stat,
            Err(e) => {
                log::error!("Error loading statistics: {} from {}, generating default.", e, load_path.display());
                Statistics::default()
            }
        };

        let load_path = get_path(parent_path, &config.paths.group_file);
        let groups = match GroupList::load(&load_path) {
            Ok(stat) => stat,
            Err(e) => {
                log::error!("Error loading groups: {} from {}, generating default.", e, load_path.display());
                GroupList::default()
            }
        };
        let mut board = IcyBoard {
            file_name: path.as_ref().to_path_buf(),
            root_path: parent_path.to_path_buf(),
            users,
            config,
            conferences,
            default_display_text,
            languages,
            protocols,
            sec_levels,
            commands,
            statistics,
            groups,
        };

        for conf in board.conferences.iter_mut() {
            let message_area_file = if conf.area_file.is_absolute() {
                conf.area_file.clone()
            } else {
                board.root_path.join(&conf.area_file)
            };
            if message_area_file.exists() {
                match AreaList::load(&message_area_file) {
                    Ok(areas) => {
                        conf.areas = areas;
                    }
                    Err(err) => {
                        log::error!("Error loading message areas {}: {}", message_area_file.display(), err);
                    }
                }
            }

            let file_area_file = if conf.dir_file.is_absolute() {
                conf.dir_file.clone()
            } else {
                board.root_path.join(&conf.dir_file)
            };
            if file_area_file.exists() {
                match DirectoryList::load(&file_area_file) {
                    Ok(directories) => {
                        conf.directories = directories;
                    }
                    Err(err) => {
                        log::error!("Error loading file areas {}: {}", file_area_file.display(), err);
                    }
                }
            }

            let doors_file = if conf.doors_file.is_absolute() {
                conf.doors_file.clone()
            } else {
                board.root_path.join(&conf.doors_file)
            };
            if doors_file.exists() {
                match DoorList::load(&doors_file) {
                    Ok(doors) => {
                        conf.doors = doors;
                    }
                    Err(err) => {
                        log::error!("loading door files {}: {}", doors_file.display(), err);
                    }
                }
            }
        }

        Ok(board)
    }

    pub fn save(&self) -> Res<()> {
        self.config.save(&self.file_name)?;
        self.conferences.save(&self.resolve_file(&self.config.paths.conferences))?;
        Ok(())
    }

    pub fn get_homedir(&self) -> PathBuf {
        PathBuf::from(self.resolve_file(&self.config.paths.home_dir))
    }

    pub fn save_userbase(&mut self) -> Res<()> {
        let home_dir = self.get_homedir();
        if let Err(e) = self.users.save_users(&home_dir) {
            log::error!("Error saving user base: {}", e);
            Err(e)
        } else {
            Ok(())
        }
    }

    pub fn export_pcboard(&self, file: &Path) -> Res<()> {
        let mut pcb_dat = PcbBoardData::default();
        pcb_dat.sysop_info.require_pwrd_to_exit = self.config.sysop.require_password_to_exit;

        // Line 2 Sysop Display Name (if answered NO to "Use Real Name")
        pcb_dat.sysop_info.sysop = self.config.sysop.name.to_string();
        // Line 3 Sysop Password (from call waiting screen)
        pcb_dat.sysop_info.password = self.config.sysop.password.to_string();
        // Line 4
        pcb_dat.sysop_info.use_real_name = self.config.sysop.use_real_name;
        // Line 5
        pcb_dat.sysop_info.use_local_graphics = true;

        // Line 8 Sysop Level
        pcb_dat.sysop_security.sysop = self.config.sysop_command_level.sysop as i32;

        // Line 24
        pcb_dat.path.help_loc = self.resolve_file(&self.config.paths.help_path).to_string_lossy().to_string();
        // Line 25
        pcb_dat.path.sec_loc = self.resolve_file(&self.config.paths.home_dir).to_string_lossy().to_string();

        // Line 31
        let base_loc = file.parent().unwrap();
        let cnames = base_loc.join("cnames");
        self.export_conference_files(&base_loc, &cnames)?;
        pcb_dat.path.conference_file = cnames.to_string_lossy().to_string();

        // Line 32 - PWRD File
        let pwrd_file = base_loc.join("pwrd");
        if let Ok(defs) = SecurityLevelDefinitions::load(&self.resolve_file(&self.config.paths.pwrd_sec_level_file)) {
            defs.export_pcboard(&pwrd_file)?;
        } else {
            fs::write(&pwrd_file, "")?;
        }
        pcb_dat.path.pwrd_file = pwrd_file.to_string_lossy().to_string();

        // Line 35
        pcb_dat.path.tcan_file = self.resolve_file(&self.config.paths.trashcan_user).to_string_lossy().to_string();
        // Line 36
        pcb_dat.path.welcome_file = self.resolve_file(&self.config.paths.welcome).to_string_lossy().to_string();
        // Line 37
        pcb_dat.path.newuser_file = self.resolve_file(&self.config.paths.newuser).to_string_lossy().to_string();
        // Line 38
        pcb_dat.path.closed_file = self.resolve_file(&self.config.paths.closed).to_string_lossy().to_string();
        // Line 39
        pcb_dat.path.warning_file = self.resolve_file(&self.config.paths.expire_warning).to_string_lossy().to_string();
        // Line 40
        pcb_dat.path.expired_file = self.resolve_file(&self.config.paths.expired).to_string_lossy().to_string();
        // Line 42
        pcb_dat.path.conf_menu = self.resolve_file(&self.config.paths.conf_join_menu).to_string_lossy().to_string();
        // Line 45
        let protocol_data_file = base_loc.join("pcbprot.dat");
        self.protocols.export_data(&protocol_data_file)?;
        pcb_dat.path.protocol_data_file = protocol_data_file.to_string_lossy().to_string();
        // Line 47
        pcb_dat.path.logoff_script = self.resolve_file(&self.config.paths.logoff_survey).to_string_lossy().to_string();
        // Line 48
        pcb_dat.path.logoff_answer = self.resolve_file(&self.config.paths.logoff_answer).to_string_lossy().to_string();
        // Line 50
        pcb_dat.path.group_chat = self.resolve_file(&self.config.paths.chat_intro_file).to_string_lossy().to_string();

        // Line 76
        pcb_dat.closed_board = self.config.system_control.is_closed_board;

        // Line 87
        pcb_dat.display_news = self.config.switches.display_news_behavior.to_pcb_char();

        // Line 94
        pcb_dat.board_name = self.config.board.name.to_string();

        // Line 108
        pcb_dat.num_conf = self.conferences.len() as i32 - 1;

        // Line 149
        pcb_dat.user_levels.agree_to_register = self.config.new_user_settings.sec_level as i32;

        // Line 202
        pcb_dat.path.no_ansi = self.resolve_file(&self.config.paths.no_ansi).to_string_lossy().to_string();

        // Line 249 Name/Location of LOGON Script Questionnaire
        pcb_dat.path.login_script = self.resolve_file(&self.config.paths.logon_survey).to_string_lossy().to_string();
        // Line 250 Name/Location of LOGON Script Questionnaire ANSWER File
        pcb_dat.path.login_answer = self.resolve_file(&self.config.paths.logon_answer).to_string_lossy().to_string();

        // Line 267
        pcb_dat.path.cmd_display_files_loc = self.resolve_file(&self.config.paths.command_display_path).to_string_lossy().to_string();

        // Line 265
        pcb_dat.min_pwrd_len = self.config.limits.min_pwd_length as i32;

        // Line 269
        pcb_dat.skip_protocol = !self.config.new_user_settings.ask_xfer_protocol;
        // Line 270
        pcb_dat.skip_alias = !self.config.new_user_settings.ask_alias;

        // Line 296 (to prevent \0 char)
        pcb_dat.uucp_high_ascii = 'N';
        let res = pcb_dat.serialize(crate::parser::Encoding::CP437);
        fs::write(file, res)?;

        Ok(())
    }

    fn export_conference_files(&self, base_loc: &Path, cnames: &PathBuf) -> Res<()> {
        let mut headers = Vec::new();
        let mut legacy_headers = Vec::new();
        let mut add_headers = Vec::new();

        legacy_headers.extend(u16::to_le_bytes(PcbLegacyConferenceHeader::HEADER_SIZE as u16));
        let mut dirs = 0;
        for conf in self.conferences.iter() {
            dirs += 1;

            // Convert dir file
            let dir_file = base_loc.join(&format!("dir{}", dirs));
            let dir_file = if let Ok(area_list) = DirectoryList::load(&self.resolve_file(&conf.dir_file)) {
                area_list.export_pcboard(&dir_file)?;

                let dir_file = dir_file.to_string_lossy().to_string();
                let len = self.root_path.to_string_lossy().len() + 1;
                dir_file[len..].to_string()
            } else {
                String::new()
            };

            let header = PcbConferenceHeader {
                name: conf.name.clone(),
                auto_rejoin: conf.auto_rejoin,
                view_members: conf.allow_view_conf_members,
                private_uploads: conf.private_uploads,
                private_msgs: conf.private_msgs,
                echo_mail: false,
                add_conference_security: conf.add_conference_security,
                add_conference_time: conf.add_conference_time,
                message_blocks: 0,
                message_file: String::new(),
                users_menu: conf.users_menu.to_string_lossy().to_string(),
                sysop_menu: conf.sysop_menu.to_string_lossy().to_string(),
                news_file: conf.news_file.to_string_lossy().to_string(),
                pub_upload_sort: conf.pub_upload_sort,
                pub_upload_dirfile: String::new(),
                pub_upload_location: conf.pub_upload_location.to_string_lossy().to_string(),
                private_upload_sort: conf.private_upload_sort,
                private_upload_dirfile: String::new(),
                private_upload_location: conf.private_upload_location.to_string_lossy().to_string(),
                public_conference: conf.is_public,
                doors_menu: conf.doors_menu.to_string_lossy().to_string(),
                doors_file: conf.doors_file.to_string_lossy().to_string(),
                required_security: conf.required_security.level(),
                blt_menu: conf.blt_menu.to_string_lossy().to_string(),
                blt_file: conf.blt_file.to_string_lossy().to_string(),
                script_menu: String::new(), // todo
                script_file: String::new(),
                dir_menu: String::new(), // todo
                dir_file: dir_file.to_string(),
                dlpth_list_file: String::new(),
            };
            headers.extend(header.serialize());

            let legacy_header = PcbLegacyConferenceHeader {
                name: conf.name.clone(),
                auto_rejoin: conf.auto_rejoin,
                view_members: conf.allow_view_conf_members,
                echo_mail: false,
                public_conf: conf.is_public,
                priv_uplds: conf.private_uploads,
                priv_msgs: conf.private_msgs,
                req_sec_level: conf.required_security.level() as u16,
                add_sec: conf.add_conference_security as u16,
                add_time: conf.add_conference_time as u16,
                msg_blocks: 0,
                msg_file: String::new(),
                user_menu: conf.users_menu.to_string_lossy().to_string(),
                sysop_menu: conf.sysop_menu.to_string_lossy().to_string(),
                news_file: conf.news_file.to_string_lossy().to_string(),
                pub_upld_sort: conf.pub_upload_sort,
                upld_dir: String::new(),
                pub_upld_loc: conf.pub_upload_location.to_string_lossy().to_string(),
                prv_upld_sort: conf.private_upload_sort,
                priv_dir: String::new(),
                prv_upld_loc: conf.private_upload_location.to_string_lossy().to_string(),
                drs_menu: conf.doors_menu.to_string_lossy().to_string(),
                drs_file: conf.doors_file.to_string_lossy().to_string(),
                blt_menu: conf.blt_menu.to_string_lossy().to_string(),
                blt_name_loc: conf.blt_file.to_string_lossy().to_string(),
                scr_menu: String::new(), // todo
                scr_name_loc: String::new(),
                dir_menu: String::new(), // todo
                dir_name_loc: dir_file,
                pth_name_loc: String::new(),
            };
            legacy_headers.extend(legacy_header.serialize());

            let add_header = PcbAdditionalConferenceHeader {
                password: conf.password.to_string(),
                attach_level: conf.sec_attachments.level(),
                req_level_to_enter: conf.sec_write_message.level(),
                allow_aliases: conf.allow_aliases,
                attach_loc: conf.attachment_location.to_string_lossy().to_string(),
                cmd_lst: conf.command_file.to_string_lossy().to_string(),
                intro: conf.intro_file.to_string_lossy().to_string(),
                force_echo: false,
                read_only: false,
                no_private_msgs: false,
                ret_receipt_level: 0,
                record_origin: false,
                prompt_for_routing: false,
                show_intro_on_ra: false,
                reg_flags: 0,
                carbon_limit: 0,
                old_index: false,
                long_to_names: false,
                carbon_level: 0,
                conf_type: 0,
                export_ptr: 0,
                charge_time: 0.0,
                charge_msg_read: 0.0,
                charge_msg_write: 0.0,
            };
            add_headers.extend(add_header.serialize());
        }

        fs::write(cnames, headers)?;
        fs::write(cnames.with_extension("@@@"), legacy_headers)?;
        fs::write(cnames.with_extension("add"), add_headers)?;

        Ok(())
    }

    pub fn set_user(&mut self, new_user: User, i: usize) -> Res<()> {
        let home_dir = UserBase::get_user_home_dir(&self.config.paths.home_dir, new_user.get_name());
        std::fs::create_dir_all(&home_dir).unwrap();
        let user_txt = toml::to_string(&new_user)?;
        fs::write(home_dir.join("user.toml"), user_txt)?;
        self.users[i] = new_user;
        Ok(())
    }

    pub fn save_statistics(&self) -> Res<()> {
        let r = &self.config.paths.statistics_file;
        if let Err(err) = self.statistics.save(&r) {
            log::error!("Error saving statistics to {} : {err}", r.display());
        }
        Ok(())
    }
}

pub(crate) fn case_insitive_lookup(path: PathBuf) -> PathBuf {
    if let Ok(mut file_path) = qfile::QFilePath::add_path(path.to_string_lossy()) {
        if let Ok(file) = file_path.get_path_buf() {
            return file;
        }
    }
    path
}

fn get_path(parent_path: &Path, home_dir: &PathBuf) -> PathBuf {
    if home_dir.is_absolute() {
        home_dir.clone()
    } else {
        parent_path.join(home_dir)
    }
}

impl Default for IcyBoard {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_false(b: impl std::borrow::Borrow<bool>) -> bool {
    !b.borrow()
}

pub fn is_true(b: impl std::borrow::Borrow<bool>) -> bool {
    *b.borrow()
}

pub fn path_is_empty(b: impl std::borrow::Borrow<PathBuf>) -> bool {
    (*b.borrow()).as_os_str().is_empty()
}

pub fn set_true() -> bool {
    true
}

pub fn is_null_8(b: impl std::borrow::Borrow<u8>) -> bool {
    *b.borrow() == 0
}

pub fn is_null_64(b: impl std::borrow::Borrow<u64>) -> bool {
    *b.borrow() == 0
}
pub fn is_null_32(b: impl std::borrow::Borrow<u32>) -> bool {
    *b.borrow() == 0
}

pub fn is_null_16(b: impl std::borrow::Borrow<u16>) -> bool {
    *b.borrow() == 0
}

pub fn is_null_i32(b: impl std::borrow::Borrow<i32>) -> bool {
    *b.borrow() == 0
}

pub fn is_null_f64(b: impl std::borrow::Borrow<f64>) -> bool {
    *b.borrow() == 0.0
}

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

pub fn read_with_encoding_detection<P: AsRef<Path>>(path: &P) -> Res<String> {
    match fs::read(path) {
        Ok(data) => {
            let import = if data.starts_with(&UTF8_BOM) {
                String::from_utf8_lossy(&data[UTF8_BOM.len()..]).to_string()
            } else {
                crate::tables::import_cp437_string(&data, false)
            };
            Ok(import)
        }
        Err(e) => Err(IcyBoardError::FileError(path.as_ref().to_path_buf(), e.to_string()).into()),
    }
}

pub fn read_data_with_encoding_detection(data: &[u8]) -> Res<String> {
    let import = if data.starts_with(&UTF8_BOM) {
        String::from_utf8_lossy(&data[UTF8_BOM.len()..]).to_string()
    } else {
        crate::tables::import_cp437_string(&data, false)
    };
    Ok(import)
}

pub fn convert_to_utf8<P: AsRef<Path>, Q: AsRef<Path>>(from: &P, to: &Q) -> Res<()> {
    let import = read_with_encoding_detection(from)?;
    write_with_bom(to, &import)?;
    Ok(())
}

pub(crate) fn load_internal<T: IcyBoardSerializer, P: AsRef<Path>>(path: &P) -> Res<T> {
    match fs::read_to_string(path) {
        Ok(txt) => match toml::from_str::<T>(&txt) {
            Ok(result) => Ok(result),
            Err(e) => {
                log::error!("Loading {} toml file '{}': {}", T::FILE_TYPE, path.as_ref().display(), e);
                Err(IcyError::ErrorLoadingFile(T::FILE_TYPE.to_string(), path.as_ref().to_string_lossy().to_string(), e.to_string()).into())
            }
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(IcyError::FileNotFound(T::FILE_TYPE.to_string(), path.as_ref().to_string_lossy().to_string()).into())
            } else {
                log::error!("Loading {} file '{}': {}", T::FILE_TYPE, path.as_ref().display(), e);
                Err(IcyError::ErrorLoadingFile(T::FILE_TYPE.to_string(), path.as_ref().to_string_lossy().to_string(), e.to_string()).into())
            }
        }
    }
}

pub(crate) fn save_internal<T: IcyBoardSerializer, P: AsRef<Path>>(s: &T, path: &P) -> Res<()> {
    match toml::to_string(s) {
        Ok(txt) => match fs::write(path, txt) {
            Ok(_) => Ok(()),
            Err(e) => {
                log::error!("Error writing {} file '{}': {}", T::FILE_TYPE, path.as_ref().display(), e);
                Err(IcyError::ErrorGeneratingToml(path.as_ref().to_string_lossy().to_string(), e.to_string()).into())
            }
        },
        Err(e) => {
            log::error!("Error generating {} toml file '{}': {}", T::FILE_TYPE, path.as_ref().display(), e);
            Err(IcyError::ErrorGeneratingToml(path.as_ref().to_string_lossy().to_string(), e.to_string()).into())
        }
    }
}

pub trait IcyBoardSerializer: serde::de::DeserializeOwned + serde::ser::Serialize {
    const FILE_TYPE: &'static str;

    fn load<P: AsRef<Path>>(path: &P) -> Res<Self> {
        load_internal::<Self, P>(path)
    }

    fn save<P: AsRef<Path>>(&self, path: &P) -> Res<()> {
        save_internal::<Self, P>(self, path)
    }
}

pub trait PCBoardImport: Sized + Default + IcyBoardSerializer {
    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self>;
}
pub trait PCBoardRecordImporter<T>: Sized + Default {
    const RECORD_SIZE: usize;

    fn push(&mut self, value: T);

    fn load_pcboard_record(record: &[u8]) -> Res<T>;

    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        let mut res = Self::default();
        match &std::fs::read(path) {
            Ok(data) => {
                let mut data = &data[..];
                while !data.is_empty() {
                    if data.len() < Self::RECORD_SIZE {
                        log::error!("Importing file '{}' from pcboard binary file ended prematurely", path.as_ref().display(),);
                        return Err(IcyBoardError::InvalidRecordSize(path.as_ref().display().to_string(), Self::RECORD_SIZE, data.len()).into());
                    }
                    match Self::load_pcboard_record(&data[..Self::RECORD_SIZE]) {
                        Ok(value) => {
                            res.push(value);
                        }
                        Err(e) => {
                            return Err(IcyBoardError::ImportRecordErorr(path.as_ref().display().to_string(), e.to_string()).into());
                        }
                    }

                    data = &data[Self::RECORD_SIZE..];
                }
                Ok(res)
            }
            Err(err) => {
                log::error!("Importing file '{}' from pcboard binary file: {}", path.as_ref().display(), err);
                Err(IcyError::ErrorLoadingFile("PCBOARD BIN FILE".to_string(), path.as_ref().to_string_lossy().to_string(), err.to_string()).into())
            }
        }
    }
}

pub trait PCBoardBinImporter: Sized + Default {
    const SIZE: usize;

    fn import_data(data: &[u8]) -> Res<Self>;

    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        match &std::fs::read(path) {
            Ok(data) => {
                if data.len() < Self::SIZE {
                    log::error!("Importing file '{}' from pcboard binary file ended prematurely", path.as_ref().display(),);
                    return Err(IcyBoardError::InvalidRecordSize(path.as_ref().display().to_string(), Self::SIZE, data.len()).into());
                }
                Self::import_data(data)
            }
            Err(err) => {
                log::error!("Importing file '{}' from pcboard binary file: {}", path.as_ref().display(), err);
                Err(IcyError::ErrorLoadingFile("PCBOARD BIN FILE".to_string(), path.as_ref().to_string_lossy().to_string(), err.to_string()).into())
            }
        }
    }
}

pub trait PCBoardTextImport: PCBoardImport {
    fn import_data(data: String) -> Res<Self>;

    fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        match read_with_encoding_detection(path) {
            Ok(data) => Self::import_data(data),
            Err(err) => {
                log::error!("Importing file '{}' from pcboard binary file: {}", path.as_ref().display(), err);
                Err(IcyError::ErrorLoadingFile("PCBOARD TEXT FILE".to_string(), path.as_ref().to_string_lossy().to_string(), err.to_string()).into())
            }
        }
    }
}
