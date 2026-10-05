mod common;

use common::Server;
use serde_json::json;

#[test]
fn object_call_arguments_get_parameter_name_hints() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/inlay-hints.pps";
    server.opened(uri, ";$LANGVERSION 400\nSURFACE image = Surface.New(80, 25)\n");

    let hints = server.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": {"uri": uri},
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 2, "character": 0}}
        }),
    );
    assert_eq!(hints[0]["label"], "width:", "{hints}");
    assert_eq!(hints[1]["label"], "height:", "{hints}");
}

#[test]
fn routines_show_reference_count_code_lenses() {
    let (mut server, _) = Server::ready();
    let uri = "file:///tmp/code-lenses.pps";
    server.opened(uri, "DECLARE PROCEDURE Show()\nShow()\nPROCEDURE Show()\nENDPROC\n");

    let lenses = server.request("textDocument/codeLens", json!({"textDocument": {"uri": uri}}));
    let titles: Vec<_> = lenses
        .as_array()
        .unwrap()
        .iter()
        .map(|lens| lens["command"]["title"].as_str().unwrap())
        .collect();
    assert!(titles.iter().any(|title| *title == "1 reference"), "{lenses}");
}

#[test]
fn archive_hover_is_localized_in_separate_servers() {
    for (locale, expected) in [("en_US.UTF-8", "Skips unread current data"), ("de_DE.UTF-8", "Überspringt ungelesene Daten")] {
        let mut server = Server::ready_in_locale(locale);
        let uri = "file:///tmp/archive-hints.pps";
        server.opened(uri, ";$LANGVERSION 400\nARCHIVEREADER reader = Archive.Open(\"data.zip\")\nreader.Next()\n");
        let hover = server.request(
            "textDocument/hover",
            json!({"textDocument": {"uri": uri}, "position": {"line": 2, "character": 8}}),
        );
        let text = hover["contents"]["value"].as_str().unwrap_or_else(|| panic!("{locale}: {hover}"));
        assert!(text.contains(expected), "{locale}: {text}");
        assert!(text.contains("Error.Last()"), "{locale}: {text}");
    }
}
