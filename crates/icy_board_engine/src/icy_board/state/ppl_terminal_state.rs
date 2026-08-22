use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::TERM_STATE_ID,
};

pub static MARGIN_TOP: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("MarginTop".to_string()));
pub static MARGIN_BOTTOM: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("MarginBottom".to_string()));
pub static MARGIN_LEFT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("MarginLeft".to_string()));
pub static MARGIN_RIGHT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("MarginRight".to_string()));
pub static HORIZONTAL_MARGINS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("HorizontalMargins".to_string()));
pub static VERTICAL_MARGINS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("VerticalMargins".to_string()));

#[derive(Clone, Copy, Debug, Default)]
pub struct PplTerminalState {
    pub margin_top: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,
    pub margin_right: i32,
    pub horizontal_margins: bool,
    pub vertical_margins: bool,
}

impl PplTerminalState {
    pub fn value(self) -> VariableValue {
        user_data_value(self, TERM_STATE_ID)
    }
}

impl UserData for PplTerminalState {
    const TYPE_NAME: &'static str = "TermState";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(MARGIN_TOP.clone(), VariableType::Integer, false);
        registry.add_property(MARGIN_BOTTOM.clone(), VariableType::Integer, false);
        registry.add_property(MARGIN_LEFT.clone(), VariableType::Integer, false);
        registry.add_property(MARGIN_RIGHT.clone(), VariableType::Integer, false);
        registry.add_property(HORIZONTAL_MARGINS.clone(), VariableType::Boolean, false);
        registry.add_property(VERTICAL_MARGINS.clone(), VariableType::Boolean, false);
    }
}

#[async_trait]
impl UserDataValue for PplTerminalState {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *MARGIN_TOP {
            return Ok(VariableValue::new_int(self.margin_top));
        }
        if *name == *MARGIN_BOTTOM {
            return Ok(VariableValue::new_int(self.margin_bottom));
        }
        if *name == *MARGIN_LEFT {
            return Ok(VariableValue::new_int(self.margin_left));
        }
        if *name == *MARGIN_RIGHT {
            return Ok(VariableValue::new_int(self.margin_right));
        }
        if *name == *HORIZONTAL_MARGINS {
            return Ok(VariableValue::new_bool(self.horizontal_margins));
        }
        if *name == *VERTICAL_MARGINS {
            return Ok(VariableValue::new_bool(self.vertical_margins));
        }
        Err(format!("Unknown TERMSTATE property {name}").into())
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Ok(())
    }

    async fn call_function(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        Err(format!("Unknown TERMSTATE function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown TERMSTATE method {name}").into())
    }
}
