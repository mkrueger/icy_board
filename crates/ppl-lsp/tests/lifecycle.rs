//! The server has to stay standing when an editor asks something odd.

mod common;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::Server;
use serde_json::{Value, json};

/// The codes of a set of diagnostics, for readable failures.
fn codes(diagnostics: &Value) -> Vec<String> {
    diagnostics
        .as_array()
        .map(|list| list.iter().map(|entry| entry["code"].as_str().unwrap_or_default().to_string()).collect())
        .unwrap_or_default()
}

fn project(name: &str, manifest: &str, source: &str) -> (PathBuf, String, String) {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("ppl-lsp-{name}-{}-{unique}", std::process::id()));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("ppl.toml"), manifest).unwrap();
    fs::write(root.join("src/main.pps"), source).unwrap();
    let root_uri = format!("file://{}", root.display());
    let source_uri = format!("file://{}", root.join("src/main.pps").display());
    (root, root_uri, source_uri)
}

/// Typing is faster than reading a program, so only the last of a burst of edits
/// is worth answering, and only once the typing has paused.
#[test]
fn a_burst_of_edits_is_waited_out_and_answered_once() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/burst.pps";
    server.opened(uri, "PRINTLN \"hello\"\n");

    for (version, text) in [(2, "INTEGER A\n"), (3, "INTEGER Ab\n"), (4, "INTEGER Abc\n"), (5, "PRINTLN \"done\"\n")] {
        server.change(uri, version, text);
    }
    let sent = Instant::now();

    let report = server.notification("textDocument/publishDiagnostics");
    assert!(sent.elapsed() >= Duration::from_millis(100), "the server did not wait for the typing to stop");
    let diagnostics = report["params"]["diagnostics"].clone();
    assert_eq!(diagnostics.as_array().map(Vec::len), Some(0), "{:?}", codes(&diagnostics));

    let rest = server.diagnostic_reports(uri);
    assert!(rest.is_empty(), "every keystroke was answered: {rest:?}");
}

/// A manifest edited outside the editor still decides how the program is read.
#[test]
fn a_manifest_changed_on_disk_is_read_again() {
    let source = "TYPE Point\n INTEGER X\nENDTYPE\nPoint value\nvalue = Point { X = 1 }\nPRINTLN value.X\n";
    let manifest = |runtime: u16| format!("[package]\nname = \"watched\"\nversion = \"0.1.0\"\nruntime = {runtime}\n[compiler]\nlanguage_version = 400\n");
    let (root, root_uri, uri) = project("watched", &manifest(400), source);

    let (mut server, _) = Server::ready_at(&root_uri);
    server.open(&uri, source);
    let before = server.diagnostics(&uri);
    assert!(!codes(&before).contains(&"ppl.runtime-too-old".to_string()), "{:?}", codes(&before));

    fs::write(root.join("ppl.toml"), manifest(340)).unwrap();
    server.send(json!({
        "jsonrpc": "2.0", "method": "workspace/didChangeWatchedFiles",
        "params": {"changes": [{"uri": format!("file://{}", root.join("ppl.toml").display()), "type": 2}]}
    }));

    let after = server.diagnostics(&uri);
    assert!(codes(&after).contains(&"ppl.runtime-too-old".to_string()), "{:?}", codes(&after));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_server_offers_what_it_implements() {
    let (_server, capabilities) = Server::ready();

    for provider in [
        "hoverProvider",
        "completionProvider",
        "signatureHelpProvider",
        "definitionProvider",
        "referencesProvider",
        "renameProvider",
        "documentSymbolProvider",
        "documentHighlightProvider",
        "codeLensProvider",
        "inlayHintProvider",
        "codeActionProvider",
        "documentFormattingProvider",
        "documentRangeFormattingProvider",
        "semanticTokensProvider",
    ] {
        assert!(!capabilities[provider].is_null(), "{provider} is not offered: {capabilities}");
    }
}

#[test]
fn invalid_positions_and_closed_documents_do_not_stop_the_server() {
    let mut server = Server::start();
    server.send(json!({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":{"processId":null,"rootUri":null,"capabilities":{}}}));
    let initialized = server.response(1);
    assert!(initialized["result"]["capabilities"]["semanticTokensProvider"].is_object(), "{initialized}");
    server.send(json!({"jsonrpc":"2.0", "method":"initialized", "params":{}}));

    let uri = "file:///tmp/lifecycle.pps";
    server.send(json!({
        "jsonrpc":"2.0", "method":"textDocument/didOpen",
        "params":{"textDocument":{"uri":uri,"languageId":"ppl","version":1,"text":"PRINTLN \"😀\"\n"}}
    }));
    server.notification("textDocument/publishDiagnostics");
    server.send(json!({
        "jsonrpc":"2.0", "id":2, "method":"textDocument/semanticTokens/full",
        "params":{"textDocument":{"uri":uri}}
    }));
    let tokens = server.response(2);
    assert!(tokens["result"]["data"].as_array().is_some_and(|data| !data.is_empty()), "{tokens}");

    server.send(json!({
        "jsonrpc":"2.0", "id":3, "method":"textDocument/hover",
        "params":{"textDocument":{"uri":uri},"position":{"line":99,"character":99}}
    }));
    assert_eq!(server.response(3).get("result"), Some(&Value::Null));

    server.send(json!({"jsonrpc":"2.0", "method":"textDocument/didClose", "params":{"textDocument":{"uri":uri}}}));
    server.send(json!({
        "jsonrpc":"2.0", "id":4, "method":"textDocument/completion",
        "params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}
    }));
    assert_eq!(server.response(4).get("result"), Some(&Value::Null));

    server.send(json!({"jsonrpc":"2.0", "id":5, "method":"shutdown"}));
    let shutdown = server.response(5);
    assert!(shutdown.get("error").is_none(), "{shutdown}");
}
