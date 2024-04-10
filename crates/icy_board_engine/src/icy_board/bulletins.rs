use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use icy_ppe::{tables::import_cp437_string, Res};
use serde::{Deserialize, Serialize};

use super::{is_null_8, IcyBoardSerializer, PCBoardRecordImporter};

#[derive(Serialize, Deserialize, Default)]
pub struct Bullettin {
    pub file: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub required_security: u8,
}

#[derive(Serialize, Deserialize, Default)]
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
            required_security: 0,
        })
    }
}
