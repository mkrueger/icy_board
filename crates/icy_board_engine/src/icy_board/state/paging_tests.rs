use super::*;
use icy_engine::{BufferType, IceMode, TextScreen};
use icy_parser_core::{AnsiParser, CommandParser};

const DEADLINE: Duration = Duration::from_secs(5);
const MORE: &str = "[state-more]";
const ENTER: &str = "[state-enter]";
const DONE: &str = "[state-done]";
const HELP: [&str; 4] = ["[help-enter]", "[help-yes]", "[help-no]", "[help-nonstop]"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Prompt {
    More,
    Enter,
    Done,
}

#[derive(Debug)]
struct Capture {
    raw: Vec<u8>,
    stops: Vec<(Prompt, usize)>,
}

impl Capture {
    fn before(&self, stop: usize) -> &[u8] {
        &self.raw[..self.stops[stop].1]
    }

    fn text(&self) -> String {
        String::from_utf8(self.raw.clone()).unwrap()
    }
}

async fn paging_state(page_len: u16, local: bool) -> (IcyBoardState, ChannelConnection) {
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.default_display_text = super::super::icb_text::DEFAULT_DISPLAY_TEXT.clone();
    for (id, marker) in [
        (IceText::MorePrompt, MORE),
        (IceText::PressEnter, ENTER),
        (IceText::MorehelpEnter, HELP[0]),
        (IceText::MorehelpYes, HELP[1]),
        (IceText::MorehelpNo, HELP[2]),
        (IceText::MorehelpNonstop, HELP[3]),
    ] {
        board.default_display_text.update_record_number(id as usize, marker).unwrap();
    }
    let user = User {
        name: "STATE PAGING TEST".into(),
        page_len,
        security_level: 255,
        ..Default::default()
    };
    board.users.new_user(user.clone());
    let mut state = IcyBoardState::new(bbs, Arc::new(Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    state.session.is_sysop = false;
    state.session.is_local = local;
    state.session.page_len = page_len;
    state.session.term_caps.is_utf8 = true;
    state.session.disp_options.grapics_mode = GraphicsMode::Graphics;
    state.set_terminal_size(80, if local { 23 } else { 25 });
    (state, peer)
}

// Every read ends at a protocol marker, never at an idle interval or a packet boundary.
async fn capture(peer: &mut ChannelConnection, replies: &[(Prompt, &str)]) -> Capture {
    let mut result = Capture {
        raw: Vec::new(),
        stops: Vec::new(),
    };
    let mut consumed = 0;
    loop {
        let next = [(MORE, Prompt::More), (ENTER, Prompt::Enter), (DONE, Prompt::Done)]
            .into_iter()
            .filter_map(|(marker, prompt)| {
                result.raw[consumed..]
                    .windows(marker.len())
                    .position(|bytes| bytes == marker.as_bytes())
                    .map(|offset| (consumed + offset, marker.len(), prompt))
            })
            .min_by_key(|(offset, _, _)| *offset);
        if let Some((offset, length, prompt)) = next {
            consumed = offset + length;
            let index = result.stops.len();
            let expected = replies.get(index).map_or(Prompt::Done, |entry| entry.0);
            assert_eq!(prompt, expected, "unexpected prompt #{index}: {}", result.text());
            result.stops.push((prompt, consumed));
            if prompt == Prompt::Done {
                assert_eq!(index, replies.len(), "operation finished before all expected prompts: {result:?}");
                return result;
            }
            let reply = replies[index].1;
            if !reply.is_empty() {
                peer.send(reply.as_bytes()).await.unwrap();
            }
        } else {
            let mut packet = [0; 4096];
            let size = peer.read(&mut packet).await.unwrap();
            assert_ne!(size, 0, "EOF before completion marker: {}", result.text());
            result.raw.extend_from_slice(&packet[..size]);
        }
    }
}

async fn exchange(
    state: &mut IcyBoardState,
    peer: &mut ChannelConnection,
    replies: &[(Prompt, &str)],
    operation: impl AsyncFnOnce(&mut IcyBoardState),
) -> Capture {
    let operation = async {
        operation(state).await;
        // A transport marker still terminates guard/abort tests with display disabled.
        state.connection.send(DONE.as_bytes()).await.unwrap();
    };
    let (_, output) = tokio::time::timeout(DEADLINE, async { tokio::join!(operation, capture(peer, replies)) })
        .await
        .expect("paging operation/reader did not complete within five seconds");
    output
}

fn screen(raw: &[u8], height: i32) -> TextScreen {
    let mut screen = TextScreen::new((80, height));
    screen.buffer.buffer_type = BufferType::Unicode;
    screen.buffer.terminal_state.is_terminal_buffer = true;
    screen.buffer.terminal_state.last_column_flag_mode = true;
    AnsiParser::default().parse(raw, &mut icy_engine::ScreenSink::new(&mut screen));
    screen
}

fn row(screen: &TextScreen, y: i32) -> String {
    (0..80).map(|x| screen.char_at((x, y).into()).ch).collect::<String>().trim_end().to_string()
}

fn body_lines(raw: &[u8]) -> Vec<usize> {
    String::from_utf8_lossy(raw)
        .split("[body-")
        .skip(1)
        .map(|suffix| suffix[..3].parse().unwrap())
        .collect()
}

async fn body_line(state: &mut IcyBoardState, number: usize) {
    state.print(TerminalTarget::Both, &format!("[body-{number:03}]")).await.unwrap();
    state.new_line().await.unwrap();
}

#[tokio::test]
async fn counted_boundaries_and_rendered_prompt_rows_remote25_local23() {
    for local in [false, true] {
        for page_len in [0, 1, 2, 5, 23, 24, 255] {
            let (mut state, mut peer) = paging_state(page_len, local).await;
            let height = if local { 23 } else { 25 };
            let limit = if page_len == 0 {
                None
            } else {
                Some(usize::from(if local { page_len.min(22) } else { page_len }))
            };
            assert_eq!(state.page_line_limit(), limit);
            let lines = limit.map_or(256, |limit| 2 * limit + limit.saturating_sub(1));
            let replies = if limit.is_some() {
                vec![(Prompt::More, "\r"), (Prompt::More, "Y\r")]
            } else {
                Vec::new()
            };
            let output = exchange(&mut state, &mut peer, &replies, async |state| {
                for line in 1..=lines {
                    body_line(state, line).await;
                    assert_eq!(
                        state.session.disp_options.num_lines_printed,
                        limit.map_or(line, |limit| line % limit),
                        "page={page_len}, local={local}, line={line}"
                    );
                }
            })
            .await;
            assert_eq!(body_lines(&output.raw), (1..=lines).collect::<Vec<_>>(), "{output:?}");
            if let Some(limit) = limit {
                for stop in 0..2 {
                    assert_eq!(body_lines(output.before(stop)), (1..=(stop + 1) * limit).collect::<Vec<_>>());
                }
                let first_page = screen(output.before(0), height);
                let prompt_row = (limit as i32).min(height - 1);
                assert_eq!(row(&first_page, prompt_row), MORE, "page={page_len}, local={local}");
                for y in 0..prompt_row {
                    let number = limit - prompt_row as usize + y as usize + 1;
                    assert_eq!(row(&first_page, y), format!("[body-{number:03}]"), "page={page_len}, local={local}, row={y}");
                }
                for y in prompt_row + 1..height {
                    assert!(row(&first_page, y).is_empty(), "prompt consumed a reserved row");
                }
            } else {
                assert!(!output.text().contains(MORE));
                let rendered = screen(&output.raw, height);
                assert_eq!(row(&rendered, height - 2), "[body-256]");
                assert_eq!(row(&rendered, height - 1), DONE);
            }
        }
    }
}

#[tokio::test]
async fn explicitly_continuing_a_partial_page_starts_a_fresh_count() {
    let (mut state, mut peer) = paging_state(5, false).await;
    let output = exchange(
        &mut state,
        &mut peer,
        &[(Prompt::More, "Y\r"), (Prompt::More, "\r"), (Prompt::Enter, "\r")],
        async |state| {
            for line in 1..=2 {
                body_line(state, line).await;
            }
            assert_eq!(state.session.disp_options.num_lines_printed, 2);
            state.more_promt().await.unwrap();
            assert_eq!(state.session.disp_options.num_lines_printed, 0);
            for line in 3..=7 {
                body_line(state, line).await;
                assert_eq!(state.session.disp_options.num_lines_printed, (line - 2) % 5);
            }
            body_line(state, 8).await;
            body_line(state, 9).await;
            assert_eq!(state.session.disp_options.num_lines_printed, 2);
            state.press_enter().await.unwrap();
            assert_eq!(state.session.disp_options.num_lines_printed, 0);
            state.new_line().await.unwrap();
            assert_eq!(state.session.disp_options.num_lines_printed, 1);
        },
    )
    .await;
    assert_eq!(body_lines(output.before(0)), vec![1, 2]);
    assert_eq!(body_lines(output.before(1)), (1..=7).collect::<Vec<_>>());
    assert_eq!(body_lines(output.before(2)), (1..=9).collect::<Vec<_>>());
}

#[tokio::test]
async fn raw_crlf_does_not_count_poff_preserves_and_pon_resets() {
    let (mut state, mut peer) = paging_state(5, true).await;
    let output = exchange(&mut state, &mut peer, &[(Prompt::More, "\r")], async |state| {
        state.print(TerminalTarget::Both, "raw\r\n".repeat(30).as_str()).await.unwrap();
        assert_eq!(state.session.disp_options.num_lines_printed, 0);
        for line in 1..=3 {
            body_line(state, line).await;
        }
        state.print(TerminalTarget::Both, "@POFF@").await.unwrap();
        assert!(!state.session.disp_options.count_lines);
        assert_eq!(state.session.disp_options.num_lines_printed, 3);
        // Even an already-overdue counter must not request a pause while counting is off.
        state.session.page_len = 2;
        for _ in 0..10 {
            state.new_line().await.unwrap();
            assert_eq!(state.session.disp_options.num_lines_printed, 3);
            assert!(!state.session.more_requested);
        }
        state.session.page_len = 5;
        state.print(TerminalTarget::Both, "@PON@").await.unwrap();
        assert!(state.session.disp_options.count_lines);
        assert_eq!(state.session.disp_options.num_lines_printed, 0);
        for line in 4..=8 {
            body_line(state, line).await;
        }
        assert_eq!(state.session.disp_options.num_lines_printed, 0);
        state.new_line().await.unwrap();
        assert_eq!(state.session.disp_options.num_lines_printed, 1);
    })
    .await;
    let before_more = String::from_utf8_lossy(output.before(0));
    assert_eq!(before_more.matches("raw\r\n").count(), 30);
    assert_eq!(body_lines(output.before(0)), (1..=8).collect::<Vec<_>>());
    assert!(!output.text().contains("@POFF@"));
    assert!(!output.text().contains("@PON@"));
}

#[tokio::test]
async fn no_aborts_and_suppresses_further_newlines_and_counting() {
    let (mut state, mut peer) = paging_state(2, false).await;
    let output = exchange(&mut state, &mut peer, &[(Prompt::More, "N\r")], async |state| {
        body_line(state, 1).await;
        body_line(state, 2).await;
        assert!(state.session.disp_options.abort_printout);
        let count = state.session.disp_options.num_lines_printed;
        state.print(TerminalTarget::Both, "[after-abort]").await.unwrap();
        for _ in 0..10 {
            state.new_line().await.unwrap();
        }
        state.more_promt().await.unwrap();
        state.press_enter().await.unwrap();
        assert_eq!(state.session.disp_options.num_lines_printed, count);
        assert!(state.session.disp_options.abort_printout);
    })
    .await;
    assert!(output.text().ends_with(&format!("[after-abort]{DONE}")), "{output:?}");
    let rendered = screen(&output.raw, 25);
    assert_eq!(row(&rendered, 2), format!("[after-abort]{DONE}"));
}

#[tokio::test]
async fn nonstop_survives_input_and_no_change_until_next_command() {
    let (mut state, mut peer) = paging_state(2, false).await;
    let output = exchange(
        &mut state,
        &mut peer,
        &[(Prompt::More, "NS\r"), (Prompt::Enter, "\r"), (Prompt::More, "\r")],
        async |state| {
            body_line(state, 1).await;
            body_line(state, 2).await;
            assert!(state.session.disp_options.non_stop_during_cmd);
            assert!(!state.session.disp_options.count_lines);
            state.session.disp_options.no_change();
            state.press_enter().await.unwrap();
            assert!(state.session.disp_options.non_stop_during_cmd);
            assert!(!state.session.disp_options.count_lines);
            for line in 3..=8 {
                body_line(state, line).await;
            }
            assert_eq!(state.session.disp_options.num_lines_printed, 0);
            assert_eq!(state.session.push_tokens(""), 0);
            assert!(!state.session.disp_options.non_stop_during_cmd);
            assert!(state.session.disp_options.count_lines);
            assert_eq!(state.session.disp_options.num_lines_printed, 0);
            body_line(state, 9).await;
            assert_eq!(state.session.disp_options.num_lines_printed, 1);
            body_line(state, 10).await;
        },
    )
    .await;
    assert_eq!(body_lines(output.before(0)), vec![1, 2]);
    assert_eq!(body_lines(output.before(2)), (1..=10).collect::<Vec<_>>());
}

#[tokio::test]
async fn press_enter_and_breakoff_ignore_letters_and_consume_only_through_enter() {
    for breakoff in [false, true] {
        let (mut state, mut peer) = paging_state(2, false).await;
        state.session.more_requested = true;
        let output = exchange(&mut state, &mut peer, &[(Prompt::Enter, "nHySxyz\r!")], async |state| {
            if breakoff {
                state.session.disp_options.allow_break = false;
                body_line(state, 1).await;
                body_line(state, 2).await;
            } else {
                state.press_enter().await.unwrap();
            }
            assert!(!state.session.more_requested);
            assert!(!state.session.disp_options.abort_printout);
            assert!(!state.session.disp_options.non_stop_during_cmd);
            assert_eq!(state.session.last_answer.as_deref(), Some(""));
            loop {
                if let Some(key) = state.get_char(TerminalTarget::Both).await.unwrap() {
                    assert_eq!(key.ch, '!', "PressEnter returned before consuming the Enter key");
                    break;
                }
            }
        })
        .await;
        assert!(!output.text().contains("nHySxyz"));
        let rendered = screen(output.before(0), 25);
        assert_eq!(row(&rendered, if breakoff { 2 } else { 0 }), ENTER);
    }
}

#[tokio::test]
async fn hidden_and_disconnected_prompts_do_not_wait_for_input() {
    for disconnected in [false, true] {
        let (mut state, mut peer) = paging_state(1, false).await;
        let output = exchange(&mut state, &mut peer, &[], async |state| {
            state.session.more_requested = true;
            state.session.disp_options.num_lines_printed = 7;
            if disconnected {
                state.session.request_logoff = true;
            } else {
                state.session.disp_options.show_on_screen = false;
                state.new_line().await.unwrap();
            }
            state.more_promt().await.unwrap();
            state.press_enter().await.unwrap();
            assert!(!state.session.more_requested);
            assert_eq!(state.session.disp_options.num_lines_printed, 7);
        })
        .await;
        assert_eq!(output.raw, DONE.as_bytes());
    }
}

#[tokio::test]
async fn press_enter_preserves_pending_command_arguments() {
    let (mut state, mut peer) = paging_state(23, false).await;
    state.session.tokens.push_back("NEXT".to_string());
    exchange(&mut state, &mut peer, &[(Prompt::Enter, "\r")], async |state| {
        state.press_enter().await.unwrap();
        assert_eq!(state.session.tokens.pop_front().as_deref(), Some("NEXT"));
        assert!(state.session.tokens.is_empty());
    })
    .await;
}

#[tokio::test]
async fn input_resets_counter_without_reenabling_menu_paging() {
    let (mut state, mut peer) = paging_state(1, false).await;
    state.session.disp_options.num_lines_printed = 9;
    state.session.disp_options.force_non_stop();
    exchange(&mut state, &mut peer, &[(Prompt::Enter, "\r")], async |state| {
        state.input_field(IceText::PressEnter, 0, "", "", None, display_flags::NEWLINE).await.unwrap();
        assert_eq!(state.session.disp_options.num_lines_printed, 0);
        assert!(!state.session.disp_options.count_lines);
        assert!(!state.session.more_requested);
    })
    .await;
}

#[tokio::test]
async fn more_accepts_the_sessions_localized_yes_character() {
    let (mut state, mut peer) = paging_state(1, false).await;
    state.session.yes_char = 'J';
    state.session.yes_no_mask = "JjNn".to_string();
    exchange(&mut state, &mut peer, &[(Prompt::More, "j\r")], async |state| {
        body_line(state, 1).await;
        assert_eq!(state.session.last_answer.as_deref(), Some("J"));
        assert!(!state.session.disp_options.abort_printout);
        assert_eq!(state.session.disp_options.num_lines_printed, 0);
    })
    .await;
}

async fn help_reprompts_with_pcboard_paging(page_len: u16) {
    let (mut state, mut peer) = paging_state(page_len, false).await;
    // Oracle: four help lines plus a leading/trailing blank; small pages also pause inside help.
    let nested = 6 / usize::from(page_len);
    let replies = std::iter::once((Prompt::More, "H\r"))
        .chain(std::iter::repeat_n((Prompt::More, "\r"), nested + 1))
        .collect::<Vec<_>>();
    let output = exchange(&mut state, &mut peer, &replies, async |state| {
        for line in 1..=usize::from(page_len) {
            body_line(state, line).await;
        }
        assert_eq!(state.session.disp_options.num_lines_printed, 0);
        assert!(!state.session.disp_options.abort_printout);
        state.print(TerminalTarget::Both, "[resumed]").await.unwrap();
    })
    .await;
    for stop in 1..replies.len() {
        assert_eq!(body_lines(output.before(0)), body_lines(output.before(stop)), "help advanced the body page");
        let help_lines = (stop * usize::from(page_len)).min(6);
        let shown = String::from_utf8_lossy(output.before(stop));
        for (index, marker) in HELP.iter().enumerate() {
            assert_eq!(shown.contains(marker), index + 2 <= help_lines, "page={page_len}, stop={stop}: {shown}");
        }
    }
    let reprompt = String::from_utf8_lossy(output.before(replies.len() - 1));
    let rendered = screen(output.before(replies.len() - 1), 25);
    let rows = (0..25).map(|y| row(&rendered, y)).collect::<Vec<_>>();
    for marker in HELP {
        assert_eq!(reprompt.matches(marker).count(), 1, "help must finish before the reprompt: {output:?}");
        assert!(rows.iter().any(|row| row.contains(marker)), "help missing on rendered screen: {rows:?}");
    }
    assert!(rows.iter().any(|row| row == MORE), "reprompt missing: {rows:?}");
    assert_eq!(output.text().matches("[resumed]").count(), 1);
}

#[tokio::test]
async fn more_help_finishes_before_reprompt_without_advancing_body() {
    help_reprompts_with_pcboard_paging(23).await;
}

#[tokio::test]
async fn more_help_small_pages_match_oracle_nested_pauses() {
    for page_len in [1, 2, 5] {
        help_reprompts_with_pcboard_paging(page_len).await;
    }
}

fn assert_cell_color(screen: &TextScreen, marker: &str, color: u8) {
    for y in 0..25 {
        if let Some(x) = row(screen, y).find(marker) {
            for x in x..x + marker.len() {
                assert_eq!(
                    screen.char_at((x as i32, y).into()).attribute.as_u8(IceMode::Blink),
                    color,
                    "{marker}: cell ({x}, {y})"
                );
            }
            return;
        }
    }
    panic!("missing colored marker {marker}");
}

#[tokio::test]
async fn more_and_press_enter_restore_distinct_user_and_sysop_foregrounds_and_backgrounds() {
    for prompt in [Prompt::More, Prompt::Enter] {
        let (mut state, mut peer) = paging_state(23, false).await;
        let (mut sysop_peer, sysop_connection) = ChannelConnection::create_pair();
        state.node_state.lock().await[state.node].as_mut().unwrap().sysop_connection = Some(sysop_connection);
        let caller_replies = [(prompt, "\r")];
        let monitor_replies = [(prompt, "")];
        let operation = exchange(&mut state, &mut peer, &caller_replies, async |state| {
            state.set_color(TerminalTarget::User, IcbColor::Dos(0x1E)).await.unwrap();
            state.set_color(TerminalTarget::Sysop, IcbColor::Dos(0x4B)).await.unwrap();
            state.print(TerminalTarget::Both, "[before]\r\n").await.unwrap();
            if prompt == Prompt::More {
                state.more_promt().await.unwrap();
            } else {
                state.press_enter().await.unwrap();
            }
            assert_eq!(state.user_screen.buffer.caret.attribute.as_u8(IceMode::Blink), 0x1E);
            assert_eq!(state.sysop_screen.buffer.caret.attribute.as_u8(IceMode::Blink), 0x4B);
            state.print(TerminalTarget::Both, "[after]").await.unwrap();
            assert_cell_color(&state.user_screen.buffer, "[after]", 0x1E);
            assert_cell_color(&state.sysop_screen.buffer, "[after]", 0x4B);
            state.node_state.lock().await[state.node]
                .as_mut()
                .unwrap()
                .sysop_connection
                .as_mut()
                .unwrap()
                .send(DONE.as_bytes())
                .await
                .unwrap();
        });
        // The monitor observes the same prompts, but only the caller supplies input.
        let (user, sysop) = tokio::time::timeout(DEADLINE, async { tokio::join!(operation, capture(&mut sysop_peer, &monitor_replies)) })
            .await
            .expect("color restoration exchange timed out");
        for (output, color) in [(user, 0x1E), (sysop, 0x4B)] {
            assert!(output.raw.contains(&0x1B), "test must exercise real ANSI output");
            let rendered = screen(&output.raw, 25);
            assert_cell_color(&rendered, "[before]", color);
            assert_cell_color(&rendered, "[after]", color);
            assert!(!row(&rendered, 1).contains(MORE));
            assert!(!row(&rendered, 1).contains(ENTER));
        }
    }
}
