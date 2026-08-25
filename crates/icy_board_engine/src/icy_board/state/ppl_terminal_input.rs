use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::{EVENT_ID, TERM_INPUT_ID},
};

/// The caller's keyboard and mouse. Turning reporting on is what takes them over from
/// the board's own reads, and `Release` is what gives them back.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplTerminalInput;

impl PplTerminalInput {
    pub fn value() -> VariableValue {
        user_data_value(PplTerminalInput, TERM_INPUT_ID)
    }
}

pub static POLL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Poll".to_string()));
pub static WAIT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Wait".to_string()));
pub static MOUSE_ON: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("MouseOn".to_string()));
pub static MOUSE_OFF: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("MouseOff".to_string()));
pub static KEYBOARD_ON: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("KeyboardOn".to_string()));
pub static KEYBOARD_OFF: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("KeyboardOff".to_string()));
pub static RELEASE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Release".to_string()));

impl UserData for PplTerminalInput {
    const TYPE_NAME: &'static str = "TermInput";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        use crate::parser::{MOUSE_MODE_ENUM_ID, MOUSE_TRACKING_ENUM_ID};

        registry.add_function(POLL.clone(), Vec::new(), VariableType::UserData(EVENT_ID as u8));
        registry.add_function(WAIT.clone(), vec![VariableType::Integer], VariableType::UserData(EVENT_ID as u8));
        registry.add_function_with(
            MOUSE_ON.clone(),
            vec![VariableType::UserData(MOUSE_MODE_ENUM_ID), VariableType::UserData(MOUSE_TRACKING_ENUM_ID)],
            1,
            VariableType::Boolean,
        );
        registry.add_function(MOUSE_OFF.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function_with(KEYBOARD_ON.clone(), vec![VariableType::Boolean], 0, VariableType::Boolean);
        registry.add_function(KEYBOARD_OFF.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(RELEASE.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplTerminalInput {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        Err(format!("Unknown TERMINPUT property {name}").into())
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("TERMINPUT properties are read-only".into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        vm.icy_board_state.term_input_member(name, arguments).await
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("TERMINPUT has no procedures".into())
    }
}
