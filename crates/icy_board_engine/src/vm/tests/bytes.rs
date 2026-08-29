use super::run_ppl;

#[test]
fn base64_round_trips_through_a_byte_blob() {
    let output = run_ppl(
        r#";$LANGVERSION 400
STRING text = "Hello, world!"
BYTES raw = ToBytes(text)
STRING enc = Base64Enc(raw)
BYTES dec = Base64Dec(enc)
PRINTLN enc
PRINTLN FromBytes(dec)
PRINTLN LEN(raw)
"#,
    );

    assert_eq!(output, "SGVsbG8sIHdvcmxkIQ==\nHello, world!\n13\n");
}

#[test]
fn a_string_argument_is_taken_as_its_utf8_bytes() {
    // Base64Enc/Sha256 declare a BYTES parameter; a STRING coerces to its UTF-8 bytes.
    let output = run_ppl(
        r#";$LANGVERSION 400
PRINTLN Base64Enc("abc")
PRINTLN Sha256("abc")
"#,
    );

    assert_eq!(output, "YWJj\nba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n");
}

#[test]
fn bytes_print_as_uppercase_hex() {
    let output = run_ppl(
        r#";$LANGVERSION 400
BYTES raw = ToBytes("abc")
PRINTLN raw
"#,
    );

    assert_eq!(output, "616263\n");
}

#[test]
fn decoding_bytes_that_are_not_utf8_reports_a_format_error() {
    // "/w==" decodes to the single byte 0xFF, which is not valid UTF-8.
    let output = run_ppl(
        r#";$LANGVERSION 400
BYTES raw = Base64Dec("/w==")
STRING text = FromBytes(raw)
PRINTLN "[", text, "] ", Error.Last().Kind = ErrKind.String, " ", Error.Last().Code = ErrCode.Format
"#,
    );

    assert_eq!(output, "[] 1 1\n");
}

#[test]
fn malformed_base64_reports_a_format_error() {
    let output = run_ppl(
        r#";$LANGVERSION 400
BYTES raw = Base64Dec("!!!!")
PRINTLN LEN(raw), " ", Error.Last().Code = ErrCode.Format
"#,
    );

    assert_eq!(output, "0 1\n");
}
