//! Nominal, checked integer bitwise operators (not the classic logical AND/OR).
use super::{compile, compile_errors, compile_errors_with_runtime, run_ppl};

const SPARSE: &str = "ENUM Bits\n One = 1\n Two = 2\nENDENUM\n";
const COMPOSITE: &str = "ENUM Bits\n Both = 3\n One = 1\n Two = 2\n Zero = 0\n Alias = 3\nENDENUM\n";

#[test]
fn enum_bitwise_fatal_errors_with_onerror_preserve_assignment_targets() {
    for statement in [
        "a = a | b",
        "a |= b",
        "a &= b",
        "items(0) |= b",
        "items(0) &= b",
        "box.Value |= b",
        "box.Value &= b",
    ] {
        let body = format!(
            r#"
TYPE Boxed
 Bits Value
ENDTYPE
Bits a = Bits.One, b = Bits.Two
Bits items(1)
Boxed box
ONERROR GOTO Failed
{statement}
PRINT "escaped"
EXIT
:Failed
PRINT a, "|", items(0), "|", box.Value, "|", Error.Last().OK
"#
        );
        // Domain violations are fatal VM errors, not recoverable subsystem
        // errors. Even an armed ONERROR must not let a fallback value escape.
        let error = checked_run_inspecting(&source(400, SPARSE, &body), |table| {
            use crate::executable::GenericVariableData;
            // PPE serialization replaces source names, so inspect storage by
            // shape; entry kinds are also inferred only by the decompiler.
            // The only enum scalars are a=1 and b=2, in that order.
            let variables = table.get_entries();
            let scalars: Vec<_> = variables
                .iter()
                .filter(|entry| matches!(entry.value.generic_data, GenericVariableData::Enum(_)))
                .map(|entry| entry.value.as_int())
                .collect();
            assert_eq!(vec![1, 2], scalars, "{statement}");
            let items = variables
                .iter()
                .find(|entry| matches!(entry.value.generic_data, GenericVariableData::Dim1(_)))
                .unwrap();
            assert_eq!(1, items.value.get_array_value(0, 0, 0).as_int(), "{statement}");
            let fields = variables
                .iter()
                .find_map(|entry| match &entry.value.generic_data {
                    GenericVariableData::Record(fields) => Some(fields),
                    _ => None,
                })
                .unwrap();
            assert_eq!(1, fields[0].as_int(), "{statement}");
        })
        .unwrap_err();
        assert!(
            error.contains("not a member of closed enum") && error.ends_with("output="),
            "{statement}: {error}"
        );
    }
}

#[test]
fn enum_bitwise_evaluated_boolean_and_arithmetic_identities_keep_checks() {
    for expression in ["FALSE & ((a | b) = a)", "TRUE | ((a & b) = a)", "0 * TOINTEGER(a | b)", "TOINTEGER(a & b) * 0"] {
        let body = format!("Bits a = Bits.One, b = Bits.Two\nPRINT {expression}");
        let error = checked_run(&source(400, SPARSE, &body)).unwrap_err();
        assert!(error.contains("not a member of closed enum"), "{expression}: {error}");
    }
}

fn source(language: u16, domain: &str, body: &str) -> String {
    format!(";$LANGVERSION {language}\n{domain}{body}\n")
}

#[test]
fn enum_bitwise_bits_domains_defaults_and_classic_logic() {
    for language in [350, 400] {
        assert_eq!(
            "3|3|0|2|3|1|1|1|0",
            run_ppl(&source(
                language,
                COMPOSITE,
                r#"
Bits defaultValue, a = Bits.One, b = Bits.Two
PRINT defaultValue, "|", a | b, "|", a & b, "|", (a | b) & b, "|"
PRINT Bits.One | Bits.Two, "|", Bits.Both = (a | b), "|", a <> b, "|", 2 | 4, "|", 2 & 0
"#
            ))
        );
        assert!(
            compile_errors_with_runtime(&source(language, COMPOSITE, "PRINT Bits.One | Bits.Two"), 340)
                .iter()
                .any(|e| e.contains("runtime 400"))
        );
    }
}

#[test]
fn enum_bitwise_known_invalid_results_are_errors_in_every_context() {
    for language in [350, 400] {
        for expression in [
            "Bits.One | Bits.Two",
            "Bits.One & Bits.Two",
            "(Bits.One | Bits.Two) & Bits.One",
            "Bits(1) | Bits(2)",
        ] {
            for body in [
                format!("PRINT {expression}"),
                format!("PRINT TOINTEGER({expression})"),
                format!("PRINT ({expression}) = Bits.One"),
                format!("IF (({expression}) = Bits.One) PRINT 1"),
                format!("CONST Bits Bad = {expression}\nPRINT Bad"),
                format!("Bits bad = {expression}\nPRINT bad"),
            ] {
                let errors = compile_errors(&source(language, SPARSE, &body));
                assert!(errors.iter().any(|e| e.contains("not a declared member")), "{language}: {body}: {errors:?}");
            }
        }
        for expression in [
            "a | 1",
            "1 & a",
            "a | Other.One",
            "a & Other.One",
            "a = Other.One",
            "a <> 1",
            "a + a",
            "-a",
            "!a",
        ] {
            let body = format!("ENUM Other\nOne = 1\nENDENUM\nBits a\nPRINT {expression}");
            assert!(!compile_errors(&source(language, COMPOSITE, &body)).is_empty(), "{language}: {body}");
        }
        let aliases = "CONST Bits A = Bits.One\nCONST Bits B = A\nCONST Bits Bad = (B | Bits.Two) & A\nPRINT Bad";
        assert!(!compile_errors(&source(language, SPARSE, aliases)).is_empty());
    }
}

/// Unlike run_ppl, retain a VM failure so tests can prove no invalid value escaped.
fn checked_run(source: &str) -> Result<String, String> {
    checked_run_inspecting(source, |_| {})
}

fn checked_run_inspecting(source: &str, inspect: impl FnOnce(&crate::executable::VariableTable)) -> Result<String, String> {
    use crate::{
        icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
        vm::{self, io::DiskIO},
    };
    use icy_net::{Connection, ConnectionType, channel::ChannelConnection};
    use std::{path::PathBuf, sync::Arc};
    let executable = compile(source);
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await;
        let reader = async {
            let mut output = Vec::new();
            let mut buffer = [0; 1024];
            while let Ok(size) = peer.read(&mut buffer).await {
                if size == 0 {
                    break;
                }
                output.extend_from_slice(&buffer[..size]);
            }
            String::from_utf8(output).unwrap()
        };
        let execute = async {
            let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
            let registry = crate::parser::UserTypeRegistry::icy_board_registry();
            let script = crate::executable::PPEScript::from_ppe_file(&executable).unwrap();
            let mut machine = vm::VirtualMachine::new(PathBuf::from("bits.ppe"), &registry, &mut io, &mut state);
            machine.commands = script.statements.iter().map(|statement| statement.command.clone()).collect::<Vec<_>>().into();
            machine.label_table = script
                .statements
                .iter()
                .enumerate()
                .map(|(index, statement)| (statement.span.start * 2, index))
                .collect();
            machine.script = script;
            machine.variable_table = executable.variable_table;
            machine.user_types = executable.user_types;
            let result = machine.run().await;
            inspect(&machine.variable_table);
            drop(machine);
            drop(state);
            result.map_err(|error| error.to_string())
        };
        let (output, result) = tokio::join!(reader, execute);
        result.map(|_| output.clone()).map_err(|error| format!("{error}; output={output}"))
    })
}

#[test]
fn enum_bitwise_dynamic_invalid_results_cannot_escape_or_be_masked() {
    for language in [350, 400] {
        for expression in ["a | b", "a & b", "(a | b) & a", "(a & b) | a"] {
            for statement in [
                format!("PRINT {expression}"),
                format!("PRINT TOINTEGER({expression})"),
                format!("PRINT ({expression}) = a"),
                format!("IF (({expression}) = a) PRINT 99"),
                format!("a = {expression}"),
                format!("Use({expression})\nPROCEDURE Use(Bits value)\nPRINT value\nENDPROC"),
                format!("PRINT Result()\nFUNCTION Result() Bits\nRETURN {expression}\nENDFUNC"),
            ] {
                let body = format!("Bits a = Bits.One, b = Bits.Two\n{statement}");
                let error = checked_run(&source(language, SPARSE, &body)).unwrap_err();
                assert!(
                    error.contains("not a member of closed enum") && error.ends_with("output="),
                    "{language}: {body}: {error}"
                );
            }
        }
    }
}

#[test]
fn enum_bitwise_constants_aliases_arrays_records_callbacks_and_results() {
    assert_eq!(
        "3|3|1|3|3|0|3|3",
        checked_run(&source(
            400,
            COMPOSITE,
            r#"
CONST Bits A = Bits.One
CONST Bits B = A | Bits.Two
CONST Bits C = (B & Bits.Alias) | A
TYPE Boxed
 Bits Value
 Bits Values(1)
ENDTYPE
Bits items(1)
Boxed box = Boxed { Value = C }
items(0) = A | Bits.Two
items(1) = C & A
box.Values(1) = items(0) | items(1)
PRINT B, "|", C, "|", items(1), "|", box.Value, "|", box.Values(1), "|"
items(0) &= Bits.Zero
box.Value |= A
PRINT items(0), "|", box.Value, "|", Apply(Combine, A, Bits.Two)
FUNCTION Combine(Bits a, Bits b) Bits
 RETURN a | b
ENDFUNC
FUNCTION Apply(FUNCTION callback(Bits x, Bits y) Bits, Bits a, Bits b) Bits
 RETURN callback(a, b)
ENDFUNC
"#
        ))
        .unwrap()
    );
}

#[test]
fn enum_bitwise_evaluates_operands_once_left_to_right_with_mutation() {
    for language in [350, 400] {
        assert_eq!(
            "LR3|2|R3|1",
            run_ppl(&source(
                language,
                COMPOSITE,
                r#"
INTEGER calls = 0
Bits current = Bits.One
PRINT LeftValue() | RightValue(), "|", calls, "|"
calls = 0
current = Bits.One
PRINT current | RightValue(), "|", calls
FUNCTION LeftValue() Bits
 calls = calls + 1
 PRINT "L"
 RETURN current
ENDFUNC
FUNCTION RightValue() Bits
 calls = calls + 1
 PRINT "R"
 current = Bits.Two
 RETURN current
ENDFUNC
"#
            ))
        );
    }
}

#[test]
fn enum_bitwise_regex_masks_support_double_equals_and_not_equals() {
    for language in [350, 400] {
        assert_eq!(
            "1|1|1|0|1|0|1",
            run_ppl(&source(
                language,
                "",
                r#"
RegexOptions options = RegexOptions.IgnoreCase | RegexOptions.MultiLine
RegexOptions mask = RegexOptions.IgnoreCase | RegexOptions.MultiLine
PRINT (options & RegexOptions.IgnoreCase) == RegexOptions.IgnoreCase, "|"
PRINT (options & mask) == mask, "|"
PRINT (options & mask) != RegexOptions.None, "|"
PRINT (options & RegexOptions.Ascii) != RegexOptions.None, "|"
PRINT options == (RegexOptions.IgnoreCase | RegexOptions.MultiLine), "|"
options &= RegexOptions.MultiLine
PRINT (options & mask) == mask, "|"
options |= RegexOptions.IgnoreCase
PRINT options == mask
"#
            ))
        );
    }
}

#[test]
fn enum_bitwise_regex_has_seven_names_full_domain_and_correct_bits() {
    use crate::{
        executable::VariableType,
        parser::{REGEX_OPTIONS_ENUM_ID, UserTypeRegistry},
    };
    let registry = UserTypeRegistry::icy_board_registry();
    let definition = registry.get_enum_from_id(REGEX_OPTIONS_ENUM_ID).unwrap();
    assert_eq!(definition.domain, (0..64).collect::<Vec<_>>());
    assert_eq!(
        definition.variants.iter().map(|(name, value)| (name.as_str(), *value)).collect::<Vec<_>>(),
        vec![
            ("None", 0),
            ("IgnoreCase", 1),
            ("MultiLine", 2),
            ("DotMatchesNewLine", 4),
            ("IgnoreWhitespace", 8),
            ("SwapGreed", 16),
            ("Ascii", 32)
        ]
    );
    let executable = compile("RegexOptions options\nPRINT options");
    assert_eq!(executable.variable_table.enums[&REGEX_OPTIONS_ENUM_ID], definition.domain);
    assert!(
        executable
            .variable_table
            .checked_enum_value(VariableType::UserData(REGEX_OPTIONS_ENUM_ID), crate::executable::VariableValue::new_int(64))
            .is_err()
    );
    for language in [350, 400] {
        let program = r#"
INTEGER i, j
FOR i = 0 TO 63
 FOR j = 0 TO 63
  PRINT TOINTEGER(RegexOptions(i) | RegexOptions(j)), ",", TOINTEGER(RegexOptions(i) & RegexOptions(j)), ";"
 NEXT
NEXT
"#;
        let expected: String = (0..64).flat_map(|i| (0..64).map(move |j| format!("{},{};", i | j, i & j))).collect();
        assert_eq!(expected, run_ppl(&source(language, "", program)));
        assert_eq!(
            "3|2|0",
            run_ppl(&source(
                language,
                "",
                "CONST RegexOptions A = RegexOptions.IgnoreCase | RegexOptions.MultiLine\nCONST RegexOptions B = A\nRegexOptions defaultValue\nPRINT B, \"|\", B & RegexOptions.MultiLine, \"|\", defaultValue"
            ))
        );
    }
}
