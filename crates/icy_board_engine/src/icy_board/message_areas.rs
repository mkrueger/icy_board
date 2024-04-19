use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use super::{security::RequiredSecurity, IcyBoardSerializer};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct MessageArea {
    pub name: String,
    pub filename: PathBuf,
    pub read_only: bool,
    pub allow_aliases: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub req_level_to_enter: RequiredSecurity,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub req_level_to_save_attach: RequiredSecurity,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct MessageAreaList {
    #[serde(rename = "area")]
    areas: Vec<MessageArea>,
}

impl MessageAreaList {
    pub fn new(areas: Vec<MessageArea>) -> Self {
        Self { areas }
    }
}

impl Deref for MessageAreaList {
    type Target = Vec<MessageArea>;
    fn deref(&self) -> &Self::Target {
        &self.areas
    }
}

impl DerefMut for MessageAreaList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.areas
    }
}
impl IcyBoardSerializer for MessageAreaList {
    const FILE_TYPE: &'static str = "message areas";
}
