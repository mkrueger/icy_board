use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use icy_board_engine::{
    compiler::{
        workspace::{CompilerData, Workspace},
        PPECompiler,
    },
    decompiler::decompile,
    executable::Executable,
    icy_board::{bbs::BBS, read_data_with_encoding_detection, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry},
};
use icy_net::{channel::ChannelConnection, Connection, ConnectionType};

const EXPECTED_OUTPUT: &str = "Hello World!\n1,2,3,4,5,6,7,8,9,10,";
#[test]
fn test_legacy_ppe_roundtrip() {
    use std::fs::{self};

    let mut data_path: PathBuf = env::current_dir().unwrap();
    data_path.push("src/test_ppe");
    //let mut success = 0;
    //let mut skipped = 0;
    for entry in fs::read_dir("tests/test_ppe").expect("Error reading test_data directory.") {
        let cur_entry = entry.unwrap().path();
        if cur_entry.extension().unwrap() != "ppe" {
            continue;
        }
        let file_name = cur_entry.as_os_str().to_str().unwrap();
        println!("Decompile {:?}...", file_name);
        let executable = Executable::read_file(&file_name, false).unwrap();
        let mut workspace = Workspace::default();
        workspace.package.runtime = Some(executable.runtime);
        if workspace.compiler.is_none() {
            workspace.compiler = Some(CompilerData::default());
        }
        workspace.compiler.as_mut().unwrap().language_version = Some(executable.runtime);

        // Check compiler version in file name
        assert!(
            file_name.contains(executable.runtime.to_string().as_str()),
            "File name does not contain version number {}.",
            executable.runtime
        );

        println!("Run {:?}...", file_name);
        let output = run_executable(file_name, &executable);
        assert_eq!(&output, EXPECTED_OUTPUT);

        println!("Recompile {:?}...", file_name);
        let (dec_ast, _) = decompile(executable, false).unwrap();

        let reg = UserTypeRegistry::default();
        let input = dec_ast.to_string();
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let ast = icy_board_engine::parser::parse_ast(PathBuf::from(&file_name), errors.clone(), &input, &reg, Encoding::Utf8, &workspace);
        let mut compiler = PPECompiler::new(&workspace, reg, errors.clone());
        compiler.compile(&[&ast]);
        check_errors(errors.clone());
        println!("success.");
        match compiler.create_executable() {
            Ok(executable) => {
                println!("Generate bin {:?}...", file_name);
                let mut bin = executable.to_buffer().unwrap();
                println!("Reload bin {:?}...", file_name);
                let loaded_exectuable = Executable::from_buffer(&mut bin, false).unwrap();
                assert_eq!(loaded_exectuable.runtime, workspace.runtime(), "Reloaded version mismatch");
                let output = run_executable(file_name, &loaded_exectuable);
                assert_eq!(&output, EXPECTED_OUTPUT);
            }
            Err(err) => {
                panic!("Error creating executable: {}", err);
            }
        }
    }
}

fn run_executable(file_name: &str, executable: &Executable) -> String {
    let result = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let icy_board = icy_board_engine::icy_board::IcyBoard::new();
        let board: Arc<tokio::sync::Mutex<icy_board_engine::icy_board::IcyBoard>> = Arc::new(tokio::sync::Mutex::new(icy_board));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let node_states = bbs.lock().await.open_connections.clone();
        let (mut ui_connection, connection) = ChannelConnection::create_pair();

        let mut state = IcyBoardState::new(bbs, board, node_states, node, Box::new(connection)).await;
        let result = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let res = result.clone();
        let _ = std::thread::Builder::new().name("Terminal update".to_string()).spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let mut buffer = [0; 1024];
                loop {
                    let Ok(size) = ui_connection.read(&mut buffer).await else {
                        break;
                    };
                    if size == 0 {
                        break;
                    }
                    res.lock().await.extend(&buffer[0..size]);
                }
            });
        });
        if let Err(err) = state.run_executable(&file_name, None, executable.clone()).await {
            println!("Error while executing: {}", err);
        }
        thread::sleep(std::time::Duration::from_millis(50));
        let x = result.as_ref().lock().await.clone();
        x
    });

    let result = read_data_with_encoding_detection(&result).unwrap();
    result.replace("\r\n", "\n")
}

fn check_errors(errors: std::sync::Arc<std::sync::Mutex<icy_board_engine::parser::ErrorReporter>>) {
    if errors.lock().unwrap().has_errors() {
        for err in &errors.lock().unwrap().errors {
            println!("ERROR: {}", err.error);
        }
        panic!("Aborted due to errors.");
    }
}
