use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::output_visitor::OutputVisitor,
    compiler::{PPECompiler, workspace::Workspace},
    decompiler::decompile,
    executable::Executable,
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    vm::{self, DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

const TEXT: &str = "\u{20ac}\u{754c}e\u{301}\u{1f600}";

fn compile(source: &str, language: u16, runtime: u16) -> Executable {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(runtime);
    workspace.set_default_language_version(Some(language));
    let ast = parse_ast(PathBuf::from("text.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let errors = errors.lock().unwrap();
    assert!(
        errors.errors.is_empty(),
        "{source}\n{:?}",
        errors.errors.iter().map(|entry| entry.error.to_string()).collect::<Vec<_>>()
    );
    compiler.create_executable().unwrap()
}

fn reload(executable: &Executable) -> Executable {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("text.ppe");
    std::fs::write(&path, executable.to_buffer().unwrap()).unwrap();
    Executable::read_file(&path, false).unwrap()
}

fn run(executable: &Executable) -> String {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.root_path = directory.path().to_path_buf();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.term_caps.is_utf8 = true;
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
            let result = vm::run(&PathBuf::from("text.ppe"), executable, &mut io, &mut state).await;
            drop(state);
            result.unwrap();
        };
        tokio::time::timeout(std::time::Duration::from_secs(10), async { tokio::join!(reader, execute).0 })
            .await
            .unwrap()
    })
}

fn roundtrip(source: &str, language: u16, runtime: u16, expected: &str) {
    let executable = reload(&compile(source, language, runtime));
    assert_eq!(run(&executable), expected);
    for raw in [false, true] {
        let (ast, issues) = decompile(executable.clone(), raw, language).unwrap();
        assert!(issues.is_empty(), "{} decompiler issues", issues.len());
        let mut visitor = OutputVisitor::default();
        visitor.version = language;
        ast.visit(&mut visitor);
        let rebuilt = reload(&compile(&visitor.output, language, runtime));
        assert_eq!(run(&rebuilt), expected, "raw={raw}\n{}", visitor.output);
    }
}

#[test]
fn s5_literal_file_roundtrip_uses_runtime_not_language() {
    for language in [350, 400] {
        roundtrip(
            &format!("STRING text = \"{TEXT}\"\nPRINT text, \"|\", LEN(text)"),
            language,
            400,
            &format!("{TEXT}|5"),
        );
    }
    roundtrip(
        "STRING text\ntext = \"\u{e9}\u{2591}\u{2502}\"\nPRINT text, \"|\", LEN(text)",
        340,
        340,
        "\u{e9}\u{2591}\u{2502}|3",
    );
}

/// S7: the language contracts decided in S1-S6 have to hold together in one
/// program that is written, loaded, decompiled and rebuilt as a real file.
#[test]
fn s7_language_contracts_hold_together_in_one_file() {
    use icy_board_engine::executable::container::Compression;
    let source = format!(
        r#";$LANGVERSION 400
ENUM Shade
 First = 7
 Second = -3
ENDENUM
TYPE Item
 SURFACE image
 INTEGER values[]
 Shade tone
 STRING label
ENDTYPE
DECLARE PROCEDURE Fill(VAR INTEGER slot, VAR STRING text)
Item box
INTEGER numbers(3)
STRING word
BOOLEAN guard
ON ERROR GOTO Failed
REDIM box.values, 2
box.values[2] = 17
box.tone = Shade(TOINTEGER(Shade.Second))
box.label = "{TEXT}"
guard = FALSE && (1 / 0)
Fill(numbers(3), word)
PRINTLN box.values[2], "|", box.tone, "|", box.label.Len(), "|", numbers(3), "|", word, "|", guard
FOPEN 1, "missing.dat", O_RD, S_DN
PRINTLN "not reached"
EXIT
:Failed
PRINTLN "handler"
PROCEDURE Fill(VAR INTEGER slot, VAR STRING text)
 slot = 42
 text = "done"
ENDPROC
"#
    );
    let expected = "17|-3|5|42|done|0\nhandler\n";
    let executable = compile(&source, 400, 400);

    for compression in [Compression::None, Compression::Zstd] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("s7.ppe");
        std::fs::write(&path, executable.to_buffer_with_compression(compression).unwrap()).unwrap();
        let loaded = Executable::read_file(&path, false).unwrap();
        assert_eq!(run(&loaded), expected, "compression {compression:?}");

        // Declared names survive, and the rebuilt program behaves the same.
        let (ast, issues) = decompile(loaded, false, 400).unwrap();
        assert!(issues.is_empty(), "{} decompiler issues", issues.len());
        let mut visitor = OutputVisitor::default();
        visitor.version = 400;
        ast.visit(&mut visitor);
        assert!(
            visitor.output.contains("TYPE Item") && visitor.output.contains("Shade tone"),
            "{}",
            visitor.output
        );
        assert!(visitor.output.contains("ENUM Shade"), "{}", visitor.output);
        assert_eq!(run(&reload(&compile(&visitor.output, 400, 400))), expected, "{}", visitor.output);
    }
}

#[test]
fn c2_wide_code_parameters_and_composed_records_survive_files() {
    use icy_board_engine::executable::container::Compression;
    let parameters = (0..300).map(|index| format!("VAR INTEGER arg{index}")).collect::<Vec<_>>().join(", ");
    let arguments = (0..300).map(|index| format!("values[{index}]")).collect::<Vec<_>>().join(", ");
    let mut source = format!("DECLARE PROCEDURE Wide({parameters})\nINTEGER counter\nINTEGER values[299]\n");
    source.push_str("TYPE Item\nSURFACE image\nINTEGER values[]\nENDTYPE\nItem item\nREDIM item.values, 2\nitem.values[2] = 17\n");
    source.push_str(&"counter += 1\n".repeat(6000));
    source.push_str(&format!("Wide({arguments})\nPRINTLN counter, \"|\", values[299], \"|\", item.values[2], \"|\", FALSE && (1 / 0)\nPROCEDURE Wide({parameters})\narg299 = 42\nENDPROC\n"));
    source = source.replace(&format!("Wide({arguments})\nPRINTLN"), "Apply(Wide)\nPRINTLN");
    source.push_str(&format!("PROCEDURE Apply(PROCEDURE callback({parameters}))\ncallback({arguments})\nENDPROC\n"));
    let executable = compile(&source, 400, 400);
    for compression in [Compression::None, Compression::Zstd] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("wide.ppe");
        std::fs::write(&path, executable.to_buffer_with_compression(compression).unwrap()).unwrap();
        let loaded = Executable::read_file(&path, false).unwrap();
        assert_eq!("6000|42|17|0\n", run(&loaded));
    }
}

#[test]
fn s5_unicode_positions_bytes_and_transformations_survive_files() {
    let source = format!(
        r#"
STRING text = "{TEXT}"
BYTES raw = TOBYTES(text)
PRINTLN text.Len(), "|", LEN(raw), "|", raw.ToString() = text
PRINTLN text.Find("e"), "|", INSTR(text, "e"), "|", text.FindLast("{face}"), "|", INSTRR(text, "{face}")
PRINTLN text.Find("missing"), "|", INSTR(text, "missing"), "|", text.Find("e", 3)
PRINTLN TOBYTES(text[3]).ToHex(), "|", TOBYTES(text[4]).ToHex(), "|", text[-1].Len(), "|", text[5].Len()
PRINTLN text.Substring(2, 2) = "e{accent}", "|", MID(text, 3, 2) = "e{accent}", "|", "e{accent}".Len(), "|", "{composed}".Len(), "|", "e{accent}" = "{composed}"
PRINTLN "[", text.Substring(4, 3), "]|[", MID(text, 5, 3), "]|[", text.Substring(-1, 2), "]"
PRINTLN text.Remove(2, 2) = "{euro}{cjk}{face}", "|", text.Insert(3, "x") = "{euro}{cjk}ex{accent}{face}"
PRINTLN text.Reverse() = "{face}{accent}e{cjk}{euro}", "|", text.PadLeft(7).Len(), "|", "e{accent}".PadRight(3).Len()
PRINTLN text.Left(2) = "{euro}{cjk}", "|", text.Right(1) = "{face}"
REGEXMATCH found = REGEX.Compile("(e{accent})").Find(text)
PRINTLN found.Start, "|", found.Length, "|", found.GroupStart(1), "|", found.GroupLength(1)
STRING parts[] = text.Split("e{accent}")
PRINTLN parts.Len(), "|", parts[0] = "{euro}{cjk}", "|", parts[1] = "{face}"
BYTES invalid = BASE64DEC("/w==")
STRING decoded = invalid.ToString()
PRINTLN decoded.Len(), "|", Error.Last().Kind = ErrKind.String, "|", Error.Last().Code = ErrCode.Format
STRING long = STRING.Repeat("{face}", 3000)
BIGSTR bounded = long
PRINTLN long.Len(), "|", LEN(TOBYTES(long)), "|", LEN(bounded)
"#,
        face = '\u{1f600}',
        accent = '\u{301}',
        composed = '\u{e9}',
        euro = '\u{20ac}',
        cjk = '\u{754c}'
    );
    let expected = format!(
        "5|13|1\n2|3|4|5\n-1|0|-1\nCC81|F09F9880|0|0\n1|1|2|1|0\n[{}  ]|[{}  ]|[ {}]\n1|1\n1|7|3\n1|1\n2|2|2|2\n2|1|1\n0|1|1\n3000|12000|2048\n",
        '\u{1f600}', '\u{1f600}', '\u{20ac}'
    );
    roundtrip(&source, 400, 400, &expected);
}
