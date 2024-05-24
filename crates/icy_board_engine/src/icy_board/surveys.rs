use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use crate::Res;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use super::{security_expr::SecurityExpression, IcyBoardSerializer, PCBoardRecordImporter};

/// A survey is a question and answer pair.
/// PCBoard calles them "Questionnairies" but we call them surveys.
#[serde_as]
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Survey {
    pub survey_file: PathBuf,
    pub answer_file: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub required_security: SecurityExpression,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SurveyList {
    #[serde(rename = "survey")]
    pub surveys: Vec<Survey>,
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
        let survey_file = PathBuf::from(crate::tables::import_cp437_string(&data[..Self::RECORD_SIZE / 2], true));
        let answer_file = PathBuf::from(crate::tables::import_cp437_string(&data[Self::RECORD_SIZE / 2..], true));
        Ok(Survey {
            survey_file,
            answer_file,
            required_security: SecurityExpression::default(),
        })
    }
}

impl IcyBoardSerializer for SurveyList {
    const FILE_TYPE: &'static str = "surveys";
}
