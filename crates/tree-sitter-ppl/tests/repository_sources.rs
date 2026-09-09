//! Parses every PPL source in the repository and fails on a syntax error, so
//! that a change to the grammar cannot silently break real sources.

use std::path::{Path, PathBuf};

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
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

fn first_error(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    if !node.has_error() {
        return None;
    }
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).find_map(first_error);
    found
}

#[test]
fn every_source_in_the_repository_parses() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sources = Vec::new();
    collect_sources(&root, &mut sources);
    assert!(sources.len() > 100, "expected the repository sources, found {}", sources.len());

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_ppl::LANGUAGE.into()).unwrap();

    let mut failures = Vec::new();
    for path in &sources {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let tree = parser.parse(&text, None).unwrap();
        if let Some(node) = first_error(tree.root_node()) {
            let start = node.start_position();
            failures.push(format!("{}:{}:{}", path.display(), start.row + 1, start.column + 1));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} sources failed to parse:\n{}",
        failures.len(),
        sources.len(),
        failures.join("\n")
    );
}

#[test]
fn s1_host_and_dynamic_record_fields_parse_with_all_ranks() {
    let source = r#";$LANGVERSION 400
TYPE Entry
    AREA Destination
    SURFACE Image
    AUDIO Sound
    INTEGER Values[]
    STRING Grid[,]
    INTEGER Cube[,,]
    INTEGER Fixed[0]
ENDTYPE
TYPE Menu
    Entry Items[]
ENDTYPE
Menu menus[1]
menus[0].Items.Redim(1)
REDIM menus[0].Items[0].Grid, 2, 3
menus[0].Items[0].Values.Redim(0)
menus[0].Items[0].Values[0] = 42
menus[0].Items[1] = Entry { Destination = menus[0].Items[0].Destination }
"#;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_ppl::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    assert!(!tree.root_node().has_error(), "{}", tree.root_node().to_sexp());
}

#[test]
fn s2_operator_precedence_matches_original_ppl() {
    fn expression_shape(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
        if let Some(operator) = node.child_by_field_name("operator") {
            let operator = operator.utf8_text(source).unwrap();
            if let Some(operand) = node.child_by_field_name("operand") {
                return Some(format!("({operator} {})", expression_shape(operand, source).unwrap()));
            }
            if let (Some(left), Some(right)) = (node.child_by_field_name("left"), node.child_by_field_name("right")) {
                return Some(format!(
                    "({operator} {} {})",
                    expression_shape(left, source).unwrap(),
                    expression_shape(right, source).unwrap()
                ));
            }
        }
        let text = node.utf8_text(source).unwrap();
        if node.named_child_count() == 0 && (text.parse::<i32>().is_ok() || matches!(text, "TRUE" | "FALSE")) {
            return Some(text.to_string());
        }
        let mut cursor = node.walk();
        let result = node.named_children(&mut cursor).find_map(|child| expression_shape(child, source));
        result
    }
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_ppl::LANGUAGE.into()).unwrap();
    for (expression, expected) in [
        ("TRUE | FALSE & FALSE", "(| TRUE (& FALSE FALSE))"),
        ("TRUE || FALSE && FALSE", "(|| TRUE (&& FALSE FALSE))"),
        ("!1 = 2 & TRUE", "(& (! (= 1 2)) TRUE)"),
        ("2^3^2", "(^ (^ 2 3) 2)"),
        ("-2^2", "(^ (- 2) 2)"),
    ] {
        let source = format!("PRINTLN {expression}\n");
        let tree = parser.parse(&source, None).unwrap();
        assert!(!tree.root_node().has_error(), "{source}: {}", tree.root_node().to_sexp());
        assert_eq!(expression_shape(tree.root_node(), source.as_bytes()).unwrap(), expected, "{source}");
    }
}
