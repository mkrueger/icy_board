use std::{path::Path, sync::Arc, time::Duration};

use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::BBS,
    commands::{CommandList, CommandType},
    state::{GraphicsMode, IcyBoardState},
    user_base::User,
};
use icy_board_engine::vm::TerminalTarget;
use icy_board_help::{Encoding, HelpTheme, RenderOptions, catalog, render};
use icy_engine::{BufferType, IceMode, TextPane, TextScreen};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use icy_parser_core::{AnsiParser, CommandParser};

use super::{setup_conference, setup_conference_with_messages, test_output};

async fn display_state(root: &Path, mode: GraphicsMode) -> (IcyBoardState, ChannelConnection) {
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let mut board = IcyBoard::new();
    board.root_path = root.to_path_buf();
    board.config.paths.help_path = root.to_path_buf();
    board.commands = CommandList::new();
    let user = User {
        name: "EXPANDED USER SENTINEL".to_string(),
        page_len: 0,
        ..Default::default()
    };
    board.users.new_user(user.clone());
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (connection, peer) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(peer)).await;
    state.session.current_user = Some(user.clone());
    state.session.user_name = user.name;
    state.session.page_len = 0;
    state.session.term_caps.is_utf8 = true;
    state.set_grapics_mode(mode).await;
    (state, connection)
}

async fn drain(connection: &mut ChannelConnection) -> String {
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let size = connection.try_read(&mut buffer).await.unwrap();
        if size == 0 {
            return String::from_utf8(output).expect("UTF-8 terminal output");
        }
        output.extend_from_slice(&buffer[..size]);
    }
}

async fn display(state: &mut IcyBoardState, connection: &mut ChannelConnection, path: &Path) -> String {
    assert!(
        tokio::time::timeout(Duration::from_secs(5), state.display_file(&path))
            .await
            .expect("display must not wait for input")
            .unwrap()
    );
    drain(connection).await
}

fn screen(output: &str, width: i32) -> TextScreen {
    let mut screen = TextScreen::new((width, 25));
    screen.buffer.buffer_type = BufferType::Unicode;
    screen.buffer.terminal_state.is_terminal_buffer = true;
    // VT terminals defer last-column wrapping until another printable character.
    screen.buffer.terminal_state.last_column_flag_mode = true;
    AnsiParser::default().parse(output.as_bytes(), &mut icy_engine::ScreenSink::new(&mut screen));
    screen
}

fn row(screen: &TextScreen, y: i32, width: i32) -> String {
    (0..width).map(|x| screen.char_at((x, y).into()).ch).collect::<String>().trim_end().to_string()
}

fn assert_cells(screen: &TextScreen, y: i32, expected: &str, attr: u8) {
    for (x, ch) in expected.chars().enumerate() {
        let cell = screen.char_at((x as i32, y).into());
        assert_eq!(cell.ch, ch, "character at {x},{y}");
        assert_eq!(cell.attribute.as_u8(IceMode::Blink), attr, "attribute at {x},{y}");
    }
}

#[tokio::test]
async fn literal_macros_and_existing_control_files_survive_real_display() {
    let directory = tempfile::tempdir().unwrap();
    for name in ["include", "menu", "example.ppe"] {
        std::fs::write(directory.path().join(name), "UNEXPECTED FILE EXECUTION\r\n").unwrap();
    }
    let markdown = "# Literal help\n\n@USER@ @X07 @@ @CLS@\n\n```text\n!example.ppe\n$menu\n%include\n```\n\nEND OF HELP\n";
    for mode in [GraphicsMode::Graphics, GraphicsMode::Ctty] {
        let (mut state, mut connection) = display_state(directory.path(), mode).await;
        // Prove the include fixture and macro are live, not merely nonexistent names.
        state.display_line("%include").await.unwrap();
        assert!(drain(&mut connection).await.contains("UNEXPECTED FILE EXECUTION"));
        state.write_raw(TerminalTarget::User, &"@USER@".chars().collect::<Vec<_>>()).await.unwrap();
        assert!(drain(&mut connection).await.contains("EXPANDED"));

        let options = RenderOptions {
            theme: HelpTheme {
                body: 0x17,
                code: 0x1A,
                decoration: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let generated = render(markdown, &options).unwrap();
        assert!(generated.bytes.windows(5).any(|bytes| bytes == b"@@XFF"));
        let path = directory.path().join("literal.pcb");
        std::fs::write(&path, generated.bytes).unwrap();
        let output = display(&mut state, &mut connection, &path).await;
        assert!(!output.contains("UNEXPECTED FILE EXECUTION"), "{output:?}");
        assert!(!output.contains("EXPANDED"), "{output:?}");
        assert!(!output.contains("@@XFF"), "{output:?}");
        if mode == GraphicsMode::Ctty {
            assert!(!output.contains('\x1b'), "{output:?}");
            assert_eq!(output, format!("\x0C{}", generated.plain_text.replace('\n', "\r\n")));
        } else {
            let screen = screen(&output, 80);
            let expected = [
                "Literal help",
                "",
                "@USER@ @X07 @@ @CLS@",
                "",
                "!example.ppe",
                "$menu",
                "%include",
                "",
                "END OF HELP",
            ];
            for (y, text) in expected.iter().enumerate() {
                assert_eq!(row(&screen, y as i32, 80), *text, "{output:?}");
            }
            assert_cells(&screen, 2, expected[2], 0x17);
            for y in 4..=6 {
                assert_cells(&screen, y, expected[y as usize], 0x1A);
            }
        }
    }
}

#[tokio::test]
async fn generated_frames_wrap_at_79_and_40_columns_and_reset_color() {
    let directory = tempfile::tempdir().unwrap();
    for width in [79, 40] {
        let (mut state, mut connection) = display_state(directory.path(), GraphicsMode::Graphics).await;
        let markdown = format!("# Frame\n\n{}\n\n`@X07`\n", "word ".repeat(20));
        let options = RenderOptions { width, ..Default::default() };
        let path = directory.path().join("frame.pcb");
        std::fs::write(&path, render(&markdown, &options).unwrap().bytes).unwrap();
        let mut output = display(&mut state, &mut connection, &path).await;
        state.write_raw(TerminalTarget::User, &"RESET".chars().collect::<Vec<_>>()).await.unwrap();
        output.push_str(&drain(&mut connection).await);
        let screen_width = if width == 79 { 80 } else { 40 };
        let screen = screen(&output, screen_width);
        assert_cells(&screen, 0, "Frame", options.theme.title);
        assert_cells(&screen, 1, &"=".repeat(width), options.theme.border);
        assert_eq!(row(&screen, 2, screen_width), "");
        let words_per_line = if width == 79 { 16 } else { 8 };
        let mut left = 20;
        let mut y = 3;
        while left > 0 {
            let count = left.min(words_per_line);
            let expected = vec!["word"; count].join(" ");
            assert_eq!(row(&screen, y, screen_width), expected);
            assert_cells(&screen, y, &expected, options.theme.body);
            left -= count;
            y += 1;
        }
        assert_eq!(row(&screen, y, screen_width), "");
        assert_cells(&screen, y + 1, "@X07", options.theme.code);
        assert_eq!(row(&screen, y + 2, screen_width), "RESET");
        assert_cells(&screen, y + 2, "RESET", 0x07);
    }
}

#[tokio::test]
async fn german_cp437_and_bom_unicode_reach_terminal_cells() {
    let directory = tempfile::tempdir().unwrap();
    for encoding in [Encoding::Cp437, Encoding::Utf8] {
        let (mut state, mut connection) = display_state(directory.path(), GraphicsMode::Graphics).await;
        let text = if encoding == Encoding::Cp437 {
            "Grüße: ÄÖÜ äöü ß"
        } else {
            "Grüße: € Ł Ž"
        };
        let options = RenderOptions {
            encoding,
            theme: HelpTheme::preset("minimal").unwrap(),
            ..Default::default()
        };
        let generated = render(&format!("# Hilfe\n\n{text}\n"), &options).unwrap();
        assert_eq!(generated.bytes.starts_with(&[0xEF, 0xBB, 0xBF]), encoding == Encoding::Utf8);
        if encoding == Encoding::Cp437 {
            assert!(generated.bytes.contains(&0x81), "ü must be encoded as CP437");
        }
        let path = directory.path().join("german.pcb");
        std::fs::write(&path, generated.bytes).unwrap();
        let output = display(&mut state, &mut connection, &path).await;
        assert!(output.contains(text), "{output:?}");
        assert!(!output.contains('\u{FEFF}'));
        let screen = screen(&output, 80);
        assert_eq!(row(&screen, 2, 80), text);
        assert_cells(&screen, 2, text, options.theme.body);
    }
}

#[tokio::test]
async fn all_68_english_sources_display_without_expansion() {
    let directory = tempfile::tempdir().unwrap();
    let sources = catalog::sources(None).unwrap();
    assert_eq!(sources.len(), 68);
    let (mut state, mut connection) = display_state(directory.path(), GraphicsMode::Ctty).await;
    for source in sources {
        for encoding in [Encoding::Cp437, Encoding::Utf8] {
            let options = RenderOptions {
                encoding,
                theme: HelpTheme::preset("minimal").unwrap(),
                clear_screen: false,
                ..Default::default()
            };
            let generated = render(&source.markdown, &options).unwrap_or_else(|error| panic!("{}: {error}", source.topic));
            let path = directory.path().join(catalog::output_name(&source.topic, "").unwrap());
            std::fs::write(&path, generated.bytes).unwrap();
            let output = display(&mut state, &mut connection, &path).await;
            assert_eq!(output.replace("\r\n", "\n"), generated.plain_text, "{} {encoding:?}", source.topic);
        }
    }
}

#[tokio::test]
async fn all_help_bodies_reach_80_column_ansi_terminal_cells() {
    let directory = tempfile::tempdir().unwrap();
    let (mut state, mut connection) = display_state(directory.path(), GraphicsMode::Graphics).await;
    for source in catalog::sources(None).unwrap() {
        for encoding in [Encoding::Cp437, Encoding::Utf8] {
            let options = RenderOptions { encoding, ..Default::default() };
            let document = icy_board_help::document::compile(&source.markdown, options.width, &options.theme).unwrap();
            let generated = render(&source.markdown, &options).unwrap();
            let path = directory.path().join(catalog::output_name(&source.topic, "").unwrap());
            std::fs::write(&path, generated.bytes).unwrap();
            let output = display(&mut state, &mut connection, &path).await;
            let mut terminal = TextScreen::new((80, 25));
            terminal.buffer.buffer_type = BufferType::Unicode;
            terminal.buffer.terminal_state.is_terminal_buffer = true;
            terminal.buffer.terminal_state.last_column_flag_mode = true;
            let mut parser = AnsiParser::default();
            let mut lines = output.split_inclusive('\n');
            for (y, expected) in document.lines.iter().enumerate() {
                let line = lines.next().expect("every rendered row must reach the terminal");
                parser.parse(line.as_bytes(), &mut icy_engine::ScreenSink::new(&mut terminal));
                // Inspect each completed row before scrolling can remove it from an 80x25 screen.
                assert_eq!(row(&terminal, (y as i32).min(23), 80), expected.plain_text(), "{} {encoding:?} row {y}", source.topic);
            }
            let remaining = lines.collect::<String>();
            parser.parse(remaining.as_bytes(), &mut icy_engine::ScreenSink::new(&mut terminal));
            assert!(!remaining.contains('\n'), "unexpected extra output for {}", source.topic);
        }
    }
}

#[test]
fn generated_catalog_covers_command_and_context_help() {
    let sources = catalog::sources(None).unwrap();
    for topic in CommandType::iter().map(CommandType::get_help).filter(|topic| !topic.is_empty()).chain([
        "hlp!", "hlpcmenu", "hlpendr", "hlpfscrn", "hlpreg", "hlpsec", "hlpsrch",
    ]) {
        assert!(sources.iter().any(|source| source.topic == topic), "Missing runtime help: {topic}");
    }
    // These two prompts are rendered from ICBTEXT, not help-path display files.
    for virtual_topic in ["hlpmore", "hlpxfrmore"] {
        assert!(!sources.iter().any(|source| source.topic == virtual_topic));
    }
}

fn install_topic(root: &Path, topic: &str) {
    let source = catalog::sources(None).unwrap().into_iter().find(|source| source.topic == topic).unwrap();
    let bytes = render(&source.markdown, &RenderOptions::default()).unwrap().bytes;
    assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]), "generated help defaults to UTF-8 with a BOM");
    std::fs::write(root.join(catalog::output_name(topic, "").unwrap()), bytes).unwrap();
}

#[test]
fn h_dispatches_generated_letter_symbol_and_sysop_topics() {
    let directory = tempfile::tempdir().unwrap();
    for topic in ["hlpa", "hlp!", "hlp@", "hlpr", "hlp1", "hlp8", "hlp16", "hlp9"] {
        install_topic(directory.path(), topic);
    }
    for (command, topic) in [
        ("H A", "hlpa"), ("H !", "hlp!"), ("H @", "hlp@"), ("H R", "hlpr"),
        ("H 1", "hlp1"), ("H 8", "hlp8"), ("H 16", "hlp16"), ("H HLP9", "hlp9"),
    ] {
        let output = test_output(format!("{command}\n\n"), |board| {
            setup_conference(board);
            board.config.paths.help_path = directory.path().to_path_buf();
            board.users[0].page_len = 0;
        });
        let source = catalog::sources(None).unwrap().into_iter().find(|source| source.topic == topic).unwrap();
        let rendered = render(&source.markdown, &RenderOptions::default()).unwrap();
        let title = rendered.plain_text.lines().next().unwrap();
        assert!(output.contains(&title), "{command}: {output:?}");
        let body = rendered.plain_text.lines().skip(3).find(|line| !line.trim().is_empty()).unwrap();
        let without_colors = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap().replace_all(&output, "");
        assert!(without_colors.contains(body), "{command}: help body missing: {output:?}");
        assert!(!output.contains("is missing!"), "{output:?}");
        assert!(output.matches("Main Board Command?").count() >= 2, "help did not return: {output:?}");
    }
}

#[tokio::test]
async fn help_lookup_selects_independently_supplied_german_and_falls_back_to_english() {
    let directory = tempfile::tempdir().unwrap();
    install_topic(directory.path(), "hlpa");
    let german = include_str!("../../../icy_board_help/data/de/hlpa.md");
    let translated = directory.path().join(catalog::output_name("hlpa", "de").unwrap());
    let (mut state, mut connection) = display_state(directory.path(), GraphicsMode::Graphics).await;
    state.session.language = "de".to_string();
    // The BBS decodes UTF-8 only when the BOM is present, so both output encodings must reach the terminal.
    for encoding in [Encoding::Cp437, Encoding::Utf8] {
        let options = RenderOptions {
            encoding,
            ..Default::default()
        };
        let generated = render(german, &options).unwrap();
        assert_eq!(generated.bytes.starts_with(&[0xEF, 0xBB, 0xBF]), encoding == Encoding::Utf8);
        std::fs::write(&translated, generated.bytes).unwrap();
        tokio::time::timeout(Duration::from_secs(5), state.show_help("hlpa")).await.unwrap().unwrap();
        let output = drain(&mut connection).await;
        assert!(output.contains("Hilfe: (A)us Konferenz aussteigen"), "{encoding:?}: {output:?}");
        assert!(output.contains("Verläßt"), "{encoding:?}: {output:?}");
    }
    std::fs::remove_file(&translated).unwrap();
    tokio::time::timeout(Duration::from_secs(5), state.show_help("hlpa")).await.unwrap().unwrap();
    let output = drain(&mut connection).await;
    assert!(output.contains("Help: (A)bandon Conference"), "{output:?}");
    assert!(!output.contains("is missing!"), "{output:?}");
}

#[test]
fn reader_context_h_uses_generated_end_of_message_help() {
    let directory = tempfile::tempdir().unwrap();
    install_topic(directory.path(), "hlpendr");
    let output = test_output("R\n1\nH\nN\n".to_string(), |board| {
        setup_conference_with_messages(board);
        board.config.paths.help_path = directory.path().to_path_buf();
        board.users[0].page_len = 0;
    });
    assert!(output.contains("Body of message 1"), "{output:?}");
    assert!(output.contains("Help: End of Message Command"), "{output:?}");
    assert!(output.contains("Scan Subcommands"), "{output:?}");
    assert!(output.contains("SysOp Subcommands"), "help was truncated: {output:?}");
    assert!(output.matches("End of Message Command?").count() >= 2, "reader did not resume: {output:?}");
    assert!(output.matches("Main Board Command?").count() >= 2, "reader did not exit: {output:?}");
}
