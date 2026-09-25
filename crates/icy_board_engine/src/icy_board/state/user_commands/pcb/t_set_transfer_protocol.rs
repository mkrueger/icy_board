use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{icb_config::IcbColor, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    /// An invalid stacked letter falls through to the menu and
    /// the prompt, which keeps asking until it gets a valid letter or no change.
    pub async fn set_transfer_protocol(&mut self) -> Res<()> {
        let cur_protocol = if let Some(user) = &self.session.current_user {
            user.protocol.clone()
        } else {
            String::new()
        };

        let mut token = self.session.tokens.pop_front();
        loop {
            let selected = if let Some(token) = token.take() {
                token.chars().next().map(|ch| ch.to_ascii_uppercase().to_string()).unwrap_or_default()
            } else {
                let answer = self.ask_protocols(&cur_protocol).await?.to_ascii_uppercase();
                if answer.is_empty() || answer == cur_protocol || self.session.is_logoff_forced() {
                    return Ok(());
                }
                answer
            };
            let Some(description) = self.protocol_description(&selected).await else {
                continue;
            };
            if let Some(user) = &mut self.session.current_user {
                user.protocol = selected;
            }
            self.display_text(IceText::DefaultProtocol, display_flags::LFBEFORE).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
            self.println(TerminalTarget::Both, &description).await?;
            return Ok(());
        }
    }

    async fn protocol_description(&self, code: &str) -> Option<String> {
        self.get_board()
            .await
            .protocols
            .iter()
            .find(|protocol| protocol.is_enabled && !code.is_empty() && protocol.char_code.eq_ignore_ascii_case(code))
            .map(|protocol| protocol.description.clone())
    }

    pub async fn ask_protocols(&mut self, cur_protocol: &str) -> Res<String> {
        self.ask_protocol_with(cur_protocol, IceText::DesiredProtocol).await
    }

    /// `PCBoard` asks this one when a transfer is about to start and the caller has
    /// no usable protocol set, and it uses its own prompt rather than the T command's.
    pub async fn ask_transfer_protocol(&mut self, cur_protocol: &str) -> Res<String> {
        self.ask_protocol_with(cur_protocol, IceText::ProtocolForTransfer).await
    }

    async fn ask_protocol_with(&mut self, cur_protocol: &str, prompt: IceText) -> Res<String> {
        // Stuffed answers skip the prompt in input_field, so the menu goes with it.
        if !self.has_hidden_typeahead() {
            self.print_protocol_list(cur_protocol).await?;
        }
        let valid: String = self
            .get_board()
            .await
            .protocols
            .iter()
            .filter(|protocol| protocol.is_enabled)
            .flat_map(|protocol| protocol.char_code.to_ascii_uppercase().chars().collect::<Vec<_>>())
            .collect();
        let protocol = self
            .input_field(
                prompt,
                1,
                &valid,
                "",
                Some(cur_protocol.to_string()),
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;
        Ok(protocol)
    }

    /// PREPROT comes first, and a PROT file replaces the built-in list.
    async fn print_protocol_list(&mut self, cur_protocol: &str) -> Res<()> {
        self.new_line().await?;
        self.displaycmdfile("preprot").await?;
        if self.displaycmdfile("prot").await? {
            return Ok(());
        }
        let mut protocols = Vec::new();
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

        self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        for line in protocols {
            self.print(TerminalTarget::Both, &line).await?;
            self.new_line().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
    use tokio::sync::Mutex;

    use crate::icy_board::{IcyBoard, bbs::BBS, icb_text::DEFAULT_DISPLAY_TEXT, state::IcyBoardState, xfer_protocols::SupportedProtocols};

    async fn state() -> (IcyBoardState, ChannelConnection) {
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
        board.protocols = SupportedProtocols::generate_pcboard_defaults();
        let state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
        (state, peer)
    }

    async fn output(peer: &mut ChannelConnection) -> String {
        let mut output = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let size = peer.try_read(&mut buffer).await.unwrap();
            if size == 0 {
                return String::from_utf8_lossy(&output).into_owned();
            }
            output.extend_from_slice(&buffer[..size]);
        }
    }

    #[tokio::test]
    async fn stuffed_protocol_answer_skips_the_protocol_menu() {
        let (mut state, mut peer) = state().await;
        state.stuff_keyboard_buffer("x\r", false).unwrap();
        assert_eq!(state.ask_protocols("N").await.unwrap(), "X");
        let shown = output(&mut peer).await;
        assert!(!shown.contains("Xmodem"), "{shown:?}");
        assert!(!shown.contains("Protocol"), "{shown:?}");
    }

    #[tokio::test]
    async fn visible_stuffed_protocol_answer_keeps_the_protocol_menu() {
        let (mut state, mut peer) = state().await;
        state.stuff_keyboard_buffer("x\r", true).unwrap();
        assert_eq!(state.ask_protocols("N").await.unwrap(), "X");
        let shown = output(&mut peer).await;
        assert!(shown.contains("=> (N) None"), "{shown:?}");
        assert!(shown.contains("Default Protocol Desired"), "{shown:?}");
    }

    #[tokio::test]
    async fn stuffed_date_format_answer_skips_the_format_menu() {
        let (mut state, mut peer) = state().await;
        let formats = state.get_board().await.languages.date_formats.clone();
        assert!(formats.len() > 1);
        state.stuff_keyboard_buffer("2\r", false).unwrap();
        assert_eq!(state.ask_date_format(&formats[0].1).await.unwrap(), formats[1].1);
        let shown = output(&mut peer).await;
        assert!(!shown.contains(&formats[1].0), "{shown:?}");

        state.stuff_keyboard_buffer("2\r", true).unwrap();
        assert_eq!(state.ask_date_format(&formats[0].1).await.unwrap(), formats[1].1);
        let shown = output(&mut peer).await;
        assert!(shown.contains(&format!("=> (1) {}", formats[0].0)), "{shown:?}");
    }
}
