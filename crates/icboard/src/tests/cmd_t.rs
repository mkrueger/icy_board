use crate::tests::test_output;

#[test]
fn test_t_no_change() {
    let output = test_output("T\n\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mT\n\n\u{1b}[1;36m   (A) Ascii\n   (X) Xmodem/Checksum\n   (C) Xmodem/CRC\n   (O) 1K-Xmodem       (a.k.a. non-BATCH Ymodem)\n   (F) 1K-Xmodem/G     (a.k.a. non-BATCH Ymodem/G)\n   (Y) Ymodem BATCH\n   (G) Ymodem/G BATCH\n=> (Z) Zmodem (batch)\n   (8) Zmodem 8k (batch)\n   (N) None\n\n\u{1b}[32mDefault Protocol Desired (Enter)=no change? (\u{1b}[1C)\u{1b}[2D\u{1b}[0mZ\u{1b}[1D\n\n\u{1b}[1;32mPress (Enter) to continue? \u{1b}[0m");
}

#[test]
fn test_t() {
    let output = test_output("T\nX\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mT\n\n\u{1b}[1;36m   (A) Ascii\n   (X) Xmodem/Checksum\n   (C) Xmodem/CRC\n   (O) 1K-Xmodem       (a.k.a. non-BATCH Ymodem)\n   (F) 1K-Xmodem/G     (a.k.a. non-BATCH Ymodem/G)\n   (Y) Ymodem BATCH\n   (G) Ymodem/G BATCH\n=> (Z) Zmodem (batch)\n   (8) Zmodem 8k (batch)\n   (N) None\n\n\u{1b}[32mDefault Protocol Desired (Enter)=no change? (\u{1b}[1C)\u{1b}[2D\u{1b}[0mZ\u{1b}[1DX\n\n\u{1b}[1;32mDefault Protocol set to \u{1b}[36mXmodem/Checksum\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}

#[test]
fn test_t_token() {
    let output = test_output("T X\n".to_string(), |_| {});
    assert_eq!(output, "\u{1b}[1;33m(\u{1b}[31m1000\u{1b}[33m min. left) Main Board Command? \u{1b}[0mT X\n\n\u{1b}[1;32mDefault Protocol set to \u{1b}[36mXmodem/Checksum\n\n\u{1b}[32mPress (Enter) to continue? \u{1b}[0m");
}
