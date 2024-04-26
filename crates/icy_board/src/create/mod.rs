use std::{fs, path::PathBuf};

use bstr::BString;
use chrono::Utc;
use icy_board_engine::{
    icy_board::{
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
        pcboard_data::{PCBSysopSecurityLevels, UserSecurityLevels},
        sec_levels::{SecurityLevel, SecurityLevelDefinitions},
        statistics::Statistics,
        surveys::{Survey, SurveyList},
        user_base::{Password, PasswordInfo, User, UserBase},
        xfer_protocols::{Protocol, SupportedProtocols},
        IcyBoardSerializer,
    },
    Res,
};
use icy_net::protocol::TransferProtocolType;
use jamjam::{jam::JamMessageBase, util::echmoail::EchomailAddress};

use crate::import::{console_logger::ConsoleLogger, default_commands::add_default_commands, OutputLogger};

pub struct IcyBoardCreator {
    destination: PathBuf,
    logger: ConsoleLogger,
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
        fs::create_dir_all(&self.destination.join("config"))?;
        fs::create_dir_all(&self.destination.join("art/help"))?;

        self.logger.start_action("Creating main configuration… at {}".to_string());

        let mut config = IcbConfig::new();

        self.logger.start_action("Creating required paths.".to_string());
        fs::create_dir_all(&self.destination.join(&config.paths.help_path))?;
        fs::create_dir_all(&self.destination.join(&config.paths.tmp_work_path))?;
        fs::create_dir_all(&self.destination.join(&config.paths.security_file_path))?;
        fs::create_dir_all(&self.destination.join(&config.paths.command_display_path))?;
        fs::create_dir_all(&self.destination.join(&config.paths.home_dir))?;
        config.paths.security_file_path = PathBuf::from("art/secmsgs");
        fs::create_dir_all(&self.destination.join(&config.paths.security_file_path))?;

        self.logger.start_action("Write ICBTEXT…".to_string());
        DEFAULT_DISPLAY_TEXT.save(&self.destination.join(&config.paths.icbtext))?;

        self.logger.start_action("Write trashcan files…".to_string());
        config.paths.trashcan_user = PathBuf::from("config/tcan_user.txt");
        fs::write(&self.destination.join(&config.paths.trashcan_user), include_str!("../../data/tcan_users.txt"))?;
        config.paths.trashcan_email = PathBuf::from("config/tcan_email.txt");
        fs::write(&self.destination.join(&config.paths.trashcan_email), include_str!("../../data/tcan_email.txt"))?;
        config.paths.trashcan_passwords = PathBuf::from("config/tcan_passwords.txt");
        fs::write(
            &self.destination.join(&config.paths.trashcan_passwords),
            include_str!("../../data/tcan_passwords.txt"),
        )?;
        config.paths.vip_users = PathBuf::from("config/vip_users.txt");
        fs::write(&self.destination.join(&config.paths.vip_users), include_str!("../../data/vip_users.txt"))?;

        self.logger.start_action("Write protocol data files…".to_string());
        config.paths.protocol_data_file = PathBuf::from("config/protocols.toml");
        generate_protocol_data(&self.destination.join(&config.paths.protocol_data_file))?;

        self.logger.start_action("Write security data files…".to_string());
        config.paths.pwrd_sec_level_file = PathBuf::from("config/security_levels.toml");
        generate_security_level_data(&self.destination.join(&config.paths.pwrd_sec_level_file))?;

        self.logger.start_action("Write default conferences".to_string());
        self.generate_default_conference(&self.destination.join(&config.paths.conferences))?;

        self.logger.start_action("Write default command file".to_string());
        config.paths.command_file = PathBuf::from("config/commands.toml");

        let mut cmd_list = CommandList::default();
        add_default_commands(&get_default_data(), &mut cmd_list);
        cmd_list.save(&self.destination.join(&config.paths.command_file))?;

        self.logger.start_action("Write default art files".to_string());

        config.paths.welcome = PathBuf::from("art/welcome");
        fs::write(
            &self.destination.join(&config.paths.welcome).with_extension("pcb"),
            include_str!("../../data/new_bbs/welcome.pcb"),
        )?;
        config.paths.newuser = PathBuf::from("art/newuser");
        fs::write(
            &self.destination.join(&config.paths.newuser).with_extension("pcb"),
            include_str!("../../data/new_bbs/newuser.pcb"),
        )?;
        config.paths.closed = PathBuf::from("art/closed");
        fs::write(
            &self.destination.join(&config.paths.closed).with_extension("pcb"),
            include_str!("../../data/new_bbs/closed.pcb"),
        )?;

        config.paths.expire_warning = PathBuf::from("art/exp_warning");
        fs::write(
            &self.destination.join(&config.paths.expire_warning).with_extension("pcb"),
            include_str!("../../data/new_bbs/warning.pcb"),
        )?;
        config.paths.expired = PathBuf::from("art/expired");
        fs::write(
            &self.destination.join(&config.paths.expired).with_extension("pcb"),
            include_str!("../../data/new_bbs/expired.pcb"),
        )?;
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
        config.paths.statistics_file = PathBuf::from("config/statistics.toml");
        Statistics::default().save(&self.destination.join(&config.paths.statistics_file))?;

        self.logger.start_action("Write default language definition file".to_string());
        config.paths.language_file = PathBuf::from("config/languages.toml");
        let mut lang = SupportedLanguages::default();
        lang.languages.push(Language {
            description: "English".to_string(),
            yes_char: 'Y',
            no_char: 'N',
            ..Default::default()
        });
        lang.save(&self.destination.join(&config.paths.language_file))?;

        self.logger.start_action("Write default groups file".to_string());
        config.paths.group_file = PathBuf::from("config/groups.toml");
        let mut list = GroupList::default();
        list.add_group("sysop", "System sysops", &vec!["SYSOP".to_string()]);
        list.add_group("user", "Users", &vec![]);
        list.save(&self.destination.join(&config.paths.group_file))?;
        config.paths.log_file = PathBuf::from("output.log");

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
        user_base.save_users(&self.destination.join(&config.paths.home_dir))?;

        config.save(&self.destination.join("icyboard.toml"))?;

        self.logger.start_action("IcyBoard created successfully.".to_string());
        self.logger
            .start_action(format!("Start with icy_board run \"{}\"", self.destination.join("icyboard.toml").display()));
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
        conf.pub_upload_dir_file = PathBuf::from("conferences/main/upload.dir");

        self.logger.start_action("Write user & sysop menus…".to_string());
        conf.users_menu = PathBuf::from("conferences/main/brdm");
        fs::write(
            &self.destination.join(&conf.users_menu).with_extension("pcb"),
            include_str!("../../data/new_bbs/brdm.pcb"),
        )?;
        conf.sysop_menu = PathBuf::from("conferences/main/brds");
        fs::write(
            &self.destination.join(&conf.sysop_menu).with_extension("pcb"),
            include_str!("../../data/new_bbs/brds.pcb"),
        )?;
        conf.news_file = PathBuf::from("conferences/main/news");
        fs::write(
            &self.destination.join(&conf.news_file).with_extension("pcb"),
            include_str!("../../data/new_bbs/news.pcb"),
        )?;

        // Bulletin Menu
        self.logger.start_action("Write bulletins…".to_string());
        conf.blt_menu = PathBuf::from("conferences/main/blt");
        fs::write(
            &self.destination.join(&conf.blt_menu).with_extension("pcb"),
            include_str!("../../data/new_bbs/blt.pcb"),
        )?;
        conf.blt_file = PathBuf::from("conferences/main/blt.toml");

        let mut list = BullettinList::default();
        let path = PathBuf::from("conferences/main/rules");
        list.bullettins.push(Bullettin::new(&path));
        fs::write(
            &self.destination.join(&path).with_extension("pcb"),
            include_str!("../../data/new_bbs/rules.pcb"),
        )?;

        let path = PathBuf::from("conferences/main/history");
        list.bullettins.push(Bullettin::new(&path));
        fs::write(
            &self.destination.join(&path).with_extension("pcb"),
            include_str!("../../data/new_bbs/history.pcb"),
        )?;

        list.save(&self.destination.join(&conf.blt_file))?;

        // Surveys
        self.logger.start_action("Write surveys".to_string());
        conf.survey_menu = PathBuf::from("conferences/main/survey");
        fs::write(
            &self.destination.join(&conf.survey_menu).with_extension("pcb"),
            include_str!("../../data/new_bbs/survey.pcb"),
        )?;
        conf.survey_file = PathBuf::from("conferences/main/survey.toml");

        let mut list = SurveyList::default();
        let s = Survey {
            question_file: PathBuf::from("conferences/main/script1.pcb"),
            answer_file: PathBuf::from("conferences/main/script1.answer"),
            ..Default::default()
        };
        fs::write(
            &self.destination.join(&s.question_file).with_extension("pcb"),
            include_str!("../../data/new_bbs/script1.pcb"),
        )?;
        list.push(s);

        let s = Survey {
            question_file: PathBuf::from("conferences/main/script2.ppe"),
            answer_file: PathBuf::from("conferences/main/script2.answer"),
            ..Default::default()
        };
        fs::write(&self.destination.join(&s.question_file), include_bytes!("../../../../ppe/script2.ppe"))?;
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
            file_base: PathBuf::from("conferences/main/general/dir00"),
            path: PathBuf::from("conferences/main/general/files/dir00"),
            ..Default::default()
        };
        fs::create_dir_all(&self.destination.join(&fd.path))?;
        dizbase::file_base::FileBase::create(&self.destination.join(&fd.file_base))?;
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

fn get_default_data() -> icy_board_engine::icy_board::pcboard_data::PcbBoardData {
    icy_board_engine::icy_board::pcboard_data::PcbBoardData {
        sysop_security: PCBSysopSecurityLevels {
            sysop: 110,
            read_all_comments: 110,
            read_all_mail: 110,
            copy_move_messages: 110,
            enter_at_vars_in_messages: 110,
            edit_any_message: 110,
            not_update_msg_read_status: 110,
            use_broadcast_command: 110,
            view_private_uploads: 110,
            enter_generic_message: 110,
            edit_message_headers: 110,
            protect_unprotect_messages: 110,
            overwrite_uploads: 110,
            set_pack_out_date_on_messages: 110,
            see_all_return_receipt_messages: 110,
            subs: 110,
            edit_all: 110,
            read_only: 110,
            sec_15: 110,
            unused0: 110,
            keep_msg: 110,
            seeretrcpt: 110,
            sec_1_view_caller_log: 110,
            sec_2_view_usr_list: 110,
            sec_3_pack_renumber_msg: 110,
            sec_4_recover_deleted_msg: 110,
            sec_5_list_message_hdr: 110,
            sec_6_view_any_file: 110,
            sec_7_user_maint: 110,
            sec_8_pack_usr_file: 110,
            sec_9_exit_to_dos: 110,
            sec_10_shelled_dos_func: 110,
            sec_11_view_other_nodes: 110,
            sec_12_logoff_alt_node: 110,
            sec_13_drop_alt_node_to_dos: 110,
            sec_14_drop_to_dos: 110,
        },

        user_levels: UserSecurityLevels {
            cmd_a: 10,
            cmd_b: 10,
            cmd_c: 10,
            cmd_d: 10,
            cmd_e: 10,
            cmd_f: 10,
            cmd_g: 10,
            cmd_h: 10,
            cmd_i: 10,
            cmd_j: 10,
            cmd_k: 10,
            cmd_l: 10,
            cmd_m: 10,
            cmd_n: 10,
            cmd_o: 10,
            cmd_p: 10,
            cmd_q: 10,
            cmd_r: 10,
            cmd_s: 10,
            cmd_t: 10,
            cmd_u: 10,
            cmd_v: 10,
            cmd_w: 10,
            cmd_x: 10,
            cmd_y: 10,
            cmd_z: 10,
            cmd_chat: 10,
            cmd_open_door: 10,
            cmd_test_file: 10,
            cmd_show_user_list: 10,
            cmd_who: 10,
            batch_file_transfer: 10,
            edit_own_messages: 10,
            agree_to_register: 10,
            refuse_to_register: 10,
        },

        ..Default::default()
    }
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
    let mut protocols = SupportedProtocols::default();

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: false,
        char_code: "A".to_string(),
        description: "Ascii".to_string(),
        send_command: TransferProtocolType::ASCII,
        recv_command: TransferProtocolType::ASCII,
    });

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: false,
        char_code: "X".to_string(),
        description: "Xmodem/Checksum".to_string(),
        send_command: TransferProtocolType::XModem,
        recv_command: TransferProtocolType::XModem,
    });

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: false,
        char_code: "C".to_string(),
        description: "Xmodem/CRC".to_string(),
        send_command: TransferProtocolType::XModemCRC,
        recv_command: TransferProtocolType::XModemCRC,
    });

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: false,
        char_code: "O".to_string(),
        description: "1K-Xmodem       (a.k.a. non-BATCH Ymodem)".to_string(),
        send_command: TransferProtocolType::XModem1k,
        recv_command: TransferProtocolType::XModem1k,
    });

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: false,
        char_code: "F".to_string(),
        description: "1K-Xmodem/G     (a.k.a. non-BATCH Ymodem/G)".to_string(),
        send_command: TransferProtocolType::XModem1kG,
        recv_command: TransferProtocolType::XModem1kG,
    });

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: true,
        char_code: "Y".to_string(),
        description: "Ymodem BATCH".to_string(),
        send_command: TransferProtocolType::YModem,
        recv_command: TransferProtocolType::YModem,
    });

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: true,
        char_code: "G".to_string(),
        description: "Ymodem/G BATCH".to_string(),
        send_command: TransferProtocolType::YModemG,
        recv_command: TransferProtocolType::YModemG,
    });

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: true,
        char_code: "Z".to_string(),
        description: "Zmodem (batch)".to_string(),
        send_command: TransferProtocolType::ZModem,
        recv_command: TransferProtocolType::ZModem,
    });

    protocols.protocols.push(Protocol {
        is_enabled: true,
        is_batch: true,
        char_code: "Z8".to_string(),
        description: "Zmodem 8k (batch)".to_string(),
        send_command: TransferProtocolType::ZModem8k,
        recv_command: TransferProtocolType::ZModem8k,
    });

    protocols.save(protocol_data_file)?;
    Ok(())
}
