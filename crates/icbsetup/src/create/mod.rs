use std::{fs, path::PathBuf};

use bstr::BString;
use chrono::Utc;
use icy_board_engine::{
    Res,
    icy_board::{
        IcyBoardSerializer,
        bulletins::{Bullettin, BullettinList},
        commands::CommandList,
        conferences::{Conference, ConferenceBase},
        doors::DoorList,
        file_directory::{DirectoryList, FileDirectory},
        group_list::GroupList,
        icb_config::IcbConfig,
        icb_text::DEFAULT_DISPLAY_TEXT,
        language::{Language, SupportedLanguages},
        message_area::{AreaList, MessageArea},
        sec_levels::{SecurityLevel, SecurityLevelDefinitions},
        statistics::Statistics,
        surveys::{Survey, SurveyList},
        user_base::{Password, PasswordInfo, User, UserBase},
        xfer_protocols::SupportedProtocols,
    },
};
use icy_engine::{Buffer, SaveOptions, ScreenPreperation};
use jamjam::{jam::JamMessageBase, util::echmoail::EchomailAddress};

use crate::import::{OutputLogger, console_logger::ConsoleLogger};

pub struct IcyBoardCreator {
    destination: PathBuf,
    logger: ConsoleLogger,
}

lazy_static::lazy_static! {
    static ref HELP_FILES: Vec<(&'static str, Vec<u8>)> = vec![
        ("hlpa", include_bytes!("../../data/new_bbs/help/hlpa.icy").to_vec()),
        ("hlpalias", include_bytes!("../../data/new_bbs/help/hlpalias.icy").to_vec()),
        ("hlpb", include_bytes!("../../data/new_bbs/help/hlpb.icy").to_vec()),
        ("hlpbrd", include_bytes!("../../data/new_bbs/help/hlpbrd.icy").to_vec()),
        ("hlpchat", include_bytes!("../../data/new_bbs/help/hlpchat.icy").to_vec()),
        ("hlpc", include_bytes!("../../data/new_bbs/help/hlpc.icy").to_vec()),
        ("hlpcmenu", include_bytes!("../../data/new_bbs/help/hlpcmenu.icy").to_vec()),
        ("hlpd", include_bytes!("../../data/new_bbs/help/hlpd.icy").to_vec()),
        ("hlpe", include_bytes!("../../data/new_bbs/help/hlpe.icy").to_vec()),
        ("hlpendr", include_bytes!("../../data/new_bbs/help/hlpendr.icy").to_vec()),
        ("hlpf", include_bytes!("../../data/new_bbs/help/hlpf.icy").to_vec()),
        ("hlpflag", include_bytes!("../../data/new_bbs/help/hlpflag.icy").to_vec()),
        ("hlpfscrn", include_bytes!("../../data/new_bbs/help/hlpfscrn.icy").to_vec()),
        ("hlpg", include_bytes!("../../data/new_bbs/help/hlpg.icy").to_vec()),
        ("hlph", include_bytes!("../../data/new_bbs/help/hlph.icy").to_vec()),
        ("hlp!", include_bytes!("../../data/new_bbs/help/hlp!.icy").to_vec()),
        ("hlpi", include_bytes!("../../data/new_bbs/help/hlpi.icy").to_vec()),
        ("hlpj", include_bytes!("../../data/new_bbs/help/hlpj.icy").to_vec()),
        ("hlpk", include_bytes!("../../data/new_bbs/help/hlpk.icy").to_vec()),
        ("hlplang", include_bytes!("../../data/new_bbs/help/hlplang.icy").to_vec()),
        ("hlpm", include_bytes!("../../data/new_bbs/help/hlpm.icy").to_vec()),
        ("hlpnews", include_bytes!("../../data/new_bbs/help/hlpnews.icy").to_vec()),
        ("hlpn", include_bytes!("../../data/new_bbs/help/hlpn.icy").to_vec()),
        ("hlpo", include_bytes!("../../data/new_bbs/help/hlpo.icy").to_vec()),
        ("hlpopen", include_bytes!("../../data/new_bbs/help/hlpopen.icy").to_vec()),
        ("hlpp", include_bytes!("../../data/new_bbs/help/hlpp.icy").to_vec()),
        ("hlpppe", include_bytes!("../../data/new_bbs/help/hlpppe.icy").to_vec()),
        ("hlpq", include_bytes!("../../data/new_bbs/help/hlpq.icy").to_vec()),
        ("hlpqwk", include_bytes!("../../data/new_bbs/help/hlpqwk.icy").to_vec()),
        ("hlpreg", include_bytes!("../../data/new_bbs/help/hlpreg.icy").to_vec()),
        ("hlprep", include_bytes!("../../data/new_bbs/help/hlprep.icy").to_vec()),
        ("hlpr", include_bytes!("../../data/new_bbs/help/hlpr.icy").to_vec()),
        ("hlprm", include_bytes!("../../data/new_bbs/help/hlprm.icy").to_vec()),
        ("hlpsec", include_bytes!("../../data/new_bbs/help/hlpsec.icy").to_vec()),
        ("hlps", include_bytes!("../../data/new_bbs/help/hlps.icy").to_vec()),
        ("hlpsrch", include_bytes!("../../data/new_bbs/help/hlpsrch.icy").to_vec()),
        ("hlptest", include_bytes!("../../data/new_bbs/help/hlptest.icy").to_vec()),
        ("hlpt", include_bytes!("../../data/new_bbs/help/hlpt.icy").to_vec()),
        ("hlpts", include_bytes!("../../data/new_bbs/help/hlpts.icy").to_vec()),
        ("hlpu", include_bytes!("../../data/new_bbs/help/hlpu.icy").to_vec()),
        ("hlpusers", include_bytes!("../../data/new_bbs/help/hlpusers.icy").to_vec()),
        ("hlpv", include_bytes!("../../data/new_bbs/help/hlpv.icy").to_vec()),
        ("hlpwho", include_bytes!("../../data/new_bbs/help/hlpwho.icy").to_vec()),
        ("hlpw", include_bytes!("../../data/new_bbs/help/hlpw.icy").to_vec()),
        ("hlpx", include_bytes!("../../data/new_bbs/help/hlpx.icy").to_vec()),
        ("hlpy", include_bytes!("../../data/new_bbs/help/hlpy.icy").to_vec()),
        ("hlpz", include_bytes!("../../data/new_bbs/help/hlpz.icy").to_vec()),
        ("hlp@", include_bytes!("../../data/new_bbs/help/hlp@.icy").to_vec()),
        ("hlp@w", include_bytes!("../../data/new_bbs/help/hlp@w.icy").to_vec()),
        ("hlparea", include_bytes!("../../data/new_bbs/help/hlparea.icy").to_vec()),
    ];
}

impl IcyBoardCreator {
    pub fn new(destination: &PathBuf) -> Self {
        Self {
            destination: destination.clone(),
            logger: ConsoleLogger::default(),
        }
    }

    pub fn create(&mut self) -> Res<()> {
        self.logger.start_action(format!("Creating IcyBoard at {}", self.destination.display()));
        fs::create_dir_all(&self.destination)?;
        fs::create_dir_all(&self.destination.join("main"))?;
        fs::create_dir_all(&self.destination.join("art/help"))?;

        self.logger.start_action("Creating main configuration… at {}".to_string());

        let mut config = IcbConfig::new();
        config.board.allow_iemsi = false;
        config.login_server.telnet.port = 1337;
        config.login_server.ssh.port = 1338;

        self.logger.start_action("Creating required paths.".to_string());
        fs::create_dir_all(&self.destination.join(&config.paths.help_path))?;

        let mut options = SaveOptions::default();
        options.screen_preparation = ScreenPreperation::ClearScreen;
        options.modern_terminal_output = true;

        for hlp in HELP_FILES.iter() {
            let path = self.destination.join(&config.paths.help_path).join(hlp.0);
            convert_to_pcb_opt(&path, &hlp.1, &options)?;
        }

        fs::create_dir_all(&self.destination.join(&config.paths.tmp_work_path))?;
        fs::create_dir_all(&self.destination.join(&config.paths.security_file_path))?;
        fs::create_dir_all(&self.destination.join(&config.paths.command_display_path))?;
        config.paths.security_file_path = PathBuf::from("art/secmsgs");
        fs::create_dir_all(&self.destination.join(&config.paths.security_file_path))?;

        self.logger.start_action("Write ICBTEXT…".to_string());
        DEFAULT_DISPLAY_TEXT.save(&self.destination.join(&config.paths.icbtext))?;

        self.logger.start_action("Write trashcan files…".to_string());
        config.paths.trashcan_upload_files = PathBuf::from("main/tcan_uploads.txt");
        fs::write(
            &self.destination.join(&config.paths.trashcan_upload_files),
            include_str!("../../data/tcan_uploads.txt"),
        )?;

        config.paths.trashcan_user = PathBuf::from("main/tcan_user.txt");
        fs::write(&self.destination.join(&config.paths.trashcan_user), include_str!("../../data/tcan_users.txt"))?;

        config.paths.trashcan_email = PathBuf::from("main/tcan_email.txt");
        fs::write(&self.destination.join(&config.paths.trashcan_email), include_str!("../../data/tcan_email.txt"))?;
        config.paths.trashcan_passwords = PathBuf::from("main/tcan_passwords.txt");
        fs::write(
            &self.destination.join(&config.paths.trashcan_passwords),
            include_str!("../../data/tcan_passwords.txt"),
        )?;
        config.paths.vip_users = PathBuf::from("main/vip_users.txt");
        fs::write(&self.destination.join(&config.paths.vip_users), include_str!("../../data/vip_users.txt"))?;

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

        let mut options = SaveOptions::default();
        options.screen_preparation = ScreenPreperation::ClearScreen;
        options.modern_terminal_output = true;
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
            &self.destination.join(&config.paths.no_ansi).with_extension("asc"),
            include_str!("../../data/new_bbs/noansi.asc"),
        )?;

        config.paths.conf_join_menu = PathBuf::from("art/cnfn");
        fs::write(
            &self.destination.join(&config.paths.conf_join_menu).with_extension("ppe"),
            include_bytes!("../../../../ppe/cnfn.ppe"),
        )?;

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
        list.save(&self.destination.join(&config.paths.group_file))?;

        self.logger.start_action("Create default user (SYSOP)".to_string());

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
        let mut user_base = UserBase::default();
        user_base.new_user(user);
        user_base.save(&self.destination.join(&config.paths.user_file))?;

        config.save(&self.destination.join(icy_board_engine::DEFAULT_ICYBOARD_FILE))?;

        self.logger.start_action("IcyBoard created successfully.".to_string());
        self.logger.start_action(format!(
            "Start with icy_board run \"{}\"",
            self.destination.join(icy_board_engine::DEFAULT_ICYBOARD_FILE).display()
        ));
        Ok(())
    }

    fn generate_default_conference(&self, conf_path: &PathBuf) -> Res<()> {
        let mut conf = Conference::default();
        conf.name = "Main Board".to_string();
        conf.is_public = true;
        conf.auto_rejoin = true;
        conf.use_main_commands = true;

        self.logger.start_action("Create conference directories".to_string());
        conf.attachment_location = PathBuf::from("conferences/main/attach");
        fs::create_dir_all(&self.destination.join(&conf.attachment_location))?;
        conf.pub_upload_location = PathBuf::from("conferences/main/upload");
        fs::create_dir_all(&self.destination.join(&conf.pub_upload_location))?;

        self.logger.start_action("Write user & sysop menus…".to_string());
        conf.users_menu = PathBuf::from("conferences/main/brdm");
        let mut options = SaveOptions::default();
        options.screen_preparation = ScreenPreperation::ClearScreen;
        options.modern_terminal_output = true;
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
        fs::write(&self.destination.join(&s.survey_file), include_bytes!("../../../../ppe/script2.ppe"))?;
        list.push(s);
        list.save(&self.destination.join(&conf.survey_file))?;

        // Create Directories
        self.logger.start_action("Create file directories…".to_string());
        conf.dir_menu = PathBuf::from("conferences/main/dir");
        fs::write(
            &self.destination.join(&conf.dir_menu).with_extension("ppe"),
            include_bytes!("../../../../ppe/dir.ppe"),
        )?;
        conf.dir_file = PathBuf::from("conferences/main/dir.toml");
        let mut list = DirectoryList::default();
        let fd = FileDirectory {
            name: "General".to_string(),
            path: PathBuf::from("conferences/main/general/files/dir00"),
            ..Default::default()
        };
        fs::create_dir_all(&self.destination.join(&fd.path))?;
        list.push(fd);
        list.save(&self.destination.join(&conf.dir_file))?;

        // Create message base
        self.logger.start_action("Create message areas…".to_string());
        conf.area_menu = PathBuf::from("conferences/main/area");
        fs::write(
            &self.destination.join(&conf.area_menu).with_extension("ppe"),
            include_bytes!("../../../../ppe/area.ppe"),
        )?;
        conf.area_file = PathBuf::from("conferences/main/area.toml");
        let mut list = AreaList::default();
        let fd = MessageArea {
            name: "General".to_string(),
            filename: PathBuf::from("conferences/main/messages/general"),
            ..Default::default()
        };
        fs::create_dir_all(&self.destination.join("conferences/main/messages"))?;
        let mut msg_base = JamMessageBase::create(&self.destination.join(&fd.filename))?;
        msg_base.write_message(&write_welcome_msg())?;
        msg_base.write_jhr_header()?;

        list.push(fd);

        list.save(&self.destination.join(&conf.area_file))?;

        // Create Door files
        self.logger.start_action("Create door file…".to_string());
        conf.doors_menu = PathBuf::from("conferences/main/door");
        fs::write(
            &self.destination.join(&conf.doors_menu).with_extension("ppe"),
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

fn write_welcome_msg() -> jamjam::jam::JamMessage {
    jamjam::jam::JamMessage::new(1, &EchomailAddress::default())
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
        uldl_ratio: 10,
        uldl_kb_ratio: 10,
        ..Default::default()
    });

    sec_level.levels.push(SecurityLevel {
        security: 20,
        time_per_day: 90,
        allow_alias: true,
        uldl_ratio: 90,
        uldl_kb_ratio: 90,
        ..Default::default()
    });

    sec_level.levels.push(SecurityLevel {
        security: 100,
        time_per_day: 540,
        allow_alias: true,
        uldl_ratio: 150,
        uldl_kb_ratio: 150,
        ..Default::default()
    });

    sec_level.levels.push(SecurityLevel {
        security: 110,
        time_per_day: 999,
        allow_alias: true,
        uldl_ratio: 250,
        uldl_kb_ratio: 250,
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

pub fn convert_to_pcb(path: &PathBuf, data: &[u8]) -> Res<()> {
    let mut options = SaveOptions::default();
    options.modern_terminal_output = true;
    convert_to_pcb_opt(path, data, &options)
}

pub fn convert_to_pcb_opt(path: &PathBuf, data: &[u8], opt: &SaveOptions) -> Res<()> {
    let mut buffer = Buffer::from_bytes(&PathBuf::from("a.icy"), true, data, None, None).unwrap();
    let bytes: Vec<u8> = buffer.to_bytes("pcb", opt).unwrap();
    fs::write(path.with_extension("pcb"), &bytes)?;
    Ok(())
}
