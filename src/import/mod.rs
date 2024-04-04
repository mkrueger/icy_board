use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use icy_board_engine::icy_board::{
    commands::CommandList,
    conferences::ConferenceBase,
    convert_to_utf8,
    icb_config::{
        ColorConfiguration, ConfigPaths, IcbColor, IcbConfig, PasswordStorageMethod,
        SubscriptionMode, SysopInformation, SysopSecurityLevels, UserPasswordPolicy,
    },
    icb_text::IcbTextFile,
    language::SupportedLanguages,
    menu::Menu,
    pcbconferences::{PcbAdditionalConferenceHeader, PcbConferenceHeader},
    pcboard_data::PcbBoardData,
    read_cp437,
    user_base::{Password, UserBase},
    user_inf::PcbUserInf,
    users::PcbUserRecord,
    xfer_protocols::SupportedProtocols,
    IcyBoardError, PCBoardImport, PcbUser,
};
use icy_board_engine::icy_board::{
    state::functions::PPECall, statistics::Statistics, write_with_bom, IcyBoardSerializer,
    PCBoardRecordImporter,
};
use icy_ppe::{datetime::IcbTime, Res};
use qfile::{QFilePath, QTraitSync};
use relative_path::{PathExt, RelativePathBuf};
use walkdir::WalkDir;

use self::{default_commands::add_default_commands, import_log::ImportLog};

pub mod console_logger;
pub mod default_commands;
pub mod import_log;

pub trait OutputLogger {
    fn start_action(&self, message: String);
    fn check_error(&self, res: Option<std::io::Error>) -> Res<()>;
    fn warning(&self, message: String);
}

pub struct PCBoardImporter {
    pub output: Box<dyn OutputLogger>,
    pub data: PcbBoardData,
    pub source_directory: PathBuf,
    pub output_directory: PathBuf,
    pub logger: ImportLog,

    /// Contains paths to map dos paths to unix paths
    /// For example:
    /// 'C:\' -> '/home/user/pcboard'
    /// Difference to map_paths is that this maps source paths to other source paths.
    pub resolve_paths: HashMap<String, String>,

    /// Map original paths to import paths
    /// For example:
    /// '/home/user/pcboard/HELP' -> '/home/user/icyboard/icb/help'
    /// Difference to resolve_paths is that this maps destination paths.
    pub map_paths: HashMap<String, PathBuf>,

    pub converted_files: HashMap<String, String>,
}

impl PCBoardImporter {
    pub fn new(
        file_name: &str,
        output: Box<dyn OutputLogger>,
        output_directory: PathBuf,
    ) -> Res<Self> {
        let data = PcbBoardData::import_pcboard(file_name)?;
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
        output.start_action(format!(
            "Importing PCBoard from base path {}\n",
            source_directory.display()
        ));

        path.push(help_loc.file_name().unwrap());
        if !path.exists() {
            return Err(Box::new(IcyBoardError::Error(
                "Can't resolve C: file".to_string(),
            )));
        }

        //let len = to_str().unwrap().len();
        let k = help_loc.parent().unwrap().to_str().unwrap().to_string();
        let v = file_path
            .parent()
            .unwrap()
            .to_path_buf()
            .to_str()
            .unwrap()
            .to_string();
        paths.insert(k, v);

        let mut map_paths = HashMap::new();
        map_paths.insert(upper.clone() + "\\PPE", output_directory.join("ppe"));
        map_paths.insert(upper.clone(), output_directory.clone());

        Ok(Self {
            output,
            data,
            output_directory,
            source_directory,
            resolve_paths: paths,
            map_paths,
            logger: ImportLog::default(),
            converted_files: HashMap::new(),
        })
    }

    pub fn resolve_file(&self, file: &str) -> Res<PathBuf> {
        let mut s: String = file
            .chars()
            .map(|x| match x {
                '\\' => '/',
                _ => x,
            })
            .collect();

        for (k, v) in &self.resolve_paths {
            if s.starts_with(k) {
                s = v.clone() + &s[k.len()..];
            }
        }

        if let Ok(mut file_path) = QFilePath::add_path(s.clone()) {
            if let Ok(file) = file_path.get_path_buf() {
                return Ok(file);
            }
        }
        let res = PathBuf::from(s);
        Ok(res)
    }

    pub fn start_import(&mut self) -> Res<()> {
        self.create_directories()?;

        self.convert_hlp_files(&self.data.path.help_loc.clone(), "help")?;

        let icbtext = self.convert_pcbtext(
            &(self.data.path.text_loc.clone() + "/PCBTEXT"),
            "data/icbtext.toml",
        )?;
        let trashcan =
            self.convert_trashcan(&self.data.path.tcan_file.clone(), "config/badusers.txt")?;

        let welcome =
            self.convert_display_file(&self.data.path.welcome_file.clone(), "art/welcome")?;
        let newuser =
            self.convert_display_file(&self.data.path.newuser_file.clone(), "art/newuser")?;
        let closed =
            self.convert_display_file(&self.data.path.closed_file.clone(), "art/closed")?;
        let warning =
            self.convert_display_file(&self.data.path.warning_file.clone(), "art/warning")?;
        let expired =
            self.convert_display_file(&self.data.path.expired_file.clone(), "art/expired")?;
        let conf_join_menu =
            self.convert_display_file(&self.data.path.conf_menu.clone(), "art/cnfn")?;
        let group_chat =
            self.convert_display_file(&self.data.path.group_chat.clone(), "art/group")?;
        let chat_menu = self.convert_display_file(&self.data.path.chat_menu.clone(), "art/chtm")?;
        let no_ansi = self.convert_display_file(&self.data.path.no_ansi.clone(), "art/noansi")?;

        let user_base = self.convert_user_base(
            &self.data.path.usr_file.clone(),
            &self.data.path.inf_file.clone(),
            "data/user_base.toml",
        )?;

        let protocol_data_file = self.convert_data::<SupportedProtocols>(
            &self.data.path.protocol_data_file.clone(),
            "config/protocols.toml",
        )?;
        let language_file = self.convert_data::<SupportedLanguages>(
            &self.data.path.pcml_dat_file.clone(),
            "config/languages.toml",
        )?;
        let security_level_file = self.convert_data::<SupportedLanguages>(
            &self.data.path.pwd_file.clone(),
            "config/security_levels.toml",
        )?;
        let command_file =
            self.convert_default_cmd_lst(&self.data.path.cmd_lst.clone(), "config/commands.toml")?;
        let statistics_file = self.convert_data::<Statistics>(
            &self.data.path.stats_file.clone(),
            "data/statistics.toml",
        )?;

        let conferences = self.convert_conferences(
            &self.data.path.conference_file.clone(),
            "config/conferences.toml",
        )?;

        let icb_cfg = IcbConfig {
            sysop: SysopInformation {
                name: self.data.sysop_info.sysop.clone(),
                password: Password::from_str(self.data.sysop_info.password.as_str()).unwrap(),
                require_password_to_exit: self.data.sysop_info.require_pwrd_to_exit,
                use_real_name: self.data.sysop_info.use_real_name,
                sysop_start: IcbTime::parse(&self.data.sysop_start),
                sysop_stop: IcbTime::parse(&self.data.sysop_stop),
            },
            sysop_security_level: SysopSecurityLevels {
                sysop: self.data.sysop_security.sysop as u8,
                read_all_comments: self.data.sysop_security.read_all_comments as u8,
                read_all_mail: self.data.sysop_security.read_all_mail as u8,
                copy_move_messages: self.data.sysop_security.copy_move_messages as u8,
                use_broadcast_command: self.data.sysop_security.use_broadcast_command as u8,
                view_private_uploads: self.data.sysop_security.view_private_uploads as u8,
                edit_message_headers: self.data.sysop_security.edit_message_headers as u8,
                protect_unprotect_messages: self.data.sysop_security.protect_unprotect_messages
                    as u8,
            },
            color_configuration: ColorConfiguration {
                default: IcbColor::Dos(self.data.colors.default as u8),
                msg_hdr_date: IcbColor::Dos(self.data.colors.msg_hdr_date as u8),
                msg_hdr_to: IcbColor::Dos(self.data.colors.msg_hdr_to as u8),
                msg_hdr_from: IcbColor::Dos(self.data.colors.msg_hdr_from as u8),
                msg_hdr_subj: IcbColor::Dos(self.data.colors.msg_hdr_subj as u8),
                msg_hdr_read: IcbColor::Dos(self.data.colors.msg_hdr_read as u8),
                msg_hdr_conf: IcbColor::Dos(self.data.colors.msg_hdr_conf as u8),
            },
            board_name: self.data.board_name.clone(),
            func_keys: self.data.func_keys.clone(),
            subscription_info: SubscriptionMode {
                is_enabled: self.data.subscription_info.is_enabled,
                subscription_length: self.data.subscription_info.subscription_length,
                default_expired_level: self.data.subscription_info.default_expired_level,
                warning_days: self.data.subscription_info.warning_days,
            },
            user_password_policy: UserPasswordPolicy {
                min_length: self.data.min_pwrd_len as u8,
                password_expire_days: self.data.pwrd_update as u16,
                password_expire_warn_days: self.data.pwrd_warn as u16,
                password_storage_method: PasswordStorageMethod::PlainText,
            },
            paths: ConfigPaths {
                help_path: PathBuf::from("./help/"),
                tmp_path: PathBuf::from("./tmp/"),
                icbtext,
                user_base,
                conferences,
                welcome,
                newuser,
                closed,
                warning,
                expired,
                conf_join_menu,
                group_chat,
                chat_menu,
                no_ansi,
                trashcan,
                protocol_data_file,
                security_level_file,
                language_file,
                command_file,
                statistics_file,
            },
        };

        let destination = self.output_directory.join("icyboard.toml");
        self.output.start_action(format!(
            "Create main configutation {}…",
            destination.display()
        ));
        self.logger
            .log_boxed_error(icb_cfg.save(&destination).err())?;
        self.output.start_action("done.".into());
        self.logger.log("done.");

        let destination = self.output_directory.join("importlog.txt");
        fs::write(destination, &self.logger.output)?;

        Ok(())
    }

    fn convert_conferences(&mut self, conference_file: &str, new_rel_name: &str) -> Res<PathBuf> {
        self.output.start_action("Convert conferences…".into());

        let conf = self.resolve_file(conference_file)?;
        let conf_add = self.resolve_file(&(conference_file.to_string() + ".ADD"))?;
        let conferences = PcbConferenceHeader::import_pcboard(&conf, self.data.num_conf as usize)?;
        let add_conferences = PcbAdditionalConferenceHeader::import_pcboard(&conf_add)?;

        let mut conf_base = ConferenceBase::import_pcboard(&conferences, &add_conferences);

        for (i, conf) in conf_base.iter_mut().enumerate() {
            let output = if i == 0 {
                "conferences/main".to_string()
            } else {
                format!("conferences/{i}")
            };
            let destination = self.output_directory.join(&output);

            let _ = fs::create_dir(destination);
            conf.users_menu = self.convert_conference_display_file(&output, &conf.users_menu)?;
            conf.sysop_menu = self.convert_conference_display_file(&output, &conf.sysop_menu)?;
            conf.news_file = self.convert_conference_display_file(&output, &conf.news_file)?;
            conf.pub_upload_dir_file = PathBuf::from(output.to_string() + "/upload");
            conf.pub_upload_location = PathBuf::from(output.to_string() + "/up");
            conf.private_upload_dir_file = PathBuf::from(output.to_string() + "/private");
            conf.private_upload_location = PathBuf::from(output.to_string() + "/pr");
            conf.doors_menu = self.convert_conference_display_file(&output, &conf.doors_menu)?;
            conf.doors_file = self.convert_conference_display_file(&output, &conf.doors_file)?;
            conf.blt_menu = self.convert_conference_display_file(&output, &conf.blt_menu)?;
            conf.blt_file = self.convert_conference_display_file(&output, &conf.blt_file)?;
            conf.script_menu = self.convert_conference_display_file(&output, &conf.script_menu)?;
            conf.script_file = self.convert_conference_display_file(&output, &conf.script_file)?;
        }

        let destination = self.output_directory.join(new_rel_name);
        conf_base.save(&destination)?;
        self.logger
            .create_new_file(destination.display().to_string());

        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_conference_display_file(&mut self, output: &str, file_name: &Path) -> Res<PathBuf> {
        let resolved_file = self.resolve_file(file_name.file_name().unwrap().to_str().unwrap())?;

        let name = resolved_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
            .to_ascii_lowercase();
        let new_name = output.to_string() + "/" + &name;
        self.convert_display_file(file_name.file_name().unwrap().to_str().unwrap(), &new_name)
    }

    pub fn create_directories(&mut self) -> Res<()> {
        self.output.start_action(format!(
            "Creating directory '{}'…",
            self.output_directory.display()
        ));
        self.logger
            .log_error(fs::create_dir(&self.output_directory).err())?;
        self.logger.created_directory(self.output_directory.clone());

        const REQUIRED_DIRECTORIES: [&str; 9] = [
            "gen",
            "conferences",
            "conferences/main",
            "ppe",
            "data",
            "config",
            "config/menus",
            "help",
            "art",
        ];

        for dir in REQUIRED_DIRECTORIES.iter() {
            let o = self.output_directory.join(dir);
            self.output
                .start_action(format!("Creating directory '{}'…", o.display()));
            self.output.check_error(fs::create_dir(&o).err())?;
            self.logger.created_directory(o);
        }
        self.logger.log("");

        Ok(())
    }

    fn convert_pcbtext(&mut self, pcb_text_file: &str, new_rel_name: &str) -> Res<PathBuf> {
        let destination = self.output_directory.join(new_rel_name);
        self.output
            .start_action(format!("Create ICBTEXT {}…", destination.display()));

        let resolved_file = self.resolve_file(pcb_text_file)?;

        let mut text_file = IcbTextFile::load(&resolved_file)?;
        for (i, entry) in text_file.iter_mut().enumerate() {
            entry.text =
                self.scan_line_for_commands(&entry.text, &format!("PCBTEXT, entry {}", i))?;
        }
        self.logger
            .log_boxed_error(text_file.save(&destination).err())?;
        self.logger
            .converted_file(&resolved_file, &destination, true);
        self.logger.log("");

        Ok(PathBuf::from(new_rel_name))
    }

    pub fn scan_line_for_commands(&mut self, line: &str, context: &str) -> Res<String> {
        if let Some(call) = PPECall::try_parse_line(line) {
            if let Ok(resolved_file) = self.resolve_file(&call.file) {
                if !resolved_file.exists() {
                    self.output
                        .warning(format!("Can't find file {}", resolved_file.display()));
                    self.logger
                        .log(&format!("Can't find file {}", resolved_file.display()));
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
                return Ok(new_line);
            } else {
                self.logger.log_cant_convert_file(&call.file, context);
            }
        }
        Ok(line.to_string())
    }

    fn convert_file(&mut self, resolved_file: PathBuf) -> Res<String> {
        let upper_file_name = resolved_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(file.clone());
        }

        if let Some(ext) = resolved_file.extension() {
            match ext
                .to_ascii_uppercase()
                .to_string_lossy()
                .to_string()
                .as_str()
            {
                "PPE" => {
                    self.converted_files.insert(
                        upper_file_name.clone(),
                        resolved_file.to_str().unwrap().to_string(),
                    );
                    return Ok(resolved_file.to_str().unwrap().to_string());
                }
                "MNU" => {
                    let imported_menu = Menu::import_pcboard(&resolved_file)?;
                    let menu_path = self
                        .output_directory
                        .join("config/menus")
                        .join(resolved_file.file_name().unwrap().to_ascii_lowercase());
                    imported_menu.save(&menu_path)?;
                    self.converted_files.insert(
                        upper_file_name.clone(),
                        menu_path.to_str().unwrap().to_string(),
                    );
                    let out_path = menu_path.file_name().unwrap().to_str().unwrap().to_string();
                    self.logger.translated_file(&resolved_file, &menu_path);
                    return Ok(out_path);
                }
                _ => {}
            }
        }
        let output_path = self
            .output_directory
            .join("gen")
            .join(resolved_file.file_name().unwrap().to_ascii_lowercase());
        convert_to_utf8(&resolved_file, &output_path)?;
        self.converted_files.insert(
            upper_file_name.clone(),
            output_path.to_str().unwrap().to_string(),
        );
        let out_path = output_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        self.logger
            .converted_file(&resolved_file, &output_path, true);
        Ok(out_path)
    }

    pub fn import_and_scan_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        from: &P,
        to: &Q,
    ) -> Res<()> {
        let in_string = read_cp437(from)?;
        self.output.start_action(format!(
            "\t convert '{}' to utf8 '{}'…",
            from.as_ref().display(),
            to.as_ref().display()
        ));
        let mut import = String::new();

        let ctx = format!("importing file {}", from.as_ref().display());
        for line in in_string.lines() {
            import.push_str(&self.scan_line_for_commands(line, &ctx)?);
            import.push('\n');
        }

        write_with_bom(to, &import)?;
        self.logger.converted_file(from.as_ref(), to.as_ref(), true);
        Ok(())
    }

    fn convert_trashcan(&mut self, trashcan_file: &str, new_rel_name: &str) -> Res<PathBuf> {
        if trashcan_file.is_empty() {
            return Ok(PathBuf::new());
        }

        let resolved_file = self.resolve_file(trashcan_file)?;
        let resolved_file = PathBuf::from(&resolved_file);
        let trashcan_header = include_str!("../../data/badusers_empty.txt");

        let dest = self.output_directory.join(new_rel_name);
        self.output.start_action(format!(
            "Convert trashcan -> badusers.txt {}…",
            dest.display()
        ));

        if !resolved_file.exists() {
            fs::write(new_rel_name, trashcan_header)?;
            self.logger.create_new_file(dest.clone().to_string_lossy());
            return Ok(dest);
        }
        let mut trashcan = regex::escape(&read_cp437(&resolved_file)?);
        if !trashcan.ends_with('\n') {
            trashcan.push('\n');
        }

        if let Err(err) = fs::write(
            dest.clone(),
            trashcan_header.to_string() + trashcan.as_str(),
        ) {
            return Err(Box::new(IcyBoardError::ErrorCreatingFile(
                new_rel_name.to_string(),
                err.to_string(),
            )));
        }
        self.logger.translated_file(&resolved_file, &dest);
        self.logger.log("");
        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_display_file(&mut self, file: &str, new_name: &str) -> Res<PathBuf> {
        if file.is_empty() {
            return Ok(PathBuf::new());
        }
        let Ok(resolved_file) = self.resolve_file(file) else {
            self.output.warning(format!("File not found {}", file));
            self.logger.log(&format!("File not found {}", file));
            return Ok(PathBuf::new());
        };

        let upper_file_name = resolved_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_ascii_uppercase();
        if let Some(file) = self.converted_files.get(&upper_file_name) {
            return Ok(PathBuf::from(file));
        }

        let from_file = PathBuf::from(&resolved_file);
        let mut dest_path = self.output_directory.join(new_name);
        dest_path.pop();

        if let Some(parent) = from_file.parent() {
            let upper_name = from_file
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_ascii_uppercase();
            for entry in fs::read_dir(parent)?.flatten() {
                if !entry.path().is_file() {
                    continue;
                }
                let found_name = entry.file_name().to_str().unwrap().to_ascii_uppercase();
                if found_name.starts_with(&upper_name) {
                    let mut dest = dest_path.to_path_buf();
                    dest.push(entry.file_name().to_ascii_lowercase());
                    if !found_name.ends_with(".PPS")
                        && (/*found_name.ends_with(".PPE") ||*/found_name.contains('.'))
                    {
                        if found_name.ends_with(".MNU") {
                            let imported_menu = Menu::import_pcboard(&entry.path())?;
                            imported_menu.save(&dest)?;
                            self.converted_files.insert(
                                entry.path().display().to_string().to_ascii_uppercase(),
                                dest.display().to_string(),
                            );
                            self.logger.translated_file(&entry.path(), &dest);
                        } else {
                            self.output.start_action(format!(
                                "\t copy '{}' to '{}'…",
                                entry.path().display(),
                                dest.display()
                            ));
                            self.output
                                .check_error(fs::copy(entry.path(), &dest).err())?;
                            self.logger.copy_file(&entry.path(), &dest);
                        }
                    } else {
                        self.import_and_scan_file(&entry.path(), &dest)?;
                    }
                }
            }
        }

        self.converted_files
            .insert(upper_file_name.clone(), new_name.to_string());
        Ok(PathBuf::from(new_name))
    }

    fn convert_user_base(
        &mut self,
        usr_file: &str,
        inf_file: &str,
        new_rel_name: &str,
    ) -> Res<PathBuf> {
        self.output.start_action("Convert user base…".into());
        let usr_file = self.resolve_file(usr_file)?;
        let inf_file = self.resolve_file(inf_file)?;

        let users = PcbUserRecord::read_users(&PathBuf::from(&usr_file))?;
        let user_inf = PcbUserInf::read_users(&PathBuf::from(&inf_file))?;

        let user_base = UserBase::import_pcboard(
            &users
                .iter()
                .map(|u| PcbUser {
                    user: u.clone(),
                    inf: user_inf[u.rec_num - 1].clone(),
                })
                .collect::<Vec<PcbUser>>(),
        );

        let destination = self.output_directory.join(new_rel_name);

        user_base.save(&destination)?;
        self.logger
            .create_new_file(destination.display().to_string());

        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_hlp_files(&mut self, help_loc: &str, rel_output: &str) -> Res<()> {
        let help_loc = self.resolve_file(help_loc)?;
        let help_loc = PathBuf::from(&help_loc);
        self.logger.log("=== Converting help files ===");

        let o = self.output_directory.join(rel_output);
        if help_loc.exists() {
            self.output.start_action(format!(
                "Copy help files from {} to {}…",
                help_loc.display(),
                o.display()
            ));
            for entry in WalkDir::new(&help_loc) {
                let entry = entry?;
                if entry.path().is_dir() {
                    continue;
                }
                let rel_path = entry.path().relative_to(&help_loc).unwrap();
                let lower_case =
                    RelativePathBuf::from_path(rel_path.as_str().to_lowercase()).unwrap();
                let to = lower_case.to_logical_path(&o);
                if let Some(parent_dir) = to.parent() {
                    if !parent_dir.exists() {
                        fs::create_dir(parent_dir).unwrap();
                    }
                }
                self.import_and_scan_file(&entry.path(), &to)?;
            }
        }
        self.logger.log("=== Done ===");

        Ok(())
    }

    fn convert_data<T: PCBoardImport>(&mut self, file: &str, new_rel_name: &str) -> Res<PathBuf> {
        if file.is_empty() {
            return Ok(PathBuf::new());
        }

        let resolved_file = self.resolve_file(file)?;
        self.output
            .start_action(format!("Convert {}…", resolved_file.display()));
        let resolved_file = PathBuf::from(&resolved_file);
        let res = if resolved_file.exists() {
            T::import_pcboard(&resolved_file)?
        } else {
            T::default()
        };
        let destination = self.output_directory.join(new_rel_name);
        self.logger.log_boxed_error(res.save(&destination).err())?;
        self.logger.log("");

        Ok(PathBuf::from(new_rel_name))
    }

    fn convert_default_cmd_lst(&mut self, file: &str, new_rel_name: &str) -> Res<PathBuf> {
        if file.is_empty() {
            return Ok(PathBuf::new());
        }
        let resolved_file = self.resolve_file(file)?;
        let resolved_file = PathBuf::from(&resolved_file);
        let mut res = if resolved_file.exists() {
            CommandList::import_pcboard(&resolved_file)?
        } else {
            CommandList::default()
        };

        add_default_commands(&self.data, &mut res);

        let destination = self.output_directory.join(new_rel_name);
        if let Err(err) = res.save(&destination) {
            return Err(Box::new(IcyBoardError::ErrorCreatingFile(
                new_rel_name.to_string(),
                err.to_string(),
            )));
        }
        Ok(PathBuf::from(new_rel_name))
    }
}
