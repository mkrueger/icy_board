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
};

use super::{IcyBoardSerializer, security_expr::SecurityExpression};

#[serde_as]
#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageArea {
    pub name: String,
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
        JamMessageBase::open(&self.path)
            .map(|jam| jam.base_messagenumber() + jam.active_messages())
            .unwrap_or(0)
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
        registry.add_function(HAS_ACCESS.clone(), Vec::new(), VariableType::Boolean);
    }
}

lazy_static::lazy_static! {
    pub static ref NAME: unicase::Ascii<String> = unicase::Ascii::new("Name".to_string());
    pub static ref HAS_ACCESS: unicase::Ascii<String> = unicase::Ascii::new("HasAccess".to_string());
}

#[async_trait]
impl UserDataValue for MessageArea {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *NAME {
            return Ok(VariableValue::new_string(self.name.clone()));
        }
        log::error!("Invalid user data call on MessageArea ({})", name);
        Ok(VariableValue::new_int(-1))
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine, name: &unicase::Ascii<String>, val: VariableValue) -> crate::Res<()> {
        if *name == *NAME {
            self.name = val.as_string();
            return Ok(());
        }
        Ok(())
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
        log::error!("Invalid function call on MessageArea ({})", name);
        Err("Function not found".into())
    }
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on MessageArea ({})", name);
        Err("Function not found".into())
    }
}
