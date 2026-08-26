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
    icy_board::{
        is_null_16,
        state::{
            ppl_error::{ERR_INVALID, ERR_KIND_MSG, PplError},
            ppl_message::{PplMessage, message_error, message_is_missing},
        },
    },
    parser::MSG_ID,
};

/// The `HDR_*` field numbers a `MsgField` names, so `Find` and `SCANMSGHDR` agree.
const HDR_TO: i32 = 0x07;
const HDR_FROM: i32 = 0x0B;
const HDR_SUBJ: i32 = 0x0C;

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

    pub fn get_low_msg(&self) -> u32 {
        JamMessageBase::open(&self.path).map_or(0, |jam| jam.lowest_message_number())
    }

    /// The message with that number, or an invalid one when the area has no such message.
    fn read_message(&self, vm: &mut crate::vm::VirtualMachine<'_>, number: i32) -> VariableValue {
        if number < 0 {
            vm.operation_succeeded();
            return PplMessage::missing();
        }
        let base = match JamMessageBase::open(&self.path) {
            Ok(base) => base,
            Err(error) => {
                vm.set_error(message_error("can't open message base", &self.path, &error));
                return PplMessage::missing();
            }
        };
        match base.read_header(number as u32) {
            Ok(header) => {
                vm.operation_succeeded();
                PplMessage::from_header(&self.path, &header).value()
            }
            Err(error) if message_is_missing(&error) => {
                vm.operation_succeeded();
                PplMessage::missing()
            }
            Err(error) => {
                vm.set_error(message_error(&format!("can't read message {number} from"), &self.path, &error));
                PplMessage::missing()
            }
        }
    }

    /// The first message at or after `start` whose field contains `text`, matched the
    /// way `SCANMSGHDR` matches: without regard to case, anywhere in the field.
    fn find_message(&self, vm: &mut crate::vm::VirtualMachine<'_>, field: i32, text: &str, start: i32) -> VariableValue {
        if !matches!(field, HDR_TO | HDR_FROM | HDR_SUBJ) {
            vm.set_error(PplError::new(ERR_KIND_MSG, ERR_INVALID, format!("unknown message field {field}")));
            return PplMessage::missing();
        }
        let base = match JamMessageBase::open(&self.path) {
            Ok(base) => base,
            Err(error) => {
                vm.set_error(message_error("can't open message base", &self.path, &error));
                return PplMessage::missing();
            }
        };
        let needle = text.to_uppercase();
        let first = (start.max(0) as u32).max(base.lowest_message_number());
        for number in first..=base.highest_message_number() {
            let header = match base.read_header(number) {
                Ok(header) => header,
                Err(error) if message_is_missing(&error) => continue,
                Err(error) => {
                    vm.set_error(message_error(&format!("can't scan message {number} in"), &self.path, &error));
                    return PplMessage::missing();
                }
            };
            let candidate = match field {
                HDR_TO => header.to().map(ToString::to_string),
                HDR_FROM => header.from().map(ToString::to_string),
                HDR_SUBJ => header.subject().map(ToString::to_string),
                _ => unreachable!(),
            };
            if candidate.unwrap_or_default().to_uppercase().contains(&needle) {
                vm.operation_succeeded();
                return PplMessage::from_header(&self.path, &header).value();
            }
        }
        vm.operation_succeeded();
        PplMessage::missing()
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
        registry.add_function(LOW_MSG.clone(), Vec::new(), VariableType::Integer);
        registry.add_function(READ.clone(), vec![VariableType::Integer], VariableType::UserData(MSG_ID as u8));
        registry.add_function_with(
            FIND.clone(),
            vec![
                VariableType::UserData(crate::parser::MSG_FIELD_ENUM_ID),
                VariableType::String,
                VariableType::Integer,
            ],
            2,
            VariableType::UserData(MSG_ID as u8),
        );
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
pub static LOW_MSG: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("LowMsg".to_string()));
pub static READ: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Read".to_string()));
pub static FIND: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Find".to_string()));

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
        arguments: &[VariableValue],
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
            return Ok(match JamMessageBase::open(&self.path) {
                Ok(base) => {
                    vm.operation_succeeded();
                    VariableValue::new_int(base.highest_message_number() as i32)
                }
                Err(error) => {
                    vm.set_error(message_error("can't open message base", &self.path, &error));
                    VariableValue::new_int(0)
                }
            });
        }
        if *name == *LOW_MSG {
            return Ok(match JamMessageBase::open(&self.path) {
                Ok(base) => {
                    vm.operation_succeeded();
                    VariableValue::new_int(base.lowest_message_number() as i32)
                }
                Err(error) => {
                    vm.set_error(message_error("can't open message base", &self.path, &error));
                    VariableValue::new_int(0)
                }
            });
        }
        if *name == *READ {
            return Ok(self.read_message(vm, arguments[0].as_int()));
        }
        if *name == *FIND {
            let field = arguments[0].as_int();
            let text = arguments[1].as_string();
            let start = arguments.get(2).map_or(0, VariableValue::as_int);
            return Ok(self.find_message(vm, field, &text, start));
        }
        log::error!("Invalid function call on MessageArea ({name})");
        Err("Function not found".into())
    }
    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on MessageArea ({name})");
        Err("Function not found".into())
    }
}
