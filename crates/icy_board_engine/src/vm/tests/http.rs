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
request.SetHeader("X-Secret", "do-not-forward")
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
request.SetHeader("X-Token", "abc")
request.SetText("hello", "text/plain")
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
fn a_request_can_send_bytes_with_default_and_explicit_content_types() {
    let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let (origin, requests, server) = serve(vec![response.to_string(), response.to_string()]);
    let source = format!(
        r#"
BYTES data = Bytes.FromBase64("AH8=")
HttpRequest first = Http.New(HttpMethod.Post, "{origin}/default")
BOOLEAN defaultSet = first.SetBytes(data)
first.Send()
HttpRequest second = Http.New(HttpMethod.Put, "{origin}/explicit")
BOOLEAN explicitSet = second.SetBytes(data, "image/x-test")
HttpResponse response = second.Send()
PRINT defaultSet, " ", explicitSet, " ", response.Status
"#
    );
    assert_eq!("1 1 204", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    let sent = requests.recv().unwrap();
    assert!(sent.to_ascii_lowercase().contains("content-type: application/octet-stream"), "{sent:?}");
    assert!(sent.ends_with("\r\n\r\n\0\u{7f}"), "{sent:?}");
    let sent = requests.recv().unwrap();
    assert!(sent.to_ascii_lowercase().contains("content-type: image/x-test"), "{sent:?}");
    assert!(sent.ends_with("\r\n\r\n\0\u{7f}"), "{sent:?}");
    server.join().unwrap();
}

#[test]
fn set_bytes_is_rejected_on_bodyless_methods() {
    assert_eq!(
        "0 1 1",
        run_ppl(
            r#"
HttpRequest request = Http.New(HttpMethod.Head, "https://example.com")
BOOLEAN changed = request.SetBytes(Bytes.FromBase64("AA=="))
PRINT changed, " ", Error.Last().Kind = ErrKind.Net, " ", Error.Last().Code = ErrCode.Invalid
"#
        )
    );
}

#[test]
fn request_mutation_is_visible_through_an_alias() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let (origin, requests, server) = serve(vec![response.to_string()]);
    let source = format!(
        r#"
HttpRequest request = Http.New(HttpMethod.Get, "{origin}/items")
HttpRequest copy = request
BOOLEAN changed = request.SetHeader("X-Alias", "yes")
HttpResponse response = copy.Send()
PRINT changed, " ", response.OK
"#
    );
    assert_eq!("1 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    let request = requests.recv().unwrap().to_ascii_lowercase();
    assert!(request.contains("x-alias: yes"), "{request}");
    server.join().unwrap();
}

#[test]
fn restricted_request_headers_are_rejected() {
    assert_eq!(
        "0 1 1",
        run_ppl(
            "HttpRequest request = Http.New(HttpMethod.Get, \"https://example.com\")\nBOOLEAN changed = request.SetHeader(\"Host\", \"internal\")\nPRINT changed, \" \", Error.Last().Kind = ErrKind.Net, \" \", Error.Last().Code = ErrCode.Invalid"
        )
    );
}

#[test]
fn set_text_is_rejected_on_bodyless_methods() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let (origin, request, server) = serve(vec![response.to_string()]);
    let source = format!(
        r#"
HttpRequest req = Http.New(HttpMethod.Get, "{origin}/items")
BOOLEAN changed = req.SetText("body", "text/plain")
ERRCODE code = Error.Last().Code
HttpResponse response = req.Send()
PRINT changed, " ", code = ErrCode.Invalid, " ", response.OK
"#
    );
    assert_eq!("0 1 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    let sent = request.recv().unwrap();
    assert!(sent.starts_with("GET /items HTTP/1.1"), "{sent}");
    assert!(!sent.to_ascii_lowercase().contains("content-type"), "{sent}");
    assert!(sent.ends_with("\r\n\r\n"), "{sent}");
    server.join().unwrap();
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
fn a_download_response_cannot_save_or_decode_an_unretained_body() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata";
    let (origin, _request, server) = serve(vec![response.to_string()]);
    let source = format!(
        "HttpResponse response = Http.Download(\"{origin}/file\", \"download.bin\")\nSTRING text = response.Text()\nERRCODE textError = Error.Last().Code\nError.Clear()\nBOOLEAN saved = response.Save(\"copy.bin\")\nPRINT response.Size, \" \", text.Len(), \" \", textError = ErrCode.Invalid, \" \", saved, \" \", EXIST(\"copy.bin\")"
    );
    assert_eq!("4 0 1 0 0", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    server.join().unwrap();
}

#[test]
fn an_empty_retained_response_body_can_be_decoded_and_saved() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let (origin, _request, server) = serve(vec![response.to_string()]);
    let source = format!(
        "HttpResponse response = Http.Get(\"{origin}/empty\")\nSTRING text = response.Text()\nBOOLEAN saved = response.Save(\"empty.bin\")\nPRINT text.Len(), \" \", saved, \" \", EXIST(\"empty.bin\"), \" \", Error.Last().OK"
    );
    assert_eq!("0 1 1 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
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

#[test]
fn url_encoding_defaults_to_the_form_dialect_and_round_trips() {
    assert_eq!(
        "[a+b%26c%3Dd] [a%20b%26c%3Dd] [a b&c=d] [a b&c=d] [%2B] [+]",
        run_ppl(
            r#"
STRING raw = "a b&c=d"
PRINT "[", Http.UrlEncode(raw), "] [", Http.UrlEncode(raw, FALSE), "]"
PRINT " [", Http.UrlDecode(Http.UrlEncode(raw)), "] [", Http.UrlDecode(Http.UrlEncode(raw, FALSE), FALSE), "]"
PRINT " [", Http.UrlEncode("+"), "] [", Http.UrlDecode("%2B"), "]"
"#
        )
    );
}

#[test]
fn url_encoding_keeps_unreserved_characters_and_encodes_utf8_per_byte() {
    assert_eq!(
        "[-_.%7EAZaz09] [-_.~AZaz09] [%C3%A4] [\u{e4}]",
        run_ppl(
            r#"STRING raw = "-_.~AZaz09"
PRINT "[", Http.UrlEncode(raw), "] [", Http.UrlEncode(raw, FALSE), "] [", Http.UrlEncode("ä"), "] [", Http.UrlDecode("%C3%A4"), "]""#
        )
    );
}

#[test]
fn set_form_accumulates_encoded_pairs_and_sets_the_content_type() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
    let (origin, request, server) = serve(vec![response.to_string()]);
    let source = format!(
        r#"
HttpRequest request = Http.New(HttpMethod.Post, "{origin}/messages")
BOOLEAN added = request.SetForm("token", "secret")
added = added & request.SetForm("title", "SYSOP wants to chat")
added = added & request.SetForm("message", "a&b=c")
HttpResponse response = request.Send()
PRINT added, " ", response.OK
"#
    );
    assert_eq!("1 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    let sent = request.recv().unwrap();
    assert!(sent.to_ascii_lowercase().contains("content-type: application/x-www-form-urlencoded"), "{sent}");
    assert!(sent.ends_with("\r\n\r\ntoken=secret&title=SYSOP+wants+to+chat&message=a%26b%3Dc"), "{sent}");
    server.join().unwrap();
}

#[test]
fn set_form_refuses_to_append_to_a_body_it_did_not_write() {
    assert_eq!(
        "1 0 1",
        run_ppl(
            r#"
HttpRequest request = Http.New(HttpMethod.Post, "https://example.com")
BOOLEAN text = request.SetText("{}", "application/json")
BOOLEAN form = request.SetForm("token", "secret")
PRINT text, " ", form, " ", Error.Last().Code = ErrCode.Invalid
"#
        )
    );
}

#[test]
fn set_form_matches_the_complete_media_type_case_insensitively() {
    assert_eq!(
        "1 1 1 0 1",
        run_ppl(
            r#"
HttpRequest valid = Http.New(HttpMethod.Post, "https://example.com")
BOOLEAN validText = valid.SetText("first=one", "Application/X-Www-Form-Urlencoded; charset=UTF-8")
BOOLEAN validForm = valid.SetForm("second", "two")
HttpRequest invalid = Http.New(HttpMethod.Post, "https://example.com")
BOOLEAN invalidText = invalid.SetText("first=one", "application/x-www-form-urlencoded-evil")
BOOLEAN invalidForm = invalid.SetForm("second", "two")
PRINT validText, " ", validForm, " ", invalidText, " ", invalidForm, " ", Error.Last().Code = ErrCode.Invalid
"#
        )
    );
}

#[test]
fn set_form_is_rejected_on_bodyless_methods() {
    assert_eq!(
        "0 1 1",
        run_ppl(
            r#"
HttpRequest request = Http.New(HttpMethod.Get, "https://example.com")
BOOLEAN added = request.SetForm("token", "secret")
PRINT added, " ", Error.Last().Kind = ErrKind.Net, " ", Error.Last().Code = ErrCode.Invalid
"#
        )
    );
}

#[test]
fn put_delete_and_patch_reach_the_wire_and_keep_their_bodies() {
    let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let (origin, requests, server) = serve(vec![response.to_string(), response.to_string(), response.to_string()]);
    let source = format!(
        r#"
HttpRequest put = Http.New(HttpMethod.Put, "{origin}/item")
put.SetText("replaced", "text/plain")
put.Send()
HttpRequest patch = Http.New(HttpMethod.Patch, "{origin}/item")
patch.SetForm("field", "new value")
patch.Send()
HttpRequest remove = Http.New(HttpMethod.Delete, "{origin}/item")
HttpResponse response = remove.Send()
PRINT response.Status, " ", put.Method = HttpMethod.Put, " ", remove.Method = HttpMethod.Delete
"#
    );
    assert_eq!("204 1 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    let sent = requests.recv().unwrap();
    assert!(sent.starts_with("PUT /item HTTP/1.1"), "{sent}");
    assert!(sent.ends_with("\r\n\r\nreplaced"), "{sent}");
    let sent = requests.recv().unwrap();
    assert!(sent.starts_with("PATCH /item HTTP/1.1"), "{sent}");
    assert!(sent.ends_with("\r\n\r\nfield=new+value"), "{sent}");
    let sent = requests.recv().unwrap();
    assert!(sent.starts_with("DELETE /item HTTP/1.1"), "{sent}");
    server.join().unwrap();
}

#[test]
fn a_binary_response_body_is_reachable_as_bytes() {
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nPK\u{3}\u{4}";
    let (origin, _request, server) = serve(vec![response.to_string()]);
    let source = format!(
        r#"
HttpResponse response = Http.Get("{origin}/archive")
BYTES raw = response.Bytes()
PRINT raw.Len(), " ", raw.ToHex(), " ", raw.ToBase64(), " ", Error.Last().OK
"#
    );
    assert_eq!("4 504B0304 UEsDBA== 1", run_ppl_on(&source, |board| allow_origin(board, &origin)));
    server.join().unwrap();
}

#[test]
fn bytes_reports_a_body_that_was_never_retained() {
    assert_eq!(
        "0 1",
        run_ppl(
            r#"
HttpResponse response = Http.Get("file:///not-http")
Error.Clear()
BYTES raw = response.Bytes()
PRINT raw.Len(), " ", Error.Last().Code = ErrCode.Invalid
"#
        )
    );
}
