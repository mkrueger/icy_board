//! `NAME()` on a variable that is not an array.
//!
//! GW-ONELINER does `SavedWho = U_ALIAS()`. The legacy PPE encoding stores that as the
//! plain variable, but PPE 4.00 files kept an empty index list, and reading one took the
//! whole board down.

use super::{compile, run_ppl};
use crate::executable::{Executable, PPECommand, PPEExpr, PPEScript};

#[test]
fn empty_parentheses_on_a_scalar_read_the_variable() {
    let output = run_ppl(
        r#"
        STRING s
        s = "alias"
        PRINTLN s()
        GETUSER
        U_ALIAS = "Prometheus"
        s = U_ALIAS()
        PRINTLN s
    "#,
    );
    assert_eq!(output, "alias\nPrometheus\n");
}

#[test]
fn empty_parentheses_on_a_scalar_compile_to_the_variable() {
    let executable = compile("STRING s\nPRINTLN s()");
    let script = PPEScript::from_ppe_file(&executable).unwrap();
    let PPECommand::PredefinedCall(_, arguments) = &script.statements[0].command else {
        panic!("PRINTLN expected")
    };
    assert!(matches!(arguments[0], PPEExpr::Value(_)), "{:?}", arguments[0]);
}

/// Replaces the argument of the program's `PRINTLN s` with `s` and no indices.
fn with_empty_index_list(executable: &Executable) -> Executable {
    let mut script = PPEScript::from_ppe_file(executable).unwrap();
    let statement = script
        .statements
        .iter_mut()
        .find(|statement| matches!(statement.command, PPECommand::PredefinedCall(..)))
        .expect("PRINTLN expected");
    let PPECommand::PredefinedCall(definition, arguments) = &statement.command else {
        unreachable!()
    };
    let PPEExpr::Value(id) = arguments[0] else { panic!("variable expected") };
    statement.command = PPECommand::PredefinedCall(definition, vec![PPEExpr::Dim(id, Vec::new())]);
    let mut changed = executable.clone();
    changed.in_memory_script = Some(script);
    changed
}

/// A PPE the earlier compiler already wrote still loads and runs.
#[test]
fn a_stored_empty_index_list_on_a_scalar_loads_as_the_variable() {
    let executable = with_empty_index_list(&compile("STRING s = \"alias\"\nPRINTLN s"));
    let loaded = Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap();
    let (result, output, ()) = super::try_run_executable_collecting_inspected(loaded, |_| {}, &[], None, b"", false, super::TestPpeBoundary::Vm, |_| ());
    result.unwrap();
    assert_eq!(output, "alias\n");
}

#[test]
fn an_empty_index_list_is_an_error_not_a_crash() {
    let executable = with_empty_index_list(&compile("STRING s = \"alias\"\nPRINTLN s"));
    let (result, _, ()) = super::try_run_executable_collecting_inspected(executable, |_| {}, &[], None, b"", false, super::TestPpeBoundary::Vm, |_| ());
    let error = result.expect_err("an empty index list must not run");
    assert!(
        matches!(
            error.downcast_ref::<crate::vm::VMError>(),
            Some(crate::vm::VMError::InvalidArrayDimensionCount(0))
        ),
        "{error}"
    );
}
