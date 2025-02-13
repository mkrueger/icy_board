use icy_engine::{Buffer, BufferParser, Caret};

pub struct VirtualScreen {
    parser: Box<dyn BufferParser>,
    pub caret: Caret,
    pub buffer: Buffer,
}

impl VirtualScreen {
    pub fn new<T: BufferParser + 'static>(parser: T) -> Self {
        let mut buffer = Buffer::new((80, 25));
        buffer.is_terminal_buffer = true;
        buffer.terminal_state.fixed_size = true;
        buffer.buffer_type = icy_engine::BufferType::Unicode;
        let caret = Caret::default();
        Self {
            parser: Box::new(parser),
            caret,
            buffer,
        }
    }

    pub fn set_parser<T: BufferParser + 'static>(&mut self, parser: T) {
        self.parser = Box::new(parser);
    }

    pub fn print_char(&mut self, c: char) -> crate::Result<()> {
        self.parser.print_char(&mut self.buffer, 0, &mut self.caret, c)?;
        Ok(())
    }
}
