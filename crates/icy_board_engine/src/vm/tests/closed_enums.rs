//! Open nominal enum contracts, exercised through serialization and the real VM.
use super::{compile, compile_errors, compile_errors_with_runtime, run_ppl};
use crate::executable::{GenericVariableData, VariableType, VariableValue};

const DOMAIN: &str = "ENUM Shade\n First = 7\n Second = -3\n Alias = 7\nENDENUM\n";

fn source(language: u16, body: &str) -> String {
    format!(";$LANGVERSION {language}\n{DOMAIN}{body}\n")
}

#[test]
fn open_enums_unnamed_constants_casts_and_masks() {
    for language in [350, 400] {
        assert_eq!(
            "3|0|1|1|0|8|-2147483648|2147483647|64",
            run_ppl(&format!(
                r#";$LANGVERSION {language}
ENUM Bits
 A = 0
 B = 1
 C = 2
ENDENUM
CONST Bits Combined = Bits.B | Bits.C
CONST Bits Unknown = Bits(8)
Bits flags = Combined
PRINT flags, "|", Bits.B & Bits.C, "|", flags.Has(Bits.B), "|"
PRINT flags.Has(Bits(0)), "|", flags.Has(Unknown), "|"
INTEGER number = 8
flags = Bits(number)
PRINT flags, "|", TOINTEGER(Bits(-2147483647 - 1)), "|", Bits(2147483647), "|"
RegexOptions future = RegexOptions(64)
PRINT future
"#
            ))
        );
    }
}

#[test]
fn closed_enums_default_scalars_arrays_records_locals_and_results() {
    for language in [350, 400] {
        let body = r#"
Shade value, vector(1), matrix(1,1), cube(1,1,1)
PRINT value, ",", vector(1), ",", matrix(1,1), ",", cube(1,1,1), "|"
Probe()
Probe()
PRINT Result(TRUE), ",", Result(FALSE)
PROCEDURE Probe()
 Shade local, items(1)
 PRINT local, ",", items(1), "|"
 local = Shade.Second
 items(1) = Shade.Second
ENDPROC
FUNCTION Result(BOOLEAN setIt) Shade
 IF (setIt) THEN
  Result = Shade.Second
 ENDIF
ENDFUNC
"#;
        assert_eq!("7,7,7,7|7,7|7,7|-3,7", run_ppl(&source(language, body)), "language {language}");
    }
    assert_eq!(
        "7,7",
        run_ppl(&source(
            400,
            "TYPE Paint\n Shade Tone\n Shade Tones(1)\nENDTYPE\nPaint paint\nPRINT paint.Tone, \",\", paint.Tones(1)"
        ))
    );
}

#[test]
fn open_enums_cast_and_reverse_preserve_integer_values() {
    for language in [350, 400] {
        assert_eq!(
            "-3|1|7",
            run_ppl(&source(
                language,
                "INTEGER number = -3\nShade value = Shade(number)\nPRINT TOINTEGER(value), \"|\", value = Shade.Second, \"|\", TOINTEGER(Shade.Alias)"
            ))
        );
        for value in ["0", "8", "-999", "2147483647"] {
            assert_eq!(value, run_ppl(&source(language, &format!("Shade value = Shade({value})\nPRINT value"))));
        }
        assert_eq!("1", run_ppl(&source(language, "PRINT Shade(TRUE)")));
        assert!(!compile_errors(&source(language, "BOOLEAN flag = TRUE\nPRINT Shade(flag)")).is_empty());
        for value in ["1.5", "\"7\"", "Shade.First"] {
            let errors = compile_errors(&source(language, &format!("Shade value = Shade({value})\nPRINT value")));
            assert!(!errors.is_empty(), "accepted cast of {value} in {language}");
        }
    }
}

#[tokio::test]
async fn open_enums_wrong_type_dynamic_cast_does_not_publish_a_value() {
    use crate::{
        executable::{FuncOpCode, PPEExpr},
        icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
        vm::{VirtualMachine, io::DiskIO},
    };
    use icy_net::{ConnectionType, channel::ChannelConnection};
    use std::{path::PathBuf, sync::Arc};
    let executable = compile(&source(400, "Shade value\nINTEGER number = 8\nvalue = Shade(number)\nPRINT value"));
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (_peer, connection) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await;
    let directory = tempfile::tempdir().unwrap();
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
    let mut vm = VirtualMachine::new(PathBuf::from("enum.ppe"), &registry, &mut io, &mut state);
    vm.variable_table = executable.variable_table;
    let entries = vm.variable_table.get_entries();
    let value = entries.iter().find(|e| vm.variable_table.is_enum(e.header.variable_type)).unwrap();
    let value_id = value.header.id;
    let enum_id = u8::from(value.header.variable_type);
    let type_constant = entries
        .iter()
        .find(|e| e.value.vtype == VariableType::Integer && e.value.as_int() == i32::from(enum_id))
        .unwrap()
        .header
        .id;
    let bad_constant = entries
        .iter()
        .find(|e| e.value.vtype == VariableType::Integer && e.value.as_int() == 8)
        .unwrap()
        .header
        .id;
    let expr = PPEExpr::PredefinedFunctionCall(
        FuncOpCode::EnumCast.get_definition(),
        vec![PPEExpr::Value(type_constant), PPEExpr::Value(bad_constant)],
    );
    *vm.variable_table.get_value_mut(bad_constant) = VariableValue::new_string("8".to_string());
    assert!(vm.eval_expr(&expr).await.is_err());
    assert_eq!(7, vm.variable_table.get_value(value_id).as_int());
    assert!(
        vm.set_variable(&PPEExpr::Value(value_id), VariableValue::new_string("8".to_string()))
            .await
            .is_err()
    );
    assert_eq!(7, vm.variable_table.get_value(value_id).as_int());
}

#[test]
fn closed_enums_reject_numeric_operations_and_implicit_conversions() {
    for language in [350, 400] {
        for statement in [
            "value = 7",
            "INTEGER n = value",
            "PRINT value + 1",
            "PRINT value - value",
            "PRINT -value",
            "PRINT +value",
            "PRINT !value",
            "PRINT value < Shade.Second",
            "PRINT value = 7",
            "FOR value = Shade.First TO Shade.Second\n PRINT value\nNEXT",
            "PRINT ABS(value)",
            "PRINT TOBOOLEAN(value)",
            "PRINT TOSTRING(value)",
            "PRINT CHR(value)",
            "PRINT MID(\"hello\", value, 1)",
            "INTEGER items(1)\nPRINT items.Len(value)",
            "INTEGER items(1)\nitems.Redim(value)",
            "PRINT \"hello\".Mid(value, 1)",
        ] {
            let errors = compile_errors(&source(language, &format!("Shade value\n{statement}")));
            assert!(!errors.is_empty(), "accepted {statement} in {language}");
        }
    }
}

#[test]
fn closed_enums_reject_untyped_outputs() {
    for language in [350, 400] {
        for statement in ["POP value", "FREAD 1, value, 4", "INC value", "DEC value", "SORT values, values"] {
            let errors = compile_errors(&source(language, &format!("Shade value, values(1)\n{statement}")));
            assert!(errors.iter().any(|e| e.contains("enum") || e.contains("Shade")), "{statement}: {errors:?}");
        }
    }
}

#[test]
fn closed_enums_redim_clear_and_dynamic_locals_keep_first_member() {
    assert_eq!(
        "7|7|7|0,7|0,7|7",
        run_ppl(&source(
            400,
            r#"
TYPE Paint
 Shade Tone
ENDTYPE
Shade values[]
values.Redim(2)
PRINT values[2], "|"
values[0] = Shade.Second
REDIM values, 3
PRINT values[0], "|"
Paint paint = Paint { }
PRINT paint.Tone, "|"
Probe()
Probe()
PRINT values[99]
PROCEDURE Probe()
 Shade local[]
 PRINT local.Len(), ","
 local.Redim(1)
 PRINT local[1], "|"
 local[1] = Shade.Second
ENDPROC
"#
        ))
    );
}

#[test]
fn closed_enums_nominal_declare_contract_in_both_languages() {
    for language in [350, 400] {
        for signature in ["INTEGER value", "VAR Shade value", "Shade value(2)"] {
            let errors = compile_errors(&source(
                language,
                &format!("DECLARE PROCEDURE Use(Shade value)\nShade item\nUse(item)\nPROCEDURE Use({signature})\nENDPROC"),
            ));
            assert!(
                errors.iter().any(|e| e.contains("parameter") || e.contains("Parameter")),
                "{signature}: {errors:?}"
            );
        }
        let errors = compile_errors(&source(
            language,
            "DECLARE FUNCTION Echo(Shade value) Shade\nShade item = Echo(Shade.First)\nFUNCTION Echo(Shade value) INTEGER\nRETURN 7\nENDFUNC",
        ));
        assert!(errors.iter().any(|e| e.contains("return type") || e.contains("Return type")), "{errors:?}");
    }
}

#[test]
fn closed_enums_require_runtime400_for_storage_casts_and_signatures() {
    for language in [350, 400] {
        for body in [
            "Shade value\nPRINT value",
            "PRINT Shade(7)",
            "DECLARE PROCEDURE Use(Shade value)",
            "DECLARE FUNCTION Make() Shade",
            "PROCEDURE Use(Shade value)\nENDPROC",
            "FUNCTION Make() Shade\nENDFUNC",
        ] {
            let errors = compile_errors_with_runtime(&source(language, body), 340);
            assert!(errors.iter().any(|e| e.contains("runtime 400")), "{language}: {body}: {errors:?}");
        }
        assert!(compile_errors_with_runtime(&source(language, "PRINT Shade.First"), 340).is_empty());
    }
}

#[test]
fn open_enums_runtime_write_guard_is_atomic_and_nominal() {
    let executable = compile(&source(400, "Shade value\nPRINT value"));
    let table = &executable.variable_table;
    let (&id, _) = table.enums.iter().find(|(_, values)| *values == &vec![7, -3, 7]).unwrap();
    let kind = VariableType::UserData(id);
    assert_eq!(7, table.checked_enum_value(kind, VariableValue::new_int(7)).unwrap().as_int());
    for number in [0, 8, i32::MIN, i32::MAX] {
        assert_eq!(number, table.checked_enum_value(kind, VariableValue::new_int(number)).unwrap().as_int());
    }
    assert!(
        table
            .checked_enum_value(kind, VariableValue::new_enum(VariableType::UserData(id - 1), 7, 7))
            .is_err()
    );
    let array = VariableValue::new_vector(VariableType::Integer, vec![VariableValue::new_int(7), VariableValue::new_int(0)]);
    let checked = table.checked_enum_value(kind, array).unwrap();
    assert_eq!(0, checked.get_array_value(1, 0, 0).as_int());
    let other = VariableValue::new_enum(VariableType::UserData(id - 1), 0, 0);
    let array = VariableValue::new_vector(kind, vec![VariableValue::new_enum(kind, 7, 7), other]);
    assert!(table.checked_enum_value(kind, array).is_err());
    let value = table.checked_enum_value(kind, VariableValue::new_int(-3)).unwrap().emptied();
    assert_eq!(7, value.as_int());
    assert!(matches!(value.generic_data, GenericVariableData::Enum(7)));
}

#[test]
fn closed_enums_regex_combinations_are_checked_bitwise_values() {
    assert_eq!(
        "3|1",
        run_ppl(
            "RegexOptions options = RegexOptions.IgnoreCase | RegexOptions.MultiLine\nPRINT TOINTEGER(options), \"|\", Regex.Compile(\"^a\", options).IsMatch(\"A\")"
        )
    );
    assert!(compile_errors("PRINT RegexOptions.IgnoreCase | RegexOptions.MultiLine").is_empty());
}

// Put a non-enum field first: a bad leaf later in the record must not publish it.
const ENUM_RECORDS: &str = r#"
TYPE Paint
 INTEGER Serial
 Shade Tone
 Shade Vector(1)
 Shade Matrix(1,1)
 Shade Cube(1,1,1)
ENDTYPE
TYPE Parcel
 INTEGER Serial
 Paint Single
 Paint Items(1)
ENDTYPE
"#;

#[test]
fn closed_enums_record_codecs_round_trip_nested_fields_and_all_array_ranks() {
    for (write, read) in [("FPUTREC", "FGETREC"), ("FWRITEREC", "FREADREC")] {
        let body = format!(
            r#"
{ENUM_RECORDS}
Parcel original
original.Serial = 99
original.Single.Tone = Shade.Second
original.Single.Vector(1) = Shade.Second
original.Single.Matrix(1,1) = Shade.Second
original.Single.Cube(1,1,1) = Shade.Second
original.Items(1) = original.Single
FCREATE 1, "enum.dat", O_WR, S_DN
{write} 1, original
PRINT FERR(1), "|"
FCLOSE 1
Parcel loaded
FOPEN 1, "enum.dat", O_RD, S_DN
{read} 1, loaded
PRINT FERR(1), "|", loaded = original, "|"
PRINT loaded.Single.Tone, ",", loaded.Single.Vector(1), ",", loaded.Single.Matrix(1,1), ",", loaded.Single.Cube(1,1,1), "|"
PRINT loaded.Items(1).Tone, ",", loaded.Items(0).Tone
FCLOSE 1
"#
        );
        assert_eq!("0|0|1|-3,-3,-3,-3|-3,7", run_ppl(&source(400, &body)), "{read}");
    }
}

#[test]
fn open_enums_record_reads_preserve_each_unnamed_leaf() {
    // Paint is 16 signed 32-bit leaves: Serial, Tone, 2+4+8 array elements.
    let paint: Vec<i32> = std::iter::once(99).chain(std::iter::repeat_n(-3, 15)).collect();
    for (read, binary) in [("FGETREC", false), ("FREADREC", true)] {
        for record_type in ["Paint", "Parcel"] {
            let good: Vec<i32> = if record_type == "Paint" {
                paint.clone()
            } else {
                std::iter::once(99).chain(paint.iter().copied().cycle().take(48)).collect()
            };
            for bad_leaf in (0..good.len()).filter(|index| good[*index] == -3) {
                let mut values = good.clone();
                values[bad_leaf] = 8;
                let bytes = if binary {
                    let mut frame = ((values.len() * 4) as u32).to_le_bytes().to_vec();
                    frame.extend(values.iter().flat_map(|value| value.to_le_bytes()));
                    frame
                } else {
                    values.iter().map(|value| format!("{value}\n")).collect::<String>().into_bytes()
                };
                // Exercise direct, nested and record-array fields, including COW aliases.
                let body = format!(
                    r#"
{ENUM_RECORDS}
{record_type} value
value.Serial = 42
{record_type} before = value
FOPEN 1, "bad.dat", O_RD, S_DN
{read} 1, value
PRINT value = before, "|", value.Serial, "|", before.Serial, "|", FERR(1), "|"
FCLOSE 1
FCREATE 1, "copy.dat", O_WR, S_DN
FPUTREC 1, value
FCLOSE 1
FOPEN 1, "copy.dat", O_RD, S_DN
STRING line
INTEGER index
FOR index = 0 TO {bad_leaf}
 FGET 1, line
NEXT
PRINT line
FCLOSE 1
"#
                );
                assert_eq!(
                    "0|99|42|0|8",
                    super::run_ppl_with_files(&source(400, &body), &[("bad.dat", &bytes)]),
                    "{read}, {record_type}, leaf {bad_leaf}"
                );
            }
        }
    }
}

#[test]
fn closed_enums_array_literals_copies_and_empty_foreach_preserve_domains() {
    assert_eq!(
        "-3,7,7|7|-3,7|0,7|0,7",
        run_ppl(&source(
            400,
            r#"
TYPE Paint
 Shade Tones(1)
ENDTYPE
Shade values[] = { Shade.Second, Shade.Alias }
Shade copied[1]
copied = values
values[0] = Shade.First
PRINT copied[0], ",", copied[1], ",", copied[99], "|"
Paint paint = Paint { Tones = copied }
copied[0] = Shade.First
PRINT copied[0], "|", paint.Tones(0), ",", paint.Tones(1), "|"
Shade empty[] = {}
Shade current
INTEGER visits
FOREACH current IN empty
 visits = visits + 1
ENDFOREACH
PRINT visits, ",", current, "|"
copied = empty
PRINT copied.Len(), ",", copied[0]
"#
        ))
    );
    for body in [
        "Shade values[] = { Shade.First, 8 }",
        "Shade values[]\nINTEGER integers[] = { 7 }\nvalues = integers",
        "ENUM Other\n First = 7\nENDENUM\nShade values[] = { Other.First }",
    ] {
        let errors = compile_errors(&source(400, body));
        assert!(
            errors
                .iter()
                .any(|error| error.contains("enum") || error.contains("Shade") || error.starts_with("Record array field '' expects UserData")),
            "{body}: {errors:?}"
        );
    }
}

#[test]
fn closed_enums_record_array_redim_uses_nested_enum_defaults() {
    for redim in ["records.Redim(1)", "REDIM records, 1"] {
        let body = format!(
            r#"
{ENUM_RECORDS}
Parcel records[]
{redim}
PRINT records[1].Single.Tone, ",", records[1].Items(1).Cube(1,1,1), "|"
records[1].Single.Tone = Shade.Second
{redim}
PRINT records[1].Single.Tone, ",", records[1].Items(1).Vector(1)
"#
        );
        assert_eq!("7,7|7,7", run_ppl(&source(400, &body)), "{redim}");
    }
}

#[test]
fn closed_enums_recursive_frames_restore_scalar_array_and_result_domains() {
    assert_eq!(
        "-3,-3,7|7,7,7|-3,-3,7|-3",
        run_ppl(&source(
            400,
            r#"
PRINT Work(2)
FUNCTION Work(INTEGER depth) Shade
 Shade local = Shade.Second
 Shade values[] = { Shade.Second }
 IF depth = 1 THEN
  local = Shade.First
  values[0] = Shade.First
 ENDIF
 Work = local
 IF depth > 0 THEN
  Shade child = Work(depth - 1)
 ENDIF
 PRINT local, ",", values[0], ",", values[99], "|"
ENDFUNC
"#
        ))
    );
}

#[test]
fn closed_enums_empty_declarations_and_constant_operations_are_checked() {
    for language in [350, 400] {
        let errors = compile_errors(&format!(";$LANGVERSION {language}\nENUM Empty\nENDENUM\nPRINT 1"));
        assert!(errors.iter().any(|error| error.to_lowercase().contains("enum")), "{errors:?}");
        assert_eq!(
            "7|1",
            run_ppl(&source(
                language,
                "CONST Shade Chosen = Shade.Alias\nShade value = Chosen\nPRINT value, \"|\", value = Shade.First"
            ))
        );
        for expression in [
            "Shade.First + 1",
            "-Shade.First",
            "Shade.First < Shade.Second",
            "Shade.First = 7",
            "Shade.First | Shade.Second",
        ] {
            let errors = compile_errors(&source(language, &format!("CONST INTEGER Bad = {expression}\nPRINT Bad")));
            assert!(
                errors
                    .iter()
                    .any(|error| error.to_lowercase().contains("enum") || error.contains("Can't assign Shade to Integer")),
                "{language}, {expression}: {errors:?}"
            );
        }
    }
}

#[test]
fn closed_enums_builtin_integer_properties_can_be_stored_and_compared_both_ways() {
    assert_eq!(
        "1,1,1|1,1|1,1",
        super::run_ppl_with_input(
            r#"
EVENT event = Terminal.Input.Poll()
EventKind kind = event.Kind
PRINT kind = EventKind.Key, ",", event.Kind = EventKind.Key, ",", EventKind.Key = event.Kind, "|"
GfxBackend backend = Terminal.Gfx.Backend
PRINT backend = GfxBackend.None, ",", GfxBackend.None = Terminal.Gfx.Backend, "|"
GfxBackend defaultBackend
PRINT defaultBackend = GfxBackend.None, ",", defaultBackend = backend
"#,
            b"a"
        )
    );
}
