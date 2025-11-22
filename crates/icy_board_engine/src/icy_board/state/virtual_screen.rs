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
        self.parser.parse(&[c as u8], &mut sink);
        Ok(())
    }
}
