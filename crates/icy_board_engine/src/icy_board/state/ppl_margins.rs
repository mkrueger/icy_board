use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::MARGINS_ID,
};

macro_rules! member_name {
    ($konst:ident, $name:literal) => {
        pub static $konst: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($name.to_string()));
    };
}

member_name!(SET_VERTICAL, "SetVertical");
member_name!(SET_HORIZONTAL, "SetHorizontal");
member_name!(RESET_VERTICAL, "ResetVertical");
member_name!(RESET_HORIZONTAL, "ResetHorizontal");
member_name!(RESET, "ResetAll");
member_name!(TOP, "Top");
member_name!(BOTTOM, "Bottom");
member_name!(LEFT, "Left");
member_name!(RIGHT, "Right");
member_name!(HAS_VERTICAL, "HasVertical");
member_name!(HAS_HORIZONTAL, "HasHorizontal");

/// The scrolling region, which is both what was asked for and where text now goes.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplMargins;

impl PplMargins {
    pub fn value() -> VariableValue {
        user_data_value(PplMargins, MARGINS_ID)
    }
}

impl UserData for PplMargins {
    const TYPE_NAME: &'static str = "Margins";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        for name in [&*TOP, &*BOTTOM, &*LEFT, &*RIGHT] {
            registry.add_property(name.clone(), VariableType::Integer, false);
        }
        registry.add_property(HAS_VERTICAL.clone(), VariableType::Boolean, false);
        registry.add_property(HAS_HORIZONTAL.clone(), VariableType::Boolean, false);

        registry.add_function(SET_VERTICAL.clone(), vec![VariableType::Integer, VariableType::Integer], VariableType::Boolean);
        registry.add_function(
            SET_HORIZONTAL.clone(),
            vec![VariableType::Integer, VariableType::Integer],
            VariableType::Boolean,
        );
        registry.add_function(RESET_VERTICAL.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(RESET_HORIZONTAL.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(RESET.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplMargins {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let terminal = &vm.icy_board_state.display_screen().buffer.buffer.terminal_state;
        let vertical = terminal.margins_top_bottom();
        let horizontal = terminal.margins_left_right();

        if *name == *TOP {
            return Ok(VariableValue::new_int(vertical.map_or(0, |(top, _)| top + 1)));
        }
        if *name == *BOTTOM {
            return Ok(VariableValue::new_int(vertical.map_or(0, |(_, bottom)| bottom + 1)));
        }
        if *name == *LEFT {
            return Ok(VariableValue::new_int(horizontal.map_or(0, |(left, _)| left + 1)));
        }
        if *name == *RIGHT {
            return Ok(VariableValue::new_int(horizontal.map_or(0, |(_, right)| right + 1)));
        }
        if *name == *HAS_VERTICAL {
            return Ok(VariableValue::new_bool(vertical.is_some()));
        }
        if *name == *HAS_HORIZONTAL {
            return Ok(VariableValue::new_bool(horizontal.is_some()));
        }
        Err(format!("Unknown MARGINS property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("MARGINS properties are read-only".into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        use crate::vm::statements::predefined_procedures as procedures;

        let integer = |index: usize| arguments.get(index).map_or(0, VariableValue::as_int);
        if *name == *SET_VERTICAL {
            return Ok(VariableValue::new_bool(procedures::margins_set_vertical(vm, integer(0), integer(1)).await?));
        }
        if *name == *SET_HORIZONTAL {
            return Ok(VariableValue::new_bool(procedures::margins_set_horizontal(vm, integer(0), integer(1)).await?));
        }
        if *name == *RESET_VERTICAL {
            procedures::reset_v_margins(vm, &[]).await?;
            return Ok(VariableValue::new_bool(true));
        }
        if *name == *RESET_HORIZONTAL {
            procedures::reset_h_margins(vm, &[]).await?;
            return Ok(VariableValue::new_bool(true));
        }
        if *name == *RESET {
            procedures::reset_margins(vm, &[]).await?;
            return Ok(VariableValue::new_bool(true));
        }
        Err(format!("Unknown MARGINS function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("MARGINS has no procedures".into())
    }
}
