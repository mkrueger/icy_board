use std::sync::Arc;

use crate::{
    compiler::user_data::user_data_value,
    executable::{VariableType, VariableValue},
    icy_board::{bulletins::PplBulletin, conferences::Conference, doors::DoorList, file_directory::DirectoryList, message_area::AreaList, surveys::PplSurvey},
    parser::{BULLETIN_ID, CONFERENCE_ID, DOOR_ID, FILE_DIRECTORY_ID, MESSAGE_AREA_ID, SURVEY_ID},
};

pub fn bulletin_array_value(conference: &Conference) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(BULLETIN_ID as u32),
        conference
            .bulletins
            .iter()
            .flat_map(|list| list.iter())
            .enumerate()
            .map(|(number, bulletin)| {
                user_data_value(
                    PplBulletin {
                        number,
                        valid: conference.valid,
                        bulletin: bulletin.clone(),
                        conference_security: conference.required_security.clone(),
                    },
                    BULLETIN_ID,
                )
            })
            .collect(),
    )
}

pub fn survey_array_value(conference: &Conference) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(SURVEY_ID as u32),
        conference
            .surveys
            .iter()
            .flat_map(|list| list.iter())
            .enumerate()
            .map(|(number, survey)| {
                user_data_value(
                    PplSurvey {
                        number,
                        valid: conference.valid,
                        survey: survey.clone(),
                        conference_security: conference.required_security.clone(),
                    },
                    SURVEY_ID,
                )
            })
            .collect(),
    )
}

pub fn area_array_value(items: Arc<AreaList>, conference: usize) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(MESSAGE_AREA_ID as u32),
        items
            .iter()
            .enumerate()
            .map(|(number, item)| {
                let mut item = item.clone();
                item.number = number;
                item.conference_number = conference;
                item.valid = true;
                user_data_value(item, MESSAGE_AREA_ID)
            })
            .collect(),
    )
}

pub fn directory_array_value(items: Arc<DirectoryList>, conference: usize) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(FILE_DIRECTORY_ID as u32),
        items
            .iter()
            .enumerate()
            .map(|(number, item)| {
                let mut item = item.clone();
                item.number = number;
                item.valid = true;
                item.conference_number = conference;
                user_data_value(item, FILE_DIRECTORY_ID)
            })
            .collect(),
    )
}

pub fn door_array_value(items: Arc<DoorList>) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(DOOR_ID as u32),
        items
            .iter()
            .enumerate()
            .map(|(number, item)| {
                let mut item = item.clone();
                item.number = number;
                item.valid = true;
                user_data_value(item, DOOR_ID)
            })
            .collect(),
    )
}

pub fn conference_array_value(conferences: &[Conference]) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(CONFERENCE_ID as u32),
        conferences
            .iter()
            .enumerate()
            .map(|(number, conference)| {
                let mut conference = conference.clone();
                conference.number = number;
                conference.valid = true;
                user_data_value(conference, CONFERENCE_ID)
            })
            .collect(),
    )
}
