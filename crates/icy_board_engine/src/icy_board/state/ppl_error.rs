use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::ERROR_ID,
};

pub const ERR_KIND_NONE: i32 = 0;
pub const ERR_KIND_FILE: i32 = 1;
pub const ERR_KIND_DBASE: i32 = 2;
pub const ERR_KIND_STACK: i32 = 3;
pub const ERR_KIND_GFX: i32 = 4;
pub const ERR_KIND_FONT: i32 = 5;
pub const ERR_KIND_SOUND: i32 = 6;
pub const ERR_KIND_TERM: i32 = 7;
pub const ERR_KIND_MSG: i32 = 8;
pub const ERR_KIND_NET: i32 = 9;
pub const ERR_KIND_USER: i32 = 10;
pub const ERR_KIND_STRING: i32 = 11;
pub const ERR_KIND_REGEX: i32 = 12;

/// What went wrong. The same codes describe every subsystem, so one piece of
/// code can handle a file, a font, a sound or a picture going wrong.
pub const ERR_OK: i32 = 0;
pub const ERR_UNAVAILABLE: i32 = 1;
pub const ERR_INVALID: i32 = 2;
pub const ERR_IO: i32 = 3;
pub const ERR_FORMAT: i32 = 4;
pub const ERR_LIMIT: i32 = 5;
pub const ERR_UNSUPPORTED: i32 = 6;
pub const ERR_STACK: i32 = 7;
pub const ERR_DENIED: i32 = 8;
pub const ERR_TIMEOUT: i32 = 9;

/// What `Channel` answers for an error that is not bound to one.
pub const NO_CHANNEL: i32 = -1;

pub static OK: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("OK".to_string()));
pub static KIND: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Kind".to_string()));
pub static CODE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Code".to_string()));
pub static MESSAGE: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Message".to_string()));
pub static CHANNEL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Channel".to_string()));
pub static LAST: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Last".to_string()));
pub static CLEAR: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Clear".to_string()));

/// What the last operation that can fail did. `Error.Last()` hands a copy of it out,
/// so a PPE can keep one around while it carries on working.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PplError {
    pub kind: i32,
    pub code: i32,
    pub message: String,
    pub channel: i32,
}

impl Default for PplError {
    fn default() -> Self {
        Self {
            kind: ERR_KIND_NONE,
            code: ERR_OK,
            message: String::new(),
            channel: NO_CHANNEL,
        }
    }
}

impl PplError {
    pub fn new(kind: i32, code: i32, message: impl Into<String>) -> Self {
        Self {
            kind,
            code,
            message: message.into(),
            channel: NO_CHANNEL,
        }
    }

    #[must_use]
    pub fn on_channel(mut self, channel: i32) -> Self {
        self.channel = channel;
        self
    }

    pub fn is_ok(&self) -> bool {
        self.code == ERR_OK
    }

    pub fn value(self) -> VariableValue {
        user_data_value(self, ERROR_ID)
    }

    /// What a static member is called on. It carries no error of its own.
    pub fn static_receiver() -> VariableValue {
        Self::default().value()
    }
}

impl UserData for PplError {
    const TYPE_NAME: &'static str = "Error";
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(PplError::static_receiver);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        use crate::parser::{ERR_CODE_ENUM_ID, ERR_KIND_ENUM_ID};

        registry.add_property(OK.clone(), VariableType::Boolean, false);
        registry.add_property(KIND.clone(), VariableType::UserData(ERR_KIND_ENUM_ID), false);
        registry.add_property(CODE.clone(), VariableType::UserData(ERR_CODE_ENUM_ID), false);
        registry.add_property(MESSAGE.clone(), VariableType::UnboundedString, false);
        registry.add_property(CHANNEL.clone(), VariableType::Integer, false);

        registry.add_static_function(LAST.clone(), Vec::new(), VariableType::UserData(ERROR_ID as u8));
        registry.add_static_function(CLEAR.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplError {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *OK {
            return Ok(VariableValue::new_bool(self.is_ok()));
        }
        if *name == *KIND {
            return Ok(VariableValue::new_int(self.kind));
        }
        if *name == *CODE {
            return Ok(VariableValue::new_int(self.code));
        }
        if *name == *MESSAGE {
            return Ok(VariableValue::new_unbounded_string(self.message.clone()));
        }
        if *name == *CHANNEL {
            return Ok(VariableValue::new_int(self.channel));
        }
        Err(format!("Unknown ERROR property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("ERROR property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *LAST {
            vm.publish_operation_result();
            return Ok(vm.last_error.clone().value());
        }
        if *name == *CLEAR {
            vm.clear_error();
            return Ok(VariableValue::new_bool(true));
        }
        Err(format!("Unknown ERROR function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown ERROR method {name}").into())
    }
}
