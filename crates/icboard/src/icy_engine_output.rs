use icy_engine::{ansi, Buffer, BufferParser, Caret};

pub struct Screen {
    pub caret: Caret,
    pub buffer: Buffer,
}

impl Screen {
    pub fn new() -> Screen {
        let mut buffer = Buffer::new((80, 24));
        buffer.is_terminal_buffer = true;
        let caret = Caret::default();
        Screen { caret, buffer }
    }

    pub fn _print(&mut self, parser: &mut ansi::Parser, c: char) {
        let _ = parser.print_char(&mut self.buffer, 0, &mut self.caret, c);
    }
}
