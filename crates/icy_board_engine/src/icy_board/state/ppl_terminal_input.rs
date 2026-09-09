use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::VariableValue,
    parser::TERM_INPUT_ID,
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
        crate::parser::board_catalog::register_members(crate::parser::TERM_INPUT_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplTerminalInput {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        Err(format!("Unknown TERMINPUT property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("TERMINPUT property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *MOUSE_ON && (!(0..=1).contains(&arguments[0].as_int()) || !(0..=2).contains(&arguments.get(1).map_or(2, VariableValue::as_int))) {
            use super::ppl_error::{ERR_INVALID, ERR_KIND_TERM, PplError};
            vm.set_error(PplError::new(ERR_KIND_TERM, ERR_INVALID, "unsupported MouseMode or MouseTracking value"));
            return Ok(VariableValue::new_bool(false));
        }
        vm.icy_board_state.term_input_member(name, arguments).await
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("TERMINPUT has no procedures".into())
    }
}
