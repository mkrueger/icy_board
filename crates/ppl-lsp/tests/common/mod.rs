#![allow(dead_code)]

use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{Receiver, RecvTimeoutError, channel},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

/// How long to wait for an answer the server owes us.
const ANSWER: Duration = Duration::from_secs(20);
/// How long the server has to stay silent before it is taken to be finished.
const QUIET: Duration = Duration::from_millis(400);

/// Drives the real language server binary over stdio, the way an editor does.
///
/// Messages are read on their own thread so a test can wait with a deadline
/// rather than block for good on a server that has nothing more to say.
pub struct Server {
    child: Child,
    input: ChildStdin,
    messages: Receiver<Value>,
    pending: VecDeque<Value>,
    next_id: i64,
}

impl Server {
    pub fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_icyboard-ppl"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());

        let (sender, messages) = channel();
        thread::spawn(move || {
            while let Some(message) = read_message(&mut output) {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });

        Self {
            child,
            input,
            messages,
            pending: VecDeque::new(),
            next_id: 1,
        }
    }

    /// Starts a server that has finished the handshake, and answers with what it
    /// said it can do.
    pub fn ready() -> (Self, Value) {
        Self::ready_with_root(None)
    }

    pub fn ready_at(root_uri: &str) -> (Self, Value) {
        Self::ready_with_root(Some(root_uri))
    }

    fn ready_with_root(root_uri: Option<&str>) -> (Self, Value) {
        let mut server = Self::start();
        let capabilities = server.request("initialize", json!({"processId": null, "rootUri": root_uri, "capabilities": {}}))["capabilities"].clone();
        server.send(json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
        (server, capabilities)
    }

    pub fn open(&mut self, uri: &str, text: &str) {
        self.send(json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "languageId": "ppl", "version": 1, "text": text}}
        }));
    }

    /// Opens a document and waits until the server has finished looking at it, so
    /// that a question about it is not asked before it is known.
    pub fn opened(&mut self, uri: &str, text: &str) {
        self.open(uri, text);
        self.diagnostics(uri);
    }

    pub fn close(&mut self, uri: &str) {
        self.send(json!({"jsonrpc": "2.0", "method": "textDocument/didClose", "params": {"textDocument": {"uri": uri}}}));
    }

    /// What the server ends up underlining in a document.
    ///
    /// It takes the old diagnostics back before it reports what it found, and it
    /// does so while answering other requests, so the answer is the last thing it
    /// said once it has fallen silent.
    pub fn diagnostics(&mut self, uri: &str) -> Value {
        let deadline = Instant::now() + ANSWER;
        let mut latest = json!([]);
        let mut others = Vec::new();
        while Instant::now() < deadline {
            let Some(message) = self.take(QUIET) else {
                break;
            };
            if message.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics") && message["params"]["uri"] == uri {
                latest = message["params"]["diagnostics"].clone();
            } else {
                others.push(message);
            }
        }
        for message in others.into_iter().rev() {
            self.pending.push_front(message);
        }
        latest
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let message = self.response(id);
        assert!(message.get("error").is_none(), "{method} answered with an error: {message}");
        message["result"].clone()
    }

    /// Asks about the given zero-based position in a document.
    pub fn at(&mut self, method: &str, uri: &str, line: u32, character: u32) -> Value {
        self.request(
            method,
            json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}),
        )
    }

    pub fn send(&mut self, message: Value) {
        let body = message.to_string();
        write!(self.input, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.input.flush().unwrap();
    }

    pub fn response(&mut self, id: i64) -> Value {
        self.wait_for(|message| message.get("id").and_then(Value::as_i64) == Some(id), &format!("a reply to {id}"))
    }

    pub fn notification(&mut self, method: &str) -> Value {
        self.wait_for(|message| message.get("method").and_then(Value::as_str) == Some(method), method)
    }

    fn wait_for(&mut self, matches: impl Fn(&Value) -> bool, what: &str) -> Value {
        let deadline = Instant::now() + ANSWER;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                panic!("the language server never sent {what}");
            }
            let Some(message) = self.take(left) else {
                panic!("the language server never sent {what}");
            };
            if matches(&message) {
                return message;
            }
        }
    }

    fn take(&mut self, timeout: Duration) -> Option<Value> {
        if let Some(message) = self.pending.pop_front() {
            return Some(message);
        }
        match self.messages.recv_timeout(timeout) {
            Ok(message) => Some(message),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => None,
        }
    }
}

fn read_message(output: &mut impl BufRead) -> Option<Value> {
    let mut content_length = None;
    loop {
        let mut header = String::new();
        if output.read_line(&mut header).ok()? == 0 {
            return None;
        }
        if header == "\r\n" {
            break;
        }
        if let Some(value) = header.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; content_length?];
    output.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
