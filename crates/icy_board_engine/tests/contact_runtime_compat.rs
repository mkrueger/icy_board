use icy_board_engine::{
    executable::{Executable, VariableType},
    parser::CONTACT_ID,
    vm::U_CONTACT,
};

const BETA_PPE: &str = "tests/test_data/gw-pass-beta-runtime-400.ppe";

#[test]
fn beta_runtime_400_ppe_does_not_gain_u_contact() {
    let executable = Executable::read_file(&BETA_PPE, false).expect("beta PPE should load");

    assert_eq!(executable.runtime, 400);
    assert_eq!(executable.variable_table.get_var_entry(U_CONTACT).header.variable_type, VariableType::Integer);
    assert!(!executable.variable_table.has_u_contact());
}

#[test]
fn transitional_runtime_400_ppe_with_contact_layout_is_recognized() {
    let mut executable = Executable::read_file(&BETA_PPE, false).expect("beta PPE should load");
    let contact = executable.variable_table.get_var_entry_mut(U_CONTACT);
    contact.header.variable_type = VariableType::UserData(CONTACT_ID as u8);
    contact.header.dim = 1;

    assert_eq!(executable.runtime, 400);
    assert!(executable.variable_table.has_u_contact());
}
