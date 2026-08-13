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
