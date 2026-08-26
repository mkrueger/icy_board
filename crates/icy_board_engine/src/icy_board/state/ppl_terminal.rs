use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::{GFX_ID, MACROS_ID, MARGINS_ID, PALETTE_ID, TERM_INFO_ID, TERM_INPUT_ID, TERMINAL_ID},
};

pub static INFO: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Info".to_string()));
pub static GFX: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Gfx".to_string()));
pub static INPUT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Input".to_string()));
pub static MARGINS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Margins".to_string()));
pub static PALETTE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Palette".to_string()));
pub static SET_FONT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("SetFont".to_string()));
pub static LOAD_FONT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("LoadFont".to_string()));
pub static MACROS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Macros".to_string()));
pub static BEGIN_UPDATE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("BeginUpdate".to_string()));
pub static END_UPDATE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("EndUpdate".to_string()));

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
        registry.add_property(INPUT.clone(), VariableType::UserData(TERM_INPUT_ID as u8), false);
        registry.add_property(MARGINS.clone(), VariableType::UserData(MARGINS_ID as u8), false);
        registry.add_property(PALETTE.clone(), VariableType::UserData(PALETTE_ID as u8), false);
        registry.add_property(MACROS.clone(), VariableType::UserData(MACROS_ID as u8), false);

        registry.add_function(BEGIN_UPDATE.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(END_UPDATE.clone(), Vec::new(), VariableType::Boolean);
        // Leaving the slot out means every attribute class, which is what changing
        // *the* font means.
        registry.add_function_with(SET_FONT.clone(), vec![VariableType::Integer, VariableType::Integer], 1, VariableType::Boolean);
        registry.add_function(LOAD_FONT.clone(), vec![VariableType::Integer, VariableType::String], VariableType::Boolean);
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
        if *name == *INPUT {
            return Ok(super::ppl_terminal_input::PplTerminalInput::value());
        }
        if *name == *MARGINS {
            return Ok(super::ppl_margins::PplMargins::value());
        }
        if *name == *PALETTE {
            return Ok(super::ppl_palette::PplPalette::value());
        }
        if *name == *MACROS {
            return Ok(super::ppl_macros::PplMacros::value());
        }
        Err(format!("Unknown TERMINAL property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("TERMINAL property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        use crate::vm::statements::predefined_procedures as procedures;

        if *name == *BEGIN_UPDATE {
            return Ok(VariableValue::new_bool(procedures::terminal_begin_update(vm).await?));
        }
        if *name == *END_UPDATE {
            return Ok(VariableValue::new_bool(procedures::terminal_end_update(vm).await?));
        }
        let integer = |index: usize| arguments.get(index).map_or(0, VariableValue::as_int);
        if *name == *SET_FONT {
            let slot = if arguments.len() > 1 { integer(1) } else { -1 };
            return Ok(VariableValue::new_bool(procedures::font_set(vm, slot, integer(0)).await?));
        }
        if *name == *LOAD_FONT {
            let file_name = arguments.get(1).map(VariableValue::as_string).unwrap_or_default();
            return Ok(VariableValue::new_bool(procedures::font_load(vm, integer(0), &file_name).await?));
        }
        Err(format!("Unknown TERMINAL function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("TERMINAL has no procedures".into())
    }
}
