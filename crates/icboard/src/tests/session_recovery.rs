//! Command-level integration tests: real sessions, isolated persistent files,
//! prompt synchronization, and joined session threads (not output-idle guesses).
//! These exercise connection loss and reopen, not process/power-loss durability.
use std::{path::Path, sync::Arc, time::Duration};

use icy_board_engine::icy_board::{
    IcyBoard, IcyBoardSerializer,
    bbs::BBS,
    commands::CommandList,
    conferences::Conference,
    icb_config::DisplayNewsBehavior,
    icb_text::{DEFAULT_DISPLAY_TEXT, IceText},
    message_area::{AreaList, MessageArea},
    state::IcyBoardState,
    user_base::{FSEMode, User, UserBase},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use jamjam::jam::{JamMessage, JamMessageBase};
use tokio::sync::{Mutex, oneshot};

use crate::bbs::{LoginOptions, internal_handle_client};

const DEADLINE: Duration = Duration::from_secs(10);
const COMMAND: &str = "[test-command]";
const TO: &str = "[test-to]";
const SUBJECT: &str = "[test-subject]";
const SECURITY: &str = "[test-security]";
const EDITOR: &str = "[test-editor]";
const SAVED: &str = "[test-saved]";
const BASE_ERROR: &str = "[test-base-error]";
const ABORT: &str = "[test-abort]";
const ABORTED: &str = "[test-aborted]";
const CONTINUE: &str = "[test-continue]";
const FULL_SCREEN: &str = "[test-full-screen]";

fn board_at(root: &Path) -> IcyBoard {
    let mut board = IcyBoard::new();
    board.root_path = root.to_path_buf();
    board.file_name = root.join("icboard.toml");
    board.config.paths.user_file = root.join("users.toml");
    board.config.paths.statistics_file = root.join("statistics.toml");
    board.resolve_paths();
    board.config.switches.display_news_behavior = DisplayNewsBehavior::Never;
    board.config.switches.scan_new_blt = false;
    board.config.system_control.confirm_caller_name = false;
    board.config.message.disable_message_scan_prompt = true;
    board.config.message.prompt_to_read_mail = false;
    board.commands = CommandList::new();
    board.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    // Only customize display text, as a sysop can. Commands and their parsing
    // remain unchanged; markers cannot match echoed input or depend on locale.
    for (id, marker) in [
        (IceText::CommandPrompt, COMMAND),
        (IceText::MessageTo, TO),
        (IceText::MessageSubject, SUBJECT),
        (IceText::MessageSecurity, SECURITY),
        (IceText::TextEntryCommand, EDITOR),
        (IceText::SavingMessage, SAVED),
        (IceText::MessageBaseError, BASE_ERROR),
        (IceText::MessageAbort, ABORT),
        (IceText::MessageAborted, ABORTED),
        (IceText::PressEnter, CONTINUE),
        (IceText::EscToExit, FULL_SCREEN),
    ] {
        board.default_display_text.update_record_number(id as usize, marker).unwrap();
    }
    if board.config.paths.user_file.exists() {
        board.users = UserBase::load(&board.config.paths.user_file).unwrap();
    } else {
        for name in ["SYSOP", "SECOND"] {
            let mut user = User {
                name: name.to_string(),
                security_level: 255,
                page_len: 0,
                ..Default::default()
            };
            user.flags.fse_mode = FSEMode::No;
            board.users.new_user(user);
        }
        board.save_userbase().unwrap();
    }
    board.conferences.push(Conference {
        name: "Recovery test".to_string(),
        areas: Some(Arc::new(AreaList::new(vec![MessageArea {
            name: "General".to_string(),
            path: root.join("general"),
            ..Default::default()
        }]))),
        ..Default::default()
    });
    board
}

struct Session {
    peer: ChannelConnection,
    bbs: Arc<Mutex<BBS>>,
    node: usize,
    done: oneshot::Receiver<Result<(), String>>,
    output: Vec<u8>,
    consumed: usize,
}

impl Session {
    async fn start(board: Arc<Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>, user: usize) -> Self {
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let username = board.lock().await.users[user].name.clone();
        let state = IcyBoardState::new(bbs.clone(), board, nodes.clone(), node, Box::new(connection)).await;
        let (start_tx, start_rx) = oneshot::channel();
        let (done_tx, done) = oneshot::channel();
        let handle = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    start_rx.await.unwrap();
                    let result = internal_handle_client(
                        state,
                        Some(LoginOptions {
                            login_sysop: false,
                            ppe: None,
                            local: true,
                        }),
                        "",
                    )
                    .await;
                    let _ = done_tx.send(result.map_err(|error| error.to_string()));
                });
            Ok(())
        });
        nodes.lock().await[node].as_mut().unwrap().handle = Some(handle);
        start_tx.send(()).unwrap();
        let mut session = Self {
            peer,
            bbs,
            node,
            done,
            output: Vec::new(),
            consumed: 0,
        };
        // Real login using fixture accounts with empty passwords, then normal
        // PCBoard commands over the connection (no keyboard stuffing).
        session.send(&format!("{username}\r\r")).await;
        session
    }

    async fn send(&mut self, input: &str) {
        self.peer.send(input.as_bytes()).await.unwrap();
    }

    async fn expect(&mut self, marker: &str) {
        let result = tokio::time::timeout(DEADLINE, async {
            loop {
                if let Some(offset) = self.output[self.consumed..]
                    .windows(marker.len())
                    .position(|window| window == marker.as_bytes())
                {
                    self.consumed += offset + marker.len();
                    return;
                }
                let mut bytes = [0; 4096];
                let size = self.peer.read(&mut bytes).await.unwrap();
                assert_ne!(size, 0, "EOF waiting for {marker}: {}", String::from_utf8_lossy(&self.output));
                self.output.extend_from_slice(&bytes[..size]);
            }
        })
        .await;
        assert!(result.is_ok(), "timeout waiting for {marker}: {}", String::from_utf8_lossy(&self.output));
    }

    async fn compose(&mut self, subject: &str, body: &str) {
        self.enter(subject).await;
        self.send(&format!("N\r{body}\r\r")).await;
        self.expect(EDITOR).await;
    }

    async fn enter(&mut self, subject: &str) {
        self.expect(COMMAND).await;
        self.send("E\r").await;
        self.expect(TO).await;
        self.send("ALL\r").await;
        self.expect(SUBJECT).await;
        self.send(&format!("{subject}\r")).await;
        self.expect(SECURITY).await;
    }

    async fn save(&mut self) {
        self.send("S\r").await;
        self.expect(SAVED).await;
        self.expect(CONTINUE).await;
        self.send("\r").await;
        self.expect(COMMAND).await;
    }

    async fn finish(mut self) -> String {
        let result = tokio::time::timeout(DEADLINE, &mut self.done)
            .await
            .expect("session did not exit after logoff/EOF")
            .unwrap();
        result.expect("session returned an error");
        // Completion means the session's connection has been dropped, so drain
        // all remaining output before checking it. No quiet-period heuristic.
        tokio::time::timeout(DEADLINE, async {
            let mut bytes = [0; 4096];
            loop {
                let size = self.peer.read(&mut bytes).await.unwrap();
                if size == 0 {
                    break;
                }
                self.output.extend_from_slice(&bytes[..size]);
            }
        })
        .await
        .expect("session connection was not released");
        let nodes = self.bbs.lock().await.open_connections.clone();
        let handle = nodes.lock().await[self.node].as_mut().unwrap().handle.take().unwrap();
        handle.join().expect("session thread panicked").unwrap();
        self.bbs.lock().await.clear_closed_connections().await;
        assert!(nodes.lock().await[self.node].is_none(), "finished node was not reclaimed");
        String::from_utf8(self.output).unwrap()
    }

    async fn bye(mut self) -> String {
        self.send("BYE\r").await;
        self.finish().await
    }
}

fn seed(root: &Path) {
    let mut base = JamMessageBase::create(root.join("general")).unwrap();
    base.write_message(
        &JamMessage::default()
            .with_from("SYSOP".into())
            .with_to("ALL".into())
            .with_subject("Existing message".into())
            .with_text("existing durable body".into()),
    )
    .unwrap();
    base.write_jhr_header().unwrap();
}

fn messages(root: &Path) -> Vec<(String, String)> {
    let base = JamMessageBase::open(root.join("general")).unwrap();
    let messages: Vec<_> = base
        .messages()
        .map(|header| {
            let header = header.unwrap();
            (header.subject().unwrap().to_string(), base.read_message_text(&header).unwrap().to_string())
        })
        .collect();
    assert_eq!(base.active_messages() as usize, messages.len());
    messages
}

fn snapshot(root: &Path) -> Vec<Vec<u8>> {
    ["jhr", "jdx", "jdt"]
        .iter()
        .map(|ext| std::fs::read(root.join("general").with_extension(ext)).unwrap())
        .collect()
}

#[tokio::test]
async fn saved_message_survives_session_exit_and_fresh_board() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let board = Arc::new(Mutex::new(board_at(dir.path())));
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let mut session = Session::start(board.clone(), bbs.clone(), 0).await;
    session.compose("Saved subject", "saved durable body").await;
    session.save().await;
    session.bye().await;
    drop(board);
    let stored = messages(dir.path());
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[1].0, "Saved subject");
    assert_eq!(stored[1].1.trim(), "saved durable body");
    assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap()[0].stats.messages_left, 1);
    let mut reader = Session::start(Arc::new(Mutex::new(board_at(dir.path()))), bbs, 0).await;
    assert_eq!(reader.node, 0, "the finished node should be reusable");
    reader.expect(COMMAND).await;
    reader.send("R\r2\r").await;
    reader.expect("saved durable body").await;
    reader.peer.shutdown().await.unwrap();
    reader.finish().await;
}

#[tokio::test]
async fn abort_discards_draft_and_preserves_existing_base() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let before = snapshot(dir.path());
    let mut session = Session::start(Arc::new(Mutex::new(board_at(dir.path()))), Arc::new(Mutex::new(BBS::new(1))), 0).await;
    session.compose("Aborted subject", "aborted draft body").await;
    session.send("A\r").await;
    session.expect(ABORT).await;
    session.send("Y\r").await;
    session.expect(ABORTED).await;
    session.expect(COMMAND).await;
    let output = session.bye().await;
    assert!(!output.contains(SAVED));
    assert_eq!(snapshot(dir.path()), before);
    assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap()[0].stats.messages_left, 0);
}

#[tokio::test]
async fn carrier_loss_at_editor_prompt_discards_draft_and_releases_node() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let before = snapshot(dir.path());
    let board = Arc::new(Mutex::new(board_at(dir.path())));
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let mut session = Session::start(board.clone(), bbs.clone(), 0).await;
    session.compose("Disconnected subject", "unsaved carrier loss body").await;
    session.peer.shutdown().await.unwrap();
    let output = session.finish().await;
    assert!(!output.contains(SAVED));
    assert_eq!(snapshot(dir.path()), before);
    assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap()[0].stats.messages_left, 0);
    let mut next = Session::start(board, bbs, 1).await;
    next.expect(COMMAND).await;
    next.bye().await;
}

#[tokio::test]
async fn unavailable_message_base_reports_failure_without_crediting_a_post() {
    let dir = tempfile::tempdir().unwrap();
    let mut board = board_at(dir.path());
    // ENOTDIR is deterministic even under root; chmod-based failures are not.
    let blocker = dir.path().join("not-a-directory");
    std::fs::write(&blocker, b"do not overwrite").unwrap();
    let mut areas = board.conferences[0].areas.as_deref().unwrap().clone();
    areas[0].path = blocker.join("general");
    board.conferences[0].areas = Some(Arc::new(areas));
    let mut session = Session::start(Arc::new(Mutex::new(board)), Arc::new(Mutex::new(BBS::new(1))), 0).await;
    session.compose("Failed subject", "failed save body").await;
    session.send("S\r").await;
    session.expect(BASE_ERROR).await;
    session.expect(CONTINUE).await;
    session.send("\r").await;
    session.expect(COMMAND).await;
    let output = session.bye().await;
    assert!(!output.contains(SAVED));
    assert_eq!(std::fs::read(blocker).unwrap(), b"do not overwrite");
    assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap()[0].stats.messages_left, 0);
}

#[tokio::test]
async fn overlapping_callers_save_distinct_messages_to_the_same_base() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let board = Arc::new(Mutex::new(board_at(dir.path())));
    let bbs = Arc::new(Mutex::new(BBS::new(2)));
    let mut first = Session::start(board.clone(), bbs.clone(), 0).await;
    let mut second = Session::start(board, bbs, 1).await;
    assert_ne!(first.node, second.node);
    tokio::join!(
        first.compose("First caller subject", "first caller durable body"),
        second.compose("Second caller subject", "second caller durable body")
    );
    // Both sessions are in the editor before either is allowed to save.
    tokio::join!(first.save(), second.save());
    first.bye().await;
    second.bye().await;
    let stored = messages(dir.path());
    assert_eq!(stored.len(), 3);
    for (subject, body) in [
        ("First caller subject", "first caller durable body"),
        ("Second caller subject", "second caller durable body"),
    ] {
        assert_eq!(stored.iter().filter(|(s, b)| s == subject && b.trim() == body).count(), 1);
    }
    let users = UserBase::load(&dir.path().join("users.toml")).unwrap();
    assert_eq!(users[0].stats.messages_left, 1);
    assert_eq!(users[1].stats.messages_left, 1);
}

#[tokio::test]
async fn carrier_loss_while_typing_a_line_discards_the_partial_body() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let before = snapshot(dir.path());
    let mut session = Session::start(Arc::new(Mutex::new(board_at(dir.path()))), Arc::new(Mutex::new(BBS::new(1))), 0).await;
    session.enter("Partial line").await;
    session.send("N\rpartial-line-marker").await;
    session.expect("partial-line-marker").await;
    session.peer.shutdown().await.unwrap();
    let output = session.finish().await;
    assert!(!output.contains(SAVED));
    assert_eq!(snapshot(dir.path()), before);
    assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap()[0].stats.messages_left, 0);
}

#[tokio::test]
async fn carrier_loss_in_full_screen_editor_discards_the_partial_body() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let before = snapshot(dir.path());
    let mut board = board_at(dir.path());
    board.users[0].flags.fse_mode = FSEMode::Yes;
    board.users[0].flags.use_graphics = true;
    let mut session = Session::start(Arc::new(Mutex::new(board)), Arc::new(Mutex::new(BBS::new(1))), 0).await;
    session.enter("Partial screen").await;
    session.send("N\r").await;
    session.expect(FULL_SCREEN).await;
    // The full-screen editor emits cursor controls between typed characters.
    // This character occurs in neither the fixture nor the remaining footer.
    session.send("~").await;
    session.expect("~").await;
    session.peer.shutdown().await.unwrap();
    let output = session.finish().await;
    assert!(!output.contains(SAVED));
    assert_eq!(snapshot(dir.path()), before);
    assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap()[0].stats.messages_left, 0);
}
