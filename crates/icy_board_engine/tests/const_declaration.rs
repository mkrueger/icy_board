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
    let reg = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();

    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &reg, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, reg, errors.clone());
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

/// A constant is worked out while compiling, so the executable is the one the value
/// written out by hand would produce.
#[test]
fn a_constant_costs_nothing_at_runtime() {
    let with_constant = compile("CONST INTEGER MaxTries = 3\nPRINTLN MaxTries\n").unwrap();
    let with_literal = compile("PRINTLN 3\n").unwrap();

    assert_eq!(with_literal.to_buffer().unwrap(), with_constant.to_buffer().unwrap());
}

#[test]
fn a_constant_may_be_written_in_terms_of_an_earlier_one() {
    let chained = compile("CONST INTEGER MaxTries = 3\nCONST INTEGER Doubled = MaxTries * 2\nPRINTLN Doubled\n").unwrap();
    let literal = compile("PRINTLN 6\n").unwrap();

    assert_eq!(literal.to_buffer().unwrap(), chained.to_buffer().unwrap());
}

#[test]
fn a_constant_of_a_routine_belongs_to_it() {
    let errors = diagnostics("DECLARE PROCEDURE Show()\nPRINTLN Local\nPROCEDURE Show()\n  CONST INTEGER Local = 7\n  PRINTLN Local\nENDPROC\n");
    assert!(errors.iter().any(|e| e.contains("Local")), "the constant should be local to Show: {errors:?}");
}

#[test]
fn a_constant_cannot_be_written_to() {
    let errors = diagnostics("CONST INTEGER MaxTries = 3\nMaxTries = 5\n");
    assert_eq!(vec!["'MaxTries' is a constant, it can only be read".to_string()], errors);
}

#[test]
fn a_constant_needs_a_value_that_is_known_while_compiling() {
    let errors = diagnostics("INTEGER x\nCONST INTEGER MaxTries = x + 1\n");
    assert_eq!(vec!["A constant needs a value the compiler can work out".to_string()], errors);
}

#[test]
fn a_constant_and_a_variable_cannot_share_a_name() {
    let errors = diagnostics("CONST INTEGER MaxTries = 3\nINTEGER MaxTries\n");
    assert_eq!(vec!["Variable name already used (MaxTries)".to_string()], errors);

    let errors = diagnostics("INTEGER MaxTries\nCONST INTEGER MaxTries = 3\n");
    assert_eq!(vec!["Variable name already used (MaxTries)".to_string()], errors);
}

#[test]
fn local_names_may_shadow_global_variables_and_constants() {
    let local_variable =
        compile("CONST INTEGER Limit = 10\nDECLARE PROCEDURE Show()\nShow()\nPROCEDURE Show()\n  INTEGER Limit\n  Limit = 3\n  PRINTLN Limit\nENDPROC\n")
            .unwrap();
    let renamed_global =
        compile("CONST INTEGER GlobalLimit = 10\nDECLARE PROCEDURE Show()\nShow()\nPROCEDURE Show()\n  INTEGER Limit\n  Limit = 3\n  PRINTLN Limit\nENDPROC\n")
            .unwrap();
    assert_eq!(renamed_global.script_buffer, local_variable.script_buffer);

    let parameter =
        compile("CONST INTEGER Limit = 10\nDECLARE PROCEDURE Show(INTEGER limit)\nShow(3)\nPROCEDURE Show(INTEGER limit)\n  PRINTLN limit\nENDPROC\n").unwrap();
    let renamed_global =
        compile("CONST INTEGER GlobalLimit = 10\nDECLARE PROCEDURE Show(INTEGER limit)\nShow(3)\nPROCEDURE Show(INTEGER limit)\n  PRINTLN limit\nENDPROC\n")
            .unwrap();
    assert_eq!(renamed_global.script_buffer, parameter.script_buffer);

    let local_constant =
        compile("INTEGER Limit\nDECLARE PROCEDURE Show()\nShow()\nPROCEDURE Show()\n  CONST INTEGER Limit = 3\n  PRINTLN Limit\nENDPROC\n").unwrap();
    let renamed_global =
        compile("INTEGER GlobalLimit\nDECLARE PROCEDURE Show()\nShow()\nPROCEDURE Show()\n  CONST INTEGER Limit = 3\n  PRINTLN Limit\nENDPROC\n").unwrap();
    assert_eq!(renamed_global.script_buffer, local_constant.script_buffer);

    let local_constant =
        compile("CONST INTEGER Limit = 10\nDECLARE PROCEDURE Show()\nShow()\nPROCEDURE Show()\n  CONST INTEGER Limit = 3\n  PRINTLN Limit\nENDPROC\n").unwrap();
    let renamed_global =
        compile("CONST INTEGER GlobalLimit = 10\nDECLARE PROCEDURE Show()\nShow()\nPROCEDURE Show()\n  CONST INTEGER Limit = 3\n  PRINTLN Limit\nENDPROC\n")
            .unwrap();
    assert_eq!(renamed_global.script_buffer, local_constant.script_buffer);
}

#[test]
fn variables_parameters_and_constants_still_share_a_local_scope() {
    let errors = diagnostics("DECLARE PROCEDURE Show()\nPROCEDURE Show()\n  INTEGER Limit\n  CONST INTEGER Limit = 3\nENDPROC\n");
    assert_eq!(vec!["Variable name already used (Limit)".to_string()], errors);

    let errors = diagnostics("DECLARE PROCEDURE Show()\nPROCEDURE Show()\n  CONST INTEGER Limit = 3\n  INTEGER Limit\nENDPROC\n");
    assert_eq!(vec!["Variable name already used (Limit)".to_string()], errors);

    let errors = diagnostics("DECLARE PROCEDURE Show(INTEGER Limit)\nPROCEDURE Show(INTEGER Limit)\n  CONST INTEGER Limit = 3\nENDPROC\n");
    assert_eq!(vec!["Variable name already used (Limit)".to_string()], errors);
}

/// CONST is a 3.50 word, so a 3.40 source may still have a variable called const.
#[test]
fn const_is_a_keyword_from_350_on() {
    let errors = diagnostics(";$LANGVERSION 340\nINTEGER Const\nConst = 2\nPRINTLN Const\n");
    assert!(errors.is_empty(), "{errors:?}");

    let errors = diagnostics(";$LANGVERSION 350\nINTEGER Const\n");
    assert!(!errors.is_empty(), "const should not be a variable name in 350");
}

/// Nothing of a constant reaches the runtime, so a 3.50 source may have one.
#[test]
fn a_constant_works_in_a_350_source() {
    let with_constant = compile(";$LANGVERSION 350\nCONST INTEGER MaxTries = 3\nPRINTLN MaxTries\n").unwrap();
    let with_literal = compile(";$LANGVERSION 350\nPRINTLN 3\n").unwrap();

    assert_eq!(with_literal.to_buffer().unwrap(), with_constant.to_buffer().unwrap());
}

/// A constant may name an enum member and keeps the enum as its type.
#[test]
fn a_constant_may_hold_an_enum_member() {
    let color = "ENUM Color\n  Red\n  Green = 5\nENDENUM\n";

    let with_constant = compile(&format!("{color}CONST Color Favorite = Color.Green\nColor shade = Favorite\nPRINTLN shade\n")).unwrap();
    let with_member = compile(&format!("{color}Color shade = Color.Green\nPRINTLN shade\n")).unwrap();
    assert_eq!(with_member.to_buffer().unwrap(), with_constant.to_buffer().unwrap());

    let errors = diagnostics(&format!("{color}CONST Color Favorite = 1\n"));
    assert_eq!(vec!["Can't assign Integer to Color".to_string()], errors);

    let errors = diagnostics(&format!("{color}CONST Color Favorite = Color.Green\nINTEGER count = Favorite\n"));
    assert!(errors.iter().any(|e| e == "Can't assign Color to Integer"), "{errors:?}");
}

/// The declared type decides what the value is written as, not the literal.
#[test]
fn the_declared_type_decides_the_constant() {
    let money = compile("CONST MONEY Fee = $1.50\nPRINTLN Fee\n").unwrap();
    let literal = compile("PRINTLN $1.50\n").unwrap();

    assert_eq!(literal.to_buffer().unwrap(), money.to_buffer().unwrap());
}
