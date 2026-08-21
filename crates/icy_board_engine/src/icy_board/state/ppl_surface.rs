//! The `SURFACE` object a PPE draws on.
//!
//! The value a PPE holds is only the name the engine gave the surface; the pixels
//! live in `PplGraphicsState`. That keeps the object itself immutable, so a member
//! reaches the surface through the VM rather than through the handle.

use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{GenericVariableData, VariableData, VariableType, VariableValue},
    parser::SURFACE_ID,
};

#[derive(Clone, Copy)]
pub struct PplSurface {
    pub handle: i32,
}

impl PplSurface {
    pub fn value(handle: i32) -> VariableValue {
        VariableValue {
            vtype: VariableType::UserData(SURFACE_ID as u8),
            // The handle rides in the data word as well, so a surface passed to
            // another surface's member can be named without downcasting the object.
            data: VariableData::from_int(handle),
            generic_data: GenericVariableData::UserData(std::sync::Arc::new(PplSurface { handle })),
        }
    }

    /// An answer for a surface that could not be made, so its members stay callable.
    pub fn invalid() -> VariableValue {
        Self::value(0)
    }
}

/// The handle inside a `SURFACE` argument.
pub fn surface_handle(value: &VariableValue) -> Option<i32> {
    if value.get_type() != VariableType::UserData(SURFACE_ID as u8) {
        return None;
    }
    Some(unsafe { value.data.int_value })
}

pub static WIDTH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Width".to_string()));
pub static HEIGHT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Height".to_string()));
pub static VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Valid".to_string()));
pub static CLEAR: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Clear".to_string()));
pub static SET_PIXEL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("SetPixel".to_string()));
pub static FILL_RECT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("FillRect".to_string()));
pub static RECT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Rect".to_string()));
pub static BLIT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Blit".to_string()));
pub static BLIT_RECT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("BlitRect".to_string()));
pub static PRESENT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Present".to_string()));
pub static PRESENT_AT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("PresentAt".to_string()));
pub static PRESENT_RECT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("PresentRect".to_string()));
pub static PIN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Pin".to_string()));
pub static UNPIN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Unpin".to_string()));
pub static FREE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Free".to_string()));

impl UserData for PplSurface {
    const TYPE_NAME: &'static str = "Surface";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        let surface = VariableType::UserData(SURFACE_ID as u8);

        registry.add_property(WIDTH.clone(), VariableType::Integer, false);
        registry.add_property(HEIGHT.clone(), VariableType::Integer, false);
        registry.add_property(VALID.clone(), VariableType::Boolean, false);

        registry.add_function(CLEAR.clone(), vec![VariableType::Unsigned], VariableType::Boolean);
        registry.add_function(
            SET_PIXEL.clone(),
            vec![VariableType::Integer, VariableType::Integer, VariableType::Unsigned],
            VariableType::Boolean,
        );
        registry.add_function(
            FILL_RECT.clone(),
            vec![
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Unsigned,
            ],
            VariableType::Boolean,
        );
        registry.add_function(
            RECT.clone(),
            vec![
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Unsigned,
            ],
            VariableType::Boolean,
        );
        registry.add_function(BLIT.clone(), vec![surface, VariableType::Integer, VariableType::Integer], VariableType::Boolean);
        registry.add_function(
            BLIT_RECT.clone(),
            vec![
                surface,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
            ],
            VariableType::Boolean,
        );
        registry.add_function(PRESENT.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(PRESENT_AT.clone(), vec![VariableType::Integer, VariableType::Integer], VariableType::Boolean);
        // Source rectangle is required; destination, size and flip are not.
        registry.add_function_with(
            PRESENT_RECT.clone(),
            vec![
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
                VariableType::Integer,
            ],
            4,
            VariableType::Boolean,
        );
        registry.add_function(PIN.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(UNPIN.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(FREE.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait]
impl UserDataValue for PplSurface {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let surface = vm
            .icy_board_state
            .ppl_graphics
            .as_ref()
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

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Ok(())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        let handled = crate::vm::statements::predefined_procedures::surface_member(vm, self.handle, name, arguments).await?;
        Ok(VariableValue::new_bool(handled))
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on Surface ({name})");
        Err("Function not found".into())
    }
}
