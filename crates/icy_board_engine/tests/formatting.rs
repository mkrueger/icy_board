use std::{
    fs,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::workspace::Workspace,
    formatting::{FormattingOptions, FormattingVisitor, StringFormattingBackend},
    parser::{Encoding, ErrorReporter, UserTypeRegistry},
};

#[test]
pub fn test_formatting() {
    for entry in fs::read_dir("tests/formatting_tests").expect("Error reading test_data directory.") {
        let cur_entry = entry.unwrap().path();
        if cur_entry.extension().unwrap() != "pps" {
            continue;
        }

        let reg = UserTypeRegistry::default();
        let input = fs::read_to_string(&cur_entry).unwrap();
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let ast = icy_board_engine::parser::parse_ast(cur_entry.clone(), errors.clone(), &input, &reg, Encoding::Utf8, &Workspace::default());
        let mut backend = StringFormattingBackend {
            text: input.chars().collect(),
            edits: Vec::new(),
        };
        let options = FormattingOptions::default();
        let mut visitor = FormattingVisitor::new(&mut backend, &options);
        ast.visit(&mut visitor);

        backend.edits.sort_by_key(|(range, _)| range.start);

        for (range, edit) in backend.edits.iter().rev() {
            backend.text.splice(range.clone(), edit.chars());
        }
        let expected_output = fs::read_to_string(&cur_entry.with_extension("out")).unwrap();

        assert_eq!(backend.text.iter().collect::<String>(), expected_output);
    }
}
