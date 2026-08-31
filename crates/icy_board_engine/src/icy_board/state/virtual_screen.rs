use icy_engine::{TextPane, TextScreen};

use crate::Res;

/// Runs the parser over `bytes` and brings the viewport along with any resize.
///
/// `CSI 8 ; h ; w t` resizes the buffer, but a terminal buffer keeps its viewport apart
/// from it and the parser only touches the buffer.
pub fn parse_into_screen(parser: &mut dyn icy_parser_core::CommandParser, screen: &mut TextScreen, bytes: &[u8]) {
    let before = (screen.width(), screen.height());
    parser.parse(bytes, &mut icy_engine::ScreenSink::new(screen));
    let after = (screen.width(), screen.height());
    if before != after {
        screen.buffer.terminal_state.set_size(icy_engine::Size::new(after.0, after.1));
    }
}

pub struct VirtualScreen {
    parser: Box<dyn icy_parser_core::CommandParser>,
    pub buffer: TextScreen,
}

impl VirtualScreen {
    pub fn new<T: icy_parser_core::CommandParser + 'static>(parser: T) -> Self {
        let mut buffer = TextScreen::new((80, 25));
        buffer.buffer.terminal_state.is_terminal_buffer = true;
        buffer.buffer.buffer_type = icy_engine::BufferType::Unicode;
        Self {
            parser: Box::new(parser),
            buffer,
        }
    }

    pub fn set_parser<T: icy_parser_core::CommandParser + 'static>(&mut self, parser: T) {
        self.parser = Box::new(parser);
    }

    pub fn print_char(&mut self, c: char) -> Res<()> {
        let mut utf8 = [0; 4];
        parse_into_screen(self.parser.as_mut(), &mut self.buffer, c.encode_utf8(&mut utf8).as_bytes());
        Ok(())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        parse_into_screen(self.parser.as_mut(), &mut self.buffer, bytes);
    }
}

#[cfg(test)]
mod tests {
    use icy_engine::{Position, TextPane};

    use super::*;

    #[test]
    fn unicode_box_drawing_is_not_truncated_to_its_low_byte() {
        let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());

        screen.print_char('═').unwrap();
        screen.print_char('\r').unwrap();

        assert_eq!(screen.buffer.char_at(Position::default()).ch, '═');
    }

    /// The sysop monitor gets the caller's screen as a serialized snapshot on attach,
    /// so the round trip has to survive the writer as well as the screen itself.
    #[test]
    fn a_snapshot_of_the_screen_keeps_its_box_drawing() {
        let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
        screen.print_char('═').unwrap();
        screen.print_char('\r').unwrap();

        let text = crate::icy_board::state::screen_to_pcboard_text(&screen.buffer.buffer).unwrap();

        assert!(text.contains('═'), "the snapshot lost the box drawing: {text:?}");
        assert!(!text.starts_with('\u{feff}'), "the snapshot leaks a BOM: {text:?}");
    }

    /// The parser resizes the buffer alone, so the viewport has to be carried along or
    /// nothing that reads it notices the new size.
    #[test]
    fn an_ansi_resize_carries_the_viewport_with_it() {
        let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());

        for c in "\x1b[8;43;132t".chars() {
            screen.print_char(c).unwrap();
        }

        assert_eq!(screen.buffer.buffer.terminal_state.size(), icy_engine::Size::new(132, 43));
        assert_eq!((screen.buffer.width(), screen.buffer.height()), (132, 43));
    }
}
