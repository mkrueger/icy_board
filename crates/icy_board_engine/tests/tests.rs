use std::{env, path::PathBuf, sync::Arc, thread};

use icy_board_engine::{
    compiler::PPECompiler,
    executable::{Executable, LAST_PPLC},
    icy_board::{bbs::BBS, state::IcyBoardState},
    parser::{Encoding, UserTypeRegistry},
};
use icy_net::{channel::ChannelConnection, Connection, ConnectionType};

#[test]
fn test_compiler() {
    use std::fs::{self};

    let mut data_path = env::current_dir().unwrap();
    data_path.push("src/test_data");
    //let mut success = 0;
    //let mut skipped = 0;
    for entry in fs::read_dir("tests/test_data").expect("Error reading test_data directory.") {
        let cur_entry = entry.unwrap().path();
        if cur_entry.extension().unwrap() != "pps" {
            continue;
        }

        let executable = fs::read_to_string(&cur_entry).unwrap();
        let mut out_path = cur_entry.clone();
        out_path.set_extension("out");
        let expected_output = fs::read_to_string(&out_path).unwrap();

        let file_name = cur_entry.file_name().unwrap().to_str().unwrap();
        run_test(file_name, &executable, &expected_output);
    }
}

fn run_test(file_name: &str, input: &str, output: &str) {
    println!("Test {}...", file_name);
    let reg = UserTypeRegistry::default();
    let (ast, errors) = icy_board_engine::parser::parse_ast(PathBuf::from(&file_name), input, &reg, Encoding::Utf8, LAST_PPLC);
    check_errors(errors.clone());
    let mut compiler = PPECompiler::new(LAST_PPLC, &reg, errors.clone());
    compiler.compile(&ast);
    check_errors(errors.clone());

    match compiler.create_executable(LAST_PPLC) {
        Ok(executable) => {
            // Save & load the executable this ensures that the vtable is correctly initialized.
            let mut bin = executable.to_buffer().unwrap();
            let executable = Executable::from_buffer(&mut bin, false).unwrap();

            let result = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
                let icy_board = icy_board_engine::icy_board::IcyBoard::new();
                let board: Arc<tokio::sync::Mutex<icy_board_engine::icy_board::IcyBoard>> = Arc::new(tokio::sync::Mutex::new(icy_board));
                let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
                let node_states = bbs.lock().await.open_connections.clone();
                let (mut ui_connection, connection) = ChannelConnection::create_pair();

                let mut state = IcyBoardState::new(bbs, board, node_states, node, Box::new(connection)).await;
                let result = Arc::new(tokio::sync::Mutex::new(String::new()));

                let res = result.clone();
                let _ = std::thread::Builder::new().name("Terminal update".to_string()).spawn(move || {
                    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                        let mut buffer = [0; 1024];
                        loop {
                            let Ok(size) = ui_connection.read(&mut buffer).await else {
                                break;
                            };
                            if size == 0 {
                                continue;
                            }
                            let s = std::str::from_utf8(&buffer[0..size]).unwrap();
                            res.lock().await.push_str(s);
                        }
                    });
                });
                state.run_executable(&file_name, None, executable.clone()).await.unwrap();
                thread::sleep(std::time::Duration::from_millis(10));
                let x = result.as_ref().lock().await.clone();
                x
            });
            let result = result.replace("\r\n", "\n");
            /*
            if result != output {
                println!("Input: {}", input);
                println!("------");
                executable.print_variable_table();
                executable.print_disassembler();
            }*/
            assert_eq!(result, output);
        }
        Err(err) => {
            panic!("Error creating executable: {}", err);
        }
    }
}

fn check_errors(errors: std::sync::Arc<std::sync::Mutex<icy_board_engine::parser::ErrorRepoter>>) {
    if errors.lock().unwrap().has_errors() {
        for err in &errors.lock().unwrap().errors {
            println!("ERROR: {}", err.error);
        }
        panic!("Aborted due to errors.");
    }
}
