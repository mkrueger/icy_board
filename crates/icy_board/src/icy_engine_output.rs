use std::{collections::VecDeque, sync::Arc};

use icy_engine::{ansi, Buffer, BufferParser, Caret};
use tokio::sync::Mutex;

pub struct Screen {
    pub caret: Caret,
    pub buffer: Buffer,
}

impl Clone for Screen {
    fn clone(&self) -> Self {
        Self {
            caret: self.caret.clone(),
            buffer: self.buffer.flat_clone(false),
        }
    }
}
impl Screen {
    pub fn new() -> Screen {
        let mut buffer = Buffer::new((80, 24));
        buffer.is_terminal_buffer = true;
        let caret = Caret::default();
        Screen { caret, buffer }
    }

    pub fn print(&mut self, parser: &mut ansi::Parser, c: char) {
        parser.print_char(&mut self.buffer, 0, &mut self.caret, c).unwrap();
    }
}
