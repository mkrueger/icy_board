//! Runs PPL snippets through the real compiler and the real VM.
//!
//! The point of these tests is `PCBoard` compatibility, so they assert on what a
//! caller would have seen rather than on the VM's internal state. Anything worth
//! checking can be printed, which keeps the assertions in the same language as
//! the behaviour they describe.

#![cfg(test)]

mod board_session;
mod builtin_enums;
mod contacts;
mod control_flow;
mod dbase;
mod display_pause;
mod errors;
mod events;
mod file_channels;
mod file_names;
mod fonts;
mod forward_calls;
mod graphics;
mod hyperlinks;
mod margins;
mod masks;
mod member_calls;
mod message_base;
mod mouse;
mod nested_records;
mod palette;
mod ppe_paths;
mod record_literals;
mod records;
mod retired_terminal_api;
mod routine_parameters;
mod scalars;
mod sound;
mod static_members;
mod terminal_control;
mod terminal_gfx;
mod terminal_info;
mod terminal_output;
mod terminal_screen;
mod tpa;
mod user_object;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use icy_net::channel::ChannelConnection;
use icy_net::{Connection, ConnectionType};

use crate::compiler::PPECompiler;
use crate::compiler::workspace::Workspace;
use crate::icy_board::bbs::BBS;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::user_base::User;
use crate::icy_board::{IcyBoard, message_area::AreaList, message_area::MessageArea};
use crate::parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast};
use crate::vm::io::DiskIO;
use crate::vm::run;

/// The diagnostics a snippet produces, for the cases where not compiling is the point.
pub fn compile_errors(source: &str) -> Vec<String> {
    compile_errors_with_runtime(source, crate::executable::LAST_PPE_RUNTIME)
}

pub fn compile_errors_with_runtime(source: &str, runtime: u16) -> Vec<String> {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let reg = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("test.pps")]);
    workspace.package.runtime = Some(runtime);
    // These gates are about what the PPE can store, so the language stays at 4.00
    // instead of following the runtime down and hiding the names being tested.
    workspace.set_default_language_version(Some(crate::executable::LAST_PPL_LANGUAGE_VERSION));

    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &reg, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, reg, errors.clone());
    compiler.compile(&[&ast]);

    let reporter = errors.lock().unwrap();
    reporter.errors.iter().map(|e| e.error.to_string()).collect()
}

/// Compiles a PPL snippet, or panics with the diagnostics if it does not build.
fn compile(source: &str) -> crate::executable::Executable {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let reg = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("test.pps")]);

    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &reg, Encoding::Utf8, &workspace);

    let mut compiler = PPECompiler::new(&workspace, reg, errors.clone());
    compiler.compile(&[&ast]);

    let reporter = errors.lock().unwrap();
    assert!(
        !reporter.has_errors(),
        "the snippet does not compile:\n{}",
        reporter.errors.iter().map(|e| format!("  {}", e.error)).collect::<Vec<_>>().join("\n")
    );
    drop(reporter);

    // Round tripping through the on disk form is what fills in the variable table's
    // array storage, so a snippet with an array behaves the way a real PPE would.
    let executable = compiler.create_executable().expect("the snippet does not compile");
    let mut bytes = executable.to_buffer().expect("the snippet does not serialize");
    crate::executable::Executable::from_buffer(&mut bytes, false).expect("the snippet does not load")
}

/// Compiles and runs a PPL snippet against a scratch board, and returns
/// everything it wrote to the terminal.
pub fn run_ppl(source: &str) -> String {
    run_ppl_on(source, |_| {})
}

/// The same, with a chance to shape the board the snippet runs against.
pub fn run_ppl_on<P: Fn(&mut IcyBoard)>(source: &str, init_fn: P) -> String {
    run_ppl_seeded(source, init_fn, &[])
}

/// The same again, with files copied into the scratch directory first so a snippet
/// can be pointed at a fixture.
pub fn run_ppl_with_files(source: &str, files: &[(&str, &[u8])]) -> String {
    run_ppl_seeded(source, |_| {}, files)
}

/// The same, with the snippet running as a PPE that lives in `ppe_dir` below the board.
/// File names may name subdirectories, which are created on the way.
pub fn run_ppl_in_ppe_dir(source: &str, ppe_dir: &str, files: &[(&str, &[u8])]) -> String {
    run_ppl_seeded_in(source, |_| {}, files, Some(ppe_dir))
}

pub fn run_ppl_with_input(source: &str, input: &[u8]) -> String {
    run_ppl_collecting(source, |_| {}, &[], None, input, false).1
}

pub fn run_ppl_with_input_after_output(source: &str, marker: &[u8], input: &[u8]) -> String {
    let executable = compile(source);
    let work_dir = scratch_dir("delayed-input");
    let output = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let mut board = IcyBoard::new();
        board.default_display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
        board.root_path = work_dir.clone();
        board.users.new_user(User {
            name: "SYSOP".to_string(),
            security_level: 255,
            ..Default::default()
        });
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let node_state = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        peer.send(b"q").await.unwrap();

        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), node_state, node, Box::new(connection)).await;
        let sysop = state.get_board().await.users[0].clone();
        state.session.current_user = Some(sysop);
        state.session.cur_user_id = 0;

        let collected = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = collected.clone();
        let marker = marker.to_vec();
        let input = input.to_vec();
        let reader = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let mut sent = false;
                let mut buffer = [0; 1024];
                while let Ok(size) = peer.read(&mut buffer).await {
                    if size == 0 {
                        break;
                    }
                    sink.lock().unwrap().extend(&buffer[..size]);
                    if !sent && sink.lock().unwrap().windows(marker.len()).any(|window| window == marker) {
                        peer.send(&input).await.unwrap();
                        sent = true;
                    }
                }
            });
        });

        let mut io = DiskIO::new(work_dir.to_str().unwrap(), None);
        run(&PathBuf::from("test.ppe"), &executable, &mut io, &mut state).await.unwrap();
        drop(state);
        reader.join().unwrap();
        collected.lock().unwrap().clone()
    });
    let _ = std::fs::remove_dir_all(&work_dir);
    String::from_utf8(output).unwrap().replace("\r\n", "\n")
}

pub fn run_ppl_with_files_and_input(source: &str, files: &[(&str, &[u8])], input: &[u8]) -> String {
    run_ppl_collecting(source, |_| {}, files, None, input, false).1
}

fn run_ppl_seeded<P: Fn(&mut IcyBoard)>(source: &str, init_fn: P, files: &[(&str, &[u8])]) -> String {
    run_ppl_seeded_in(source, init_fn, files, None)
}

fn run_ppl_seeded_in<P: Fn(&mut IcyBoard)>(source: &str, init_fn: P, files: &[(&str, &[u8])], ppe_dir: Option<&str>) -> String {
    run_ppl_collecting(source, init_fn, files, ppe_dir, &[], false).1
}

/// True when the program ran to its end rather than giving up with STOP, which is what
/// decides whether a script questionnaire keeps the answers it collected.
pub fn ppl_keeps_script_answers(source: &str) -> bool {
    run_ppl_collecting(source, |_| {}, &[], None, &[], false).0
}

pub fn run_ppl_with_cleanup(source: &str) -> String {
    run_ppl_collecting(source, |_| {}, &[], None, &[], true).1
}

fn run_ppl_collecting<P: Fn(&mut IcyBoard)>(
    source: &str,
    init_fn: P,
    files: &[(&str, &[u8])],
    ppe_dir: Option<&str>,
    input: &[u8],
    cleanup: bool,
) -> (bool, String) {
    let executable = compile(source);
    let work_dir = scratch_dir("run");
    for (name, bytes) in files {
        let path = work_dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let ppe_file = match ppe_dir {
        Some(dir) => {
            std::fs::create_dir_all(work_dir.join(dir)).unwrap();
            work_dir.join(dir).join("test.ppe")
        }
        None => PathBuf::from("test.ppe"),
    };

    let output = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let mut board = IcyBoard::new();
        board.default_display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
        // Relative paths a snippet names resolve into the scratch directory, so
        // nothing it writes outlives the test.
        board.root_path = work_dir.clone();
        board.users.new_user(User {
            name: "SYSOP".to_string(),
            security_level: 255,
            ..Default::default()
        });
        init_fn(&mut board);
        for conference in board.conferences.iter_mut() {
            if conference.areas.is_none() {
                conference.areas = Some(scratch_message_area());
            }
        }

        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let node_state = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        if !input.is_empty() {
            peer.send(input).await.unwrap();
        }

        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), node_state, node, Box::new(connection)).await;
        let sysop = state.get_board().await.users[0].clone();
        state.session.current_user = Some(sysop);
        state.session.cur_user_id = 0;

        // The peer end is drained on its own thread, so a snippet that writes
        // more than one channel buffer cannot deadlock against us.
        let collected = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = collected.clone();
        let reader = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let mut buffer = [0; 1024];
                while let Ok(size) = peer.read(&mut buffer).await {
                    if size == 0 {
                        break;
                    }
                    sink.lock().unwrap().extend(&buffer[0..size]);
                }
            });
        });

        let mut io = DiskIO::new(work_dir.to_str().unwrap(), None);
        let result = run(&ppe_file, &executable, &mut io, &mut state).await;
        if cleanup {
            state.cleanup_ppl_media().await;
        }

        // Dropping the board end closes the channel, which is what lets the
        // reader finish instead of blocking on a connection nobody will write to.
        drop(state);
        reader.join().unwrap();

        let kept_answers = result.expect("the snippet failed to run");
        let bytes = collected.lock().unwrap().clone();
        (kept_answers, bytes)
    });

    let _ = std::fs::remove_dir_all(&work_dir);
    let (kept_answers, bytes) = output;
    (
        kept_answers,
        String::from_utf8(bytes).expect("PPE output is not valid UTF-8").replace("\r\n", "\n"),
    )
}

/// A directory the snippet is free to write into, unique per test.
fn scratch_dir(kind: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ppl-test-{kind}-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs a snippet against a board whose conference 0 holds the given messages,
/// each one a `(from, to, subject)` triple numbered from one in order.
pub fn run_ppl_with_messages(source: &str, messages: &[(&str, &str, &str)]) -> String {
    use jamjam::jam::{JamMessage, JamMessageBase};

    let path = scratch_dir("base").join("general");
    let mut base = JamMessageBase::create(&path).expect("can't create the scratch message base");
    for (from, to, subject) in messages {
        base.write_message(
            &JamMessage::default()
                .with_from(bstr::BString::from(*from))
                .with_to(bstr::BString::from(*to))
                .with_subject(bstr::BString::from(*subject))
                .with_date_time(chrono::Utc::now())
                .with_text(bstr::BString::from("body")),
        )
        .expect("can't write a scratch message");
    }
    base.write_jhr_header().unwrap();
    drop(base);

    run_ppl_on(source, |board| {
        board.conferences.push(crate::icy_board::conferences::Conference {
            name: "Main Board".to_string(),
            areas: Some(AreaList::new(vec![MessageArea {
                name: "General".to_string(),
                path: path.clone(),
                ..Default::default()
            }])),
            ..Default::default()
        });
        // A second, empty conference, so a snippet has somewhere to move a message to.
        board.conferences.push(crate::icy_board::conferences::Conference {
            name: "Elsewhere".to_string(),
            ..Default::default()
        });
    })
}

fn scratch_message_area() -> AreaList {
    let dir = scratch_dir("msg");
    AreaList::new(vec![MessageArea {
        name: "General".to_string(),
        path: dir.join("general"),
        ..Default::default()
    }])
}

#[test]
fn test_the_harness_runs_a_program_and_captures_its_output() {
    assert_eq!(run_ppl(r#"PRINT "Hello, World!""#), "Hello, World!");
}

#[test]
fn stop_gives_up_on_a_program_and_exit_does_not() {
    assert!(ppl_keeps_script_answers("PRINT \"a\""), "reaching the end should keep the answers");
    assert!(ppl_keeps_script_answers("PRINT \"a\"\nEXIT"), "EXIT should keep the answers");
    assert!(!ppl_keeps_script_answers("PRINT \"a\"\nSTOP"), "STOP should drop the answers");
    assert!(!ppl_keeps_script_answers("IF (1) THEN\n  STOP\nENDIF\nPRINT \"unreachable\""));
}

#[test]
fn test_the_harness_reports_a_value_computed_by_the_vm() {
    assert_eq!(run_ppl("PRINT 6 * 7"), "42");
}

#[test]
fn test_a_statement_we_have_not_implemented_does_not_stop_the_program() {
    assert_eq!(run_ppl("POKEB 0, 0\nPRINT \"still running\""), "still running");
}

#[test]
fn test_the_database_functions_report_a_channel_that_was_never_opened() {
    // An error, and both ends of the file at once, so a record loop written the
    // usual way stops on its first test instead of running forever.
    assert_eq!(run_ppl(r#"PRINT DERR(1), ",", DEOF(1), ",", DBOF(1), ",", DRECCOUNT(1)"#), "1,1,1,0");
}
