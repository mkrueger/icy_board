use icy_board_engine::{
    ast::Ast,
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use ppl_lsp::{completion::get_completion, hover::get_user_hover, signature_help::get_signature_help};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_lsp::lsp_types::{CompletionItemKind, HoverContents};

const DOMAIN: &str = "ENUM Bits\n One = 1\n Two = 2\nENDENUM\n";

fn analyze(source: &str, language: u16) -> (Ast, SemanticVisitor) {
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(language));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("has.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
    (ast, visitor)
}

#[test]
fn enum_has_completion_and_nominal_parameter_suggestions() {
    for language in [350, 400] {
        let source = format!(
            "{DOMAIN}CONST Bits Mask = Bits.One\nBits value, items(1)\nDECLARE FUNCTION Result() Bits\nPRINT value.Has(Mask)\nFUNCTION Result() Bits\n RETURN Bits.One\nENDFUNC\n"
        );
        let (ast, visitor) = analyze(&source, language);
        assert!(!visitor.errors.lock().unwrap().has_errors());
        for line in ["value.", "Mask.", "Bits.One.", "items(0).", "items[0].", "Result().", "Bits(1)."] {
            let items = get_completion(&ast, &visitor, line, source.len());
            let has = items.iter().find(|item| item.label == "Has").unwrap_or_else(|| panic!("{line}: {items:?}"));
            assert_eq!(Some(CompletionItemKind::METHOD), has.kind);
            assert_eq!(Some("(Bits mask) BOOLEAN"), has.detail.as_deref());
        }
        for line in ["items.", "Bits."] {
            let items = get_completion(&ast, &visitor, line, source.len());
            assert!(!items.iter().any(|item| item.label == "Has"), "{line}: {items:?}");
        }
        for receiver in ["value", "Mask", "Bits.One", "items(0)", "Result()"] {
            let line = format!("PRINT {receiver}.hAs(");
            let help = get_signature_help(&line, &visitor).unwrap();
            assert_eq!("Bits.Has(Bits mask) BOOLEAN", help.signatures[0].label);
            assert_eq!(Some(0), help.active_parameter);
            assert_eq!(1, help.signatures[0].parameters.as_ref().unwrap().len());
            let items = get_completion(&ast, &visitor, &line, source.len());
            assert!(items.iter().any(|item| item.label == "Bits.One"), "{line}: {items:?}");
            assert!(items.iter().any(|item| item.label == "Bits.Two"));
            assert!(!items.iter().any(|item| item.label == "RegexOptions.IgnoreCase"));
        }
        for line in ["items.Has(", "Bits.Has("] {
            assert!(get_signature_help(line, &visitor).is_none(), "{line}");
        }
    }
}

#[test]
fn enum_has_hover_for_members_constants_calls_and_record_fields() {
    for language in [350, 400] {
        let source = format!("{DOMAIN}CONST Bits Mask = Bits.One\nBits value, items(1)\nPRINT value.hAs(Mask), Bits.One.Has(Bits.Two), items(0).Has(Mask)\n");
        let (ast, visitor) = analyze(&source, language);
        for (offset, _) in source.to_ascii_lowercase().match_indices(".has(") {
            let hover = get_user_hover(&ast, &visitor, offset + 2).expect("enum method hover");
            let HoverContents::Markup(markup) = hover.contents else {
                panic!("markdown hover expected")
            };
            assert!(
                markup.value.contains("BOOLEAN Bits.") && markup.value.contains("(Bits mask)"),
                "{}",
                markup.value
            );
        }
    }
    let source = format!("{DOMAIN}TYPE Boxed\n Bits Value\n Bits Items(1)\nENDTYPE\nBoxed box\nPRINT box.Value.Has(Bits.One), box.Items(0).Has(Bits.One)\n");
    let (ast, visitor) = analyze(&source, 400);
    assert!(!visitor.errors.lock().unwrap().has_errors());
    for line in ["box.Value.", "box.Items(0).", "box.Items[0]."] {
        assert!(
            get_completion(&ast, &visitor, line, source.len()).iter().any(|item| item.label == "Has"),
            "{line}"
        );
    }
    assert!(
        !get_completion(&ast, &visitor, "box.Items.", source.len())
            .iter()
            .any(|item| item.label == "Has")
    );
    assert!(get_signature_help("box.Items.Has(", &visitor).is_none());
    assert_eq!(
        "Bits.Has(Bits mask) BOOLEAN",
        get_signature_help("box.Value.Has(", &visitor).unwrap().signatures[0].label
    );
}
