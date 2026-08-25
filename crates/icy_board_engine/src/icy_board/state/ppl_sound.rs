use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::SOUND_ID,
};

pub static AVAILABLE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Available".to_string()));
pub static STOP_ALL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("StopAll".to_string()));

/// What the caller's terminal can play. The sounds themselves are `AUDIO` values.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplSound;

impl PplSound {
    pub fn value() -> VariableValue {
        user_data_value(PplSound, SOUND_ID)
    }
}

impl UserData for PplSound {
    const TYPE_NAME: &'static str = "Sound";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        // Asking the terminal is a question it may not have been asked yet, so it is a call.
        registry.add_function(AVAILABLE.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(STOP_ALL.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplSound {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        Err(format!("Unknown SOUND property {name}").into())
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("SOUND properties are read-only".into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *AVAILABLE {
            vm.icy_board_state.probe_terminal_media().await?;
            return Ok(VariableValue::new_bool(vm.icy_board_state.session.term_caps.sound));
        }
        if *name == *STOP_ALL {
            return Ok(VariableValue::new_bool(
                crate::vm::statements::predefined_procedures::sound_stop_all(vm).await?,
            ));
        }
        Err(format!("Unknown SOUND function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("SOUND has no procedures".into())
    }
}
