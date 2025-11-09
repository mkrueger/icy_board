use icy_engine::{BufferParser, TextScreen};

pub struct VirtualScreen {
    parser: Box<dyn BufferParser>,
    pub buffer: TextScreen,
}

impl VirtualScreen {
    pub fn new<T: BufferParser + 'static>(parser: T) -> Self {
        let mut buffer = TextScreen::new((80, 25));
        buffer.buffer.terminal_state.is_terminal_buffer = true;
        buffer.buffer.terminal_state.fixed_size = true;
        buffer.buffer.buffer_type = icy_engine::BufferType::Unicode;
        Self {
            parser: Box::new(parser),
            buffer,
        }
    }

    pub fn set_parser<T: BufferParser + 'static>(&mut self, parser: T) {
        self.parser = Box::new(parser);
    }

    pub fn print_char(&mut self, c: char) -> crate::Result<()> {
        self.parser.print_char(&mut self.buffer, c)?;
        Ok(())
    }
}
