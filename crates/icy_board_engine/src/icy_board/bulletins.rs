use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use crate::{Res, tables::import_cp437_string};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use super::{IcyBoardSerializer, PCBoardRecordImporter, security_expr::SecurityExpression};

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Bullettin {
    pub file: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub required_security: SecurityExpression,
}

impl Bullettin {
    pub fn new(file: &Path) -> Self {
        Self {
            file: file.to_path_buf(),
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
            file: PathBuf::from(file_name),
            required_security: SecurityExpression::default(),
        })
    }
}
