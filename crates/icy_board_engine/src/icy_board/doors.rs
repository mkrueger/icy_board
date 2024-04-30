use std::ops::{Deref, DerefMut};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{PPEExpr, VariableType, VariableValue},
};

use super::{security::RequiredSecurity, IcyBoardSerializer};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BBSLink {
    pub system_code: String,
    pub auth_code: String,
    pub sheme_code: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum DoorServerAccount {
    BBSLink(BBSLink),
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub enum DoorType {
    #[default]
    Local,
    BBSlink,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Door {
    pub name: String,
    pub description: String,
    pub password: String,
    pub securiy_level: RequiredSecurity,

    pub door_type: DoorType,
    pub path: String,
}

impl UserData for Door {
    const TYPE_NAME: &'static str = "Door";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(NAME.clone(), VariableType::String, false);
        registry.add_property(DESCRIPTION.clone(), VariableType::String, false);
        registry.add_property(PASSWORD.clone(), VariableType::String, false);
        registry.add_function(HAS_ACCESS.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait]
impl UserDataValue for Door {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *NAME {
            return Ok(VariableValue::new_string(self.name.clone()));
        }
        if *name == *DESCRIPTION {
            return Ok(VariableValue::new_string(self.description.clone()));
        }
        if *name == *PASSWORD {
            return Ok(VariableValue::new_string(self.password.clone()));
        }
        log::error!("Invalid user data call on Door ({})", name);
        Ok(VariableValue::new_int(-1))
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        log::error!("Invalid set field call on Door ({})", name);
        Ok(())
    }

    async fn call_function(&self, vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[PPEExpr]) -> crate::Res<VariableValue> {
        if *name == *HAS_ACCESS {
            let res = self.securiy_level.user_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        log::error!("Invalid function call on Door ({})", name);
        Err("Function not found".into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[PPEExpr]) -> crate::Res<()> {
        log::error!("Invalid method call on Door ({})", name);
        Err("Function not found".into())
    }
}

lazy_static::lazy_static! {
    pub static ref NAME: unicase::Ascii<String> = unicase::Ascii::new("Name".to_string());
    pub static ref DESCRIPTION: unicase::Ascii<String> = unicase::Ascii::new("Description".to_string());
    pub static ref PASSWORD: unicase::Ascii<String> = unicase::Ascii::new("Password".to_string());
    pub static ref HAS_ACCESS: unicase::Ascii<String> = unicase::Ascii::new("HasAccess".to_string());
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct DoorList {
    #[serde(rename = "account")]
    pub accounts: Vec<DoorServerAccount>,

    #[serde(rename = "door")]
    pub doors: Vec<Door>,
}

impl Deref for DoorList {
    type Target = Vec<Door>;
    fn deref(&self) -> &Self::Target {
        &self.doors
    }
}

impl DerefMut for DoorList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.doors
    }
}

impl IcyBoardSerializer for DoorList {
    const FILE_TYPE: &'static str = "doors";
}
