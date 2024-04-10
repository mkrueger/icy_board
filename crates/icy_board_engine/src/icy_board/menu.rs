use icy_ppe::Res;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::vm::errors::IcyError;

use super::{
    commands::{Command, CommandType},
    is_false, path_is_empty, read_with_encoding_detection,
    security::RequiredSecurity,
    IcyBoardSerializer,
};

#[derive(Serialize, Deserialize, Default)]
pub struct Menu {
    pub title: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "path_is_empty")]
    pub display_file: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "path_is_empty")]
    pub help_file: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub force_display: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_hotkeys: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub pass_through: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub prompt: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<(String, String)>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<Command>,
}

impl Menu {
    pub fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        let mut res = Self::default();

        let txt = &read_with_encoding_detection(path)?;
        let lines = txt.lines().collect::<Vec<&str>>();

        if lines.len() < 5 {
            return Err(IcyError::InvalidMNU(
                path.as_ref().to_string_lossy().to_string(),
                "Lines < 5".to_string(),
            )
            .into());
        }
        res.title = lines[0].to_string();

        let splitted_line = lines[1].split(',').collect::<Vec<&str>>();
        res.display_file = PathBuf::from(splitted_line[0]);
        if splitted_line.len() > 1 {
            res.force_display = splitted_line[1] == "1";
            res.use_hotkeys = splitted_line[2] == "1";
            res.pass_through = splitted_line[3] == "1";
        }
        res.help_file = PathBuf::from(lines[2].to_string());

        let prompts = lines[3].parse::<i32>().unwrap_or(0);
        if prompts > 0 {
            res.prompt = lines[4].to_string();
            for i in 1..prompts {
                let command_line = lines[4 + i as usize].to_string();
                let splitted_line = command_line.split(',').collect::<Vec<&str>>();

                if splitted_line.len() != 2 {
                    return Err(IcyError::InvalidMNU(
                        path.as_ref().to_string_lossy().to_string(),
                        "Invalid prompt line.".to_string(),
                    )
                    .into());
                }
                res.prompts
                    .push((splitted_line[0].to_string(), splitted_line[1].to_string()));
            }
        }
        let mut cur_line = 4 + prompts as usize;

        let commands = lines[cur_line].parse::<i32>().unwrap_or(0);
        cur_line += 1;
        for i in 0..commands {
            let cmd = lines[cur_line + i as usize].to_string();
            let splitted_line = cmd.split(',').collect::<Vec<&str>>();

            if splitted_line.len() != 4 {
                return Err(IcyError::InvalidMNU(
                    path.as_ref().to_string_lossy().to_string(),
                    "Invalid command line.".to_string(),
                )
                .into());
            }
            let keyword = splitted_line[0].to_string();
            let security = splitted_line[1].parse::<u8>().unwrap_or(0);
            let cmd_type = splitted_line[2].parse::<u8>().unwrap_or(0);
            let parameter = splitted_line[3].to_string();

            let cmd_type = match cmd_type {
                0 => CommandType::Menu,
                1 => CommandType::Script,
                2 => CommandType::BulletinList,
                3 => CommandType::DisplayFile,
                4 => CommandType::Door,
                5 => CommandType::Conference,
                6 => CommandType::DisplayDir,
                7 => CommandType::StuffText,
                8 => CommandType::StuffFile,
                9 => CommandType::ExpertMode,
                10 => CommandType::Goodbye,
                11 => CommandType::Goodbye,
                12 => CommandType::QuitMenu,
                13 => CommandType::ExitMenus,
                14 => CommandType::PPE,
                15 => CommandType::StuffTextAndExitMenu,
                16 => CommandType::StuffTextAndExitMenuSilent,
                17 => CommandType::Disabled,
                18 => CommandType::StuffFile,
                19 => CommandType::StuffTextAndExitMenuSilent,
                20 => CommandType::Command,
                21 => CommandType::GlobalCommand,

                err => {
                    log::error!("Invalid command type: {}, defaulting to command.", err);
                    CommandType::Command
                }
            };

            res.commands.push(Command {
                input: vec![keyword],
                security: RequiredSecurity::new(security),
                help: "".to_string(),
                command_type: cmd_type,
                parameter,
            });
        }

        Ok(res)
    }
}

impl IcyBoardSerializer for Menu {
    const FILE_TYPE: &'static str = "menu";
}
