//! The server has to stay standing when an editor asks something odd.

mod common;

use common::Server;
use serde_json::{Value, json};

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
