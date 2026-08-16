use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use serde_json::{Value, json};

struct Server {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Server {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_icyboard-ppl"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self { child, input, output }
    }

    fn send(&mut self, message: Value) {
        let body = message.to_string();
        write!(self.input, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.input.flush().unwrap();
    }

    fn response(&mut self, id: i64) -> Value {
        loop {
            let message = self.message();
            if message.get("id").and_then(Value::as_i64) == Some(id) {
                return message;
            }
        }
    }

    fn notification(&mut self, method: &str) -> Value {
        loop {
            let message = self.message();
            if message.get("method").and_then(Value::as_str) == Some(method) {
                return message;
            }
        }
    }

    fn message(&mut self) -> Value {
        let mut content_length = None;
        loop {
            let mut header = String::new();
            self.output.read_line(&mut header).unwrap();
            assert!(!header.is_empty(), "language server stopped before answering");
            if header == "\r\n" {
                break;
            }
            if let Some(value) = header.strip_prefix("Content-Length:") {
                content_length = Some(value.trim().parse::<usize>().unwrap());
            }
        }
        let mut body = vec![0; content_length.expect("response has no Content-Length")];
        self.output.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
