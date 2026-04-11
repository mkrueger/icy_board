#![allow(clippy::field_reassign_with_default)]

use async_trait::async_trait;
use icy_net::{
    Connection, ConnectionType,
    iemsi::{ICIRequests, ICITerminalSettings, ICIUserSettings, decode_ici, encode_ici, try_iemsi},
    telnet::{TermCaps, TerminalEmulation},
};
use pretty_assertions::assert_eq;
use std::collections::VecDeque;

struct ScriptedConnection {
    chunks: VecDeque<Vec<u8>>,
}

impl ScriptedConnection {
    fn new(chunks: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            chunks: chunks.into_iter().collect(),
        }
    }

    fn drain_chunk(&mut self, buf: &mut [u8]) -> usize {
        let Some(chunk) = self.chunks.front_mut() else {
            return 0;
        };

        let len = buf.len().min(chunk.len());
        buf[..len].copy_from_slice(&chunk[..len]);
        if len == chunk.len() {
            self.chunks.pop_front();
        } else {
            chunk.drain(..len);
        }
        len
    }
}

#[async_trait]
impl Connection for ScriptedConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Websocket
    }

    async fn read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        Ok(self.drain_chunk(buf))
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> icy_net::Result<usize> {
        Ok(self.drain_chunk(buf))
    }

    async fn send(&mut self, _buf: &[u8]) -> icy_net::Result<()> {
        Ok(())
    }
}

#[test]
fn test_emsi_ici_encoding() {
    let ici = ICIUserSettings {
        name: "fooboar".to_string(),
        alias: "foo".to_string(),
        location: "Unit test".to_string(),
        data_phone: "-Unpublished-".to_string(),
        voice_phone: "-Unpublished-".to_string(),
        password: "bar".to_string(),
        birth_date: String::new(),
    };
    let term = ICITerminalSettings {
        term_caps: TermCaps {
            window_size: (80, 24),
            terminal: TerminalEmulation::Ansi,
        },
        protocols: "ZAP,ZMO,KER".to_string(),
        can_chat: true,
        can_download_ascii: false,
        can_tab_char: true,
        can_ascii8: true,
        software: "Rust".to_string(),
        xlattabl: String::new(),
    };
    let requests = ICIRequests::default();

    let result = encode_ici(&ici, &term, &requests).unwrap();
    assert_eq!(
        "**EMSI_ICI0089{fooboar}{foo}{Unit test}{-Unpublished-}{-Unpublished-}{bar}{}{ANSI,24,80,0}{ZAP,ZMO,KER}{CHT,TAB,ASCII8}{HOT,MORE,FSED,NEWS,CLR}{Rust}{}29535C6F\r",
        std::str::from_utf8(&result).unwrap()
    );
}

#[test]
fn test_emsi_ici_decoding() {
    let package = b"**EMSI_ICI0089{fooboar}{foo}{Unit test}{-Unpublished-}{-Unpublished-}{bar}{}{ANSI,24,80,0}{ZAP,ZMO,KER}{CHT,TAB,ASCII8}{HOT,MORE,FSED,NEWS,CLR}{Rust}{}29535C6F\r";
    let ici = decode_ici(package).unwrap();

    assert_eq!(ici.user.name, "fooboar");
    assert_eq!(ici.user.alias, "foo");
    assert_eq!(ici.user.location, "Unit test");
    assert_eq!(ici.user.data_phone, "-Unpublished-");
    assert_eq!(ici.user.voice_phone, "-Unpublished-");
    assert_eq!(ici.user.password, "bar");
    assert_eq!(ici.term.term_caps.window_size, (80, 24));
    assert_eq!(ici.term.term_caps.terminal, TerminalEmulation::Ansi);
    assert_eq!(ici.term.protocols, "ZAP,ZMO,KER");
    assert_eq!(ici.term.can_chat, true);
    assert_eq!(ici.term.can_download_ascii, false);
    assert_eq!(ici.term.can_tab_char, true);
    assert_eq!(ici.term.can_ascii8, true);
    assert_eq!(ici.term.software, "Rust");
    assert_eq!(ici.requests.hot_keys, true);
    assert_eq!(ici.requests.more, true);
    assert_eq!(ici.requests.full_screen_editor, true);
    assert_eq!(ici.requests.news, true);
    assert_eq!(ici.requests.clear_screen, true);
}

#[test]
fn test_emsi_ici_decoding_rejects_truncated_packets() {
    assert!(decode_ici(b"**EMSI_ICI0001").is_err());
}

#[tokio::test]
async fn test_try_iemsi_handles_prefixed_and_fragmented_ici_packets() {
    let ici = ICIUserSettings {
        name: "fooboar".to_string(),
        alias: "foo".to_string(),
        location: "Unit test".to_string(),
        data_phone: "-Unpublished-".to_string(),
        voice_phone: "-Unpublished-".to_string(),
        password: "bar".to_string(),
        birth_date: String::new(),
    };
    let term = ICITerminalSettings {
        term_caps: TermCaps {
            window_size: (80, 24),
            terminal: TerminalEmulation::Ansi,
        },
        protocols: "ZAP,ZMO,KER".to_string(),
        can_chat: true,
        can_download_ascii: false,
        can_tab_char: true,
        can_ascii8: true,
        software: "Rust".to_string(),
        xlattabl: String::new(),
    };
    let encoded = encode_ici(&ici, &term, &ICIRequests::default()).unwrap();

    let split_at = 12;
    let scripted = ScriptedConnection::new([b"\x1B[25;80R".to_vec(), encoded[..split_at].to_vec(), encoded[split_at..].to_vec()]);
    let mut connection: Box<dyn Connection> = Box::new(scripted);

    let parsed = try_iemsi(
        &mut connection,
        "IcyBoard".to_string(),
        "Somewhere".to_string(),
        "Sysop".to_string(),
        "Notice".to_string(),
        "HOT".to_string(),
    )
    .await
    .unwrap()
    .expect("IEMSI ICI packet should be accepted");

    assert_eq!(parsed.user.name, "fooboar");
    assert_eq!(parsed.user.alias, "foo");
}
