use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use icy_board_engine::vm::BoardIO;
use icy_engine::{ansi, Buffer, BufferParser, Caret};
use icy_ppe::Res;

pub struct Screen {
    pub caret: Caret,
    pub buffer: Buffer,
}
impl Screen {
    pub fn new() -> Screen {
        let buffer = Buffer::new((80, 24));
        let caret = Caret::default();
        Screen { caret, buffer }
    }

    pub fn print(&mut self, parser: &mut ansi::Parser, c: char) {
        parser
            .print_char(&mut self.buffer, 0, &mut self.caret, c)
            .unwrap();
    }
}

pub struct IcyEngineOutput {
    input_buffer: Arc<Mutex<VecDeque<char>>>,
    parser: ansi::Parser,
    screen: Arc<Mutex<Screen>>,
}

impl IcyEngineOutput {
    pub fn new(screen: Arc<Mutex<Screen>>, input_buffer: Arc<Mutex<VecDeque<char>>>) -> Self {
        Self {
            input_buffer,
            parser: ansi::Parser::default(),
            screen,
        }
    }
}

impl BoardIO for IcyEngineOutput {
    fn write_raw(&mut self, data: &[char]) -> Res<()> {
        if let Ok(scr) = self.screen.lock().as_mut() {
            for c in data {
                scr.print(&mut self.parser, *c);
            }
        }
        Ok(())
    }

    fn read(&mut self) -> Res<String> {
        let mut result = String::new();
        loop {
            let Ok(Some(ch)) = self.get_char() else {
                continue;
            };
            if ch == '\r' || ch == '\n' {
                break;
            }
            result.push(ch);
        }
        Ok(result)
    }

    fn get_char(&mut self) -> Res<Option<char>> {
        if let Some(c) = self.input_buffer.lock().unwrap().pop_front() {
            Ok(Some(c))
        } else {
            Ok(None)
        }
    }

    fn inbytes(&mut self) -> i32 {
        self.input_buffer.lock().unwrap().len() as i32
    }

    fn hangup(&mut self) -> Res<()> {
        Ok(())
    }
}
