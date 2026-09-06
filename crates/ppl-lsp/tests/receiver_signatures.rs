//! F12 regression scope: rank-aware completion/signatures, scalar enum arguments,
//! source 350/400 gates, and EN/DE unbounded STRING prose (legacy BIGSTR retained).
//! Direct public LSP handlers are exercised; no core or root documentation edits.

use icy_board_engine::{
    ast::Ast,
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::{BYTES_MEMBERS, STRING_MEMBERS, SemanticVisitor},
};
use ppl_lsp::{
    completion::get_completion,
    context::{CursorContext, cursor_context},
    signature_help::get_signature_help_for_version,
    type_lookup::receiver_type,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_lsp::lsp_types::{CompletionItem, ParameterLabel, SignatureHelp};

fn analyze(source: &str, language: u16) -> (Ast, SemanticVisitor) {
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(language));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("receivers.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
    let errors = visitor.errors.lock().unwrap();
    let messages: Vec<_> = errors.errors.iter().map(|error| error.error.to_string()).collect();
    assert!(!errors.has_errors(), "invalid fixture: {source}\n{messages:?}");
    drop(errors);
    (ast, visitor)
}

fn complete(ast: &Ast, visitor: &SemanticVisitor, line: &str) -> Vec<CompletionItem> {
    get_completion(ast, visitor, line, usize::MAX)
}

fn signature(visitor: &SemanticVisitor, line: &str, language: u16) -> SignatureHelp {
    get_signature_help_for_version(line, visitor, language).unwrap_or_else(|| panic!("missing signature for {line}"))
}

fn assert_array(ast: &Ast, visitor: &SemanticVisitor, line: &str, rank: u8, resizable: bool) {
    let items = complete(ast, visitor, line);
    let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, if resizable { vec!["Len", "Redim"] } else { vec!["Len"] }, "{line}");
    let CursorContext::Member(path) = cursor_context(line) else {
        panic!("{line}")
    };
    let receiver = receiver_type(visitor, &path).unwrap();
    assert_eq!(rank, receiver.rank, "{line}");
    assert_eq!(resizable, receiver.resizable, "{line}");
    let help = signature(visitor, &format!("{line}Len("), ast.language_version);
    assert!(help.signatures[0].label.ends_with("Len([INTEGER dimension]) INTEGER"), "{help:?}");
    assert_eq!(
        resizable,
        get_signature_help_for_version(&format!("{line}Redim("), visitor, ast.language_version).is_some(),
        "{line}"
    );
    for method in ["Contains", "GetChecksum", "Has", "FindAll"] {
        assert!(
            get_signature_help_for_version(&format!("{line}{method}("), visitor, ast.language_version).is_none(),
            "{line}{method}"
        );
    }
}

#[test]
fn arrays_fields_and_computed_results_keep_rank_and_mutability() {
    // Board objects cannot be embedded in user records; use a user record for
    // nested fields and exercise RegexMatch arrays independently.
    let source = "STRING text, parts[]\nBIGSTR bigs(2)\nBYTES blobs[]\nREGEX rx\nRegexMatch matches[]\nTYPE Hit\n STRING Groups[1]\nENDTYPE\nTYPE Boxed\n STRING names[2, 3]\n Hit hits[1]\nENDTYPE\nBoxed box, boxes[1]\n";
    let (ast, visitor) = analyze(source, 400);
    for line in ["parts.", "bigs.", "blobs.", "matches.", "boxes."] {
        assert_array(&ast, &visitor, line, 1, true);
    }
    for line in ["box.names.", "boxes[0].names.", "boxes(0).names."] {
        assert_array(&ast, &visitor, line, 2, false);
    }
    for line in [
        "box.hits.",
        "text.Split(\",\").",
        "STRING.Split(text, \",\").",
        "BIGSTR.Split(text, \",\", 2).",
        "text.Trim().Split(\",\").",
        "rx.FindAll(text).",
        "rx.Split(text).",
        "REGEX.Compile(\"a\").FindAll(text).",
        "box.hits[0].Groups.",
        "boxes[0].hits[0].Groups.",
    ] {
        assert_array(&ast, &visitor, line, 1, false);
    }
    for line in [
        "parts[0].",
        "parts(0).",
        "box.names[0, 0].",
        "boxes[0].names(0, 0).",
        "text.Split(\",\")[0].",
        "rx.Split(text)[0].",
    ] {
        let items = complete(&ast, &visitor, line);
        assert!(items.iter().any(|item| item.label == "Contains"), "{line}: {items:?}");
        assert!(!items.iter().any(|item| item.label == "Redim"));
    }
    assert!(complete(&ast, &visitor, "box.hits(0).").iter().any(|item| item.label == "Groups"));
    for line in ["matches[0].", "rx.FindAll(text)[0].", "REGEX.Compile(\"a\").FindAll(text)[0]."] {
        let items = complete(&ast, &visitor, line);
        assert!(items.iter().any(|item| item.label == "Group"), "{line}: {items:?}");
        assert!(!items.iter().any(|item| item.label == "Redim"));
    }
    assert!(complete(&ast, &visitor, "blobs[0].").iter().any(|item| item.label == "GetChecksum"));
    assert!(
        complete(&ast, &visitor, "rx.FindAll(text)[0].Group(0).")
            .iter()
            .any(|item| item.label == "Contains")
    );
}

#[test]
fn ordinary_and_callback_returns_preserve_all_ranks() {
    for rank in 1..=3 {
        let shape = format!("[{}]", ",".repeat(rank - 1));
        let indices = vec!["0"; rank].join(", ");
        let source = format!(
            "ENUM Bits\n One = 1\nENDENUM\nTYPE Boxed\n Bits flags{shape}\nENDTYPE\n\
             DECLARE FUNCTION Make() Boxed{shape}\n\
             PROCEDURE UseCallback(FUNCTION callback() Boxed{shape})\n ENDFUNCPLACEHOLDER\n\
             FUNCTION Make() Boxed{shape}\n Boxed result{shape}\n Make = result\nENDFUNC\n"
        )
        .replace(" ENDFUNCPLACEHOLDER", "ENDPROC");
        let (ast, visitor) = analyze(&source, 400);
        for root in ["Make()", "callback()"] {
            assert_array(&ast, &visitor, &format!("{root}."), rank as u8, false);
            let indexed = format!("{root}[{indices}]");
            assert!(complete(&ast, &visitor, &format!("{indexed}.")).iter().any(|item| item.label == "flags"));
            assert_array(&ast, &visitor, &format!("{indexed}.flags."), rank as u8, false);
            let line = format!("{indexed}.flags[{indices}].Has(");
            assert_eq!("Bits.Has(Bits mask) BOOLEAN", signature(&visitor, &line, 400).signatures[0].label);
            assert!(complete(&ast, &visitor, &line).iter().any(|item| item.label == "Bits.One"));
        }
        let ordinary = signature(&visitor, "Make(", 400);
        assert!(ordinary.signatures[0].label.ends_with(&format!("Boxed{shape}")));
        let callback = signature(&visitor, "callback(", 400);
        assert!(callback.signatures[0].label.ends_with(&format!("Boxed{shape}")));
    }
}

#[test]
fn scalar_signatures_cover_every_shared_definition_and_parameter_offset() {
    let (ast, visitor) = analyze("STRING text\nBIGSTR big\nBYTES blob\n", 400);
    for (receivers, definitions) in [
        (vec!["text", "big", "text.Trim()"], STRING_MEMBERS),
        (vec!["blob", "BYTES.FromBase64(\"YQ==\")"], BYTES_MEMBERS),
    ] {
        for definition in definitions {
            let statics = if std::ptr::eq(definitions, BYTES_MEMBERS) {
                vec!["BYTES"]
            } else {
                vec!["STRING", "BIGSTR"]
            };
            for receiver in if definition.is_static { &statics } else { &receivers } {
                let line = format!("{receiver}.{}(", definition.name);
                let help = signature(&visitor, &line, 400);
                let signature = &help.signatures[0];
                let parameters = signature.parameters.as_ref().unwrap();
                assert_eq!(*definition.arguments.end(), parameters.len(), "{line}: {signature:?}");
                for (index, parameter) in parameters.iter().enumerate() {
                    let ParameterLabel::LabelOffsets([start, end]) = parameter.label else {
                        panic!("{line}")
                    };
                    let text: String = signature.label.chars().skip(start as usize).take((end - start) as usize).collect();
                    assert!(!text.is_empty());
                    assert_eq!(index >= *definition.arguments.start(), text.starts_with('['), "{line}: {text}");
                }
                let return_type = ppl_lsp::type_lookup::type_name(&visitor.type_registry, definition.return_type);
                assert!(
                    signature
                        .label
                        .ends_with(&format!("{return_type}{}", if definition.name == "Split" { "[]" } else { "" })),
                    "{line}: {signature:?}"
                );
                let offered = complete(&ast, &visitor, &format!("{receiver}."));
                let item = offered.iter().find(|item| item.label == definition.name).unwrap();
                let detail = item.detail.as_ref().unwrap();
                assert!(signature.label.ends_with(detail), "{line}: {signature:?} vs {detail}");
            }
        }
    }
    assert_eq!(
        "STRING.Repeat(STRING text, INTEGER count) STRING",
        signature(&visitor, "STRING.Repeat(", 400).signatures[0].label
    );
    assert_eq!(
        "STRING.Join(STRING[] values, STRING separator) STRING",
        signature(&visitor, "STRING.Join(", 400).signatures[0].label
    );
    for line in ["text.Repeat(", "STRING.Contains(", "blob.FromBase64(", "BYTES.GetChecksum("] {
        assert!(get_signature_help_for_version(line, &visitor, 400).is_none(), "{line}");
    }
}

#[test]
fn contextual_comparison_and_checksum_enums_follow_calls_and_argument_positions() {
    let (ast, visitor) = analyze("STRING text, parts[]\nBIGSTR big\nBYTES blob, blobs[]\nREGEX rx\n", 400);
    for receiver in [
        "text",
        "big",
        "text.Trim()",
        "parts[0]",
        "rx.Split(text)[0]",
        "BYTES.FromBase64(\"YQ==\").ToString()",
    ] {
        for method in ["Contains", "StartsWith", "EndsWith", "Equals", "Count", "Find", "FindLast"] {
            let start = if method.starts_with("Find") { ", 0" } else { "" };
            let line = format!("{receiver}.{method}(\"a\"{start}, ");
            let items = complete(&ast, &visitor, &line);
            let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
            assert_eq!(vec!["StringComparison.Ordinal", "StringComparison.OrdinalIgnoreCase"], labels, "{line}");
            let help = signature(&visitor, &line, 400);
            assert_eq!(Some(if method.starts_with("Find") { 2 } else { 1 }), help.active_parameter);
            assert!(help.signatures[0].label.contains("[StringComparison comparison]"));
        }
    }
    for receiver in ["blob", "blobs[0]", "BYTES.FromBase64(\"YQ==\")", "blob.GetChecksum(Checksum.MD5)"] {
        let line = format!("{receiver}.GetChecksum(");
        let items = complete(&ast, &visitor, &line);
        assert_eq!(
            vec!["Checksum.CRC32", "Checksum.MD5", "Checksum.SHA256"],
            items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(),
            "{line}"
        );
        assert_eq!(
            "BYTES.GetChecksum(Checksum algorithm) BYTES",
            signature(&visitor, &line, 400).signatures[0].label
        );
    }
    for line in [
        "text.Contains(",
        "text.Find(\"a\", ",
        "parts.Contains(\"a\", ",
        "blobs.GetChecksum(",
        "text.Split(\",\").Contains(\"a\", ",
    ] {
        assert!(
            !complete(&ast, &visitor, line)
                .iter()
                .any(|item| item.label.starts_with("StringComparison.") || item.label.starts_with("Checksum.")),
            "{line}"
        );
    }
}

#[test]
fn regex_signatures_report_arrays_and_classic_methods_still_resolve() {
    let (_, visitor) = analyze("REGEX rx\n", 400);
    for receiver in ["rx", "REGEX.Compile(\"a\")"] {
        assert!(
            signature(&visitor, &format!("{receiver}.FindAll("), 400).signatures[0]
                .label
                .ends_with("RegexMatch[]")
        );
        assert!(
            signature(&visitor, &format!("{receiver}.Split("), 400).signatures[0]
                .label
                .ends_with("STRING[]")
        );
    }
    assert!(signature(&visitor, "Board.Users.Len(", 400).signatures[0].label.ends_with("INTEGER"));
    assert!(
        signature(&visitor, "Board.Conferences[0].HasAccess(", 400).signatures[0]
            .label
            .contains("HasAccess")
    );
    assert!(signature(&visitor, "Len(", 350).signatures[0].label.contains("LEN("));
    assert!(signature(&visitor, "Mid(\"text\", ", 350).signatures[0].label.contains("MID("));
}

#[test]
fn source_gates_preserve_enum_has_and_classic_array_methods() {
    for language in [350, 400] {
        let (ast, visitor) = analyze("ENUM Bits\n One = 1\nENDENUM\nBits flags(1)\nSTRING text, parts(1)\nBIGSTR big\n", language);
        for line in [
            "text.Contains(\"a\", ",
            "big.Contains(\"a\", ",
            "STRING.Repeat(",
            "BIGSTR.Split(",
            "BYTES.FromBase64(",
            "text.Split(\",\").Len(",
        ] {
            assert_eq!(
                language == 400,
                get_signature_help_for_version(line, &visitor, language).is_some(),
                "{language}: {line}"
            );
        }
        for line in ["text.", "big.", "STRING.", "BIGSTR.", "BYTES.", "text.Split(\",\")."] {
            assert_eq!(language == 350, complete(&ast, &visitor, line).is_empty(), "{language}: {line}");
        }
        assert_array(&ast, &visitor, "flags.", 1, true);
        assert_array(&ast, &visitor, "parts.", 1, true);
        for line in ["flags[0].Has(", "flags(0).Has(", "Bits.One.Has(", "Bits(1).Has("] {
            assert_eq!("Bits.Has(Bits mask) BOOLEAN", signature(&visitor, line, language).signatures[0].label);
            assert!(complete(&ast, &visitor, line).iter().any(|item| item.label == "Bits.One"));
        }
    }
}

#[test]
fn localized_new_string_results_are_unbounded_without_rewriting_legacy_bigstr() {
    for text in [include_str!("../i18n/en/ppl_lsp.ftl"), include_str!("../i18n/de/ppl_lsp.ftl")] {
        for key in [
            "regex-split",
            "string-replace",
            "string-trim",
            "string-trim-start",
            "string-trim-end",
            "string-to-upper",
            "string-to-lower",
            "string-split",
            "string-join",
            "string-repeat",
        ] {
            let prefix = format!("hint-{key}=");
            let line = text.lines().find(|line| line.starts_with(&prefix)).unwrap();
            assert!(line.contains("STRING") && !line.contains("BIGSTR"), "{line}");
            assert!(line.contains("unbounded") || line.contains("unbeschränkt"), "{line}");
        }
        assert!(
            text.lines()
                .find(|line| line.starts_with("hint-function-tobigstr="))
                .unwrap()
                .contains("BIGSTR")
        );
        assert!(text.lines().any(|line| line.contains("BIGSTR") && line.contains("2048")));
    }
}
