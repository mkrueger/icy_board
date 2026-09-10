//! The `SURFACE` object a PPE draws on.
//!
//! Aliases retain one allocation identity; `PplGraphicsState` owns the pixels.

use async_trait::async_trait;

use crate::{
    compiler::user_data::{ResourceIdentity, ResourceUserData, UserData, UserDataMemberRegistry, UserDataValue, resource_user_data_value},
    executable::{GenericVariableData, VariableData, VariableType, VariableValue},
    parser::SURFACE_ID,
};

#[derive(Clone)]
pub struct PplSurface {
    pub handle: i32,
    identity: Option<ResourceIdentity>,
}

impl PplSurface {
    /// A detached identity; allocation binds a value with `with_identity` instead.
    pub fn value(handle: i32) -> VariableValue {
        Self::with_identity(handle, (handle != 0).then(ResourceIdentity::default))
    }

    pub(crate) fn with_identity(handle: i32, identity: Option<ResourceIdentity>) -> VariableValue {
        let mut value = resource_user_data_value(
            PplSurface {
                handle,
                identity: identity.clone(),
            },
            SURFACE_ID,
            identity,
        );
        // Surface arguments carry the handle in the data word as well.
        value.data = VariableData::from_int(handle);
        value
    }

    pub(crate) fn is_live(&self, graphics: &super::ppl_graphics::PplGraphicsState) -> bool {
        self.identity.as_ref().is_some_and(|identity| graphics.identity(self.handle) == Some(identity))
    }

    /// An answer for a surface that could not be made, so its members stay callable.
    pub fn invalid() -> VariableValue {
        Self::value(0)
    }
}

/// Resolve a `SURFACE` argument only in the graphics allocation that issued it.
pub fn surface_handle(value: &VariableValue, graphics: &super::ppl_graphics::PplGraphicsState) -> Option<i32> {
    if value.get_type() != VariableType::UserData(SURFACE_ID as u32) {
        return None;
    }
    let GenericVariableData::UserData(object) = &value.generic_data else {
        return None;
    };
    let identity = object.downcast_ref::<ResourceUserData>()?.identity.as_ref()?;
    let handle = unsafe { value.data.int_value };
    (graphics.identity(handle) == Some(identity)).then_some(handle)
}

pub static WIDTH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Width".to_string()));
pub static HEIGHT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Height".to_string()));
pub static VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Valid".to_string()));
pub static CLEAR: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Clear".to_string()));
pub static SET_PIXEL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("SetPixel".to_string()));
pub static GET_PIXEL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("GetPixel".to_string()));
pub static FILL_RECT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("FillRect".to_string()));
pub static RECT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("DrawRect".to_string()));
pub static BLIT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Blit".to_string()));
pub static BLIT_RECT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("BlitRect".to_string()));
pub static PRESENT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Present".to_string()));
pub static PRESENT_AT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("PresentAt".to_string()));
pub static PRESENT_RECT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("PresentRect".to_string()));
pub static PIN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Pin".to_string()));
pub static UNPIN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Unpin".to_string()));
pub static FREE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Free".to_string()));
pub static NEW: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("New".to_string()));
pub static LOAD: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Load".to_string()));

impl UserData for PplSurface {
    const TYPE_NAME: &'static str = "Surface";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(PplSurface::invalid);
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(PplSurface::invalid);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(SURFACE_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplSurface {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let surface = vm
            .icy_board_state
            .ppl_graphics
            .as_ref()
            .filter(|graphics| self.is_live(graphics))
            .and_then(|graphics| graphics.surfaces.get(&self.handle));
        if *name == *WIDTH {
            return Ok(VariableValue::new_int(surface.map_or(0, |surface| surface.width as i32)));
        }
        if *name == *HEIGHT {
            return Ok(VariableValue::new_int(surface.map_or(0, |surface| surface.height as i32)));
        }
        if *name == *VALID {
            return Ok(VariableValue::new_bool(surface.is_some()));
        }
        log::error!("Invalid user data call on Surface ({name})");
        Ok(VariableValue::new_int(-1))
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("SURFACE property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *NEW {
            let width = arguments.first().map_or(0, VariableValue::as_int);
            let height = arguments.get(1).map_or(0, VariableValue::as_int);
            return crate::vm::statements::predefined_procedures::gfx_new_surface(vm, width, height);
        }
        if *name == *LOAD {
            let file_name = arguments.first().map(VariableValue::as_string).unwrap_or_default();
            return crate::vm::statements::predefined_procedures::gfx_load_surface(vm, &file_name).await;
        }
        if !vm.icy_board_state.ppl_graphics.as_ref().is_some_and(|graphics| self.is_live(graphics)) {
            // Pin reports an unsupported backend before checking allocation validity.
            vm.icy_board_state.gfx_error = match vm.icy_board_state.ppl_graphics.as_ref() {
                None => 1,
                Some(graphics) if *name == *PIN && graphics.backend != super::ppl_graphics::GFX_BACKEND_JXL => 6,
                Some(_) => 2,
            };
            return Ok(if *name == *GET_PIXEL {
                VariableValue::new_unsigned(0)
            } else {
                VariableValue::new_bool(false)
            });
        }
        if *name == *GET_PIXEL {
            return crate::vm::statements::predefined_procedures::surface_get_pixel(vm, self.handle, arguments).await;
        }
        let handled = crate::vm::statements::predefined_procedures::surface_member(vm, self.handle, name, arguments).await?;
        Ok(VariableValue::new_bool(handled))
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on Surface ({name})");
        Err("Function not found".into())
    }
}
