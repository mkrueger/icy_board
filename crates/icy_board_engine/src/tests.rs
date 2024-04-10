use std::{
    env,
    fs::read_to_string,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_ppe::{
    compiler::PPECompiler,
    executable::{Executable, LAST_PPLC},
    parser::{parse_ast, Encoding},
    semantic::SemanticVisitor,
    Res,
};

use crate::{
    icy_board::state::IcyBoardState,
    vm::{run, BoardIO, MemoryIO},
};
#[test]
fn test_compiler() {
    use std::fs::{self};

    let mut data_path = env::current_dir().unwrap();
    data_path.push("src/test_data");
    //let mut success = 0;
    //let mut skipped = 0;
    for entry in fs::read_dir(data_path).expect("Error reading test_data directory.") {
        let cur_entry = entry.unwrap().path();

        if cur_entry.extension().unwrap() != "pps" {
            continue;
        }

        let file_name = cur_entry.as_os_str();
        println!(
            "File: {}...",
            cur_entry.file_name().unwrap().to_str().unwrap()
        );

        let mut data = String::new();
        let mut expected = String::new();
        let mut read_expected = false;
        for line in read_to_string(file_name).unwrap().lines() {
            if read_expected {
                expected.push_str(line);
                expected.push('\n');
                continue;
            }
            if line.starts_with(";;") {
                read_expected = true;
            } else {
                data.push_str(line);
                data.push('\n');
            }
        }

        run_test(&data, expected.trim_end());
    }
}

fn run_test(data: &str, output: &str) {
    let (ast, errors) = parse_ast(PathBuf::from("."), data, Encoding::Utf8, LAST_PPLC);

    let mut sv = SemanticVisitor::new(LAST_PPLC, errors);
    ast.visit(&mut sv);
    if sv.errors.lock().unwrap().has_errors() {
        println!("Errors:");
        for e in &sv.errors.lock().unwrap().errors {
            println!("{}", e.error);
        }
        panic!();
    }

    let mut exec = PPECompiler::default();
    exec.compile(&ast);
    let binary = exec.create_executable(330).unwrap();
    let mut buffer: Vec<u8> = binary.to_buffer().unwrap();
    let exe = Executable::from_buffer(&mut buffer, false).unwrap();
    let mut io = MemoryIO::new();
    let output_str = Arc::new(Mutex::new(String::new()));

    let mut state = IcyBoardState::default();
    state.ctx = Arc::new(Mutex::new(TestContext::new(output_str.clone())));
    run(&".", &exe, &mut io, &mut state).unwrap();

    let error = output != output_str.lock().unwrap().to_string().trim_end();
    if error {
        println!("{output}");
        println!("------------Was:");
        println!("{}", output_str.lock().unwrap());
        assert!(!error);
    }
}

struct TestContext {
    output: Arc<Mutex<String>>,
}
impl TestContext {
    pub fn new(output: Arc<Mutex<String>>) -> Self {
        Self { output }
    }
}

impl BoardIO for TestContext {
    fn get_char(&mut self) -> Res<Option<(bool, char)>> {
        todo!()
    }

    fn write_raw(&mut self, data: &[char]) -> Res<()> {
        self.output.lock().unwrap().extend(data);
        Ok(())
    }

    fn read(&mut self) -> Res<String> {
        Ok(String::new())
    }

    fn inbytes(&mut self) -> i32 {
        0
    }

    fn hangup(&mut self) -> Res<()> {
        Ok(())
    }

    fn put_keyboard_buffer(&mut self, _value: &[char]) -> Res<()> {
        Ok(())
    }
}
