use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
};

use crate::icy_board::icb_config::PplHttpDestinationPolicy;

use super::{compile_errors, run_ppl, run_ppl_on};

fn serve(responses: Vec<String>) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..read]);
                let headers_end = bytes.windows(4).position(|window| window == b"\r\n\r\n").map(|position| position + 4);
                let content_length = String::from_utf8_lossy(&bytes)
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: ").or_else(|| line.strip_prefix("Content-Length: ")))
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if headers_end.is_some_and(|end| bytes.len() >= end + content_length) {
                    break;
                }
            }
            sender.send(String::from_utf8_lossy(&bytes).into_owned()).unwrap();
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    (format!("http://{address}"), receiver, handle)
}

fn allow_origin(board: &mut crate::icy_board::IcyBoard, origin: &str) {
    board.config.ppl_http.destination_policy = PplHttpDestinationPolicy::Allowlist;
    board.config.ppl_http.allowed_origins = vec![origin.to_string()];
    board.config.ppl_http.allow_http = true;
}

#[test]
fn default_http_policy_rejects_non_http_urls() {
    assert_eq!(
        "0 1 1",
        run_ppl(
            "HttpResponse response = Http.Get(\"file:///etc/passwd\")\nPRINT response.Valid, \" \", Error.Last().Kind = ErrKind.Net, \" \", Error.Last().Code = ErrCode.Unsupported"
        )
    );
}

#[test]
fn webrequest_is_not_part_of_ppl_400() {
    assert!(!compile_errors("PRINT WebRequest(\"https://example.com\")").is_empty());
    assert!(!compile_errors("WebRequest \"https://example.com\", \"out.txt\"").is_empty());
}

#[test]
fn an_allowlisted_non_success_response_keeps_its_http_details() {
    let response = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nX-Test: yes\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmissing";
    let (origin, _request, server) = serve(vec![response.to_string()]);
    let source = format!(
        "HttpResponse response = Http.Get(\"{origin}/item\")\nPRINT response.Valid, \" \", response.OK, \" \", response.Status, \" [\", response.Text(), \"] [\", response.Header(\"x-test\"), \"] \", Error.Last().OK"
    );
    assert_eq!("1 0 404 [missing] [yes] 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    server.join().unwrap();
}

#[test]
fn public_policy_rejects_loopback_before_connecting() {
    let output = run_ppl_on(
        "HttpResponse response = Http.Get(\"http://127.0.0.1:9/\")\nPRINT response.Valid, \" \", Error.Last().Code = ErrCode.Denied",
        |board| {
            board.config.ppl_http.destination_policy = PplHttpDestinationPolicy::Public;
            board.config.ppl_http.allow_http = true;
        },
    );
    assert_eq!("0 1", output);
}

#[test]
fn redirects_are_checked_against_the_allowlist() {
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    let target_url = format!("http://{}", target.local_addr().unwrap());
    drop(target);
    let redirect = format!("HTTP/1.1 302 Found\r\nLocation: {target_url}/private\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let (origin, _request, server) = serve(vec![redirect]);
    let source = format!("HttpResponse response = Http.Get(\"{origin}/redirect\")\nPRINT response.Valid, \" \", Error.Last().Code = ErrCode.Denied");
    assert_eq!("0 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    server.join().unwrap();
}

#[test]
fn cross_origin_redirects_drop_script_headers() {
    let target_response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
    let (target_origin, target_request, target_server) = serve(vec![target_response.to_string()]);
    let redirect = format!("HTTP/1.1 302 Found\r\nLocation: {target_origin}/final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let (origin, _request, redirect_server) = serve(vec![redirect]);
    let source = format!(
        r#"
HttpRequest request = Http.New(HttpMethod.Get, "{origin}/redirect")
request = request.SetHeader("X-Secret", "do-not-forward")
HttpResponse response = request.Send()
PRINT response.OK
"#
    );
    assert_eq!(
        "1",
        run_ppl_on(&source, |board| {
            allow_origin(board, &origin);
            board.config.ppl_http.allowed_origins.push(target_origin.clone());
        })
    );
    let target_request = target_request.recv().unwrap().to_ascii_lowercase();
    assert!(!target_request.contains("x-secret"), "{target_request}");
    redirect_server.join().unwrap();
    target_server.join().unwrap();
}

#[test]
fn response_bodies_are_bounded_while_streaming() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n12345";
    let (origin, _request, server) = serve(vec![response.to_string()]);
    let source = format!("HttpResponse response = Http.Get(\"{origin}/large\")\nPRINT response.Valid, \" \", Error.Last().Code = ErrCode.Limit");
    assert_eq!(
        "0 1",
        run_ppl_on(&source, |board| {
            allow_origin(board, &origin);
            board.config.ppl_http.max_response_bytes = 4;
        })
    );
    server.join().unwrap();
}

#[test]
fn head_ignores_the_get_body_size_advertised_by_content_length() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\nConnection: close\r\n\r\n";
    let (origin, request, server) = serve(vec![response.to_string()]);
    let source = format!(
        "HttpRequest request = Http.New(HttpMethod.Head, \"{origin}/large\")\nHttpResponse response = request.Send()\nPRINT response.Valid, \" \", response.Status, \" \", response.Size"
    );
    assert_eq!(
        "1 200 0",
        run_ppl_on(&source, |board| {
            allow_origin(board, &origin);
            board.config.ppl_http.max_response_bytes = 4;
        })
    );
    assert!(request.recv().unwrap().starts_with("HEAD /large HTTP/1.1"));
    server.join().unwrap();
}

#[test]
fn a_request_can_send_text_and_safe_headers() {
    let response = "HTTP/1.1 201 Created\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
    let (origin, request, server) = serve(vec![response.to_string()]);
    let source = format!(
        r#"
HttpRequest request = Http.New(HttpMethod.Post, "{origin}/items")
request = request.SetHeader("X-Token", "abc")
request = request.SetText("hello", "text/plain")
HttpResponse response = request.Send()
PRINT response.Status, " ", response.Text()
"#
    );
    assert_eq!("201 ok", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    let request = request.recv().unwrap();
    assert!(request.starts_with("POST /items HTTP/1.1"), "{request}");
    assert!(request.to_ascii_lowercase().contains("x-token: abc"), "{request}");
    assert!(request.ends_with("\r\n\r\nhello"), "{request}");
    server.join().unwrap();
}

#[test]
fn request_builders_do_not_mutate_the_original_value() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let (origin, requests, server) = serve(vec![response.to_string(), response.to_string()]);
    let source = format!(
        r#"
HttpRequest original = Http.New(HttpMethod.Get, "{origin}/items")
HttpRequest copy = original.SetHeader("X-Copy", "yes")
HttpResponse first = original.Send()
HttpResponse second = copy.Send()
PRINT first.OK, " ", second.OK
"#
    );
    assert_eq!("1 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    let original = requests.recv().unwrap().to_ascii_lowercase();
    let copy = requests.recv().unwrap().to_ascii_lowercase();
    assert!(!original.contains("x-copy"), "{original}");
    assert!(copy.contains("x-copy: yes"), "{copy}");
    server.join().unwrap();
}

#[test]
fn restricted_request_headers_are_rejected() {
    assert_eq!(
        "1",
        run_ppl(
            "HttpRequest request = Http.New(HttpMethod.Get, \"https://example.com\")\nrequest = request.SetHeader(\"Host\", \"internal\")\nPRINT Error.Last().Code = ErrCode.Invalid"
        )
    );
}

#[test]
fn an_invalid_response_cannot_create_a_file() {
    assert_eq!(
        "0 0 1",
        run_ppl(
            "HttpResponse response = Http.Get(\"file:///not-http\")\nError.Clear()\nPRINT response.Save(\"invalid.bin\"), \" \", EXIST(\"invalid.bin\"), \" \", Error.Last().Code = ErrCode.Invalid"
        )
    );
}

#[test]
fn downloads_are_committed_only_after_success() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata";
    let (origin, _request, server) = serve(vec![response.to_string()]);
    let source = format!(
        "HttpResponse response = Http.Download(\"{origin}/file\", \"download.bin\")\nPRINT response.OK, \" \", response.Size, \" \", EXIST(\"download.bin\")"
    );
    assert_eq!("1 4 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    server.join().unwrap();
}

#[test]
fn a_download_to_an_absolute_temp_path_lands_where_it_was_asked_to() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata";
    let (origin, _request, server) = serve(vec![response.to_string()]);
    let source = format!(
        "STRING dest = TempPath() + \"file.bin\"\nHttpResponse response = Http.Download(\"{origin}/file\", dest)\nPRINT response.OK, \" \", EXIST(dest)"
    );
    assert_eq!("1 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    server.join().unwrap();
}

#[test]
fn a_failed_download_preserves_the_existing_file() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nlarge";
    let (origin, _request, server) = serve(vec![response.to_string()]);
    let source = format!(
        "FCREATE 1, \"download.bin\", O_WR, S_DN\nFPUT 1, \"old\"\nFCLOSE 1\nHttpResponse response = Http.Download(\"{origin}/file\", \"download.bin\")\nERROR failure = Error.Last()\nSTRING value\nFOPEN 1, \"download.bin\", O_RD, S_DN\nFGET 1, value\nFCLOSE 1\nPRINT response.Valid, \" [\", value, \"] \", failure.Code = ErrCode.Limit"
    );
    assert_eq!(
        "0 [old] 1",
        run_ppl_on(&source, |board| {
            allow_origin(board, &origin);
            board.config.ppl_http.max_response_bytes = 4;
        })
    );
    server.join().unwrap();
}
