use std::sync::Arc;

use crate::{
    compiler::user_data::user_data_value,
    executable::{VariableType, VariableValue},
    icy_board::{conferences::Conference, doors::DoorList, file_directory::DirectoryList, message_area::AreaList},
    parser::{CONFERENCE_ID, DOOR_ID, FILE_DIRECTORY_ID, MESSAGE_AREA_ID},
};

pub fn area_array_value(items: Arc<AreaList>) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(MESSAGE_AREA_ID as u8),
        items
            .iter()
            .enumerate()
            .map(|(number, item)| {
                let mut item = item.clone();
                item.number = number;
                item.valid = true;
                user_data_value(item, MESSAGE_AREA_ID)
            })
            .collect(),
    )
}

pub fn directory_array_value(items: Arc<DirectoryList>) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(FILE_DIRECTORY_ID as u8),
        items
            .iter()
            .enumerate()
            .map(|(number, item)| {
                let mut item = item.clone();
                item.number = number;
                item.valid = true;
                user_data_value(item, FILE_DIRECTORY_ID)
            })
            .collect(),
    )
}

pub fn door_array_value(items: Arc<DoorList>) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(DOOR_ID as u8),
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
        VariableType::UserData(CONFERENCE_ID as u8),
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
