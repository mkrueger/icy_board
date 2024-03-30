use icy_board_engine::icy_board::{
    state::{functions::display_flags, IcyBoardState},
    text_messages::{COMMANDPROMPT, HELPPROMPT, PRESSENTER},
    IcyBoardError,
};
use icy_ppe::Res;

use self::actions::Menu;

pub mod actions;

pub struct IcyBoardCommand {
    pub state: IcyBoardState,
    pub menu: Menu,
}
const MASK_COMMAND: &str  = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;':,.<>?/\\\" ";

impl IcyBoardCommand {
    pub fn new(state: IcyBoardState) -> Self {
        let cmd_txt = include_str!("../../data/menu.cmd");
        let menu = Menu::read(cmd_txt);
        Self { state, menu }
    }

    pub fn do_command(&mut self) -> Res<()> {
        let current_conference = self.state.session.current_conference.number;

        let menu_file = if let Some(conference) = self
            .state
            .board
            .lock()
            .as_ref()
            .unwrap()
            .conferences
            .get(current_conference as usize)
        {
            if self.state.session.is_sysop {
                &conference.sysop_menu
            } else {
                &conference.users_menu
            }
            .clone()
        } else {
            return Ok(());
        };

        self.state.display_file(&menu_file)?;

        let command = self.state.input_field(
            COMMANDPROMPT,
            40,
            MASK_COMMAND,
            display_flags::NEWLINE | display_flags::UPCASE,
        )?;

        for action in &self.menu.actions {
            if action.commands.contains(&command) {
                return self.dispatch_action(action.action.clone());
            }
        }
        Ok(())
    }

    fn dispatch_action(&mut self, clone: Option<String>) -> Res<()> {
        if let Some(act) = clone {
            match act.as_str() {
                "GOODBYE" => {
                    self.state
                        .hangup(icy_board_engine::vm::HangupType::Goodbye)?;
                }
                "WHO" => {}
                "HELP" => {
                    self.show_help()?;
                }
                _ => {
                    return Err(Box::new(IcyBoardError::UnknownAction(act.to_string())));
                }
            }
        }
        Ok(())
    }

    fn show_help(&mut self) -> Res<()> {
        let command = self.state.input_field(
            HELPPROMPT,
            8,
            MASK_COMMAND,
            display_flags::UPCASE | display_flags::NEWLINE | display_flags::HIGHASCII,
        )?;
        if !command.is_empty() {
            let mut help_loc = self.state.board.lock().unwrap().data.path.help_loc.clone();
            for action in &self.menu.actions {
                if action.commands.contains(&command) {
                    if !action.help.is_empty() {
                        help_loc.push_str(&action.help);
                    } else {
                        return Ok(());
                    }
                    break;
                }
            }
            self.state.display_file(&help_loc)?;
            self.state.more_promt(PRESSENTER)?;
        }

        Ok(())
    }
}
