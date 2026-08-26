use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::MACROS_ID,
};

pub static RECORD: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("BeginRecord".to_string()));
pub static END: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("EndRecord".to_string()));
pub static PLAY: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Play".to_string()));
pub static DELETE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Delete".to_string()));
pub static CLEAR: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("DeleteAll".to_string()));
pub static RECORDING: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Recording".to_string()));

/// The terminal's own macro slots, which hold display output it can replay for itself.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplMacros;

impl PplMacros {
    pub fn value() -> VariableValue {
        user_data_value(PplMacros, MACROS_ID)
    }
}

impl UserData for PplMacros {
    const TYPE_NAME: &'static str = "Macros";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(RECORDING.clone(), VariableType::Boolean, false);

        registry.add_function(RECORD.clone(), vec![VariableType::Integer], VariableType::Boolean);
        registry.add_function(END.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(PLAY.clone(), vec![VariableType::Integer], VariableType::Boolean);
        registry.add_function(DELETE.clone(), vec![VariableType::Integer], VariableType::Boolean);
        registry.add_function(CLEAR.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplMacros {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *RECORDING {
            return Ok(VariableValue::new_bool(vm.icy_board_state.ppl_terminal.is_recording()));
        }
        Err(format!("Unknown MACROS property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("MACROS property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        use crate::vm::statements::predefined_procedures as procedures;

        let slot = arguments.first().map_or(0, VariableValue::as_int);
        if *name == *RECORD {
            return Ok(VariableValue::new_bool(procedures::macros_record(vm, slot).await?));
        }
        if *name == *END {
            return Ok(VariableValue::new_bool(procedures::macros_end(vm).await?));
        }
        if *name == *PLAY {
            return Ok(VariableValue::new_bool(procedures::macros_play(vm, slot).await?));
        }
        if *name == *DELETE {
            return Ok(VariableValue::new_bool(procedures::macros_delete(vm, slot).await?));
        }
        if *name == *CLEAR {
            return Ok(VariableValue::new_bool(procedures::macros_clear(vm).await?));
        }
        Err(format!("Unknown MACROS function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("MACROS has no procedures".into())
    }
}
