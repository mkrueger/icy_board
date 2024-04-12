use icy_board_engine::{
    icy_board::{commands::Command, icb_config::IcbColor, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};
use icy_ppe::Res;

use super::PcbBoardCommand;

impl PcbBoardCommand {
    pub async fn set_transfer_protocol(&mut self, _action: &Command) -> Res<()> {
        self.displaycmdfile("preprot").await?;
        if self.displaycmdfile("prot").await? {
            return Ok(());
        }

        let mut protocols = Vec::new();
        let mut valid_protocols = String::new();
        let cur_protocol = if let Some(user) = &mut self.state.current_user { user.protocol } else { ' ' };
        self.state.new_line().await?;

        {
            let board = self.state.board.lock().unwrap();
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

        self.state.set_color(IcbColor::Dos(11)).await?;
        for line in protocols {
            self.state.print(TerminalTarget::Both, &line).await?;
            self.state.new_line().await?;
        }

        let protocol = self
            .state
            .input_field(
                IceText::DesiredProtocol,
                1,
                &valid_protocols,
                "",
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;

        if !protocol.is_empty() {
            let selected_protocol = protocol.chars().next().unwrap_or(' ').to_ascii_uppercase();
            if let Some(user) = &mut self.state.current_user {
                user.protocol = selected_protocol;
            }
            self.state.display_text(IceText::DefaultProtocol, display_flags::DEFAULT).await?;
            let mut res = String::new();
            {
                let board = self.state.board.lock().unwrap();
                for protocol in board.protocols.iter() {
                    if protocol.char_code == selected_protocol {
                        res.clone_from(&protocol.description);
                        break;
                    }
                }
            }

            self.state.set_color(IcbColor::Dos(11)).await?;
            self.state.print(TerminalTarget::Both, &res).await?;
            self.state.new_line().await?;
            self.state.new_line().await?;
        }
        self.state.press_enter().await;
        self.display_menu = true;
        Ok(())
    }
}
