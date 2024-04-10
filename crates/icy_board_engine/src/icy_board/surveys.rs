use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use icy_ppe::Res;
use serde::{Deserialize, Serialize};

use super::{is_null_8, IcyBoardSerializer, PCBoardRecordImporter};

/// A survey is a question and answer pair.
/// PCBoard calles them "Questionnairies" but we call them surveys.
#[derive(Serialize, Deserialize, Default)]
pub struct Survey {
    pub question_file: PathBuf,
    pub answer_file: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub required_security: u8,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SurveyList {
    #[serde(rename = "survey")]
    surveys: Vec<Survey>,
}

impl Deref for SurveyList {
    type Target = Vec<Survey>;
    fn deref(&self) -> &Self::Target {
        &self.surveys
    }
}

impl DerefMut for SurveyList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.surveys
    }
}

impl PCBoardRecordImporter<Survey> for SurveyList {
    const RECORD_SIZE: usize = 60;

    fn push(&mut self, value: Survey) {
        self.surveys.push(value);
    }

    fn load_pcboard_record(data: &[u8]) -> Res<Survey> {
        let question_file = PathBuf::from(icy_ppe::tables::import_cp437_string(
            &data[..Self::RECORD_SIZE / 2],
            true,
        ));
        let answer_file = PathBuf::from(icy_ppe::tables::import_cp437_string(
            &data[Self::RECORD_SIZE / 2..],
            true,
        ));
        Ok(Survey {
            question_file,
            answer_file,
            required_security: 0,
        })
    }
}

impl IcyBoardSerializer for SurveyList {
    const FILE_TYPE: &'static str = "surveys";
}
