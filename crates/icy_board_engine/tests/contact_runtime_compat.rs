use icy_board_engine::{
    executable::{SUPPORTED_PPE_VERSIONS, USER_VARIABLES, VariableType},
    parser::CONTACT_ID,
};

#[test]
fn released_user_variable_layouts_are_frozen() {
    let expected = [
        ("U_EXPERT", 100, VariableType::Boolean, 0, 0),
        ("U_FSE", 100, VariableType::Boolean, 0, 0),
        ("U_FSEP", 100, VariableType::Boolean, 0, 0),
        ("U_CLS", 100, VariableType::Boolean, 0, 0),
        ("U_EXPDATE", 100, VariableType::Date, 0, 0),
        ("U_SEC", 100, VariableType::Integer, 0, 0),
        ("U_PAGELEN", 100, VariableType::Integer, 0, 0),
        ("U_EXPSEC", 100, VariableType::Integer, 0, 0),
        ("U_CITY", 100, VariableType::String, 0, 0),
        ("U_BDPHONE", 100, VariableType::String, 0, 0),
        ("U_HVPHONE", 100, VariableType::String, 0, 0),
        ("U_TRANS", 100, VariableType::String, 0, 0),
        ("U_CMNT1", 100, VariableType::String, 0, 0),
        ("U_CMNT2", 100, VariableType::String, 0, 0),
        ("U_PWD", 100, VariableType::String, 0, 0),
        ("U_SCROLL", 100, VariableType::Boolean, 0, 0),
        ("U_LONGHDR", 100, VariableType::Boolean, 0, 0),
        ("U_DEF79", 100, VariableType::Boolean, 0, 0),
        ("U_ALIAS", 100, VariableType::String, 0, 0),
        ("U_VER", 100, VariableType::String, 0, 0),
        ("U_ADDR", 100, VariableType::String, 1, 5),
        ("U_NOTES", 100, VariableType::String, 1, 4),
        ("U_PWDEXP", 100, VariableType::Date, 0, 0),
        ("U_ACCOUNT", 300, VariableType::Integer, 1, 16),
        ("U_SHORTDESC", 340, VariableType::Boolean, 0, 0),
        ("U_GENDER", 340, VariableType::String, 0, 0),
        ("U_BIRTHDATE", 340, VariableType::String, 0, 0),
        ("U_EMAIL", 340, VariableType::String, 0, 0),
        ("U_WEB", 340, VariableType::String, 0, 0),
        ("U_CONTACT", 400, VariableType::UserData(CONTACT_ID as u8), 1, 0),
    ];
    let actual: Vec<_> = USER_VARIABLES
        .iter()
        .map(|variable| {
            (
                variable.name,
                variable.runtime_version,
                variable.value.get_type(),
                variable.value.get_dimensions(),
                variable.value.get_vector_size(),
            )
        })
        .collect();

    assert_eq!(
        actual, expected,
        "predefined user-variable slots are a PPE ABI; add new slots only in a new runtime"
    );
}

#[test]
fn released_runtimes_keep_their_user_variable_prefix_lengths() {
    let expected = [(100, 23), (200, 23), (300, 24), (310, 24), (320, 24), (330, 24), (340, 29), (400, 30)];
    let actual: Vec<_> = SUPPORTED_PPE_VERSIONS
        .iter()
        .map(|runtime| {
            (
                *runtime,
                USER_VARIABLES.iter().take_while(|variable| variable.runtime_version <= *runtime).count(),
            )
        })
        .collect();

    assert_eq!(actual, expected, "a released runtime's user-variable prefix must never change");
}
