use icy_board_engine::{
    icy_board::{commands::Command, icb_config::IcbColor, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};
use icy_ppe::Res;

use super::PcbBoardCommand;

impl PcbBoardCommand {
    pub fn set_transfer_protocol(&mut self, _action: &Command) -> Res<()> {
        self.displaycmdfile("preprot")?;
        if self.displaycmdfile("prot")? {
            return Ok(());
        }

        let mut protocols = Vec::new();
        let mut valid_protocols = String::new();
        let cur_protocol = if let Some(user) = &mut self.state.current_user { user.protocol } else { ' ' };
        self.state.new_line()?;

        if let Ok(board) = self.state.board.lock() {
            for protocol in board.protocols.iter() {
                if !protocol.is_enabled {
                    continue;
                }
                valid_protocols.push(protocol.char_code.to_ascii_uppercase());
                if protocol.char_code == cur_protocol {
                    protocols.push(format!("=> ({}) {}", protocol.char_code, protocol.description));
                } else {
                    protocols.push(format!("   ({}) {}", protocol.char_code, protocol.description));
                }
            }
        }
        self.state.set_color(IcbColor::Dos(11))?;
        for line in protocols {
            self.state.print(TerminalTarget::Both, &line)?;
            self.state.new_line()?;
        }

        let protocol = self.state.input_field(
            IceText::DesiredProtocol,
            1,
            &valid_protocols,
            "",
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::UPCASE | display_flags::FIELDLEN,
        )?;

        if !protocol.is_empty() {
            let selected_protocol = protocol.chars().next().unwrap_or(' ').to_ascii_uppercase();
            if let Some(user) = &mut self.state.current_user {
                user.protocol = selected_protocol;
            }
            self.state.display_text(IceText::DefaultProtocol, display_flags::DEFAULT)?;
            let txt = if let Ok(board) = self.state.board.lock() {
                let mut res = String::new();
                for protocol in board.protocols.iter() {
                    if protocol.char_code == selected_protocol {
                        res.clone_from(&protocol.description);
                        break;
                    }
                }
                res
            } else {
                "Error".to_string()
            };
            self.state.set_color(IcbColor::Dos(11))?;
            self.state.print(TerminalTarget::Both, &txt)?;
            self.state.new_line()?;
            self.state.new_line()?;
        }
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }
}
