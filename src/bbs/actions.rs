use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Action {
    pub commands: Vec<String>,
    pub help: String,
    pub action: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct Menu {
    pub actions: Vec<Action>,
}

impl Menu {
    pub fn read(str: &str) -> Self {
        toml::from_str(str).unwrap()
    }
}
