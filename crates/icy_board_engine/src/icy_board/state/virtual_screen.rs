use icy_engine::TextScreen;

use crate::Res;

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
        let mut sink = icy_engine::ScreenSink::new(&mut self.buffer);
        let mut utf8 = [0; 4];
        self.parser.parse(c.encode_utf8(&mut utf8).as_bytes(), &mut sink);
        Ok(())
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
}
