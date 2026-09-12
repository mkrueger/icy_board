use crate::icy_board::{
    IcyBoard,
    bbs::BBS,
    state::{IcyBoardState, virtual_screen::VirtualScreen},
    user_base::User,
};
use icy_engine::{Position, TextPane};
use icy_net::{ConnectionType, connection::telnet::TelnetConnection};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

fn source() -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ppe/paint/src/paint.pps")).unwrap()
}

#[test]
fn e3_paint_ansi_startup_and_exit() {
    let output = super::run_ppl_with_input(&source(), b"q");
    assert!(!output.contains("requires"), "{output:?}");
    assert!(output.contains("Paint"), "{output:?}");
}

struct Client {
    stream: TcpStream,
    pending: Vec<u8>,
    screen: VirtualScreen,
}

impl Client {
    async fn through(&mut self, marker: &[u8]) -> Vec<u8> {
        loop {
            if let Some(end) = self.pending.windows(marker.len()).position(|part| part == marker) {
                return self.pending.drain(..end + marker.len()).collect();
            }
            let mut buffer = [0; 8192];
            let count = self.stream.read(&mut buffer).await.unwrap();
            assert_ne!(count, 0, "connection ended before {marker:?}");
            self.pending.extend_from_slice(&buffer[..count]);
        }
    }

    async fn frame(&mut self) -> Vec<u8> {
        let output = self.through(b"\x1b[?2026l").await;
        self.screen.write_bytes(&output);
        output
    }

    async fn send(&mut self, bytes: &[u8]) {
        self.stream.write_all(bytes).await.unwrap();
    }

    async fn resize(&mut self, columns: u16, rows: u16) {
        self.screen.write_bytes(format!("\x1b[8;{rows};{columns}t").as_bytes());
        let mut naws = vec![255, 250, 31];
        for byte in columns.to_be_bytes().into_iter().chain(rows.to_be_bytes()) {
            naws.push(byte);
            if byte == 255 {
                naws.push(byte);
            }
        }
        naws.extend([255, 240]);
        self.send(&naws).await;
    }

    fn line(&self, row: i32, columns: i32) -> String {
        (0..columns)
            .map(|column| self.screen.buffer.char_at(Position::new(column, row)).ch)
            .collect::<String>()
            .trim_end()
            .into()
    }

    fn background(&self, column: i32, row: i32) -> u32 {
        self.screen.buffer.char_at(Position::new(column, row)).attribute.background()
    }

    fn image(&self) -> &icy_engine::Sixel {
        self.screen
            .buffer
            .buffer
            .layers
            .iter()
            .flat_map(|layer| &layer.sixels)
            .last()
            .expect("missing rendered Sixel")
    }

    fn pixel(&self, column: usize, row: usize) -> [u8; 4] {
        let image = self.image();
        let offset = (row * image.width() as usize + column) * 4;
        image.picture_data[offset..offset + 4].try_into().unwrap()
    }

    async fn mouse(&mut self, button: u8, column: usize, row: usize) {
        self.send(format!("\x1b[<{button};{};{}M", column + 1, row + 1).as_bytes()).await;
    }

    fn capture(&self, name: &str) {
        if let Some(directory) = std::env::var_os("ICB_PAINT_CAPTURE_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let image = self.image();
            image::save_buffer(
                directory.join(name),
                &image.picture_data,
                image.width() as u32,
                image.height() as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}

fn assert_clean(state: &mut IcyBoardState) {
    assert!(state.ppl_graphics.is_none());
    assert!(!state.ppl_mouse.is_enabled());
    assert!(!state.ppl_keys.is_enabled());
    assert_eq!(state.ppl_terminal.take_update_depth(), 0);
    assert!(!state.ppl_terminal.take_margins_changed());
}

async fn subsequent_input(root: &std::path::Path, state: &mut IcyBoardState, client: &mut Client) -> Vec<u8> {
    let path = root.join("after.ppe");
    std::fs::write(&path, super::compile("PRINT \"AFTER:\", INKEY()").to_buffer().unwrap()).unwrap();
    assert!(state.run_ppe(&path, None).await.unwrap());
    client.through(b"AFTER:z").await
}

async fn fixture(program: &str) -> (tempfile::TempDir, IcyBoardState, Client, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("paint.ppe");
    std::fs::write(&path, super::compile(program).to_buffer().unwrap()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let peer = TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let mut board = IcyBoard::new();
    board.root_path = root.path().into();
    board.default_display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
    board.users.new_user(User {
        name: "PAINTER".into(),
        ..Default::default()
    });
    let user = board.users[0].clone();
    let node = bbs.lock().await.create_new_node(ConnectionType::Telnet).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let mut state = IcyBoardState::new(
        bbs,
        Arc::new(tokio::sync::Mutex::new(board)),
        nodes,
        node,
        Box::new(TelnetConnection::accept(socket).unwrap()),
    )
    .await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    state.session.term_caps.is_utf8 = true;
    state.session.time_limit = 30;
    state.session.page_len = 0;
    let client = Client {
        stream: peer,
        pending: Vec::new(),
        screen: VirtualScreen::new(icy_parser_core::AnsiParser::default()),
    };
    (root, state, client, path)
}

#[tokio::test]
async fn e3_paint_ansi_draw_erase_resize_and_exit() {
    for language in ["en", "de"] {
        let program = source().replacen(
            "Cleanup()\nEXIT",
            "STRING remaining\nGETTOKEN remaining\nPRINT \"REMAINING:\", remaining\nCleanup()\nEXIT",
            1,
        );
        let (_root, mut state, mut client, path) = fixture(&program).await;
        state.session.tokens.push_back(language.into());
        state.session.tokens.push_back("caller token".into());
        let drive = async {
            client.frame().await;
            assert!(client.line(0, 80).starts_with("Paint"));
            assert!(client.line(1, 80).contains(if language == "de" { "Farbe 1" } else { "Color 1" }));
            client.send(b"2").await;
            client.frame().await;
            client.send(b" ").await;
            client.frame().await;
            assert_eq!(client.background(1, 3), 4);
            client.send(b"\x1b[C").await;
            client.frame().await;
            client.send(b" ").await;
            client.frame().await;
            assert_eq!(client.background(2, 3), 4);
            client.send(b"\x1b[3~").await;
            client.frame().await;
            assert_eq!(client.background(2, 3), 7);
            client.resize(132, 43).await;
            client.frame().await;
            assert_eq!(client.background(1, 3), 4);
            assert_eq!(client.background(120, 35), 7);
            client.resize(80, 25).await;
            client.frame().await;
            assert_eq!(client.background(1, 3), 4);
            client.mouse(0, 77, 21).await;
            client.frame().await;
            assert_eq!(client.background(77, 21), 4);
            client.mouse(2, 77, 21).await;
            client.frame().await;
            assert_eq!(client.background(77, 21), 7);
            for (columns, rows) in [(1, 1), (2, 4), (20, 8), (80, 25)] {
                client.resize(columns, rows).await;
                client.frame().await;
                assert_eq!(client.screen.buffer.size(), icy_engine::Size::new(i32::from(columns), i32::from(rows)));
            }
            assert_eq!(client.background(1, 3), 4);
            client.mouse(0, 1, 2).await;
            client.frame().await;
            assert_eq!(client.background(1, 3), 7);
            client.send(b"qz").await;
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(8), async { tokio::join!(state.run_ppe(&path, None), drive) })
            .await
            .expect("paint stalled");
        assert!(result.unwrap());
        assert_clean(&mut state);
        tokio::time::timeout(Duration::from_secs(3), client.through(b"REMAINING:caller token"))
            .await
            .unwrap();
        assert!(state.session.tokens.is_empty());
        tokio::time::timeout(Duration::from_secs(3), subsequent_input(_root.path(), &mut state, &mut client))
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn e3_paint_sixel_keyboard_text_mouse_and_resize() {
    for language in ["en", "de"] {
        for pixel_mouse in [false, true] {
            let (root, mut state, mut client, path) = fixture(&source()).await;
            state.session.tokens.push_back(language.into());
            client.send(b"\x1b[<1;4c\x1b[6;16;8t\x1b[4;400;640t").await;
            let drive = async {
                client.frame().await;
                assert!(client.line(0, 80).contains("Sixel"), "{}", client.line(0, 80));
                client.send(if pixel_mouse { b"\x1b[?1016;1$y" } else { b"\x1b[?1016;0$y" }).await;
                client.send(b"2").await;
                client.frame().await;
                client.send(b" ").await;
                client.frame().await;
                let image = client.image();
                assert_eq!(image.position, Position::new(1, 3));
                assert_eq!(image.width(), 624);
                assert!(image.picture_data[0] > image.picture_data[1]);
                client.send(b"\x1b[3~").await;
                client.frame().await;
                assert_eq!(client.pixel(0, 0), [255, 255, 255, 255]);
                client.send(b" ").await;
                client.frame().await;
                let (column, row, local_column, local_row) = if pixel_mouse { (28, 68, 20, 20) } else { (5, 5, 36, 40) };
                client.mouse(0, column, row).await;
                client.frame().await;
                let painted = client.pixel(local_column, local_row);
                assert!(painted[0] > painted[1], "mouse={pixel_mouse}: {painted:?}");
                client.mouse(32, if pixel_mouse { 68 } else { 7 }, row).await;
                client.frame().await;
                let stroke = client.pixel(if pixel_mouse { 40 } else { 44 }, local_row);
                assert!(stroke[0] > stroke[1], "stroke={stroke:?}");
                client.capture(&format!("paint-{language}-80x25-pixels-{pixel_mouse}.png"));
                client.send(b"+").await;
                client.frame().await;
                assert!(client.line(1, 80).contains(if language == "de" { "Pinsel 9" } else { "Brush 9" }));
                let minus = if language == "de" { 39 } else { 38 };
                client
                    .mouse(0, if pixel_mouse { minus * 8 } else { minus }, if pixel_mouse { 16 } else { 1 })
                    .await;
                client.frame().await;
                assert!(client.line(1, 80).contains(if language == "de" { "Pinsel 7" } else { "Brush 7" }));
                client.resize(132, 43).await;
                client.frame().await;
                let image = client.image();
                assert_eq!(image.width(), 1024);
                assert!(image.picture_data[0] > image.picture_data[1]);
                client.capture(&format!("paint-{language}-132x43-pixels-{pixel_mouse}.png"));
                client.resize(1, 1).await;
                client.frame().await;
                assert!(client.screen.buffer.buffer.layers.iter().all(|layer| layer.sixels.is_empty()));
                client.resize(80, 25).await;
                client.frame().await;
                assert!(client.pixel(0, 0)[0] > client.pixel(0, 0)[1]);
                for (columns, rows, width, height) in [(80, 25, 624, 320), (132, 43, 1024, 608)] {
                    if columns != 80 {
                        client.resize(columns, rows).await;
                        client.frame().await;
                    }
                    for (right_edge, bottom_edge) in [(false, false), (true, false), (false, true), (true, true)] {
                        client.send(b"c").await;
                        client.frame().await;
                        let corner_column = if right_edge { width - 1 } else { 0 };
                        let corner_row = if bottom_edge { height - 1 } else { 0 };
                        let (column, row) = if pixel_mouse {
                            (corner_column + 8, corner_row + 48)
                        } else {
                            (corner_column / 8 + 1, corner_row / 16 + 3)
                        };
                        client.mouse(0, column, row).await;
                        client.frame().await;
                        client
                            .send(match (right_edge, bottom_edge) {
                                (false, false) => b"\x1b[D\x1b[A",
                                (true, false) => b"\x1b[C\x1b[A",
                                (false, true) => b"\x1b[D\x1b[B",
                                (true, true) => b"\x1b[C\x1b[B",
                            })
                            .await;
                        client.frame().await;
                        client.send(b" ").await;
                        client.frame().await;
                        assert_eq!(client.image().position, Position::new(1, 3));
                        assert_eq!(client.image().width() as usize, width);
                        assert_eq!(client.image().height() as usize, height.div_ceil(6) * 6);
                        let painted = client.pixel(corner_column, corner_row);
                        assert!(
                            painted[0] > painted[1],
                            "{columns}x{rows}, mouse={pixel_mouse}, corner=({corner_column},{corner_row}): {painted:?}"
                        );
                        let outline = client.pixel(if right_edge { width - 5 } else { 4 }, if bottom_edge { height - 5 } else { 4 });
                        assert!(outline[..3].iter().all(|channel| *channel <= 5), "outline={outline:?}");
                        assert_eq!(outline[3], 255);
                        assert_eq!(client.pixel(width / 2, height / 2), [255, 255, 255, 255]);
                    }
                }
                client.send(b"qz").await;
            };
            let (result, ()) = tokio::time::timeout(Duration::from_secs(20), async { tokio::join!(state.run_ppe(&path, None), drive) })
                .await
                .expect("sixel paint stalled");
            assert!(result.unwrap());
            assert_clean(&mut state);
            tokio::time::timeout(Duration::from_secs(3), subsequent_input(root.path(), &mut state, &mut client))
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn e3_paint_cleanup_on_stop_error_and_disconnect() {
    for graphics in [false, true] {
        for ending in ["stop", "error", "disconnect"] {
            let program = match ending {
                "stop" => source().replacen("Cleanup()\nEXIT", "Terminal.BeginUpdate()\nSTOP", 1),
                "error" => source().replacen(
                    "Cleanup()\nEXIT",
                    "FOPEN 1, \"fault.dat\", O_RW, S_DN\nON ERROR OFF\nTerminal.BeginUpdate()\nFSEEK 1, 0, 99",
                    1,
                ),
                _ => source(),
            };
            let (root, mut state, mut client, path) = fixture(&program).await;
            if graphics {
                client.send(b"\x1b[<1;4c\x1b[6;16;8t").await;
            }
            let drive = async {
                client.frame().await;
                if ending == "disconnect" {
                    client.stream.shutdown().await.unwrap();
                } else {
                    client.send(b"q").await;
                }
            };
            let (result, ()) = tokio::time::timeout(Duration::from_secs(10), async { tokio::join!(state.run_ppe(&path, None), drive) })
                .await
                .expect("cleanup stalled");
            if ending == "disconnect" {
                assert!(!result.unwrap_or(false));
            } else {
                assert!(!result.unwrap());
            }
            assert_clean(&mut state);
            if ending != "disconnect" {
                let cleanup = tokio::time::timeout(Duration::from_secs(3), client.through(b"\x1b[?2026l")).await.unwrap();
                assert!(cleanup.windows(b"\x1b[?1006l".len()).any(|part| part == b"\x1b[?1006l"));
                client.send(b"z").await;
                let after = tokio::time::timeout(Duration::from_secs(3), subsequent_input(root.path(), &mut state, &mut client))
                    .await
                    .unwrap();
                if ending == "error" {
                    let output = String::from_utf8_lossy(&after);
                    assert!(output.to_ascii_lowercase().contains("seek"), "{output:?}");
                }
            }
        }
    }
}
