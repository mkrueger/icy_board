use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::Executable,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

fn compile(source: &str) -> Result<Executable, Vec<String>> {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);

    let reporter = errors.lock().unwrap();
    if reporter.has_errors() {
        return Err(reporter.errors.iter().map(|e| e.error.to_string()).collect());
    }
    drop(reporter);
    Ok(compiler.create_executable().unwrap())
}

fn diagnostics(source: &str) -> Vec<String> {
    compile(source).err().unwrap_or_default()
}

const COLOR: &str = "ENUM Color\n  Red\n  Green = 5\n  Blue\nENDENUM\n";

#[test]
fn an_enum_costs_nothing_at_runtime() {
    let enum_source = format!("{COLOR}Color favorite = Color.Green\nPRINTLN favorite\n");
    let integer_source = "INTEGER favorite = 5\nPRINTLN favorite\n";

    assert_eq!(
        compile(integer_source).unwrap().to_buffer().unwrap(),
        compile(&enum_source).unwrap().to_buffer().unwrap()
    );
}

#[test]
fn an_implicit_member_follows_the_previous_value() {
    let enum_source = format!("{COLOR}PRINTLN Color.Red, Color.Green, Color.Blue\n");
    let integer_source = "PRINTLN 0, 5, 6\n";

    assert_eq!(
        compile(integer_source).unwrap().to_buffer().unwrap(),
        compile(&enum_source).unwrap().to_buffer().unwrap()
    );
}

#[test]
fn enum_values_are_nominal() {
    let source = format!(
        "{COLOR}ENUM Status\n  Ready\nENDENUM\nColor favorite = Color.Red\nStatus state = Status.Ready\nfavorite = 1\nfavorite = state\nIF favorite == 1 PRINTLN \"bad\"\nIF favorite == state PRINTLN \"bad\"\n"
    );
    let errors = diagnostics(&source);

    assert!(errors.iter().any(|e| e == "Can't assign Integer to Color"), "{errors:?}");
    assert!(errors.iter().any(|e| e == "Can't assign Status to Color"), "{errors:?}");
    assert!(errors.iter().any(|e| e == "Can't compare Color with Integer"), "{errors:?}");
    assert!(errors.iter().any(|e| e == "Can't compare Color with Status"), "{errors:?}");
}

#[test]
fn the_enum_name_is_the_namespace() {
    let errors = diagnostics(&format!("{COLOR}PRINTLN Green\nPRINTLN Color.Missing\n"));
    assert!(errors.iter().any(|e| e.contains("Variable not found (Green)")), "{errors:?}");
    assert!(errors.iter().any(|e| e == "Enum Color has no member named Missing"), "{errors:?}");
}

#[test]
fn enum_parameters_and_returns_lower_to_integer() {
    let enum_source = format!(
        "{COLOR}DECLARE FUNCTION Echo(Color value) Color\nColor color = Echo(Color.Blue)\nPRINTLN color\nFUNCTION Echo(Color value) Color\n  RETURN value\nENDFUNC\n"
    );
    let integer_source =
        "DECLARE FUNCTION Echo(INTEGER value) INTEGER\nINTEGER color = Echo(6)\nPRINTLN color\nFUNCTION Echo(INTEGER value) INTEGER\n  RETURN value\nENDFUNC\n";

    assert_eq!(
        compile(integer_source).unwrap().to_buffer().unwrap(),
        compile(&enum_source).unwrap().to_buffer().unwrap()
    );
}

#[test]
fn var_enum_parameters_lower_to_var_integer_parameters() {
    let enum_source = format!(
        "{COLOR}DECLARE PROCEDURE SetBlue(VAR Color value)\nColor favorite = Color.Red\nSetBlue(favorite)\nPRINTLN favorite\nPROCEDURE SetBlue(VAR Color value)\n  value = Color.Blue\nENDPROC\n"
    );
    let integer_source = "DECLARE PROCEDURE SetBlue(VAR INTEGER value)\nINTEGER favorite = 0\nSetBlue(favorite)\nPRINTLN favorite\nPROCEDURE SetBlue(VAR INTEGER value)\n  value = 6\nENDPROC\n";

    assert_eq!(
        compile(integer_source).unwrap().to_buffer().unwrap(),
        compile(&enum_source).unwrap().to_buffer().unwrap()
    );

    let errors = diagnostics(&format!("{COLOR}DECLARE PROCEDURE Use(Color value)\nUse(1)\n"));
    assert!(!errors.is_empty(), "a plain integer must not satisfy a Color parameter");
}

#[test]
fn enum_arrays_lower_to_integer_arrays() {
    let enum_source = format!("{COLOR}Color colors(2)\ncolors(1) = Color.Blue\nPRINTLN colors(1)\n");
    let integer_source = "INTEGER colors(2)\ncolors(1) = 6\nPRINTLN colors(1)\n";

    assert_eq!(
        compile(integer_source).unwrap().to_buffer().unwrap(),
        compile(&enum_source).unwrap().to_buffer().unwrap()
    );
}

#[test]
fn enum_fields_lower_to_integer_fields() {
    let enum_source = format!("{COLOR}TYPE Paint\n  Color Shade\nENDTYPE\nPaint item\nitem.Shade = Color.Green\nPRINTLN item.Shade\n");
    let integer_source = "TYPE Paint\n  INTEGER Shade\nENDTYPE\nPaint item\nitem.Shade = 5\nPRINTLN item.Shade\n";

    assert_eq!(
        compile(integer_source).unwrap().to_buffer().unwrap(),
        compile(&enum_source).unwrap().to_buffer().unwrap()
    );
}

#[test]
fn duplicate_and_nonconstant_members_are_errors() {
    let errors = diagnostics("ENUM Bad\n  A\n  A\nENDENUM\n");
    assert!(errors.iter().any(|e| e == "Enum member 'A' is already declared"), "{errors:?}");

    let errors = diagnostics("INTEGER value\nENUM Bad\n  A = value\nENDENUM\n");
    assert!(
        errors.iter().any(|e| e == "An enum member needs an integer value the compiler can work out"),
        "{errors:?}"
    );
}

/// A FOR writes its own comparison and step, so it may count over an enum even
/// though hand-written arithmetic on one stays an error.
#[test]
fn a_for_loop_may_count_over_an_enum() {
    let enum_source = format!("{COLOR}Color shade\nFOR shade = Color.Red TO Color.Blue\n  PRINTLN shade\nNEXT\n");
    let integer_source = "INTEGER shade\nFOR shade = 0 TO 6\n  PRINTLN shade\nNEXT\n";

    assert_eq!(
        compile(integer_source).unwrap().to_buffer().unwrap(),
        compile(&enum_source).unwrap().to_buffer().unwrap()
    );

    let errors = diagnostics(&format!("{COLOR}Color shade\nFOR shade = Color.Red TO 5\n  PRINTLN shade\nNEXT\n"));
    assert!(errors.iter().any(|e| e == "Can't compare Color with Integer"), "{errors:?}");

    let errors = diagnostics(&format!("{COLOR}Color shade = Color.Red\nshade = shade + 1\n"));
    assert!(!errors.is_empty(), "counting an enum up by hand is still an error");

    let errors = diagnostics(&format!("{COLOR}Color shade = Color.Red\nshade += 1\n"));
    assert!(!errors.is_empty(), "a compound assignment is not loop machinery");
}

#[test]
fn an_enum_is_named_in_argument_and_field_errors() {
    let errors = diagnostics(&format!(
        "{COLOR}DECLARE PROCEDURE Use(Color value)\nUse(1)\nPROCEDURE Use(Color value)\nENDPROC\n"
    ));
    assert!(errors.iter().any(|e| e == "Argument 1 expects Color, got Integer"), "{errors:?}");

    let errors = diagnostics(&format!("{COLOR}TYPE Paint\n  Color Shade\nENDTYPE\nPaint item = Paint {{ Shade = 1 }}\n"));
    assert!(errors.iter().any(|e| e == "Record field 'Shade' expects Color, got Integer"), "{errors:?}");
}

#[test]
fn enum_is_a_keyword_from_350_on() {
    let errors = diagnostics(";$LANGVERSION 340\nINTEGER Enum\nEnum = 2\nPRINTLN Enum\n");
    assert!(errors.is_empty(), "{errors:?}");

    let errors = diagnostics(";$LANGVERSION 350\nINTEGER Enum\n");
    assert!(!errors.is_empty(), "ENUM should not be a variable name in 350");
}

/// The members are gone before anything is emitted, so a 3.50 source may have an enum.
#[test]
fn an_enum_works_in_a_350_source() {
    let enum_source = format!(";$LANGVERSION 350\n{COLOR}Color favorite = Color.Green\nPRINTLN favorite\n");
    let integer_source = ";$LANGVERSION 350\nINTEGER favorite = 5\nPRINTLN favorite\n";

    assert_eq!(
        compile(integer_source).unwrap().to_buffer().unwrap(),
        compile(&enum_source).unwrap().to_buffer().unwrap()
    );
}
