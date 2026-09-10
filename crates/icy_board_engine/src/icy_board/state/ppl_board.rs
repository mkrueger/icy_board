use async_trait::async_trait;

use std::sync::{Arc, OnceLock};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::VariableValue,
    icy_board::{conferences::Conference, snapshot::Snapshot, user_base::User},
    parser::BOARD_ID,
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
member_name!(USERS, "Users");

/// What the board is configured to be, apart from any one call.
#[derive(Clone, Default)]
pub struct PplBoard {
    name: String,
    location: String,
    operator: String,
    sysop_name: String,
    nodes: i32,
    /// Both collections are captured as they were when the board was first read,
    /// but turned into PPL values only when a program asks for them. Reading
    /// `Board.Name` on a large user base must not pay for `Board.Users`.
    conferences: Arc<Vec<Conference>>,
    users: Snapshot<Vec<User>>,
    conference_value: Arc<OnceLock<VariableValue>>,
    user_value: Arc<OnceLock<VariableValue>>,
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
            conferences: Arc::new(board.conferences.to_vec()),
            users: board.users.snapshot(),
            conference_value: Arc::default(),
            user_value: Arc::default(),
        }
    }

    pub fn value(self) -> VariableValue {
        user_data_value(self, BOARD_ID)
    }
}

impl UserData for PplBoard {
    const TYPE_NAME: &'static str = "Board";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(|| Self::default().value());
    const INSTANCE_PROVIDER: Option<crate::executable::FuncOpCode> = Some(crate::executable::FuncOpCode::Board);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(BOARD_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplBoard {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let value = if *name == *NAME {
            VariableValue::new_unbounded_string(self.name.clone())
        } else if *name == *LOCATION {
            VariableValue::new_unbounded_string(self.location.clone())
        } else if *name == *OPERATOR {
            VariableValue::new_unbounded_string(self.operator.clone())
        } else if *name == *SYSOP_NAME {
            VariableValue::new_unbounded_string(self.sysop_name.clone())
        } else if *name == *NODES {
            VariableValue::new_int(self.nodes)
        } else if *name == *CONFERENCES {
            self.conference_value
                .get_or_init(|| crate::icy_board::state::ppl_array::conference_array_value(&self.conferences))
                .clone()
        } else if *name == *USERS {
            self.user_value
                .get_or_init(|| crate::icy_board::state::ppl_user::user_array_value(&self.users))
                .clone()
        } else {
            return Err(format!("Unknown BOARD property {name}").into());
        };
        Ok(value)
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("BOARD property {name} is read-only").into())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn board_with(users: usize) -> PplBoard {
        let mut list = Vec::new();
        for index in 0..users {
            let mut user = User::default();
            user.set_name(format!("User {index}"));
            list.push(user);
        }
        PplBoard {
            name: "Icy Board".to_string(),
            users: Snapshot::from(list),
            ..PplBoard::default()
        }
    }

    /// F6: reading board metadata must not turn the whole user base into PPL values.
    #[test]
    fn metadata_does_not_materialize_the_user_base() {
        let board = board_with(3);
        assert!(board.user_value.get().is_none());

        let name = board.name.clone();
        assert_eq!(name, "Icy Board");
        assert!(board.user_value.get().is_none(), "reading metadata built the user array");

        let users = board
            .user_value
            .get_or_init(|| crate::icy_board::state::ppl_user::user_array_value(&board.users));
        assert_eq!(users.get_vector_size() + 1, 3);
        assert!(board.user_value.get().is_some());
    }

    /// The captured list is the one the board had when it was first read, and
    /// later reads of the same board hand out that same value.
    #[test]
    fn the_captured_list_is_frozen_and_shared() {
        let board = board_with(2);
        let first = board
            .user_value
            .get_or_init(|| crate::icy_board::state::ppl_user::user_array_value(&board.users));
        let again = board.user_value.get().unwrap();
        assert_eq!(first.get_vector_size(), again.get_vector_size());

        let copy = board.clone();
        assert!(copy.user_value.get().is_some(), "a clone shares the already built array");
    }
}
