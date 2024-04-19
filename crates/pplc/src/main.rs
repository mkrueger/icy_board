use ariadne::{Label, Report, ReportKind, Source};
use clap::Parser;
use icy_board_engine::{
    compiler::PPECompiler,
    parser::{load_with_encoding, parse_ast, Encoding, UserTypeRegistry},
    semantic::SemanticVisitor,
};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

use semver::Version;
use std::{
    fs,
    io::stdout,
    path::{Path, PathBuf},
};

/// PCBoard Programming Language Compiler  
#[derive(clap::Parser)]
#[command(version="", about="PCBoard Programming Language Compiler", long_about = None)]
struct Cli {
    /// output the disassembly instead of compiling
    #[arg(long, short)]
    disassemble: bool,

    /// force no user variables
    #[arg(long)]
    nouvar: bool,

    /// force user variables
    #[arg(long)]
    forceuvar: bool,

    /// don't report any warnings
    #[arg(long)]
    nowarnings: bool,

    /// version number for the compiler, valid: 100, 200, 300, 310, 330 (default), 340
    #[arg(long)]
    ppl_version: Option<u16>,

    /// version number for the runtime, valid: 100, 200, 300, 310, 330 (default), 340
    #[arg(long)]
    runtime_version: Option<u16>,

    /// input file is CP437
    #[arg(long)]
    dos: bool,

    /// file[.pps] to compile (extension defaults to .pps if not specified)
    file: PathBuf,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

fn main() {
    let arguments = Cli::parse();
    let ppl_version = if let Some(v) = arguments.ppl_version { v } else { 400 };
    let runtime_version = if let Some(v) = arguments.ppl_version { v } else { 330 };
    let valid_versions: Vec<u16> = vec![100, 200, 300, 310, 330, 340, 400];
    if !valid_versions.contains(&ppl_version) {
        println!("Invalid version number valid values {valid_versions:?}");
        return;
    }
    if !valid_versions.contains(&runtime_version) {
        println!("Invalid version number valid values {valid_versions:?}");
        return;
    }
    if arguments.nouvar && arguments.forceuvar {
        println!("--nouvar can't be used in conjunction with --forceuvar");
        return;
    }
    let file_name = if arguments.file.extension().is_none() {
        arguments.file.with_extension("pps")
    } else {
        arguments.file
    };

    let encoding = if arguments.dos { Encoding::CP437 } else { Encoding::Utf8 };

    match load_with_encoding(&PathBuf::from(&file_name), encoding) {
        Ok(src) => {
            println!();
            println!("Parsing...");
            let reg = UserTypeRegistry::icy_board_registry();
            let (mut ast, errors) = parse_ast(PathBuf::from(&file_name), &src, &reg, encoding, ppl_version);
            if arguments.nouvar {
                ast.require_user_variables = false;
            }
            if arguments.forceuvar {
                ast.require_user_variables = true;
            }
            println!("Compiling...");

            let mut sv = SemanticVisitor::new(ppl_version, errors, &reg);
            ast.visit(&mut sv);

            let errors = sv.errors.clone();

            if errors.lock().unwrap().has_errors() || (errors.lock().unwrap().has_warnings() && !arguments.nowarnings) {
                let mut error_count = 0;
                let mut warning_count = 0;
                let file_name = file_name.to_string_lossy().to_string();
                for err in &errors.lock().unwrap().errors {
                    error_count += 1;
                    Report::build(ReportKind::Error, &file_name, err.span.start)
                        .with_code(error_count)
                        .with_message(format!("{}", err.error))
                        .with_label(Label::new((&file_name, err.span.clone())).with_color(ariadne::Color::Red))
                        .finish()
                        .print((&file_name, Source::from(&src)))
                        .unwrap();
                }

                if !arguments.nowarnings {
                    for err in &errors.lock().unwrap().warnings {
                        warning_count += 1;
                        Report::build(ReportKind::Warning, &file_name, err.span.start)
                            .with_code(warning_count)
                            .with_message(format!("{}", err.error))
                            .with_label(Label::new((&file_name, err.span.clone())).with_color(ariadne::Color::Yellow))
                            .finish()
                            .print((&file_name, Source::from(&src)))
                            .unwrap();
                    }
                    println!("{} errors, {} warnings", error_count, warning_count);
                } else {
                    println!("{} errors", error_count);
                }
                if errors.lock().unwrap().has_errors() {
                    return;
                }
            }
            println!();
            let mut compiler = PPECompiler::new(ppl_version, &reg);
            compiler.compile(&ast);

            match compiler.create_executable(runtime_version) {
                Ok(executable) => {
                    if arguments.disassemble {
                        println!();
                        executable.print_variable_table();
                        println!();
                        let mut visitor = icy_board_engine::executable::disassembler::DisassembleVisitor::new(&executable);
                        visitor.generate_statement_data = true;
                        compiler.get_script().visit(&mut visitor);
                        println!();
                        println!("Generated:");
                        executable.print_script_buffer_dump();
                        println!();
                        return;
                    }

                    let bin = executable.to_buffer().unwrap();
                    let out_file_name = Path::new(&file_name).with_extension("ppe");
                    let len = bin.len();
                    fs::write(&out_file_name, bin).expect("Unable to write file");
                    let lines = src.lines().count();
                    println!("{} lines, {} chars compiled. {} bytes written to {:?}", lines, src.len(), len, &out_file_name);
                }
                Err(err) => {
                    execute!(
                        stdout(),
                        SetAttribute(Attribute::Bold),
                        SetForegroundColor(Color::Red),
                        Print("ERROR: ".to_string()),
                        SetAttribute(Attribute::Reset),
                        SetAttribute(Attribute::Bold),
                        Print(format!("{}", err)),
                        SetAttribute(Attribute::Reset),
                    )
                    .unwrap();
                    println!();
                    println!();
                }
            }
        }
        Err(err) => {
            execute!(
                stdout(),
                SetAttribute(Attribute::Bold),
                SetForegroundColor(Color::Red),
                Print("ERROR: ".to_string()),
                SetAttribute(Attribute::Reset),
                SetAttribute(Attribute::Bold),
                Print(format!("{}", err)),
                SetAttribute(Attribute::Reset),
            )
            .unwrap();
            println!();
            println!();
        }
    }
}
