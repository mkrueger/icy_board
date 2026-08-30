use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::Ast,
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use ppl_lsp::{document_symbol::get_document_symbols, documentation::get_type_hover, hover::get_user_hover};
use ropey::Rope;
use tower_lsp::lsp_types::{HoverContents, SymbolKind};

fn analyze(source: &str) -> (Ast, SemanticVisitor) {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
    (ast, visitor)
}

/// Hovers inside the first word of the first occurrence of `pattern`.
fn hover(source: &str, pattern: &str) -> Option<String> {
    let (ast, visitor) = analyze(source);
    let offset = source.find(pattern).unwrap_or_else(|| panic!("{pattern} not found")) + 1;
    let hover = get_user_hover(&ast, &visitor, offset)?;
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup");
    };
    Some(content.value)
}

const SOURCE: &str = r#"TYPE Member
    STRING  Name
    INTEGER Age
ENDTYPE

DECLARE PROCEDURE Show(STRING text)

Member people(10)
INTEGER count = 0

:START
count = count + 1
PRINTLN people[0].Name
Show("done")
GOTO START

PROCEDURE Show(STRING text)
    PRINTLN text
ENDPROC

FUNCTION Total(INTEGER v) INTEGER
    RETURN v + 1
ENDFUNC
"#;

#[test]
fn the_outline_lists_types_routines_and_variables() {
    let (ast, _) = analyze(SOURCE);
    let symbols = get_document_symbols(&ast, &Rope::from_str(SOURCE));

    let names: Vec<(String, SymbolKind)> = symbols.iter().map(|s| (s.name.clone(), s.kind)).collect();
    assert_eq!(
        names,
        vec![
            ("Member".to_string(), SymbolKind::STRUCT),
            ("people".to_string(), SymbolKind::VARIABLE),
            ("count".to_string(), SymbolKind::VARIABLE),
            ("Show".to_string(), SymbolKind::METHOD),
            ("Total".to_string(), SymbolKind::FUNCTION),
        ]
    );

    let fields = symbols[0].children.as_ref().expect("the record has fields");
    assert_eq!(fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>(), vec!["Name", "Age"]);
    assert_eq!(fields[0].kind, SymbolKind::FIELD);
}

#[test]
fn the_outline_spans_the_whole_routine() {
    let (ast, _) = analyze(SOURCE);
    let symbols = get_document_symbols(&ast, &Rope::from_str(SOURCE));
    let total = symbols.iter().find(|s| s.name == "Total").unwrap();
    assert!(total.range.end.line > total.range.start.line, "{:?}", total.range);
    assert_eq!(total.selection_range.start.line, total.range.start.line);
}

#[test]
fn hover_over_a_variable_shows_its_type() {
    assert_eq!(hover(SOURCE, "count = count"), Some("```PPL\nINTEGER count\n```".to_string()));
}

#[test]
fn hover_over_a_user_constant_shows_type_name_and_value() {
    let source = ";$LANGVERSION 400\nCONST INTEGER BASE = 17\nCONST INTEGER OFFSET = BASE + 1\nPRINTLN OFFSET\n";
    assert_eq!(
        hover(source, "BASE +"),
        Some("```PPL\nCONSTANT INTEGER BASE = 17\n```".to_string())
    );
    assert_eq!(
        hover(source, "OFFSET\n"),
        Some("```PPL\nCONSTANT INTEGER OFFSET = BASE + 1\n```".to_string())
    );
}

#[test]
fn hover_over_an_array_shows_its_bounds() {
    assert_eq!(hover(SOURCE, "people[0]"), Some("```PPL\nMember people[10]\n```".to_string()));
}

#[test]
fn hover_over_a_routine_shows_its_signature() {
    assert_eq!(hover(SOURCE, "Show(\"done\")"), Some("```PPL\nPROCEDURE Show(STRING text)\n```".to_string()));
    assert_eq!(
        hover(SOURCE, "Total(INTEGER v)"),
        Some("```PPL\nFUNCTION Total(INTEGER v) INTEGER\n```".to_string())
    );
}

#[test]
fn hover_over_a_label_shows_it_is_one() {
    assert_eq!(hover(SOURCE, "START\ncount"), Some("```PPL\n:START\n```".to_string()));
}

#[test]
fn hover_over_a_field_shows_the_record_it_belongs_to() {
    assert_eq!(hover(SOURCE, "Name\nShow"), Some("```PPL\nSTRING Member.Name\n```".to_string()));
}

#[test]
fn hover_over_new_board_user_members_includes_documentation() {
    let source = "PRINTLN Board.Users.Len()\nPRINTLN Board.Users[0].Valid\n";
    let users = hover(source, "Users.Len").unwrap();
    assert!(users.contains("User[] Board.Users"), "{users}");
    assert!(users.contains("\n```\n\n") && users.contains("`Board`"), "{users}");

    let valid = hover(source, "Valid").unwrap();
    assert!(valid.contains("BOOLEAN User.Valid"), "{valid}");
    assert!(valid.contains("\n```\n\n") && valid.contains("`Board.Users`"), "{valid}");
}

#[test]
fn hover_over_new_api_call_names_and_describes_parameters() {
    let source = ";$LANGVERSION 400\nBOOLEAN ok = Terminal.Gfx.Init(GfxBackend.Auto, TRUE)\n";
    let init = hover(source, "Init").unwrap();
    assert!(init.contains("GfxBackend backend") && init.contains("BOOLEAN fullscreen"), "{init}");
    assert!(init.contains("**Parameters**") || init.contains("**Parameter**"), "{init}");
    assert!(init.contains("`backend`") && init.contains("`fullscreen`"), "{init}");
    assert!(init.contains("CSI ? 25 l") && init.contains("Sixel"), "{init}");
}

#[test]
fn margin_hover_exposes_the_exact_terminal_sequence() {
    let source = ";$LANGVERSION 400\nBOOLEAN ok = Terminal.Margins.SetVertical(2, 23)\n";
    let set_vertical = hover(source, "SetVertical").unwrap();
    assert!(set_vertical.contains("CSI top ; bottom r"), "{set_vertical}");
    assert!(set_vertical.contains("ESC [ top ; bottom r"), "{set_vertical}");
    assert!(set_vertical.contains("DECSTBM"), "{set_vertical}");
}

#[test]
fn hover_over_bytes_type_and_members_includes_documentation() {
    let source = r#";$LANGVERSION 400
BYTES raw = ToBytes("abc")
PRINTLN raw.ToHex()
PRINTLN raw.GetChecksum(Checksum.SHA256)
PRINTLN Bytes.FromBase64("YWJj").ToString()
"#;

    let HoverContents::Markup(bytes_type) = get_type_hover(icy_board_engine::executable::VariableType::Bytes).unwrap().contents else {
        panic!("expected markup");
    };
    let bytes_type = bytes_type.value;
    assert!(bytes_type.contains("BYTES"), "{bytes_type}");
    assert!(bytes_type.contains("binary data") || bytes_type.contains("Binärdaten"), "{bytes_type}");

    let to_hex = hover(source, "ToHex").unwrap();
    assert!(to_hex.contains("STRING BYTES.ToHex"), "{to_hex}");
    assert!(to_hex.contains("leading zero bytes") || to_hex.contains("führende Nullbytes"), "{to_hex}");

    let checksum = hover(source, "GetChecksum").unwrap();
    assert!(checksum.contains("BYTES BYTES.GetChecksum"), "{checksum}");
    assert!(checksum.contains("CRC32") && checksum.contains("MD5") && checksum.contains("SHA256") && checksum.contains("32"), "{checksum}");

    let from_base64 = hover(source, "FromBase64").unwrap();
    assert!(from_base64.contains("BYTES BYTES.FromBase64"), "{from_base64}");
    assert!(from_base64.contains("malformed input") || from_base64.contains("Ungültige Eingabe"), "{from_base64}");
}

#[test]
fn hover_over_graphics_api_includes_documentation() {
    let source = r#";$LANGVERSION 400
Terminal.Gfx.Init(GfxBackend.Auto, FALSE)
IF Terminal.Gfx.Backend <> GfxBackend.None THEN
    SURFACE banner = Surface.Load("banner.png")
    banner.PresentAt(18, 2)
    banner.Free()
ENDIF
Terminal.Gfx.Shutdown()
"#;

    let assert_documented = |text: &str| assert!(text.contains("\n```\n\n"), "{text}");

    let gfx = hover(source, "Gfx.Init").unwrap();
    assert!(gfx.contains("Gfx Terminal.Gfx"), "{gfx}");
    assert_documented(&gfx);

    let init = hover(source, "Init").unwrap();
    assert!(init.contains("BOOLEAN Gfx.Init([GfxBackend backend], [BOOLEAN fullscreen])"), "{init}");
    assert_documented(&init);

    let auto = hover(source, "Auto").unwrap();
    assert!(auto.contains("GfxBackend.AUTO"), "{auto}");
    assert_documented(&auto);

    let backend = hover(source, "Backend <>").unwrap();
    assert!(backend.contains("GfxBackend Gfx.Backend"), "{backend}");
    assert_documented(&backend);

    let load = hover(source, "Load").unwrap();
    assert!(load.contains("Surface Surface.Load"), "{load}");
    assert_documented(&load);

    let present_at = hover(source, "PresentAt").unwrap();
    assert!(present_at.contains("BOOLEAN Surface.PresentAt(INTEGER column, INTEGER row)"), "{present_at}");
    assert_documented(&present_at);

    let free = hover(source, "Free").unwrap();
    assert!(free.contains("BOOLEAN Surface.Free"), "{free}");
    assert_documented(&free);

    let shutdown = hover(source, "Shutdown").unwrap();
    assert!(shutdown.contains("BOOLEAN Gfx.Shutdown"), "{shutdown}");
    assert_documented(&shutdown);
}

#[test]
fn hover_over_a_record_type_shows_its_fields() {
    let text = hover(SOURCE, "Member people").unwrap();
    assert!(text.contains("TYPE Member"), "{text}");
    assert!(text.contains("STRING Name"), "{text}");
    assert!(text.contains("INTEGER Age"), "{text}");
}
