use super::{compile_errors, compile_errors_with_runtime, run_ppl, run_ppl_with_files};

#[test]
fn record_io_requires_records_and_runtime_400() {
    for statement in ["FGETREC 1, value", "FPUTREC 1, value", "FREADREC 1, value", "FWRITEREC 1, value"] {
        let errors = compile_errors(&format!("INTEGER value\n{statement}"));
        assert!(
            errors.iter().any(|error| error.contains("expects user-defined record")),
            "{statement}: {errors:?}"
        );
    }

    let source = "TYPE Item\n INTEGER Value\nENDTYPE\nItem value\nFPUTREC 1, value";
    let errors = compile_errors_with_runtime(source, 340);
    assert!(errors.iter().any(|error| error.contains("FPutRec needs runtime 400")), "{errors:?}");
}

#[test]
fn a_line_record_round_trips_and_leaves_documentation_unread() {
    let output = run_ppl(
        r#"
        TYPE Profile
            STRING Name
            INTEGER Age
            BIGSTR Note
        ENDTYPE

        Profile source
        source.Name = "Alice\\Admin"
        source.Age = 42
        source.Note = "first" + Chr(10) + "second"

        FCREATE 1, "profile.txt", O_WR, S_DN
        FPUTREC 1, source
        FPUTLN 1, "This text documents the record."
        FCLOSE 1

        Profile target
        STRING documentation
        FOPEN 1, "profile.txt", O_RD, S_DN
        FGETREC 1, target
        FGET 1, documentation
        FCLOSE 1

        PRINTLN target = source
        PRINTLN "[", target.Name, "] [", target.Note, "]"
        PRINTLN documentation
        "#,
    );

    assert_eq!(output, "1\n[Alice\\\\Admin] [first\nsecond]\nThis text documents the record.\n");
}

#[test]
fn binary_records_round_trip_nested_records_arrays_and_multiple_frames() {
    let output = run_ppl(
        r#"
        TYPE Point
            INTEGER X, Y
        ENDTYPE
        TYPE Packet
            STRING Name
            Point Position
            INTEGER Values(1)
        ENDTYPE

        Packet first
        first.Name = "one"
        first.Position.X = 10
        first.Position.Y = 20
        first.Values(0) = 30
        first.Values(1) = 40

        Packet second = first
        second.Name = "two"
        second.Position.X = 50

        FCREATE 1, "packets.bin", O_WR, S_DN
        FWRITEREC 1, first
        FWRITEREC 1, second
        FCLOSE 1

        Packet actualFirst
        Packet actualSecond
        FOPEN 1, "packets.bin", O_RD, S_DN
        FREADREC 1, actualFirst
        FREADREC 1, actualSecond
        FCLOSE 1

        PRINTLN actualFirst = first
        PRINTLN actualSecond = second
        PRINTLN actualSecond.Name, " ", actualSecond.Position.X, " ", actualSecond.Values(1)
        "#,
    );

    assert_eq!(output, "1\n1\ntwo 50 40\n");
}

#[test]
fn a_malformed_line_record_leaves_the_destination_unchanged() {
    let output = run_ppl_with_files(
        r#"
        TYPE Pair
            INTEGER First, Second
        ENDTYPE
        Pair value = Pair { First = 10, Second = 20 }

        FOPEN 1, "broken.txt", O_RD, S_DN
        FGETREC 1, value
        PRINTLN value.First, " ", value.Second
        PRINTLN FERR(1), " ", Error.Last().Code = ErrCode.Format
        FCLOSE 1
        "#,
        &[("broken.txt", b"not-a-number\n20\n")],
    );

    assert_eq!(output, "10 20\n1 1\n");
}

#[test]
fn a_truncated_binary_record_leaves_the_destination_unchanged() {
    let output = run_ppl_with_files(
        r#"
        TYPE Pair
            INTEGER First, Second
        ENDTYPE
        Pair value = Pair { First = 10, Second = 20 }

        FOPEN 1, "broken.bin", O_RD, S_DN
        FREADREC 1, value
        PRINTLN value.First, " ", value.Second
        PRINTLN FERR(1), " ", Error.Last().Code = ErrCode.Format
        FCLOSE 1
        "#,
        &[("broken.bin", &[8, 0, 0, 0, 1, 2, 3, 4])],
    );

    assert_eq!(output, "10 20\n1 1\n");
}

#[test]
fn every_supported_scalar_type_round_trips_through_both_codecs() {
    let output = run_ppl(
        r#"
        TYPE Scalars
            BOOLEAN BoolValue
            UNSIGNED UnsignedValue
            DATE DateValue
            EDATE EDateValue
            INTEGER IntegerValue
            MONEY MoneyValue
            FLOAT FloatValue
            STRING StringValue
            TIME TimeValue
            BYTE ByteValue
            WORD WordValue
            SBYTE SByteValue
            SWORD SWordValue
            BIGSTR BigValue
            DOUBLE DoubleValue
            DDATE DDateValue
            LONG LongValue
            ULONG ULongValue
        ENDTYPE

        Scalars source
        source.BoolValue = TRUE
        source.UnsignedValue = 4000000000
        source.DateValue = Date()
        source.EDateValue = Date()
        source.IntegerValue = -123456
        source.MoneyValue = 123.45
        source.FloatValue = 1.25
        source.StringValue = "text"
        source.TimeValue = Time()
        source.ByteValue = 250
        source.WordValue = 60000
        source.SByteValue = -100
        source.SWordValue = -30000
        source.BigValue = "big" + Chr(0) + "value"
        source.DoubleValue = 1.23456789
        source.DDateValue = 20260828
        source.LongValue = ToLong("-5000000000")
        source.ULongValue = ToULong("10000000000")

        FCREATE 1, "scalars.txt", O_WR, S_DN
        FPUTREC 1, source
        FCLOSE 1
        Scalars fromText
        FOPEN 1, "scalars.txt", O_RD, S_DN
        FGETREC 1, fromText
        FCLOSE 1

        FCREATE 1, "scalars.bin", O_WR, S_DN
        FWRITEREC 1, source
        FCLOSE 1
        Scalars fromBinary
        FOPEN 1, "scalars.bin", O_RD, S_DN
        FREADREC 1, fromBinary
        FCLOSE 1

        PRINTLN fromText = source
        PRINTLN fromBinary = source
        "#,
    );

    assert_eq!(output, "1\n1\n");
}

#[test]
fn a_record_format_failure_enters_on_error() {
    let output = run_ppl_with_files(
        r#"
        TYPE Item
            INTEGER Value
        ENDTYPE
        Item item
        FOPEN 1, "broken.txt", O_RD, S_DN
        ON ERROR GOTO Failed
        FGETREC 1, item
        PRINTLN "not handled"
        EXIT
        :Failed
        PRINTLN Error.Last().Kind = ErrKind.File, " ", Error.Last().Code = ErrCode.Format
        "#,
        &[("broken.txt", b"wrong\n")],
    );

    assert_eq!(output, "1 1\n");
}
