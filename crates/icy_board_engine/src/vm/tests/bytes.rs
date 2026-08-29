use super::{compile_errors_with_runtime, run_ppl};

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
fn base64_is_available_as_bytes_instance_and_static_members() {
    let output = run_ppl(
        r#";$LANGVERSION 400
BYTES raw = ToBytes("abc")
STRING encoded = raw.ToBase64()
PRINTLN encoded
PRINTLN Bytes.FromBase64(encoded).ToString()
"#,
    );

    assert_eq!(output, "YWJj\nabc\n");
}

#[test]
fn bytes_get_checksum_returns_binary_digests() {
    let output = run_ppl(
        r#";$LANGVERSION 400
BYTES raw = ToBytes("123456789")
STRING crc = raw.GetChecksum(Checksum.CRC32).ToHex()
STRING md5 = raw.GetChecksum(Checksum.MD5).ToHex()
STRING sha256 = raw.GetChecksum(Checksum.SHA256).ToHex()
PRINTLN crc
PRINTLN md5
PRINTLN sha256
"#,
    );

    assert_eq!(
        output,
        "CBF43926\n25F9E794323B453885F5181F1B624D0B\n15E2B0D3C33891EBB0F1EF609EC419420C20E320CE94C65FBC8C3312448EB225\n"
    );
}

#[test]
fn tohex_preserves_leading_zero_bytes() {
    let output = run_ppl(
        r#";$LANGVERSION 400
BYTES raw = Bytes.FromBase64("AP8=")
PRINTLN raw.ToHex()
"#,
    );

    assert_eq!(output, "00FF\n");
}

#[test]
fn complex_values_cannot_be_converted_to_bytes() {
    let output = run_ppl(
        r#";$LANGVERSION 400
TYPE Item
INTEGER Value
ENDTYPE
Item item
BYTES raw = ToBytes(item)
PRINTLN LEN(raw), " ", Error.Last().Code = ErrCode.Invalid
"#,
    );

    assert_eq!(output, "0 1\n");
}

#[test]
fn base64_rejects_complex_values_instead_of_encoding_empty_data() {
    let output = run_ppl(
        r#";$LANGVERSION 400
TYPE Item
INTEGER Value
ENDTYPE
Item item
STRING encoded = Base64Enc(item)
PRINTLN "[", encoded, "] ", Error.Last().Code = ErrCode.Invalid
"#,
    );

    assert_eq!(output, "[] 1\n");
}

#[test]
fn tobytes_uses_little_endian_scalar_layouts() {
    let output = run_ppl(
        r#";$LANGVERSION 400
INTEGER integerValue = 1
DOUBLE doubleValue = 1.0
PRINTLN ToBytes(integerValue)
PRINTLN ToBytes(doubleValue)
"#,
    );

    assert_eq!(output, "01000000\n000000000000F03F\n");
}

#[test]
fn global_sha256_is_not_part_of_the_language() {
    let errors = compile_errors_with_runtime("PRINTLN Sha256(ToBytes(\"abc\"))", 400);
    assert!(!errors.is_empty());
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
