use argh::FromArgs;
use ariadne::{Label, Report, ReportKind};

use codepages::tables::{write_cp437, write_utf8_with_bom};
use icy_board_engine::{
    ast::Ast,
    compiler::PPECompiler,
    icy_board::read_with_encoding_detection,
    parser::{load_with_encoding, parse_ast, Encoding, ErrorReporter, UserTypeRegistry},
    Res,
};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

use icy_engine::{Buffer, SaveOptions};
use package::Config;
use semver::Version;
pub mod package;
use std::{
    fs,
    io::stdout,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(FromArgs)]
/// PCBoard Programming Language Compiler  
struct Cli {
    /// output the disassembly instead of compiling
    #[argh(switch, short = 'd')]
    disassemble: bool,

    /// don't report any warnings
    #[argh(switch)]
    nowarnings: bool,

    /// version number for the compiled PPE, valid: 100, 200, 300, 310, 320, 330, 340, 400 (default)
    #[argh(option)]
    version: Option<u16>,

    /// version number for the language (defaults to version)
    #[argh(option)]
    lang_version: Option<u16>,

    /// specify the encoding of the file, defaults to autodetection
    #[argh(option)]
    cp437: Option<bool>,

    /// file[.pps] to compile (extension defaults to .pps if not specified)
    #[argh(positional)]
    file: Option<PathBuf>,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

fn main() {
    let arguments: Cli = argh::from_env();

    let version = if let Some(v) = arguments.version { v } else { 400 };
    let valid_versions: Vec<u16> = vec![100, 200, 300, 310, 320, 330, 340, 400];
    if !valid_versions.contains(&version) {
        println!("Invalid version number valid values {valid_versions:?}");
        return;
    }

    let lang_version: u16 = if let Some(v) = arguments.lang_version { v } else { version };
    let toml_f = PathBuf::from("ppl.toml");
    let file = arguments.file.as_ref().unwrap_or(&toml_f);

    let file_name = if file.extension().is_none() {
        file.with_extension("pps")
    } else {
        file.clone()
    };

    if !file.exists() {
        execute!(
            stdout(),
            SetAttribute(Attribute::Bold),
            SetForegroundColor(Color::Red),
            Print("File not found.".to_string()),
            SetAttribute(Attribute::Reset),
            SetAttribute(Attribute::Reset),
        )
        .unwrap();
        println!();
        println!();
        std::process::exit(1);
    }

    if file_name.extension().unwrap() == "toml" {
        if let Err(err) = compile_toml(&file_name, &arguments, version) {
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
            std::process::exit(1);
        }
        return;
    }

    println!();
    println!("Parsing...");

    let encoding = if let Some(cp437) = arguments.cp437 {
        if cp437 {
            Encoding::CP437
        } else {
            Encoding::Utf8
        }
    } else {
        Encoding::Detect
    };
    let out_file_name = Path::new(&file_name).with_extension("ppe");
    compile_files(&arguments, version, encoding, lang_version, vec![PathBuf::from(&file_name)], &out_file_name);
}

fn compile_toml(file_name: &PathBuf, arguments: &Cli, version: u16) -> Res<()> {
    let toml_str = fs::read_to_string(file_name)?;
    let config: Config = toml::from_str(&toml_str)?;

    let base_path = file_name.parent().unwrap();
    let encoding: Encoding = Encoding::Detect;
    let lang_version = config.package.language_version();
    
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(&base_path.join("src")).into_iter().flatten() {
        if !entry.path().is_file() {
            continue;
        }
        if let Some(ext) = entry.path().extension() {
            if ext != "pps" {
                continue;
            }
        }
        files.push(entry.path().to_path_buf());
    }

    let path = match version {
        100 => "pcboard_15.0",
        200 => "pcboard_15.10",
        300 => "pcboard_15.20",
        310 => "pcboard_15.21",
        320 => "pcboard_15.22",
        330 => "pcboard_15.30",
        340 => "pcboard_15.40",
        _ => "icboard",
    };
    let target_path = base_path.join("target").join(path);
    fs::create_dir_all(&target_path).expect("Unable to create target directory");
    let out_file_name = target_path.join(config.package.name()).with_extension("ppe");
    compile_files(arguments, version, encoding, lang_version, files, &out_file_name);
    println!("Copying data files...");
    for file in config.data.art_files {
        let src_file = base_path.join(&file);
        let out_file = target_path.join(&file);
        fs::create_dir_all(out_file.parent().unwrap())?;

        if src_file.extension().unwrap() == "icy" {
            let data = fs::read(&src_file)?;
            let mut buffer = Buffer::from_bytes(&src_file, true, &data, None, None).unwrap();
            let mut options = SaveOptions::default();
            options.modern_terminal_output = version > 340;
            let bytes = buffer.to_bytes("pcb", &options).unwrap();
            let out_file: PathBuf = out_file.with_extension("pcb");
            fs::write(out_file, bytes)?;
            continue;
        }

        let txt: String = read_with_encoding_detection(&src_file)?;
        if version <= 340 {
            write_cp437(&out_file, &txt)?;
        } else {
            write_utf8_with_bom(&out_file, &txt)?;
        }
    }

    for file in config.data.text_files {
        let src_file = base_path.join(&file);
        let out_file = target_path.join(&file);
        fs::create_dir_all(out_file.parent().unwrap())?;
        let txt = read_with_encoding_detection(&src_file)?;

        if version <= 340 {
            write_cp437(&out_file, &txt)?;
        } else {
            write_utf8_with_bom(&out_file, &txt)?;
        }
    }

    Ok(())
}

fn compile_files(arguments: &Cli, version: u16, encoding: Encoding, lang_version: u16, mut files: Vec<PathBuf>, out_file_name: &Path) {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    
    files.sort_by(|a, b| 
        if a.file_stem().unwrap() == "main" { 
            std::cmp::Ordering::Less
        } else if b.file_stem().unwrap() == "main" {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    );
    for f in &files {
        println!("Compiling: {:?}", f.display());
    }

    let reg = UserTypeRegistry::icy_board_registry();
    let mut asts = Vec::new();
    println!();
    println!("Parsing...");

    for src_file in files {
        match load_with_encoding(&src_file, encoding) {
            Ok(src) => {
                let ast = parse_ast(src_file.to_path_buf(), errors.clone(), &src, &reg, encoding, lang_version);
                asts.push((ast, src));
                if check_errors(errors.clone(), &arguments, &asts) {
                    std::process::exit(1);
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
                std::process::exit(1);
            }
        }
    }

    println!("Compiling...");
    let mut compiler = PPECompiler::new(version, &reg, errors.clone());
    compiler.compile(&asts.iter().map(|(ast, _)| ast).collect::<Vec<&Ast>>());
    if check_errors(errors.clone(), &arguments, &asts) {
        std::process::exit(1);
    }

    match compiler.create_executable(version) {
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
            //let len = bin.len();
            fs::write(out_file_name, bin).expect("Unable to write file");
            //let lines = src.lines().count();
            //println!("{} lines, {} chars compiled. {} bytes written to {:?}", lines, src.len(), len, &out_file_name);
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
            std::process::exit(1);
        }
    }
}

fn check_errors(errors: std::sync::Arc<std::sync::Mutex<icy_board_engine::parser::ErrorReporter>>, arguments: &Cli, src: &[(Ast, String)]) -> bool {
    if errors.lock().unwrap().has_errors() || (errors.lock().unwrap().has_warnings() && !arguments.nowarnings) {
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut cache = Vec::new();
        for (ast, txt) in src {
            cache.push((format!("{}", ast.file_name.display()), txt));
        }

        // let file_name = file_name.to_string_lossy().to_string();
        for err in &errors.lock().unwrap().errors {
            error_count += 1;
            let cache = ariadne::sources(cache.clone());
            Report::build(ReportKind::Error, (format!("{}", err.file_name.display()), err.span.clone()))
                .with_message(format!("{}", err.error))
                .with_label(Label::new((format!("{}", err.file_name.display()), err.span.clone())).with_color(ariadne::Color::Red))
                .finish()
                .print(cache)
                .unwrap();
        }

        if !arguments.nowarnings {
            for err in &errors.lock().unwrap().warnings {
                warning_count += 1;
                let cache = ariadne::sources(cache.clone());
                Report::build(ReportKind::Warning, (err.file_name.to_string_lossy().to_string(), err.span.clone()))
                    .with_message(format!("{}", err.error))
                    .with_label(Label::new((err.file_name.to_string_lossy().to_string(), err.span.clone())).with_color(ariadne::Color::Yellow))
                    .finish()
                    .print(cache)
                    .unwrap();
            }
            println!("{} errors, {} warnings", error_count, warning_count);
        } else {
            println!("{} errors", error_count);
        }
        return error_count > 0;
    }
    return false;
}
