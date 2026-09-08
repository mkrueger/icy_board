//! Command-level integration tests: real sessions, isolated persistent files,
//! prompt synchronization, and joined session threads (not output-idle guesses).
//! These exercise connection loss and reopen, not process/power-loss durability.
use std::{path::Path, sync::Arc, time::Duration};

use chrono::{Local, Utc};
use icy_board_engine::datetime::IcbTime;
use icy_board_engine::icy_board::{
    IcyBoard, IcyBoardSerializer,
    bbs::{BBS, BBSMessage},
    commands::CommandList,
    conferences::Conference,
    events::{BoardEvent, EventMode, event_window},
    icb_config::DisplayNewsBehavior,
    icb_text::{DEFAULT_DISPLAY_TEXT, IceText},
    message_area::{AreaList, MessageArea},
    sec_levels::SecurityLevel,
    state::IcyBoardState,
    user_base::{FSEMode, User, UserBase},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use jamjam::jam::{JamMessage, JamMessageBase};
use tokio::sync::{Mutex, oneshot};

use crate::bbs::{LoginOptions, internal_handle_client};
use crate::menu_runner::PcbBoardCommand;

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
const SHUTDOWN: &str = "[test-event-shutdown]";
const TIME_ADJUSTED: &str = "[test-event-time-adjusted]";
const EVENT_DENIED: &str = "[test-event-denied]";
const FIRST_NAME: &str = "[test-first-name]";
const REGISTER: &str = "[test-register]";

#[derive(Default)]
struct RecoveryMail(std::sync::Mutex<Vec<String>>);

#[async_trait::async_trait]
impl icy_board_engine::icy_board::password_recovery::MailSender for RecoveryMail {
    async fn send(&self, _: &icy_board_engine::icy_board::password_recovery::PasswordRecoveryConfig, to: &str, _: &str, body: String) -> Result<(), ()> {
        assert_eq!(to, "caller@example.invalid");
        self.0.lock().unwrap().push(body);
        Ok(())
    }
}

impl RecoveryMail {
    fn password(&self) -> String {
        self.0
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("Your temporary password: "))
            .unwrap()
            .to_string()
    }
}

fn password_recovery_board(root: &Path) -> (Arc<Mutex<IcyBoard>>, Arc<RecoveryMail>) {
    use icy_board_engine::icy_board::{
        icb_config::PasswordStorageMethod,
        password_recovery::{PasswordRecoveryConfig, RecoveryService},
        user_base::Password,
    };
    let mut board = board_at(root);
    board.config.system_control.password_storage_method = PasswordStorageMethod::Argon2;
    board.config.password_recovery = PasswordRecoveryConfig {
        enabled: true,
        smtp_host: "smtp.example.invalid".into(),
        sender: "bbs@example.invalid".into(),
        ..Default::default()
    };
    board.users[1].security_level = 10;
    board.users[1].email = "caller@example.invalid".into();
    board.users[1].password.password = Password::new_argon2("old-secret");
    board
        .default_display_text
        .update_record_number(IceText::WrongPasswordEntered as usize, "[recovery-wrong]")
        .unwrap();
    let mail = Arc::new(RecoveryMail::default());
    board.password_recovery_service = Arc::new(RecoveryService::new(mail.clone()));
    board.save_userbase().unwrap();
    (Arc::new(Mutex::new(board)), mail)
}

#[tokio::test]
async fn password_recovery_failed_three_sends_only_registered_mail_and_disconnects() {
    for answer in ["y\r", "Y\r"] {
        let dir = tempfile::tempdir().unwrap();
        let (board, mail) = password_recovery_board(dir.path());
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let mut session = Session::start_with_password(board.clone(), bbs, 1, "wrong").await;
        session.send("wrong\rwrong\r").await;
        session.expect("Send a temporary password").await;
        assert!(mail.0.lock().unwrap().is_empty());
        session.send(answer).await;
        let output = session.finish().await;
        assert_eq!(output.matches("[recovery-wrong]").count(), 3, "{output}");
        assert_eq!(output.matches("Send a temporary password").count(), 1, "{output}");
        assert!(output.contains("If eligible"), "{output}");
        assert!(!output.contains(COMMAND));
        assert_eq!(mail.0.lock().unwrap().len(), 1);
        assert!(board.lock().await.users[1].password.password.is_valid("old-secret"));
        assert!(board.lock().await.users[1].recovery.is_some());
    }
}

#[tokio::test]
async fn password_recovery_sysop_request_is_acknowledged_but_never_sent() {
    let dir = tempfile::tempdir().unwrap();
    let (board, mail) = password_recovery_board(dir.path());
    {
        let mut b = board.lock().await;
        b.users[1].security_level = b.config.sysop_command_level.sysop;
        b.save_userbase().unwrap();
    }
    let mut session = Session::start_with_password(board.clone(), Arc::new(Mutex::new(BBS::new(1))), 1, "wrong").await;
    session.send("wrong\rwrong\r").await;
    session.expect("Send a temporary password").await;
    session.send("y\r").await;
    let output = session.finish().await;
    assert!(output.contains("If eligible"), "{output}");
    assert!(!output.contains(COMMAND));
    assert!(mail.0.lock().unwrap().is_empty());
    assert!(board.lock().await.users[1].recovery.is_none());
}

#[tokio::test]
async fn password_recovery_without_valid_email_skips_offer_and_preserves_failure_comment() {
    for email in ["", "   ", "invalid", "one@example.invalid,two@example.invalid"] {
        let dir = tempfile::tempdir().unwrap();
        let (board, mail) = password_recovery_board(dir.path());
        {
            let mut b = board.lock().await;
            b.users[1].email = email.into();
            b.config.system_control.allow_password_failure_comment = true;
            b.save_userbase().unwrap();
        }
        let mut session = Session::start_with_password(board.clone(), Arc::new(Mutex::new(BBS::new(1))), 1, "wrong").await;
        session.send("wrong\rwrong\r").await;
        session.expect("leave a comment to the sysop").await;
        session.send("N\r").await;
        let output = session.finish().await;
        assert_eq!(output.matches("[recovery-wrong]").count(), 3, "{output}");
        assert!(!output.contains("Send a temporary password"), "{output}");
        assert!(!output.contains("If eligible"), "{output}");
        assert!(!output.contains(COMMAND), "{output}");
        assert!(mail.0.lock().unwrap().is_empty());
        let b = board.lock().await;
        assert!(b.users[1].recovery.is_none());
        assert!(b.users[1].recovery_issues.is_empty());
    }
}

#[tokio::test]
async fn password_recovery_restricted_change_persists_then_requires_normal_relogin() {
    let dir = tempfile::tempdir().unwrap();
    let (board, mail) = password_recovery_board(dir.path());
    let service = board.lock().await.password_recovery_service.clone();
    assert!(service.issue(&board, 1, Utc::now()).await.unwrap());
    let temporary = mail.password();
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let mut session = Session::start_with_password(board.clone(), bbs.clone(), 1, &temporary).await;
    session.expect("Temporary password verified").await;
    session.send("brand-new\rbrand-new\r").await;
    let output = session.finish().await;
    assert!(output.contains("Password saved"), "{output}");
    assert!(!output.contains(COMMAND));
    let users = UserBase::load(&dir.path().join("users.toml")).unwrap();
    assert!(users[1].password.password.is_valid("brand-new"));
    assert!(users[1].recovery.is_none());
    assert!(!std::fs::read_to_string(dir.path().join("users.toml")).unwrap().contains(&temporary));
    let mut relogin = Session::start_with_password(board, bbs, 1, "brand-new").await;
    relogin.expect(COMMAND).await;
    let output = relogin.bye().await;
    assert!(!output.contains("Temporary password verified"));
    assert!(!output.contains("Send a temporary password"));
}

#[tokio::test]
async fn password_recovery_cancellation_never_enters_menu_or_changes_password() {
    let dir = tempfile::tempdir().unwrap();
    let (board, mail) = password_recovery_board(dir.path());
    let service = board.lock().await.password_recovery_service.clone();
    service.issue(&board, 1, Utc::now()).await.unwrap();
    let mut session = Session::start_with_password(board.clone(), Arc::new(Mutex::new(BBS::new(1))), 1, &mail.password()).await;
    session.expect("Temporary password verified").await;
    session.send("\r").await;
    let output = session.finish().await;
    assert!(!output.contains(COMMAND));
    assert!(!output.contains("Password saved"));
    assert!(board.lock().await.users[1].password.password.is_valid("old-secret"));
}

#[tokio::test]
async fn password_recovery_no_keeps_the_existing_failure_comment_order() {
    let dir = tempfile::tempdir().unwrap();
    let (board, mail) = password_recovery_board(dir.path());
    board.lock().await.config.system_control.allow_password_failure_comment = true;
    let mut session = Session::start_with_password(board, Arc::new(Mutex::new(BBS::new(1))), 1, "wrong").await;
    session.send("wrong\rwrong\r").await;
    session.expect("Send a temporary password").await;
    session.send("N\r").await;
    session.expect("leave a comment to the sysop").await;
    session.send("N\r").await;
    let output = session.finish().await;
    assert!(output.find("Send a temporary password").unwrap() < output.find("leave a comment to the sysop").unwrap());
    assert!(mail.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn password_recovery_commit_failure_never_announces_success_or_enters_menu() {
    let dir = tempfile::tempdir().unwrap();
    let (board, mail) = password_recovery_board(dir.path());
    let service = board.lock().await.password_recovery_service.clone();
    service.issue(&board, 1, Utc::now()).await.unwrap();
    let mut session = Session::start_with_password(board.clone(), Arc::new(Mutex::new(BBS::new(1))), 1, &mail.password()).await;
    session.expect("Temporary password verified").await;
    board.lock().await.config.paths.user_file = dir.path().to_path_buf();
    session.send("brand-new\rbrand-new\r").await;
    let output = session.finish().await;
    assert!(!output.contains(COMMAND));
    assert!(!output.contains("Password saved"));
    assert!(board.lock().await.users[1].password.password.is_valid("old-secret"));
}

#[tokio::test]
async fn password_recovery_revokes_another_live_node_without_restoring_old_password() {
    let dir = tempfile::tempdir().unwrap();
    let (board, mail) = password_recovery_board(dir.path());
    let bbs = Arc::new(Mutex::new(BBS::new(2)));
    let mut old = Session::start_with_password(board.clone(), bbs.clone(), 1, "old-secret").await;
    old.expect(COMMAND).await;
    let service = board.lock().await.password_recovery_service.clone();
    service.issue(&board, 1, Utc::now()).await.unwrap();
    let mut recovery = Session::start_with_password(board.clone(), bbs, 1, &mail.password()).await;
    recovery.expect("Temporary password verified").await;
    recovery.send("brand-new\rbrand-new\r").await;
    let recovered = recovery.finish().await;
    assert!(recovered.contains("Password saved"), "{recovered}");
    old.finish().await;
    let users = UserBase::load(&dir.path().join("users.toml")).unwrap();
    assert!(users[1].password.password.is_valid("brand-new"));
    assert!(!users[1].password.password.is_valid("old-secret"));
    assert!(users[1].recovery.is_none());
}

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
        (IceText::TimeAdjusted, TIME_ADJUSTED),
        (IceText::DeniedAccessForEvent, EVENT_DENIED),
        (IceText::YourFirstName, FIRST_NAME),
        (IceText::Register, REGISTER),
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
        Self::start_with_password(board, bbs, user, "").await
    }

    async fn start_with_password(board: Arc<Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>, user: usize, password: &str) -> Self {
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
        session.send(&format!("{username}\r{password}\r")).await;
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
        let handle = {
            let bbs = self.bbs.lock().await;
            let mut nodes = nodes.lock().await;
            match nodes[self.node].as_mut() {
                Some(node) => Some(node.handle.take().expect("live node lost its session handle")),
                None => {
                    // A finished sibling's cleanup may already have joined this
                    // thread. Only a fully reclaimed node is acceptable here.
                    assert!(bbs.bbs_channels[self.node].is_none(), "reclaimed node retained its session sender");
                    None
                }
            }
        };
        if let Some(handle) = handle {
            handle.join().expect("session thread panicked").unwrap();
            // The harness owns this explicit join. A receiver restored into
            // NodeState is not a live writer once the thread has been joined;
            // remove that node as production's explicit join owners do.
            let mut bbs = self.bbs.lock().await;
            let mut nodes = nodes.lock().await;
            if let Some(node) = nodes[self.node].take() {
                assert!(node.handle.is_none(), "joined node acquired another session handle");
            }
            bbs.bbs_channels[self.node] = None;
        }
        let mut bbs = self.bbs.lock().await;
        bbs.clear_closed_connections().await;
        assert!(nodes.lock().await[self.node].is_none(), "finished node was not reclaimed");
        assert!(bbs.bbs_channels[self.node].is_none(), "finished session retained its writer channel");
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

async fn assert_no_session_writers(bbs: &Arc<Mutex<BBS>>) {
    let bbs = bbs.lock().await;
    assert!(
        bbs.open_connections.lock().await.iter().all(Option::is_none),
        "stale session thread or reserved node"
    );
    assert!(bbs.bbs_channels.iter().all(Option::is_none), "stale session sender");
}

async fn event_shutdown_sessions(editor: bool) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let before = snapshot(dir.path());
    let started = Utc::now();
    let board = Arc::new(Mutex::new(board_at(dir.path())));
    // Leave a free slot so admission denial cannot accidentally pass because
    // the board is full instead of because maintenance closed the gate.
    let bbs = Arc::new(Mutex::new(BBS::new(3)));
    let mut first = Session::start(board.clone(), bbs.clone(), 0).await;
    let mut second = Session::start(board.clone(), bbs.clone(), 1).await;
    assert_ne!(first.node, second.node);
    first.expect(COMMAND).await;
    if editor {
        second.compose("Event interrupted draft", "event must not save this draft").await;
    } else {
        second.expect(COMMAND).await;
    }
    let senders = {
        let mut bbs = bbs.lock().await;
        bbs.event_maintenance = true;
        assert!(bbs.admissions_closed());
        assert!(bbs.try_create_new_node(ConnectionType::Channel).await.is_none());
        let nodes = bbs.open_connections.lock().await;
        assert_eq!(nodes[first.node].as_ref().unwrap().cur_user, 0);
        assert_eq!(nodes[second.node].as_ref().unwrap().cur_user, 1);
        [first.node, second.node].map(|node| bbs.bbs_channels[node].as_ref().unwrap().clone())
    };
    // No BYE, carrier loss, or extra key is sent: blocked session input must
    // service BBSMessage::Shutdown itself, including inside the editor.
    for sender in &senders {
        sender.try_send(BBSMessage::Shutdown(SHUTDOWN.to_string())).unwrap();
    }
    if editor {
        // Exercise automatic joining, not just the harness's explicit join.
        // Thread completion is the barrier, never an output-idle delay.
        tokio::time::timeout(DEADLINE, async {
            loop {
                let nodes = bbs.lock().await.open_connections.clone();
                let finished = nodes.lock().await.iter().flatten().all(|node| node.handle.as_ref().unwrap().is_finished());
                if finished {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("event shutdown left a session thread running");
        bbs.lock().await.clear_closed_connections().await;
        assert_no_session_writers(&bbs).await;
    }
    let first_output = first.finish().await;
    let second_output = second.finish().await;
    for output in [&first_output, &second_output] {
        assert_eq!(output.matches(SHUTDOWN).count(), 1, "{output}");
        assert!(!output.contains(SAVED), "shutdown saved an unsolicited message: {output}");
    }
    assert!(senders.iter().all(|sender| sender.is_closed()), "a stale session still owns its receiver");
    drop(senders);
    assert_no_session_writers(&bbs).await;
    assert!(bbs.lock().await.admissions_closed(), "session cleanup reopened event admission");
    drop(board);

    assert_eq!(snapshot(dir.path()), before, "shutdown changed the durable message base");
    let users = UserBase::load(&dir.path().join("users.toml")).unwrap();
    assert_eq!(users.len(), 2);
    for (index, name) in ["SYSOP", "SECOND"].iter().enumerate() {
        assert_eq!(users[index].name, *name);
        assert_eq!(users[index].stats.num_times_on, 1, "shutdown did not persist {name}");
        assert!(users[index].stats.last_on >= started, "shutdown did not save {name}'s login timestamp");
        assert_eq!(users[index].stats.messages_left, 0);
    }
    assert_eq!(
        messages(dir.path()),
        vec![("Existing message".to_string(), "existing durable body".to_string())]
    );

    // Reopen from disk, not the old board's caches, and prove both reading and
    // writing still work after the event has joined every previous writer.
    bbs.lock().await.event_maintenance = false;
    let fresh_board = Arc::new(Mutex::new(board_at(dir.path())));
    let mut reader = Session::start(fresh_board.clone(), bbs.clone(), 0).await;
    assert_eq!(reader.node, 0);
    reader.expect(COMMAND).await;
    reader.send("R\r1\r").await;
    reader.expect("existing durable body").await;
    reader.peer.shutdown().await.unwrap();
    reader.finish().await;
    let mut writer = Session::start(fresh_board, bbs.clone(), 1).await;
    writer.compose("After event", "fresh session durable body").await;
    writer.save().await;
    writer.bye().await;
    assert_no_session_writers(&bbs).await;
    let stored = messages(dir.path());
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[1].0, "After event");
    assert_eq!(stored[1].1.trim(), "fresh session durable body");
    let users = UserBase::load(&dir.path().join("users.toml")).unwrap();
    assert_eq!(users[0].stats.num_times_on, 2);
    assert_eq!(users[1].stats.num_times_on, 2);
    assert_eq!(users[0].stats.messages_left, 0);
    assert_eq!(users[1].stats.messages_left, 1);
}

#[tokio::test]
async fn event_shutdown_joins_two_callers_at_command_prompts_and_reopens_mail() {
    event_shutdown_sessions(false).await;
}

#[tokio::test]
async fn event_shutdown_joins_prompt_and_editor_callers_without_saving_the_draft() {
    event_shutdown_sessions(true).await;
}

#[tokio::test]
async fn login_announces_time_adjustment_for_a_future_fixed_event_only() {
    for (mode, time_per_day, adjusted) in [
        (EventMode::Fixed, 120, true),
        (EventMode::Slide, 120, false),
        (EventMode::Idle, 120, false),
        // A caller whose ordinary allowance ends before suspension is unaffected.
        (EventMode::Fixed, 10, false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut board = board_at(dir.path());
        board.sec_levels.push(SecurityLevel {
            security: 255,
            time_per_day,
            ..Default::default()
        });
        board.config.event.enabled = true;
        board.config.event.suspend_minutes = 5;
        let run_at = Local::now() + chrono::Duration::minutes(30);
        let event = BoardEvent {
            description: "Login adjustment regression".to_string(),
            time: IcbTime::parse(&run_at.format("%H:%M:%S").to_string()),
            mode,
            ..Default::default()
        };
        board.events.push(event.clone());
        let mut bbs = BBS::new(1);
        bbs.event_window = Some(event_window(&board.config.event, event, run_at));
        let bbs = Arc::new(Mutex::new(bbs));
        let mut session = Session::start(Arc::new(Mutex::new(board)), bbs.clone(), 0).await;
        if adjusted {
            session.expect(TIME_ADJUSTED).await;
            session.expect(CONTINUE).await;
            session.send("\r").await;
        }
        session.expect(COMMAND).await;
        // Inspect login output before logoff can add any unrelated text.
        let login_output = String::from_utf8_lossy(&session.output).into_owned();
        session.bye().await;
        assert_no_session_writers(&bbs).await;
        assert_eq!(
            login_output.matches(TIME_ADJUSTED).count(),
            usize::from(adjusted),
            "{mode:?}, allowance {time_per_day}: {login_output}"
        );
    }
}

#[tokio::test]
async fn suspended_event_denies_new_user_before_registration() {
    let dir = tempfile::tempdir().unwrap();
    let mut board = board_at(dir.path());
    let before = std::fs::read(&board.config.paths.user_file).unwrap();
    board.config.event.enabled = true;
    board.config.event.suspend_minutes = 10;
    let run_at = Local::now() + chrono::Duration::minutes(5);
    let event = BoardEvent {
        time: IcbTime::parse(&run_at.format("%H:%M:%S").to_string()),
        mode: EventMode::Fixed,
        ..Default::default()
    };
    let window = event_window(&board.config.event, event.clone(), run_at);
    assert!(window.is_suspended(&Local::now()));
    board.events.push(event);
    let board = Arc::new(Mutex::new(board));
    let bbs = Arc::new(Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (mut peer, connection) = ChannelConnection::create_pair();
    let state = IcyBoardState::new(bbs.clone(), board.clone(), nodes, node, Box::new(connection)).await;
    // Model a connection accepted just before suspension. Bypass listener
    // admission deliberately to test login's own PCBoard event denial.
    bbs.lock().await.event_window = Some(window);
    let mut command = PcbBoardCommand::new(state);
    peer.send(b"NEW PERSON\r\rY\r").await.unwrap();
    let accepted = tokio::time::timeout(DEADLINE, command.login(true))
        .await
        .expect("suspended login waited for registration input")
        .unwrap();
    let logged_off = command.state.session.request_logoff;
    let has_user = command.state.session.current_user.is_some();
    drop(command);
    let mut output = Vec::new();
    tokio::time::timeout(DEADLINE, async {
        let mut bytes = [0; 4096];
        loop {
            let size = peer.read(&mut bytes).await.unwrap();
            if size == 0 {
                break;
            }
            output.extend_from_slice(&bytes[..size]);
        }
    })
    .await
    .expect("denied login retained its connection");
    // This direct command never had a session thread. Release its reservation
    // only after dropping the command; automatic cleanup must not mistake an
    // unstarted reservation for a completed thread.
    {
        let bbs = bbs.lock().await;
        let mut nodes = bbs.open_connections.lock().await;
        let node = nodes[node].as_mut().unwrap();
        assert!(node.handle.is_none());
        drop(node.bbs_channel.take());
    }
    bbs.lock().await.clear_closed_connections().await;
    assert_no_session_writers(&bbs).await;
    let output = String::from_utf8(output).unwrap();
    assert!(!accepted, "new user was admitted during suspension");
    assert!(logged_off);
    assert!(!has_user);
    assert_eq!(output.matches(EVENT_DENIED).count(), 1, "{output}");
    for forbidden in [FIRST_NAME, REGISTER, COMMAND, TIME_ADJUSTED] {
        assert!(!output.contains(forbidden), "suspended login reached {forbidden}: {output}");
    }
    assert_eq!(board.lock().await.users.len(), 2);
    assert_eq!(
        std::fs::read(dir.path().join("users.toml")).unwrap(),
        before,
        "denied registration changed the user base"
    );
}
