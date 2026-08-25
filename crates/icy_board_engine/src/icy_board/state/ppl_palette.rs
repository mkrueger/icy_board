use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::PALETTE_ID,
};

pub static SET: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Set".to_string()));
pub static SET_RGB: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("SetRgb".to_string()));
pub static RESET: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Reset".to_string()));
pub static RESET_ALL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("ResetAll".to_string()));

/// The 16 DOS colours `COLOR` selects from.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplPalette;

impl PplPalette {
    pub fn value() -> VariableValue {
        user_data_value(PplPalette, PALETTE_ID)
    }
}

impl UserData for PplPalette {
    const TYPE_NAME: &'static str = "Palette";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_function(SET.clone(), vec![VariableType::Integer, VariableType::Unsigned], VariableType::Boolean);
        registry.add_function(
            SET_RGB.clone(),
            vec![VariableType::Integer, VariableType::Integer, VariableType::Integer, VariableType::Integer],
            VariableType::Boolean,
        );
        registry.add_function(RESET.clone(), vec![VariableType::Integer], VariableType::Boolean);
        registry.add_function(RESET_ALL.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplPalette {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        Err(format!("Unknown PALETTE property {name}").into())
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("PALETTE properties are read-only".into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        use crate::vm::statements::predefined_procedures::{self as procedures, PaletteComponents};

        let integer = |index: usize| arguments.get(index).map_or(0, VariableValue::as_int);
        if *name == *SET {
            let packed = arguments.get(1).map_or(0, |value| value.as_unsigned() as u32);
            return Ok(VariableValue::new_bool(
                procedures::palette_set(vm, integer(0), PaletteComponents::Packed(packed)).await?,
            ));
        }
        if *name == *SET_RGB {
            let components = PaletteComponents::Separate(integer(1), integer(2), integer(3));
            return Ok(VariableValue::new_bool(procedures::palette_set(vm, integer(0), components).await?));
        }
        if *name == *RESET {
            return Ok(VariableValue::new_bool(procedures::palette_reset(vm, integer(0)).await?));
        }
        if *name == *RESET_ALL {
            return Ok(VariableValue::new_bool(procedures::palette_reset_all(vm).await?));
        }
        Err(format!("Unknown PALETTE function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("PALETTE has no procedures".into())
    }
}
