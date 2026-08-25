use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::{GFX_ID, TERM_INFO_ID, TERMINAL_ID},
};

pub static INFO: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Info".to_string()));
pub static GFX: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Gfx".to_string()));

/// The caller's terminal, and the way into everything that draws on it.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplTerminal;

impl PplTerminal {
    pub fn value() -> VariableValue {
        user_data_value(PplTerminal, TERMINAL_ID)
    }
}

impl UserData for PplTerminal {
    const TYPE_NAME: &'static str = "Terminal";
    const INSTANCE_PROVIDER: Option<crate::executable::FuncOpCode> = Some(crate::executable::FuncOpCode::Terminal);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(INFO.clone(), VariableType::UserData(TERM_INFO_ID as u8), false);
        registry.add_property(GFX.clone(), VariableType::UserData(GFX_ID as u8), false);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplTerminal {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *INFO {
            return Ok(super::ppl_terminal_info::PplTerminalInfo::from(&vm.icy_board_state.session.term_caps).value());
        }
        if *name == *GFX {
            return Ok(super::ppl_gfx::PplGfx::value());
        }
        Err(format!("Unknown TERMINAL property {name}").into())
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("TERMINAL properties are read-only".into())
    }

    async fn call_function(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        Err(format!("Unknown TERMINAL function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("TERMINAL has no procedures".into())
    }
}
