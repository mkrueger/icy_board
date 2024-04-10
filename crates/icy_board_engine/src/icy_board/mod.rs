use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use icy_ppe::Res;
use qfile::{QFilePath, QTraitSync};
use relative_path::RelativePath;
use thiserror::Error;

use crate::vm::errors::IcyError;

use self::{
    commands::CommandList,
    conferences::ConferenceBase,
    file_areas::FileAreaList,
    group_list::GroupList,
    icb_config::IcbConfig,
    icb_text::IcbTextFile,
    language::SupportedLanguages,
    pcbconferences::{PcbAdditionalConferenceHeader, PcbConferenceHeader, PcbLegacyConferenceHeader},
    pcboard_data::PcbBoardData,
    sec_levels::SecurityLevelDefinitions,
    statistics::Statistics,
    user_base::UserBase,
    xfer_protocols::SupportedProtocols,
};

pub mod archivers;
pub mod bulletins;
pub mod commands;
pub mod conferences;
pub mod file_areas;
pub mod group_list;
pub mod icb_config;
pub mod icb_text;
pub mod language;
pub mod menu;
pub mod output;
pub mod pcb;
pub mod sec_levels;
pub mod security;
pub mod state;
pub mod statistics;
pub mod surveys;
pub mod user_base;
pub mod xfer_protocols;

pub use pcb::*;

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
}

pub struct IcyBoard {
    pub root_path: PathBuf,
    pub users: UserBase,
    pub config: IcbConfig,
    pub conferences: ConferenceBase,
    pub display_text: IcbTextFile,

    // TODO: proper board statistics.
    pub num_callers: usize,

    pub languages: SupportedLanguages,
    pub protocols: SupportedProtocols,
    pub sec_levels: SecurityLevelDefinitions,
    pub groups: GroupList,
    pub statistics: Statistics,
    pub commands: CommandList,
}

impl IcyBoard {
    pub fn new() -> Self {
        let display_text = IcbTextFile::default();

        IcyBoard {
            display_text,
            root_path: PathBuf::new(),
            users: UserBase::default(),
            config: IcbConfig::new(),
            conferences: ConferenceBase::default(),
            num_callers: 0,
            languages: SupportedLanguages::default(),
            protocols: SupportedProtocols::default(),
            sec_levels: SecurityLevelDefinitions::default(),
            commands: CommandList::default(),
            statistics: Statistics::default(),
            groups: GroupList::default(),
        }
    }

    pub fn resolve_file<P: AsRef<Path>>(&self, file: &P) -> String {
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

        if !s.starts_with('/') {
            s = self.root_path.join(s).to_string_lossy().to_string();
        }

        if let Ok(mut file_path) = QFilePath::add_path(s.clone()) {
            if let Ok(file) = file_path.get_path_buf() {
                return file.to_string_lossy().to_string();
            }
        }
        s
    }

    pub fn load<P: AsRef<Path>>(path: &P) -> Res<Self> {
        let config = IcbConfig::load(path)?;

        let parent_path = path.as_ref().parent().unwrap();
        let load_path = &RelativePath::from_path(&config.paths.user_base)?.to_path(parent_path);
        let users = UserBase::load(&load_path).map_err(|e| {
            log::error!("Error loading user base: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = RelativePath::from_path(&config.paths.conferences)?.to_path(parent_path);
        let conferences = ConferenceBase::load(&load_path).map_err(|e| {
            log::error!("Error loading conference base: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = RelativePath::from_path(&config.paths.icbtext)?.to_path(parent_path);
        let display_text = IcbTextFile::load(&load_path).map_err(|e| {
            log::error!("Error loading display text: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = &RelativePath::from_path(&config.paths.language_file)?.to_path(parent_path);
        let languages = SupportedLanguages::load(load_path).map_err(|e| {
            log::error!("Error loading languages: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = RelativePath::from_path(&config.paths.protocol_data_file)?.to_path(parent_path);
        let protocols = SupportedProtocols::load(&load_path).map_err(|e| {
            log::error!("Error loading protocols: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = RelativePath::from_path(&config.paths.security_level_file)?.to_path(parent_path);
        let sec_levels = SecurityLevelDefinitions::load(&load_path).map_err(|e| {
            log::error!("Error loading security levels: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = &RelativePath::from_path(&config.paths.command_file)?.to_path(parent_path);
        let commands = CommandList::load(load_path).map_err(|e| {
            log::error!("Error loading commands: {} from {}", e, load_path.display());
            e
        })?;

        let load_path = RelativePath::from_path(&config.paths.statistics_file)?.to_path(parent_path);
        let statistics = Statistics::load(&load_path).map_err(|e| {
            log::error!("Error loading statistics: {} from {}", e, load_path.display());
            e
        })?;

        Ok(IcyBoard {
            root_path: path.as_ref().parent().unwrap().canonicalize()?,
            num_callers: 0,
            users,
            config,
            conferences,
            display_text,
            languages,
            protocols,
            sec_levels,
            commands,
            statistics,
            groups: GroupList::default(),
        })
    }

    pub fn export_pcboard(&self, file: &Path) -> Res<()> {
        let mut pcb_dat = PcbBoardData::default();

        pcb_dat.sysop_info.sysop = self.config.sysop.name.to_string();
        pcb_dat.sysop_info.password = self.config.sysop.password.to_string();
        pcb_dat.sysop_info.require_pwrd_to_exit = self.config.sysop.require_password_to_exit;
        pcb_dat.sysop_info.use_real_name = self.config.sysop.use_real_name;

        pcb_dat.sysop_security.sysop = self.config.sysop_security_level.sysop as i32;
        pcb_dat.board_name = self.config.board.name.to_string();

        pcb_dat.path.help_loc = self.resolve_file(&self.config.paths.help_path);

        let base_loc = file.parent().unwrap();

        let cnames = base_loc.join("cnames");
        self.export_conference_files(&base_loc, &cnames)?;
        pcb_dat.path.conference_file = cnames.to_string_lossy().to_string();

        pcb_dat.num_conf = self.conferences.len() as i32 - 1;
        pcb_dat.path.sec_loc = self.resolve_file(&self.config.paths.security_level_file);
        pcb_dat.path.cmd_display_files_loc = self.resolve_file(&self.config.paths.command_display_path);
        pcb_dat.path.welcome_file = self.resolve_file(&self.config.paths.welcome);
        pcb_dat.path.newuser_file = self.resolve_file(&self.config.paths.newuser);
        pcb_dat.path.closed_file = self.resolve_file(&self.config.paths.closed);
        pcb_dat.path.warning_file = self.resolve_file(&self.config.paths.warning);
        pcb_dat.path.expired_file = self.resolve_file(&self.config.paths.expired);
        pcb_dat.path.conf_menu = self.resolve_file(&self.config.paths.conf_join_menu);
        pcb_dat.path.group_chat = self.resolve_file(&self.config.paths.group_chat);
        pcb_dat.path.no_ansi = self.resolve_file(&self.config.paths.no_ansi);

        let res = pcb_dat.serialize(icy_ppe::parser::Encoding::CP437);
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
            let dir_file = base_loc.join(&format!("dir{}", dirs));
            let area_list = FileAreaList::load(&self.resolve_file(&conf.file_area_file))?;

            area_list.export_pcboard(&dir_file)?;
            let dir_file = dir_file.to_string_lossy().to_string();
            let len = self.root_path.to_string_lossy().len() + 1;
            let dir_file = dir_file[len..].to_string();
            log::warn!("dir_file: {}", dir_file);

            let header = PcbConferenceHeader {
                name: conf.name.clone(),
                auto_rejoin: conf.auto_rejoin,
                view_members: conf.view_members,
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
                pub_upload_dirfile: conf.pub_upload_dir_file.to_string_lossy().to_string(),
                pub_upload_location: conf.pub_upload_location.to_string_lossy().to_string(),
                private_upload_sort: conf.private_upload_sort,
                private_upload_dirfile: conf.private_upload_dir_file.to_string_lossy().to_string(),
                private_upload_location: conf.private_upload_location.to_string_lossy().to_string(),
                public_conference: conf.is_public,
                doors_menu: conf.doors_menu.to_string_lossy().to_string(),
                doors_file: conf.doors_file.to_string_lossy().to_string(),
                required_security: conf.required_security.level(),
                blt_menu: conf.blt_menu.to_string_lossy().to_string(),
                blt_file: conf.blt_file.to_string_lossy().to_string(),
                script_menu: conf.survey_menu.to_string_lossy().to_string(),
                script_file: conf.survey_file.to_string_lossy().to_string(),
                dir_menu: String::new(),
                dir_file: dir_file.to_string(),
                dlpth_list_file: String::new(),
            };
            headers.extend(header.serialize());

            let legacy_header = PcbLegacyConferenceHeader {
                name: conf.name.clone(),
                auto_rejoin: conf.auto_rejoin,
                view_members: conf.view_members,
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
                upld_dir: conf.pub_upload_dir_file.to_string_lossy().to_string(),
                pub_upld_loc: conf.pub_upload_location.to_string_lossy().to_string(),
                prv_upld_sort: conf.private_upload_sort,
                priv_dir: conf.private_upload_dir_file.to_string_lossy().to_string(),
                prv_upld_loc: conf.private_upload_location.to_string_lossy().to_string(),
                drs_menu: conf.doors_menu.to_string_lossy().to_string(),
                drs_file: conf.doors_file.to_string_lossy().to_string(),
                blt_menu: conf.blt_menu.to_string_lossy().to_string(),
                blt_name_loc: conf.blt_file.to_string_lossy().to_string(),
                scr_menu: conf.survey_menu.to_string_lossy().to_string(),
                scr_name_loc: conf.survey_file.to_string_lossy().to_string(),
                dir_menu: String::new(),
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

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

pub fn read_with_encoding_detection<P: AsRef<Path>>(path: &P) -> Res<String> {
    match fs::read(path) {
        Ok(data) => {
            let import = if data.starts_with(&UTF8_BOM) {
                String::from_utf8_lossy(&data[UTF8_BOM.len()..]).to_string()
            } else {
                icy_ppe::tables::import_cp437_string(&data, false)
            };
            Ok(import)
        }
        Err(e) => Err(IcyBoardError::FileError(path.as_ref().to_path_buf(), e.to_string()).into()),
    }
}

pub fn write_with_bom<P: AsRef<Path>>(path: &P, buf: &str) -> Res<()> {
    match fs::File::create(path) {
        Ok(mut file) => {
            file.write_all(&UTF8_BOM)?;
            file.write_all(buf.as_bytes())?;
        }
        Err(e) => return Err(IcyBoardError::ErrorCreatingFile(path.as_ref().to_string_lossy().to_string(), e.to_string()).into()),
    }
    Ok(())
}

pub fn convert_to_utf8<P: AsRef<Path>, Q: AsRef<Path>>(from: &P, to: &Q) -> Res<()> {
    let import = read_with_encoding_detection(from)?;
    write_with_bom(to, &import)?;
    Ok(())
}

pub trait IcyBoardSerializer: serde::de::DeserializeOwned + serde::ser::Serialize {
    const FILE_TYPE: &'static str;

    fn load<P: AsRef<Path>>(path: &P) -> Res<Self> {
        match fs::read_to_string(path) {
            Ok(txt) => match toml::from_str(&txt) {
                Ok(result) => Ok(result),
                Err(e) => {
                    log::error!("Loading {} toml file '{}': {}", Self::FILE_TYPE, path.as_ref().display(), e);
                    Err(IcyError::ErrorParsingConfig(path.as_ref().to_string_lossy().to_string(), e.to_string()).into())
                }
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(IcyError::FileNotFound(path.as_ref().to_string_lossy().to_string()).into())
                } else {
                    log::error!("Loading {} file '{}': {}", Self::FILE_TYPE, path.as_ref().display(), e);
                    Err(IcyError::ErrorLoadingFile(path.as_ref().to_string_lossy().to_string(), e.to_string()).into())
                }
            }
        }
    }

    fn save<P: AsRef<Path>>(&self, path: &P) -> Res<()> {
        match toml::to_string(self) {
            Ok(txt) => match fs::write(path, txt) {
                Ok(_) => Ok(()),
                Err(e) => {
                    log::error!("Error writing {} file '{}': {}", Self::FILE_TYPE, path.as_ref().display(), e);
                    Err(IcyError::ErrorGeneratingToml(path.as_ref().to_string_lossy().to_string(), e.to_string()).into())
                }
            },
            Err(e) => {
                log::error!("Error generating {} toml file '{}': {}", Self::FILE_TYPE, path.as_ref().display(), e);
                Err(IcyError::ErrorGeneratingToml(path.as_ref().to_string_lossy().to_string(), e.to_string()).into())
            }
        }
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
                Err(IcyError::ErrorLoadingFile(path.as_ref().to_string_lossy().to_string(), err.to_string()).into())
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
                Err(IcyError::ErrorLoadingFile(path.as_ref().to_string_lossy().to_string(), err.to_string()).into())
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
                Err(IcyError::ErrorLoadingFile(path.as_ref().to_string_lossy().to_string(), err.to_string()).into())
            }
        }
    }
}
