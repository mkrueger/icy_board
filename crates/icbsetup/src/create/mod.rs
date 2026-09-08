use std::{
    fs,
    path::{Path, PathBuf},
};

use bstr::BString;
use chrono::Utc;
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoardSerializer,
        accounting_cfg::AccountingConfig,
        bulletins::{Bullettin, BullettinList},
        commands::CommandList,
        conferences::{Conference, ConferenceBase},
        doors::DoorList,
        events::EventList,
        file_directory::{DirectoryList, FileDirectory},
        ftn::FtnConfig,
        group_list::GroupList,
        icb_config::IcbConfig,
        icb_text::DEFAULT_DISPLAY_TEXT,
        language::{Language, SupportedLanguages},
        lock::BoardLock,
        message_area::{AreaList, MessageArea},
        sec_levels::{SecurityLevel, SecurityLevelDefinitions},
        statistics::Statistics,
        surveys::{Survey, SurveyList},
        user_base::{Password, PasswordInfo, User, UserBase},
        xfer_protocols::SupportedProtocols,
    },
};
use icy_engine::{CharacterFormatOptions, FileFormat, FormatOptions, SaveOptions, ScreenPreperation};
use jamjam::{jam::JamMessageBase, util::echomail::EchomailAddress};

use crate::import::{OutputLogger, console_logger::ConsoleLogger};

pub struct IcyBoardCreator {
    destination: PathBuf,
    logger: ConsoleLogger,
}

impl IcyBoardCreator {
    pub fn new(destination: &Path) -> Self {
        Self {
            destination: destination.to_path_buf(),
            logger: ConsoleLogger::default(),
        }
    }

    pub fn create(&mut self) -> Res<()> {
        self.logger.start_action(format!("Creating IcyBoard at {}", self.destination.display()));
        fs::create_dir_all(&self.destination)?;
        let _lock = BoardLock::acquire(&self.destination)?;
        fs::create_dir_all(self.destination.join("main"))?;
        fs::create_dir_all(self.destination.join("art/help"))?;

        self.logger.start_action("Creating main configuration… at {}".to_string());

        let mut config = IcbConfig::new();
        config.board.allow_iemsi = false;
        config.login_server.telnet.port = 1337;
        config.login_server.ssh.port = 1338;
        config.qwk_settings.bbs_id = "QWKMAIL".to_string();

        self.logger.start_action("Creating required paths.".to_string());
        crate::genhelp::install_defaults(&self.destination, &config)?;

        fs::create_dir_all(self.destination.join(&config.paths.tmp_work_path))?;
        fs::create_dir_all(self.destination.join(&config.paths.security_file_path))?;
        fs::create_dir_all(self.destination.join(&config.paths.command_display_path))?;
        config.paths.security_file_path = PathBuf::from("art/secmsgs");
        fs::create_dir_all(self.destination.join(&config.paths.security_file_path))?;

        self.logger.start_action("Write ICBTEXT…".to_string());
        DEFAULT_DISPLAY_TEXT.save(&self.destination.join(&config.paths.icbtext))?;

        self.logger.start_action("Write trashcan files…".to_string());
        config.paths.trashcan_upload_files = PathBuf::from("main/tcan_uploads.txt");
        fs::write(
            self.destination.join(&config.paths.trashcan_upload_files),
            include_str!("../../data/tcan_uploads.txt"),
        )?;

        config.paths.trashcan_user = PathBuf::from("main/tcan_user.txt");
        fs::write(self.destination.join(&config.paths.trashcan_user), include_str!("../../data/tcan_users.txt"))?;

        config.paths.trashcan_email = PathBuf::from("main/tcan_email.txt");
        fs::write(self.destination.join(&config.paths.trashcan_email), include_str!("../../data/tcan_email.txt"))?;
        config.paths.trashcan_passwords = PathBuf::from("main/tcan_passwords.txt");
        fs::write(
            self.destination.join(&config.paths.trashcan_passwords),
            include_str!("../../data/tcan_passwords.txt"),
        )?;
        config.paths.vip_users = PathBuf::from("main/vip_users.txt");
        fs::write(self.destination.join(&config.paths.vip_users), include_str!("../../data/vip_users.txt"))?;

        self.logger.start_action("Write protocol data files…".to_string());
        config.paths.protocol_data_file = PathBuf::from("main/protocols.toml");
        generate_protocol_data(&self.destination.join(&config.paths.protocol_data_file))?;

        self.logger.start_action("Write security data files…".to_string());
        config.paths.pwrd_sec_level_file = PathBuf::from("main/security_levels.toml");
        generate_security_level_data(&self.destination.join(&config.paths.pwrd_sec_level_file))?;

        self.logger.start_action("Write default conferences".to_string());
        self.generate_default_conference(&self.destination.join(&config.paths.conferences))?;

        self.logger.start_action("Write default command file".to_string());
        config.paths.command_file = PathBuf::from("main/commands.toml");

        let cmd_list = CommandList::new();
        cmd_list.save(&self.destination.join(&config.paths.command_file))?;

        self.logger.start_action("Write default art files".to_string());

        let options = SaveOptions {
            format: FormatOptions::Character(CharacterFormatOptions {
                screen_prep: ScreenPreperation::ClearScreen,
                ..Default::default()
            }),
            ..Default::default()
        };
        config.paths.welcome = PathBuf::from("art/welcome");
        convert_to_pcb_opt(
            &self.destination.join(&config.paths.welcome),
            include_bytes!("../../data/new_bbs/welcome.icy"),
            &options,
        )?;

        config.paths.newuser = PathBuf::from("art/newuser");
        convert_to_pcb_opt(
            &self.destination.join(&config.paths.newuser),
            include_bytes!("../../data/new_bbs/newuser.icy"),
            &options,
        )?;
        config.paths.closed = PathBuf::from("art/closed");
        convert_to_pcb(&self.destination.join(&config.paths.closed), include_bytes!("../../data/new_bbs/closed.icy"))?;

        config.paths.expire_warning = PathBuf::from("art/exp_warning");
        convert_to_pcb_opt(
            &self.destination.join(&config.paths.expire_warning),
            include_bytes!("../../data/new_bbs/warning.icy"),
            &options,
        )?;

        config.paths.expired = PathBuf::from("art/expired");
        convert_to_pcb(&self.destination.join(&config.paths.expired), include_bytes!("../../data/new_bbs/expired.icy"))?;

        config.paths.no_ansi = PathBuf::from("art/noansi");
        fs::write(
            self.destination.join(&config.paths.no_ansi).with_extension("asc"),
            include_str!("../../data/new_bbs/noansi.asc"),
        )?;

        config.paths.conf_join_menu = PathBuf::from("art/cnfn");
        fs::write(
            self.destination.join(&config.paths.conf_join_menu).with_extension("ppe"),
            include_bytes!("../../../../ppe/cnfn.ppe"),
        )?;

        config.paths.chat_intro_file = PathBuf::from("art/group");
        convert_to_pcb(
            &self.destination.join(&config.paths.chat_intro_file),
            include_bytes!("../../data/new_bbs/group.icy"),
        )?;

        config.paths.chat_menu = PathBuf::from("art/chtm");
        convert_to_pcb(&self.destination.join(&config.paths.chat_menu), include_bytes!("../../data/new_bbs/chtm.icy"))?;

        self.logger.start_action("Write default statistics file".to_string());
        config.paths.statistics_file = PathBuf::from("main/statistics.toml");
        Statistics::default().save(&self.destination.join(&config.paths.statistics_file))?;

        self.logger.start_action("Write default language definition file".to_string());
        config.paths.language_file = PathBuf::from("main/languages.toml");
        let mut lang = SupportedLanguages::default();
        lang.push(Language {
            description: "English".to_string(),
            yes_char: 'Y',
            no_char: 'N',
            ..Default::default()
        });
        lang.save(&self.destination.join(&config.paths.language_file))?;

        self.logger.start_action("Write default groups file".to_string());
        config.paths.group_file = PathBuf::from("main/groups");
        let mut list = GroupList::default();
        list.add_group("sysop", "System Operators");
        list.add_group("users", "Common Users");
        list.add_group("new_users", "New users");
        list.save(&self.destination.join(&config.paths.group_file))?;

        self.logger.start_action("Write default fidonet config file".to_string());
        config.paths.ftn_file = PathBuf::from("main/ftn.toml");
        let ftn = FtnConfig::default();
        fs::create_dir_all(self.destination.join(&ftn.inbound))?;
        fs::create_dir_all(self.destination.join(&ftn.outbound))?;
        ftn.save(&self.destination.join(&config.paths.ftn_file))?;

        self.logger.start_action("Write default event file".to_string());
        config.event.event_file = PathBuf::from("main/events.toml");
        EventList::default().save(&self.destination.join(&config.event.event_file))?;

        self.logger.start_action("Create default user (SYSOP)".to_string());

        let initial_password = format!("{:08x}{:08x}", fastrand::u32(..), fastrand::u32(..));

        let mut user = User {
            name: "SYSOP".to_string(),
            password: PasswordInfo {
                password: Password::new_argon2(&initial_password),
                ..Default::default()
            },
            page_len: 23,
            security_level: 110,
            ..Default::default()
        };
        user.stats.first_date_on = chrono::Utc::now();
        let mut user_base = UserBase::default();
        user_base.new_user(user);
        user_base.save(&self.destination.join(&config.paths.user_file))?;
        self.logger.start_action(format!("Initial SYSOP password: {initial_password}"));

        // Accounting
        config.accounting.cfg_file = PathBuf::from("main/accounting.toml");
        config.accounting.peak_holiday_list_file = PathBuf::from("main/holidays.toml");
        config.accounting.tracking_file = PathBuf::from("main/tracking.txt");
        config.accounting.info_file = PathBuf::from("art/actinfo");
        config.accounting.warning_file = PathBuf::from("art/actwarn");
        config.accounting.logoff_file = PathBuf::from("art/actbye");

        AccountingConfig::default().save(&self.destination.join(&config.accounting.cfg_file))?;

        fs::write(self.destination.join(&config.accounting.peak_holiday_list_file), [])?;

        fs::write(self.destination.join(&config.accounting.tracking_file), [])?;

        fs::write(self.destination.join(&config.accounting.info_file).with_extension("pcb"), [])?;
        fs::write(self.destination.join(&config.accounting.warning_file).with_extension("pcb"), [])?;
        fs::write(self.destination.join(&config.accounting.logoff_file).with_extension("pcb"), [])?;

        config.save(&self.destination.join(icy_board_engine::DEFAULT_ICYBOARD_FILE))?;

        self.logger.start_action("IcyBoard created successfully.".to_string());
        self.logger.start_action(format!(
            "Start with icboard \"{}\"",
            self.destination.join(icy_board_engine::DEFAULT_ICYBOARD_FILE).display()
        ));
        Ok(())
    }

    fn generate_default_conference(&self, conf_path: &PathBuf) -> Res<()> {
        let mut conf = Conference {
            name: "Main Board".to_string(),
            is_public: true,
            auto_rejoin: true,
            use_main_commands: true,
            ..Default::default()
        };

        self.logger.start_action("Create conference directories".to_string());
        conf.attachment_location = PathBuf::from("conferences/main/attach");
        fs::create_dir_all(self.destination.join(&conf.attachment_location))?;
        conf.pub_upload_location = PathBuf::from("conferences/main/upload");
        fs::create_dir_all(self.destination.join(&conf.pub_upload_location))?;

        self.logger.start_action("Write user & sysop menus…".to_string());
        conf.users_menu = PathBuf::from("conferences/main/brdm");
        let options = SaveOptions {
            format: FormatOptions::Character(CharacterFormatOptions {
                screen_prep: ScreenPreperation::ClearScreen,
                ..Default::default()
            }),
            ..Default::default()
        };
        convert_to_pcb_opt(
            &self.destination.join(&conf.users_menu),
            include_bytes!("../../data/new_bbs/brdm.icy"),
            &options,
        )?;

        conf.sysop_menu = PathBuf::from("conferences/main/brds");
        convert_to_pcb_opt(
            &self.destination.join(&conf.sysop_menu),
            include_bytes!("../../data/new_bbs/brds.icy"),
            &options,
        )?;
        conf.news_file = PathBuf::from("conferences/main/news");
        convert_to_pcb(&self.destination.join(&conf.news_file), include_bytes!("../../data/new_bbs/news.icy"))?;

        // Bulletin Menu
        self.logger.start_action("Write bulletins…".to_string());
        conf.blt_menu = PathBuf::from("conferences/main/blt");
        convert_to_pcb(&self.destination.join(&conf.blt_menu), include_bytes!("../../data/new_bbs/blt.icy"))?;
        conf.blt_file = PathBuf::from("conferences/main/blt.toml");

        let mut list = BullettinList::default();
        let path = PathBuf::from("conferences/main/rules");
        list.bullettins.push(Bullettin::new(&path));
        convert_to_pcb(&self.destination.join(&path), include_bytes!("../../data/new_bbs/rules.icy"))?;

        let path = PathBuf::from("conferences/main/history");
        list.bullettins.push(Bullettin::new(&path));
        convert_to_pcb(&self.destination.join(&path), include_bytes!("../../data/new_bbs/history.icy"))?;

        list.save(&self.destination.join(&conf.blt_file))?;

        // Surveys
        self.logger.start_action("Write surveys".to_string());
        conf.survey_menu = PathBuf::from("conferences/main/survey");
        convert_to_pcb(&self.destination.join(&conf.survey_menu), include_bytes!("../../data/new_bbs/survey.icy"))?;
        conf.survey_file = PathBuf::from("conferences/main/survey.toml");

        let mut list = SurveyList::default();
        let s = Survey {
            survey_file: PathBuf::from("conferences/main/script1.pcb"),
            answer_file: PathBuf::from("conferences/main/script1.answer"),
            ..Default::default()
        };
        convert_to_pcb_opt(
            &self.destination.join(&s.survey_file),
            include_bytes!("../../data/new_bbs/script1.icy"),
            &options,
        )?;
        list.push(s);

        let s = Survey {
            survey_file: PathBuf::from("conferences/main/script2.ppe"),
            answer_file: PathBuf::from("conferences/main/script2.answer"),
            ..Default::default()
        };
        fs::write(self.destination.join(&s.survey_file), include_bytes!("../../../../ppe/script2.ppe"))?;
        list.push(s);
        list.save(&self.destination.join(&conf.survey_file))?;

        // Create Directories
        self.logger.start_action("Create file directories…".to_string());
        conf.dir_menu = PathBuf::from("conferences/main/dir");
        fs::write(
            self.destination.join(&conf.dir_menu).with_extension("ppe"),
            include_bytes!("../../../../ppe/dir.ppe"),
        )?;
        conf.dir_file = PathBuf::from("conferences/main/dir.toml");
        let mut list = DirectoryList::default();
        let fd = FileDirectory {
            name: "General".to_string(),
            path: PathBuf::from("conferences/main/general/files/dir00"),
            ..Default::default()
        };
        fs::create_dir_all(self.destination.join(&fd.path))?;
        list.push(fd);
        list.save(&self.destination.join(&conf.dir_file))?;

        // Create message base
        self.logger.start_action("Create message areas…".to_string());
        conf.area_menu = PathBuf::from("conferences/main/area");
        fs::write(
            self.destination.join(&conf.area_menu).with_extension("ppe"),
            include_bytes!("../../../../ppe/area.ppe"),
        )?;
        conf.area_file = PathBuf::from("conferences/main/area.toml");
        let mut list = AreaList::default();
        let fd = MessageArea {
            name: "General".to_string(),
            path: PathBuf::from("conferences/main/messages/general"),
            ..Default::default()
        };
        fs::create_dir_all(self.destination.join("conferences/main/messages"))?;
        let mut msg_base = JamMessageBase::create(self.destination.join(&fd.path))?;
        msg_base.write_message(&write_welcome_msg())?;
        msg_base.write_jhr_header()?;

        list.push(fd);

        list.save(&self.destination.join(&conf.area_file))?;

        // Create Door files
        self.logger.start_action("Create door file…".to_string());
        conf.doors_menu = PathBuf::from("conferences/main/door");
        fs::write(
            self.destination.join(&conf.doors_menu).with_extension("ppe"),
            include_bytes!("../../../../ppe/door.ppe"),
        )?;
        conf.doors_file = PathBuf::from("conferences/main/door.toml");
        let list = DoorList::default();
        list.save(&self.destination.join(&conf.doors_file))?;

        self.logger.start_action("Write conference…".to_string());
        let mut base = ConferenceBase::default();
        base.push(conf);
        base.save(conf_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_help::{RenderOptions, catalog, render, sha256};

    fn assert_default_help(destination: &Path) {
        let creator = IcyBoardCreator::new(destination);
        let config = IcbConfig::new();
        fs::create_dir_all(&creator.destination).unwrap();
        let _lock = BoardLock::acquire(&creator.destination).unwrap();

        // Exercise creation's help installer without creating or printing SYSOP credentials.
        crate::genhelp::install_defaults(&creator.destination, &config).unwrap();

        let output = creator.destination.join(&config.paths.help_path);
        let resolved_output = output.canonicalize().unwrap();
        let ledger_path = creator.destination.join("main/help-generation.toml");
        let ledger_bytes = fs::read(&ledger_path).unwrap();
        let ledger: toml::Value = toml::from_str(std::str::from_utf8(&ledger_bytes).unwrap()).unwrap();
        assert_eq!(ledger["schema_version"].as_integer(), Some(1));
        let entries = ledger["entries"].as_array().unwrap();
        let sources = catalog::sources(None).unwrap();
        assert_eq!(sources.len(), 68);
        assert_eq!(entries.len(), sources.len());
        assert_eq!(fs::read_dir(&output).unwrap().count(), sources.len());
        for number in 1..=16 {
            assert!(output.join(format!("hlp{number}.pcb")).is_file());
        }

        let options = RenderOptions {
            clear_screen: true,
            ..Default::default()
        };
        assert_eq!(options.encoding, icy_board_help::Encoding::Utf8);
        assert!(
            fs::read(output.join("hlpa.pcb")).unwrap().starts_with(&[0xEF, 0xBB, 0xBF]),
            "new boards get UTF-8 help"
        );
        for source in &sources {
            let name = format!("{}.pcb", source.topic);
            let expected = render(&source.markdown, &options).unwrap().bytes;
            assert_eq!(fs::read(output.join(&name)).unwrap(), expected, "{name}");
            let matching: Vec<_> = entries.iter().filter(|entry| entry["name"].as_str() == Some(name.as_str())).collect();
            assert_eq!(matching.len(), 1, "{name}");
            let entry = matching[0];
            assert_eq!(entry["output"].as_str(), resolved_output.to_str(), "{name}");
            assert_eq!(entry["hash"].as_str(), Some(sha256(&expected).as_str()), "{name}");
            assert_eq!(entry["source_hash"].as_str(), Some(source.source_hash.as_str()), "{name}");
            let settings_hash = entry["settings_hash"].as_str().unwrap();
            assert_eq!(settings_hash.len(), 64);
            assert!(settings_hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
            assert_eq!(entry["settings_hash"], entries[0]["settings_hash"]);
        }

        let backups = creator.destination.join("main/help-generation.backups");
        let transactions = fs::read_dir(&backups).unwrap().count();
        assert_eq!(transactions, 1);
        crate::genhelp::install_defaults(&creator.destination, &config).unwrap();
        assert_eq!(fs::read(&ledger_path).unwrap(), ledger_bytes);
        assert_eq!(fs::read_dir(&backups).unwrap().count(), transactions);
        assert!(!creator.destination.join("main/help-generation.pending.toml").exists());
        assert!(config.paths.language_file.as_os_str().is_empty());
        assert!(!creator.destination.join("main/languages.toml").exists());
        assert!(!creator.destination.join(&config.paths.user_file).exists());
    }

    #[test]
    fn creation_help_matches_catalog_and_ledger() {
        let directory = tempfile::tempdir().unwrap();
        assert_default_help(&directory.path().join("new-board"));
    }

    #[test]
    fn creation_help_supports_relative_destination() {
        let cwd = std::env::current_dir().unwrap();
        let directory = tempfile::Builder::new().prefix(".creation-help-test-").tempdir_in(&cwd).unwrap();
        let destination = directory.path().strip_prefix(&cwd).unwrap().join("new-board");
        assert!(destination.is_relative());
        assert_default_help(&destination);
    }

    #[test]
    fn creation_help_ignores_malformed_language_file() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = IcbConfig::new();
        config.paths.language_file = "languages.toml".into();
        let language_file = directory.path().join(&config.paths.language_file);
        let malformed = b"[[languages\nnot valid TOML";
        fs::write(&language_file, malformed).unwrap();
        let _lock = BoardLock::acquire(directory.path()).unwrap();
        for _ in 0..2 {
            crate::genhelp::install_defaults(directory.path(), &config).unwrap();
            let output = directory.path().join(&config.paths.help_path);
            assert_eq!(fs::read_dir(&output).unwrap().count(), 68);
            for source in catalog::sources(None).unwrap() {
                assert!(output.join(format!("{}.pcb", source.topic)).is_file());
                assert!(!output.join(format!("{}.ger.pcb", source.topic)).exists());
            }
            assert_eq!(fs::read(&language_file).unwrap(), malformed);
        }
    }
}

fn write_welcome_msg() -> jamjam::jam::JamMessage {
    jamjam::jam::JamMessage::new(&EchomailAddress::default())
        .with_date_time(Utc::now())
        .with_from(BString::from("Mike Krueger"))
        .with_to(BString::from("SYSOP"))
        .with_subject(BString::from("Welcome to IcyBoard"))
        .with_text(BString::from(
            r#"Thank you for trying IcyBoard! I think you will like it.

It was made out of passion and love for PCBoard. A BBS system that was part
of my youth. It's a tribute to the good old days of BBS systems. 

It's not just a clone of PCBoard. It's a modern BBS system with a lot of 
new features.

IcyBoard is an ongoing project I'll continue to improve it.
I would like to get some feedback about this project. 

Visit the project site at:
https://github.com/mkrueger/icy_board

And also check out my other ansi/bbs releated tools:
https://github.com/mkrueger/icy_tools

   Mike Krueger"#,
        ))
}

fn generate_security_level_data(security_file_path: &PathBuf) -> Res<()> {
    let mut sec_level = SecurityLevelDefinitions::default();

    sec_level.levels.push(SecurityLevel {
        security: 10,
        time_per_day: 60,
        allow_alias: true,
        uldl_ratio_tenths: 10,
        uldl_kb_ratio_tenths: 10,
        ..Default::default()
    });

    sec_level.levels.push(SecurityLevel {
        security: 20,
        time_per_day: 90,
        allow_alias: true,
        uldl_ratio_tenths: 90,
        uldl_kb_ratio_tenths: 90,
        ..Default::default()
    });

    sec_level.levels.push(SecurityLevel {
        security: 100,
        time_per_day: 540,
        allow_alias: true,
        uldl_ratio_tenths: 150,
        uldl_kb_ratio_tenths: 150,
        ..Default::default()
    });

    sec_level.levels.push(SecurityLevel {
        security: 110,
        time_per_day: 999,
        allow_alias: true,
        uldl_ratio_tenths: 250,
        uldl_kb_ratio_tenths: 250,
        daily_file_kb_limit: 32767,
        ..Default::default()
    });

    sec_level.save(security_file_path)?;
    Ok(())
}

fn generate_protocol_data(protocol_data_file: &PathBuf) -> Res<()> {
    SupportedProtocols::generate_pcboard_defaults().save(protocol_data_file)?;
    Ok(())
}

pub fn convert_to_pcb(path: &Path, data: &[u8]) -> Res<()> {
    let options = SaveOptions::default();
    convert_to_pcb_opt(path, data, &options)
}

pub fn convert_to_pcb_opt(path: &Path, data: &[u8], opt: &SaveOptions) -> Res<()> {
    let loaded = FileFormat::IcyDraw.from_bytes(data, None).unwrap();
    let bytes: Vec<u8> = FileFormat::PCBoard.to_bytes(&loaded.screen.buffer, opt).unwrap();
    fs::write(path.with_extension("pcb"), &bytes)?;
    Ok(())
}
