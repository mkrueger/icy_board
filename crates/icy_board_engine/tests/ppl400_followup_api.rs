//! F9/F10/F11 follow-up: serialized PPE, private harness (no shared helper edits).
//! Classic evidence: PCBoard PPL/VAR.CPP cVARVAL::stripatx and MAIN/SCRIPT.C
//! removecodes delete only complete uppercase @X plus two hex digits. This is
//! source evidence, NOT a fresh DOS runtime oracle capture. The already separate
//! runtime-400 StringStripAtx opcode is fixed; released STRIPATX stays unchanged.
//! Error policy: pure/default/explicit Ordinal comparisons preserve old errors;
//! fallible IgnoreCase comparisons clear old errors on success, never same-statement
//! failures. Invalid modes and regex resource failures still publish errors.

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::Executable,
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    vm::{self, io::DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

fn run(source: &str, language: u16, runtime: u16) -> String {
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(runtime);
    workspace.set_default_language_version(Some(language));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("followup_api.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let messages: Vec<_> = errors.lock().unwrap().errors.iter().map(|e| e.error.to_string()).collect();
    assert!(messages.is_empty(), "{source}\n{messages:?}");
    let executable = compiler.create_executable().unwrap();
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
            let result = vm::run(&PathBuf::from("followup_api.ppe"), &executable, &mut io, &mut state).await;
            drop(state);
            result
        };
        let (output, result) = tokio::join!(reader, execute);
        assert!(result.is_ok(), "{source}\n{result:?}; output={output}");
        output
    })
}

#[test]
fn split_evaluates_receiver_separator_limit_once_in_order() {
    for language in [400] {
        for static_form in [false, true] {
            for limit in [None, Some(0), Some(1), Some(2), Some(-1)] {
                let arguments = if limit.is_some() { "Separator(), LimitValue()" } else { "Separator()" };
                let call = if static_form {
                    format!("STRING.Split(SourceValue(), {arguments})")
                } else {
                    format!("SourceValue().Split({arguments})")
                };
                let source = format!(
                    r#"
DECLARE FUNCTION SourceValue() STRING
DECLARE FUNCTION Separator() STRING
DECLARE FUNCTION LimitValue() INTEGER
INTEGER sources, separators, limits
STRING parts[]
parts = {call}
PRINT "|", sources, separators, limits, "|", parts.Len(), "|", Error.Last().OK
FUNCTION SourceValue() STRING
 sources = sources + 1
 PRINT "S"
 SourceValue = "a,b,c"
ENDFUNC
FUNCTION Separator() STRING
 separators = separators + 1
 PRINT "P"
 Separator = ","
ENDFUNC
FUNCTION LimitValue() INTEGER
 limits = limits + 1
 PRINT "L"
 LimitValue = {}
ENDFUNC
"#,
                    limit.unwrap_or(0)
                );
                let count = match limit {
                    Some(-1) => 0,
                    Some(1) => 1,
                    Some(2) => 2,
                    _ => 3,
                };
                let expected = format!(
                    "{}|11{}|{count}|{}",
                    if limit.is_some() { "SPL" } else { "SP" },
                    i32::from(limit.is_some()),
                    i32::from(limit != Some(-1))
                );
                assert_eq!(expected, run(&source, language, 400), "language={language}, {call}");
            }
        }
    }
}

#[test]
fn split_validates_after_all_arguments_and_keeps_first_argument_error() {
    for language in [400] {
        for call in [
            "Base64Dec(\"!\").ToString().Split(\"\", -1)",
            "STRING.Split(Base64Dec(\"!\").ToString(), \"\", -1)",
        ] {
            assert_eq!(
                "0|1|1",
                run(
                    &format!(
                        "STRING parts[]\nparts = {call}\nPRINT parts.Len(), \"|\", Error.Last().Kind = ErrKind.String, \"|\", Error.Last().Code = ErrCode.Format\n"
                    ),
                    language,
                    400
                )
            );
        }
    }
}

#[test]
fn find_all_validates_limit_at_every_start_boundary() {
    for language in [400] {
        let mut source = String::from("REGEX rx = REGEX.Compile(\"a\")\nREGEXMATCH matches[]\nSTRING bad\n");
        let mut expected = String::new();
        for start in [-2147483647, -1, 0, 1, 2, 2147483647] {
            for limit in [-2147483647, -1, 0, 1, 100000, 100001, 2147483647] {
                let code = if limit < 0 {
                    "Invalid"
                } else if limit > 100000 {
                    "Limit"
                } else {
                    "Ok"
                };
                source.push_str(&format!(
                    "bad = Base64Dec(\"!\")\nmatches = rx.FindAll(\"a\", {start}, {limit})\nPRINT matches.Len(), Error.Last().Code = ErrCode.{code}, Error.Last().OK, \"|\"\n"
                ));
                let valid = (0..=100000).contains(&limit);
                expected.push_str(&format!("{}1{}|", i32::from(valid && start == 0), i32::from(valid)));
            }
        }
        assert_eq!(expected, run(&source, language, 400), "language={language}");
    }
}

#[test]
fn find_all_unicode_end_position_and_same_statement_failure() {
    for language in [400] {
        assert_eq!(
            "1|2|1|0|1|0|1|1",
            run(
                r#"
REGEX rx = REGEX.Compile("$")
REGEXMATCH matches[]
matches = rx.FindAll("äβ", 2, 100000)
PRINT matches.Len(), "|", matches[0].Start, "|", Error.Last().OK, "|"
matches = rx.FindAll("äβ", 3, 100000)
PRINT matches.Len(), "|", Error.Last().OK, "|"
matches = rx.FindAll(Base64Dec("!"), 2, -1)
PRINT matches.Len(), "|", Error.Last().Code = ErrCode.Format, "|", Error.Last().Kind = ErrKind.String
"#,
                language,
                400
            )
        );
    }
}

#[test]
fn modern_stripatx_preserves_literal_bytes_and_removes_only_complete_controls() {
    let cases = [
        ("", ""),
        ("email@", "email@"),
        ("@X", "@X"),
        ("@X1", "@X1"),
        ("a@X1Zb", "a@X1Zb"),
        ("@XZ1", "@XZ1"),
        ("@x1F", "@x1F"),
        ("@@X0F", "@"),
        ("@X@X0F", "@X"),
        ("@X1@X0F", "@X1"),
        ("@X0Fhello@X07 world", "hello world"),
        ("@Xaf@XAF@X00@X99", ""),
        ("ä@X0Fβ@X1Z雪@", "äβ@X1Z雪@"),
        ("@XéF@X１F", "@XéF@X１F"),
        ("@USER@ @CLS@", "@USER@ @CLS@"),
    ];
    for language in [400] {
        let mut source = String::new();
        for (input, expected) in cases {
            // Compare values, never send @ control text through board rendering.
            source.push_str(&format!("PRINT \"{input}\".StripATX() = \"{expected}\"\n"));
        }
        source.push_str("STRING longtext\nlongtext = STRING.Repeat(\"ä\", 300) + \"@X0Fβ@\"\nPRINT longtext.StripATX() = STRING.Repeat(\"ä\", 300) + \"β@\"\n");
        assert_eq!("1".repeat(cases.len() + 1), run(&source, language, 400));
    }
}

#[test]
fn released_classic_stripatx_scanner_and_string_storage_are_unchanged() {
    // Characterize current released Icy behavior, not PCBoard oracle expectations.
    for (language, runtime) in [(340, 340), (350, 350), (350, 400), (400, 400)] {
        assert_eq!(
            "111111",
            run(
                &format!(
                    "PRINT STRIPATX(\"email@\") = \"email\"\nPRINT STRIPATX(\"a@X1Zb\") = \"a@AZb\"\nPRINT STRIPATX(\"@X0Fok\") = \"ok\"\nPRINT STRIPATX(\"@x0F\") = \"@x0F\"\nPRINT LEN(STRIPATX(\"{0}\")) = 300\nSTRING stored\nstored = STRIPATX(\"{0}\")\nPRINT LEN(stored) = {1}\n",
                    "a".repeat(300),
                    if language < 400 { 256 } else { 300 }
                ),
                language,
                runtime
            ),
            "language={language}, runtime={runtime}"
        );
    }
}

#[test]
fn pure_ordinal_overloads_preserve_stale_errors_consistently() {
    for language in [400] {
        let expressions = [
            "\"abc\".Contains(\"a\")",
            "\"abc\".Contains(\"a\", StringComparison.Ordinal)",
            "\"abc\".Contains(\"z\")",
            "\"abc\".Contains(\"z\", StringComparison.Ordinal)",
            "\"abc\".Contains(\"\")",
            "\"abc\".Contains(\"\", StringComparison.Ordinal)",
            "\"abc\".StartsWith(\"a\", StringComparison.Ordinal)",
            "\"abc\".EndsWith(\"c\", StringComparison.Ordinal)",
            "\"abc\".Equals(\"abc\", StringComparison.Ordinal)",
            "\"abc\".Find(\"a\", 0, StringComparison.Ordinal)",
            "\"abc\".FindLast(\"a\", 2, StringComparison.Ordinal)",
            "\"abc\".Count(\"a\", StringComparison.Ordinal)",
        ];
        let mut source = String::from("STRING bad\nINTEGER result\n");
        for expression in expressions {
            source.push_str(&format!(
                "bad = Base64Dec(\"!\")\nresult = {expression}\nPRINT Error.Last().Code = ErrCode.Format\n"
            ));
        }
        assert_eq!("1".repeat(expressions.len()), run(&source, language, 400));
    }
}

#[test]
fn fallible_comparisons_clear_older_errors_but_keep_current_statement_failures() {
    for language in [400] {
        assert_eq!(
            "11|011|011|11",
            run(
                r#"
STRING bad = Base64Dec("!")
BOOLEAN found = "abc".Contains("A", StringComparison.OrdinalIgnoreCase)
PRINT found, Error.Last().OK, "|"
found = Base64Dec("!").ToString().Contains("a", StringComparison.OrdinalIgnoreCase)
PRINT found, Error.Last().Code = ErrCode.Format, Error.Last().Kind = ErrKind.String, "|"
STRING needle = STRING.Repeat("a", 1000000)
found = "a".Contains(needle, StringComparison.OrdinalIgnoreCase)
PRINT found, Error.Last().Code = ErrCode.Limit, Error.Last().Kind = ErrKind.String, "|"
found = "abc".Contains("A", StringComparison.OrdinalIgnoreCase)
PRINT found, Error.Last().OK
"#,
                language,
                400
            )
        );
    }
}
