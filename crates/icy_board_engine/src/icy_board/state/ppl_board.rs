use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    icy_board::conferences::Conference,
    parser::{BOARD_ID, CONFERENCE_ID},
};

macro_rules! member_name {
    ($name:ident, $value:literal) => {
        static $name: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($value.to_string()));
    };
}

member_name!(NAME, "Name");
member_name!(LOCATION, "Location");
member_name!(OPERATOR, "Operator");
member_name!(SYSOP_NAME, "SysopName");
member_name!(NODES, "NodeCount");
member_name!(CONFERENCES, "ConferenceCount");
member_name!(GET_CONFERENCE, "GetConference");

/// What the board is configured to be, apart from any one call.
#[derive(Clone, Default)]
pub struct PplBoard {
    name: String,
    location: String,
    operator: String,
    sysop_name: String,
    nodes: i32,
    conferences: Vec<Conference>,
}

impl PplBoard {
    pub async fn snapshot(state: &crate::icy_board::state::IcyBoardState) -> Self {
        let board = state.get_board().await;
        Self {
            name: board.config.board.name.clone(),
            location: board.config.board.location.clone(),
            operator: board.config.board.operator.clone(),
            sysop_name: board.config.sysop.name.clone(),
            nodes: i32::from(board.config.board.num_nodes),
            conferences: board.conferences.iter().cloned().collect(),
        }
    }

    pub fn value(self) -> VariableValue {
        user_data_value(self, BOARD_ID)
    }
}

impl UserData for PplBoard {
    const TYPE_NAME: &'static str = "Board";
    const INSTANCE_PROVIDER: Option<crate::executable::FuncOpCode> = Some(crate::executable::FuncOpCode::Board);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        for name in [&*NAME, &*LOCATION, &*OPERATOR, &*SYSOP_NAME] {
            registry.add_property(name.clone(), VariableType::String, false);
        }
        for name in [&*NODES, &*CONFERENCES] {
            registry.add_property(name.clone(), VariableType::Integer, false);
        }
        registry.add_function(GET_CONFERENCE.clone(), vec![VariableType::Integer], VariableType::UserData(CONFERENCE_ID as u8));
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplBoard {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let value = if *name == *NAME {
            VariableValue::new_string(self.name.clone())
        } else if *name == *LOCATION {
            VariableValue::new_string(self.location.clone())
        } else if *name == *OPERATOR {
            VariableValue::new_string(self.operator.clone())
        } else if *name == *SYSOP_NAME {
            VariableValue::new_string(self.sysop_name.clone())
        } else if *name == *NODES {
            VariableValue::new_int(self.nodes)
        } else if *name == *CONFERENCES {
            VariableValue::new_int(self.conferences.len() as i32)
        } else {
            return Err(format!("Unknown BOARD property {name}").into());
        };
        Ok(value)
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("BOARD properties are read-only".into())
    }

    async fn call_function(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *GET_CONFERENCE {
            let number = arguments[0].as_int();
            if number >= 0
                && let Some(conference) = self.conferences.get(number as usize)
            {
                let mut conference = conference.clone();
                conference.number = number as usize;
                conference.valid = true;
                return Ok(user_data_value(conference, CONFERENCE_ID));
            }
            log::error!("PPL: Can't get conference {number} (Board.GetConference)");
            return Ok(user_data_value(Conference::default(), CONFERENCE_ID));
        }
        Err(format!("Unknown BOARD function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown BOARD method {name}").into())
    }
}
