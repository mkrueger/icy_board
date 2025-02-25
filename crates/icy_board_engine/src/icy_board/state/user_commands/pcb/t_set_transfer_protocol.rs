use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{MASK_ALNUM, display_flags},
    },
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn set_transfer_protocol(&mut self) -> Res<()> {
        self.displaycmdfile("preprot").await?;
        if self.displaycmdfile("prot").await? {
            return Ok(());
        }
        let cur_protocol = if let Some(user) = &mut self.session.current_user {
            user.protocol.clone()
        } else {
            String::new()
        };

        let protocol = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.ask_protocols(&cur_protocol).await?
        };

        if !protocol.is_empty() {
            let selected_protocol: String = protocol.to_ascii_uppercase();

            let mut txt = String::new();
            for protocol in self.get_board().await.protocols.iter() {
                if &protocol.char_code == &selected_protocol {
                    txt.clone_from(&protocol.description);
                    break;
                }
            }
            if let Some(user) = &mut self.session.current_user {
                if user.protocol != selected_protocol {
                    user.protocol = selected_protocol;
                    self.display_text(IceText::DefaultProtocol, display_flags::LFBEFORE).await?;
                    self.set_color(TerminalTarget::Both, IcbColor::dos_cyan()).await?;
                    self.println(TerminalTarget::Both, &txt).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn ask_protocols(&mut self, cur_protocol: &str) -> Res<String> {
        let mut protocols = Vec::new();
        self.new_line().await?;
        for protocol in self.get_board().await.protocols.iter() {
            if !protocol.is_enabled {
                continue;
            }
            if protocol.char_code == cur_protocol {
                protocols.push(format!("=> ({}) {}", protocol.char_code, protocol.description));
            } else {
                protocols.push(format!("   ({}) {}", protocol.char_code, protocol.description));
            }
        }

        self.set_color(TerminalTarget::Both, IcbColor::dos_cyan()).await?;
        for line in protocols {
            self.print(TerminalTarget::Both, &line).await?;
            self.new_line().await?;
        }
        let protocol = self
            .input_field(
                IceText::DesiredProtocol,
                1,
                &MASK_ALNUM,
                "",
                Some(cur_protocol.to_string()),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;
        Ok(protocol)
    }
}
