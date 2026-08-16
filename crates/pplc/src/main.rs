use argh::FromArgs;
use ariadne::{Label, Report, ReportKind};

use codepages::tables::UNICODE_TO_CP437;
use icy_board_engine::{
    Res,
    ast::Ast,
    compiler::{
        PPECompiler,
        workspace::{CompilerData, Package, Workspace},
    },
    executable::{LAST_PPL_LANGUAGE_VERSION, SUPPORTED_PPE_VERSIONS, SUPPORTED_PPL_LANGUAGE_VERSIONS, language_version_from_env},
    formatting::{FormattingVisitor, StringFormattingBackend},
    icy_board::{read_with_encoding_detection, write_atomic},
    parser::{
        Encoding, ErrorReporter, UserTypeRegistry, lexer::scan_language_version, load_with_encoding, parse_ast_with_predeclared_types,
        preparse_type_declarations,
    },
};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

use icy_engine::SaveOptions;
use icy_engine::formats::{CharacterFormatOptions, FileFormat, FormatOptions};
use semver::Version;
use std::{
    fs::{self},
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

    /// version number for the compiled PPE, valid: 100, 200, 300, 310, 320, 330, 340, 400, 401 (default)
    #[argh(option)]
    runtime: Option<u16>,

    /// language version (defaults to the manifest, PPL_LANG_VERSION, then runtime capped at 400)
    #[argh(option)]
    lang_version: Option<u16>,

    /// specify the encoding of the file (cp437 = true, utf8 = false), defaults to autodetection
    #[argh(switch)]
    cp437: Option<bool>,

    /// create & init new ppl package in target directory
    #[argh(switch)]
    init: bool,

    /// semicolon separated list of pre processor variables
    #[argh(option)]
    defines: Option<String>,

    /// formats source file instead of compile
    #[argh(switch)]
    format: bool,

    /// with --format, write the result to stdout and leave the file alone
    #[argh(switch)]
    stdout: bool,

    /// checks source/package for errors without compiling
    #[argh(switch)]
    check: bool,

    /// file[.pps] to compile (extension defaults to .pps if not specified)
    #[argh(positional)]
    file: Option<PathBuf>,
}

lazy_static::lazy_static! {
    static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
}

fn main() {
    let arguments: Cli = argh::from_env();
    // With --stdout the formatted source is the output, so nothing else may go there.
    if arguments.stdout {
        eprintln!("PPLC v{} - PCBoard Programming Language Compiler", *VERSION);
    } else {
        println!("PPLC v{} - PCBoard Programming Language Compiler", *VERSION);
    }

    if let Some(version) = arguments.runtime {
        if !SUPPORTED_PPE_VERSIONS.contains(&version) {
            eprintln!("Invalid version number valid values {SUPPORTED_PPE_VERSIONS:?}");
            std::process::exit(2);
        }
    }
    if let Some(version) = arguments.lang_version {
        if !SUPPORTED_PPL_LANGUAGE_VERSIONS.contains(&version) {
            eprintln!("Invalid language version valid values {SUPPORTED_PPL_LANGUAGE_VERSIONS:?}");
            std::process::exit(2);
        }
    }
    if arguments.init {
        let Some(file) = arguments.file.clone() else {
            eprintln!("No target directory specified.");
            std::process::exit(2);
        };

        if file.exists() {
            eprintln!("Target directory already exists.");
            std::process::exit(1);
        }
        let src_dir = file.join("src");
        if let Err(err) = init_package(&file, &src_dir, &arguments) {
            eprintln!("ERROR: {err}");
            let _ = fs::remove_dir_all(&file);
            std::process::exit(1);
        }
        println!("Created new ppl package in {}", file.display());
        return;
    }

    let toml_f = PathBuf::from("ppl.toml");
    let file = arguments.file.as_ref().unwrap_or(&toml_f);

    let file_name = if file.extension().is_none() {
        file.with_extension("pps")
    } else {
        file.clone()
    };

    if !file_name.exists() {
        if arguments.file.is_none() {
            if let Err(err) = Cli::from_args(&["pplc"], &["--help"]) {
                eprintln!("{}", err.output);
            }
        } else {
            eprintln!("ERROR: {} not found on disk, aborting...", file_name.display());
        }
        std::process::exit(1);
    }

    if file_name.extension().is_some_and(|extension| extension == "toml") {
        if let Err(err) = compile_toml(&file_name, &arguments) {
            eprintln!("ERROR: {err}");
            std::process::exit(1);
        }
        return;
    }

    if !(arguments.format || arguments.check) {
        println!();
        println!("Parsing...");
    }

    let encoding = if let Some(cp437) = arguments.cp437 {
        if cp437 { Encoding::CP437 } else { Encoding::Utf8 }
    } else {
        Encoding::Detect
    };
    let out_file_name = Path::new(&file_name).with_extension("ppe");

    let mut ws = Workspace::default();
    ws.hard_coded_files = Some(vec![PathBuf::from(&file_name)]);
    apply_arguments(&mut ws, &arguments);

    if let Err(err) = compile_files(&arguments, encoding, &mut ws, &out_file_name) {
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}

fn init_package(file: &Path, src_dir: &Path, arguments: &Cli) -> Res<()> {
    fs::create_dir_all(src_dir)?;
    write_atomic(src_dir.join("main.pps"), b"PRINTLN \"Hello, World!\"")?;

    let mut ws = Workspace::default();
    ws.file_name = file.to_path_buf();
    ws.package = Package {
        name: file.file_name().and_then(|name| name.to_str()).unwrap_or("package").to_string(),
        runtime: None,
        version: Version::new(0, 1, 0),
        authors: None,
    };
    ws.compiler = Some(CompilerData {
        language_version: Some(arguments.lang_version.or_else(read_environment).unwrap_or(LAST_PPL_LANGUAGE_VERSION)),
        defines: if let Some(defines) = &arguments.defines {
            Some(defines.split(';').map(|s| s.to_string()).collect())
        } else {
            None
        },
    });
    ws.save(file.join("ppl.toml"))?;
    Ok(())
}

fn read_environment() -> Option<u16> {
    match language_version_from_env() {
        Ok(version) => version,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}

/// A manifest and the command line are explicit, so the user's environment only
/// fills in what neither of them decided.
fn apply_environment(workspace: &mut Workspace, arguments: &Cli) {
    if arguments.lang_version.is_none() && workspace.compiler.as_ref().and_then(|compiler| compiler.language_version).is_none() {
        workspace.compiler.get_or_insert_with(CompilerData::default).language_version = read_environment();
    }
}

/// The command line wins over the manifest, and a package that has none still needs these.
fn apply_arguments(workspace: &mut Workspace, arguments: &Cli) {
    if let Some(runtime) = arguments.runtime {
        workspace.package.runtime = Some(runtime);
    }
    if let Some(lang_version) = arguments.lang_version {
        workspace.compiler.get_or_insert_with(CompilerData::default).language_version = Some(lang_version);
    }
    if let Some(defines) = &arguments.defines {
        workspace.compiler.get_or_insert_with(CompilerData::default).defines = Some(defines.split(';').map(str::to_string).collect());
    }
}

/// A source states which language it is written in, so it wins over the manifest and
/// the command line. Two files may not disagree about it.
fn apply_declared_language_version(workspace: &mut Workspace, encoding: Encoding) -> Res<()> {
    let mut declared: Option<(PathBuf, u16)> = None;
    for src_file in workspace.files() {
        let Ok(src) = load_with_encoding(&src_file, encoding) else {
            continue;
        };
        let Some(version) = scan_language_version(&src) else {
            continue;
        };
        if let Some((other_file, other_version)) = &declared {
            if *other_version != version {
                return Err(format!(
                    "{} declares language version {} while {} declares {}",
                    src_file.display(),
                    version,
                    other_file.display(),
                    other_version
                )
                .into());
            }
        } else {
            declared = Some((src_file, version));
        }
    }

    if let Some((file, version)) = declared {
        let configured = workspace.compiler.as_ref().and_then(|c| c.language_version);
        if configured.is_some_and(|configured| configured != version) {
            println!("{} declares language version {}, using that one.", file.display(), version);
        }
        workspace.compiler.get_or_insert_with(CompilerData::default).language_version = Some(version);
    }
    Ok(())
}

fn compile_toml(file_name: &PathBuf, arguments: &Cli) -> Res<()> {
    let mut workspace = Workspace::load(file_name)?;
    apply_arguments(&mut workspace, arguments);

    let base_path = file_name.parent().unwrap_or_else(|| Path::new("."));
    let encoding: Encoding = Encoding::Detect;

    let target_path = workspace.target_path(workspace.runtime());
    fs::create_dir_all(&target_path)?;

    let out_file_name = target_path.join(workspace.package.name()).with_extension("ppe");
    compile_files(arguments, encoding, &mut workspace, &out_file_name)?;
    println!("Copying data files...");
    if let Some(data) = &workspace.data {
        if let Some(art_files) = &data.art_files {
            for file in art_files {
                let src_file = base_path.join(&file);
                let out_file = target_path.join(&file);
                fs::create_dir_all(out_file.parent().unwrap())?;

                if src_file.extension().is_some_and(|extension| extension == "icy") {
                    let data = fs::read(&src_file)?;
                    let format = FileFormat::from_extension("icy").ok_or("ICY format is unavailable")?;
                    let loaded = format.from_bytes(&data, None)?;
                    let options = SaveOptions {
                        format: FormatOptions::Character(CharacterFormatOptions::default()),
                        ..Default::default()
                    };
                    let bytes = FileFormat::PCBoard.to_bytes(&loaded.screen.buffer, &options)?;
                    let out_file: PathBuf = out_file.with_extension("pcb");
                    write_atomic(out_file, &bytes)?;
                    continue;
                }

                let txt = read_with_encoding_detection(&src_file)?;
                if workspace.runtime() <= 340 {
                    write_atomic(&out_file, &encode_cp437(&txt))?;
                } else {
                    write_atomic(&out_file, &encode_utf8(&txt))?;
                }
            }
        }
        if let Some(text_files) = &data.text_files {
            for file in text_files {
                let src_file = base_path.join(&file);
                let out_file = target_path.join(&file);
                fs::create_dir_all(out_file.parent().unwrap())?;
                let txt = read_with_encoding_detection(&src_file)?;

                if workspace.runtime() <= 340 {
                    write_atomic(&out_file, &encode_cp437(&txt))?;
                } else {
                    write_atomic(&out_file, &encode_utf8(&txt))?;
                }
            }
        }
    }

    Ok(())
}

/// Writes beside the source and renames, so a failure cannot leave a half file.
fn write_formatted(file: &Path, text: &str) -> Res<()> {
    write_atomic(file, text.as_bytes())?;
    Ok(())
}

fn compile_files(arguments: &Cli, encoding: Encoding, workspace: &mut Workspace, out_file_name: &Path) -> Res<()> {
    apply_declared_language_version(workspace, encoding)?;
    apply_environment(workspace, arguments);
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));

    let reg = UserTypeRegistry::icy_board_registry();
    let mut asts = Vec::new();
    if !(arguments.format || arguments.check) {
        println!();
        println!("Parsing...");
    }
    let mut exit_code = 0;

    for src_file in workspace.files() {
        if let Ok(src) = load_with_encoding(&src_file, encoding) {
            preparse_type_declarations(src_file, errors.clone(), &src, &reg, encoding, workspace);
        }
    }

    for src_file in workspace.files() {
        match load_with_encoding(&src_file, encoding) {
            Ok(src) => {
                let ast = parse_ast_with_predeclared_types(src_file.to_path_buf(), errors.clone(), &src, &reg, encoding, workspace);
                if arguments.format || arguments.check {
                    let mut backend = StringFormattingBackend::new(&src);
                    let mut visitor = FormattingVisitor::new(&mut backend, workspace.formatting());
                    visitor.format(&ast);
                    if !backend.edits.is_empty() {
                        let formatted_text = backend.apply();
                        let mut last_line = 0;
                        if arguments.check {
                            let lines = diff::lines(&src, &formatted_text);
                            if lines.iter().any(|l| matches!(l, diff::Result::Left(_) | diff::Result::Right(_))) {
                                exit_code = 1;
                                println!("Diff in {}", src_file.display());
                                for (i, diff) in lines.iter().enumerate() {
                                    let mut block_start = false;
                                    let mut block_end = false;

                                    if i + 1 < lines.len() {
                                        if matches!(lines[i], diff::Result::Both(_, _)) && !matches!(lines[i + 1], diff::Result::Both(_, _)) {
                                            block_start = true;
                                            block_end = false;
                                        } else if i > 0 && !matches!(lines[i - 1], diff::Result::Both(_, _)) {
                                            block_end = true;
                                        }
                                    }
                                    match diff {
                                        diff::Result::Left(l) => {
                                            last_line = i;

                                            execute!(
                                                stdout(),
                                                Print(format!("{i:>3}:")),
                                                SetForegroundColor(Color::Red),
                                                Print(format!("-{}\n", l)),
                                                SetAttribute(Attribute::Reset),
                                            )
                                            .unwrap()
                                        }
                                        diff::Result::Both(l, _) => {
                                            if block_start || block_end {
                                                if last_line + 1 < i {
                                                    println!();
                                                }
                                                last_line = i;
                                                println!("{i:>3}: {}", l)
                                            };
                                        }
                                        diff::Result::Right(r) => {
                                            last_line = i;

                                            execute!(
                                                stdout(),
                                                Print(format!("{i:>3}:")),
                                                SetForegroundColor(Color::Green),
                                                Print(format!("+{}\n", r)),
                                                SetAttribute(Attribute::Reset),
                                            )
                                            .unwrap()
                                        }
                                    }
                                }
                            }
                        } else if arguments.stdout {
                            print!("{formatted_text}");
                        } else {
                            write_formatted(&src_file, &formatted_text)?;
                        }
                    } else if arguments.stdout {
                        print!("{src}");
                    }
                    if arguments.format {
                        continue;
                    }
                }
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
    if arguments.format {
        if exit_code != 0 {
            std::process::exit(exit_code);
        }
        return Ok(());
    }

    // --check runs the compiler for its diagnostics, it just keeps the result.
    if !arguments.check {
        println!("Compiling...");
    }
    let mut compiler = PPECompiler::new(&workspace, reg, errors.clone());
    compiler.compile(&asts.iter().map(|(ast, _)| ast).collect::<Vec<&Ast>>());
    if check_errors(errors.clone(), &arguments, &asts) {
        std::process::exit(1);
    }
    if arguments.check {
        if exit_code != 0 {
            std::process::exit(exit_code);
        }
        return Ok(());
    }

    match compiler.create_executable() {
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
                return Ok(());
            }

            let bin = executable.to_buffer()?;
            //let len = bin.len();
            write_atomic(out_file_name, &bin)?;
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
    Ok(())
}

fn encode_utf8(text: &str) -> Vec<u8> {
    let mut data = vec![0xEF, 0xBB, 0xBF];
    data.extend_from_slice(text.as_bytes());
    data
}

fn encode_cp437(text: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(text.len());
    for c in text.chars() {
        if c == '\r' {
            continue;
        }
        if c == '\n' {
            data.push(b'\r');
        }
        data.push(UNICODE_TO_CP437.get(&c).copied().unwrap_or(b'.'));
    }
    data
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
