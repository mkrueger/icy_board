use icy_engine::{Size, TextPane, TextScreen};
use icy_parser_core::{CommandSink, TerminalCommand};

use crate::Res;

/// Keeps the viewport and text layers in step before commands following a resize.
pub fn parse_into_screen(parser: &mut dyn icy_parser_core::CommandParser, screen: &mut TextScreen, bytes: &[u8]) {
    let before = (screen.width(), screen.height());
    parser.parse(bytes, &mut TerminalScreenSink(screen));
    let after = (screen.width(), screen.height());
    if before != after {
        resize_screen(screen, Size::new(after.0, after.1));
    }
}

pub(super) fn resize_screen(screen: &mut TextScreen, size: Size) {
    screen.buffer.set_size(size);
    screen.buffer.terminal_state.set_size(size);
    for layer in &mut screen.buffer.layers {
        layer.set_size(size);
    }
}

struct TerminalScreenSink<'a>(&'a mut TextScreen);

impl CommandSink for TerminalScreenSink<'_> {
    fn print(&mut self, text: &[u8]) {
        icy_engine::ScreenSink::new(self.0).print(text);
    }

    fn emit(&mut self, command: TerminalCommand) {
        let before = (self.0.width(), self.0.height());
        icy_engine::ScreenSink::new(self.0).emit(command);
        let after = (self.0.width(), self.0.height());
        if before != after {
            resize_screen(self.0, Size::new(after.0, after.1));
        }
    }

    fn emit_rip(&mut self, command: icy_parser_core::RipCommand) {
        icy_engine::ScreenSink::new(self.0).emit_rip(command);
    }

    fn emit_skypix(&mut self, command: icy_parser_core::SkypixCommand) {
        icy_engine::ScreenSink::new(self.0).emit_skypix(command);
    }

    fn emit_igs(&mut self, command: icy_parser_core::IgsCommand) {
        icy_engine::ScreenSink::new(self.0).emit_igs(command);
    }

    fn emit_view_data(&mut self, command: icy_parser_core::ViewDataCommand) -> bool {
        icy_engine::ScreenSink::new(self.0).emit_view_data(command)
    }

    fn device_control(&mut self, command: icy_parser_core::DeviceControlString) {
        icy_engine::ScreenSink::new(self.0).device_control(command);
    }

    fn operating_system_command(&mut self, command: icy_parser_core::OperatingSystemCommand) {
        icy_engine::ScreenSink::new(self.0).operating_system_command(command);
    }

    fn aps(&mut self, data: &[u8]) {
        icy_engine::ScreenSink::new(self.0).aps(data);
    }

    fn play_music(&mut self, music: icy_parser_core::AnsiMusic) {
        icy_engine::ScreenSink::new(self.0).play_music(music);
    }

    fn request(&mut self, request: icy_parser_core::TerminalRequest) {
        icy_engine::ScreenSink::new(self.0).request(request);
    }

    fn report_error(&mut self, error: icy_parser_core::ParseError, level: icy_parser_core::ErrorLevel) {
        icy_engine::ScreenSink::new(self.0).report_error(error, level);
    }

    fn begin_igs_xor_mode(&mut self) {
        icy_engine::ScreenSink::new(self.0).begin_igs_xor_mode();
    }

    fn end_igs_xor_mode(&mut self) {
        icy_engine::ScreenSink::new(self.0).end_igs_xor_mode();
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
    fn a5_resize_clear_and_edge_output_are_independent_of_chunk_boundaries() {
        for (width, height) in [(132, 43), (80, 25)] {
            let data = format!(
                "\x1b[8;50;160t\x1b[8;{height};{width}t\x1b[2J\x1b[HGr\u{fc}sse\x1b[{};{}HX\x1b[1;1H",
                height - 1,
                width
            );
            for split in 0..=data.len() {
                let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
                screen.write_bytes(&data.as_bytes()[..split]);
                screen.write_bytes(&data.as_bytes()[split..]);
                assert_eq!(screen.buffer.buffer.terminal_state.size(), Size::new(width, height), "split {split}");
                assert_eq!((screen.buffer.width(), screen.buffer.height()), (width, height), "split {split}");
                assert_eq!(screen.buffer.char_at(Position::new(2, 0)).ch, '\u{fc}', "split {split}");
                assert_eq!(screen.buffer.char_at(Position::new(width - 1, height - 2)).ch, 'X', "split {split}");
                assert_eq!(screen.buffer.caret.position(), Position::default(), "split {split}");
            }
            let mut screen = VirtualScreen::new(icy_parser_core::AnsiParser::default());
            for byte in data.bytes() {
                screen.write_bytes(&[byte]);
            }
            assert_eq!(screen.buffer.buffer.terminal_state.size(), Size::new(width, height));
            assert_eq!(screen.buffer.char_at(Position::new(2, 0)).ch, '\u{fc}');
            assert_eq!(screen.buffer.char_at(Position::new(width - 1, height - 2)).ch, 'X');
        }
    }

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
