use std::{
    fs,
    io::{stdout, BufWriter, Write},
    path::{Path, PathBuf},
    process,
    str::FromStr,
};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};
use icy_board_engine::icy_board::{
    commands::CommandList,
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
    write_with_bom,
    xfer_protocols::SupportedProtocols,
    IcyBoardError, PcbBoard,
};
use icy_ppe::{datetime::IcbTime, tables::import_cp437_string, Res};
use relative_path::{PathExt, RelativePathBuf};
use walkdir::WalkDir;

const REQUIRED_DIRECTORIES: [&str; 7] = [
    "gen",
    "conferences",
    "conferences/main",
    "data",
    "config",
    "help",
    "art",
];

pub fn convert_pcb(pcb_file: &PcbBoard, output_directory: &PathBuf) -> Res<()> {
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
    check_result(pcb_file.display_text.save(&o));

    let o = output_directory.join("data/user_base.toml");
    start_action(format!("Create user base {}…", o.display()));
    let user_base = UserBase::import_pcboard(&pcb_file.users);
    check_result(user_base.save(&o));

    let o = output_directory.join("conferences/main/commands.toml");
    fs::write(o, include_str!("../data/menu.cmd"))?;
    let mut conferences = ConferenceBase::import_pcboard(pcb_file);
    conferences[0].command_file = PathBuf::from("conferences/main/commands.toml");

    /* TODO: Directory conversion
    let mut conference_directories = HashMap::new();
    for c in conferences.entries {

    }*/

    let o = output_directory.join("conferences/list.toml");
    start_action(format!("Create conferences {}…", o.display()));
    check_result(conferences.save(&o));

    let o = output_directory.join("icbcfg.toml");
    let icb_cfg = import_pcboard_cfg(pcb_file, o.parent().unwrap())?;
    start_action(format!("Create main configutation {}…", o.display()));
    check_result(icb_cfg.save(&o));

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

            check_result(convert_to_utf8(&entry.path(), &to));
        }
    }

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

fn import_file(pcb: &PcbBoard, dest_path: &Path, file: &str, new_name: &str) -> Res<PathBuf> {
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
                    check_result(convert_to_utf8(&entry.path(), &dest));
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
    let res = if resolved_file.exists() {
        CommandList::import_pcboard(&resolved_file)?
    } else {
        CommandList::default()
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

pub fn import_pcboard_cfg(pcb: &PcbBoard, dest_path: &Path) -> Res<IcbConfig> {
    let welcome = import_file(pcb, dest_path, &pcb.data.path.welcome_file, "art/welcome")?;
    let newuser = import_file(pcb, dest_path, &pcb.data.path.newuser_file, "art/newuser")?;
    let closed = import_file(pcb, dest_path, &pcb.data.path.closed_file, "art/closed")?;
    let warning = import_file(pcb, dest_path, &pcb.data.path.warning_file, "art/warning")?;
    let expired = import_file(pcb, dest_path, &pcb.data.path.expired_file, "art/expired")?;
    let conf_join_menu = import_file(pcb, dest_path, &pcb.data.path.conf_menu, "art/cnfn")?;
    let group_chat = import_file(pcb, dest_path, &pcb.data.path.group_chat, "art/group")?;
    let chat_menu = import_file(pcb, dest_path, &pcb.data.path.chat_menu, "art/chtm")?;
    let no_ansi = import_file(pcb, dest_path, &pcb.data.path.no_ansi, "art/noansi")?;
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
            conferences: PathBuf::from("conferences/list.toml"),
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
