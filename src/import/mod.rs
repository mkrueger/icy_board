use std::{
    fs,
    io::stdout,
    path::{Path, PathBuf},
    process,
    str::FromStr,
};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};
use icy_board_engine::icy_board::{
    commands::{CommandList, CommandType},
    conferences::ConferenceBase,
    convert_to_utf8,
    icb_config::{
        ColorConfiguration, ConfigPaths, IcbColor, IcbConfig, PasswordStorageMethod,
        SubscriptionMode, SysopInformation, SysopSecurityLevels, UserPasswordPolicy,
    },
    language::SupportedLanguages,
    read_cp437,
    sec_levels::SecurityLevelDefinitions,
    user_base::{Password, UserBase},
    xfer_protocols::SupportedProtocols,
    IcyBoardError, PcbBoard,
};
use icy_board_engine::icy_board::{
    state::functions::PPECall, statistics::Statistics, write_with_bom, IcyBoardSerializer,
    PCBoardImporter, PCBoardRecordImporter,
};
use icy_ppe::{datetime::IcbTime, Res};
use relative_path::{PathExt, RelativePathBuf};
use walkdir::WalkDir;

pub mod import_log;

const REQUIRED_DIRECTORIES: [&str; 7] = [
    "gen",
    "conferences",
    "conferences/main",
    "data",
    "config",
    "help",
    "art",
];

pub fn convert_pcb(pcb_file: &mut PcbBoard, output_directory: &PathBuf) -> Res<()> {
    start_action(format!(
        "Creating directory '{}'…",
        output_directory.display()
    ));
    check_result(fs::create_dir(output_directory));

    for dir in REQUIRED_DIRECTORIES.iter() {
        let o = output_directory.join(dir);
        start_action(format!("Creating directory '{}'…", o.display()));
        check_result(fs::create_dir(o));
    }

    let o = output_directory.join("data/icbtext.toml");
    start_action(format!("Create ICBTEXT {}…", o.display()));

    for entry in pcb_file.display_text.iter_mut() {
        entry.text = scan_line_for_commands(&entry.text);
    }

    check_result(pcb_file.display_text.save(&o));

    let o = output_directory.join("data/user_base.toml");
    start_action(format!("Create user base {}…", o.display()));
    let user_base = UserBase::import_pcboard(&pcb_file.users);
    check_result(user_base.save(&o));

    let o = output_directory.join("icyboard.toml");
    let icb_cfg = import_pcboard_cfg(pcb_file, o.parent().unwrap())?;
    start_action(format!("Create main configutation {}…", o.display()));
    check_result(icb_cfg.save(&o));

    let o = output_directory.join("config/conferences.toml");
    start_action(format!("Create conferences {}…", o.display()));
    let mut conferences = ConferenceBase::import_pcboard(pcb_file);
    check_result(conferences.save(&o));

    let help_loc = pcb_file.resolve_file(&pcb_file.data.path.help_loc);
    let help_loc = PathBuf::from(&help_loc);

    let o = output_directory.join("help");
    if help_loc.exists() {
        start_action(format!(
            "Copy help files from {} to {}…",
            help_loc.display(),
            o.display()
        ));
        println!();
        for entry in WalkDir::new(&help_loc) {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                continue;
            }
            let rel_path = entry.path().relative_to(&help_loc).unwrap();
            let lower_case = RelativePathBuf::from_path(rel_path.as_str().to_lowercase()).unwrap();
            let to = lower_case.to_logical_path(&o);
            if let Some(parent_dir) = to.parent() {
                if !parent_dir.exists() {
                    fs::create_dir(parent_dir).unwrap();
                }
            }
            start_action(format!("\t{} -> {}…", entry.path().display(), to.display()));
            check_result(import_and_scan_file(&entry.path(), &to));
        }
    }
    Ok(())
}

pub fn scan_line_for_commands(pcb: &PcbBoard, logger: &mut ImportLog, line: &str) -> String {
    if let Some(call) = PPECall::try_parse_line(line) {
        let resolved_file = pcb.resolve_file(&call.file);
    }
    line.to_string()
}

pub fn import_and_scan_file<P: AsRef<Path>, Q: AsRef<Path>>(from: &P, to: &Q) -> Res<()> {
    let in_string = read_cp437(from)?;

    let mut import = String::new();

    for line in in_string.lines() {
        import.push_str(&scan_line_for_commands(line));
        import.push('\n');
    }

    write_with_bom(to, &import)?;
    Ok(())
}

fn start_action(format: String) {
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        Print(format),
        SetAttribute(Attribute::Reset)
    )
    .unwrap();
}

fn scan_display_file(pcb: &PcbBoard, dest_path: &Path, file: &str, new_name: &str) -> Res<PathBuf> {
    if file.is_empty() {
        return Ok(PathBuf::new());
    }
    let resolved_file = pcb.resolve_file(file);
    let from_file = PathBuf::from(&resolved_file);

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
            //     println!("{}---{}", found_name, upper_name );

            if found_name.starts_with(&upper_name) {
                let mut dest = dest_path.to_path_buf();
                dest = dest.join(new_name);
                dest.pop();

                dest.push(entry.file_name().to_ascii_lowercase());
                if !found_name.ends_with(".PPS")
                    && (/*found_name.ends_with(".PPE") ||*/found_name.contains('.'))
                {
                    start_action(format!(
                        "\t copy '{}' to '{}'…",
                        entry.path().display(),
                        dest.display()
                    ));
                    check_result(fs::copy(entry.path(), dest));
                } else {
                    start_action(format!(
                        "\t convert '{}' to utf8 '{}'…",
                        entry.path().display(),
                        dest.display()
                    ));
                    check_result(import_and_scan_file(&entry.path(), &dest));
                }
            }
        }
    }
    Ok(PathBuf::from(new_name))
}

fn convert_trashcan(pcb: &PcbBoard, dest_path: &Path, file: &str, new_name: &str) -> Res<PathBuf> {
    if file.is_empty() {
        return Ok(PathBuf::new());
    }

    let resolved_file = pcb.resolve_file(file);
    let resolved_file = PathBuf::from(&resolved_file);
    let trashcan_header = include_str!("../data/badusers_empty.txt");
    if !resolved_file.exists() {
        fs::write(new_name, trashcan_header)?;
        return Ok(PathBuf::from(new_name));
    }
    let mut trashcan = regex::escape(&read_cp437(&resolved_file)?);
    if !trashcan.ends_with('\n') {
        trashcan.push('\n');
    }

    let mut dest = dest_path.to_path_buf();
    dest = dest.join(new_name);

    if let Err(err) = fs::write(dest, trashcan_header.to_string() + trashcan.as_str()) {
        return Err(Box::new(IcyBoardError::ErrorCreatingFile(
            new_name.to_string(),
            err.to_string(),
        )));
    }
    println!("done.");
    Ok(PathBuf::from(new_name))
}

fn convert_pcml_dat(pcb: &PcbBoard, dest_path: &Path, file: &str, new_name: &str) -> Res<PathBuf> {
    if file.is_empty() {
        return Ok(PathBuf::new());
    }

    let resolved_file = pcb.resolve_file(file);
    let resolved_file = PathBuf::from(&resolved_file);

    let res = if resolved_file.exists() {
        SupportedLanguages::import_pcboard(&resolved_file)?
    } else {
        SupportedLanguages::default()
    };

    let mut dest = dest_path.to_path_buf();
    dest = dest.join(new_name);
    if let Err(err) = res.save(&dest) {
        return Err(Box::new(IcyBoardError::ErrorCreatingFile(
            new_name.to_string(),
            err.to_string(),
        )));
    }
    Ok(PathBuf::from(new_name))
}

fn convert_pcprot_dat(
    pcb: &PcbBoard,
    dest_path: &Path,
    file: &str,
    new_name: &str,
) -> Res<PathBuf> {
    if file.is_empty() {
        return Ok(PathBuf::new());
    }
    let resolved_file = pcb.resolve_file(file);
    let resolved_file = PathBuf::from(&resolved_file);
    let res = if resolved_file.exists() {
        SupportedProtocols::import_pcboard(&resolved_file)?
    } else {
        SupportedProtocols::default()
    };
    let mut dest = dest_path.to_path_buf();
    dest = dest.join(new_name);
    if let Err(err) = res.save(&dest) {
        return Err(Box::new(IcyBoardError::ErrorCreatingFile(
            new_name.to_string(),
            err.to_string(),
        )));
    }
    Ok(PathBuf::from(new_name))
}

fn convert_pwrd(pcb: &PcbBoard, dest_path: &Path, file: &str, new_name: &str) -> Res<PathBuf> {
    if file.is_empty() {
        return Ok(PathBuf::new());
    }
    let resolved_file = pcb.resolve_file(file);
    let resolved_file = PathBuf::from(&resolved_file);
    let res = if resolved_file.exists() {
        SecurityLevelDefinitions::import_pcboard(&resolved_file)?
    } else {
        SecurityLevelDefinitions::default()
    };
    let mut dest = dest_path.to_path_buf();
    dest = dest.join(new_name);
    if let Err(err) = res.save(&dest) {
        return Err(Box::new(IcyBoardError::ErrorCreatingFile(
            new_name.to_string(),
            err.to_string(),
        )));
    }
    Ok(PathBuf::from(new_name))
}

fn convert_cmd_lst(pcb: &PcbBoard, dest_path: &Path, file: &str, new_name: &str) -> Res<PathBuf> {
    if file.is_empty() {
        return Ok(PathBuf::new());
    }
    let resolved_file = pcb.resolve_file(file);
    let resolved_file = PathBuf::from(&resolved_file);
    let mut res = if resolved_file.exists() {
        CommandList::import_pcboard(&resolved_file)?
    } else {
        CommandList::default()
    };

    add_default_commands(pcb, &mut res);

    let mut dest = dest_path.to_path_buf();
    dest = dest.join(new_name);
    if let Err(err) = res.save(&dest) {
        return Err(Box::new(IcyBoardError::ErrorCreatingFile(
            new_name.to_string(),
            err.to_string(),
        )));
    }
    Ok(PathBuf::from(new_name))
}
fn convert_pcbstats_lst(
    pcb: &PcbBoard,
    dest_path: &Path,
    file: &str,
    new_name: &str,
) -> Res<PathBuf> {
    if file.is_empty() {
        return Ok(PathBuf::new());
    }
    let resolved_file = pcb.resolve_file(file);
    let resolved_file = PathBuf::from(&resolved_file);
    let mut res = if resolved_file.exists() {
        Statistics::import_pcboard(&resolved_file)?
    } else {
        Statistics::default()
    };
    let mut dest = dest_path.to_path_buf();
    dest = dest.join(new_name);
    if let Err(err) = res.save(&dest) {
        return Err(Box::new(IcyBoardError::ErrorCreatingFile(
            new_name.to_string(),
            err.to_string(),
        )));
    }
    Ok(PathBuf::from(new_name))
}

fn convert_cmd(
    name: &[&str],
    cmd_type: CommandType,
    security: i32,
) -> icy_board_engine::icy_board::commands::Command {
    icy_board_engine::icy_board::commands::Command {
        input: name.iter().map(|s| s.to_string()).collect(),
        help: format!("hlp{}", name[0].to_ascii_lowercase()),
        command_type: cmd_type,
        parameter: "".to_string(),
        security: security as u8,
    }
}

fn add_default_commands(pcb: &PcbBoard, cmd_list: &mut CommandList) {
    cmd_list.push(convert_cmd(
        &["A"],
        CommandType::AbandonConference,
        pcb.data.user_levels.cmd_a,
    ));
    cmd_list.push(convert_cmd(
        &["B"],
        CommandType::BulletinList,
        pcb.data.user_levels.cmd_b,
    ));
    cmd_list.push(convert_cmd(
        &["C"],
        CommandType::CommentToSysop,
        pcb.data.user_levels.cmd_c,
    ));
    cmd_list.push(convert_cmd(
        &["D", "FLAG", "FLA", "FL", "DOWN", "DOW", "DO"],
        CommandType::Download,
        pcb.data.user_levels.cmd_d,
    ));
    cmd_list.push(convert_cmd(
        &["E"],
        CommandType::EnterMessage,
        pcb.data.user_levels.cmd_e,
    ));
    cmd_list.push(convert_cmd(
        &["F"],
        CommandType::FileDirectory,
        pcb.data.user_levels.cmd_f,
    ));

    // doesn't make sense to have a sec for that but it's in the record
    cmd_list.push(convert_cmd(
        &["G", "BYE", "BY"],
        CommandType::Goodbye,
        pcb.data.user_levels.cmd_g,
    ));

    cmd_list.push(convert_cmd(
        &["H", "HELP", "HEL", "HE", "?"],
        CommandType::Help,
        pcb.data.user_levels.cmd_h,
    ));
    cmd_list.push(convert_cmd(
        &["IW"],
        CommandType::InitialWelcome,
        pcb.data.user_levels.cmd_i,
    ));
    cmd_list.push(convert_cmd(
        &["J", "JOIN", "JOI", "JO"],
        CommandType::JoinConference,
        pcb.data.user_levels.cmd_j,
    ));
    cmd_list.push(convert_cmd(
        &["I"],
        CommandType::MessageAreas,
        pcb.data.user_levels.cmd_j,
    ));
    cmd_list.push(convert_cmd(
        &["K"],
        CommandType::KillMessage,
        pcb.data.user_levels.cmd_k,
    ));
    cmd_list.push(convert_cmd(
        &["L"],
        CommandType::LocateFile,
        pcb.data.user_levels.cmd_l,
    ));
    cmd_list.push(convert_cmd(
        &["M"],
        CommandType::ToggleGraphics,
        pcb.data.user_levels.cmd_m,
    ));
    cmd_list.push(convert_cmd(
        &["N"],
        CommandType::NewFileScan,
        pcb.data.user_levels.cmd_n,
    ));
    cmd_list.push(convert_cmd(
        &["O"],
        CommandType::PageSysop,
        pcb.data.user_levels.cmd_o,
    ));
    cmd_list.push(convert_cmd(
        &["P"],
        CommandType::SetPageLength,
        pcb.data.user_levels.cmd_p,
    ));
    cmd_list.push(convert_cmd(
        &["Q"],
        CommandType::QuickMessageScan,
        pcb.data.user_levels.cmd_q,
    ));
    cmd_list.push(convert_cmd(
        &["R"],
        CommandType::ReadMessages,
        pcb.data.user_levels.cmd_r,
    ));
    cmd_list.push(convert_cmd(
        &["S"],
        CommandType::ScriptQuest,
        pcb.data.user_levels.cmd_s,
    ));
    cmd_list.push(convert_cmd(
        &["T"],
        CommandType::TransferProtocol,
        pcb.data.user_levels.cmd_t,
    ));
    cmd_list.push(convert_cmd(
        &["U"],
        CommandType::UploadFile,
        pcb.data.user_levels.cmd_u,
    ));
    cmd_list.push(convert_cmd(
        &["V"],
        CommandType::ViewSettings,
        pcb.data.user_levels.cmd_v,
    ));
    cmd_list.push(convert_cmd(
        &["W"],
        CommandType::WriteUserSettings,
        pcb.data.user_levels.cmd_w,
    ));
    cmd_list.push(convert_cmd(
        &["X"],
        CommandType::ExpertMode,
        pcb.data.user_levels.cmd_x,
    ));
    cmd_list.push(convert_cmd(
        &["Y"],
        CommandType::PersonalMail,
        pcb.data.user_levels.cmd_y,
    ));
    cmd_list.push(convert_cmd(
        &["Z"],
        CommandType::ZippyDirectoryScan,
        pcb.data.user_levels.cmd_z,
    ));

    cmd_list.push(convert_cmd(
        &["CHAT", "CHA", "CH"],
        CommandType::GroupChat,
        pcb.data.user_levels.cmd_chat,
    ));
    cmd_list.push(convert_cmd(
        &["DOOR", "DOO", "DO", "OPEN", "OPE", "OP"],
        CommandType::OpenDoor,
        pcb.data.user_levels.cmd_open_door,
    ));
    cmd_list.push(convert_cmd(
        &["TEST", "TES", "TE"],
        CommandType::TestFile,
        pcb.data.user_levels.cmd_test_file,
    ));
    cmd_list.push(convert_cmd(
        &["USER", "USE", "US"],
        CommandType::UserList,
        pcb.data.user_levels.cmd_show_user_list,
    ));
    cmd_list.push(convert_cmd(
        &["WHO", "WH"],
        CommandType::WhoIsOnline,
        pcb.data.user_levels.cmd_who,
    ));
    cmd_list.push(convert_cmd(&["MENU", "MEN", "ME"], CommandType::Menu, 0));
    cmd_list.push(convert_cmd(
        &["NEWS", "NEW", "NE"],
        CommandType::DisplayNews,
        0,
    ));
    cmd_list.push(convert_cmd(
        &["LANG", "LAN", "LA"],
        CommandType::SetLanguage,
        0,
    ));
    cmd_list.push(convert_cmd(
        &["REPLY", "REPL", "REP", "RE"],
        CommandType::ReplyMessage,
        0,
    ));
    cmd_list.push(convert_cmd(
        &["ALIAS", "ALIA", "ALI", "AL"],
        CommandType::EnableAlias,
        0,
    ));

    cmd_list.push(convert_cmd(
        &["BRDCST"],
        CommandType::Broadcast,
        pcb.data.sysop_security.sysop,
    ));
}

pub fn import_pcboard_cfg(pcb: &PcbBoard, dest_path: &Path) -> Res<IcbConfig> {
    let welcome = scan_display_file(pcb, dest_path, &pcb.data.path.welcome_file, "art/welcome")?;
    let newuser = scan_display_file(pcb, dest_path, &pcb.data.path.newuser_file, "art/newuser")?;
    let closed = scan_display_file(pcb, dest_path, &pcb.data.path.closed_file, "art/closed")?;
    let warning = scan_display_file(pcb, dest_path, &pcb.data.path.warning_file, "art/warning")?;
    let expired = scan_display_file(pcb, dest_path, &pcb.data.path.expired_file, "art/expired")?;
    let conf_join_menu = scan_display_file(pcb, dest_path, &pcb.data.path.conf_menu, "art/cnfn")?;
    let group_chat = scan_display_file(pcb, dest_path, &pcb.data.path.group_chat, "art/group")?;
    let chat_menu = scan_display_file(pcb, dest_path, &pcb.data.path.chat_menu, "art/chtm")?;
    let no_ansi = scan_display_file(pcb, dest_path, &pcb.data.path.no_ansi, "art/noansi")?;
    let trashcan = convert_trashcan(
        pcb,
        dest_path,
        &pcb.data.path.tcan_file,
        "config/badusers.txt",
    )?;
    let language_file = convert_pcml_dat(
        pcb,
        dest_path,
        &pcb.data.path.pcml_dat_file,
        "config/languages.toml",
    )?;
    let protocol_data_file = convert_pcprot_dat(
        pcb,
        dest_path,
        &pcb.data.path.protocol_data_file,
        "config/protocols.toml",
    )?;
    let security_level_file = convert_pwrd(
        pcb,
        dest_path,
        &pcb.data.path.pwd_file,
        "config/security_levels.toml",
    )?;
    let command_file = convert_cmd_lst(
        pcb,
        dest_path,
        &pcb.data.path.cmd_lst,
        "config/commands.toml",
    )?;
    let statistics_file = convert_pcbstats_lst(
        pcb,
        dest_path,
        &pcb.data.path.stats_file,
        "data/statistics.toml",
    )?;

    let pcb_data = &pcb.data;
    Ok(IcbConfig {
        sysop: SysopInformation {
            name: pcb_data.sysop_info.sysop.clone(),
            password: Password::from_str(pcb_data.sysop_info.password.as_str()).unwrap(),
            require_password_to_exit: pcb_data.sysop_info.require_pwrd_to_exit,
            use_real_name: pcb_data.sysop_info.use_real_name,
            sysop_start: IcbTime::parse(&pcb_data.sysop_start),
            sysop_stop: IcbTime::parse(&pcb_data.sysop_stop),
        },
        sysop_security_level: SysopSecurityLevels {
            sysop: pcb_data.sysop_security.sysop as u8,
            read_all_comments: pcb_data.sysop_security.read_all_comments as u8,
            read_all_mail: pcb_data.sysop_security.read_all_mail as u8,
            copy_move_messages: pcb_data.sysop_security.copy_move_messages as u8,
            use_broadcast_command: pcb_data.sysop_security.use_broadcast_command as u8,
            view_private_uploads: pcb_data.sysop_security.view_private_uploads as u8,
            edit_message_headers: pcb_data.sysop_security.edit_message_headers as u8,
            protect_unprotect_messages: pcb_data.sysop_security.protect_unprotect_messages as u8,
        },
        color_configuration: ColorConfiguration {
            default: IcbColor::Dos(pcb_data.colors.default as u8),
            msg_hdr_date: IcbColor::Dos(pcb_data.colors.msg_hdr_date as u8),
            msg_hdr_to: IcbColor::Dos(pcb_data.colors.msg_hdr_to as u8),
            msg_hdr_from: IcbColor::Dos(pcb_data.colors.msg_hdr_from as u8),
            msg_hdr_subj: IcbColor::Dos(pcb_data.colors.msg_hdr_subj as u8),
            msg_hdr_read: IcbColor::Dos(pcb_data.colors.msg_hdr_read as u8),
            msg_hdr_conf: IcbColor::Dos(pcb_data.colors.msg_hdr_conf as u8),
        },
        board_name: pcb_data.board_name.clone(),
        func_keys: pcb_data.func_keys.clone(),
        subscription_info: SubscriptionMode {
            is_enabled: pcb_data.subscription_info.is_enabled,
            subscription_length: pcb_data.subscription_info.subscription_length,
            default_expired_level: pcb_data.subscription_info.default_expired_level,
            warning_days: pcb_data.subscription_info.warning_days,
        },
        user_password_policy: UserPasswordPolicy {
            min_length: pcb_data.min_pwrd_len as u8,
            password_expire_days: pcb_data.pwrd_update as u16,
            password_expire_warn_days: pcb_data.pwrd_warn as u16,
            password_storage_method: PasswordStorageMethod::PlainText,
        },
        paths: ConfigPaths {
            help_path: PathBuf::from("./help/"),
            tmp_path: PathBuf::from("./tmp/"),
            icbtxt: PathBuf::from("data/icbtext.toml"),
            user_base: PathBuf::from("data/user_base.toml"),
            conferences: PathBuf::from("config/conferences.toml"),
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
    })
}

fn check_result<S, T: std::fmt::Display>(res: Result<S, T>) {
    match res {
        Ok(_) => {
            execute!(
                stdout(),
                SetAttribute(Attribute::Bold),
                SetForegroundColor(Color::Green),
                Print(" OK".to_string()),
                SetAttribute(Attribute::Reset),
                Print("\n")
            )
            .unwrap();
        }
        Err(e) => {
            execute!(
                stdout(),
                SetAttribute(Attribute::Bold),
                SetForegroundColor(Color::Red),
                Print(" Error:".to_string()),
                SetAttribute(Attribute::Reset),
                Print(format!(" {}\n", e))
            )
            .unwrap();
            process::exit(1);
        }
    }
}
