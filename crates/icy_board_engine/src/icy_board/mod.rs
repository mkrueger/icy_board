use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Component, Path, PathBuf},
};

use crate::Res;
use accounting_cfg::AccountingConfig;
use bulletins::BullettinList;
use codepages::tables::write_utf8_with_bom;
use surveys::SurveyList;
use thiserror::Error;

use crate::vm::errors::IcyError;

use self::{
    commands::CommandList,
    conferences::ConferenceBase,
    doors::DoorList,
    events::EventList,
    file_directory::DirectoryList,
    ftn::FtnConfig,
    group_list::GroupList,
    icb_config::IcbConfig,
    icb_text::IcbTextFile,
    language::SupportedLanguages,
    message_area::AreaList,
    pcbconferences::{PcbAdditionalConferenceHeader, PcbConferenceHeader, PcbLegacyConferenceHeader},
    pcboard_data::PcbBoardData,
    sec_levels::SecurityLevelDefinitions,
    statistics::Statistics,
    user_base::UserBase,
    xfer_protocols::SupportedProtocols,
};

pub mod bbs;
pub mod bulletins;
pub mod commands;
pub mod conferences;
pub mod doors;
pub mod events;
pub mod file_directory;
pub mod ftn;
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

    #[error("Error node not found ({0})")]
    NodeNotFound(usize),
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
    pub ftn: FtnConfig,
    pub events: EventList,
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
            ftn: FtnConfig::default(),
            events: EventList::default(),
        }
    }

    pub fn resolve_paths(&mut self) {
        // Core system paths
        self.config.paths.help_path = get_path(&self.root_path, &self.config.paths.help_path);
        self.config.paths.tmp_work_path = get_path(&self.root_path, &self.config.paths.tmp_work_path);
        self.config.paths.icbtext = get_path(&self.root_path, &self.config.paths.icbtext);
        self.config.paths.conferences = get_path(&self.root_path, &self.config.paths.conferences);
        self.config.paths.security_file_path = get_path(&self.root_path, &self.config.paths.security_file_path);
        self.config.paths.command_display_path = get_path(&self.root_path, &self.config.paths.command_display_path);
        self.config.paths.email_msgbase = get_path(&self.root_path, &self.config.paths.email_msgbase);
        self.config.paths.caller_log = get_path(&self.root_path, &self.config.paths.caller_log);

        // User/group/command files
        self.config.paths.user_file = get_path(&self.root_path, &self.config.paths.user_file);
        self.config.paths.group_file = get_path(&self.root_path, &self.config.paths.group_file);
        self.config.paths.command_file = get_path(&self.root_path, &self.config.paths.command_file);

        // Display files
        self.config.paths.welcome = get_path(&self.root_path, &self.config.paths.welcome);
        self.config.paths.newuser = get_path(&self.root_path, &self.config.paths.newuser);
        self.config.paths.closed = get_path(&self.root_path, &self.config.paths.closed);
        self.config.paths.expire_warning = get_path(&self.root_path, &self.config.paths.expire_warning);
        self.config.paths.expired = get_path(&self.root_path, &self.config.paths.expired);
        self.config.paths.conf_join_menu = get_path(&self.root_path, &self.config.paths.conf_join_menu);
        self.config.paths.no_ansi = get_path(&self.root_path, &self.config.paths.no_ansi);

        // Chat files
        self.config.paths.chat_intro_file = get_path(&self.root_path, &self.config.paths.chat_intro_file);
        self.config.paths.chat_menu = get_path(&self.root_path, &self.config.paths.chat_menu);
        self.config.paths.chat_actions_menu = get_path(&self.root_path, &self.config.paths.chat_actions_menu);

        // Config files
        self.config.paths.language_file = get_path(&self.root_path, &self.config.paths.language_file);
        self.config.paths.protocol_data_file = get_path(&self.root_path, &self.config.paths.protocol_data_file);
        self.config.paths.pwrd_sec_level_file = get_path(&self.root_path, &self.config.paths.pwrd_sec_level_file);
        self.config.paths.statistics_file = get_path(&self.root_path, &self.config.paths.statistics_file);
        self.config.paths.ftn_file = get_path(&self.root_path, &self.config.paths.ftn_file);

        self.config.event.event_file = get_path(&self.root_path, &self.config.event.event_file);

        // Fidonet mail spool
        self.ftn.inbound = get_path(&self.root_path, &self.ftn.inbound);
        self.ftn.outbound = get_path(&self.root_path, &self.ftn.outbound);
        self.ftn.netmail = get_path(&self.root_path, &self.ftn.netmail);
        self.ftn.bad_netmail = get_path(&self.root_path, &self.ftn.bad_netmail);
        self.ftn.new_areas = get_path(&self.root_path, &self.ftn.new_areas);

        // Trashcan files
        self.config.paths.trashcan_upload_files = get_path(&self.root_path, &self.config.paths.trashcan_upload_files);
        self.config.paths.trashcan_email = get_path(&self.root_path, &self.config.paths.trashcan_email);
        self.config.paths.trashcan_passwords = get_path(&self.root_path, &self.config.paths.trashcan_passwords);
        self.config.paths.trashcan_user = get_path(&self.root_path, &self.config.paths.trashcan_user);
        self.config.paths.vip_users = get_path(&self.root_path, &self.config.paths.vip_users);

        // Survey files
        self.config.paths.newask_answer = get_path(&self.root_path, &self.config.paths.newask_answer);
        self.config.paths.newask_survey = get_path(&self.root_path, &self.config.paths.newask_survey);
        self.config.paths.logon_answer = get_path(&self.root_path, &self.config.paths.logon_answer);
        self.config.paths.logon_survey = get_path(&self.root_path, &self.config.paths.logon_survey);
        self.config.paths.logoff_answer = get_path(&self.root_path, &self.config.paths.logoff_answer);
        self.config.paths.logoff_survey = get_path(&self.root_path, &self.config.paths.logoff_survey);

        // Conference paths
        for c in self.conferences.iter_mut() {
            c.command_file = get_path(&self.root_path, &c.command_file);
            c.intro_file = get_path(&self.root_path, &c.intro_file);

            c.area_file = get_path(&self.root_path, &c.area_file);
            c.area_menu = get_path(&self.root_path, &c.area_menu);

            c.dir_file = get_path(&self.root_path, &c.dir_file);
            c.dir_menu = get_path(&self.root_path, &c.dir_menu);

            c.doors_file = get_path(&self.root_path, &c.doors_file);
            c.doors_menu = get_path(&self.root_path, &c.doors_menu);

            c.blt_file = get_path(&self.root_path, &c.blt_file);
            c.blt_menu = get_path(&self.root_path, &c.blt_menu);

            c.survey_file = get_path(&self.root_path, &c.survey_file);
            c.survey_menu = get_path(&self.root_path, &c.survey_menu);

            c.pub_upload_location = get_path(&self.root_path, &c.pub_upload_location);
            c.private_upload_location = get_path(&self.root_path, &c.private_upload_location);

            if let Some(areas) = &mut c.areas {
                for area in areas.iter_mut() {
                    area.path = get_path(&self.root_path, &area.path);
                }
            }
            if let Some(directories) = &mut c.directories {
                for dir in directories.iter_mut() {
                    dir.path = get_path(&self.root_path, &dir.path);
                    dir.metadata_path = get_path(&self.root_path, &dir.metadata_path);
                }
            }

            if let Some(blt) = &mut c.bulletins {
                for dir in blt.iter_mut() {
                    dir.path = get_path(&self.root_path, &dir.path);
                }
            }

            if let Some(surveys) = &mut c.surveys {
                for survey in surveys.iter_mut() {
                    survey.survey_file = get_path(&self.root_path, &survey.survey_file);
                    survey.answer_file = get_path(&self.root_path, &survey.answer_file);
                }
            }
        }
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
        return lookup_case_insensitive(&s);
    }

    pub fn load<P: AsRef<Path>>(path: &P) -> Res<Self> {
        let mut config = IcbConfig::load(path).map_err(|e| {
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

        let users_path = get_path(parent_path, &config.paths.user_file);
        let users = UserBase::load(&users_path).map_err(|e| {
            log::error!("Error loading users: {} from {}", e, users_path.display());
            e
        })?;

        let load_path = get_path(parent_path, &config.paths.conferences);
        let conferences = ConferenceBase::load(&load_path).map_err(|e| {
            log::error!("Error loading conference base: {} from {}", e, load_path.display());
            e
        })?;

        let load_path: PathBuf = get_path(parent_path, &config.paths.icbtext);
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

        let ftn = if config.paths.ftn_file.as_os_str().is_empty() {
            FtnConfig::default()
        } else {
            let load_path = get_path(parent_path, &config.paths.ftn_file);
            match FtnConfig::load(&load_path) {
                Ok(ftn) => ftn,
                Err(e) => {
                    log::error!("Error loading ftn config: {} from {}, generating default.", e, load_path.display());
                    FtnConfig::default()
                }
            }
        };

        let events = if config.event.event_file.as_os_str().is_empty() {
            EventList::default()
        } else {
            let load_path = get_path(parent_path, &config.event.event_file);
            match EventList::load(&load_path) {
                Ok(events) => events,
                Err(e) => {
                    log::error!("Error loading events: {} from {}, generating default.", e, load_path.display());
                    EventList::default()
                }
            }
        };

        let load_path = get_path(parent_path, &config.accounting.cfg_file);
        match AccountingConfig::load(&load_path) {
            Ok(acc) => {
                config.accounting.accounting_config = Some(acc);
            }
            Err(e) => {
                if config.accounting.enabled {
                    log::error!("Error loading accounting: {} from {}", e, load_path.display());
                }
            }
        }

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
            ftn,
            events,
        };

        for conf in board.conferences.iter_mut() {
            // A conference command list takes priority over the global one.
            let command_file = if conf.command_file.is_absolute() {
                conf.command_file.clone()
            } else {
                board.root_path.join(&conf.command_file)
            };
            if command_file.is_file() {
                match CommandList::load(&command_file) {
                    Ok(commands) => {
                        conf.commands = commands.commands;
                    }
                    Err(err) => {
                        log::error!("Error loading conference commands {}: {}", command_file.display(), err);
                    }
                }
            }

            let area_file = if conf.area_file.is_absolute() {
                conf.area_file.clone()
            } else {
                board.root_path.join(&conf.area_file)
            };
            if area_file.is_file() {
                match AreaList::load(&area_file) {
                    Ok(mut areas) => {
                        for area in areas.iter_mut() {
                            if !area.path.is_absolute() {
                                area.path = board.root_path.join(&area.path);
                            }
                        }
                        conf.areas = Some(areas);
                    }
                    Err(err) => {
                        log::error!("Error loading message areas {}: {}", area_file.display(), err);
                    }
                }
            }

            let dir_file = if conf.dir_file.is_absolute() {
                conf.dir_file.clone()
            } else {
                board.root_path.join(&conf.dir_file)
            };
            if dir_file.is_file() {
                match DirectoryList::load(&dir_file) {
                    Ok(directories) => {
                        conf.directories = Some(directories);
                    }
                    Err(err) => {
                        log::error!("Error loading file areas {}: {}", dir_file.display(), err);
                    }
                }
            }

            let doors_file = if conf.doors_file.is_absolute() {
                conf.doors_file.clone()
            } else {
                board.root_path.join(&conf.doors_file)
            };
            if doors_file.is_file() {
                match DoorList::load(&doors_file) {
                    Ok(doors) => {
                        conf.doors = Some(doors);
                    }
                    Err(err) => {
                        log::error!("loading door files {}: {}", doors_file.display(), err);
                    }
                }
            }

            let blt_file = if conf.blt_file.is_absolute() {
                conf.blt_file.clone()
            } else {
                board.root_path.join(&conf.blt_file)
            };
            if blt_file.is_file() {
                match BullettinList::load(&blt_file) {
                    Ok(blts) => {
                        conf.bulletins = Some(blts);
                    }
                    Err(err) => {
                        log::error!("loading door files {}: {}", blt_file.display(), err);
                    }
                }
            }

            let survey_file = if conf.survey_file.is_absolute() {
                conf.survey_file.clone()
            } else {
                board.root_path.join(&conf.survey_file)
            };
            if survey_file.is_file() {
                match SurveyList::load(&survey_file) {
                    Ok(surveys) => {
                        conf.surveys = Some(surveys);
                    }
                    Err(err) => {
                        log::error!("loading door files {}: {}", survey_file.display(), err);
                    }
                }
            }
        }

        Ok(board)
    }

    pub fn save(&self) -> Res<()> {
        self.config.save(&self.file_name)?;
        self.conferences.save(&self.resolve_file(&self.config.paths.conferences))?;
        if !self.config.paths.ftn_file.as_os_str().is_empty() {
            self.ftn.save(&self.resolve_file(&self.config.paths.ftn_file))?;
        }
        Ok(())
    }

    pub fn save_userbase(&mut self) -> Res<()> {
        let users_file = PathBuf::from(self.resolve_file(&self.config.paths.user_file));
        if let Err(e) = self.users.save(&users_file) {
            log::error!("Error saving user base: {}", e);
            Err(e)
        } else {
            Ok(())
        }
    }

    pub fn export_pcboard(&self, file: &Path) -> Res<()> {
        let base_loc = file.parent().unwrap();
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
        pcb_dat.path.sec_loc = self.resolve_file(&self.config.paths.user_file).to_string_lossy().to_string();

        // Line 29  Name/Location of USERS File
        let users_file = base_loc.join("users");
        let users_inf_file = base_loc.join("users.inf");
        self.users.export_pcboard(&users_file, &users_inf_file)?;

        pcb_dat.path.usr_file = users_file.to_string_lossy().to_string();

        // Line 31
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

        // Line 180
        pcb_dat.path.inf_file = users_inf_file.to_string_lossy().to_string();

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

        // Line 72
        pcb_dat.disable_quick = self.config.system_control.disable_ns_logon;
        // Line 85
        pcb_dat.last_read_update = self.config.message.update_last_read_pointer;
        // Line 107 - dropped in PCBoard 15.0, the flag now lives in the conference record
        pcb_dat.pub_conf = self
            .conferences
            .iter()
            .take(40)
            .enumerate()
            .map(|(i, c)| if i == 0 || c.is_public { 'X' } else { ' ' })
            .collect();
        // Line 170
        pcb_dat.max_total_msgs = self.config.qwk_settings.max_msgs as i32;
        // Line 171
        pcb_dat.max_conf_msgs = self.config.qwk_settings.max_msgs_per_conf as i32;
        // Line 191
        pcb_dat.log_caller_number = self.config.options.log_caller_number;
        // Line 192
        pcb_dat.log_connect_str = self.config.options.log_connect_string;
        // Line 193
        pcb_dat.log_sec_level = self.config.options.log_security_level;
        // Line 194
        pcb_dat.conf_pwrd_adjust = self.config.system_control.reread_sec_level_on_join;
        // Line 195
        pcb_dat.confirm_caller = self.config.system_control.confirm_caller_name;
        // Line 207
        pcb_dat.force_main = self.config.message.force_comments_to_main;
        // Line 215
        pcb_dat.auto_reg_conf = self.config.new_user_settings.auto_register_conferences;
        // Line 251
        pcb_dat.qwk_file = self.config.qwk_settings.bbs_id.clone();

        // The fido block, lines 173-177, 232-236 and 336-347. PCBoard keeps the
        // addresses and the links in the files under FidoLoc, only the options
        // that steer the tosser are in here.
        pcb_dat.enable_fido = self.ftn.is_configured();
        pcb_dat.fido_process_in = self.ftn.options.process_in;
        pcb_dat.fido_process_out = self.ftn.options.process_out;
        pcb_dat.fido_process_orphan = self.ftn.options.process_orphan;
        pcb_dat.fido_dial_out = self.ftn.options.dial_out;
        pcb_dat.fido_import_after_xfer = self.ftn.options.import_after_xfer;
        pcb_dat.fido_check_dupe_msg_id = self.ftn.options.check_dupe_msg_id;
        pcb_dat.fido_check_dupe_path = self.ftn.options.check_dupe_path;
        pcb_dat.fido_num_msgs_to_track = self.ftn.options.msgs_to_track.min(i32::MAX as u32) as i32;
        pcb_dat.fido_secure = self.ftn.options.secure;
        pcb_dat.fido_sysop_change = self.ftn.options.sysop_change;
        pcb_dat.fido_auto_add = self.ftn.options.auto_add;
        pcb_dat.fido_enable_pass_thru = self.ftn.options.pass_thru;
        pcb_dat.fido_default_zone = self.ftn.options.default_zone as i32;
        pcb_dat.fido_default_net = self.ftn.options.default_net as i32;
        pcb_dat.fido_log_level = self.ftn.options.verbose_log as i32;

        // Line 296 (to prevent \0 char)
        pcb_dat.uucp_high_ascii = 'N';
        let res = pcb_dat.serialize(crate::parser::Encoding::CP437);
        fs::write(file, res)?;

        Ok(())
    }

    /// PCBoard's records hold 32 byte paths, so what is exported stays relative to the board.
    fn export_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root_path).unwrap_or(path).to_string_lossy().to_string()
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
            let dir_file = if let Ok(area_list) = DirectoryList::load(&conf.dir_file) {
                area_list.export_pcboard(&dir_file)?;
                self.export_path(&dir_file)
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
                users_menu: self.export_path(&conf.users_menu),
                sysop_menu: self.export_path(&conf.sysop_menu),
                news_file: self.export_path(&conf.news_file),
                pub_upload_sort: conf.pub_upload_sort,
                pub_upload_dirfile: self.export_path(&conf.pub_upload_metadata),
                pub_upload_location: self.export_path(&conf.pub_upload_location),
                private_upload_sort: conf.private_upload_sort,
                private_upload_dirfile: self.export_path(&conf.private_upload_metadata),
                private_upload_location: self.export_path(&conf.private_upload_location),
                public_conference: conf.is_public,
                doors_menu: self.export_path(&conf.doors_menu),
                doors_file: self.export_path(&conf.doors_file),
                required_security: conf.required_security.level(),
                blt_menu: self.export_path(&conf.blt_menu),
                blt_file: self.export_path(&conf.blt_file),
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
                user_menu: self.export_path(&conf.users_menu),
                sysop_menu: self.export_path(&conf.sysop_menu),
                news_file: self.export_path(&conf.news_file),
                pub_upld_sort: conf.pub_upload_sort,
                upld_dir: self.export_path(&conf.pub_upload_metadata),
                pub_upld_loc: self.export_path(&conf.pub_upload_location),
                prv_upld_sort: conf.private_upload_sort,
                priv_dir: self.export_path(&conf.private_upload_metadata),
                prv_upld_loc: self.export_path(&conf.private_upload_location),
                drs_menu: self.export_path(&conf.doors_menu),
                drs_file: self.export_path(&conf.doors_file),
                blt_menu: self.export_path(&conf.blt_menu),
                blt_name_loc: self.export_path(&conf.blt_file),
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
                attach_loc: self.export_path(&conf.attachment_location),
                cmd_lst: self.export_path(&conf.command_file),
                intro: self.export_path(&conf.intro_file),
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
    /*
    pub fn set_user(&mut self, new_user: User, i: usize) -> Res<()> {
        let home_dir = UserBase::get_user_home_dir(&self.config.paths.user_file, new_user.get_name());
        std::fs::create_dir_all(&home_dir).unwrap();
        let user_txt = toml::to_string(&new_user)?;
        fs::write(home_dir.join("user.toml"), user_txt)?;
        self.users[i] = new_user;
        Ok(())
    }*/

    pub fn save_statistics(&self) -> Res<()> {
        let r = &self.config.paths.statistics_file;
        if let Err(err) = self.statistics.save(&r) {
            log::error!("Error saving statistics to {} : {err}", r.display());
        }
        Ok(())
    }
}

/// PPEs and PCBoard configurations name their files the way DOS did, so the case of a
/// path says nothing about what is on disk.
pub fn lookup_case_insensitive(path: &Path) -> PathBuf {
    if path.exists() {
        return path.to_path_buf();
    }
    let mut resolved = PathBuf::new();
    let mut corrected = false;
    for component in path.components() {
        let Component::Normal(name) = component else {
            resolved.push(component.as_os_str());
            continue;
        };
        if resolved.join(name).exists() {
            resolved.push(name);
            continue;
        }
        match entry_ignoring_case(&resolved, name) {
            Some(found) => {
                resolved.push(found);
                corrected = true;
            }
            // Nothing on disk answers to this name, so the rest is taken as written.
            None => resolved.push(name),
        }
    }
    if corrected { resolved } else { path.to_path_buf() }
}

fn entry_ignoring_case(dir: &Path, name: &OsStr) -> Option<OsString> {
    let dir = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
    let name = name.to_str()?;
    fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name())
        .find(|entry| entry.to_str().is_some_and(|entry| entry.eq_ignore_ascii_case(name)))
}

fn get_path(parent_path: &Path, home_dir: &PathBuf) -> PathBuf {
    if home_dir.as_os_str().is_empty() {
        return PathBuf::new();
    }
    let res: PathBuf = if home_dir.is_absolute() {
        home_dir.clone()
    } else {
        parent_path.join(home_dir)
    };

    res
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
    write_utf8_with_bom(to, &import)?;
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

/// Writes to a temporary file in the target directory and renames it into place,
/// so a crashing or failing write can never leave a half-written config behind.
pub(crate) fn save_atomic_internal<T: IcyBoardSerializer, P: AsRef<Path>>(s: &T, path: &P) -> Res<()> {
    use std::io::Write as _;

    let path = path.as_ref();
    let txt = match toml::to_string(s) {
        Ok(txt) => txt,
        Err(e) => {
            log::error!("Error generating {} toml file '{}': {}", T::FILE_TYPE, path.display(), e);
            return Err(IcyError::ErrorGeneratingToml(path.to_string_lossy().to_string(), e.to_string()).into());
        }
    };

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let map_io = |e: std::io::Error| -> Box<dyn std::error::Error + Send + Sync> {
        log::error!("Error writing {} file '{}': {}", T::FILE_TYPE, path.display(), e);
        IcyError::ErrorGeneratingToml(path.to_string_lossy().to_string(), e.to_string()).into()
    };

    let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(map_io)?;
    tmp.write_all(txt.as_bytes()).map_err(map_io)?;
    tmp.as_file().sync_all().map_err(map_io)?;

    if let Ok(meta) = fs::metadata(path) {
        let _ = tmp.as_file().set_permissions(meta.permissions());
    }

    tmp.persist(path).map_err(|e| map_io(e.error))?;
    Ok(())
}

pub trait IcyBoardSerializer: serde::de::DeserializeOwned + serde::ser::Serialize {
    const FILE_TYPE: &'static str;

    fn load<P: AsRef<Path>>(path: &P) -> Res<Self> {
        load_internal::<Self, P>(path)
    }

    fn save<P: AsRef<Path>>(&self, path: &P) -> Res<()> {
        save_internal::<Self, P>(self, path)
    }

    /// Crash-safe variant of [`IcyBoardSerializer::save`].
    fn save_atomic<P: AsRef<Path>>(&self, path: &P) -> Res<()> {
        save_atomic_internal::<Self, P>(self, path)
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

/// Tests of the path lookup that stands in for the case insensitive file system PCBoard had.
#[cfg(test)]
mod tests {
    use super::*;

    fn board(files: &[&str]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for file in files {
            let path = root.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        root
    }

    #[test]
    fn test_a_path_that_is_there_is_handed_back_untouched() {
        let root = board(&["gen/BRDM.PPE"]);
        let path = root.path().join("gen/BRDM.PPE");
        assert_eq!(lookup_case_insensitive(&path), path);
    }

    #[test]
    fn test_a_dos_name_finds_the_file_it_means() {
        let root = board(&["gen/brdm.ppe"]);
        assert_eq!(lookup_case_insensitive(&root.path().join("GEN/BRDM.PPE")), root.path().join("gen/brdm.ppe"));
    }

    #[test]
    fn test_every_directory_on_the_way_is_looked_up() {
        let root = board(&["Ppe/Door/setup.cfg"]);
        assert_eq!(
            lookup_case_insensitive(&root.path().join("PPE/DOOR/SETUP.CFG")),
            root.path().join("Ppe/Door/setup.cfg")
        );
    }

    #[test]
    fn test_a_file_that_is_nowhere_leaves_the_path_as_it_was() {
        let root = board(&["gen/brdm.ppe"]);
        let path = root.path().join("GEN/NOTHERE.PPE");
        assert_eq!(lookup_case_insensitive(&path), root.path().join("gen/NOTHERE.PPE"));
    }

    #[test]
    fn test_a_path_no_part_of_which_exists_is_left_alone() {
        let path = Path::new("/no/such/place/AT/ALL.PPE");
        assert_eq!(lookup_case_insensitive(path), path);
    }
}
