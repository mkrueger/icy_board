use std::{path::Path, sync::Arc, time::Duration};

use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::BBS,
    commands::CommandList,
    conferences::Conference,
    icb_config::DisplayNewsBehavior,
    icb_text::{DEFAULT_DISPLAY_TEXT, IceText},
    message_area::AreaList,
    state::IcyBoardState,
    user_base::User,
};
use icy_engine::{BufferType, TextPane, TextScreen};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection, termcap_detect};
use icy_parser_core::{AnsiParser, CommandParser};
use tokio::sync::Mutex;

use crate::bbs::{LoginOptions, internal_handle_client};

const DEADLINE: Duration = Duration::from_secs(10);
const COMMAND: &str = "[paging-command]";
const MORE: &str = "[paging-more]";
const ENTER: &str = "[paging-enter]";
const MENU: &str = "[paging-menu]";
const LINE: &str = "[paging-line-";

fn board_at(root: &Path, page_len: u16, lines: usize, inline_cls: bool) -> IcyBoard {
    let mut board = IcyBoard::new();
    board.root_path = root.to_path_buf();
    board.file_name = root.join("icboard.toml");
    board.config.paths.user_file = root.join("users.toml");
    board.config.paths.statistics_file = root.join("statistics.toml");
    board.config.paths.help_path = root.to_path_buf();
    board.resolve_paths();
    board.config.switches.display_news_behavior = DisplayNewsBehavior::Never;
    board.config.switches.scan_new_blt = false;
    board.config.switches.display_userinfo_at_login = false;
    board.config.message.disable_message_scan_prompt = true;
    board.config.message.prompt_to_read_mail = false;
    board.commands = CommandList::new();
    board.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    for (id, marker) in [(IceText::CommandPrompt, COMMAND), (IceText::MorePrompt, MORE), (IceText::PressEnter, ENTER)] {
        board.default_display_text.update_record_number(id as usize, marker).unwrap();
    }
    let mut user = User {
        name: "PAGING CALLER".into(),
        security_level: 255,
        page_len,
        ..Default::default()
    };
    // The sysop login shortcut must not hide the non-expert end-of-command pause.
    user.flags.expert_mode = false;
    user.flags.use_graphics = true;
    board.users.new_user(user);
    board.save_userbase().unwrap();
    let menu = root.join("menu");
    std::fs::write(&menu, format!("@POFF@{MENU}\r\n")).unwrap();
    board.conferences.push(Conference {
        name: "Paging".into(),
        users_menu: menu.clone(),
        sysop_menu: menu,
        areas: Some(Arc::new(AreaList::new(Vec::new()))),
        ..Default::default()
    });
    // CLS shares the first numbered line, exactly like the oracle's PGC fixtures.
    let mut help = if inline_cls { "@CLS@".to_string() } else { String::new() };
    for line in 1..=lines {
        help.push_str(&format!("{LINE}{line:03}]\r\n"));
    }
    std::fs::write(root.join("hlpe"), help).unwrap();
    board
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Prompt {
    More,
    Enter,
    Command,
}

#[derive(Debug)]
struct Stop {
    prompt: Prompt,
    after_line: usize,
    rows: Vec<String>,
}

#[derive(Debug)]
struct HelpOutput {
    stops: Vec<Stop>,
    lines: Vec<usize>,
    transcript: String,
}

fn numbered_lines(output: &[u8]) -> Vec<usize> {
    String::from_utf8_lossy(output)
        .split(LINE)
        .skip(1)
        .map(|suffix| suffix[..3].parse().expect("complete numbered line before prompt"))
        .collect()
}

fn rendered_rows(output: &[u8]) -> Vec<String> {
    let mut screen = TextScreen::new((80, 25));
    screen.buffer.buffer_type = BufferType::Unicode;
    screen.buffer.terminal_state.is_terminal_buffer = true;
    screen.buffer.terminal_state.last_column_flag_mode = true;
    AnsiParser::default().parse(output, &mut icy_engine::ScreenSink::new(&mut screen));
    (0..25)
        .map(|y| (0..80).map(|x| screen.char_at((x, y).into()).ch).collect::<String>().trim_end().to_string())
        .collect()
}

struct Terminal {
    peer: ChannelConnection,
    output: Vec<u8>,
    consumed: usize,
    size_probes: usize,
    encoding_probes: usize,
}

impl Terminal {
    async fn send(&mut self, input: &str) {
        tokio::time::timeout(DEADLINE, self.peer.send(input.as_bytes()))
            .await
            .expect("send timed out")
            .unwrap();
    }

    async fn read(&mut self) -> bool {
        let mut bytes = [0; 4096];
        let size = self.peer.read(&mut bytes).await.unwrap();
        let packet = &bytes[..size];
        // ChannelConnection retains these small probe packets. Never send command
        // input until detection has finished and the actual command prompt arrives.
        match packet {
            b"\x1b[999;999H\x1b[6n" => {
                self.size_probes += 1;
                self.send("\x1b[25;80R").await;
            }
            b"\x1b[1;1H\x01\xF6\x1c\x1b[6n" => {
                self.encoding_probes += 1;
                self.send("\x1b[1;1R").await;
            }
            b"\x1b[!\x07\x07\x07" => {}
            _ if [
                termcap_detect::DEVICE_ATTRIBUTES_QUERY,
                termcap_detect::CTERM_ATTRIBUTES_QUERY,
                termcap_detect::CELL_SIZE_QUERY,
                termcap_detect::PIXEL_SIZE_QUERY,
                termcap_detect::JXL_QUERY,
                termcap_detect::SOUND_QUERY,
                termcap_detect::SYNCHRONIZED_OUTPUT_QUERY,
                termcap_detect::TERMINAL_MACRO_QUERY,
            ]
            .contains(&packet) => {}
            _ => self.output.extend_from_slice(packet),
        }
        size != 0
    }

    async fn prompt(&mut self) -> Prompt {
        let result = tokio::time::timeout(DEADLINE, async {
            loop {
                let next = [(MORE, Prompt::More), (ENTER, Prompt::Enter), (COMMAND, Prompt::Command)]
                    .into_iter()
                    .filter_map(|(marker, prompt)| {
                        self.output[self.consumed..]
                            .windows(marker.len())
                            .position(|bytes| bytes == marker.as_bytes())
                            .map(|offset| (offset, marker.len(), prompt))
                    })
                    .min_by_key(|(offset, _, _)| *offset);
                if let Some((offset, length, prompt)) = next {
                    self.consumed += offset + length;
                    return prompt;
                }
                assert!(self.read().await, "EOF waiting for prompt: {:?}", String::from_utf8_lossy(&self.output));
            }
        })
        .await;
        result.unwrap_or_else(|_| panic!("prompt timeout: {:?}", String::from_utf8_lossy(&self.output)))
    }

    async fn help(&mut self, first_more_response: &str) -> HelpOutput {
        let start = self.consumed;
        self.send("H E\r").await;
        let mut stops = Vec::new();
        let mut answered_more = false;
        loop {
            let prompt = self.prompt().await;
            if prompt == Prompt::Command {
                let output = &self.output[start..self.consumed];
                return HelpOutput {
                    stops,
                    lines: numbered_lines(output),
                    transcript: String::from_utf8_lossy(output).into_owned(),
                };
            }
            assert!(stops.len() < 64, "prompt loop: {:?}", String::from_utf8_lossy(&self.output));
            stops.push(Stop {
                prompt,
                after_line: numbered_lines(&self.output[start..self.consumed]).last().copied().unwrap_or(0),
                rows: rendered_rows(&self.output[..self.consumed]),
            });
            if prompt == Prompt::More && !answered_more {
                answered_more = true;
                self.send(first_more_response).await;
            } else {
                self.send("\r").await;
            }
        }
    }
}

async fn remote_help(page_len: u16, lines: usize, inline_cls: bool, first_more_response: &str) -> Vec<HelpOutput> {
    let directory = tempfile::tempdir().unwrap();
    let board = Arc::new(Mutex::new(board_at(directory.path(), page_len, lines, inline_cls)));
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let state = IcyBoardState::new(bbs, board, nodes, node, Box::new(connection)).await;
    let mut terminal = Terminal {
        peer,
        output: Vec::new(),
        consumed: 0,
        size_probes: 0,
        encoding_probes: 0,
    };
    let client = async {
        assert_eq!(terminal.prompt().await, Prompt::Command, "unexpected login pause");
        assert!(String::from_utf8_lossy(&terminal.output[..terminal.consumed]).contains(MENU));
        assert_eq!((terminal.size_probes, terminal.encoding_probes), (1, 1), "must exercise remote detection");
        let mut results = vec![terminal.help(first_more_response).await];
        if first_more_response != "\r" {
            results.push(terminal.help("\r").await);
        }
        terminal.send("BYE\r").await;
        tokio::time::timeout(DEADLINE, async { while terminal.read().await {} })
            .await
            .expect("BYE did not close the connection");
        results
    };
    // Both futures stay owned here: timeout/panic cancels the server too, rather
    // than leaving a detached session thread holding a deleted fixture directory.
    let options = LoginOptions {
        login_sysop: true,
        ppe: None,
        local: false,
    };
    let (server, results) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(internal_handle_client(state, Some(options), ""), client)
    })
    .await
    .unwrap_or_else(|_| panic!("session timeout: page={page_len}, lines={lines}, CLS={inline_cls}, response={first_more_response:?}"));
    server.expect("remote session failed");
    results
}

fn assert_help(output: &HelpOutput, more: usize, enter: usize, lines: usize) {
    let expected = std::iter::repeat_n(Prompt::More, more)
        .chain(std::iter::repeat_n(Prompt::Enter, enter))
        .collect::<Vec<_>>();
    assert_eq!(output.stops.iter().map(|stop| stop.prompt).collect::<Vec<_>>(), expected, "{output:#?}");
    assert_eq!(
        output.lines,
        (1..=lines).collect::<Vec<_>>(),
        "missing, duplicated or aborted help: {output:#?}"
    );
    assert_eq!(output.transcript.matches(MENU).count(), 1, "menu must redisplay exactly once: {output:#?}");
    assert_eq!(output.transcript.matches(COMMAND).count(), 1, "{output:#?}");
}

fn assert_remote23_geometry(output: &HelpOutput, lines: usize) {
    let first = &output.stops[0];
    assert_eq!(first.after_line, lines.min(23), "{output:#?}");
    // Oracle cursor row 24 (1-based) is TextScreen row 23, not a local 23-row screen.
    assert!(first.rows[23].contains(MORE), "{first:#?}");
    assert_eq!(first.rows[24], "", "{first:#?}");
    for line in 1..=lines.min(23) {
        assert_eq!(first.rows[line - 1], format!("{LINE}{line:03}]"), "{first:#?}");
    }
    if output.stops.len() > 1 && output.stops[1].prompt == Prompt::More {
        let second = &output.stops[1];
        assert_eq!(second.after_line, 46, "{second:#?}");
        assert!(second.rows[24].contains(MORE), "{second:#?}");
        for line in 24..=46 {
            assert_eq!(second.rows[line - 23], format!("{LINE}{line:03}]"), "{second:#?}");
        }
    }
}

// PCBoard 15.4/M measurements: target/paging-oracle/run-fdmydq7f/run.json
// and transcript.txt, PGC/PGN 22/23/24/46 Enter cases. No runtime oracle dependency.
async fn enter_matrix(page_len: u16, expected: [(usize, usize); 4]) {
    for inline_cls in [false, true] {
        for (lines, (more, enter)) in [22, 23, 24, 46].into_iter().zip(expected) {
            let results = remote_help(page_len, lines, inline_cls, "\r").await;
            assert_help(&results[0], more, enter, lines);
            for (index, stop) in results[0].stops.iter().enumerate() {
                let after_line = if stop.prompt == Prompt::More {
                    ((index + 1) * usize::from(page_len)).min(lines)
                } else {
                    lines
                };
                assert_eq!(stop.after_line, after_line, "page={page_len}, lines={lines}, CLS={inline_cls}: {stop:#?}");
            }
            if page_len == 23 && inline_cls {
                assert_remote23_geometry(&results[0], lines);
            }
        }
    }
}

#[tokio::test]
async fn remote_page23_matches_pcboard_more_and_end_of_command() {
    enter_matrix(23, [(1, 1), (1, 0), (1, 1), (2, 0)]).await;
}

#[tokio::test]
async fn page_zero_disables_more_but_keeps_end_of_command_enter() {
    enter_matrix(0, [(0, 1); 4]).await;
}

#[tokio::test]
async fn page_two_matches_pcboard_trailing_newline_boundaries() {
    enter_matrix(2, [(11, 0), (12, 1), (12, 0), (23, 0)]).await;
}

#[tokio::test]
async fn page_one_matches_pcboard_post_content_pauses() {
    enter_matrix(1, [(23, 0), (24, 0), (25, 0), (47, 0)]).await;
}

#[tokio::test]
async fn page_twenty_four_matches_pcboard_end_of_command() {
    enter_matrix(24, [(0, 1), (1, 1), (1, 0), (1, 1)]).await;
}

#[tokio::test]
async fn page_five_matches_pcboard_partial_final_pages() {
    enter_matrix(5, [(4, 1), (4, 1), (5, 1), (9, 1)]).await;
}

#[tokio::test]
async fn more_no_aborts_without_enter_and_next_help_pages_again() {
    for inline_cls in [false, true] {
        let results = remote_help(23, 46, inline_cls, "N\r").await;
        assert_help(&results[0], 1, 0, 23);
        assert_help(&results[1], 2, 0, 46);
        if inline_cls {
            assert_remote23_geometry(&results[0], 46);
            assert_remote23_geometry(&results[1], 46);
        }
    }
}

#[tokio::test]
async fn more_nonstop_finishes_without_enter_and_next_help_pages_again() {
    for inline_cls in [false, true] {
        let results = remote_help(23, 46, inline_cls, "NS\r").await;
        assert_help(&results[0], 1, 0, 46);
        assert_help(&results[1], 2, 0, 46);
        if inline_cls {
            assert_remote23_geometry(&results[0], 46);
            assert_remote23_geometry(&results[1], 46);
        }
    }
}
