use icy_board_engine::{
    icy_board::{
        commands::Command,
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{display_flags, MASK_ALNUM},
    },
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
        let cur_protocol = if let Some(user) = &mut self.state.current_user {
            user.protocol.clone()
        } else {
            String::new()
        };

        let protocol = self.ask_protocols(cur_protocol)?;
        if !protocol.is_empty() {
            let selected_protocol = protocol.to_ascii_uppercase();

            self.state.display_text(IceText::DefaultProtocol, display_flags::DEFAULT)?;
            let txt = if let Ok(board) = self.state.board.lock() {
                let mut res = String::new();
                for protocol in board.protocols.iter() {
                    if &protocol.char_code == &selected_protocol {
                        res.clone_from(&protocol.description);
                        break;
                    }
                }
                res
            } else {
                "Error".to_string()
            };
            if let Some(user) = &mut self.state.current_user {
                user.protocol = selected_protocol;
            }
            self.state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
            self.state.print(TerminalTarget::Both, &txt)?;
            self.state.new_line()?;
            self.state.new_line()?;
        }
        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    pub fn ask_protocols(&mut self, cur_protocol: String) -> Res<String> {
        let mut protocols = Vec::new();
        self.state.new_line()?;
        if let Ok(board) = self.state.board.lock() {
            for protocol in board.protocols.iter() {
                if !protocol.is_enabled {
                    continue;
                }
                if protocol.char_code == cur_protocol {
                    protocols.push(format!("=> ({}) {}", protocol.char_code, protocol.description));
                } else {
                    protocols.push(format!("   ({}) {}", protocol.char_code, protocol.description));
                }
            }

            if "N" == cur_protocol {
                protocols.push(format!("=> (N) {}", board.default_display_text.get_display_text(IceText::None).unwrap().text));
            } else {
                protocols.push(format!("   (N) {}", board.default_display_text.get_display_text(IceText::None).unwrap().text));
            }
        }

        self.state.set_color(TerminalTarget::Both, IcbColor::Dos(11))?;
        for line in protocols {
            self.state.print(TerminalTarget::Both, &line)?;
            self.state.new_line()?;
        }
        let protocol = self.state.input_field(
            IceText::DesiredProtocol,
            1,
            &MASK_ALNUM,
            "",
            Some(cur_protocol.to_string()),
            display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::UPCASE | display_flags::FIELDLEN,
        )?;
        Ok(protocol)
    }
}
