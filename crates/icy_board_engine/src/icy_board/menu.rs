use crate::Res;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use strum::{EnumIter, EnumString};

use crate::vm::errors::IcyError;

use super::{
    commands::{ActionTrigger, AutoRun, Command, CommandAction, CommandType},
    is_false, path_is_empty, read_with_encoding_detection,
    security::RequiredSecurity,
    IcyBoardSerializer,
};

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, EnumString, EnumIter, Debug)]
pub enum MenuType {
    #[default]
    Hotkey,
    Lightbar,
    Command,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
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
    pub menu_type: MenuType,

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
            return Err(IcyError::InvalidMNU(path.as_ref().to_string_lossy().to_string(), "Lines < 5".to_string()).into());
        }
        res.title = lines[0].to_string();

        let splitted_line = lines[1].split(',').collect::<Vec<&str>>();
        res.display_file = PathBuf::from(splitted_line[0]);
        if splitted_line.len() > 1 {
            res.force_display = splitted_line[1] == "1";
            res.menu_type = if splitted_line[2] == "1" { MenuType::Hotkey } else { MenuType::Command };
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
                    return Err(IcyError::InvalidMNU(path.as_ref().to_string_lossy().to_string(), "Invalid prompt line.".to_string()).into());
                }
                res.prompts.push((splitted_line[0].to_string(), splitted_line[1].to_string()));
            }
        }
        let mut cur_line = 4 + prompts as usize;

        let commands = lines[cur_line].parse::<i32>().unwrap_or(0);
        cur_line += 1;
        for i in 0..commands {
            let cmd = lines[cur_line + i as usize].to_string();
            let splitted_line = cmd.split(',').collect::<Vec<&str>>();

            if splitted_line.len() != 4 {
                return Err(IcyError::InvalidMNU(path.as_ref().to_string_lossy().to_string(), "Invalid command line.".to_string()).into());
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
                14 => CommandType::RunPPE,
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
                keyword,
                display: "".to_string(),
                lighbar_display: "".to_string(),
                position: Default::default(),
                auto_run: AutoRun::Disabled,
                autorun_time: 0,
                security: RequiredSecurity::new(security),
                help: "".to_string(),
                actions: vec![CommandAction {
                    command_type: cmd_type,
                    parameter,
                    trigger: ActionTrigger::default(),
                }],
            });
        }

        Ok(res)
    }

    pub fn up(&self, cur: usize) -> usize {
        let cp = self.commands[cur].position;
        let mut next = None;

        for i in 0..self.commands.len() {
            let cmd = &self.commands[i];
            if cur == i || cmd.position.y >= cp.y {
                continue;
            }
            if let Some(j) = next {
                let current: &Command = &self.commands[j];
                if (cmd.position.x as i32 - cp.x as i32).abs() < (current.position.x as i32 - cp.x as i32).abs() || current.position.y < cmd.position.y {
                    next = Some(i);
                }
            } else {
                next = Some(i);
            }
        }

        if next.is_none() {
            next = Some(cur);
            for i in 0..self.commands.len() {
                let cmd = &self.commands[i];
                let current: &Command = &self.commands[next.unwrap()];
                if current.position.y < cmd.position.y && cmd.position.x < cp.x {
                    next = Some(i);
                }
            }
        }
        if next == Some(cur) {
            for i in 0..self.commands.len() {
                let cmd = &self.commands[i];
                let current: &Command = &self.commands[next.unwrap()];
                if current.position.y < cmd.position.y || current.position.x < cmd.position.x {
                    next = Some(i);
                }
            }
        }
        next.unwrap_or_default()
    }

    pub fn down(&self, cur: usize) -> usize {
        let cp = self.commands[cur].position;
        let mut next = None;

        for i in 0..self.commands.len() {
            let cmd = &self.commands[i];
            if cur == i || cmd.position.y <= cp.y {
                continue;
            }
            if let Some(j) = next {
                let current: &Command = &self.commands[j];
                if (cmd.position.x as i32 - cp.x as i32).abs() < (current.position.x as i32 - cp.x as i32).abs() || current.position.y > cmd.position.y {
                    next = Some(i);
                }
            } else {
                next = Some(i);
            }
        }

        if next.is_none() {
            next = Some(cur);
            for i in 0..self.commands.len() {
                let cmd = &self.commands[i];
                let current: &Command = &self.commands[next.unwrap()];
                if current.position.y > cmd.position.y && cmd.position.x < cp.x {
                    next = Some(i);
                }
            }
        }
        if next == Some(cur) {
            for i in 0..self.commands.len() {
                let cmd = &self.commands[i];
                let current: &Command = &self.commands[next.unwrap()];
                if current.position.y > cmd.position.y || current.position.x < cmd.position.x {
                    next = Some(i);
                }
            }
        }
        next.unwrap_or_default()
    }

    pub fn left(&self, cur: usize) -> usize {
        let cp = self.commands[cur].position;
        let mut next = None;

        for i in 0..self.commands.len() {
            let cmd = &self.commands[i];
            if cur == i || cmd.position.x >= cp.x {
                continue;
            }
            if let Some(j) = next {
                let current: &Command = &self.commands[j];
                if (cmd.position.y as i32 - cp.y as i32).abs() < (current.position.y as i32 - cp.y as i32).abs() || current.position.x < cmd.position.x {
                    next = Some(i);
                }
            } else {
                next = Some(i);
            }
        }
        if next.is_none() {
            next = Some(cur);
        }
        next.unwrap_or_default()
    }

    pub fn right(&self, cur: usize) -> usize {
        let cp = self.commands[cur].position;
        let mut next = None;

        for i in 0..self.commands.len() {
            let cmd = &self.commands[i];
            if cur == i || cmd.position.x <= cp.x {
                continue;
            }
            if let Some(j) = next {
                let current: &Command = &self.commands[j];
                if (cmd.position.y as i32 - cp.y as i32).abs() < (current.position.y as i32 - cp.y as i32).abs() || current.position.x > cmd.position.x {
                    next = Some(i);
                }
            } else {
                next = Some(i);
            }
        }
        if next.is_none() {
            next = Some(cur);
        }
        next.unwrap_or_default()
    }
}

impl IcyBoardSerializer for Menu {
    const FILE_TYPE: &'static str = "menu";
}
