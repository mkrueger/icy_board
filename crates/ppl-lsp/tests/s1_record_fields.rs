use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_ppl::{
    ast::Ast,
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use ppl_lsp::{completion::get_completion, hover::get_user_hover};
use tower_lsp::lsp_types::{CompletionItemKind, HoverContents};

const SOURCE: &str = r#";$LANGVERSION 400
TYPE Leaf
    INTEGER Number
ENDTYPE
TYPE Bundle
    SURFACE Image
    USER Owner
    AUDIO Clips[]
    INTEGER Values[]
    STRING Grid[,]
    Leaf Cells[,,]
    INTEGER Zero[0]
    INTEGER FixedGrid[2,3]
    INTEGER FixedCube[1,2,3]
    INTEGER Scalar
ENDTYPE
Bundle item
item.Image = item.Image
item.Owner = item.Owner
PRINTLN item.Clips.Len(), item.Values.Len(), item.Grid.Len(), item.Cells.Len()
PRINTLN item.Zero[0], item.FixedGrid[0,0], item.FixedCube[0,0,0], item.Scalar
"#;

const FIELDS: &[(&str, &str)] = &[
    ("Image", "Surface"),
    ("Owner", "User"),
    ("Clips", "Audio[]"),
    ("Values", "INTEGER[]"),
    ("Grid", "STRING[,]"),
    ("Cells", "Leaf[,,]"),
    ("Zero", "INTEGER[0]"),
    ("FixedGrid", "INTEGER[2, 3]"),
    ("FixedCube", "INTEGER[1, 2, 3]"),
    ("Scalar", "INTEGER"),
];

fn analyze(source: &str) -> (Ast, SemanticVisitor) {
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(400));
    workspace.package.runtime = Some(400);
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(
        PathBuf::from("s1_record_fields.pps"),
        errors.clone(),
        source,
        &registry,
        Encoding::Utf8,
        &workspace,
    );
    let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
    ast.visit(&mut visitor);
    visitor.finish();
    let errors = errors.lock().unwrap();
    assert!(
        !errors.has_errors(),
        "{:?}",
        errors.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
    );
    (ast, visitor)
}

#[track_caller]
fn hover_at(ast: &Ast, visitor: &SemanticVisitor, offset: usize, context: &str) -> String {
    let hover = get_user_hover(ast, visitor, offset).unwrap_or_else(|| panic!("missing hover for {context} at offset {offset}"));
    let HoverContents::Markup(contents) = hover.contents else {
        panic!("expected markup");
    };
    contents.value
}

fn assert_field_uses(ast: &Ast, visitor: &SemanticVisitor, source: &str, receiver: &str, owner: &str, fields: &[(&str, &str)]) {
    for (field, detail) in fields {
        let pattern = format!("{receiver}.{field}");
        let offsets: Vec<_> = source.match_indices(&pattern).map(|(start, _)| start + receiver.len() + 2).collect();
        assert!(!offsets.is_empty(), "missing fixture use: {pattern}");
        for offset in offsets {
            assert_eq!(
                hover_at(ast, visitor, offset, &pattern),
                format!("```PPL\n{detail} {owner}.{field}\n```"),
                "{pattern} at offset {offset}"
            );
        }
    }
}

#[test]
fn s1_record_type_hover_preserves_host_dynamic_and_fixed_fields() {
    let (ast, visitor) = analyze(SOURCE);
    let hover = hover_at(&ast, &visitor, SOURCE.find("Bundle item").unwrap() + 1, "Bundle item");
    let mut expected = "```PPL\nTYPE Bundle".to_string();
    for (field, detail) in FIELDS {
        expected.push_str(&format!("\n    {detail} {field}"));
    }
    expected.push_str("\nENDTYPE\n```");
    assert_eq!(hover, expected);
}

#[test]
fn s1_field_use_hover_preserves_dynamic_rank_and_fixed_zero_bounds() {
    let (ast, visitor) = analyze(SOURCE);
    assert_field_uses(&ast, &visitor, SOURCE, "item", "Bundle", FIELDS);
}

#[test]
fn s1_assignment_field_hover_preserves_all_shapes_on_both_sides() {
    let mut source = SOURCE.to_string();
    for (field, _) in FIELDS {
        source.push_str(&format!("LET item.{field} = item.{field}\n"));
    }
    let (ast, visitor) = analyze(&source);
    assert_field_uses(&ast, &visitor, &source, "item", "Bundle", FIELDS);
}

#[test]
fn s1_nested_assignment_hover_visits_each_member_and_array_index() {
    let mut source = SOURCE.replacen(
        "Bundle item\n",
        "TYPE Wrapper\n Bundle Nested\n Bundle Rows[]\n Bundle Matrix[,]\n Bundle Cube[,,]\nENDTYPE\nBundle item, items[0]\nWrapper outer, outers[0]\n",
        1,
    );
    let receivers = [
        "outer.Nested",
        "outer.Rows[0]",
        "outer.Rows(0)",
        "outer.Matrix[0,0]",
        "outer.Cube(0,0,0)",
        "outers[0].Nested",
        "outers(0).Rows[0]",
        "items[item.Scalar]",
        "items(0)",
    ];
    for receiver in receivers {
        for (field, _) in FIELDS {
            source.push_str(&format!("{receiver}.{field} = {receiver}.{field}\n"));
        }
    }
    let (ast, visitor) = analyze(&source);
    for receiver in receivers {
        assert_field_uses(&ast, &visitor, &source, receiver, "Bundle", FIELDS);
    }
    assert_field_uses(
        &ast,
        &visitor,
        &source,
        "outer",
        "Wrapper",
        &[("Nested", "Bundle"), ("Rows", "Bundle[]"), ("Matrix", "Bundle[,]"), ("Cube", "Bundle[,,]")],
    );
    assert_field_uses(&ast, &visitor, &source, "outers[0]", "Wrapper", &[("Nested", "Bundle")]);
    assert_field_uses(&ast, &visitor, &source, "outers(0)", "Wrapper", &[("Rows", "Bundle[]")]);
    assert_field_uses(&ast, &visitor, &source, "item", "Bundle", FIELDS);
}

#[test]
fn s1_indexed_field_hover_preserves_declared_shape_through_writes_and_calls() {
    let source = format!(
        "{SOURCE}\
         item.Clips[0] = item.Clips[0]\n\
         item.Values(0) = item.Values(0)\n\
         item.Grid[item.Scalar,0] = item.Grid[0,item.Scalar]\n\
         item.Cells[0,0,0].Number = item.Cells(0,0,0).Number\n\
         item.Zero[0] += item.Scalar\n\
         item.FixedGrid[0,0] = item.FixedGrid(0,0)\n\
         item.FixedCube(0,0,0) = item.FixedCube[0,0,0]\n\
         item.Image.Free()\n\
         item.Clips[0].Free()\n\
         PRINTLN item.Grid[0,0].Trim(), (item.Cells[0,0,0]).Number\n"
    );
    let (ast, visitor) = analyze(&source);
    assert_field_uses(&ast, &visitor, &source, "item", "Bundle", FIELDS);
    for receiver in ["item.Cells[0,0,0]", "item.Cells(0,0,0)", "(item.Cells[0,0,0])"] {
        assert_field_uses(&ast, &visitor, &source, receiver, "Leaf", &[("Number", "INTEGER")]);
    }
}

#[test]
fn s1_member_and_record_literal_completion_preserve_field_shapes() {
    let (ast, visitor) = analyze(SOURCE);
    for line in ["item.", "Bundle other = Bundle { "] {
        let items = get_completion(&ast, &visitor, line, SOURCE.chars().count());
        assert_eq!(items.len(), FIELDS.len(), "{line}: {items:?}");
        for (field, detail) in FIELDS {
            let item = items.iter().find(|item| item.label == *field).unwrap_or_else(|| panic!("{field}: {items:?}"));
            assert_eq!(item.detail.as_deref(), Some(*detail), "{line}: {field}");
            assert_eq!(item.kind, Some(CompletionItemKind::FIELD), "{line}: {field}");
        }
    }
}

#[test]
fn s1_host_and_multirank_field_chains_offer_their_members() {
    let (ast, visitor) = analyze(SOURCE);
    for (line, expected, absent) in [
        ("item.Image.", "Free", "Len"),
        ("item.Owner.", "Name", "Len"),
        ("item.Clips.", "Len", "Free"),
        ("item.Clips[0].", "Free", "Len"),
        ("item.Grid.", "Len", "ToUpper"),
        ("item.Cells.", "Len", "Number"),
        ("item.Cells[0,0,0].", "Number", "Len"),
    ] {
        let items = get_completion(&ast, &visitor, line, SOURCE.chars().count());
        assert!(items.iter().any(|item| item.label == expected), "{line}: {items:?}");
        assert!(!items.iter().any(|item| item.label == absent), "{line}: {items:?}");
    }
}
