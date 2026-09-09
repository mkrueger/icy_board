//! The compiler and the editor share the formatter but not its backend, so both
//! have to answer with the same text.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use icy_board_ppl::{
    compiler::workspace::Workspace,
    formatting::{FormattingOptions, FormattingVisitor, StringFormattingBackend},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};
use ppl_lsp::formatting::VSCodeFormattingBackend;
use ropey::Rope;

fn parse(path: &Path, source: &str) -> icy_board_ppl::ast::Ast {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    parse_ast(path.to_path_buf(), errors, source, &registry, Encoding::Utf8, &Workspace::default())
}

/// What `pplc --format` writes.
fn format_as_compiler(path: &Path, source: &str) -> String {
    let ast = parse(path, source);
    let options = FormattingOptions::default();
    let mut backend = StringFormattingBackend::new(source);
    let mut visitor = FormattingVisitor::new(&mut backend, &options);
    visitor.format(&ast);
    backend.apply()
}

/// What the editor gets from the server, applied the way an editor applies it.
fn format_as_editor(path: &Path, source: &str) -> String {
    let ast = parse(path, source);
    let rope = Rope::from_str(source);
    let options = FormattingOptions::default();
    let mut edits = {
        let mut backend = VSCodeFormattingBackend {
            edits: Vec::new(),
            rope: &rope,
        };
        let mut visitor = FormattingVisitor::new(&mut backend, &options);
        visitor.format(&ast);
        backend.edits
    };
    edits.sort_by_key(|b| std::cmp::Reverse(b.range.start));

    let mut text = rope;
    for edit in &edits {
        let start = text.line_to_char(edit.range.start.line as usize) + edit.range.start.character as usize;
        let end = text.line_to_char(edit.range.end.line as usize) + edit.range.end.character as usize;
        text.remove(start..end);
        text.insert(start, &edit.new_text);
    }
    text.to_string()
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

#[test]
fn the_editor_formats_like_the_compiler() {
    let mut sources = Vec::new();
    collect_sources(Path::new("../.."), &mut sources);
    assert!(sources.len() > 100, "expected the repository sources, found {}", sources.len());

    let mut different = Vec::new();
    for path in &sources {
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        let compiler = format_as_compiler(path, &source);
        let editor = format_as_editor(path, &source);
        if compiler != editor {
            let line = compiler
                .lines()
                .zip(editor.lines())
                .enumerate()
                .find(|(_, (left, right))| left != right)
                .map(|(number, (left, right))| format!("line {}:\n  compiler: {left:?}\n  editor:   {right:?}", number + 1))
                .unwrap_or_else(|| "differing line count".to_string());
            different.push(format!("{}\n{line}", path.display()));
        }
    }
    assert!(different.is_empty(), "the two answers differ for:\n{}", different.join("\n"));
}

#[test]
fn s2_formatting_preserves_short_circuit_spelling_and_parentheses() {
    for language in [340, 400] {
        let path = Path::new("s2.pps");
        let source = format!(";$LANGVERSION {language}\nBOOLEAN answer\nanswer=(TRUE||FALSE)&&!1=2|TRUE&FALSE\n");
        let expected = format!(";$LANGVERSION {language}\nBOOLEAN answer\nanswer = (TRUE || FALSE) && !1 = 2 | TRUE & FALSE\n");
        assert_eq!(format_as_compiler(path, &source), expected);
        assert_eq!(format_as_editor(path, &source), expected);
        assert_eq!(format_as_editor(path, &expected), expected);
    }
}

#[test]
fn s3_array_formatting_preserves_rank_and_call_delimiters() {
    let path = Path::new("s3.pps");
    let source = ";$LANGVERSION 400\nTYPE Payload\n INTEGER Values[]\n INTEGER Matrix[,]\n INTEGER Cube[,,]\n INTEGER Fixed[2]\nENDTYPE\nPayload item\nINTEGER values[]={}\nINTEGER matrix[,]\nINTEGER cube[,,]\nINTEGER bounded[10]\nvalues=Make()\nitem.Values[0]=values[0]\nmatrix[0,1]=cube[0,1,2]\nPRINTLN bounded.Len(),item.Fixed[0],Read(values)\nFUNCTION Make() INTEGER[]\nENDFUNC\nFUNCTION Read(INTEGER input[]) INTEGER\nRETURN input.Len()\nENDFUNC\n";
    let formatted = format_as_compiler(path, source);
    assert_eq!(format_as_editor(path, source), formatted);
    assert_eq!(format_as_compiler(path, &formatted), formatted);
    assert_eq!(format_as_editor(path, &formatted), formatted);
    for text in [
        "Values[]",
        "Matrix[,]",
        "Cube[,,]",
        "Fixed[2]",
        "values[]",
        "Make()",
        "item.Values[0]",
        "values[0]",
        "Read(values)",
    ] {
        assert!(formatted.contains(text), "{text}: {formatted}");
    }
    for language in [340, 400] {
        let source = format!(";$LANGVERSION {language}\nINTEGER data(2)\nPRINTLN data(0)\n");
        let formatted = format_as_compiler(path, &source);
        assert!(formatted.contains("data(2)") && formatted.contains("data(0)"), "{formatted}");
        assert_eq!(format_as_editor(path, &source), formatted);
    }
}
