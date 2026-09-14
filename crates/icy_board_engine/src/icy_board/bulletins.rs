use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use crate::{
    Res,
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::VariableValue,
    parser::BULLETIN_ID,
    tables::import_cp437_string,
    vm::VirtualMachine,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use super::{IcyBoardSerializer, PCBoardRecordImporter, security_expr::SecurityExpression};

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Bullettin {
    pub path: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub required_security: SecurityExpression,
}

impl Bullettin {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            required_security: SecurityExpression::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct BullettinList {
    #[serde(rename = "bullettin")]
    pub bullettins: Vec<Bullettin>,
}

impl Deref for BullettinList {
    type Target = Vec<Bullettin>;
    fn deref(&self) -> &Self::Target {
        &self.bullettins
    }
}

impl DerefMut for BullettinList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bullettins
    }
}

impl IcyBoardSerializer for BullettinList {
    const FILE_TYPE: &'static str = "bullettins";
}

pub const MASK_BULLETINS: &str = "0123456789ADGHLNRS";

impl PCBoardRecordImporter<Bullettin> for BullettinList {
    const RECORD_SIZE: usize = 30;

    fn push(&mut self, value: Bullettin) {
        self.bullettins.push(value);
    }

    fn load_pcboard_record(data: &[u8]) -> Res<Bullettin> {
        let file_name = import_cp437_string(data, true);
        Ok(Bullettin {
            path: PathBuf::from(file_name),
            required_security: SecurityExpression::default(),
        })
    }
}

#[derive(Clone, Default)]
pub struct PplBulletin {
    pub(crate) number: usize,
    pub(crate) valid: bool,
    pub(crate) bulletin: Bullettin,
    pub(crate) conference_security: SecurityExpression,
}

impl UserData for PplBulletin {
    const TYPE_NAME: &'static str = "Bulletin";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(|| user_data_value(Self::default(), BULLETIN_ID));

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(BULLETIN_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplBulletin {
    fn get_property_value(&self, _vm: &VirtualMachine, name: &unicase::Ascii<String>) -> Res<VariableValue> {
        Ok(match name.as_str().to_ascii_lowercase().as_str() {
            "number" => VariableValue::new_int(self.number as i32),
            "valid" => VariableValue::new_bool(self.valid),
            "path" => VariableValue::new_unbounded_string(self.bulletin.path.to_string_lossy().to_string()),
            _ => return Err(format!("Unknown BULLETIN property {name}").into()),
        })
    }

    async fn set_property_value(&self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> Res<()> {
        Err(format!("BULLETIN property {name} is read-only").into())
    }

    async fn call_function(&self, vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> Res<VariableValue> {
        if name.as_str().eq_ignore_ascii_case("HasAccess") {
            return Ok(VariableValue::new_bool(
                self.valid
                    && self.conference_security.session_can_access(&vm.icy_board_state.session)
                    && self.bulletin.required_security.session_can_access(&vm.icy_board_state.session),
            ));
        }
        Err(format!("Unknown BULLETIN function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> Res<()> {
        Err(format!("Unknown BULLETIN method {name}").into())
    }
}
