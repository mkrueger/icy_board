use crate::tests::test_output;

#[test]
fn test_t_no_change() {
    let output = test_output("T\n\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mT\n\n\u{1b}[1;36m   (A) Ascii\n   (X) Xmodem/Checksum\n   (C) Xmodem/CRC\n   (O) 1K-Xmodem       (a.k.a. non-BATCH Ymodem)\n   (F) 1K-Xmodem/G     (a.k.a. non-BATCH Ymodem/G)\n   (Y) Ymodem BATCH\n   (G) Ymodem/G BATCH\n=> (Z) Zmodem (batch)\n   (8) Zmodem 8k (batch)\n   (N) None\n\n\u{1b}[32mDefault Protocol Desired (Enter)=no change? (\u{1b}[1C)\u{1b}[2D\u{1b}[0mZ\u{1b}[1D\n\n\u{1b}[1;32mPress (Enter) to continue? \u{1b}[0m"
    );
}

#[test]
fn test_t() {
    let output = test_output("T\nX\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mT\n\n\u{1b}[1;36m   (A) Ascii\n   (X) Xmodem/Checksum\n   (C) Xmodem/CRC\n   (O) 1K-Xmodem       (a.k.a. non-BATCH Ymodem)\n   (F) 1K-Xmodem/G     (a.k.a. non-BATCH Ymodem/G)\n   (Y) Ymodem BATCH\n   (G) Ymodem/G BATCH\n=> (Z) Zmodem (batch)\n   (8) Zmodem 8k (batch)\n   (N) None\n\n\u{1b}[32mDefault Protocol Desired (Enter)=no change? (\u{1b}[1C)\u{1b}[2D\u{1b}[0mZ\u{1b}[1D \u{1b}[1DX\n\n\u{1b}[1;32mDefault Protocol set to \u{1b}[36mXmodem/Checksum\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}

/// Erasing the offered default before the typed answer must not change what the caller
/// ends up seeing.
#[test]
fn test_t_shows_only_the_typed_protocol() {
    let output = test_output("T\nX\n".to_string(), |_| {});
    let line = crate::tests::cmd_file_lists::rendered_lines(&output)
        .into_iter()
        .find(|line| line.contains("Default Protocol Desired"))
        .expect("the protocol prompt is missing");
    assert!(line.contains("(X)"), "{line:?}");
    assert!(!line.contains("(Z)"), "the offered default was left on screen: {line:?}");
}

#[test]
fn test_t_token() {
    let output = test_output("T X\n".to_string(), |_| {});
    assert_eq!(
        output,
        "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mT X\n\n\u{1b}[1;32mDefault Protocol set to \u{1b}[36mXmodem/Checksum\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m"
    );
}

/// SETTINGS.C setprotocol drops an invalid stacked letter and asks with the menu instead.
#[test]
fn test_t_invalid_token_shows_the_menu() {
    let output = test_output("T 123\nX\n".to_string(), |_| {});
    let lines = crate::tests::cmd_file_lists::rendered_lines(&output);
    assert!(lines.iter().any(|line| line.contains("(X) Xmodem/Checksum")), "{lines:#?}");
    assert!(lines.iter().any(|line| line.contains("Default Protocol Desired")), "{lines:#?}");
    assert!(
        lines.iter().any(|line| line.trim_end() == "Default Protocol set to Xmodem/Checksum"),
        "{lines:#?}"
    );
}

#[test]
fn test_t_invalid_token_then_enter_keeps_the_protocol() {
    let output = test_output("T 1\n\n".to_string(), |_| {});
    let lines = crate::tests::cmd_file_lists::rendered_lines(&output);
    assert!(lines.iter().any(|line| line.contains("Default Protocol Desired")), "{lines:#?}");
    assert!(!lines.iter().any(|line| line.contains("Default Protocol set to")), "{lines:#?}");
}

/// The prompt only takes letters of installed protocols, like the Valid mask in SETTINGS.C.
#[test]
fn test_t_prompt_ignores_letters_without_a_protocol() {
    let output = test_output("T\n1X\n".to_string(), |_| {});
    let lines = crate::tests::cmd_file_lists::rendered_lines(&output);
    assert!(
        lines.iter().any(|line| line.trim_end() == "Default Protocol set to Xmodem/Checksum"),
        "{lines:#?}"
    );
}
