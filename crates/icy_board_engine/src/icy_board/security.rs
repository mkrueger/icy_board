use serde::{Deserialize, Serialize};

use super::state::Session;

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequiredSecurity {
    pub level: u8,
    pub min_age: u8,
    pub groups: Vec<String>,
}

impl RequiredSecurity {
    pub fn new(sec_level: u8) -> Self {
        Self {
            level: sec_level,
            min_age: 0,
            groups: Vec::new(),
        }
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn is_empty(&self) -> bool {
        self.level == 0 && self.min_age == 0 && self.groups.is_empty()
    }

    pub fn user_can_access(&self, session: &Session) -> bool {
        if session.is_sysop {
            return true;
        }

        if session.cur_security < self.level {
            return false;
        }

        if session.cur_groups.contains(&"no_age".to_string()) {
            // Todo: Age check
        }

        if self.groups.is_empty() {
            return true;
        }

        for sg in &session.cur_groups {
            for group in &self.groups {
                if sg == group {
                    return true;
                }
            }
        }
        false
    }
}
