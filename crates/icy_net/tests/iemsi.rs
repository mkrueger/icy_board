#![allow(clippy::field_reassign_with_default)]

use icy_net::{
    iemsi::{encode_ici, ICIRequests, ICITerminalSettings, ICIUserSettings},
    telnet::{TermCaps, Terminal},
};
use pretty_assertions::assert_eq;

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
            terminal: Terminal::Ansi,
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
    assert_eq!("**EMSI_ICI0089{fooboar}{foo}{Unit test}{-Unpublished-}{-Unpublished-}{bar}{}{ANSI,24,80,0}{ZAP,ZMO,KER}{CHT,TAB,ASCII8}{HOT,MORE,FSED,NEWS,CLR}{Rust}{}29535C6F\r**EMSI_ACKA490\r**EMSI_ACKA490\r", std::str::from_utf8(&result).unwrap());
}
