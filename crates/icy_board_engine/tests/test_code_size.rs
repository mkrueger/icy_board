use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

/// Emitted code size in bytes per test program. Nothing here may grow without a reason,
/// and a shrinking number is the point of the exercise, so both directions are reported.
const EXPECTED_CODE_SIZE: &[(&str, usize)] = &[
    ("bitfunctions.pps", 198),
    ("bool_check.pps", 84),
    ("bool_function_value.pps", 118),
    ("bool_function_value2.pps", 46),
    ("bs2i.pps", 42),
    ("by_ref_parameter.pps", 102),
    ("cursor_pos.pps", 34),
    ("for_to_string.pps", 200),
    ("function_test.pps", 60),
    ("get_user.pps", 52),
    ("if_then.pps", 266),
    ("local_variables.pps", 80),
    ("oracle_money_date.pps", 364),
    ("oracle_string_edge.pps", 316),
    ("oracle_type_coercion.pps", 364),
    ("push_pop_test.pps", 46),
    ("recurse.pps", 144),
    ("select_case.pps", 510),
    ("sort.pps", 218),
    ("string_functions.pps", 492),
    ("test_constants.pps", 914),
    ("test_dim1.pps", 66),
    ("test_dim_bounds.pps", 176),
    ("test_dim_string_concat.pps", 24),
    ("test_functions.pps", 40),
    ("test_optext.pps", 20),
    ("test_rounding_bug.pps", 294),
    ("use_funcs1.pps", 24),
];

fn code_size(file_name: &PathBuf, source: &str) -> usize {
    let reg = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();

    let ast = parse_ast(file_name.clone(), errors.clone(), source, &reg, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, reg, errors.clone());
    compiler.compile(&[&ast]);

    let reporter = errors.lock().unwrap();
    assert!(
        !reporter.has_errors(),
        "{} did not compile:\n{}",
        file_name.display(),
        reporter.errors.iter().map(|e| format!("  {}", e.error)).collect::<Vec<_>>().join("\n")
    );
    drop(reporter);

    compiler.get_script().serialize().len() * 2
}

#[test]
fn test_the_emitted_code_does_not_grow() {
    let mut entries: Vec<PathBuf> = fs::read_dir("tests/test_data")
        .expect("Error reading test_data directory.")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "pps"))
        .collect();
    entries.sort();

    let mut measured = Vec::new();
    for entry in &entries {
        let name = entry.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(entry).unwrap();
        measured.push((name, code_size(entry, &source)));
    }

    let expected: Vec<(String, usize)> = EXPECTED_CODE_SIZE.iter().map(|(n, s)| ((*n).to_string(), *s)).collect();
    if measured == expected {
        return;
    }

    let mut report = String::new();
    for (name, size) in &measured {
        let was = expected.iter().find(|(n, _)| n == name).map(|(_, s)| *s);
        match was {
            Some(was) if was == *size => {}
            Some(was) => report.push_str(&format!("  {name}: {was} -> {size} ({:+})\n", *size as isize - was as isize)),
            None => report.push_str(&format!("  {name}: new, {size}\n")),
        }
    }
    for (name, size) in &expected {
        if !measured.iter().any(|(n, _)| n == name) {
            report.push_str(&format!("  {name}: gone, was {size}\n"));
        }
    }

    let table = measured
        .iter()
        .map(|(name, size)| format!("    (\"{name}\", {size}),"))
        .collect::<Vec<_>>()
        .join("\n");

    panic!("emitted code size changed:\n{report}\nupdate EXPECTED_CODE_SIZE to:\n{table}");
}
