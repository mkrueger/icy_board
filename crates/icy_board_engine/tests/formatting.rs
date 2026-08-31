use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::workspace::Workspace,
    formatting::{FormattingOptions, FormattingVisitor, StringFormattingBackend},
    parser::{Encoding, ErrorReporter, UserTypeRegistry},
};

fn format(path: &Path, source: &str) -> String {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = icy_board_engine::parser::parse_ast(path.to_path_buf(), errors, source, &registry, Encoding::Utf8, &Workspace::default());
    let mut backend = StringFormattingBackend::new(source);
    let options = FormattingOptions::default();
    let mut visitor = FormattingVisitor::new(&mut backend, &options);
    visitor.format(&ast);
    backend.apply()
}

#[test]
pub fn test_formatting() {
    for entry in fs::read_dir("tests/formatting_tests").expect("Error reading test_data directory.") {
        let cur_entry = entry.unwrap().path();
        if cur_entry.extension().unwrap() != "pps" {
            continue;
        }
        let input = fs::read_to_string(&cur_entry).unwrap();
        let expected_output = fs::read_to_string(cur_entry.with_extension("out")).unwrap();
        assert_eq!(format(&cur_entry, &input), expected_output, "{}", cur_entry.display());
    }
}

#[test]
fn indexed_member_assignments_format_without_becoming_calls() {
    let source = r#";$LANGVERSION 400
BEGIN
    Board.Conferences[0].Name = "x"
    LET Board.Conferences[0].Areas[0].Name = "y"
    Board.Conferences[0].Doors[0].Description += "!"
    LET Session.User.Notes[0] += "note"
END
"#;

    assert_eq!(format(Path::new("indexed_assignments.pps"), source), source);
}

#[test]
fn modules_format_as_nested_declaration_blocks() {
    let source = "MODULE Example\nPROCEDURE PublicCall()\nPRINTLN \"public\"\nENDPROC\nPRIVATE\nINTEGER state\nENDMODULE\n";
    let expected = "MODULE Example\n    PROCEDURE PublicCall()\n        PRINTLN \"public\"\n    ENDPROC\n    PRIVATE\n    INTEGER state\nENDMODULE\n";
    assert_eq!(format(Path::new("module.pps"), source), expected);
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target" || name == ".git") {
                continue;
            }
            collect_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "pps") {
            out.push(path);
        }
    }
}

/// Formatting an already formatted source may not change it again, or an editor
/// would rewrite a file every time it is saved.
#[test]
pub fn formatting_settles() {
    let mut sources = Vec::new();
    collect_sources(Path::new("../.."), &mut sources);
    assert!(sources.len() > 100, "expected the repository sources, found {}", sources.len());

    let mut unsettled = Vec::new();
    for path in &sources {
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        let once = format(path, &source);
        let twice = format(path, &once);
        if once != twice {
            unsettled.push(path.display().to_string());
        }
    }
    assert!(unsettled.is_empty(), "formatting these does not settle:\n{}", unsettled.join("\n"));
}
