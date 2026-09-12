use super::{run_ppl, run_ppl_at_boundary};
use crate::icy_board::state::virtual_screen::VirtualScreen;
use icy_engine::{IceMode, Position, Size, TextPane};

#[test]
fn a_url_macro_wraps_its_label_in_osc_8() {
    let output = run_ppl(r#"PRINT "@URL:IcyBoard docs(https://example.com/docs)@""#);

    assert_eq!(output, "\x1b]8;;https://example.com/docs\x1b\\IcyBoard docs\x1b]8;;\x1b\\");
}

#[test]
fn a_url_followed_immediately_by_a_color_macro_closes_both_cleanly() {
    let output = run_ppl(
        r#"PRINTLN "@X08Map: @URL:OpenStreetMap(https://www.openstreetmap.org/copyright)@ / @URL:OpenTopoMap(https://opentopomap.org/credits)@@X07"
           PRINT "@X0Fnext""#,
    );

    assert_eq!(
        output,
        "\x1b[1;30mMap: \x1b]8;;https://www.openstreetmap.org/copyright\x1b\\OpenStreetMap\x1b]8;;\x1b\\ / \
         \x1b]8;;https://opentopomap.org/credits\x1b\\OpenTopoMap\x1b]8;;\x1b\\\x1b[0m\n\x1b[1;37mnext"
    );

    let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
    screen.write_bytes(output.replace('\n', "\r\n").as_bytes());
    assert_eq!(screen.buffer.size(), Size::new(80, 25));
    for (row, text, color) in [(0, "Map: OpenStreetMap / OpenTopoMap", 0x08), (1, "next", 0x0F)] {
        for (column, character) in text.chars().enumerate() {
            let position = Position::new(column as i32, row);
            let cell = screen.buffer.char_at(position);
            assert_eq!(cell.ch, character, "{position:?}");
            assert_eq!(cell.attribute.as_u8(IceMode::Blink), color, "{position:?}");
        }
    }
}

#[test]
fn a_ppe_ending_with_a_colored_url_restores_the_caller_color() {
    let output = run_ppl_at_boundary(
        r#"
        PRINTLN "@X08@URL:Foo(https://bar)@"
        EXIT
        "#,
    );

    assert!(output.ends_with("\x1b[0;1;37;44m"), "caller color was not restored: {output:?}");
}

/// A macro that never closes reaches the caller as the text it was written as.
#[test]
fn an_unclosed_url_macro_prints_itself() {
    let output = run_ppl(r#"PRINT "@URL:oops no closing at""#);

    assert_eq!(output, "@URL:oops no closing at");
}
