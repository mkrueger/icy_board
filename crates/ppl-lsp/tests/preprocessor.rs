mod common;

use common::Server;
use serde_json::json;

#[test]
fn every_preprocessor_directive_has_hover_help() {
    let source = ";$LANGVERSION 400\n;$DEFINE FEATURE=1\n;$IF FEATURE\n;$ELSEIF RUNTIME < 400\n;$ELIF LANGVERSION < 400\n;$ELSE\n;$ENDIF\n;$USEFUNCS\nPRINTLN ;#FEATURE\n";
    let uri = "file:///tmp/preprocessor-hover.pps";
    let (mut server, _) = Server::ready();
    server.opened(uri, source);

    for (line, expected) in [
        (0, ";$LANGVERSION"),
        (1, ";$DEFINE"),
        (2, ";$IF"),
        (3, ";$ELSEIF"),
        (4, ";$ELIF"),
        (5, ";$ELSE"),
        (6, ";$ENDIF"),
        (7, ";$USEFUNCS"),
    ] {
        let hover = server.at("textDocument/hover", uri, line, 2);
        let text = hover["contents"]["value"].as_str().unwrap_or_else(|| panic!("{expected}: {hover}"));
        assert!(text.contains(expected), "{expected}: {text}");
    }

    let substitution = server.at("textDocument/hover", uri, 8, 10);
    assert!(substitution["contents"]["value"].as_str().is_some_and(|text| text.contains(";#name")), "{substitution}");
}

#[test]
fn server_advertises_preprocessor_completion_triggers() {
    let (_server, capabilities) = Server::ready();
    let triggers = capabilities["completionProvider"]["triggerCharacters"].as_array().unwrap();
    assert!(triggers.contains(&json!("$")), "{triggers:?}");
    assert!(triggers.contains(&json!("#")), "{triggers:?}");
}

#[test]
fn server_returns_directives_after_dollar_marker() {
    let uri = "file:///tmp/preprocessor-completion.pps";
    let (mut server, _) = Server::ready();
    server.opened(uri, ";$");

    let result = server.at("textDocument/completion", uri, 0, 2);
    let items = result.as_array().unwrap_or_else(|| panic!("{result}"));
    let define = items.iter().find(|item| item["label"] == "DEFINE").unwrap_or_else(|| panic!("{items:?}"));
    assert_eq!(define["filterText"], ";$DEFINE");
    assert_eq!(define["insertText"], "DEFINE");
}