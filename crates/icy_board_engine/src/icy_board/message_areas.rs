use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{PPEExpr, VariableType, VariableValue},
};

use super::{security::RequiredSecurity, IcyBoardSerializer};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct MessageArea {
    pub name: String,
    pub filename: PathBuf,
    pub read_only: bool,
    pub allow_aliases: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub req_level_to_enter: RequiredSecurity,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub req_level_to_list: RequiredSecurity,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub req_level_to_save_attach: RequiredSecurity,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct MessageAreaList {
    #[serde(rename = "area")]
    areas: Vec<MessageArea>,
}

impl MessageAreaList {
    pub fn new(areas: Vec<MessageArea>) -> Self {
        Self { areas }
    }
}

impl Deref for MessageAreaList {
    type Target = Vec<MessageArea>;
    fn deref(&self) -> &Self::Target {
        &self.areas
    }
}

impl DerefMut for MessageAreaList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.areas
    }
}
impl IcyBoardSerializer for MessageAreaList {
    const FILE_TYPE: &'static str = "message areas";
}

impl UserData for MessageArea {
    const TYPE_NAME: &'static str = "Conference";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_field(NAME.clone(), VariableType::String);
        registry.add_procedure(HAS_ACCESS.clone(), Vec::new());
    }
}

lazy_static::lazy_static! {
    pub static ref NAME: unicase::Ascii<String> = unicase::Ascii::new("Name".to_string());
    pub static ref HAS_ACCESS: unicase::Ascii<String> = unicase::Ascii::new("HasAccess".to_string());
}

impl UserDataValue for MessageArea {
    fn get_field_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *NAME {
            return Ok(VariableValue::new_string(self.name.clone()));
        }
        Ok(VariableValue::new_int(-1))
    }

    fn set_field_value(&mut self, _vm: &mut crate::vm::VirtualMachine, name: &unicase::Ascii<String>, val: VariableValue) -> crate::Res<()> {
        if *name == *NAME {
            self.name = val.as_string();
            return Ok(());
        }
        Ok(())
    }

    fn call_function(&mut self, vm: &mut crate::vm::VirtualMachine, name: &unicase::Ascii<String>, _arguments: &[PPEExpr]) -> crate::Res<VariableValue> {
        if *name == *HAS_ACCESS {
            let res = self.req_level_to_list.user_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        Err("Function not found".into())
    }
    fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine, _name: &unicase::Ascii<String>, _arguments: &[PPEExpr]) -> crate::Res<()> {
        Err("Function not found".into())
    }
}
