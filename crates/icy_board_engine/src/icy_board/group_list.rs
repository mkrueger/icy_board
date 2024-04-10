use serde::{Deserialize, Serialize};

use super::IcyBoardSerializer;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Group {
    pub name: String,
    pub description: String,
    pub members: Vec<i64>,
}

#[derive(Default, Serialize, Deserialize, Clone, PartialEq, Debug)]

pub struct GroupList {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "group")]
    groups: Vec<Group>,
}
impl GroupList {
    pub fn add_group(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        members: &[i64],
    ) {
        self.groups.push(Group {
            name: name.into(),
            description: description.into(),
            members: members.to_vec(),
        })
    }
}

impl IcyBoardSerializer for GroupList {
    const FILE_TYPE: &'static str = "groups";
}
