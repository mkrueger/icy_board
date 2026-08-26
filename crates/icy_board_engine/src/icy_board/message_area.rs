use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use async_trait::async_trait;
use jamjam::jam::JamMessageBase;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{VariableType, VariableValue},
    icy_board::is_null_16,
};

use super::{IcyBoardSerializer, security_expr::SecurityExpression};

#[serde_as]
#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageArea {
    pub name: String,

    /// Set when the area is handed to a PPE, so the object can report where it sits.
    #[serde(skip)]
    pub number: usize,

    #[serde(skip)]
    pub valid: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub qwk_name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub qwk_conference_number: u16,

    /// The tag a fidonet technology network knows this area under, empty when
    /// the area is local.
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ftn_area_tag: String,

    pub path: PathBuf,
    pub is_read_only: bool,
    pub allow_aliases: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub req_level_to_enter: SecurityExpression,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub req_level_to_list: SecurityExpression,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub req_level_to_save_attach: SecurityExpression,
}

impl MessageArea {
    pub fn get_high_msg(&self) -> u32 {
        JamMessageBase::open(&self.path).map_or(0, |jam| jam.highest_message_number())
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct AreaList {
    #[serde(rename = "area")]
    areas: Vec<MessageArea>,
}

impl AreaList {
    pub fn new(areas: Vec<MessageArea>) -> Self {
        Self { areas }
    }
}

impl Deref for AreaList {
    type Target = Vec<MessageArea>;
    fn deref(&self) -> &Self::Target {
        &self.areas
    }
}

impl DerefMut for AreaList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.areas
    }
}
impl IcyBoardSerializer for AreaList {
    const FILE_TYPE: &'static str = "message areas";
}

impl UserData for MessageArea {
    const TYPE_NAME: &'static str = "Area";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(NAME.clone(), VariableType::String, false);
        registry.add_property(NUMBER.clone(), VariableType::Integer, false);
        registry.add_property(VALID.clone(), VariableType::Boolean, false);
        registry.add_property(IS_READ_ONLY.clone(), VariableType::Boolean, false);
        registry.add_property(ALLOW_ALIASES.clone(), VariableType::Boolean, false);
        registry.add_property(QWK_NAME.clone(), VariableType::String, false);
        registry.add_property(ECHO_TAG.clone(), VariableType::String, false);
        registry.add_function(HAS_ACCESS.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(CAN_ENTER.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(CAN_ATTACH.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(HIGH_MSG.clone(), Vec::new(), VariableType::Integer);
    }
}

pub static NAME: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Name".to_string()));
pub static NUMBER: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Number".to_string()));
pub static VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Valid".to_string()));
pub static IS_READ_ONLY: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("IsReadOnly".to_string()));
pub static ALLOW_ALIASES: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("AllowAliases".to_string()));
pub static QWK_NAME: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("QwkName".to_string()));
pub static ECHO_TAG: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("EchoTag".to_string()));
pub static HAS_ACCESS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("HasAccess".to_string()));
pub static CAN_ENTER: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("CanEnter".to_string()));
pub static CAN_ATTACH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("CanAttach".to_string()));
pub static HIGH_MSG: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("HighMsg".to_string()));

#[async_trait(?Send)]
impl UserDataValue for MessageArea {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *NAME {
            return Ok(VariableValue::new_string(self.name.clone()));
        }
        if *name == *NUMBER {
            return Ok(VariableValue::new_int(self.number as i32));
        }
        if *name == *VALID {
            return Ok(VariableValue::new_bool(self.valid));
        }
        if *name == *IS_READ_ONLY {
            return Ok(VariableValue::new_bool(self.is_read_only));
        }
        if *name == *ALLOW_ALIASES {
            return Ok(VariableValue::new_bool(self.allow_aliases));
        }
        if *name == *QWK_NAME {
            return Ok(VariableValue::new_string(self.qwk_name.clone()));
        }
        if *name == *ECHO_TAG {
            return Ok(VariableValue::new_string(self.ftn_area_tag.clone()));
        }
        log::error!("Invalid user data call on MessageArea ({name})");
        Ok(VariableValue::new_int(-1))
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("AREA property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *HAS_ACCESS {
            let res = self.req_level_to_list.session_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        if *name == *CAN_ENTER {
            let res = self.req_level_to_enter.session_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        if *name == *CAN_ATTACH {
            let res = self.req_level_to_save_attach.session_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        if *name == *HIGH_MSG {
            return Ok(VariableValue::new_int(self.get_high_msg() as i32));
        }
        log::error!("Invalid function call on MessageArea ({name})");
        Err("Function not found".into())
    }
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on MessageArea ({name})");
        Err("Function not found".into())
    }
}
