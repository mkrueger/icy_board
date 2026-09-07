//! Real login/registration and command-loop accounting, with isolated durable
//! files and joined sessions. Carrier tests cover EOF/socket errors, not crashes.
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use icy_board_engine::icy_board::{
    IcyBoard, IcyBoardSerializer,
    accounting_cfg::AccountingConfig,
    bbs::BBS,
    commands::CommandList,
    conferences::Conference,
    icb_config::DisplayNewsBehavior,
    icb_text::{DEFAULT_DISPLAY_TEXT, IceText},
    message_area::{AreaList, MessageArea},
    pcb::user_inf::AccountUserInf,
    sec_levels::SecurityLevel,
    state::{IcyBoardState, PPEExecute},
    user_base::{FSEMode, Password, User, UserBase},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use jamjam::jam::{JamMessage, JamMessageBase};
use tokio::sync::{Mutex, oneshot};

use crate::bbs::{LoginOptions, internal_handle_client};

const DEADLINE: Duration = Duration::from_secs(15);
const COMMAND: &str = "[account-command]";
const CONTINUE: &str = "[account-continue]";
const INFO: &str = "[account-info]";
const WARNING: &str = "[account-warning]";
const INTRO: &str = "[account-intro]";
const SOCKET_ERROR: &str = "accounting-test socket shutdown failed";

fn board_at(root: &Path) -> IcyBoard {
    let mut board = IcyBoard::new();
    board.root_path = root.to_path_buf();
    board.file_name = root.join("icboard.toml");
    board.config.paths.user_file = root.join("users.toml");
    board.config.paths.group_file = root.join("groups");
    board.config.paths.statistics_file = root.join("statistics.toml");
    board.resolve_paths();
    board.config.switches.display_news_behavior = DisplayNewsBehavior::Never;
    board.config.switches.scan_new_blt = false;
    board.config.switches.exclude_local_calls_stats = false;
    board.config.system_control.confirm_caller_name = false;
    board.config.system_control.allow_password_failure_comment = false;
    board.config.message.disable_message_scan_prompt = true;
    board.config.message.prompt_to_read_mail = false;
    board.commands = CommandList::new();
    board.default_display_text = DEFAULT_DISPLAY_TEXT.clone();
    for (id, marker) in [
        (IceText::YourFirstName, "[account-name]"),
        (IceText::YourPassword, "[account-password]"),
        (IceText::ReEnterName, "[account-reenter]"),
        (IceText::Register, "[account-register]"),
        (IceText::NewPassword, "[account-new-password]"),
        (IceText::ReEnterPassword, "[account-confirm-password]"),
        (IceText::CommandPrompt, COMMAND),
        (IceText::PressEnter, CONTINUE),
        (IceText::MessageTo, "[account-to]"),
        (IceText::MessageSubject, "[account-subject]"),
        (IceText::MessageSecurity, "[account-security]"),
        (IceText::TextEntryCommand, "[account-editor]"),
        (IceText::SavingMessage, "[account-saved]"),
    ] {
        board.default_display_text.update_record_number(id as usize, marker).unwrap();
    }
    let settings = &mut board.config.new_user_settings;
    settings.sec_level = 100;
    settings.ask_city_or_state = false;
    settings.ask_business_phone = false;
    settings.ask_home_phone = false;
    settings.ask_comment = false;
    settings.ask_clr_msg = false;
    settings.ask_xfer_protocol = false;
    settings.ask_fse = false;
    settings.ask_use_short_descr = false;
    board.config.accounting.enabled = true;
    board.config.accounting.use_money = false;
    board.config.accounting.tracking_file = root.join("account.log");
    board.config.accounting.info_file = root.join("account-info");
    board.config.accounting.warning_file = root.join("account-warning");
    board.config.accounting.logoff_file = Default::default();
    board.config.accounting.accounting_config = Some(AccountingConfig {
        new_user_balance: 40.0,
        charge_per_logon: 3.0,
        charge_per_msg_written: 5.0,
        charge_per_msg_read: 2.0,
        warn_level: 100.0,
        ..Default::default()
    });
    board.sec_levels.push(SecurityLevel {
        security: 100,
        is_enabled: true,
        time_per_day: 1000,
        ..Default::default()
    });
    std::fs::write(
        &board.config.accounting.info_file,
        format!("{INFO} @CREDSTART@/@CREDNOW@/@CREDUSED@/@CREDLEFT@\r\n"),
    )
    .unwrap();
    std::fs::write(&board.config.accounting.warning_file, format!("{WARNING}\r\n")).unwrap();
    std::fs::write(root.join("intro"), format!("{INTRO}\r\n")).unwrap();
    if board.config.paths.user_file.exists() {
        board.users = UserBase::load(&board.config.paths.user_file).unwrap();
    } else {
        let mut user = User {
            name: "ACCOUNT USER".into(),
            security_level: 100,
            page_len: 0,
            account: Some(AccountUserInf {
                starting_balance: 100.0,
                drop_sec_level: 10,
                ..Default::default()
            }),
            ..Default::default()
        };
        user.password.password = Password::new_plaintext("SECRET").unwrap();
        user.flags.fse_mode = FSEMode::No;
        board.users.new_user(user);
        board.save_userbase().unwrap();
    }
    board.conferences.push(Conference {
        name: "Accounting".into(),
        users_menu: root.join("intro"),
        sysop_menu: root.join("intro"),
        areas: Some(Arc::new(AreaList::new(vec![MessageArea {
            name: "General".into(),
            path: root.join("general"),
            ..Default::default()
        }]))),
        ..Default::default()
    });
    if !root.join("general.jhr").exists() {
        let mut base = JamMessageBase::create(root.join("general")).unwrap();
        base.write_message(
            &JamMessage::default()
                .with_from("SYSOP".into())
                .with_to("ALL".into())
                .with_subject("Existing message".into())
                .with_text("[account-message-body]".into()),
        )
        .unwrap();
        base.write_jhr_header().unwrap();
    }
    board
}

// All I/O still uses a real ChannelConnection. A switchable shutdown failure
// models a broken transport without relying on timing a TCP reset.
struct BrokenShutdown {
    connection: ChannelConnection,
    fail: Arc<AtomicBool>,
}

#[async_trait]
impl Connection for BrokenShutdown {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, bytes: &mut [u8]) -> icy_net::Result<usize> {
        self.connection.read(bytes).await
    }

    async fn try_read(&mut self, bytes: &mut [u8]) -> icy_net::Result<usize> {
        self.connection.try_read(bytes).await
    }

    async fn send(&mut self, bytes: &[u8]) -> icy_net::Result<()> {
        self.connection.send(bytes).await
    }

    async fn shutdown(&mut self) -> icy_net::Result<()> {
        self.connection.shutdown().await?;
        if self.fail.load(Ordering::SeqCst) {
            return Err(SOCKET_ERROR.into());
        }
        Ok(())
    }
}

struct Session {
    peer: ChannelConnection,
    done: oneshot::Receiver<Result<(), String>>,
    thread: std::thread::JoinHandle<()>,
    fail_shutdown: Arc<AtomicBool>,
    output: Vec<u8>,
    consumed: usize,
}

impl Session {
    async fn start(board: Arc<Mutex<IcyBoard>>, options: LoginOptions) -> Self {
        let bbs = Arc::new(Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (peer, connection) = ChannelConnection::create_pair();
        let fail_shutdown = Arc::new(AtomicBool::new(false));
        let state = IcyBoardState::new(
            bbs,
            board,
            nodes,
            node,
            Box::new(BrokenShutdown {
                connection,
                fail: fail_shutdown.clone(),
            }),
        )
        .await;
        let (tx, done) = oneshot::channel();
        let thread = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let result = internal_handle_client(state, Some(options), "").await;
                let _ = tx.send(result.map_err(|error| error.to_string()));
            });
        });
        Self {
            peer,
            done,
            thread,
            fail_shutdown,
            output: Vec::new(),
            consumed: 0,
        }
    }

    async fn login(board: IcyBoard) -> Self {
        let mut session = Self::start(Arc::new(Mutex::new(board)), options(false)).await;
        session.authenticate("ACCOUNT USER", "SECRET").await;
        session
    }

    async fn authenticate(&mut self, name: &str, password: &str) {
        self.expect("[account-name]").await;
        self.send(&format!("{name}\r")).await;
        self.expect("[account-password]").await;
        self.send(&format!("{password}\r")).await;
        self.expect(COMMAND).await;
    }

    async fn send(&mut self, input: &str) {
        self.peer.send(input.as_bytes()).await.unwrap();
    }

    async fn expect(&mut self, marker: &str) {
        let result = tokio::time::timeout(DEADLINE, async {
            loop {
                let unread = &self.output[self.consumed..];
                if let Some(offset) = unread.windows(marker.len()).position(|window| window == marker.as_bytes()) {
                    self.consumed += offset + marker.len();
                    return;
                }
                if let Some(offset) = unread.windows(CONTINUE.len()).position(|window| window == CONTINUE.as_bytes()) {
                    self.consumed += offset + CONTINUE.len();
                    self.send("\r").await;
                    continue;
                }
                let mut bytes = [0; 4096];
                let size = self.peer.read(&mut bytes).await.unwrap();
                if size == 0 {
                    let result = tokio::time::timeout(DEADLINE, &mut self.done).await;
                    panic!("EOF waiting for {marker}, result {result:?}: {}", String::from_utf8_lossy(&self.output));
                }
                self.output.extend_from_slice(&bytes[..size]);
            }
        })
        .await;
        assert!(result.is_ok(), "timeout waiting for {marker}: {}", String::from_utf8_lossy(&self.output));
    }

    async fn write_message(&mut self) {
        self.send("E\r").await;
        self.expect("[account-to]").await;
        self.send("ALL\r").await;
        self.expect("[account-subject]").await;
        self.send("Charged message\r").await;
        self.expect("[account-security]").await;
        self.send("N\rA billed body\r\r").await;
        self.expect("[account-editor]").await;
        self.send("S\r").await;
        self.expect("[account-saved]").await;
        self.expect(COMMAND).await;
    }

    async fn finish(mut self) -> (Result<(), String>, String) {
        let result = tokio::time::timeout(DEADLINE, &mut self.done).await.expect("session did not exit").unwrap();
        self.thread.join().expect("session thread panicked");
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
        .expect("session retained its connection");
        (result, String::from_utf8(self.output).unwrap())
    }

    async fn bye(mut self) -> String {
        self.send("BYE\r").await;
        let (result, output) = self.finish().await;
        result.unwrap();
        output
    }
}

fn options(login_sysop: bool) -> LoginOptions {
    LoginOptions {
        login_sysop,
        ppe: None,
        local: true,
    }
}

fn account(root: &Path, user: usize) -> AccountUserInf {
    UserBase::load(&root.join("users.toml")).unwrap()[user].account.clone().unwrap()
}

fn tracking(root: &Path, activity: &str) -> usize {
    std::fs::read_to_string(root.join("account.log"))
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains(activity))
        .count()
}

fn assert_login_display(output: &str, credits: &str) {
    assert_eq!(output.matches(INFO).count(), 1, "{output}");
    assert_eq!(output.matches(WARNING).count(), 1, "{output}");
    assert!(output.contains(credits), "{output}");
    assert!(output.find(INFO).unwrap() < output.find(WARNING).unwrap(), "{output}");
    assert!(output.find(WARNING).unwrap() < output.find(INTRO).unwrap(), "{output}");
    assert!(output.find(INTRO).unwrap() < output.find(COMMAND).unwrap(), "{output}");
}

#[tokio::test]
async fn accounting_login_command_logout_and_reconnect_do_not_replay_charges() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::login(board_at(dir.path())).await;
    session.write_message().await;
    let output = session.bye().await;
    assert_login_display(&output, "100/3/3/97");
    let first = account(dir.path(), 0);
    assert_eq!(first.debit_call, 3.0);
    assert_eq!(first.debit_msg_write, 5.0);
    assert_eq!(first.balance(false, 0.0), 92.0);
    assert_eq!(tracking(dir.path(), "LOGON"), 1);
    assert_eq!(tracking(dir.path(), "MSG WRITE"), 1);

    // Re-open the actual user file, not the first session's in-memory board.
    let session = Session::login(board_at(dir.path())).await;
    let output = session.bye().await;
    assert_login_display(&output, "100/3/11/89");
    let second = account(dir.path(), 0);
    assert_eq!(second.start_this_session, 92.0);
    assert_eq!(second.debit_call, 6.0);
    assert_eq!(second.debit_msg_write, 5.0);
    assert_eq!(second.balance(false, 0.0), 89.0);
    assert_eq!(tracking(dir.path(), "LOGON"), 2);
    assert_eq!(tracking(dir.path(), "MSG WRITE"), 1);
}

#[tokio::test]
async fn accounting_registration_grants_opening_credit_only_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::start(Arc::new(Mutex::new(board_at(dir.path()))), options(false)).await;
    session.expect("[account-name]").await;
    session.send("NEW CALLER\r").await;
    session.expect("[account-reenter]").await;
    session.send("C\r").await;
    session.expect("[account-register]").await;
    session.send("Y\r").await;
    session.expect("[account-new-password]").await;
    session.send("NEWSECRET\r").await;
    session.expect("[account-confirm-password]").await;
    session.send("NEWSECRET\r").await;
    session.expect(COMMAND).await;
    let output = session.bye().await;
    assert_login_display(&output, "40/3/3/37");
    assert_eq!(account(dir.path(), 1).starting_balance, 40.0);
    assert_eq!(account(dir.path(), 1).balance(false, 0.0), 37.0);
    assert_eq!(account(dir.path(), 0).debit_call, 0.0);

    let mut session = Session::start(Arc::new(Mutex::new(board_at(dir.path()))), options(false)).await;
    session.authenticate("NEW CALLER", "NEWSECRET").await;
    session.bye().await;
    assert_eq!(account(dir.path(), 1).starting_balance, 40.0);
    assert_eq!(account(dir.path(), 1).balance(false, 0.0), 34.0);
    assert_eq!(tracking(dir.path(), "LOGON"), 2);
}

#[tokio::test]
async fn accounting_failed_password_never_charges_or_saves_selected_user() {
    let dir = tempfile::tempdir().unwrap();
    let board = board_at(dir.path());
    let before = std::fs::read(dir.path().join("users.toml")).unwrap();
    let mut session = Session::start(Arc::new(Mutex::new(board)), options(false)).await;
    session.expect("[account-name]").await;
    session.send("ACCOUNT USER\r").await;
    session.expect("[account-password]").await;
    session.send("WRONG\rWRONG\rWRONG\rWRONG\r").await;
    let (result, output) = session.finish().await;
    result.unwrap();
    assert!(!output.contains(INFO));
    assert!(!output.contains(COMMAND));
    assert_eq!(before, std::fs::read(dir.path().join("users.toml")).unwrap());
    assert_eq!(tracking(dir.path(), "LOGON"), 0);
}

#[tokio::test]
async fn accounting_local_sysop_starts_before_conference_presentation() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::start(Arc::new(Mutex::new(board_at(dir.path()))), options(true)).await;
    session.expect(COMMAND).await;
    let output = session.bye().await;
    assert_login_display(&output, "100/3/3/97");
    assert_eq!(account(dir.path(), 0).balance(false, 0.0), 97.0);
    assert_eq!(tracking(dir.path(), "LOGON"), 1);
}

#[tokio::test]
async fn accounting_direct_ppe_bypasses_ordinary_logon_even_with_sysop_option() {
    let dir = tempfile::tempdir().unwrap();
    let board = board_at(dir.path());
    let before = std::fs::read(dir.path().join("users.toml")).unwrap();
    let compiled = super::compile_test_ppe("PRINT \"[account-direct-ppe]\"");
    let ppe = dir.path().join("direct.ppe");
    std::fs::copy(&compiled, &ppe).unwrap();
    std::fs::remove_dir_all(compiled.parent().unwrap()).unwrap();
    let mut opts = options(true);
    opts.ppe = Some(PPEExecute {
        ppe,
        user_name: Some("ACCOUNT USER".into()),
        password: Some("SECRET".into()),
        args: Vec::new(),
    });
    let mut session = Session::start(Arc::new(Mutex::new(board)), opts).await;
    session.expect("[account-direct-ppe]").await;
    session.send("\r").await;
    let (result, output) = session.finish().await;
    result.unwrap();
    assert!(!output.contains(INFO));
    assert!(!output.contains(INTRO));
    assert!(!output.contains(COMMAND));
    assert_eq!(tracking(dir.path(), "LOGON"), 0);
    assert_eq!(before, std::fs::read(dir.path().join("users.toml")).unwrap());
}

#[tokio::test]
async fn accounting_carrier_eof_settles_unsaved_logon_and_read_charge() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::login(board_at(dir.path())).await;
    session.send("R\r1\r").await;
    session.expect("[account-message-body]").await;
    session.peer.shutdown().await.unwrap();
    let (result, _) = session.finish().await;
    result.unwrap();
    let saved = account(dir.path(), 0);
    assert_eq!(saved.debit_call, 3.0);
    assert_eq!(saved.debit_msg_read, 2.0);
    assert_eq!(saved.balance(false, 0.0), 95.0);
    assert_eq!(tracking(dir.path(), "LOGON"), 1);
    assert_eq!(tracking(dir.path(), "MSG READ"), 1);
}

#[tokio::test]
async fn accounting_broken_socket_still_finalizes_and_preserves_transport_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::login(board_at(dir.path())).await;
    session.fail_shutdown.store(true, Ordering::SeqCst);
    session.peer.shutdown().await.unwrap();
    let (result, _) = session.finish().await;
    assert_eq!(result.unwrap_err(), SOCKET_ERROR);
    assert_eq!(account(dir.path(), 0).balance(false, 0.0), 97.0);
    assert_eq!(tracking(dir.path(), "LOGON"), 1);
}

#[tokio::test]
async fn accounting_cleanup_failure_does_not_replace_original_socket_error() {
    let dir = tempfile::tempdir().unwrap();
    let board = Arc::new(Mutex::new(board_at(dir.path())));
    let mut session = Session::start(board.clone(), options(false)).await;
    session.authenticate("ACCOUNT USER", "SECRET").await;
    // A directory cannot be replaced by the user serializer. This deliberately
    // fails cleanup as well as transport shutdown; the transport error wins.
    board.lock().await.config.paths.user_file = dir.path().to_path_buf();
    session.fail_shutdown.store(true, Ordering::SeqCst);
    session.peer.shutdown().await.unwrap();
    let (result, _) = session.finish().await;
    assert_eq!(result.unwrap_err(), SOCKET_ERROR);
    assert_eq!(tracking(dir.path(), "LOGON"), 1);
}

#[tokio::test]
async fn accounting_tracking_only_allows_negative_balance_without_security_drop() {
    let dir = tempfile::tempdir().unwrap();
    let mut board = board_at(dir.path());
    board.sec_levels[0].accounting_tracking = true;
    board.users[0].account.as_mut().unwrap().starting_balance = 1.0;
    board.save_userbase().unwrap();
    let mut session = Session::login(board).await;
    session.write_message().await;
    let output = session.bye().await;
    assert_eq!(output.matches(INFO).count(), 1);
    assert!(!output.contains(WARNING), "{output}");
    let saved = UserBase::load(&dir.path().join("users.toml")).unwrap();
    assert_eq!(saved[0].security_level, 100);
    assert_eq!(saved[0].account.as_ref().unwrap().balance(false, 0.0), -7.0);
    assert_eq!(tracking(dir.path(), "LOGON"), 1);
    assert_eq!(tracking(dir.path(), "MSG WRITE"), 1);
}

#[tokio::test]
async fn accounting_existing_missing_account_does_not_receive_registration_grant() {
    let dir = tempfile::tempdir().unwrap();
    let mut board = board_at(dir.path());
    board.users[0].account = None;
    board.config.accounting.ignore_empty_sec_level = true;
    board.save_userbase().unwrap();
    let session = Session::login(board).await;
    session.bye().await;
    let saved = account(dir.path(), 0);
    assert_eq!(saved.starting_balance, 0.0);
    assert_eq!(saved.debit_call, 3.0);
    assert_eq!(saved.balance(false, 0.0), -3.0);
}
