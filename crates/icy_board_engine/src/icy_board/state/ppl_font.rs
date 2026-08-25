use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::FONT_ID,
};

pub static SET: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Set".to_string()));
pub static SET_ALL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("SetAll".to_string()));
pub static LOAD: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Load".to_string()));

/// The four typefaces a terminal keeps, one per attribute class.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplFont;

impl PplFont {
    pub fn value() -> VariableValue {
        user_data_value(PplFont, FONT_ID)
    }
}

impl UserData for PplFont {
    const TYPE_NAME: &'static str = "Font";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_function(SET.clone(), vec![VariableType::Integer, VariableType::Integer], VariableType::Boolean);
        registry.add_function(SET_ALL.clone(), vec![VariableType::Integer], VariableType::Boolean);
        registry.add_function(LOAD.clone(), vec![VariableType::Integer, VariableType::String], VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplFont {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        Err(format!("Unknown FONT property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("FONT properties are read-only".into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        use crate::vm::statements::predefined_procedures as procedures;

        let integer = |index: usize| arguments.get(index).map_or(0, VariableValue::as_int);
        if *name == *SET {
            return Ok(VariableValue::new_bool(procedures::font_set(vm, integer(0), integer(1)).await?));
        }
        if *name == *SET_ALL {
            // Every attribute class at once, which is what changing *the* font means.
            return Ok(VariableValue::new_bool(procedures::font_set(vm, -1, integer(0)).await?));
        }
        if *name == *LOAD {
            let file_name = arguments.get(1).map(VariableValue::as_string).unwrap_or_default();
            return Ok(VariableValue::new_bool(procedures::font_load(vm, integer(0), &file_name).await?));
        }
        Err(format!("Unknown FONT function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("FONT has no procedures".into())
    }
}
