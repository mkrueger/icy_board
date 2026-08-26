use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::{BOARD_ID, CONFERENCES_ID},
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
member_name!(CONFERENCES, "Conferences");

/// What the board is configured to be, apart from any one call.
#[derive(Clone, Default)]
pub struct PplBoard {
    name: String,
    location: String,
    operator: String,
    sysop_name: String,
    nodes: i32,
    /// The ready made `Conferences` value: nothing about it can change during a run,
    /// so handing it out is a share rather than a fresh object every time.
    conferences: VariableValue,
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
            conferences: crate::icy_board::state::ppl_collection::PplConferences::new(crate::icy_board::state::ppl_collection::PplConferences::build(
                &board.conferences,
            ))
            .value(),
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
        registry.add_property(NODES.clone(), VariableType::Integer, false);
        registry.add_property(CONFERENCES.clone(), VariableType::UserData(CONFERENCES_ID as u8), false);
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
            self.conferences.clone()
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
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        Err(format!("Unknown BOARD function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown BOARD method {name}").into())
    }
}
