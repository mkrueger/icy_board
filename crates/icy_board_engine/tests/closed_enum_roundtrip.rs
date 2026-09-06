//! Closed enum metadata must survive PPE -> source -> PPE, not just AST printing.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{Ast, output_visitor::OutputVisitor},
    compiler::{PPECompiler, workspace::Workspace},
    decompiler::decompile,
    executable::{Executable, FuncOpCode, PPECommand, PPEExpr, PPEScript},
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    vm::{self, io::DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

const DOMAIN: &str = "ENUM Shade\n First = 7\n Second = -3\n Alias = 7\nENDENUM\n";

#[test]
fn enum_bitwise_roundtrips_nested_operations_and_unnamed_regex_values() {
    for language in [350, 400] {
        let source = r#"
ENUM Bits
 Both = 3
 One = 1
 Two = 2
 Zero = 0
ENDENUM
Bits a = Bits.One, b = Bits.Two, defaultValue
CONST RegexOptions Options = RegexOptions.IgnoreCase | RegexOptions.MultiLine
RegexOptions r = Options, empty
PRINT a | b, ",", (a | b) & b, ",", a & b, ",", defaultValue, "|"
PRINT r, ",", r & RegexOptions.MultiLine, ",", empty, ",", RegexOptions(63), "|"
IF ((a | b) = Bits.Both) PRINT "yes"
a |= b
PRINT "|", a
"#;
        for text in roundtrip(compile(source, language), language, "3,2,0,3|3,2,0,63|yes|3") {
            assert!(text.contains(" | ") && text.contains(" & "), "{text}");
            assert!(!text.contains("IgnoreCaseAnd") && !text.contains("ENUM ENUM244"), "{text}");
            let registry = UserTypeRegistry::icy_board_registry();
            let options = registry.get_enum_from_id(icy_board_engine::parser::REGEX_OPTIONS_ENUM_ID).unwrap();
            assert_eq!(7, options.variants.len());
            let rebuilt = reload(&compile(&text, language));
            assert_eq!((0..64).collect::<Vec<_>>(), rebuilt.variable_table.enums[&options.id]);
        }
    }
}

#[test]
fn enum_bitwise_roundtrip_preserves_invalid_intermediate_checks() {
    for language in [350, 400] {
        let source = "ENUM Bits\n One = 1\n Two = 2\nENDENUM\nBits a = Bits.One, b = Bits.Two\nPRINT TOINTEGER((a | b) & a)\n";
        let executable = reload(&compile(source, language));
        assert!(run(&executable).unwrap_err().contains("not a member of closed enum"));
        for raw in [false, true] {
            let (ast, issues) = decompile(executable.clone(), raw, language).unwrap();
            assert!(issues.is_empty());
            let text = source_text(&ast, language);
            assert!(text.contains(" | ") && text.contains(" & "), "{text}");
            let error = run(&reload(&compile(&text, language))).unwrap_err();
            assert!(error.contains("not a member of closed enum") && error.ends_with("output="), "{text}: {error}");
        }
    }
}

#[test]
fn enum_bitwise_roundtrip_record_literals_and_array_elements() {
    let source = r#"
TYPE Boxed
 RegexOptions Option
ENDTYPE
RegexOptions items(1)
items(0) = RegexOptions.IgnoreCase | RegexOptions.MultiLine
Boxed box = Boxed { Option = items(0) & RegexOptions.MultiLine }
PRINT box.Option, "|", items(0)
"#;
    roundtrip(compile(source, 400), 400, "2|3");
}

#[test]
fn integer_bit_helpers_inside_casts_do_not_gain_invalid_operand_casts() {
    for language in [350, 400] {
        roundtrip(
            compile(
                "ENUM Bits\n Both = 3\nENDENUM\nINTEGER a = 1, b = 2\nPRINT Bits(OR(a,b)), \"|\", Bits(OR(1,2))\n",
                language,
            ),
            language,
            "3|3",
        );
    }
}

fn compile(source: &str, language: u16) -> Executable {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(language));
    let ast = parse_ast(PathBuf::from("enum.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let errors = errors.lock().unwrap();
    assert!(
        errors.errors.is_empty(),
        "{source}\n{:?}",
        errors.errors.iter().map(|e| e.error.to_string()).collect::<Vec<_>>()
    );
    compiler.create_executable().unwrap()
}

fn reload(executable: &Executable) -> Executable {
    Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap()
}

fn source_text(ast: &Ast, language: u16) -> String {
    // Match ppld: Display uses the classic formatter, not ast.language_version.
    let mut visitor = OutputVisitor::default();
    visitor.version = language;
    ast.visit(&mut visitor);
    visitor.output
}

fn run(executable: &Executable) -> Result<String, String> {
    icy_board_engine::executable::PPEScript::from_ppe_file(executable).map_err(|error| format!("PPE decoding: {error}"))?;
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
            let result = vm::run(&PathBuf::from("enum.ppe"), executable, &mut io, &mut state).await;
            drop(state);
            result.map_err(|error| error.to_string())
        };
        let (output, result) = tokio::join!(reader, execute);
        result.map(|_| output.clone()).map_err(|error| format!("{error}; output={output}"))
    })
}

fn roundtrip(executable: Executable, language: u16, expected: &str) -> Vec<String> {
    let executable = reload(&executable);
    assert_eq!(expected, run(&executable).unwrap());
    [false, true]
        .into_iter()
        .map(|raw| {
            let (ast, issues) = decompile(executable.clone(), raw, language).unwrap();
            assert!(issues.is_empty());
            let source = source_text(&ast, language);
            assert!(!source.to_ascii_lowercase().contains("<enumcast>"), "{source}");
            let rebuilt = reload(&compile(&source, language));
            assert_eq!(expected, run(&rebuilt).unwrap(), "raw={raw}\n{source}");
            source
        })
        .collect()
}

#[test]
fn scalars_aliases_defaults_and_equality_roundtrip_in_both_languages() {
    for language in [350, 400] {
        let source = format!(
            "{DOMAIN}Shade value\nPRINT value, \"|\"\nvalue = Shade.Second\nPRINT value, \"|\", value = Shade.Second, \"|\", Shade.First <> value\nvalue = Shade.Alias\nPRINT \"|\", TOINTEGER(value)\n"
        );
        for text in roundtrip(compile(&source, language), language, "7|-3|1|1|7") {
            assert!(text.contains("ENUM ENUM241"), "{text}");
            assert!(
                text.contains("MEMBER001 = 7") && text.contains("MEMBER002 = -3") && text.contains("MEMBER003 = 7"),
                "{text}"
            );
            assert!(text.contains("ENUM241.MEMBER002"), "{text}");
        }
    }
}

#[test]
fn arrays_record_fields_and_record_literals_roundtrip() {
    let source = format!(
        r#"{DOMAIN}
TYPE Paint
 Shade Tone
 Shade Tones(1)
ENDTYPE
TYPE Boxed
 Paint Item
ENDTYPE
Shade values(1), matrix(1,1), cube(1,1,1)
Paint paint
Boxed boxed
PRINT values(1), ",", matrix(1,1), ",", cube(1,1,1), ",", paint.Tone, ",", paint.Tones(1), "|"
values(1) = Shade.Second
matrix(1,1) = Shade.Second
cube(1,1,1) = Shade.Second
paint = Paint {{ Tone = Shade(-3) }}
paint.Tone = Shade.Second
paint.Tones(1) = Shade.Second
boxed.Item.Tone = Shade.Second
PRINT values(1), ",", matrix(1,1), ",", cube(1,1,1), ",", paint.Tone, ",", paint.Tones(1), ",", boxed.Item.Tone
"#
    );
    for text in roundtrip(compile(&source, 400), 400, "7,7,7,7,7|-3,-3,-3,-3,-3,-3") {
        assert!(text.find("ENDENUM").unwrap() < text.find("TYPE TYPE001").unwrap(), "{text}");
    }
}

#[test]
fn parameters_local_defaults_and_function_results_roundtrip() {
    for language in [350, 400] {
        let source = format!(
            r#"{DOMAIN}
Shade value, items(1)
Show(Shade.Second)
Change(value)
Fill(items)
PRINT value, ",", items(1), "|", Echo(Shade.Second), ",", DefaultValue(), ",", Make()
PROCEDURE Show(Shade item)
 Shade local
 PRINT item, ",", local, "|"
ENDPROC
PROCEDURE Change(VAR Shade item)
 item = Shade.Second
ENDPROC
PROCEDURE Fill(VAR Shade items(1))
 items(1) = Shade.Second
ENDPROC
FUNCTION Echo(Shade item) Shade
 Echo = item
ENDFUNC
FUNCTION DefaultValue() Shade
ENDFUNC
FUNCTION Make() Shade
 Make = Shade.Second
ENDFUNC
"#
        );
        // Legacy array parameters pass an element (PPLC semantics), while 4.00
        // passes a whole array.
        let source = if language < 400 {
            source.replace("Fill(items)", "Fill(items(0))")
        } else {
            source
        };
        let expected = if language < 400 { "-3,7|-3,7|-3,7,-3" } else { "-3,7|-3,-3|-3,7,-3" };
        roundtrip(compile(&source, language), language, expected);
    }
}

#[test]
fn checked_casts_and_reverse_conversions_roundtrip() {
    for language in [350, 400] {
        let source = format!(
            "{DOMAIN}INTEGER number = -4\nShade value = Shade(number + 1)\nPRINT TOINTEGER(value), \"|\", Shade(number + 1) = Shade.Second, \"|\", Shade.First = Shade(7)\n"
        );
        for text in roundtrip(compile(&source, language), language, "-3|1|1") {
            assert!(text.contains("ENUM241("), "{text}");
            assert!(!text.contains("EnumCast"), "{text}");
        }
    }
}

#[test]
fn invalid_dynamic_cast_still_fails_after_recompilation() {
    let executable = reload(&compile(
        &format!("{DOMAIN}INTEGER number = 8\nShade value = Shade(number)\nPRINT value\n"),
        400,
    ));
    let original = run(&executable).unwrap_err();
    assert!(original.contains("not a member of closed enum"), "{original}");
    for raw in [false, true] {
        let (ast, _) = decompile(executable.clone(), raw, 400).unwrap();
        let rebuilt = reload(&compile(&source_text(&ast, 400), 400));
        let error = run(&rebuilt).unwrap_err();
        assert!(error.contains("not a member of closed enum"), "{error}");
    }
}

#[test]
fn builtin_enums_keep_names_in_storage_calls_and_comparisons() {
    let source = r#"
RegexOptions options = RegexOptions.IgnoreCase | RegexOptions.MultiLine
StringComparison comparison = StringComparison.OrdinalIgnoreCase
MouseButton button = MouseButton.None
PRINT TOINTEGER(options), "|", Regex.Compile("^a", options).IsMatch("A"), "|"
PRINT "Hello".Equals("hello", comparison), "|", "Hello".Equals("hello", StringComparison.OrdinalIgnoreCase), "|"
PRINT button = MouseButton.None, "|", TOINTEGER(MouseButton(-1))
"#;
    for text in roundtrip(compile(source, 400), 400, "3|1|1|1|1|-1") {
        assert!(!text.contains("ENDENUM"), "{text}");
        assert!(text.contains("RegexOptions.IgnoreCase | RegexOptions.MultiLine"), "{text}");
        assert!(text.contains("StringComparison.OrdinalIgnoreCase"), "{text}");
        assert!(text.contains("MouseButton.None"), "{text}");
    }
}

#[test]
fn sparse_enum_ids_are_remapped_in_fields_signatures_and_casts() {
    let mut executable = compile(
        r#"
ENUM Unused
 Zero = 0
ENDENUM
ENUM Shade
 First = 7
 Second = -3
ENDENUM
TYPE Paint
 Shade Tone
ENDTYPE
Paint paint
INTEGER number = -3
paint.Tone = Echo(Shade(number))
PRINT paint.Tone, "|", Echo(Shade.First)
FUNCTION Echo(Shade value) Shade
 Echo = value
ENDFUNC
"#,
        400,
    );
    // A PPE need not carry unused enum domains. Its remaining id 240 must not
    // be confused with the source registry's first free id, 241.
    executable.variable_table.enums.remove(&241);
    for text in roundtrip(executable, 400, "-3|7") {
        assert!(text.contains("ENUM ENUM240") && text.contains("ENUM240("), "{text}");
    }
}

#[test]
fn builtin_id_with_a_different_default_gets_a_synthetic_declaration() {
    let mut executable = compile("MouseTracking tracking\nPRINT tracking\n", 400);
    executable
        .variable_table
        .enums
        .insert(icy_board_engine::parser::MOUSE_TRACKING_ENUM_ID, vec![2, 0, 1]);
    for text in roundtrip(executable, 400, "2") {
        assert!(text.contains("ENUM ENUM251"), "{text}");
        assert!(text.contains("MEMBER001 = 2"), "{text}");
        assert!(!text.contains("MouseTracking"), "{text}");
    }
}

#[test]
fn integer_expressions_in_typed_bytecode_contexts_gain_explicit_casts() {
    fn strip_casts(expression: &mut PPEExpr, zero: usize) {
        match expression {
            PPEExpr::PredefinedFunctionCall(definition, arguments) if definition.opcode == FuncOpCode::EnumCast => {
                *expression = PPEExpr::BinaryExpression(
                    icy_board_engine::ast::BinOp::Add,
                    Box::new(arguments[1].clone()),
                    Box::new(PPEExpr::Value(zero)),
                );
            }
            PPEExpr::PredefinedFunctionCall(_, arguments) | PPEExpr::FunctionCall(_, arguments) => {
                for argument in arguments {
                    strip_casts(argument, zero);
                }
            }
            _ => {}
        }
    }
    let mut executable = compile(
        &format!(
            r#"{DOMAIN}
Shade value
INTEGER number = -4
INTEGER zero = 0
value = Shade(number + zero + 1)
PRINT value, "|"
Show(Shade(number + 1))
value = Echo(Shade(number + 1))
PRINT value, "|", "Hello".Equals("hello", StringComparison(number + 5))
PROCEDURE Show(Shade item)
 PRINT item, "|"
ENDPROC
FUNCTION Echo(Shade item) Shade
 Echo = item
ENDFUNC
"#
        ),
        400,
    );
    // Runtime assignment/parameter guards accept valid raw integers. Source
    // nominal checks do not: recovering bytecode like this needs explicit casts.
    // Replacing EnumCast(id, expr) with expr + zero keeps every code offset intact.
    let zero = executable
        .variable_table
        .get_entries()
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case("zero"))
        .unwrap()
        .header
        .id;
    let mut script = PPEScript::from_ppe_file(&executable).unwrap();
    for statement in &mut script.statements {
        match &mut statement.command {
            PPECommand::Let(_, expression) => strip_casts(expression, zero),
            PPECommand::ProcedureCall(_, arguments) | PPECommand::PredefinedCall(_, arguments) => {
                for argument in arguments {
                    strip_casts(argument, zero);
                }
            }
            _ => {}
        }
    }
    let rewritten = script.serialize();
    assert_eq!(executable.script_buffer.len(), rewritten.len());
    executable.script_buffer = rewritten;
    for text in roundtrip(executable, 400, "-3|-3|-3|1") {
        assert!(text.matches("ENUM241(").count() >= 3, "{text}");
        assert!(text.to_ascii_uppercase().contains("STRINGCOMPARISON("), "{text}");
    }
}

#[test]
fn distinct_enums_with_identical_domains_remain_nominally_distinct() {
    let source = r#"
ENUM First
 Initial = 7
 Other = -3
ENDENUM
ENUM Second
 Initial = 7
 Other = -3
ENDENUM
First one = First.Other
Second two = Second.Other
PRINT TOINTEGER(one), ",", TOINTEGER(two)
"#;
    for text in roundtrip(compile(source, 400), 400, "-3,-3") {
        assert!(text.contains("ENUM ENUM241") && text.contains("ENUM ENUM240"), "{text}");
        assert!(text.contains("ENUM241.MEMBER002") && text.contains("ENUM240.MEMBER002"), "{text}");
    }
}

#[test]
fn dynamic_enum_arrays_keep_their_defaults_and_indexed_assignments() {
    let source = format!(
        r#"{DOMAIN}
Shade values[]
values.Redim(2)
PRINT values[2], "|"
values[1] = Shade.Second
PRINT values[1], "|", values[1] = Shade.Second
"#
    );
    roundtrip(compile(&source, 400), 400, "7|-3|1");
}
