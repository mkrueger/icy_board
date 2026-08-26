use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    icy_board::{
        conferences::Conference,
        doors::{Door, DoorList},
        file_directory::{DirectoryList, FileDirectory},
        message_area::{AreaList, MessageArea},
    },
    parser::{AREAS_ID, CONFERENCE_ID, CONFERENCES_ID, DIRECTORIES_ID, DOOR_ID, DOORS_ID, FILE_DIRECTORY_ID, MESSAGE_AREA_ID},
};

/// What a collection is counted by.
pub static COUNT: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Count".to_string()));

/// What `collection[index]` reads with. Angle brackets keep it out of reach of source,
/// so the index is the only way to write it.
pub static GET: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("<get>".to_string()));

/// A list the board hands to a PPE. It shares the list rather than copying it, so
/// reading the same collection once per loop step stays cheap.
macro_rules! ppl_collection {
    ($name:ident, $type_name:literal, $container:ty, $item:ty, $collection_id:expr, $item_id:expr) => {
        #[derive(Clone, Default)]
        pub struct $name(std::sync::Arc<$container>);

        impl $name {
            pub fn new(items: std::sync::Arc<$container>) -> Self {
                Self(items)
            }

            pub fn value(self) -> VariableValue {
                user_data_value(self, $collection_id)
            }
        }

        impl UserData for $name {
            const TYPE_NAME: &'static str = $type_name;

            fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
                registry.add_property(COUNT.clone(), VariableType::Integer, false);
                registry.add_function(GET.clone(), vec![VariableType::Integer], VariableType::UserData($item_id as u8));
            }
        }

        #[async_trait(?Send)]
        impl UserDataValue for $name {
            fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
                if *name == *COUNT {
                    return Ok(VariableValue::new_int(self.0.len() as i32));
                }
                Err(format!("Unknown {} property {name}", $type_name).into())
            }

            async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
                Err(format!("{} property {name} is read-only", $type_name).into())
            }

            async fn call_function(
                &self,
                _vm: &mut crate::vm::VirtualMachine<'_>,
                name: &unicase::Ascii<String>,
                arguments: &[VariableValue],
            ) -> crate::Res<VariableValue> {
                if *name == *GET {
                    let index = arguments[0].as_int();
                    if index >= 0
                        && let Some(item) = self.0.get(index as usize)
                    {
                        let mut item: $item = item.clone();
                        item.number = index as usize;
                        item.valid = true;
                        return Ok(user_data_value(item, $item_id));
                    }
                    // An index nobody has answers with an invalid item rather than failing,
                    // the way the accessors it replaces did.
                    return Ok(user_data_value(<$item>::default(), $item_id));
                }
                Err(format!("Unknown {} function {name}", $type_name).into())
            }

            async fn call_method(
                &mut self,
                _vm: &mut crate::vm::VirtualMachine<'_>,
                name: &unicase::Ascii<String>,
                _arguments: &[VariableValue],
            ) -> crate::Res<()> {
                Err(format!("Unknown {} method {name}", $type_name).into())
            }
        }
    };
}

ppl_collection!(PplAreas, "Areas", AreaList, MessageArea, AREAS_ID, MESSAGE_AREA_ID);
ppl_collection!(PplDirectories, "Directories", DirectoryList, FileDirectory, DIRECTORIES_ID, FILE_DIRECTORY_ID);
ppl_collection!(PplDoors, "Doors", DoorList, Door, DOORS_ID, DOOR_ID);
ppl_collection!(PplConferences, "Conferences", Vec<Conference>, Conference, CONFERENCES_ID, CONFERENCE_ID);
