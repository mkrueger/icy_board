use crate::{icy_board::state::IcyBoardState, Res};
use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::functions::{display_flags, MASK_ALNUM},
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

        let protocol = self.ask_protocols(cur_protocol).await?;
        if !protocol.is_empty() {
            let selected_protocol = protocol.to_ascii_uppercase();

            self.display_text(IceText::DefaultProtocol, display_flags::DEFAULT).await?;
            let mut txt = String::new();
            for protocol in self.get_board().await.protocols.iter() {
                if &protocol.char_code == &selected_protocol {
                    txt.clone_from(&protocol.description);
                    break;
                }
            }
            if let Some(user) = &mut self.session.current_user {
                user.protocol = selected_protocol;
            }
            self.set_color(TerminalTarget::Both, IcbColor::Dos(11)).await?;
            self.print(TerminalTarget::Both, &txt).await?;
            self.new_line().await?;
            self.new_line().await?;
        }
        self.press_enter().await?;
        self.display_current_menu = true;
        Ok(())
    }

    pub async fn ask_protocols(&mut self, cur_protocol: String) -> Res<String> {
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

        if "N" == cur_protocol {
            protocols.push(format!(
                "=> (N) {}",
                self.get_board().await.default_display_text.get_display_text(IceText::None).unwrap().text
            ));
        } else {
            protocols.push(format!(
                "   (N) {}",
                self.get_board().await.default_display_text.get_display_text(IceText::None).unwrap().text
            ));
        }

        self.set_color(TerminalTarget::Both, IcbColor::Dos(11)).await?;
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
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;
        Ok(protocol)
    }
}
