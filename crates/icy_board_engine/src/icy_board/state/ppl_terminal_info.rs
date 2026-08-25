use async_trait::async_trait;
use icy_net::termcap_detect::{TerminalCaps, TerminalProgram};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::TERM_INFO_ID,
};

macro_rules! property_name {
    ($name:ident, $value:literal) => {
        static $name: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($value.to_string()));
    };
}

property_name!(PROGRAM, "Program");
property_name!(DEVICE_ATTRS, "DeviceAttrs");
property_name!(COLUMNS, "Columns");
property_name!(ROWS, "Rows");
property_name!(UTF8, "Utf8");
property_name!(RIP_VERSION, "RipVersion");
property_name!(CTERM_LEVEL, "CTermLevel");
property_name!(SIXEL, "Sixel");
property_name!(JXL, "Jxl");
property_name!(INLINE_GRAPHICS, "InlineGraphics");
property_name!(SOUND, "Sound");
property_name!(PHYSICAL_KEYS, "PhysicalKeys");
property_name!(PIXEL_MOUSE, "PixelMouse");
property_name!(CLIENT_BLIT, "ClientBlit");
property_name!(SYNCHRONIZED_OUTPUT, "SynchronizedOutput");
property_name!(TERMINAL_MACROS, "TerminalMacros");
property_name!(CELL_WIDTH, "CellWidth");
property_name!(CELL_HEIGHT, "CellHeight");
property_name!(SCREEN_WIDTH, "ScreenWidth");
property_name!(SCREEN_HEIGHT, "ScreenHeight");

#[derive(Clone, Debug, Default)]
pub struct PplTerminalInfo {
    program: String,
    device_attrs: String,
    columns: i32,
    rows: i32,
    utf8: bool,
    rip_version: String,
    cterm_level: i32,
    sixel: bool,
    jxl: bool,
    inline_graphics: bool,
    sound: bool,
    physical_keys: bool,
    pixel_mouse: bool,
    client_blit: bool,
    synchronized_output: bool,
    terminal_macros: bool,
    cell_width: i32,
    cell_height: i32,
    screen_width: i32,
    screen_height: i32,
}

impl From<&TerminalCaps> for PplTerminalInfo {
    fn from(caps: &TerminalCaps) -> Self {
        let program = match &caps.program {
            TerminalProgram::IcyTerm => "IcyTerm",
            TerminalProgram::SyncTerm => "SyncTerm",
            TerminalProgram::Unknown | TerminalProgram::Name(_) => "Unknown",
        };
        Self {
            program: program.to_string(),
            device_attrs: caps.device_attributes.clone().unwrap_or_default(),
            columns: i32::from(caps.term_size.0),
            rows: i32::from(caps.term_size.1),
            utf8: caps.is_utf8,
            rip_version: caps.rip_version.clone().unwrap_or_default(),
            cterm_level: caps.gfx.cterm_revision.and_then(|value| i32::try_from(value).ok()).unwrap_or_default(),
            sixel: caps.gfx.sixel,
            jxl: caps.gfx.jxl,
            inline_graphics: caps.gfx.inline_blobs(),
            sound: caps.sound,
            physical_keys: caps.gfx.physical_keys,
            pixel_mouse: caps.gfx.cterm_revision.is_some_and(|revision| revision >= 1330),
            client_blit: caps.gfx.cterm_revision.is_some_and(|revision| revision >= 1318),
            synchronized_output: caps.synchronized_output == Some(true),
            terminal_macros: caps.terminal_macros == Some(true),
            cell_width: caps.gfx.cell_width,
            cell_height: caps.gfx.cell_height,
            screen_width: caps.gfx.screen_width,
            screen_height: caps.gfx.screen_height,
        }
    }
}

impl PplTerminalInfo {
    pub fn value(self) -> VariableValue {
        user_data_value(self, TERM_INFO_ID)
    }
}

impl UserData for PplTerminalInfo {
    const TYPE_NAME: &'static str = "TermInfo";
    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        for name in [&*PROGRAM, &*DEVICE_ATTRS, &*RIP_VERSION] {
            registry.add_property(name.clone(), VariableType::String, false);
        }
        for name in [&*COLUMNS, &*ROWS, &*CTERM_LEVEL, &*CELL_WIDTH, &*CELL_HEIGHT, &*SCREEN_WIDTH, &*SCREEN_HEIGHT] {
            registry.add_property(name.clone(), VariableType::Integer, false);
        }
        for name in [
            &*UTF8,
            &*SIXEL,
            &*JXL,
            &*INLINE_GRAPHICS,
            &*SOUND,
            &*PHYSICAL_KEYS,
            &*PIXEL_MOUSE,
            &*CLIENT_BLIT,
            &*SYNCHRONIZED_OUTPUT,
            &*TERMINAL_MACROS,
        ] {
            registry.add_property(name.clone(), VariableType::Boolean, false);
        }
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplTerminalInfo {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let value = if *name == *PROGRAM {
            VariableValue::new_string(self.program.clone())
        } else if *name == *DEVICE_ATTRS {
            VariableValue::new_string(self.device_attrs.clone())
        } else if *name == *COLUMNS {
            VariableValue::new_int(self.columns)
        } else if *name == *ROWS {
            VariableValue::new_int(self.rows)
        } else if *name == *UTF8 {
            VariableValue::new_bool(self.utf8)
        } else if *name == *RIP_VERSION {
            VariableValue::new_string(self.rip_version.clone())
        } else if *name == *CTERM_LEVEL {
            VariableValue::new_int(self.cterm_level)
        } else if *name == *SIXEL {
            VariableValue::new_bool(self.sixel)
        } else if *name == *JXL {
            VariableValue::new_bool(self.jxl)
        } else if *name == *INLINE_GRAPHICS {
            VariableValue::new_bool(self.inline_graphics)
        } else if *name == *SOUND {
            VariableValue::new_bool(self.sound)
        } else if *name == *PHYSICAL_KEYS {
            VariableValue::new_bool(self.physical_keys)
        } else if *name == *PIXEL_MOUSE {
            VariableValue::new_bool(self.pixel_mouse)
        } else if *name == *CLIENT_BLIT {
            VariableValue::new_bool(self.client_blit)
        } else if *name == *SYNCHRONIZED_OUTPUT {
            VariableValue::new_bool(self.synchronized_output)
        } else if *name == *TERMINAL_MACROS {
            VariableValue::new_bool(self.terminal_macros)
        } else if *name == *CELL_WIDTH {
            VariableValue::new_int(self.cell_width)
        } else if *name == *CELL_HEIGHT {
            VariableValue::new_int(self.cell_height)
        } else if *name == *SCREEN_WIDTH {
            VariableValue::new_int(self.screen_width)
        } else if *name == *SCREEN_HEIGHT {
            VariableValue::new_int(self.screen_height)
        } else {
            return Err(format!("Unknown TERMINFO property {name}").into());
        };
        Ok(value)
    }

    fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Ok(())
    }

    async fn call_function(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        Err(format!("Unknown TERMINFO function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown TERMINFO method {name}").into())
    }
}

#[cfg(test)]
mod tests {
    use icy_net::termcap_detect::{GfxCapabilities, TerminalCaps, TerminalProgram};

    use super::PplTerminalInfo;

    #[test]
    fn snapshot_preserves_public_terminal_facts() {
        let caps = TerminalCaps {
            program: TerminalProgram::SyncTerm,
            device_attributes: Some("\x1b[=67;84;101;114;109;1;332c".to_string()),
            term_size: (132, 43),
            is_utf8: true,
            rip_version: Some("3.0".to_string()),
            gfx: GfxCapabilities {
                sixel: true,
                jxl: true,
                physical_keys: true,
                cterm_revision: Some(1332),
                cell_width: 9,
                cell_height: 18,
                screen_width: 1188,
                screen_height: 774,
            },
            sound: true,
            synchronized_output: Some(true),
            terminal_macros: Some(true),
            answered: true,
        };

        let info = PplTerminalInfo::from(&caps);

        assert_eq!(info.program, "SyncTerm");
        assert_eq!(info.device_attrs, "\x1b[=67;84;101;114;109;1;332c");
        assert_eq!((info.columns, info.rows), (132, 43));
        assert_eq!(info.rip_version, "3.0");
        assert_eq!(info.cterm_level, 1332);
        assert!(
            info.utf8 && info.sixel && info.jxl && info.inline_graphics && info.sound && info.physical_keys && info.synchronized_output && info.terminal_macros
        );
        assert_eq!((info.cell_width, info.cell_height), (9, 18));
        assert_eq!((info.screen_width, info.screen_height), (1188, 774));
    }
}
