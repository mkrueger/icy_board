//! Runs PPE files that were built with 4.00 and checked in as they are.
//!
//! Every other test compiles its source at test time, which only ever measures
//! the current toolchain against itself. These files are the evidence that a
//! program a user compiled once keeps loading and behaving on a later runtime.
//! They are deliberately not rebuilt on a normal test run.

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::Executable,
    icy_board::{
        IcyBoard,
        bbs::BBS,
        conferences::Conference,
        state::IcyBoardState,
        user_base::{User, UserContact},
    },
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    vm::{self, io::DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const FIXTURES: [&str; 3] = ["host_objects", "optional_arguments", "language_core"];

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/stored_ppe")
}

/// The board every fixture sees, so the recorded output means something.
fn seed_board(board: &mut IcyBoard) {
    board.config.board.name = "Icy Board".to_string();
    board.config.board.location = "Somewhere".to_string();
    board.config.board.operator = "The Operator".to_string();
    board.config.board.num_nodes = 4;
    board.config.sysop.name = "The Sysop".to_string();
    board.conferences.clear();
    board.conferences.push(Conference {
        name: "Main Board".to_string(),
        ..Default::default()
    });
    board.conferences.push(Conference {
        name: "Second".to_string(),
        ..Default::default()
    });
    board.users.clear();
    let mut first = User::default();
    first.set_name("First User".to_string());
    first.city_or_state = "Somewhere".to_string();
    first.contacts.push(UserContact {
        service: "matrix".to_string(),
        account: "@first:example.org".to_string(),
    });
    board.users.new_user(first);
    let mut second = User::default();
    second.set_name("Second User".to_string());
    second.city_or_state = "Elsewhere".to_string();
    board.users.new_user(second);
}

fn compile(source_path: &Path) -> Vec<u8> {
    let source = std::fs::read_to_string(source_path).unwrap();
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(400));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(source_path.to_path_buf(), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let messages: Vec<_> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert!(messages.is_empty(), "{}: {messages:?}", source_path.display());
    compiler.create_executable().unwrap().to_buffer().unwrap()
}

fn run(name: &str, executable: &Executable) -> String {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.root_path = directory.path().to_path_buf();
        seed_board(&mut board);
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        let reader = async {
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            while let Ok(size) = peer.read(&mut buffer).await {
                if size == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..size]);
            }
            String::from_utf8(bytes).unwrap().replace("\r\n", "\n")
        };
        let execute = async {
            let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
            let result = vm::run(&PathBuf::from(format!("{name}.ppe")), executable, &mut io, &mut state).await;
            drop(state);
            result
        };
        let (output, result) = tokio::join!(reader, execute);
        assert!(result.is_ok(), "{name}: {result:?}; output={output}");
        output
    })
}

/// Rebuilding is a decision, not a side effect of running the suite: a fixture
/// that is regenerated silently proves nothing about older programs.
#[test]
fn stored_programs_keep_loading_and_behaving() {
    let directory = fixture_dir();
    let update = std::env::var("UPDATE_PPE_FIXTURES").is_ok();
    for name in FIXTURES {
        let binary = directory.join(format!("{name}.ppe"));
        if update {
            std::fs::write(&binary, compile(&directory.join(format!("{name}.pps")))).unwrap();
        }

        let mut bytes = std::fs::read(&binary).unwrap_or_else(|error| panic!("{}: {error}", binary.display()));
        let executable = Executable::from_buffer(&mut bytes, false).unwrap_or_else(|error| panic!("{name} no longer loads: {error}"));
        assert_eq!(executable.runtime, 400, "{name} is not a 4.00 file");

        let output = run(name, &executable);
        let expected_path = directory.join(format!("{name}.out"));
        if update {
            std::fs::write(&expected_path, &output).unwrap();
        }
        let expected = std::fs::read_to_string(&expected_path).unwrap_or_default().replace("\r\n", "\n");
        assert_eq!(output, expected, "{name} behaves differently than when it was stored");
    }
}
