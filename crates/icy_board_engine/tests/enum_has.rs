//! Enum containment is Boolean, never a checked enum intersection.
use icy_board_engine::{
    ast::output_visitor::OutputVisitor,
    compiler::{PPECompiler, workspace::Workspace},
    decompiler::decompile,
    executable::{Executable, FuncOpCode},
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
    vm::{self, io::DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

const SPARSE: &str = "ENUM Bits\n One = 1\n Two = 2\nENDENUM\n";
const FULL: &str = "ENUM Bits\n Both = 3\n One = 1\n Two = 2\n Zero = 0\n All = -1\nENDENUM\n";

fn compile(source: &str, language: u16, runtime: u16, semantic_only: bool) -> Result<Executable, Vec<String>> {
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(runtime);
    workspace.set_default_language_version(Some(language));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("has.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    if semantic_only {
        let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
        ast.visit(&mut visitor);
        visitor.finish();
        let messages: Vec<_> = errors.lock().unwrap().errors.iter().map(|e| e.error.to_string()).collect();
        return if messages.is_empty() { Ok(Executable::default()) } else { Err(messages) };
    }
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let messages: Vec<_> = errors.lock().unwrap().errors.iter().map(|e| e.error.to_string()).collect();
    if !messages.is_empty() {
        return Err(messages);
    }
    let executable = compiler.create_executable().unwrap();
    Ok(Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap())
}

fn run(executable: &Executable) -> Result<String, String> {
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
            let result = vm::run(&PathBuf::from("has.ppe"), executable, &mut io, &mut state).await;
            drop(state);
            result.map_err(|error| error.to_string())
        };
        let (output, result) = tokio::join!(reader, execute);
        result.map(|_| output.clone()).map_err(|error| format!("{error}; output={output}"))
    })
}

fn verify(source: &str, language: u16, expected: &str) {
    let executable = compile(source, language, 400, false).unwrap_or_else(|errors| panic!("{source}\n{errors:?}"));
    assert_eq!(expected, run(&executable).unwrap(), "{source}");
    for raw in [false, true] {
        let (ast, issues) = decompile(executable.clone(), raw, language).unwrap();
        assert!(issues.is_empty());
        let mut output = OutputVisitor::default();
        output.version = language;
        ast.visit(&mut output);
        let rebuilt = compile(&output.output, language, 400, false).unwrap_or_else(|errors| panic!("{}\n{errors:?}", output.output));
        assert_eq!(expected, run(&rebuilt).unwrap(), "{}", output.output);
    }
}

#[test]
fn sparse_domains_need_no_intermediate_enum_value() {
    assert_eq!(-356, FuncOpCode::EnumHas as i16);
    assert_eq!(400, FuncOpCode::EnumHas.minimum_runtime());
    for language in [350, 400] {
        verify(&format!("{SPARSE}PRINT Bits.One.Has(Bits.Two), Bits.One.hAs(Bits.One)\n"), language, "01");
        verify(
            &format!("{SPARSE}Bits items(1)\nitems(0) = Bits.Two\nPRINT items(0).Has(Bits.One), items(0).Has(Bits.Two), items(0)\n"),
            language,
            "012",
        );
        let executable = compile(&format!("{SPARSE}PRINT Bits.One.Has(Bits.Two)\n"), language, 400, false).unwrap();
        assert!(executable.variable_table.enums.values().any(|domain| domain == &[1, 2]));
        let errors = compile(&format!("{SPARSE}PRINT Bits.One.Has(Bits.One)\n"), language, 340, false).err().unwrap();
        assert!(errors.iter().any(|e| e.contains("runtime 400")), "{errors:?}");
    }
}

#[test]
fn every_builtin_enum_and_every_regex_mask_pair() {
    for language in [350, 400] {
        let registry = UserTypeRegistry::icy_board_registry();
        let mut source = String::new();
        let mut expected = String::new();
        for definition in registry.enums() {
            if let Some((member, _)) = definition.variants.first() {
                source.push_str(&format!("PRINT {}.{}.Has({}.{})\n", definition.name, member, definition.name, member));
                expected.push('1');
            }
        }
        verify(&source, language, &expected);
        let source =
            "INTEGER receiver, mask\nFOR receiver = 0 TO 63\n FOR mask = 0 TO 63\n  PRINT RegexOptions(receiver).Has(RegexOptions(mask))\n NEXT\nNEXT\n";
        let expected: String = (0..64)
            .flat_map(|receiver| (0..64).map(move |mask| if receiver & mask == mask { '1' } else { '0' }))
            .collect();
        verify(source, language, &expected);
    }
}

#[test]
fn masks_constants_and_chained_boolean_expressions() {
    for language in [350, 400] {
        verify(
            &format!(
                r#"{FULL}
CONST Bits Mask = Bits.Both
CONST BOOLEAN Found = Bits.Both.Has(Mask)
CONST BOOLEAN Absent = Bits.One.Has(Bits.Two)
Bits value = Bits.One
PRINT Found, Absent, "|", Mask.Has(Bits.One), Mask.Has(Bits.Zero), Bits.Zero.Has(Bits.Zero), "|"
PRINT value.Has(Mask), Bits.All.Has(Mask), (Bits.One | Bits.Two).Has(Mask), "|"
PRINT !value.Has(Bits.Two), value.Has(Bits.One) = TRUE, "|", value, Mask
"#
            ),
            language,
            "10|111|011|11|13",
        );
    }
}

#[test]
fn receiver_then_mask_once_and_no_mutation() {
    for language in [350, 400] {
        verify(
            &format!(
                r#"{FULL}
DECLARE FUNCTION Receiver() Bits
DECLARE FUNCTION MaskValue() Bits
Bits value = Bits.Both
INTEGER receivers, masks
PRINT Receiver().Has(MaskValue()), "|", receivers, masks, "|", value, "|"
value = Bits.Both
PRINT value.Has(MaskValue()), "|", value
FUNCTION Receiver() Bits
 receivers = receivers + 1
 PRINT "R"
 Receiver = value
ENDFUNC
FUNCTION MaskValue() Bits
 masks = masks + 1
 PRINT "M"
 value = Bits.Zero
 MaskValue = Bits.One
ENDFUNC
"#
            ),
            language,
            "RM1|11|0|M1|0",
        );
    }
}

#[test]
fn indexed_arrays_and_record_fields() {
    // User records are a source-400 feature; enum arrays also work in 350.
    for language in [400] {
        verify(
            &format!(
                r#"{FULL}
TYPE Boxed
 Bits Value
 Bits Items(1)
ENDTYPE
Bits items(1)
Boxed box
items(0) = Bits.One
box.Value = Bits.Two
box.Items(0) = Bits.Both
PRINT items(0).Has(Bits.Two), box.Value.Has(Bits.Two), box.Items(0).Has(Bits.Both), "|"
PRINT items(0), box.Value, box.Items(0)
"#
            ),
            language,
            "011|123",
        );
    }
}

#[test]
fn invalid_arguments_and_receivers_are_rejected_in_compiler_and_source_semantics() {
    for language in [350, 400] {
        for semantic_only in [false, true] {
            for body in [
                "PRINT Bits.One.Has()",
                "PRINT Bits.One.Has(Bits.One, Bits.Two)",
                "PRINT Bits.One.Has(1)",
                "PRINT Bits.One.Has(TRUE)",
                "PRINT Bits.One.Has(\"1\")",
                "PRINT Bits.One.Has(RegexOptions.IgnoreCase)",
                "PRINT Bits.One.Has(Bits)",
                "PRINT Bits.One.Has",
                "PRINT Bits.Has(Bits.One)",
                "INTEGER n\nPRINT n.Has(Bits.One)",
                "Bits value\nvalue.Has = Bits.One",
                "Bits value\nvalue.Has |= Bits.One",
                "Bits items(1)\nPRINT items.Has(Bits.One)",
                "Bits items(1)\nPRINT Bits.One.Has(items)",
                "TYPE Boxed\n Bits Items(1)\nENDTYPE\nBoxed box\nPRINT box.Items.Has(Bits.One)",
                "ENUM Other\n One = 1\nENDENUM\nPRINT Bits.One.Has(Other.One)",
            ] {
                let source = format!("{SPARSE}{body}\n");
                assert!(
                    compile(&source, language, 400, semantic_only).is_err(),
                    "accepted language={language} semantic={semantic_only}: {source}"
                );
            }
        }
    }
}

#[test]
fn unnamed_constant_and_dynamic_masks_use_all_bits() {
    for language in [350, 400] {
        for (expression, expected) in [
            ("Bits(n).Has(Bits.One)", "0"),
            ("Bits.One.Has(Bits(n))", "1"),
            ("(a & b).Has(a)", "0"),
            ("a.Has(a & b)", "1"),
            ("Bits(0).Has(Bits.One)", "0"),
            ("Bits.One.Has(Bits(0))", "1"),
            ("(Bits.One & Bits.Two).Has(Bits.One)", "0"),
            ("Bits.One.Has(Bits.One & Bits.Two)", "1"),
            ("Bits(64).Has(Bits(64))", "1"),
            ("Bits(-1).Has(Bits(64))", "1"),
            ("Bits.One.Has(Bits(64))", "0"),
        ] {
            let executable = compile(
                &format!("{SPARSE}INTEGER n = 0\nBits a = Bits.One, b = Bits.Two\nPRINT {expression}\n"),
                language,
                400,
                false,
            )
            .unwrap();
            assert_eq!(expected, run(&executable).unwrap(), "{expression}");
            let source =
                format!("{SPARSE}CONST INTEGER n = 0\nCONST Bits a = Bits.One\nCONST Bits b = Bits.Two\nCONST BOOLEAN Result = {expression}\nPRINT Result\n");
            compile(&source, language, 400, true).unwrap();
            let executable = compile(&source, language, 400, false).unwrap();
            assert_eq!(expected, run(&executable).unwrap(), "{source}");
        }
    }
}
