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

#[tokio::test]
async fn logoff_text_hooks_run_ppe_before_disconnect() {
    let compiled = crate::tests::compile_test_ppe(
        r#"
STRING ARG, RECORD
ARG = TOKENSTR()
TOKENIZE ARG
GETTOKEN ARG
GETTOKEN RECORD
FAPPEND 1, PPEPATH() + "hooks.txt", O_WR, S_DN
FPUTLN 1, ARG + ":" + RECORD
FCLOSE 1
PRINTLN "[logoff-hook-" + RECORD + "]"
PRINT "@HANGUP@"
EXIT
"#,
    );
    for enabled in [false, true] {
        for command in ["G", "BYE"] {
            let root = tempfile::tempdir().unwrap();
            let ppe = root.path().join("hook.ppe");
            std::fs::copy(&compiled, &ppe).unwrap();
            let mut board = board_at(root.path());
            board.config.accounting.enabled = enabled;
            for record in [192, 166] {
                board
                    .default_display_text
                    .update_record_number(record, format!("!{} /LOGOFF {record}", ppe.display()))
                    .unwrap();
            }
            let mut session = Session::login(board).await;
            session.send(&format!("{command}\r")).await;
            let (result, output) = session.finish().await;
            assert!(result.is_ok(), "{command}, accounting={enabled}: {result:?}; {output:?}");
            let hooks = std::fs::read_to_string(root.path().join("hooks.txt"));
            assert!(hooks.is_ok(), "{command}, accounting={enabled}: {hooks:?}; {output:?}");
            assert_eq!(hooks.unwrap().trim_start_matches('\u{feff}'), "/LOGOFF:192\n/LOGOFF:166\n", "{output:?}");
            assert_eq!(output.matches("[logoff-hook-192]").count(), 1, "{output:?}");
            assert_eq!(output.matches("[logoff-hook-166]").count(), 1, "{output:?}");
            assert!(
                output.find("[logoff-hook-192]").unwrap() < output.find("[logoff-hook-166]").unwrap(),
                "{output:?}"
            );
            assert!(!output.contains("Error occurred executing PPE"), "{output:?}");
        }
    }
}

#[tokio::test]
async fn logoff_hook_accepts_input_after_time_limit_without_restarting_logoff() {
    let root = tempfile::tempdir().unwrap();
    let compiled = crate::tests::compile_test_ppe(
        r#"
STRING KEY, ANSWER
PRINT "@HANGUP@"
ADJTIME -2000
PRINTLN "[logoff-hook-input]"
KEY = ""
WHILE (KEY = "") DO
    KEY = INKEY()
ENDWHILE
PRINTLN "[logoff-hook-key-" + KEY + "]"
INPUTSTR "", ANSWER, 7, 20, "abcdefghijklmnopqrstuvwxyz", 0
PRINTLN "[logoff-hook-answer-" + ANSWER + "]"
EXIT
"#,
    );
    let mut board = board_at(root.path());
    board
        .default_display_text
        .update_record_number(192, format!("!{}", compiled.display()))
        .unwrap();
    let mut session = Session::login(board).await;
    session.send("BYE\r").await;
    session.expect("[logoff-hook-input]").await;
    session.send("xanswer\r").await;
    let (result, output) = session.finish().await;
    assert!(result.is_ok(), "{result:?}; {output:?}");
    assert_eq!(output.matches("[logoff-hook-input]").count(), 1, "{output:?}");
    assert_eq!(output.matches("[logoff-hook-key-x]").count(), 1, "{output:?}");
    assert_eq!(output.matches("[logoff-hook-answer-answer]").count(), 1, "{output:?}");
    assert_eq!(output.matches("Thanks for calling").count(), 1, "{output:?}");
    assert!(!output.contains("Error occurred executing PPE"), "{output:?}");
}

#[tokio::test]
async fn normal_logoff_allows_parent_ppe_to_finish_before_hooks() {
    use icy_board_engine::icy_board::commands::{Command, CommandAction, CommandType};

    let root = tempfile::tempdir().unwrap();
    let parent = crate::tests::compile_test_ppe("COMMAND FALSE, \"BYE\"\nPRINTLN \"[parent-resumed-after-logoff]\"\nEXIT");
    let hook = crate::tests::compile_test_ppe("PRINTLN \"[logoff-final-ppe]\"\nEXIT");
    let mut board = board_at(root.path());
    board.commands.push(Command {
        keyword: "PPEEXIT".into(),
        actions: vec![CommandAction {
            command_type: CommandType::RunPPE,
            parameter: parent.to_string_lossy().into_owned(),
            ..Default::default()
        }],
        ..Default::default()
    });
    board.default_display_text.update_record_number(192, format!("!{}", hook.display())).unwrap();
    let mut session = Session::login(board).await;
    session.send("PPEEXIT\r").await;
    let (result, output) = session.finish().await;
    assert!(result.is_ok(), "{result:?}; {output:?}");
    assert_eq!(output.matches("[parent-resumed-after-logoff]").count(), 1, "{output:?}");
    assert_eq!(output.matches("[logoff-final-ppe]").count(), 1, "{output:?}");
    assert!(
        output.find("[parent-resumed-after-logoff]").unwrap() < output.find("[logoff-final-ppe]").unwrap(),
        "{output:?}"
    );
    assert_eq!(output.matches("Thanks for calling").count(), 1, "{output:?}");
    assert!(!output.contains("Error occurred executing PPE"), "{output:?}");
}

#[tokio::test]
async fn logoff_statements_follow_pcboard_kinds() {
    use icy_board_engine::icy_board::commands::{Command, CommandAction, CommandType};

    // BYE/GOODBYE are what the caller asked for, HANGUP and @HANGUP@ are not.
    for (statement, asks_caller, courtesies) in [
        ("BYE", true, true),
        ("GOODBYE", true, true),
        ("HANGUP", false, false),
        ("PRINT \"@HANGUP@\"", false, true),
    ] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("logoff.survey"), "[logoff-survey]\r\n*****\r\n").unwrap();
        let ppe = crate::tests::compile_test_ppe(&format!("{statement}\nEXIT"));
        let hook = crate::tests::compile_test_ppe("PRINTLN \"[logoff-hook]\"\nEXIT");
        let mut board = board_at(root.path());
        board.config.paths.logoff_survey = root.path().join("logoff.survey");
        board.config.paths.logoff_answer = root.path().join("logoff.answers");
        board.commands.push(Command {
            keyword: "QUIT".into(),
            actions: vec![CommandAction {
                command_type: CommandType::RunPPE,
                parameter: ppe.to_string_lossy().into_owned(),
                ..Default::default()
            }],
            ..Default::default()
        });
        board.default_display_text.update_record_number(192, format!("!{}", hook.display())).unwrap();
        let mut session = Session::login(board).await;
        session.send("QUIT\r").await;
        let (result, output) = session.finish().await;
        assert!(result.is_ok(), "{statement}: {result:?}; {output:?}");
        assert_eq!(output.matches("[logoff-survey]").count(), usize::from(asks_caller), "{statement}: {output:?}");
        assert_eq!(output.matches("[logoff-hook]").count(), 1, "{statement}: {output:?}");
        assert_eq!(output.matches("Thanks for calling").count(), usize::from(courtesies), "{statement}: {output:?}");
        assert!(!output.contains("Error occurred executing PPE"), "{statement}: {output:?}");
    }
}

#[tokio::test]
#[ignore = "requires the original LastCaller package in ICB_LASTCALLER_DIR"]
async fn logoff_original_lastcaller_updates_data_from_icbtext_192() {
    let package = std::path::PathBuf::from(std::env::var_os("ICB_LASTCALLER_DIR").expect("ICB_LASTCALLER_DIR"));
    for enabled in [false, true] {
        for command in ["G", "BYE"] {
            let root = tempfile::tempdir().unwrap();
            for file in ["LC.PPE", "4EVER83.DAT", "GATE.PCB"] {
                std::fs::copy(package.join(file), root.path().join(file)).unwrap();
            }
            let data = root.path().join("4EVER83.DAT");
            let before = std::fs::read(&data).unwrap();
            let mut board = board_at(root.path());
            board.config.accounting.enabled = enabled;
            board
                .default_display_text
                .update_record_number(192, format!("!{} /LOGOFF", root.path().join("LC.PPE").display()))
                .unwrap();
            let mut session = Session::login(board).await;
            session.send(&format!("{command}\r")).await;
            let (result, output) = session.finish().await;
            assert!(result.is_ok(), "{command}, accounting={enabled}: {result:?}; {output:?}");
            let after = std::fs::read(&data).unwrap();
            assert_ne!(before, after, "{command}, accounting={enabled}: {output:?}");
            let lines: Vec<_> = after
                .split(|byte| *byte == b'\n')
                .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
                .collect();
            assert_eq!(lines.get(65).copied(), Some(b"ACCOUNT USER".as_slice()), "{output:?}");
            assert_eq!(output.matches("Thanks for calling").count(), 1, "{output:?}");
            assert!(!output.contains("Error occurred executing PPE"), "{output:?}");
        }
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

fn newask_board(root: &Path, closed: bool, use_newask: bool, survey: &str) -> IcyBoard {
    let mut board = board_at(root);
    board.config.accounting.enabled = false;
    board.config.system_control.is_closed_board = closed;
    board.config.new_user_settings.use_newask_and_builtin = use_newask;
    board.config.paths.newask_survey = match survey {
        "unset" => Default::default(),
        "directory" => root.to_path_buf(),
        _ => root.join("newask"),
    };
    board.config.paths.newask_answer = root.join("newask.answers");
    if survey == "file" {
        std::fs::write(root.join("newask"), "*****\n[newask-question]\n").unwrap();
        std::fs::write(root.join("newask.answers"), "").unwrap();
    }
    let settings = &mut board.config.new_user_settings;
    settings.ask_city_or_state = true;
    settings.ask_business_phone = true;
    settings.ask_home_phone = true;
    settings.ask_address = true;
    settings.ask_verification = true;
    settings.ask_comment = true;
    settings.ask_clr_msg = true;
    settings.ask_use_short_descr = true;
    for (id, marker) in [
        (IceText::CityState, "[newask-city-state]"),
        (IceText::BusDataPhone, "[newask-business-phone]"),
        (IceText::HomeVoicePhone, "[newask-home-phone]"),
        (IceText::CommentFieldPrompt, "[newask-comment]"),
        (IceText::CLSBetweenMessages, "[newask-clear]"),
        (IceText::EnterAddress, "[newask-address]"),
        (IceText::Street1, "[newask-street1]"),
        (IceText::Street2, "[newask-street2]"),
        (IceText::City, "[newask-city]"),
        (IceText::State, "[newask-state]"),
        (IceText::Zip, "[newask-zip]"),
        (IceText::Country, "[newask-country]"),
        (IceText::EnterVerifyText, "[newask-verify]"),
        (IceText::UseShortDescription, "[newask-short]"),
        (IceText::CompleteQuestion, "[newask-confirm]"),
    ] {
        board.default_display_text.update_record_number(id as usize, marker).unwrap();
    }
    board
}

async fn begin_newask_registration(board: Arc<Mutex<IcyBoard>>) -> Session {
    let mut session = Session::start(board, options(false)).await;
    for (prompt, answer) in [("[account-name]", "NEW CALLER"), ("[account-reenter]", "C"), ("[account-register]", "Y")] {
        session.expect(prompt).await;
        session.send(&format!("{answer}\r")).await;
    }
    session
}

#[tokio::test]
async fn pcboard_newask_on_open_boards_is_additive_and_never_replaces_builtin_questions() {
    for use_newask in [false, true] {
        for survey in ["file", "missing", "unset", "directory"] {
            let dir = tempfile::tempdir().unwrap();
            let board = Arc::new(Mutex::new(newask_board(dir.path(), false, use_newask, survey)));
            let mut session = begin_newask_registration(board.clone()).await;
            for (prompt, answer) in [
                ("[account-new-password]", "NEWSECRET"),
                ("[account-confirm-password]", "NEWSECRET"),
                ("[newask-city-state]", "Hamburg"),
                ("[newask-business-phone]", "123-4567"),
                ("[newask-home-phone]", "234-5678"),
                ("[newask-comment]", "New caller comment"),
                ("[newask-clear]", "Y"),
                ("[newask-street1]", "Test Street 12"),
                ("[newask-street2]", "Floor 3"),
                ("[newask-city]", "Hamburg"),
                ("[newask-state]", "HH"),
                ("[newask-zip]", "20095"),
                ("[newask-country]", "Germany"),
                ("[newask-verify]", "Verification answer"),
                ("[newask-short]", "Y"),
            ] {
                session.expect(prompt).await;
                session.send(&format!("{answer}\r")).await;
            }
            let run_survey = use_newask && survey == "file";
            if run_survey {
                session.expect("[newask-question]").await;
                assert_eq!(board.lock().await.users.len(), 1, "account published before NEWASK finished");
                assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap().len(), 1);
                session.send("Survey answer\r").await;
            }
            session.expect(COMMAND).await;
            let output = session.bye().await;
            assert_eq!(output.matches("[newask-question]").count(), usize::from(run_survey), "{output}");
            assert!(!output.contains("[newask-confirm]"), "NEWASK must not ask permission: {output}");
            let users = UserBase::load(&dir.path().join("users.toml")).unwrap();
            assert_eq!(users.len(), 2);
            let user = &users[1];
            assert_eq!(user.get_name(), "NEW CALLER");
            assert!(user.password.password.is_valid("NEWSECRET"));
            assert_eq!(user.city_or_state, "Hamburg");
            assert_eq!(user.bus_data_phone, "123-4567");
            assert_eq!(user.home_voice_phone, "234-5678");
            assert_eq!(user.user_comment, "New caller comment");
            assert!(user.flags.msg_clear);
            assert_eq!(user.street1, "Test Street 12");
            assert_eq!(user.street2, "Floor 3");
            assert_eq!(user.city, "Hamburg");
            assert_eq!(user.state, "HH");
            assert_eq!(user.zip, "20095");
            assert_eq!(user.country, "Germany");
            assert_eq!(user.verify_answer, "Verification answer");
            assert!(user.flags.use_short_filedescr);
            let answers = std::fs::read_to_string(dir.path().join("newask.answers")).unwrap_or_default();
            assert_eq!(answers.contains("A: Survey answer"), run_survey, "{answers}");
        }
    }
}

#[tokio::test]
async fn pcboard_newask_on_closed_boards_never_creates_an_account_or_asks_builtin_questions() {
    for use_newask in [false, true] {
        for survey in ["file", "missing", "unset", "directory"] {
            let dir = tempfile::tempdir().unwrap();
            let board = Arc::new(Mutex::new(newask_board(dir.path(), true, use_newask, survey)));
            let before = std::fs::read(dir.path().join("users.toml")).unwrap();
            let mut session = begin_newask_registration(board.clone()).await;
            if survey == "file" {
                session.expect("[newask-question]").await;
                session.send("Closed board answer\r").await;
            }
            let (result, output) = session.finish().await;
            result.unwrap();
            for forbidden in [
                "[account-new-password]",
                "[newask-city-state]",
                "[newask-address]",
                "[newask-verify]",
                "[newask-confirm]",
                COMMAND,
            ] {
                assert!(!output.contains(forbidden), "closed board reached {forbidden}: {output}");
            }
            assert_eq!(board.lock().await.users.len(), 1);
            assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), before);
            let answers = std::fs::read_to_string(dir.path().join("newask.answers")).unwrap_or_default();
            assert_eq!(answers.contains("A: Closed board answer"), survey == "file", "{answers}");
        }
    }
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
async fn pcboard_newask_creates_answer_file_and_requires_nonempty_answers() {
    for existing_answers in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut board = newask_board(dir.path(), true, false, "file");
        board
            .default_display_text
            .update_record_number(IceText::ResponseRequired as usize, "[newask-required]")
            .unwrap();
        if !existing_answers {
            std::fs::remove_file(&board.config.paths.newask_answer).unwrap();
            std::fs::write(&board.config.paths.newask_survey, ";[newask-intro]\n[newask-question]\n").unwrap();
        }
        let mut session = begin_newask_registration(Arc::new(Mutex::new(board))).await;
        session.expect("[newask-question]").await;
        session.send("\r").await;
        session.expect("[newask-required]").await;
        session.expect("[newask-question]").await;
        session.send("Required answer\r").await;
        let (result, output) = session.finish().await;
        result.unwrap();
        assert_eq!(output.matches("[newask-question]").count(), 2, "{output}");
        let answers = std::fs::read_to_string(dir.path().join("newask.answers")).unwrap();
        assert!(answers.contains("Q: [newask-question]\nA: Required answer"), "{answers}");
        assert!(!answers.contains("A: \n"), "{answers}");
        assert_eq!(UserBase::load(&dir.path().join("users.toml")).unwrap().len(), 1);
    }
}

#[tokio::test]
async fn pcboard_newask_respects_disabled_or_blank_builtin_prompts_and_display_only_surveys() {
    for blank_prompts in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut board = newask_board(dir.path(), false, true, "file");
        board.config.paths.newask_answer = Default::default();
        if blank_prompts {
            for id in [
                IceText::CityState,
                IceText::BusDataPhone,
                IceText::HomeVoicePhone,
                IceText::CommentFieldPrompt,
                IceText::CLSBetweenMessages,
                IceText::EnterAddress,
                IceText::EnterVerifyText,
                IceText::UseShortDescription,
            ] {
                board.default_display_text.update_record_number(id as usize, "").unwrap();
            }
        } else {
            let settings = &mut board.config.new_user_settings;
            settings.ask_city_or_state = false;
            settings.ask_business_phone = false;
            settings.ask_home_phone = false;
            settings.ask_address = false;
            settings.ask_verification = false;
            settings.ask_comment = false;
            settings.ask_clr_msg = false;
            settings.ask_use_short_descr = false;
        }
        let mut session = begin_newask_registration(Arc::new(Mutex::new(board))).await;
        session.expect("[account-new-password]").await;
        session.send("NEWSECRET\r").await;
        session.expect("[account-confirm-password]").await;
        session.send("NEWSECRET\r").await;
        session.expect(COMMAND).await;
        let output = session.bye().await;
        for forbidden in [
            "[newask-city-state]",
            "[newask-business-phone]",
            "[newask-home-phone]",
            "[newask-comment]",
            "[newask-clear]",
            "[newask-address]",
            "[newask-street1]",
            "[newask-verify]",
            "[newask-short]",
            "[newask-confirm]",
        ] {
            assert!(!output.contains(forbidden), "disabled prompt {forbidden} was asked: {output}");
        }
        assert_eq!(output.matches("[newask-question]").count(), 1, "{output}");
        assert!(std::fs::read_to_string(dir.path().join("newask.answers")).unwrap().is_empty());
        let users = UserBase::load(&dir.path().join("users.toml")).unwrap();
        assert_eq!(users.len(), 2);
        assert!(users[1].city_or_state.is_empty());
        assert!(users[1].bus_data_phone.is_empty());
        assert!(users[1].verify_answer.is_empty());
    }
}

#[tokio::test]
async fn pcboard_newask_disconnect_before_completion_does_not_publish_an_account() {
    let dir = tempfile::tempdir().unwrap();
    let mut board = newask_board(dir.path(), false, true, "file");
    for id in [
        IceText::CityState,
        IceText::BusDataPhone,
        IceText::HomeVoicePhone,
        IceText::CommentFieldPrompt,
        IceText::CLSBetweenMessages,
        IceText::EnterAddress,
        IceText::EnterVerifyText,
        IceText::UseShortDescription,
    ] {
        board.default_display_text.update_record_number(id as usize, "").unwrap();
    }
    let before = std::fs::read(&board.config.paths.user_file).unwrap();
    let board = Arc::new(Mutex::new(board));
    let mut session = begin_newask_registration(board.clone()).await;
    session.expect("[account-new-password]").await;
    session.send("NEWSECRET\r").await;
    session.expect("[account-confirm-password]").await;
    session.send("NEWSECRET\r").await;
    session.expect("[newask-question]").await;
    session.peer.shutdown().await.unwrap();
    let (result, output) = session.finish().await;
    result.unwrap();
    assert!(!output.contains(COMMAND), "{output}");
    assert_eq!(board.lock().await.users.len(), 1);
    assert_eq!(std::fs::read(dir.path().join("users.toml")).unwrap(), before);
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
