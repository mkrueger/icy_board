use super::{compile_errors, compile_errors_with_runtime, run_ppl};

#[test]
fn array_parameters_copy_values_without_mutating_the_caller() {
    for parameter in ["INTEGER values[]", "INTEGER values[9]"] {
        assert_eq!(
            "2 7 8 7 2",
            run_ppl(&format!(
                r#"
INTEGER data[] = {{ 7, 8 }}
Show(data)
PRINT data[0], " ", data.Len()
PROCEDURE Show({parameter})
PRINT values.Len(), " ", values[0], " ", values[1], " "
values[0] = 99
values.Redim(4)
ENDPROC
"#
            ))
        );
    }
}

#[test]
fn array_parameters_accept_empty_and_computed_values() {
    assert_eq!(
        "0 2 7 8 ",
        run_ppl(
            r#"
INTEGER empty[]
Show(empty)
Show(Make())
FUNCTION Make() INTEGER[]
INTEGER result[] = { 7, 8 }
RETURN result
ENDFUNC
PROCEDURE Show(INTEGER values[])
PRINT values.Len(), " "
INTEGER value
FOREACH value IN values
PRINT value, " "
NEXT
ENDPROC
"#
        )
    );
}

#[test]
fn var_array_parameters_copy_back_values_and_bounds() {
    assert_eq!(
        "3 9 8 7",
        run_ppl(
            r#"
INTEGER data[] = { 1 }
Change(data)
PRINT data.Len(), " ", data[0], " ", data[1], " ", data[2]
PROCEDURE Change(VAR INTEGER values[])
INTEGER replacement[] = { 9, 8, 7 }
values = replacement
ENDPROC
"#
        )
    );
}

#[test]
fn array_parameters_preserve_rank_two_and_three() {
    for (rank, bounds, index) in [("[,]", "[1, 2]", "[1, 2]"), ("[,,]", "[1, 2, 3]", "[1, 2, 3]")] {
        assert_eq!(
            "7 9",
            run_ppl(&format!(
                r#"
INTEGER data{bounds}
data{index} = 7
Show(data)
Change(data)
PRINT data{index}
PROCEDURE Show(INTEGER values{rank})
PRINT values{index}, " "
ENDPROC
PROCEDURE Change(VAR INTEGER values{rank})
values{index} = 9
ENDPROC
"#
            ))
        );
    }
}

#[test]
fn array_parameters_work_in_recursive_functions_and_callbacks() {
    assert_eq!(
        "7 7",
        run_ppl(
            r#"
INTEGER data[] = { 7, 8 }
PRINT Forward(Read, data), " ", Read(data, 2)[0]
FUNCTION Forward(FUNCTION callback(INTEGER values[], INTEGER depth) INTEGER[], INTEGER values[]) INTEGER
INTEGER result[] = callback(values, 1)
RETURN result[0]
ENDFUNC
FUNCTION Read(INTEGER values[], INTEGER depth) INTEGER[]
IF depth > 0 THEN
INTEGER nested[] = Read(values, depth - 1)
nested[0] = 99
ENDIF
RETURN values
ENDFUNC
"#
        )
    );
}

#[test]
fn procedure_callbacks_forward_var_array_parameters() {
    assert_eq!(
        "2 9",
        run_ppl(
            r#"
INTEGER data[]
RelayArray(Change, data)
PRINT data.Len(), " ", data[0]
PROCEDURE RelayArray(PROCEDURE callback(VAR INTEGER values[]), VAR INTEGER values[])
callback(values)
ENDPROC
PROCEDURE Change(VAR INTEGER values[])
values.Redim(1)
values[0] = 9
ENDPROC
"#
        )
    );
}

#[test]
fn array_parameters_reject_scalars_wrong_rank_and_wrong_type() {
    for source in [
        "Show(7)\nPROCEDURE Show(INTEGER values[])\nENDPROC",
        "INTEGER data[1, 1]\nShow(data)\nPROCEDURE Show(INTEGER values[])\nENDPROC",
        "STRING data[1]\nShow(data)\nPROCEDURE Show(INTEGER values[])\nENDPROC",
        "INTEGER data[1]\nShow(data)\nPROCEDURE Show(INTEGER value)\nENDPROC",
        "Show(Make())\nPROCEDURE Show(VAR INTEGER values[])\nENDPROC\nFUNCTION Make() INTEGER[]\nENDFUNC",
    ] {
        assert!(!compile_errors(source).is_empty(), "{source}");
    }
}

#[test]
fn array_parameters_require_runtime_400() {
    let source = ";$LANGVERSION 400\nPROCEDURE Show(INTEGER values[1])\nENDPROC";
    assert!(!compile_errors_with_runtime(source, 340).is_empty());
}

#[test]
fn record_array_parameters_copy_nested_values_and_write_back_var_changes() {
    assert_eq!(
        "first 7 first 7 changed 9",
        run_ppl(
            r#"
TYPE Item
    STRING name
    INTEGER numbers[1]
ENDTYPE
Item data[1]
data[0].name = "first"
data[0].numbers(1) = 7
Show(data)
PRINT data[0].name, " ", data[0].numbers(1), " "
Change(data)
PRINT data[0].name, " ", data[0].numbers(1)
PROCEDURE Show(Item values[])
PRINT values[0].name, " ", values[0].numbers(1), " "
values[0].name = "local"
values[0].numbers(1) = 88
ENDPROC
PROCEDURE Change(VAR Item values[])
values[0].name = "changed"
values[0].numbers(1) = 9
ENDPROC
"#
        )
    );
}

#[test]
fn string_array_parameters_preserve_capacity_and_computed_elements() {
    assert_eq!(
        "2 3000 tail 3000 changed a|b",
        run_ppl(
            r#"
STRING data[] = { STRING.Repeat("x", 3000), "tail" }
Show(data)
PRINT data[0].Len(), " "
Change(data)
PRINT data[0], " "
PrintWords((String.Split("a,b", ",")))
PROCEDURE Show(STRING values[])
PRINT values.Len(), " ", values[0].Len(), " ", values[1], " "
values[0] = "local"
ENDPROC
PROCEDURE Change(VAR STRING values[])
values[0] = "changed"
ENDPROC
PROCEDURE PrintWords(BIGSTR values[])
PRINT String.Join(values, "|")
ENDPROC
"#
        )
    );
}

#[test]
fn enum_array_parameters_keep_enum_elements() {
    assert_eq!(
        "1 1 1",
        run_ppl(
            r#"
StringComparison data[] = { StringComparison.Ordinal, StringComparison.OrdinalIgnoreCase }
Show(data)
PRINT data[0] == StringComparison.Ordinal, " "
Change(data)
PRINT data[0] == StringComparison.OrdinalIgnoreCase
PROCEDURE Show(StringComparison values[])
PRINT values[1] == StringComparison.OrdinalIgnoreCase, " "
values[0] = StringComparison.OrdinalIgnoreCase
ENDPROC
PROCEDURE Change(VAR StringComparison values[])
values[0] = StringComparison.OrdinalIgnoreCase
ENDPROC
"#
        )
    );
}

#[test]
fn value_array_parameters_accept_fixed_record_fields_and_parentheses() {
    assert_eq!(
        "2 7 8 2 7 8 7 8",
        run_ppl(
            r#"
TYPE Pair
    INTEGER values[1]
ENDTYPE
Pair record
record.values(0) = 7
record.values(1) = 8
Show(record.values)
Show(((record.values)))
PRINT record.values[0], " ", record.values[1]
PROCEDURE Show(INTEGER values[])
PRINT values.Len(), " ", values[0], " ", values[1], " "
values[0] = 99
values.Redim(4)
ENDPROC
"#
        )
    );
}

#[test]
fn value_array_parameters_accept_property_snapshots() {
    assert_eq!(
        "5 old new",
        run_ppl(
            r#"
Session.User.SetNote(1, "old")
Show((Session.User.Notes))
PRINT Session.User.Notes[1]
PROCEDURE Show(STRING values[])
Session.User.SetNote(1, "new")
PRINT values.Len(), " ", values[1], " "
values[1] = "local"
values.Redim(0)
ENDPROC
"#
        )
    );
}

#[test]
fn declared_array_parameters_execute_with_value_and_var_semantics() {
    assert_eq!(
        "7 2 9",
        run_ppl(
            r#"
DECLARE FUNCTION Read(INTEGER values[]) INTEGER
DECLARE PROCEDURE Change(VAR INTEGER values[])
INTEGER data[] = { 7 }
PRINT Read((data)), " "
Change(data)
PRINT data.Len(), " ", data[1]
FUNCTION Read(INTEGER values[]) INTEGER
RETURN values[0]
ENDFUNC
PROCEDURE Change(VAR INTEGER values[])
INTEGER replacement[] = { 8, 9 }
values = replacement
ENDPROC
"#
        )
    );
}

#[test]
fn array_callback_parameter_rank_mismatches_are_rejected() {
    for source in [
        "Apply(Work)\nPROCEDURE Apply(PROCEDURE callback(INTEGER values[]))\nENDPROC\nPROCEDURE Work(INTEGER values[,])\nENDPROC",
        "Relay(Work)\nPROCEDURE Relay(PROCEDURE source(INTEGER values[,]))\nApply(source)\nENDPROC\nPROCEDURE Apply(PROCEDURE callback(INTEGER values[]))\nENDPROC\nPROCEDURE Work(INTEGER values[,])\nENDPROC",
        "DECLARE PROCEDURE Apply(PROCEDURE callback(INTEGER values[]))\nPROCEDURE Apply(PROCEDURE callback(INTEGER values[,]))\nENDPROC",
    ] {
        let errors = compile_errors(source);
        assert!(errors.iter().any(|error| error.contains("parameters not match")), "{source}\n{errors:?}");
    }
}

#[test]
fn multiple_array_arguments_are_evaluated_left_to_right_and_snapshotted() {
    assert_eq!(
        "M 1 5 9A1S2A3 1 2 3",
        run_ppl(
            r#"
INTEGER data[] = { 1 }
Show(data, Mutate(), data)
Show(Make(1), Mark(), (Make(3)))
FUNCTION Mutate() INTEGER
PRINT "M"
data[0] = 9
RETURN 5
ENDFUNC
FUNCTION Make(INTEGER value) INTEGER[]
PRINT "A", value
INTEGER result[] = { value }
RETURN result
ENDFUNC
FUNCTION Mark() INTEGER
PRINT "S2"
RETURN 2
ENDFUNC
PROCEDURE Show(INTEGER first[], INTEGER middle, INTEGER last[])
PRINT " ", first[0], " ", middle, " ", last[0]
ENDPROC
"#
        )
    );
}

#[test]
fn aliased_var_array_arguments_copy_in_independently_and_first_copy_out_wins() {
    // Preserve the existing reverse-order VAR copy-out rule, not reference aliasing.
    assert_eq!(
        "7 2 10 11",
        run_ppl(
            r#"
INTEGER data[] = { 7 }
Change(data, data)
PRINT data.Len(), " ", data[0], " ", data[1]
PROCEDURE Change(VAR INTEGER first[], VAR INTEGER second[])
first[0] = 10
PRINT second[0], " "
INTEGER firstReplacement[] = { 10, 11 }
INTEGER secondReplacement[] = { 20, 21, 22 }
first = firstReplacement
second = secondReplacement
ENDPROC
"#
        )
    );
}

#[test]
fn recursive_var_array_parameters_restore_frames_and_propagate_bounds() {
    assert_eq!(
        "2 33 99 2 54 99",
        run_ppl(
            r#"
INTEGER data[] = { 0 }
Work(data, 2)
PRINT data.Len(), " ", data[0], " ", data[1], " "
Work(data, 1)
PRINT data.Len(), " ", data[0], " ", data[1]
PROCEDURE Work(VAR INTEGER values[], INTEGER depth)
values[0] = values[0] + depth
IF depth > 0 THEN
Work(values, depth - 1)
ELSE
INTEGER replacement[] = { values[0], 99 }
values = replacement
ENDIF
values[0] = values[0] + 10
ENDPROC
"#
        )
    );
}

#[test]
fn var_array_parameters_resize_bounded_callers_in_both_directions() {
    assert_eq!(
        "3 9 0",
        run_ppl(
            r#"
INTEGER data[0]
Change(data, TRUE)
PRINT data.Len(), " ", data[2], " "
Change(data, FALSE)
PRINT data.Len()
PROCEDURE Change(VAR INTEGER values[9], BOOLEAN grow)
IF grow THEN
values.Redim(2)
values[2] = 9
ELSE
INTEGER empty[]
values = empty
ENDIF
ENDPROC
"#
        )
    );
}

#[test]
fn legacy_array_parameter_syntax_preserves_headers_but_only_400_marks_whole_array_parameters() {
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use crate::{
        compiler::{PPECompiler, workspace::Workspace},
        executable::{EntryType, Executable},
        parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    };

    for (language, runtime) in [(340, 340), (340, 400), (350, 350), (350, 400), (400, 400)] {
        let call = if language < 400 { "Show(7)" } else { "INTEGER data[1]\nShow(data)" };
        let source = format!(";$LANGVERSION {language}\n{call}\nPROCEDURE Show(INTEGER values(1))\nENDPROC");
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let registry = UserTypeRegistry::icy_board_registry();
        let mut workspace = Workspace::default();
        workspace.hard_coded_files = Some(vec![PathBuf::from("test.pps")]);
        workspace.package.runtime = Some(runtime);
        workspace.set_default_language_version(Some(language));
        let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);
        let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
        compiler.compile(&[&ast]);
        let diagnostics = errors.lock().unwrap();
        assert!(
            !diagnostics.has_errors(),
            "language={language}, runtime={runtime}: {:?}",
            diagnostics.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
        );
        drop(diagnostics);
        let mut bytes = compiler.create_executable().unwrap().to_buffer().unwrap();
        let executable = Executable::from_buffer(&mut bytes, false).unwrap();
        let parameters: Vec<_> = executable
            .variable_table
            .get_entries()
            .iter()
            .filter(|entry| entry.get_type() == EntryType::Parameter)
            .collect();
        assert_eq!(1, parameters.len());
        let header = &parameters[0].header;
        assert_eq!(1, header.dim, "language={language}, runtime={runtime}");
        assert_eq!(1, header.vector_size);
        let expected_flags = if language >= 400 {
            crate::executable::variable_table::VARIABLE_FLAG_ARRAY_PARAMETER
        } else {
            0
        };
        assert_eq!((0, 0, expected_flags), (header.matrix_size, header.cube_size, header.flags));
    }
}
