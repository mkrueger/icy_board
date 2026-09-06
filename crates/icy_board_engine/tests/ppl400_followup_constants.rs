//! F4/F7/F8: direct (LSP) semantics, compiler lowering and serialized execution.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{Ast, convert_const_declaration},
    compiler::{PPECompiler, lower_modules, workspace::Workspace},
    executable::{Executable, VariableType, VariableValue},
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast_with_predeclared_types, preparse_type_declarations},
    semantic::SemanticVisitor,
    vm::{self, io::DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

const BITS: &str = "ENUM Bits\n One=1\n Two=2\nENDENUM\n";

fn messages(errors: &Arc<Mutex<ErrorReporter>>) -> Vec<String> {
    errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect()
}

fn parse(sources: &[(&str, &str)], language: u16) -> (Workspace, UserTypeRegistry, Arc<Mutex<ErrorReporter>>, Vec<Ast>) {
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(language));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    for (file, source) in sources {
        preparse_type_declarations(PathBuf::from(file), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    }
    let asts = sources
        .iter()
        .map(|(file, source)| parse_ast_with_predeclared_types(PathBuf::from(file), errors.clone(), source, &registry, Encoding::Utf8, &workspace))
        .collect();
    assert!(messages(&errors).is_empty(), "parse: {:?}; {sources:?}", messages(&errors));
    (workspace, registry, errors, asts)
}

fn compile(sources: &[(&str, &str)], language: u16) -> Result<Executable, Vec<String>> {
    let (workspace, registry, errors, asts) = parse(sources, language);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&asts.iter().collect::<Vec<_>>());
    let diagnostics = messages(&errors);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    compiler.create_executable().map_err(|error| vec![error.to_string()])
}

fn semantic(sources: &[(&str, &str)], language: u16) -> Vec<String> {
    let (workspace, registry, errors, asts) = parse(sources, language);
    let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
    visitor.set_modules(&asts.iter().collect::<Vec<_>>());
    let lowered = lower_modules(&asts.iter().collect::<Vec<_>>(), errors.clone(), &visitor.type_registry);
    visitor.prepare_legacy_call_signatures(&lowered.iter().collect::<Vec<_>>());
    for ast in lowered
        .iter()
        .filter(|ast| ast.module.is_some())
        .chain(lowered.iter().filter(|ast| ast.module.is_none()))
    {
        visitor.set_file_name(&ast.file_name);
        ast.visit(&mut visitor);
    }
    visitor.finish();
    messages(&errors)
}

fn run(executable: Executable) -> String {
    let executable = Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap();
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.root_path = directory.path().to_path_buf();
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
            let result = vm::run(&PathBuf::from("constants.ppe"), &executable, &mut io, &mut state).await;
            drop(state);
            result.unwrap();
        };
        let (output, ()) = tokio::join!(reader, execute);
        output
    })
}

fn succeeds(sources: &[(&str, &str)], language: u16, expected: &str) {
    assert_eq!(Vec::<String>::new(), semantic(sources, language), "LSP language={language}: {sources:?}");
    let executable = compile(sources, language).unwrap_or_else(|errors| panic!("compiler language={language}: {errors:?}; {sources:?}"));
    assert_eq!(expected, run(executable), "language={language}: {sources:?}");
}

fn rejects(sources: &[(&str, &str)], language: u16, diagnostic: &str) {
    let compiler = compile(sources, language).err().unwrap_or_else(|| panic!("compiler accepted {sources:?}"));
    for (path, errors) in [("compiler", compiler), ("LSP", semantic(sources, language))] {
        assert!(
            errors.iter().any(|error| error.to_ascii_lowercase().contains(&diagnostic.to_ascii_lowercase())),
            "{path} language={language}: expected {diagnostic:?}, got {errors:?}; {sources:?}"
        );
    }
}

#[test]
fn source400_integer_const_boundaries_are_checked_even_when_unused() {
    for (kind, minimum, maximum, below, above) in [
        ("BYTE", "0", "255", "-1", "256"),
        ("SBYTE", "-128", "127", "-129", "128"),
        ("WORD", "0", "65535", "-1", "65536"),
        ("SWORD", "-32768", "32767", "-32769", "32768"),
        ("INTEGER", "-2147483648", "2147483647", "-2147483649", "2147483648"),
        ("UNSIGNED", "0", "4294967295", "-1", "4294967296"),
        (
            "LONG",
            "-9223372036854775808",
            "9223372036854775807",
            "-9223372036854775809",
            "9223372036854775808",
        ),
        ("ULONG", "0", "18446744073709551615", "-1", "\"18446744073709551616\""),
    ] {
        for value in [minimum, maximum] {
            let source = format!("CONST {kind} N={value}\nCONST {kind} Alias=N\nPRINT N,\"|\",Alias\n");
            succeeds(&[("main.pps", &source)], 400, &format!("{value}|{value}"));
        }
        for value in [below, above] {
            let source = format!("CONST {kind} N={value}\n");
            // An unrepresentable negative literal may fail constant evaluation
            // before declaration conversion. Either path must diagnose it.
            rejects(&[("main.pps", &source)], 400, "constant");
        }
    }
    rejects(&[("main.pps", "CONST BYTE N=257\nCONST INTEGER M=N\nPRINT N, M\n")], 400, "range");
}

#[test]
fn finite_float_and_money_const_bounds_are_checked() {
    for (kind, value) in [
        ("REAL", "\"3.5e38\""),
        ("REAL", "\"-3.5e38\""),
        ("REAL", "\"NaN\""),
        ("DOUBLE", "\"inf\""),
        ("DOUBLE", "\"-inf\""),
        ("DOUBLE", "\"1e309\""),
        ("MONEY", "2147483648"),
        ("MONEY", "-2147483649"),
    ] {
        rejects(&[("main.pps", &format!("CONST {kind} N={value}\n"))], 400, "range");
    }
    for kind in [
        VariableType::Byte,
        VariableType::SByte,
        VariableType::Word,
        VariableType::SWord,
        VariableType::Integer,
        VariableType::Unsigned,
        VariableType::Long,
        VariableType::ULong,
        VariableType::Money,
        VariableType::Float,
        VariableType::Double,
    ] {
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                convert_const_declaration(VariableValue::new_double(number), kind, 400).is_none(),
                "{kind} {number}"
            );
        }
    }
    for number in [f64::MAX, -f64::MAX, f64::MIN_POSITIVE, 0.0] {
        assert!(convert_const_declaration(VariableValue::new_double(number), VariableType::Double, 400).is_some());
    }
    for number in [f32::MAX as f64, -(f32::MAX as f64), f32::MIN_POSITIVE as f64, 0.0] {
        assert!(convert_const_declaration(VariableValue::new_double(number), VariableType::Float, 400).is_some());
    }
}

#[test]
fn fractions_keep_the_existing_integer_conversion_policy() {
    for kind in [
        VariableType::Byte,
        VariableType::SByte,
        VariableType::Word,
        VariableType::SWord,
        VariableType::Integer,
        VariableType::Unsigned,
    ] {
        for number in [0.5, 1.5, 126.75] {
            let value = VariableValue::new_double(number);
            assert_eq!(value.clone().convert_to(kind), convert_const_declaration(value, kind, 400).unwrap());
        }
    }
    for kind in [VariableType::SByte, VariableType::SWord, VariableType::Integer] {
        let value = VariableValue::new_double(-1.5);
        assert_eq!(value.clone().convert_to(kind), convert_const_declaration(value, kind, 400).unwrap());
    }
    succeeds(
        &[(
            "main.pps",
            &format!("{BITS}CONST INTEGER N=1.5\nCONST INTEGER M=N\nCONST Bits Value=Bits(M)\nPRINT N,\"|\",M,\"|\",Value\n"),
        )],
        400,
        "1|1|1",
    );
    for kind in ["BYTE", "WORD", "UNSIGNED", "ULONG"] {
        rejects(&[("main.pps", &format!("CONST {kind} N=-0.5\n"))], 400, "range");
    }
}

#[test]
fn legacy_const_wrapping_and_ordinary_assignments_remain_unchanged() {
    succeeds(&[("main.pps", "CONST BYTE N=257\nCONST INTEGER M=N\nPRINT N,\"|\",M\n")], 350, "1|1");
    succeeds(&[("main.pps", "BYTE N=257\nPRINT N\n")], 400, "1");
    for kind in [
        VariableType::Byte,
        VariableType::SByte,
        VariableType::Word,
        VariableType::SWord,
        VariableType::Integer,
        VariableType::Unsigned,
    ] {
        let value = VariableValue::new_int(65537);
        assert_eq!(value.clone().convert_to(kind), convert_const_declaration(value, kind, 350).unwrap());
    }
}

#[test]
fn module_aliases_and_local_constants_use_declared_converted_values() {
    for language in [350, 400] {
        succeeds(
            &[
                (
                    "main.pps",
                    "IMPORT Numbers AS N\nIMPORT Values AS V\nPRINT N.Whole,\"|\",V.Alias,\"|\",V.Initial,\"|\"\nShow()\nPROCEDURE Show()\nCONST INTEGER Whole=2.75\nCONST INTEGER Alias=Whole\nPRINT Whole,\"|\",Alias\nENDPROC\n",
                ),
                ("numbers.pps", "MODULE Numbers\nCONST INTEGER Whole=1.5\nENDMODULE\n"),
                (
                    "values.pps",
                    "IMPORT Numbers AS N\nMODULE Values\nCONST INTEGER Alias=N.Whole\nINTEGER Initial=Alias\nENDMODULE\n",
                ),
            ],
            language,
            "1|1|1|2|2",
        );
    }
    rejects(
        &[
            ("main.pps", "IMPORT Numbers AS N\nPRINT N.Bad\n"),
            ("numbers.pps", "MODULE Numbers\nCONST BYTE Bad=257\nENDMODULE\n"),
        ],
        400,
        "range",
    );
}

#[test]
fn enum_type_receivers_are_not_replaced_by_same_named_constants() {
    for language in [350, 400] {
        succeeds(
            &[(
                "main.pps",
                &format!(
                    "{BITS}CONST INTEGER Bits=7\nCONST Bits Member=Bits.One\nPRINT Bits,\"|\",Bits.One,\"|\",Bits.One.Has(Bits.One),\"|\",Bits(1),\"|\",Member\n"
                ),
            )],
            language,
            "7|1|1|1|1",
        );
        succeeds(
            &[
                (
                    "main.pps",
                    "IMPORT Flags AS F\nPRINT F.Bits,\"|\",F.Bits.One,\"|\",F.Bits.One.Has(F.Bits.One),\"|\",F.Bits(1)\n",
                ),
                ("flags.pps", &format!("MODULE Flags\n{BITS}CONST INTEGER Bits=7\nENDMODULE\n")),
            ],
            language,
            "7|1|1|1",
        );
    }
}

#[test]
fn rgb_cannot_erase_enum_types_in_any_argument_or_nested_cast() {
    for language in [350, 400] {
        for arguments in ["Bits.One,0,0", "0,Bits.One,0", "0,0,Bits.One", "0,0,0,Bits.One", "ToByte(Bits.One),0,0"] {
            for statement in [format!("CONST UNSIGNED Packed=RGB({arguments})"), format!("PRINT RGB({arguments})")] {
                let diagnostic = if statement.starts_with("CONST") {
                    "constant"
                } else if language < 400 {
                    "not supported"
                } else {
                    "TOINTEGER"
                };
                rejects(&[("main.pps", &format!("{BITS}{statement}\n"))], language, diagnostic);
            }
        }
        succeeds(
            &[("main.pps", &format!("{BITS}CONST UNSIGNED Packed=RGB(TOINTEGER(Bits.One),0,0)\nPRINT Packed\n"))],
            language,
            "16777471",
        );
    }
}

#[test]
fn declared_numeric_types_survive_substitution_before_checked_enum_casts() {
    for kind in [
        "BYTE", "SBYTE", "WORD", "SWORD", "UNSIGNED", "REAL", "DOUBLE", "MONEY", "LONG", "ULONG", "DATE", "EDATE", "DDATE", "TIME",
    ] {
        for statement in ["PRINT Bits(N)", "CONST Bits Member=Bits(N)"] {
            let diagnostic = if statement.starts_with("CONST") { "constant" } else { "INTEGER" };
            rejects(&[("main.pps", &format!("{BITS}CONST {kind} N=1\n{statement}\n"))], 400, diagnostic);
        }
    }
    for language in [350, 400] {
        succeeds(
            &[(
                "main.pps",
                &format!("{BITS}CONST SWORD N=1\nCONST Bits Member=Bits(TOINTEGER(N))\nPRINT Member,\"|\",Bits(TOINTEGER(N))\n"),
            )],
            language,
            "1|1",
        );
        rejects(
            &[("main.pps", &format!("{BITS}CONST SWORD N=-1\nCONST Bits Member=Bits(-N)\n"))],
            language,
            "constant",
        );
        rejects(&[("main.pps", &format!("{BITS}CONST Bits Member=Bits($0.01)\n"))], language, "constant");
    }
}

#[test]
fn string_constants_keep_the_source_version_length_contract() {
    let text = "ä".repeat(300);
    let source = format!("CONST STRING Text=\"{text}\"\nCONST STRING Alias=Text\nPRINT LEN(Text),\"|\",LEN(Alias)\n");
    succeeds(&[("main.pps", &source)], 400, "300|300");
    succeeds(&[("main.pps", &source)], 350, "256|256");
}

#[test]
fn double_constants_keep_precision_through_aliases_and_serialization() {
    succeeds(
        &[("main.pps", "CONST DOUBLE N=\"1.23456789012345\"\nCONST DOUBLE Alias=N\nPRINT N,\"|\",Alias\n")],
        400,
        "1.23456789012345|1.23456789012345",
    );
    for number in [f64::MAX, -f64::MAX, f64::MIN_POSITIVE] {
        let source = format!("CONST DOUBLE N=\"{number:e}\"\nCONST DOUBLE Alias=N\nPRINT N,\"|\",Alias\n");
        succeeds(&[("main.pps", &source)], 400, &format!("{number}|{number}"));
    }
}
