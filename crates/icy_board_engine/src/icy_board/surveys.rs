use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use crate::{
    Res,
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::VariableValue,
    parser::SURVEY_ID,
    vm::VirtualMachine,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use super::{IcyBoardSerializer, PCBoardRecordImporter, security_expr::SecurityExpression};

/// A survey is a question and answer pair.
/// `PCBoard` calles them "Questionnairies" but we call them surveys.
#[serde_as]
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Survey {
    pub survey_file: PathBuf,
    pub answer_file: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub required_security: SecurityExpression,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
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

#[derive(Clone, Default)]
pub struct PplSurvey {
    pub(crate) number: usize,
    pub(crate) valid: bool,
    pub(crate) survey: Survey,
    pub(crate) conference_security: SecurityExpression,
}

impl UserData for PplSurvey {
    const TYPE_NAME: &'static str = "Survey";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(|| user_data_value(Self::default(), SURVEY_ID));

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(SURVEY_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplSurvey {
    fn get_property_value(&self, _vm: &VirtualMachine, name: &unicase::Ascii<String>) -> Res<VariableValue> {
        Ok(match name.as_str().to_ascii_lowercase().as_str() {
            "number" => VariableValue::new_int(self.number as i32),
            "valid" => VariableValue::new_bool(self.valid),
            "path" => VariableValue::new_unbounded_string(self.survey.survey_file.to_string_lossy().to_string()),
            "answerfile" => VariableValue::new_unbounded_string(self.survey.answer_file.to_string_lossy().to_string()),
            _ => return Err(format!("Unknown SURVEY property {name}").into()),
        })
    }

    async fn set_property_value(&self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> Res<()> {
        Err(format!("SURVEY property {name} is read-only").into())
    }

    async fn call_function(&self, vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> Res<VariableValue> {
        if name.as_str().eq_ignore_ascii_case("HasAccess") {
            return Ok(VariableValue::new_bool(
                self.valid
                    && self.conference_security.session_can_access(&vm.icy_board_state.session)
                    && self.survey.required_security.session_can_access(&vm.icy_board_state.session),
            ));
        }
        Err(format!("Unknown SURVEY function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> Res<()> {
        Err(format!("Unknown SURVEY method {name}").into())
    }
}
