//! Follow-up language regressions, including shared CONST integration checks.
//! Tests exercise untransformed SemanticVisitor and serialized executable paths.
use icy_board_engine::{
    ast::{Ast, output_visitor::OutputVisitor},
    compiler::{PPECompiler, lower_modules, workspace::Workspace},
    decompiler::decompile,
    executable::Executable,
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast_with_predeclared_types, preparse_type_declarations},
    semantic::SemanticVisitor,
    vm::{self, io::DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

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
    (workspace, registry, errors, asts)
}

fn messages(errors: &Arc<Mutex<ErrorReporter>>) -> Vec<String> {
    errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect()
}

fn compile(sources: &[(&str, &str)], language: u16) -> Result<Executable, Vec<String>> {
    let (workspace, registry, errors, asts) = parse(sources, language);
    if !messages(&errors).is_empty() {
        return Err(messages(&errors));
    }
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&asts.iter().collect::<Vec<_>>());
    if !messages(&errors).is_empty() {
        return Err(messages(&errors));
    }
    compiler.create_executable().map_err(|error| vec![error.to_string()])
}

fn semantic(sources: &[(&str, &str)], language: u16) -> Vec<String> {
    let (workspace, registry, errors, asts) = parse(sources, language);
    if !messages(&errors).is_empty() {
        return messages(&errors);
    }
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

fn run(executable: &Executable) -> Result<String, String> {
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
            let result = vm::run(&PathBuf::from("followup.ppe"), &executable, &mut io, &mut state).await;
            drop(state);
            result.map_err(|error| error.to_string())
        };
        let (output, result) = tokio::join!(reader, execute);
        result.map(|_| output.clone()).map_err(|error| format!("{error}; output={output}"))
    })
}

fn succeeds(source: &str, expected: &str) {
    let sources = [("main.pps", source)];
    assert!(semantic(&sources, 400).is_empty(), "{:?}; {source}", semantic(&sources, 400));
    let executable = compile(&sources, 400).unwrap_or_else(|errors| panic!("{errors:?}; {source}"));
    assert_eq!(expected, run(&executable).unwrap(), "{source}");
}

fn roundtrips(source: &str, expected: &str) {
    succeeds(source, expected);
    let executable = compile(&[("main.pps", source)], 400).unwrap();
    let executable = Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap();
    for raw in [false, true] {
        let (ast, issues) = decompile(executable.clone(), raw, 400).unwrap();
        assert!(issues.is_empty(), "decompiler reported {} issues", issues.len());
        let mut output = OutputVisitor::default();
        output.version = 400;
        ast.visit(&mut output);
        succeeds(&output.output, expected);
    }
}

fn rejects(source: &str) {
    let sources = [("main.pps", source)];
    assert!(!semantic(&sources, 400).is_empty(), "semantic accepted {source}");
    assert!(compile(&sources, 400).is_err(), "compiler accepted {source}");
}

const BITS: &str = "ENUM Bits\n One = 1\n Two = 2\nENDENUM\n";

#[test]
fn foreach_requires_array_and_assignable_nominal_scalar() {
    for source in [
        format!("{BITS}INTEGER values[] = {{99}}\nBits item\nFOREACH item IN values\nPRINT TOINTEGER(item)\nNEXT\n"),
        "INTEGER values[1]\nINTEGER item[1]\nFOREACH item IN values\nNEXT\n".to_string(),
        "INTEGER value\nINTEGER item\nFOREACH item IN value\nNEXT\n".to_string(),
        "INTEGER values[1]\nCONST INTEGER item=1\nFOREACH item IN values\nNEXT\n".to_string(),
        "TYPE A\n INTEGER x\nENDTYPE\nTYPE B\n INTEGER x\nENDTYPE\nA values[1]\nB item\nFOREACH item IN values\nNEXT\n".to_string(),
    ] {
        rejects(&source);
    }
    succeeds(
        &format!("{BITS}Bits values[] = {{Bits.One, Bits.Two}}\nBits item\nFOREACH item IN values\nPRINT TOINTEGER(item)\nNEXT\n"),
        "12",
    );
    succeeds(
        "INTEGER values[1,1]\nvalues[1,1]=7\nINTEGER item\nFOREACH item IN values\nPRINT item\nNEXT\n",
        "0007",
    );
}

#[test]
fn indexed_record_assignments_are_nominal() {
    for target in ["box.entry", "boxes[0].entry"] {
        rejects(&format!(
            "TYPE First\n INTEGER value\nENDTYPE\nTYPE Second\n STRING text\nENDTYPE\nTYPE Boxed\n First entry\nENDTYPE\nBoxed box\nBoxed boxes[0]\n{target} = Second {{text=\"wrong\"}}\n"
        ));
    }
}

#[test]
fn fixed_record_fields_check_actual_resized_shape() {
    for assignment in ["box.items = a", "box = Boxed {items=a}", "boxes[0].items=a", "boxes[0]=Boxed {items=a}"] {
        let source = format!("TYPE Boxed\n INTEGER items[1]\nENDTYPE\nBoxed box\nBoxed boxes[0]\nINTEGER a[1]\na.Redim(4)\na[4]=9\n{assignment}\n");
        let executable = compile(&[("main.pps", &source)], 400).unwrap();
        assert!(run(&executable).unwrap_err().contains("fixed shape"), "{assignment}");
    }
    succeeds(
        "TYPE Boxed\n INTEGER items[1]\nENDTYPE\nBoxed box\nINTEGER a[1]\na[1]=9\nbox=Boxed {items=a}\nPRINT box.items[1]\n",
        "9",
    );
}

#[test]
fn constants_use_converted_values_and_reject_ranges() {
    for (kind, value) in [
        ("BYTE", "257"),
        ("BYTE", "-1"),
        ("SBYTE", "128"),
        ("SBYTE", "-129"),
        ("WORD", "65536"),
        ("WORD", "-1"),
        ("SWORD", "32768"),
        ("SWORD", "-32769"),
        ("INTEGER", "2147483648"),
        ("INTEGER", "-2147483649"),
        ("UNSIGNED", "4294967296"),
        ("UNSIGNED", "-1"),
        ("LONG", "9223372036854775808"),
        ("ULONG", "-1"),
        ("BYTE", "-0.5"),
    ] {
        for suffix in ["", "CONST INTEGER Alias=N\nPRINT Alias\n"] {
            rejects(&format!("CONST {kind} N={value}\n{suffix}"));
        }
    }
    succeeds(
        &format!("{BITS}CONST INTEGER N=1.5\nCONST INTEGER M=N\nCONST Bits Value=Bits(N)\nPRINT N,\"|\",M,\"|\",Value\n"),
        "1|1|1",
    );
    succeeds("CONST BYTE N=255\nCONST INTEGER M=N\nPRINT N,\"|\",M\n", "255|255");
    succeeds(
        "CONST LONG N=9223372036854775807\nCONST LONG M=N\nCONST ULONG U=18446744073709551615\nPRINT N,\"|\",M,\"|\",U\n",
        "9223372036854775807|9223372036854775807|18446744073709551615",
    );
}

#[test]
fn ordinary_variables_and_pre400_constants_keep_wrapping() {
    succeeds("BYTE N=257\nPRINT N\n", "1");
    let sources = [("main.pps", "CONST BYTE N=257\nCONST INTEGER M=N\nPRINT N,\"|\",M\n")];
    assert!(semantic(&sources, 350).is_empty());
    assert_eq!("1|1", run(&compile(&sources, 350).unwrap()).unwrap());
}

#[test]
fn record_return_is_per_invocation() {
    succeeds(
        "TYPE Item\n INTEGER value\nENDTYPE\nPRINT Build(2).value\nFUNCTION Build(INTEGER depth) Item\nBuild=Item {value=depth}\nIF depth>0 THEN\nItem child=Build(depth-1)\nENDIF\nENDFUNC\n",
        "2",
    );
}

#[test]
fn computed_matrix_and_cube_returns() {
    for (rank, bounds, indices) in [("[,]", "[1,1]", "[1,1]"), ("[,,]", "[1,1,1]", "[1,1,1]")] {
        succeeds(
            &format!("PRINT Make(){indices}\nFUNCTION Make() INTEGER{rank}\nINTEGER result{bounds}\nresult{indices}=7\nRETURN result\nENDFUNC\n"),
            "7",
        );
        rejects(&format!(
            "PRINT Make()[1]\nFUNCTION Make() INTEGER{rank}\nINTEGER result{bounds}\nRETURN result\nENDFUNC\n"
        ));
    }
}

#[test]
fn computed_array_results_evaluate_receiver_and_indices_once_in_order() {
    for (rank, bounds, indices, trace) in [
        ("[]", "[1]", "[Index(1)]", "M1"),
        ("[,]", "[1,1]", "[Index(1),Index(2)]", "M12"),
        ("[,,]", "[1,1,1]", "[Index(1),Index(2),Index(3)]", "M123"),
    ] {
        for (main, callback) in [
            (format!("PRINT Make(){indices}"), String::new()),
            (
                "Apply(Make)".to_string(),
                format!("PROCEDURE Apply(FUNCTION callback() INTEGER{rank})\nPRINT callback(){indices}\nENDPROC\n"),
            ),
        ] {
            roundtrips(
                &format!(
                    "{main}\nFUNCTION Make() INTEGER{rank}\nPRINT \"M\"\nINTEGER result{bounds}\nresult{bounds}=7\nRETURN result\nENDFUNC\nFUNCTION Index(INTEGER digit) INTEGER\nPRINT digit\nRETURN 1\nENDFUNC\n{callback}"
                ),
                &format!("{trace}7"),
            );
        }
    }
}

#[test]
fn callback_record_return_is_per_invocation_but_scalar_return_is_legacy() {
    roundtrips(
        "TYPE Item\n INTEGER value\nENDTYPE\nPRINT Build(2).value\nFUNCTION Build(INTEGER depth) Item\nBuild=Item {value=depth}\nIF depth>0 THEN\nItem child=Invoke(Build,depth-1)\nENDIF\nENDFUNC\nFUNCTION Invoke(FUNCTION callback(INTEGER depth) Item, INTEGER depth) Item\nRETURN callback(depth)\nENDFUNC\n",
        "2",
    );
    for source in [
        "PRINT Build(2)\nFUNCTION Build(INTEGER depth) INTEGER\nBuild=depth\nIF depth>0 THEN\nINTEGER child=Build(depth-1)\nENDIF\nENDFUNC\n",
        "PRINT Build(2)\nFUNCTION Build(INTEGER depth) INTEGER\nBuild=depth\nIF depth>0 THEN\nINTEGER child=Invoke(Build,depth-1)\nENDIF\nENDFUNC\nFUNCTION Invoke(FUNCTION callback(INTEGER depth) INTEGER, INTEGER depth) INTEGER\nRETURN callback(depth)\nENDFUNC\n",
    ] {
        succeeds(source, "0");
    }
}

#[test]
fn fixed_record_shapes_cover_every_axis_and_nested_paths() {
    for (bounds, resized) in [
        ("1", "2"),
        ("1,2", "2,2"),
        ("1,2", "1,3"),
        ("1,2,3", "2,2,3"),
        ("1,2,3", "1,3,3"),
        ("1,2,3", "1,2,4"),
    ] {
        for assignment in [
            "box.child.items=a",
            "box=Outer {child=Inner {items=a}}",
            "boxes[0].child.items=a",
            "boxes[0].child=Inner {items=a}",
            "box.children[0].items=a",
        ] {
            let source = format!(
                "TYPE Inner\n INTEGER items[{bounds}]\nENDTYPE\nTYPE Outer\n Inner child\n Inner children[0]\nENDTYPE\nOuter box\nOuter boxes[0]\nINTEGER a[{bounds}]\na.Redim({resized})\n{assignment}\n"
            );
            assert!(semantic(&[("main.pps", &source)], 400).is_empty(), "{source}");
            let error = run(&compile(&[("main.pps", &source)], 400).unwrap()).unwrap_err();
            assert!(error.contains("fixed shape"), "{error}; {source}");
        }
        succeeds(
            &format!(
                "TYPE Inner\n INTEGER items[{bounds}]\nENDTYPE\nTYPE Outer\n Inner child\nENDTYPE\nINTEGER a[{bounds}]\na[{bounds}]=9\nOuter box=Outer {{child=Inner {{items=a}}}}\nPRINT box.child.items[{bounds}]\n"
            ),
            "9",
        );
    }
}

#[test]
fn indexed_nested_record_and_record_array_targets_reject_nominal_mismatch() {
    for target in ["boxes[0].entry", "boxes[0].entries[0]", "box.entries[0]", "entries[0]"] {
        rejects(&format!(
            "TYPE First\n INTEGER value\nENDTYPE\nTYPE Second\n INTEGER value\nENDTYPE\nTYPE Boxed\n First entry\n First entries[0]\nENDTYPE\nBoxed box\nBoxed boxes[0]\nFirst entries[0]\n{target}=Second {{value=1}}\n"
        ));
    }
}

#[test]
fn foreach_converts_scalars_and_preserves_record_snapshot() {
    succeeds("INTEGER values[]={257,258}\nBYTE item\nFOREACH item IN values\nPRINT item\nNEXT\n", "12");
    succeeds(
        "TYPE Item\n INTEGER value\nENDTYPE\nItem values[]={Item {value=1},Item {value=2}}\nItem item\nFOREACH item IN values\nPRINT item.value\nvalues[1].value=9\nitem.value=7\nNEXT\nPRINT values[0].value,values[1].value\n",
        "1219",
    );
    succeeds(
        "INTEGER values[1,1,1]\nvalues[1,1,1]=7\nINTEGER item\nFOREACH item IN values\nPRINT item\nNEXT\n",
        "00000007",
    );
}

#[test]
fn computed_nominal_array_results_keep_element_types() {
    for (rank, bounds) in [("[,]", "[1,1]"), ("[,,]", "[1,1,1]")] {
        roundtrips(
            &format!(
                "TYPE Item\n INTEGER value\nENDTYPE\nPRINT Make(){bounds}.value\nFUNCTION Make() Item{rank}\nItem result{bounds}\nresult{bounds}=Item {{value=7}}\nRETURN result\nENDFUNC\n"
            ),
            "7",
        );
        roundtrips(
            &format!(
                "{BITS}PRINT TOINTEGER(Make(){bounds})\nFUNCTION Make() Bits{rank}\nBits result{bounds}\nresult{bounds}=Bits.Two\nRETURN result\nENDFUNC\n"
            ),
            "2",
        );
    }
}

#[test]
fn enum_namespace_and_typed_constant_rgb_parity() {
    succeeds(
        &format!("{BITS}CONST INTEGER Bits=7\nPRINT Bits.One,\"|\",Bits.One.Has(Bits.One),\"|\",Bits(1)\n"),
        "1|1|1",
    );
    rejects(&format!("{BITS}CONST UNSIGNED Packed=RGB(Bits.One,0,0)\nPRINT Packed\n"));
    rejects(&format!("{BITS}CONST SWORD N=1\nPRINT Bits(N)\n"));
    rejects(&format!("{BITS}CONST SWORD N=1\nCONST Bits Value=Bits(N)\n"));
    succeeds(&format!("{BITS}CONST UNSIGNED Packed=RGB(TOINTEGER(Bits.One),0,0)\nPRINT Packed\n"), "16777471");
}
