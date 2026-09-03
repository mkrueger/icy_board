use icy_board_engine::{
    decompiler::decompile,
    executable::{Executable, LAST_PPL_LANGUAGE_VERSION},
};

#[test]
fn excessive_array_allocations_are_rejected() {
    let mut bytes = include_bytes!("test_data/malformed_array_allocation.ppe").to_vec();
    let Err(error) = Executable::from_buffer(&mut bytes, false) else {
        panic!("the malformed PPE loaded");
    };

    assert!(error.to_string().contains("loading is limited"), "{error}");
}

#[test]
fn invalid_routine_metadata_is_rejected() {
    let mut bytes = include_bytes!("test_data/malformed_routine_metadata.ppe").to_vec();
    let Err(error) = Executable::from_buffer(&mut bytes, false) else {
        panic!("the malformed PPE loaded");
    };

    assert!(error.to_string().contains("Invalid index in variable table"), "{error}");
}

#[test]
fn an_assignment_target_that_is_not_a_variable_is_decompiled_as_a_comment() {
    let mut bytes = include_bytes!("test_ppe/test_pplc_100.ppe").to_vec();
    bytes[650] = 102;
    let executable = Executable::from_buffer(&mut bytes, false).unwrap();

    let (ast, _) = decompile(executable, false, LAST_PPL_LANGUAGE_VERSION).unwrap();

    assert!(ast.to_string().contains("Invalid assignment target"), "{ast}");
}
