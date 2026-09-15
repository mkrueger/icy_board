//! The loose scalar functions: DDATE conversion, the event flag, the keyboard
//! script flag and free disk space.

use crate::executable::{EntryType, VariableType};

use super::{compile, compile_errors, run_ppl, run_ppl_on};

#[test]
fn temporal_legacy_comparisons_propagate_conversion_errors() {
    use crate::{ast::BinOp, executable::VariableValue, vm::VirtualMachine};
    let native = VariableValue::new_time(43200).convert_to(VariableType::ClockTime).unwrap();
    for seconds in [-1, 86400, i32::MAX] {
        let legacy = VariableValue {
            vtype: VariableType::Time,
            ..VariableValue::new_int(seconds)
        };
        assert!(legacy.clone().convert_to(VariableType::ClockTime).is_err());
        for operation in [BinOp::Eq, BinOp::NotEq, BinOp::Lower, BinOp::LowerEq, BinOp::Greater, BinOp::GreaterEq] {
            for (left, right) in [(native.clone(), legacy.clone()), (legacy.clone(), native.clone())] {
                let error = VirtualMachine::apply_bin_op(operation, left, right).expect_err("invalid legacy TIME must fail comparisons");
                assert!(
                    matches!(
                        error.downcast_ref::<crate::executable::VMError>(),
                        Some(crate::executable::VMError::InvalidTemporalValue(message)) if message == "Invalid legacy time"
                    ),
                    "{operation:?}, {seconds}: {error}"
                );
            }
        }
    }
}

#[test]
fn temporal_bytecode_numeric_arguments_and_conditions_return_errors() {
    use crate::executable::{Executable, FuncOpCode, OpCode, PPECommand, PPEExpr, PPEScript};
    for source in [
        "DATE value\nPRINT value",
        "PRINT DATE.Create(1983, 9, 15)",
        "TIME value\nPRINT value",
        "TIMESTAMP value\nPRINT value",
    ] {
        let executable = compile(&format!("{source}\nINTEGER numbers[1]\nPRINT numbers[0]"));
        let script = PPEScript::from_ppe_file(&executable).unwrap();
        let PPECommand::PredefinedCall(print, arguments) = &script.statements[0].command else {
            panic!("PRINT expected")
        };
        let value = arguments[0].clone();
        let PPECommand::PredefinedCall(_, array_arguments) = &script.statements[1].command else {
            panic!("array PRINT expected")
        };
        let PPEExpr::Dim(array_id, _) = &array_arguments[0] else {
            panic!("array access expected")
        };
        let mut commands = vec![PPECommand::IfNot(Box::new(value.clone()), 0)];
        for opcode in [
            FuncOpCode::ABS,
            FuncOpCode::SPACE,
            FuncOpCode::CHR,
            FuncOpCode::RANDOM,
            FuncOpCode::TOINTEGER,
            FuncOpCode::TOREAL,
            FuncOpCode::TOLONG64,
            FuncOpCode::TOUNSIGNED,
        ] {
            commands.push(PPECommand::PredefinedCall(
                print,
                vec![PPEExpr::PredefinedFunctionCall(opcode.get_definition(), vec![value.clone()])],
            ));
        }
        for opcode in [OpCode::COLOR, OpCode::DELAY] {
            commands.push(PPECommand::PredefinedCall(opcode.get_definition(), vec![value.clone()]));
        }
        commands.push(PPECommand::PredefinedCall(print, vec![PPEExpr::Dim(*array_id, vec![value.clone()])]));
        commands.push(PPECommand::Let(Box::new(PPEExpr::Dim(*array_id, vec![value.clone()])), Box::new(value.clone())));
        commands.push(PPECommand::PredefinedCall(
            print,
            vec![PPEExpr::PredefinedFunctionCall(
                FuncOpCode::TemporalCall.get_definition(),
                vec![value.clone(); 5],
            )],
        ));
        for command in commands {
            let description = format!("{command:?}");
            let mut invalid = executable.clone();
            let mut invalid_script = script.clone();
            invalid_script.statements[0].command = command;
            invalid.in_memory_script = Some(invalid_script);
            let invalid = Executable::from_buffer(&mut invalid.to_buffer().unwrap(), false).unwrap();
            let (result, _, ()) = super::try_run_executable_collecting_inspected(invalid, |_| {}, &[], None, b"", false, super::TestPpeBoundary::Vm, |_| ());
            let error = result.expect_err(&description);
            assert!(
                matches!(
                    error.downcast_ref::<crate::executable::VMError>(),
                    Some(crate::executable::VMError::InvalidTemporalValue(_))
                ),
                "{description}: {error}"
            );
        }
    }
}

#[test]
fn temporal_bytecode_operators_return_errors_without_panicking() {
    use crate::ast::{BinOp, UnaryOp};
    use crate::executable::{PPECommand, PPEExpr, PPEScript};
    for source in ["DATE value\nPRINT value", "PRINT DATE.Create(1983, 9, 15)"] {
        let executable = compile(source);
        let script = PPEScript::from_ppe_file(&executable).unwrap();
        let PPECommand::PredefinedCall(definition, arguments) = &script.statements[0].command else {
            panic!("PRINT expected")
        };
        let value = arguments[0].clone();
        let mut expressions = Vec::new();
        for operation in [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Mul,
            BinOp::Div,
            BinOp::Mod,
            BinOp::PoW,
            BinOp::And,
            BinOp::Or,
            BinOp::ShortAnd,
            BinOp::ShortOr,
        ] {
            expressions.push(PPEExpr::BinaryExpression(operation, Box::new(value.clone()), Box::new(value.clone())));
        }
        for operation in [UnaryOp::Plus, UnaryOp::Minus, UnaryOp::Not] {
            expressions.push(PPEExpr::UnaryExpression(operation, Box::new(value.clone())));
        }
        for expression in expressions {
            let description = format!("{expression:?}");
            let mut invalid = executable.clone();
            let mut invalid_script = script.clone();
            invalid_script.statements[0].command = PPECommand::PredefinedCall(definition, vec![expression]);
            invalid.in_memory_script = Some(invalid_script);
            let (result, _, ()) = super::try_run_executable_collecting_inspected(invalid, |_| {}, &[], None, b"", false, super::TestPpeBoundary::Vm, |_| ());
            let error = result.expect_err(&description);
            assert!(
                matches!(
                    error.downcast_ref::<crate::executable::VMError>(),
                    Some(crate::executable::VMError::InvalidTemporalValue(_))
                ),
                "{error}"
            );
        }
    }
}

#[test]
fn temporal_host_field_assignment_reports_invalid_without_mutation() {
    let output = run_ppl(
        r#"
DATE original = Session.User.BirthDate
Session.User.BirthDate = "not-a-date"
PRINT Error.Last().Kind = ErrKind.User, Error.Last().Code = ErrCode.Invalid
PRINT Session.User.BirthDate = original
DATE emptyDate
Session.User.BirthDate = emptyDate
PRINT Error.Last().Code = ErrCode.Invalid, Session.User.BirthDate = original
"#,
    );
    assert_eq!(output, "11111");
}

#[test]
fn temporal_conditions_require_explicit_boolean_values() {
    for typ in ["DATE", "TIME", "TIMESTAMP"] {
        for statement in [
            "IF value PRINT 1",
            "IF value GOTO done\n:done",
            "IF value THEN\nPRINT 1\nENDIF",
            "WHILE value PRINT 1",
            "WHILE value DO\nBREAK\nENDWHILE",
            "REPEAT\nPRINT 1\nUNTIL value",
            "PRINT !value",
            "PRINT value && TRUE",
            "PRINT value || FALSE",
        ] {
            let source = format!("{typ} value\n{statement}");
            assert!(
                compile_errors(&source).iter().any(|error| error.contains("Invalid date/time operation")),
                "{source}"
            );
        }
        assert_eq!(run_ppl(&format!("{typ} value\nIF value.IsEmpty PRINT 1\nIF !TOBOOLEAN(value) PRINT 2")), "12");
    }
    assert_eq!(run_ppl(";$LANGVERSION 330\nDATE value\nvalue = 1\nIF (value) PRINT 1"), "1");
}

#[test]
fn temporal_input_preserves_iso_date_and_fractional_seconds() {
    let output = super::run_ppl_with_input(
        "DATE birthday\nTIME clock\nINPUTDATE \"Date\", birthday, 7\nINPUTTIME \"Time\", clock, 7\nPRINTLN \"RESULT=\", birthday, \"|\", clock\n",
        b"1883-09-15\r12:34:56.123456789\r",
    );
    assert!(output.contains("RESULT=1883-09-15|12:34:56.123456789"), "{output:?}");
    let legacy = super::run_ppl_with_input(
        ";$LANGVERSION 340\nDATE birthday\nTIME clock\nINPUTDATE \"Date\", birthday, 7\nINPUTTIME \"Time\", clock, 7\nPRINTLN \"RESULT=\", birthday, \"|\", clock\n",
        b"09-15-83\r12:34:56\r",
    );
    assert!(legacy.contains("RESULT=09/15/83|12:34:56"), "{legacy:?}");
}

#[test]
fn temporal_ppl400_constants_and_routines_use_checked_values() {
    let source = r#"
        ;$LANGVERSION 400
        CONST DATE birthday = "1883-09-15"
        CONST TIMESTAMP epoch = "1970-01-01T00:00:00Z"
        PRINTLN birthday, "|", epoch.IsEmpty
        PRINTLN NextDay(birthday)
        EXIT
        FUNCTION NextDay(DATE value) DATE
            RETURN value.AddDays(1)
        ENDFUNC
    "#;
    assert_eq!(run_ppl(source), "1883-09-15|0\n1883-09-16\n");
    assert!(!compile_errors("CONST DATE invalid = \"2023-02-29\"\nPRINT invalid").is_empty());
    assert!(!super::compile_errors_with_runtime("DATE value\nPRINT value", 340).is_empty());
}

#[test]
fn temporal_ppl400_values_members_and_timestamp_roundtrip() {
    assert_eq!(
        run_ppl(
            r#"
        ;$LANGVERSION 400
        DATE birthday
        TIME clock
        TIMESTAMP moment
        PRINTLN birthday.IsEmpty, "|", clock.IsEmpty, "|", moment.IsEmpty
        birthday = DATE.Create(1883, 9, 15)
        clock = TIME.Parse("00:30:00.123456789")
        moment = TIMESTAMP.FromUtc(birthday, clock)
        PRINTLN birthday, "|", birthday.Year, "|", birthday.Month, "|", birthday.Day
        PRINTLN birthday.WithYear(1983).Format("%d-%m-%Y")
        PRINTLN moment, "|", moment.UtcDate, "|", moment.UtcTime, "|", moment.Nanosecond
        PRINTLN YEAR(MKDATE(2400, 2, 29)), "|", DATE.Parse("2400-02-29").AddDays(1)
        PRINTLN TIMESTAMP.FromUnix(0).IsEmpty
    "#
        ),
        "1|1|1\n1883-09-15|1883|9|15\n15-09-1983\n1883-09-15T00:30:00.123456789Z|1883-09-15|00:30:00.123456789|123456789\n2400|2400-03-01\n0\n"
    );
}

#[test]
fn temporal_ppl400_arrays_records_and_legacy_bridge() {
    assert_eq!(
        run_ppl(
            r#"
        ;$LANGVERSION 400
        TYPE Entry
            DATE birthday
            TIMESTAMP moment
        ENDTYPE
        Entry record
        DATE dates[]
        REDIM dates, 2
        dates[0] = "1983-09-15"
        record.birthday = dates[0]
        record.moment = TIMESTAMP.Parse("1970-01-01T00:00:00Z")
        PRINTLN dates[1].IsEmpty, "|", record.birthday, "|", record.moment.IsEmpty
        PRINTLN record.birthday.ToLegacy(), "|", TODATE(record.birthday.ToLegacy())
        TIME midnight
        midnight = "00:00:00"
        PRINTLN midnight.IsEmpty, "|", midnight.ToLegacy()
    "#
        ),
        "1|1983-09-15|0\n09/15/83|1983-09-15\n0|00:00:00\n"
    );
}

/// Legacy day and second counts reach the new API on their own, so old data and
/// old functions stay usable. Only the lossy direction still needs `.ToLegacy()`.
#[test]
fn temporal_legacy_values_widen_into_the_new_api() {
    assert_eq!(
        run_ppl(
            r#"
;$LANGVERSION 400
DECLARE PROCEDURE Show(DATE day)
DATE day = DATE.Create(1996, 3, 15)
TIME moment = TIME.Create(12, 34, 56)
PRINTLN day = MKDATE(1996, 3, 15).ToLegacy(), day = MKDATE(1996, 3, 16).ToLegacy()
PRINTLN day < MKDATE(1996, 3, 16).ToLegacy(), day > MKDATE(1996, 3, 16).ToLegacy()
PRINTLN moment = moment.ToLegacy(), "|", DATE.Create(1996, 3, 1).DaysUntil(MKDATE(1996, 3, 15).ToLegacy())
PRINTLN TIMESTAMP.FromUtc(MKDATE(1996, 3, 15).ToLegacy(), moment.ToLegacy())
DATE assigned = MKDATE(1996, 3, 15).ToLegacy()
PRINTLN assigned, "|", assigned.Year
Show(MKDATE(1996, 3, 15).ToLegacy())
DATE nothing
PRINTLN nothing = nothing.ToLegacy(), day = nothing.ToLegacy()
TIME emptyTime
TIME midnight = emptyTime.ToLegacy()
PRINTLN midnight.IsEmpty, "|", midnight, "|", midnight = emptyTime.ToLegacy(), "|", emptyTime = emptyTime.ToLegacy()
Session.User.BirthDate = MKDATE(1996, 3, 15).ToLegacy()
PRINTLN Error.Last().OK, "|", Session.User.BirthDate
EXIT
PROCEDURE Show(DATE day)
    PRINTLN "shown ", day, " ", day.DayOfWeek
ENDPROC
"#
        ),
        "10\n10\n1|14\n1996-03-15T12:34:56Z\n1996-03-15|1996\nshown 1996-03-15 5\n10\n0|00:00:00|1|0\n1|1996-03-15\n"
    );
    // The lossy direction stays explicit, and widening never mixes dates with times.
    for source in [
        ";$LANGVERSION 400\nPRINT DATE.Create(1996, 3, 15) = TIME.Create(1, 2, 3).ToLegacy()",
        ";$LANGVERSION 400\nPRINT DATE.Create(1996, 3, 15) < TIME.Create(1, 2, 3).ToLegacy()",
        ";$LANGVERSION 400\nPRINT TIMESTAMP.Now().SecondsUntil(MKDATE(1996, 3, 15).ToLegacy())",
        ";$LANGVERSION 400\nPRINT DATE.Create(1996, 3, 15).DaysUntil(TIME.Create(1, 2, 3).ToLegacy())",
    ] {
        assert!(!compile_errors(source).is_empty(), "{source}");
    }
}

#[test]
fn temporal_legacy_arrays_widen_by_value() {
    for typ in ["EDATE", "DDATE"] {
        for (shape, parameter, first, second) in [
            ("[2]", "[]", "[0]", "[1]"),
            ("[2,2]", "[,]", "[0,0]", "[1,1]"),
            ("[2,2,2]", "[,,]", "[0,0,0]", "[1,1,1]"),
        ] {
            let source = format!(
                r#"
;$LANGVERSION 400
DECLARE PROCEDURE Change(DATE values{parameter})
DECLARE PROCEDURE ChangeInPlace(VAR DATE values{parameter})
TYPE Holder
    DATE items{shape}
ENDTYPE
{typ} legacy{shape}
legacy{first} = DATE.Create(1996, 3, 15).ToLegacy()
DATE native{shape}
native = legacy
PRINTLN native{first}, "|", native{second}.IsEmpty
Holder assigned
assigned.items = legacy
Holder literal = Holder {{ items = legacy }}
PRINTLN assigned.items{first}, "|", literal.items{first}
Change(legacy)
PRINTLN native{first}, "|", YEAR(legacy{first})
ChangeInPlace(native)
PRINTLN native{first}
EXIT
PROCEDURE Change(DATE values{parameter})
    PRINTLN values{first}, "|", values{second}.IsEmpty
    values{first} = DATE.Create(2000, 1, 1)
    PRINTLN values{first}
ENDPROC
PROCEDURE ChangeInPlace(VAR DATE values{parameter})
    values{first} = DATE.Create(2001, 1, 1)
ENDPROC
"#
            );
            assert_eq!(
                run_ppl(&source),
                "1996-03-15|1\n1996-03-15|1996-03-15\n1996-03-15|1\n2000-01-01\n1996-03-15|1996\n2001-01-01\n",
                "{typ} {shape}"
            );
        }
    }
    for source in [
        "DATE native[2]\nEDATE legacy[2]\nlegacy = native",
        "EDATE legacy[2]\nTIME native[2]\nnative = legacy",
        "EDATE legacy[2]\nDATE native[2,2]\nnative = legacy",
        "TYPE Holder\nDATE items[2]\nENDTYPE\nHolder target\nEDATE legacy[3]\ntarget.items = legacy",
        "DECLARE PROCEDURE Change(VAR DATE values[])\nEDATE legacy[2]\nChange(legacy)\nEXIT\nPROCEDURE Change(VAR DATE values[])\nENDPROC",
        "DECLARE PROCEDURE Change(VAR DATE value)\nEDATE legacy\nChange(legacy)\nEXIT\nPROCEDURE Change(VAR DATE value)\nENDPROC",
    ] {
        let errors = compile_errors(source);
        assert!(errors.iter().any(|error| error.contains("expects")), "{source}: {errors:?}");
    }
}

#[test]
fn temporal_ppl400_invalid_operations_are_rejected() {
    for source in [
        "DATE value\nINTEGER values[1]\nPRINT values[value]",
        "DATE value\nPRINT STRING.Repeat(\"x\", value)",
        "DATE value\nPRINT User.Load(value)",
        "DATE value\nvalue.Year = 1983",
    ] {
        assert!(!compile_errors(source).is_empty(), "{source}");
    }
    assert!(!compile_errors("CONST DATE value = \"2024-01-01\"\nCONST INTEGER broken = TOINTEGER(value)\nPRINT broken").is_empty());
    assert!(!compile_errors("DATE value\nINC value").is_empty());
    assert!(!compile_errors("CONST DATE value = \"2024-01-01\"\nCONST INTEGER broken = value + 1\nPRINT broken").is_empty());
    assert!(!compile_errors("DATE dates[1]\nPRINT dates < dates").is_empty());
    assert!(!compile_errors(";$LANGVERSION 400\nPRINT TOINTEGER(DATE.Today())").is_empty());
    assert_eq!(run_ppl("PRINT TODDATE(DATE.Create(1994, 5, 27))"), "19940527");
    assert_eq!(run_ppl("PRINT TODATE(TODDATE(DATE.Create(1994, 5, 27)))"), "1994-05-27");
    assert!(!compile_errors(";$LANGVERSION 400\nDATE value\nPRINT value * 2").is_empty());
    assert!(!compile_errors(";$LANGVERSION 400\nPRINT DATE.Create(2024, \"two\", 1)").is_empty());
}

#[test]
fn ppl400_string_uses_unbounded_storage_without_changing_literal_encoding() {
    let executable = compile(
        ";$LANGVERSION 400\nDECLARE FUNCTION Echo(STRING input) STRING\nSTRING text\nSTRING values[]\ntext = Echo(\"literal\")\nPRINT text, values.Len()\nFUNCTION Echo(STRING input) STRING\n STRING local\n local = input\n RETURN local\nENDFUNC",
    );
    let text = executable
        .variable_table
        .get_entries()
        .iter()
        .find(|entry| entry.header.variable_type == VariableType::UnboundedString && entry.header.dim == 0)
        .unwrap();
    let values = executable
        .variable_table
        .get_entries()
        .iter()
        .find(|entry| entry.header.variable_type == VariableType::UnboundedString && entry.header.dim == 1)
        .unwrap();
    let literal = executable
        .variable_table
        .get_entries()
        .iter()
        .find(|entry| entry.value.as_string() == "literal")
        .unwrap();
    assert_eq!(VariableType::UnboundedString, text.header.variable_type);
    assert_eq!(VariableType::UnboundedString, values.header.variable_type);
    assert_eq!(VariableType::String, literal.header.variable_type);
    assert!(
        executable
            .variable_table
            .get_entries()
            .iter()
            .filter(|entry| entry.entry_type != EntryType::Constant)
            .all(|entry| entry.header.variable_type != VariableType::String)
    );

    let legacy = compile(";$LANGVERSION 340\nSTRING text\nPRINT text");
    let text = legacy
        .variable_table
        .get_entries()
        .iter()
        .find(|entry| entry.header.variable_type == VariableType::String)
        .unwrap();
    assert_eq!(VariableType::String, text.header.variable_type);

    assert_eq!("70000\n", run_ppl("STRING text\ntext = STRING.Repeat(\"x\", 70000)\nPRINTLN text.Len()"));
}

#[test]
fn ppl400_const_string_keeps_long_literal_values() {
    let literal = "x".repeat(300);
    let source = format!(";$LANGVERSION 400\nCONST STRING text = \"{literal}\"\nSTRING value = text\nPRINT value.Len()");

    assert_eq!("300", run_ppl(&source));
}

#[test]
fn test_toddate_reads_a_ccyymmdd_string() {
    assert_eq!(run_ppl("PRINT TODDATE(\"19940527\")"), "19940527");
}

/// `ABS` answers in the type it was given rather than truncating to a whole number,
/// the way `cVARVAL::abs` does; only the unsigned and date family folds to an integer.
#[test]
fn test_abs_keeps_the_type_of_its_argument() {
    let output = run_ppl(
        r#"
        DOUBLE d
        SWORD w
        d = 0.0 - 2.5
        w = 0 - 300
        PRINTLN Abs(d)
        PRINTLN Abs(0.0 - 1.75)
        PRINTLN Abs(0 - 3)
        PRINTLN Abs(w)
        PRINTLN Abs(3)
        "#,
    );

    assert_eq!(output, "2.5\n1.75\n3\n300\n3\n");
}

/// Arithmetic is evaluated without the async walk, so an expression that mixes it with a
/// call has to fall back and still answer the same. `&` and `|` evaluate both sides either
/// way, which a counting function makes visible.
#[test]
fn test_expressions_mixing_arithmetic_and_calls_evaluate_the_same() {
    let output = run_ppl(
        r#"
        INTEGER calls, values(3)
        values[0] = 10
        values[1] = 20
        values[2] = 30
        values[3] = 40
        PRINTLN 2 * 3 + 4
        PRINTLN values[1 + 1] + 5
        PRINTLN Bump(1) * 2 + values[Bump(0) - 1]
        PRINTLN calls
        calls = 0
        PRINTLN (Bump(1) > 0) | (Bump(1) > 0)
        PRINTLN calls
        EXIT

        FUNCTION Bump(INTEGER add) INTEGER
            calls = calls + 1
            Bump = add + 1
        ENDFUNC
        "#,
    );

    assert_eq!(output, "10\n35\n14\n2\n1\n2\n");
}

#[test]
fn test_mixed_strings_and_numbers_promote_to_integer() {
    let output = run_ppl(
        r#"
STRING value
value = "2"
PRINTLN value + 1
PRINTLN 1 + value
PRINTLN value + value
PRINTLN value - 1
PRINTLN value * 3
PRINTLN value / 2
PRINTLN value % 2
PRINTLN value = 2
PRINTLN value < 10
INC value
PRINTLN value
DEC value
PRINTLN value
PRINTLN " 7x" - "2x"
PRINTLN " 7x" / "2x"
PRINTLN " 7x" % "2x"
PRINTLN -" 7x"
PRINTLN ABS(" -7x")
"#,
    );

    assert_eq!(output, "3\n3\n22\n1\n6\n1\n0\n1\n1\n3\n2\n5\n3\n1\n-7\n7\n");
}

#[test]
fn test_division_and_modulo_by_zero_answer_zero() {
    assert_eq!(run_ppl("PRINTLN 7 / 0\nPRINTLN 7 % 0\nPRINTLN \"7\" / \"0\""), "0\n0\n0\n");
}

#[test]
fn test_signed_small_integers_compare_as_signed() {
    let output = run_ppl(
        r#"
SBYTE byte_value
SWORD word_value
byte_value = -1
word_value = -1
PRINTLN byte_value < 1
PRINTLN word_value < 1
"#,
    );

    assert_eq!(output, "1\n1\n");
}

#[test]
fn test_function_name_assignment_returns_a_value_in_classic_languages() {
    for language_version in [300, 310, 320, 330, 340] {
        let source = format!(
            ";$LANGVERSION {language_version}\n\
             DECLARE FUNCTION LegacyAddOne(INTEGER value) INTEGER\n\
             PRINT LegacyAddOne(41)\n\
             END\n\
             FUNCTION LegacyAddOne(INTEGER value) INTEGER\n\
               LegacyAddOne = value + 1\n\
             ENDFUNC\n"
        );

        assert_eq!(run_ppl(&source), "42", "language version {language_version}");
    }
}

#[test]
fn test_a_function_can_read_and_rewrite_its_return_value() {
    let source = r#"
DECLARE FUNCTION NumberResult(INTEGER value) INTEGER
DECLARE FUNCTION StringResult(STRING value) STRING
PRINTLN NumberResult(1)
PRINTLN StringResult("ABCDEFGHIJKLMNOPQRST")
EXIT

FUNCTION NumberResult(INTEGER value) INTEGER
    NumberResult = value
    IF (NumberResult = 1) NumberResult = NumberResult + 4
ENDFUNC

FUNCTION StringResult(STRING value) STRING
    StringResult = value
    IF (LEN(StringResult) > 17) StringResult = LEFT(StringResult, 17)
ENDFUNC
"#;

    assert_eq!(run_ppl(source), "5\nABCDEFGHIJKLMNOPQ\n");
}

/// PCBACCSTAT field 0 reports the session mode, not the global enable flag:
/// 0 is disabled, 1 is tracking, and 2 requires a started, enforced session.
#[test]
fn test_pcbaccstat_reports_the_accounting_status() {
    use std::sync::Arc;

    use crate::{
        icy_board::{
            IcyBoard, accounting_cfg::AccountingConfig, bbs::BBS, pcb::user_inf::AccountUserInf, sec_levels::SecurityLevel, state::IcyBoardState,
            user_base::User,
        },
        vm::{DiskIO, run},
    };
    use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

    assert_eq!(run_ppl("PRINT PCBACCSTAT(0)"), "0");
    let configured_only = run_ppl_on("PRINT PCBACCSTAT(0)", |board| {
        board.config.accounting.enabled = true;
    });
    assert_eq!(configured_only, "0");

    let enabled = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.config.accounting.enabled = true;
        board.config.accounting.accounting_config = Some(AccountingConfig::default());
        board.sec_levels.levels.push(SecurityLevel {
            security: 10,
            is_enabled: true,
            ..Default::default()
        });
        board.users.new_user(User {
            name: "CALLER".into(),
            security_level: 10,
            account: Some(AccountUserInf {
                starting_balance: 100.0,
                ..Default::default()
            }),
            ..Default::default()
        });
        let user = board.users[0].clone();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        state.session.current_user = Some(user);
        state.session.cur_user_id = 0;
        state.session.cur_security = 10;
        state.session.user_name = "CALLER".into();
        assert!(!state.accounting_active());
        state.accounting_start().await.unwrap();
        assert!(state.accounting_active());

        let executable = compile("PRINT PCBACCSTAT(0)");
        let mut io = DiskIO::new(".", None);
        run(&std::path::PathBuf::from("test.ppe"), &executable, &mut io, &mut state).await.unwrap();
        drop(state);
        let mut output = Vec::new();
        let mut buffer = [0; 64];
        while let Ok(size) = peer.read(&mut buffer).await {
            if size == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..size]);
        }
        String::from_utf8(output).unwrap()
    });
    assert_eq!(enabled, "2");
}

#[test]
fn test_a_door_password_can_be_compared_but_not_printed() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf
        DOOR item
        conf = Board.Conferences[0]
        item = conf.Doors[0]
        PRINT "[", item.Password, "] ", item.Password = "SeCrEt", " ", item.Password = "wrong"
    "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                doors: Some(std::sync::Arc::new(crate::icy_board::doors::DoorList {
                    doors: vec![crate::icy_board::doors::Door {
                        password: "secret".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
                })),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "[******] 1 0");
}

/// Reading the password must not hash it: a key derivation costs milliseconds and
/// megabytes a call, which a loop over the doors would spend for nothing.
#[test]
fn test_reading_a_door_password_stays_cheap() {
    let start = std::time::Instant::now();
    let output = run_ppl_on(
        r#"
        CONFERENCE conf
        DOOR item
        INTEGER i, hits
        conf = Board.Conferences[0]
        item = conf.Doors[0]
        FOR i = 1 TO 200
            IF item.Password = "secret" hits = hits + 1
        NEXT
        PRINT hits
    "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                doors: Some(std::sync::Arc::new(crate::icy_board::doors::DoorList {
                    doors: vec![crate::icy_board::doors::Door {
                        password: "secret".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
                })),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "200");
    // Argon2 would need seconds for this, comparing the secret needs microseconds.
    assert!(start.elapsed() < std::time::Duration::from_secs(2), "took {:?}", start.elapsed());
}

#[test]
fn test_conference_properties_report_configuration_and_counts() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf
        conf = Board.Conferences[0]
        PRINT conf.IsPublic, " ", conf.HasAccess(), " ", conf.Directories.Len(), " ", conf.Areas.Len(), " ", conf.Doors.Len()
    "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                is_public: false,
                areas: Some(std::sync::Arc::new(crate::icy_board::message_area::AreaList::new(vec![
                    crate::icy_board::message_area::MessageArea::default(),
                    crate::icy_board::message_area::MessageArea::default(),
                ]))),
                doors: Some(std::sync::Arc::new(crate::icy_board::doors::DoorList {
                    doors: vec![crate::icy_board::doors::Door::default(), crate::icy_board::doors::Door::default()],
                    ..Default::default()
                })),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "0 1 0 2 2");
}

#[test]
fn test_an_invalid_conference_number_still_returns_a_conference() {
    let output = run_ppl(
        r#"
        CONFERENCE conf
        conf = Board.Conferences[999]
        PRINT "[", conf.Name, "] ", conf.IsPublic, " ", conf.Directories.Len(), " ", conf.Areas.Len(), " ", conf.Doors.Len()
    "#,
    );
    assert_eq!(output, "[] 0 0 0 0");
}

#[test]
fn test_len_dimension_returns_each_dimension_element_count() {
    let output = run_ppl(
        r#"
        INTEGER one(10)
        INTEGER two(2, 3)
        INTEGER three(1, 2, 3)
        PRINT one.Len(), " ", LEN(one, 0), " ", LEN(two, 0), " ", LEN(two, 1), " ", LEN(three, 0), " ", LEN(three, 1), " ", LEN(three, 2)
    "#,
    );
    assert_eq!(output, "11 11 3 4 2 3 4");
}

#[test]
fn test_ppl400_string_member_positions_are_zero_based() {
    let output = run_ppl(
        r#"
        BIGSTR text = "ä two two"
        PRINTLN text.Find("ä"), " ", text.Find("two"), " ", text.Find("two", 3)
        PRINTLN text.FindLast("two"), " ", text.FindLast("two", 4)
        PRINTLN text.Find("missing"), " ", text.FindLast("missing")
        PRINTLN INSTR(text, "two"), " ", INSTRR(text, "two")
        "#,
    );
    assert_eq!(output, "0 2 6\n6 2\n-1 -1\n3 7\n");
}

#[test]
fn test_ppl400_scalar_strings_support_zero_based_character_indices() {
    let output = run_ppl(
        r#"
        BIGSTR text = "Aäß"
        PRINTLN "[", text[0], "][", text[1], "][", text[2], "]"
        PRINTLN "[", text[-1], "][", text[3], "]"
        PRINTLN "xy"[1], " ", " z ".Trim()[0]
        STRING words[0]
        words[0] = "whole"
        PRINTLN words[0], " ", words[0][0], words[0][4]
        "#,
    );
    assert_eq!(output, "[A][ä][ß]\n[][]\ny z\nwhole we\n");

    let errors = compile_errors(";$LANGVERSION 340\nSTRING text = \"abc\"\nPRINTLN text[0]");
    assert!(!errors.is_empty(), "scalar string indexing should require language 400");
}

#[test]
fn test_ppl400_string_comparison_controls_search_and_equality() {
    let output = run_ppl(
        r#"
        BIGSTR text = "Ä One ONE"
        PRINTLN text.Find("one"), " ", text.Find("one", 0, StringComparison.OrdinalIgnoreCase)
        PRINTLN text.FindLast("one", 8, StringComparison.OrdinalIgnoreCase), " ", text.FindLast("one", 5, StringComparison.OrdinalIgnoreCase)
        PRINTLN text.Contains("one"), " ", text.Contains("one", StringComparison.OrdinalIgnoreCase)
        PRINTLN text.StartsWith("ä", StringComparison.OrdinalIgnoreCase), " ", text.EndsWith("one", StringComparison.OrdinalIgnoreCase)
        PRINTLN text.Count("one"), " ", text.Count("one", StringComparison.OrdinalIgnoreCase)
        PRINTLN "Äpfel".Equals("äPFEL"), " ", "Äpfel".Equals("äPFEL", StringComparison.OrdinalIgnoreCase)
        "#,
    );
    assert_eq!(output, "-1 2\n6 2\n0 1\n1 1\n0 2\n0 1\n");

    let errors = compile_errors("PRINTLN \"text\".Contains(\"x\", 1)");
    assert!(errors.iter().any(|error| error.contains("StringComparison")), "{errors:?}");
}

#[test]
fn test_ppl400_string_split_returns_a_dynamic_bigstr_array() {
    let output = run_ppl(
        r#"
        BIGSTR text = "one,,two,three"
        BIGSTR parts[]
        parts = text.Split(",")
        PRINTLN parts.Len(), " ", parts[0], "[", parts[1], "]", parts[2], " ", parts[3]
        PRINTLN text.Split(",", 3).Len(), " ", text.Split(",", 3)[2]
        BIGSTR part
        FOREACH part IN STRING.Split("a:b:c", ":")
            PRINT part
        ENDFOREACH
        BIGSTR invalid[]
        invalid = text.Split("")
        PRINTLN " ", invalid.Len(), " ", Error.Last().Kind = ErrKind.String, " ", Error.Last().Code = ErrCode.Invalid
        "#,
    );
    assert_eq!(output, "4 one[]two three\n3 two,three\nabc 0 1 1\n");

    let errors = compile_errors("STRING parts[]\n\"a,b\".Split(\",\", parts)");
    assert!(!errors.is_empty(), "the removed output-array Split signature should not compile");
}

/// An object is held by the values that name it rather than by a table that only
/// ever grows, so a loop can keep asking for one and reading it back.
#[test]
fn test_a_loop_can_keep_asking_for_objects() {
    let output = run_ppl_on(
        r#"
        CONFERENCE conf
        AREA first
        INTEGER i, seen
        FOR i = 1 TO 500
            conf = Board.Conferences[0]
            first = conf.Areas[0]
            IF first.Name = "General" seen = seen + 1
        NEXT
        PRINT seen, " ", conf.Name
    "#,
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                name: "Main".to_string(),
                areas: Some(std::sync::Arc::new(crate::icy_board::message_area::AreaList::new(vec![
                    crate::icy_board::message_area::MessageArea {
                        name: "General".to_string(),
                        ..Default::default()
                    },
                ]))),
                ..Default::default()
            });
        },
    );
    assert_eq!(output, "500 Main");
}

/// `PCBoard` kept a name and a city per node in USERNET, so what WRUNET writes is
/// what `UN_NAME` and `UN_CITY` read back.
#[test]
fn test_wrunet_keeps_the_name_and_city_a_ppe_wrote() {
    let output = run_ppl(
        r#"
        WRUNET PCBNODE(), "", "FAKE CALLER", "FAKE CITY", "doing things", ""
        RDUNET PCBNODE()
        PRINTLN "name=", UN_NAME(), " city=", UN_CITY(), " oper=", UN_OPER()
    "#,
    );
    assert_eq!(output, "name=FAKE CALLER city=FAKE CITY oper=doing things\n");
}

/// A DDATE holds the julian date a DATE holds; only its text form is CCYYMMDD.
/// Verified against `PCBoard` 15.4/M.
#[test]
fn test_a_ddate_holds_the_julian_date_behind_its_ccyymmdd_text() {
    assert_eq!(run_ppl("INTEGER i\ni = TODDATE(\"19940527\")\nPRINT i"), "34480");
    assert_eq!(run_ppl("DDATE d\nd = TODDATE(\"19940527\")\nPRINT d"), "19940527");
}

/// An EDATE holds that same julian and shows itself as YYMM.DD.
#[test]
fn test_an_edate_shows_the_date_as_yymm_dd() {
    assert_eq!(run_ppl(";$LANGVERSION 340\nEDATE e\ne = MKDATE(1996, 3, 15)\nPRINT e"), "9603.15");
    assert_eq!(run_ppl(";$LANGVERSION 340\nEDATE e\ne = MKDATE(1996, 3, 15)\nPRINT TOINTEGER(e)"), "35138");
}

#[test]
fn test_an_edate_requires_yymm_dd_text() {
    assert_eq!(run_ppl("PRINT TOEDATE(\"03-15-96\")"), "0000.00");
    assert_eq!(run_ppl("PRINT TOEDATE(\"9603.15\")"), "9603.15");
}

#[test]
fn test_toddate_converts_a_date() {
    assert_eq!(run_ppl("PRINT TODDATE(MKDATE(1994, 5, 27))"), "19940527");
}

#[test]
fn pcboard_datetime_parts_and_time_validation() {
    let output = run_ppl(
        r#"
        PRINTLN YEAR(MKDATE(1996, 3, 15)), "|", DOW(MKDATE(1996, 3, 15))
        PRINTLN "[", TIMEAP(TOTIME("14:22:36")), "]"
        PRINTLN VALTIME("00:00:00"), "|", VALTIME("12:34"), "|", VALTIME("25:61:99")
        "#,
    );
    assert_eq!(output, "1996|5\n[ 2:22:36 PM]\n1|1|0\n");
}

#[test]
fn pcboard_datetime_oracle_fixture() {
    let source = include_str!("../../../../../compat/datetime.pps");
    let actual = run_ppl(source);
    let expected = include_str!("../../../../../compat/datetime.out");
    assert_eq!(actual.lines().count(), expected.lines().count());
    assert_eq!(actual, expected);
    assert!(
        compile(source)
            .variable_table
            .get_entries()
            .iter()
            .all(|entry| !entry.header.variable_type.is_temporal())
    );
}

#[test]
fn pcboard_datetime_invalid_memory_access_is_not_emulated() {
    assert_eq!(
        run_ppl(";$LANGVERSION 340\nPRINTLN TOINTEGER(MKDATE(2024, 13, 32)), \"|\", DOW(MKDATE(2100, 2, 29))"),
        "0|6\n"
    );
    assert_eq!(run_ppl("PRINTLN VALDATE(\"1\u{20ac}2345\")"), "0\n");
}

#[test]
fn test_a_date_survives_the_trip_through_ddate_and_back() {
    assert_eq!(run_ppl(";$LANGVERSION 340\nPRINT TODATE(TODDATE(MKDATE(1994, 5, 27)))"), "05/27/94");
}

#[test]
fn test_a_ddate_variable_takes_a_date_by_assignment() {
    assert_eq!(run_ppl(";$LANGVERSION 340\nDDATE d\nd = MKDATE(2001, 12, 31)\nPRINT d"), "20011231");
}

#[test]
fn test_no_event_has_taken_time_away() {
    assert_eq!(run_ppl("PRINT EVTTIMEADJ()"), "0");
}

#[test]
fn test_adjtime_adds_time_while_no_event_is_pending() {
    assert_eq!(run_ppl("ADJTIME 10\nPRINT MINLEFT()"), "1010");
}

#[test]
fn test_no_keyboard_script_is_running_to_start_with() {
    assert_eq!(run_ppl("PRINT KBDFILUSED()"), "0");
}

#[test]
fn test_kbdstuff_is_not_a_keyboard_script() {
    assert_eq!(run_ppl("KBDSTUFF \"X\"\nPRINT KBDFILUSED()"), "0");
}

#[test]
fn test_kbdfile_is_a_keyboard_script() {
    assert_eq!(
        run_ppl("FCREATE 1, \"S.KBD\", O_WR, S_DN\nFPUTLN 1, \"HELLO\"\nFCLOSE 1\nKBDFILE \"S.KBD\"\nPRINT KBDFILUSED()"),
        "1"
    );
}

#[test]
fn test_drivespace_reports_room_on_the_drive_the_board_is_on() {
    assert_eq!(run_ppl("PRINT DRIVESPACE(\"C:\\\\\") > 0"), "1");
}

#[test]
fn test_drivespace_reports_nothing_for_a_path_that_is_not_there() {
    assert_eq!(run_ppl("PRINT DRIVESPACE(\"NOSUCHDIR\")"), "0");
}
