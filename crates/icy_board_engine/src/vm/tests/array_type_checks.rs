use super::{compile_errors, run_ppl};

const WHOLE_ARRAY: &str = "Whole arrays cannot be used as scalar values; index an element first";
const REDIM_VARIABLE: &str = "REDIM requires an assignable array variable";

fn with_array_function(body: &str) -> String {
    format!(
        ";$LANGVERSION 400\n{body}\n\
         FUNCTION MakeValues() INTEGER[]\n\
             INTEGER result[] = {{ 17, 23 }}\n\
             RETURN result\n\
         ENDFUNC\n"
    )
}

#[test]
fn array_results_are_rejected_in_scalar_contexts() {
    for body in [
        "PRINT MakeValues()",
        "PRINT ((MakeValues()))",
        "PRINT MakeValues() + 1",
        "PRINT 1 + (MakeValues())",
        "PRINT -MakeValues()",
        "PRINT ABS(MakeValues())",
        "INTEGER value = MakeValues()",
        "INTEGER value\nvalue = (MakeValues())",
        "IF MakeValues() PRINT 1",
        "WHILE MakeValues() PRINT 1",
        "INTEGER i\nFOR i = MakeValues() TO 1\nNEXT",
        "INTEGER i\nFOR i = 0 TO MakeValues()\nNEXT",
        "INTEGER i\nFOR i = 0 TO 1 STEP MakeValues()\nNEXT",
        "SELECT CASE MakeValues()\nCASE 1\nENDSELECT",
        "SELECT CASE 1\nCASE MakeValues()\nENDSELECT",
        "INTEGER values[1]\nPRINT values[MakeValues()]",
        "INTEGER values[1]\nvalues[MakeValues()] = 1",
        "INTEGER values[]\nREDIM values, MakeValues()",
        "INTEGER values[]\nvalues.Redim(MakeValues())",
        "INTEGER values[]\nPRINT values.Len(MakeValues())",
        "INTEGER values[]\nPRINT Len(values, MakeValues())",
        "Take(MakeValues())\nPROCEDURE Take(INTEGER value)\nENDPROC",
        "PRINT Take(MakeValues())\nFUNCTION Take(INTEGER value) INTEGER\nRETURN value\nENDFUNC",
        "PRINT Scalar()\nFUNCTION Scalar() INTEGER\nRETURN MakeValues()\nENDFUNC",
        "PRINT Scalar()\nFUNCTION Scalar() INTEGER\nScalar = MakeValues()\nENDFUNC",
        "TYPE Rec\nINTEGER Value\nENDTYPE\nRec r = Rec { Value = MakeValues() }",
        "PRINT \"abc\".Left(MakeValues())",
        "PRINT String.Repeat(\"x\", MakeValues())",
        "PRINT String.Split(\"a,b\", MakeValues()).Len()",
        "PRINT String.Join(String.Split(\"a,b\", \",\"), MakeValues())",
        "PRINT Bytes.FromBase64(MakeValues())",
        "PRINT MakeValues()[MakeValues()]",
        "PRINT (\"abc\")[MakeValues()]",
        "TYPE Rec\nINTEGER Values[1]\nENDTYPE\nRec r\nPRINT r.Values[MakeValues()]",
    ] {
        let errors = compile_errors(&with_array_function(body));
        assert!(errors.iter().any(|error| error == WHOLE_ARRAY), "{body}\n{errors:?}");
    }
}

#[test]
fn member_array_results_and_properties_are_not_scalars() {
    for expression in [
        "\"a,b\".Split(\",\")",
        "(String.Split(\"a,b\", \",\"))",
        "Regex.Compile(\",\").Split(\"a,b\")",
        "Board.Users",
        "(Session.User.Notes)",
        "\"a,b\".Split(\",\").Upper()",
        "Board.Users.Name",
    ] {
        let errors = compile_errors(&format!(";$LANGVERSION 400\nPRINT {expression}"));
        assert!(errors.iter().any(|error| error == WHOLE_ARRAY), "{expression}\n{errors:?}");
    }
}

#[test]
fn array_callback_results_are_not_scalars() {
    let source = with_array_function("Apply(MakeValues)\nPROCEDURE Apply(FUNCTION callback() INTEGER[])\nPRINT (callback())\nENDPROC");
    assert_eq!(vec![WHOLE_ARRAY], compile_errors(&source));
}

#[test]
fn bare_identifiers_keep_legacy_missing_subscript_diagnostics() {
    for language in [330, 350, 400] {
        for (bounds, rank) in [("1", 1), ("1, 2", 2), ("1, 2, 3", 3)] {
            for expression in ["values", "((values))"] {
                let source = format!(";$LANGVERSION {language}\nINTEGER values({bounds})\nPRINT {expression}");
                assert_eq!(vec![format!("Not enough arguments passed (values:0:{rank})")], compile_errors(&source));
            }
        }
    }
}

#[test]
fn whole_array_results_remain_legal_array_operands() {
    assert_eq!(
        "2 2 17 23 a|b",
        run_ppl(&with_array_function(
            r#"
INTEGER values[] = (MakeValues())
PRINT MakeValues().Len(), " ", Len((MakeValues()), 0), " "
INTEGER value
FOREACH value IN (MakeValues())
    PRINT value, " "
ENDFOREACH
PRINT String.Join(String.Split("a,b", ","), "|")
"#
        ))
    );
    assert!(compile_errors(";$LANGVERSION 400\nINTEGER values[2], indices[2]\nSORT values, indices").is_empty());
}

#[test]
fn redim_requires_exactly_the_declared_rank_in_both_forms() {
    for (declaration, rank) in [("[]", 1), ("[,]", 2), ("[,,]", 3), ("[1]", 1), ("[1, 1]", 2), ("[1, 1, 1]", 3)] {
        for (bounds, given) in [("2", 1), ("2, 3", 2), ("2, 3, 4", 3)] {
            for statement in [format!("REDIM values, {bounds}"), format!("values.Redim({bounds})")] {
                let source = format!(";$LANGVERSION 400\nINTEGER values{declaration}\n{statement}");
                let errors = compile_errors(&source);
                if rank == given {
                    assert!(errors.is_empty(), "{source}\n{errors:?}");
                } else {
                    assert_eq!(
                        vec![format!("REDIM cannot change declared rank {rank}; got {given} bounds")],
                        errors,
                        "{source}"
                    );
                }
            }
        }
    }
    assert_eq!(
        vec!["REDIM cannot change declared rank 1; got 2 bounds"],
        compile_errors(";$LANGVERSION 400\nINTEGER values[] = { 1, 2 }\nvalues.Redim(2, 3)")
    );
}

#[test]
fn rank_preserving_redim_executes_at_every_rank_in_both_forms() {
    for (rank, bounds, indices, count) in [("[]", "2", "2", 3), ("[,]", "2, 3", "2, 3", 12), ("[,,]", "2, 3, 4", "2, 3, 4", 60)] {
        for statement in [format!("REDIM values, {bounds}"), format!("values.Redim({bounds})")] {
            let source = format!(";$LANGVERSION 400\nINTEGER values{rank}\n{statement}\nvalues[{indices}] = 42\nPRINT values.Len(), \" \", values[{indices}]");
            assert_eq!(format!("{count} 42"), run_ppl(&source), "{source}");
        }
    }
    assert_eq!("3", run_ppl(";$LANGVERSION 400\nINTEGER values[]\nREDIM (values), 2\nPRINT values.Len()"));
}

#[test]
fn redim_rejects_temporary_arrays_and_read_only_properties() {
    for target in [
        "MakeValues()",
        "(MakeValues())",
        "\"a,b\".Split(\",\")",
        "String.Split(\"a,b\", \",\")",
        "Regex.Compile(\",\").Split(\"a,b\")",
        "Board.Users",
        "Session.User.Notes",
    ] {
        for statement in [format!("REDIM {target}, 2"), format!("{target}.Redim(2)")] {
            // Only identifier-led member statements are accepted by the parser.
            if !statement.starts_with("REDIM ") && !target.starts_with(char::is_alphabetic) {
                continue;
            }
            let errors = compile_errors(&with_array_function(&statement));
            assert!(errors.iter().any(|error| error == REDIM_VARIABLE), "{statement}\n{errors:?}");
        }
    }
}

#[test]
fn redim_requires_a_whole_array_variable_not_a_scalar_or_element() {
    for target in ["value", "values[0]", "values(0)", "1", "(1 + 2)"] {
        let source = format!(";$LANGVERSION 400\nINTEGER value, values[1]\nREDIM {target}, 2");
        let errors = compile_errors(&source);
        assert!(errors.iter().any(|error| error == REDIM_VARIABLE), "{source}\n{errors:?}");
    }
}

#[test]
fn fixed_record_fields_remain_prohibited_redim_targets() {
    for target in ["record.Values", "(record.Values)", "records[0].Values"] {
        for statement in [format!("REDIM {target}, 2"), format!("{target}.Redim(2)")] {
            if !statement.starts_with("REDIM ") && target.starts_with('(') {
                continue;
            }
            let source = format!(";$LANGVERSION 400\nTYPE Rec\nINTEGER Values[1]\nENDTYPE\nRec record, records[1]\n{statement}");
            assert_eq!(
                vec!["Record array field 'Values' has a fixed size and cannot be redimensioned"],
                compile_errors(&source),
                "{source}"
            );
        }
    }
}

#[test]
fn legacy_redim_can_still_change_rank() {
    // 3.40 has the statement but not member-call statement syntax.
    assert_eq!("0", run_ppl(";$LANGVERSION 340\nINTEGER values(1)\nREDIM values, 2, 3\nPRINT values(0)"));
    for statement in ["REDIM values, 2, 3", "values.Redim(2, 3)"] {
        let source = format!(";$LANGVERSION 350\nINTEGER values(1)\n{statement}\nPRINT values.Len(0), \" \", values.Len(1)");
        assert_eq!("3 4", run_ppl(&source), "{source}");
    }
}
