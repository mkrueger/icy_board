use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::GFX_ID,
};

macro_rules! member_name {
    ($konst:ident, $name:literal) => {
        pub static $konst: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($name.to_string()));
    };
}

member_name!(INIT, "Init");
member_name!(SHUTDOWN, "Shutdown");
member_name!(BACKEND, "Backend");
member_name!(PACING, "Pacing");
member_name!(SET_PACING, "SetPacing");
member_name!(CELL_WIDTH, "CellWidth");
member_name!(CELL_HEIGHT, "CellHeight");
member_name!(SCREEN_WIDTH, "ScreenWidth");
member_name!(SCREEN_HEIGHT, "ScreenHeight");
member_name!(SIXEL, "Sixel");
member_name!(JXL, "Jxl");
member_name!(JXL_BLOB, "JxlBlob");
member_name!(PIXEL_MOUSE, "PixelMouse");
member_name!(CLIENT_BLIT, "ClientBlit");

/// What the session draws with: the backend, the surfaces and what the terminal can do.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplGfx;

impl PplGfx {
    pub fn value() -> VariableValue {
        user_data_value(PplGfx, GFX_ID)
    }
}

impl UserData for PplGfx {
    const TYPE_NAME: &'static str = "Gfx";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        let backend = VariableType::UserData(crate::parser::GFX_BACKEND_ENUM_ID);
        registry.add_property(BACKEND.clone(), backend, false);
        registry.add_property(PACING.clone(), VariableType::Integer, false);

        registry.add_function_with(INIT.clone(), vec![backend, VariableType::Boolean], 0, VariableType::Boolean);
        registry.add_function(SHUTDOWN.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(SET_PACING.clone(), vec![VariableType::Integer], VariableType::Boolean);

        // These ask the terminal when it has not been asked yet, so they are calls
        // rather than properties. `Terminal.Info` answers the same from the cache.
        for name in [&*CELL_WIDTH, &*CELL_HEIGHT, &*SCREEN_WIDTH, &*SCREEN_HEIGHT] {
            registry.add_function(name.clone(), Vec::new(), VariableType::Integer);
        }
        for name in [&*SIXEL, &*JXL, &*JXL_BLOB, &*PIXEL_MOUSE, &*CLIENT_BLIT] {
            registry.add_function(name.clone(), Vec::new(), VariableType::Boolean);
        }
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplGfx {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *BACKEND {
            let backend = vm
                .icy_board_state
                .ppl_graphics
                .as_ref()
                .map_or(crate::icy_board::state::ppl_graphics::GFX_BACKEND_NONE, |graphics| graphics.backend);
            return Ok(VariableValue::new_int(backend));
        }
        if *name == *PACING {
            let pacing = vm.icy_board_state.ppl_graphics.as_ref().is_some_and(|graphics| graphics.pacing);
            return Ok(VariableValue::new_int(i32::from(pacing)));
        }
        Err(format!("Unknown GFX property {name}").into())
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("GFX properties are read-only".into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        crate::vm::statements::predefined_procedures::gfx_member(vm, name, arguments).await
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err("GFX has no procedures".into())
    }
}
