use super::{compile, run_executable_collecting, run_ppl};
use crate::{
    executable::VariableType,
    icy_board::state::{ppl_error::PplError, ppl_events::PplEvent},
    parser::{ERROR_ID, EVENT_ID, EVENT_KIND_ENUM_ID},
};

#[test]
fn s4_open_enums_future_host_values_reach_fallbacks_without_catalog_changes() {
    let language = 400;
    for future_kind in [99, -99, i32::MAX] {
        let mut executable = compile(&format!(
            r#";$LANGVERSION {language}
EVENT incoming
ERROR problem
TYPE Envelope
 EventKind Kind
 EventKind Kinds(1)
 ErrKind ErrorKind
 ErrCode ErrorCode
ENDTYPE
EventKind kind = incoming.Kind
Envelope saved
saved.Kind = Echo(kind)
saved.Kinds(1) = saved.Kind
saved.ErrorKind = problem.Kind
saved.ErrorCode = problem.Code
PRINT incoming.Kind = kind, "|", saved.Kind = saved.Kinds(1), "|"
SELECT CASE saved.Kind
 CASE EventKind.Key
  PRINT "key"
 CASE ELSE
  PRINT "future:", TOINTEGER(saved.Kind)
ENDSELECT
PRINT "|", saved.ErrorKind, ":", saved.ErrorCode, "|"
SELECT CASE saved.ErrorCode
 CASE ErrCode.Ok
  PRINT "ok"
 CASE ELSE
  PRINT "fallback"
ENDSELECT
FUNCTION Echo(EventKind value) EventKind
 EventKind local = value
 RETURN local
ENDFUNC
"#
        ));
        assert!(!executable.variable_table.enums[&EVENT_KIND_ENUM_ID].contains(&future_kind));
        for index in 1..=executable.variable_table.len() {
            let entry = executable.variable_table.get_var_entry_mut(index);
            if entry.header.variable_type == VariableType::UserData(EVENT_ID as u32) {
                entry.value = PplEvent {
                    event_type: future_kind,
                    ..Default::default()
                }
                .value();
            } else if entry.header.variable_type == VariableType::UserData(ERROR_ID as u32) {
                entry.value = PplError::new(91, 92, "future error").value();
            }
        }
        assert_eq!(
            format!("1|1|future:{future_kind}|91:92|fallback"),
            run_executable_collecting(executable, |_| {}, &[], None, &[], false, false).1,
            "language {language}"
        );
    }
}

#[test]
fn s4_open_enums_host_operations_reject_unsupported_values() {
    for (body, kind, code) in [
        ("PRINT Regex.Compile(\"x\", RegexOptions(64)).Valid", "Regex", "Invalid"),
        (
            "HttpRequest request = Http.New(HttpMethod(99), \"https://example.com\")\nPRINT 0",
            "Net",
            "Unsupported",
        ),
        ("PRINT \"a\".Equals(\"a\", StringComparison(99))", "String", "Invalid"),
        ("BYTES raw = ToBytes(\"abc\")\nPRINT LEN(raw.GetChecksum(Checksum(99)))", "String", "Invalid"),
        ("PRINT Board.Conferences[0].Areas[0].Find(MsgField(99), \"x\").Valid", "Msg", "Invalid"),
        ("PRINT Terminal.Input.MouseOn(MouseMode(99))", "Term", "Invalid"),
        ("PRINT Terminal.Input.MouseOn(MouseMode.Text, MouseTracking(99))", "Term", "Invalid"),
    ] {
        assert_eq!(
            "0|1|1",
            run_ppl(&format!(
                "{body}\nERROR result = Error.Last()\nPRINT \"|\", result.Kind = ErrKind.{kind}, \"|\", result.Code = ErrCode.{code}\n"
            )),
            "{body}"
        );
    }
}

#[test]
fn s4_open_enums_unsupported_backend_preserves_live_resources() {
    for number in [-1, 1, 99] {
        assert_eq!(
            "0|1|1|2|1|123",
            run_ppl(&format!(
                r#"
Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
SURFACE image = Surface.New(2, 2)
image.SetPixel(0, 0, 123)
PRINT Terminal.Gfx.Init(GfxBackend({number}), FALSE)
ERROR result = Error.Last()
PRINT "|", result.Kind = ErrKind.Gfx, "|", result.Code = ErrCode.Unsupported
PRINT "|", Terminal.Gfx.Backend, "|", image.Valid, "|", image.GetPixel(0, 0)
"#
            ))
        );
    }
}

#[test]
fn s4_open_enums_malformed_record_reads_remain_atomic() {
    for (operation, bytes) in [
        ("FGETREC", b"99\n2147483648\n".as_slice()),
        ("FGETREC", b"99\nnot-an-integer\n".as_slice()),
        ("FREADREC", &[8, 0, 0, 0, 99, 0, 0, 0, 8]),
    ] {
        let source = format!(
            r#"
ENUM Bits
 One = 1
ENDENUM
TYPE Saved
 INTEGER Serial
 Bits Value
ENDTYPE
Saved value
value.Serial = 42
value.Value = Bits(64)
Saved before = value
FOPEN 1, "bad.dat", O_RD, S_DN
{operation} 1, value
ERROR result = Error.Last()
PRINT value = before, "|", value.Serial, "|", value.Value, "|", FERR(1)
PRINT "|", result.Kind = ErrKind.File, "|", result.Code = ErrCode.Format
FCLOSE 1
"#
        );
        assert_eq!("1|42|64|1|1|1", super::run_ppl_with_files(&source, &[("bad.dat", bytes)]), "{operation}");
    }
}
