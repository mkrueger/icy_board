use std::{
    collections::VecDeque,
    fs,
    io::IsTerminal,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Args;
use color_eyre::{Result, eyre::eyre};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, terminal,
};
use icy_board_engine::icy_board::{doors::dos, lock::BoardLock};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    style::{Color, Style},
};
use x86::{Image, ImageKind, Machine, MachineConfig, NativeBackend, RunOptions};

#[derive(Args, PartialEq, Debug)]
pub struct DosConsole {
    #[arg(help = icy_board_cli::text("icbsetup", "dos-image-directory"))]
    directory: PathBuf,
    #[arg(help = icy_board_cli::text("icbsetup", "dos-console-door"))]
    door: String,
    #[arg(long, help = icy_board_cli::text("icbsetup", "dos-console-source"))]
    source: Option<PathBuf>,
    #[arg(long, default_value_t = 64, value_parser = clap::value_parser!(u32).range(1..=256), help = icy_board_cli::text("icbsetup", "dos-console-memory"))]
    memory: u32,
}

const STARTUP: &[u8] = b"@ECHO OFF\r\nSET DOSDIR=C:\\FREEDOS\r\nSET PATH=%DOSDIR%\\BIN\r\nCTTY CON\r\nCD C:\\DOOR\r\nC:\\FREEDOS\\BIN\\COMMAND.COM /E:1024\r\nC:\\ICB\\POWEROFF.COM\r\n";
const COLORS: [Color; 16] = [
    Color::Black,
    Color::Blue,
    Color::Green,
    Color::Cyan,
    Color::Red,
    Color::Magenta,
    Color::Yellow,
    Color::Gray,
    Color::DarkGray,
    Color::LightBlue,
    Color::LightGreen,
    Color::LightCyan,
    Color::LightRed,
    Color::LightMagenta,
    Color::LightYellow,
    Color::White,
];

pub fn run(command: &DosConsole) -> Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(eyre!(icy_board_cli::text("icbsetup", "dos-console-size")));
    }
    let (columns, rows) = terminal::size()?;
    if columns < 80 || rows < 25 {
        return Err(eyre!(icy_board_cli::text("icbsetup", "dos-console-size")));
    }
    let _lock = BoardLock::acquire(&command.directory).map_err(|error| eyre!(error.to_string()))?;
    let assets = command.directory.join("assets/dos");
    let image = assets.join("doors").join(dos::image_file_name(&command.door));
    if !image.is_file() && command.source.is_none() {
        return Err(eyre!("{}: {}", icy_board_cli::text("icbsetup", "dos-console-missing"), image.display()));
    }
    dos::prepare_dos_assets(&assets).map_err(|error| eyre!(error.to_string()))?;
    if !image.is_file() {
        dos::create_door_image(&assets.join("freedos.img"), &image, command.source.as_ref().unwrap()).map_err(|error| eyre!(error.to_string()))?;
    }
    let staged = tempfile::NamedTempFile::new_in(image.parent().unwrap())?;
    fs::copy(&image, staged.path())?;
    let startup = dos::replace_image_startup(staged.path(), STARTUP).map_err(|error| eyre!(error.to_string()))?;
    let mut machine = Machine::new(
        MachineConfig::default()
            .with_ram_bytes(command.memory as u64 * 1024 * 1024)
            .with_vga_memory_bytes(2 * 1024 * 1024),
    );
    machine.set_bios(Image::from_file(ImageKind::Bios, assets.join("seabios.bin"))?)?;
    machine.set_vga_bios(Image::from_file(ImageKind::VgaBios, assets.join("vgabios.bin"))?)?;
    machine.set_disk(Image::from_file(ImageKind::RawDisk, staged.path())?)?;
    machine.attach_backend(NativeBackend::new().with_instructions_per_step(10_000));
    x86::set_native_log_handler(|_| {});
    machine.prepare()?;
    let saved = interact(&mut machine)?;
    if saved {
        fs::write(staged.path(), machine.hard_disk_snapshot(0)?)?;
        dos::replace_image_startup(staged.path(), &startup).map_err(|error| eyre!(error.to_string()))?;
        staged.as_file().sync_all()?;
        let backup = tempfile::Builder::new()
            .prefix(&format!("{}.pre-console-", dos::image_file_name(&command.door)))
            .suffix(".bak")
            .tempfile_in(image.parent().unwrap())?;
        fs::copy(&image, backup.path())?;
        backup.as_file().sync_all()?;
        let (_, backup_path) = backup.keep()?;
        staged.persist(&image)?;
        println!("{}: {}", icy_board_cli::text("icbsetup", "dos-console-saved"), image.display());
        println!("{}: {}", icy_board_cli::text("icbsetup", "dos-fossil-backup"), backup_path.display());
    } else {
        println!("{}", icy_board_cli::text("icbsetup", "dos-console-discarded"));
    }
    Ok(())
}

struct ConsoleTerminal;

impl Drop for ConsoleTerminal {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            std::io::stdout(),
            crossterm::style::ResetColor,
            crossterm::cursor::Show,
            event::DisableBracketedPaste,
            terminal::LeaveAlternateScreen
        );
    }
}

fn interact(machine: &mut Machine) -> Result<bool> {
    terminal::enable_raw_mode()?;
    let _guard = ConsoleTerminal;
    execute!(std::io::stdout(), terminal::EnterAlternateScreen, event::EnableBracketedPaste)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    terminal.clear()?;
    let mut input = VecDeque::new();
    let mut last_key = Instant::now();
    let mut last_render = Instant::now();
    loop {
        while event::poll(Duration::from_millis(1))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::CONTROL {
                        return Ok(false);
                    }
                    if input.len() < 4096 {
                        input.push_back(key);
                    }
                }
                Event::Paste(text) => {
                    for character in text.replace("\r\n", "\n").chars().take(4096 - input.len()) {
                        input.push_back(KeyEvent::new(
                            if character == '\n' || character == '\r' {
                                KeyCode::Enter
                            } else {
                                KeyCode::Char(character)
                            },
                            KeyModifiers::NONE,
                        ));
                    }
                }
                _ => {}
            }
        }
        if last_key.elapsed() >= Duration::from_millis(10) {
            if let Some(key) = input.pop_front() {
                if let Some(scancodes) = key_scancodes(key) {
                    machine.inject_scancodes(&scancodes)?;
                } else if let KeyCode::Char(character) = key.code {
                    machine.inject_text(&character.to_string())?;
                }
            }
            last_key = Instant::now();
        }
        if machine
            .run(RunOptions {
                max_steps: Some(1),
                ..Default::default()
            })?
            .halted
        {
            return Ok(true);
        }
        if last_render.elapsed() >= Duration::from_millis(33) {
            if let Some((columns, rows, cells)) = machine.vga_text_snapshot() {
                terminal.draw(|frame| {
                    render_cells(frame.buffer_mut(), columns, rows, &cells);
                    if let Some((column, row)) = machine.vga_text_cursor() {
                        if column < frame.area().width as u32 && row < frame.area().height as u32 {
                            frame.set_cursor_position((column as u16, row as u16));
                        }
                    }
                })?;
            }
            last_render = Instant::now();
        }
    }
}

fn key_scancodes(key: KeyEvent) -> Option<Vec<u8>> {
    let (scan, extended) = match key.code {
        KeyCode::Esc => (0x01, false),
        KeyCode::Enter => (0x1C, false),
        KeyCode::Backspace => (0x0E, false),
        KeyCode::Tab | KeyCode::BackTab => (0x0F, false),
        KeyCode::Up => (0x48, true),
        KeyCode::Down => (0x50, true),
        KeyCode::Left => (0x4B, true),
        KeyCode::Right => (0x4D, true),
        KeyCode::Home => (0x47, true),
        KeyCode::End => (0x4F, true),
        KeyCode::PageUp => (0x49, true),
        KeyCode::PageDown => (0x51, true),
        KeyCode::Insert => (0x52, true),
        KeyCode::Delete => (0x53, true),
        KeyCode::F(number @ 1..=10) => (0x3A + number, false),
        KeyCode::F(11) => (0x57, false),
        KeyCode::F(12) => (0x58, false),
        KeyCode::Char(character) if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            let letters = b"abcdefghijklmnopqrstuvwxyz";
            let scans = [
                0x1E, 0x30, 0x2E, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31, 0x18, 0x19, 0x10, 0x13, 0x1F, 0x14, 0x16, 0x2F, 0x11, 0x2D,
                0x15, 0x2C,
            ];
            (scans[letters.iter().position(|byte| *byte as char == character.to_ascii_lowercase())?], false)
        }
        _ => return None,
    };
    let mut output = Vec::new();
    let shift = key.modifiers.contains(KeyModifiers::SHIFT) || key.code == KeyCode::BackTab;
    if shift {
        output.push(0x2A);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        output.push(0x1D);
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        output.push(0x38);
    }
    if extended {
        output.push(0xE0);
    }
    output.push(scan);
    if extended {
        output.push(0xE0);
    }
    output.push(scan | 0x80);
    if key.modifiers.contains(KeyModifiers::ALT) {
        output.push(0xB8);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        output.push(0x9D);
    }
    if shift {
        output.push(0xAA);
    }
    Some(output)
}

fn render_cells(buffer: &mut Buffer, columns: u32, rows: u32, cells: &[u8]) {
    let area = buffer.area;
    for row in 0..rows.min(area.height as u32) {
        for column in 0..columns.min(area.width as u32) {
            let offset = ((row as usize * columns as usize) + column as usize) * 2;
            let Some(cell) = cells.get(offset..offset + 2) else { return };
            let character = if cell[0] == 0 {
                ' '
            } else {
                codepages::tables::CP437_TO_UNICODE[cell[0] as usize]
            };
            buffer[(area.x + column as u16, area.y + row as u16)]
                .set_char(character)
                .set_style(Style::default().fg(COLORS[(cell[1] & 15) as usize]).bg(COLORS[((cell[1] >> 4) & 7) as usize]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn dos_console_cli_validates_memory_and_required_arguments() {
        let parsed = icy_board_cli::try_parse_from::<crate::Cli, _, _>(["icbsetup", "dos-console", "board", "LORD", "--source", "game"]).unwrap();
        assert_eq!(
            parsed.command,
            Some(crate::Commands::DosConsole(DosConsole {
                directory: "board".into(),
                door: "LORD".into(),
                source: Some("game".into()),
                memory: 64,
            }))
        );
        for arguments in [
            vec!["icbsetup", "dos-console", "board"],
            vec!["icbsetup", "dos-console", "board", "LORD", "--memory", "0"],
            vec!["icbsetup", "dos-console", "board", "LORD", "--memory", "257"],
        ] {
            assert!(icy_board_cli::try_parse_from::<crate::Cli, _, _>(arguments).is_err());
        }
    }

    #[test]
    fn dos_console_navigation_keys_keep_make_and_break_codes() {
        assert_eq!(key_scancodes(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)), Some(vec![1, 0x81]));
        assert_eq!(
            key_scancodes(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            Some(vec![0xE0, 0x48, 0xE0, 0xC8])
        );
        assert_eq!(key_scancodes(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE)), Some(vec![0x44, 0xC4]));
        assert_eq!(
            key_scancodes(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(vec![0x1D, 0x2E, 0xAE, 0x9D])
        );
    }

    #[test]
    fn dos_console_renders_cp437_at_terminal_edges() {
        for (width, height) in [(80, 25), (132, 43), (40, 12)] {
            let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
            let mut cells = vec![0; 80 * 25 * 2];
            cells[..2].copy_from_slice(&[0xDA, 0x1F]);
            cells[3998..].copy_from_slice(&[b'Z', 0x4E]);
            render_cells(&mut buffer, 80, 25, &cells);
            assert_eq!(buffer[(0, 0)].symbol(), "\u{250c}");
            assert_eq!(buffer[(0, 0)].fg, Color::White);
            assert_eq!(buffer[(0, 0)].bg, Color::Blue);
            if width >= 80 && height >= 25 {
                assert_eq!(buffer[(79, 24)].symbol(), "Z");
            }
        }
    }
}
