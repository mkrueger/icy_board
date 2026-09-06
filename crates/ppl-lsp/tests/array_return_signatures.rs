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
use ppl_lsp::{hover::get_user_hover, signature_help::get_signature_help};
use tower_lsp::lsp_types::{HoverContents, ParameterLabel};

fn analyze(source: &str) -> (Ast, SemanticVisitor) {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    assert!(errors.lock().unwrap().errors.is_empty(), "parse errors in {source}");
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
    (ast, visitor)
}

fn assert_signature_and_hover(source: &str, name: &str, expected: &str) {
    let (ast, visitor) = analyze(source);
    let help = get_signature_help(&format!("{name}("), &visitor).expect("signature help");
    assert_eq!(help.signatures.len(), 1);
    assert_eq!(help.signatures[0].label, expected);

    let offset = source.find(&format!("{name}(")).expect("routine name") + 1;
    let hover = get_user_hover(&ast, &visitor, offset).expect("routine hover");
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markup");
    };
    assert_eq!(content.value, format!("```PPL\n{expected}\n```"));
}

#[test]
fn declared_function_return_ranks_appear_in_signature_help_and_hover() {
    for suffix in ["", "[]", "[,]", "[,,]"] {
        let signature = format!("FUNCTION Values(INTEGER count) INTEGER{suffix}");
        let source = format!(";$LANGVERSION 400\nDECLARE {signature}\n");
        assert_signature_and_hover(&source, "Values", &signature);
    }
}

#[test]
fn implemented_function_return_ranks_appear_in_signature_help_and_hover() {
    for (suffix, bounds) in [("", ""), ("[]", "[2]"), ("[,]", "[2, 3]"), ("[,,]", "[2, 3, 4]")] {
        let signature = format!("FUNCTION Values() INTEGER{suffix}");
        let source = format!(";$LANGVERSION 400\n{signature}\nINTEGER result{bounds}\nRETURN result\nENDFUNC\n");
        assert_signature_and_hover(&source, "Values", &signature);
    }
}

#[test]
fn callback_return_ranks_appear_in_signature_help_and_hover() {
    for suffix in ["", "[]", "[,]", "[,,]"] {
        let callback = format!("FUNCTION Fetch(INTEGER count) INTEGER{suffix}");
        let signature = format!("PROCEDURE Apply({callback}, INTEGER count)");
        let source = format!(";$LANGVERSION 400\n{signature}\nENDPROC\n");
        assert_signature_and_hover(&source, "Apply", &signature);
        assert_signature_and_hover(&source, "Fetch", &callback);
    }
}

#[test]
fn nested_callback_return_ranks_preserve_signatures_and_parameter_offsets() {
    let inner = "FUNCTION Read(INTEGER count) INTEGER[]";
    let procedure = format!("PROCEDURE Visit({inner})");
    let callback = format!("FUNCTION Build({procedure}, FUNCTION Grid() INTEGER[,]) INTEGER[,,]");
    let signature = format!("FUNCTION Collect({callback}, INTEGER count) INTEGER[,]");
    let source = format!(";$LANGVERSION 400\n{signature}\nINTEGER result[2, 3]\nRETURN result\nENDFUNC\n");
    assert_signature_and_hover(&source, "Collect", &signature);
    assert_signature_and_hover(&source, "Build", &callback);

    let (_, visitor) = analyze(&source);
    let help = get_signature_help("Collect(Build, ", &visitor).expect("signature help");
    assert_eq!(help.active_parameter, Some(1));
    let signature = &help.signatures[0];
    let parameters = signature.parameters.as_ref().unwrap();
    assert_eq!(parameters.len(), 2);
    for (parameter, expected) in parameters.iter().zip([callback.as_str(), "INTEGER count"]) {
        let ParameterLabel::LabelOffsets([start, end]) = parameter.label else {
            panic!("expected offsets");
        };
        let marked: String = signature.label.chars().skip(start as usize).take((end - start) as usize).collect();
        assert_eq!(marked, expected);
    }
}
