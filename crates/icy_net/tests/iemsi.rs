#![allow(clippy::field_reassign_with_default)]

use icy_net::{
    iemsi::{encode_ici, ICIRequests, ICITerminalSettings, ICIUserSettings},
    telnet::{TermCaps, TerminalEmulation},
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
    assert_eq!("**EMSI_ICI0089{fooboar}{foo}{Unit test}{-Unpublished-}{-Unpublished-}{bar}{}{ANSI,24,80,0}{ZAP,ZMO,KER}{CHT,TAB,ASCII8}{HOT,MORE,FSED,NEWS,CLR}{Rust}{}29535C6F\r**EMSI_ACKA490\r**EMSI_ACKA490\r", std::str::from_utf8(&result).unwrap());
}

#[test]
fn test_emsi_ici_decoding() {
    let package = b"**EMSI_ICI0089{fooboar}{foo}{Unit test}{-Unpublished-}{-Unpublished-}{bar}{}{ANSI,24,80,0}{ZAP,ZMO,KER}{CHT,TAB,ASCII8}{HOT,MORE,FSED,NEWS,CLR}{Rust}{}29535C6F\r";
    let ici = icy_net::iemsi::decode_ici(package).unwrap();

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
