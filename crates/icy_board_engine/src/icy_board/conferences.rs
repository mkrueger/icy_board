use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{GenericVariableData, VariableData, VariableType, VariableValue},
    icy_board::{doors::Door, file_directory::FileDirectory},
    parser::{DOOR_ID, FILE_DIRECTORY_ID, MESSAGE_AREA_ID},
};

use super::{
    commands::Command,
    doors::DoorList,
    file_directory::DirectoryList,
    is_false, is_null_16, is_null_8, is_null_i32,
    message_area::{AreaList, MessageArea},
    pcbconferences::{PcbAdditionalConferenceHeader, PcbConferenceHeader},
    security_expr::SecurityExpression,
    user_base::Password,
    IcyBoardSerializer,
};

#[serde_as]
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Conference {
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub is_public: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub is_read_only: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "Password::is_empty")]
    pub password: Password,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub required_security: SecurityExpression,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub sec_attachments: SecurityExpression,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub sec_write_message: SecurityExpression,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub auto_rejoin: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub view_members: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub private_uploads: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub no_private_msgs: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub allow_aliases: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_i32")]
    pub add_conference_security: i32,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub add_conference_time: u16,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_main_commands: bool,

    pub users_menu: PathBuf,
    pub sysop_menu: PathBuf,
    pub news_file: PathBuf,
    pub attachment_location: PathBuf,

    /// Sort type for public upload DIR file
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub pub_upload_sort: u8,
    pub pub_upload_dir_file: PathBuf,
    pub pub_upload_location: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub private_upload_sort: u8,
    pub private_upload_dir_file: PathBuf,
    pub private_upload_location: PathBuf,

    pub command_file: PathBuf,
    pub intro_file: PathBuf,
    pub doors_menu: PathBuf,
    pub doors_file: PathBuf,

    pub blt_menu: PathBuf,
    pub blt_file: PathBuf,

    pub survey_menu: PathBuf,
    pub survey_file: PathBuf,

    pub dir_menu: PathBuf,
    pub dir_file: PathBuf,

    pub area_menu: PathBuf,
    pub area_file: PathBuf,

    #[serde(skip)]
    pub commands: Vec<Command>,

    #[serde(skip)]
    pub areas: AreaList,

    #[serde(skip)]
    pub directories: DirectoryList,

    #[serde(skip)]
    pub doors: DoorList,
}

impl Conference {}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ConferenceBase {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "conference")]
    entries: Vec<Conference>,
}

impl ConferenceBase {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn import_pcboard(output_directory: &Path, conferences: &[PcbConferenceHeader], add_conferences: &[PcbAdditionalConferenceHeader]) -> ConferenceBase {
        let mut confs = Vec::new();
        for (i, c) in conferences.iter().enumerate() {
            let d = &add_conferences[i];

            let general_area = MessageArea {
                name: "General".to_string(),
                filename: PathBuf::from(&c.message_file),
                is_read_only: d.read_only,
                allow_aliases: d.allow_aliases,
                req_level_to_list: SecurityExpression::from_req_security(d.req_level_to_enter),
                req_level_to_enter: SecurityExpression::from_req_security(d.req_level_to_enter),
                req_level_to_save_attach: SecurityExpression::from_req_security(d.attach_level),
            };
            let output = if i == 0 { "conferences/main".to_string() } else { format!("conferences/{i}") };
            let destination = output_directory.join(&output);
            std::fs::create_dir_all(&destination).unwrap();

            let areas = AreaList::new(vec![general_area]);
            areas.save(&destination.join(&"area.toml")).unwrap();

            let new = Conference {
                name: c.name.clone(),
                is_public: c.public_conference,
                is_read_only: d.read_only,
                use_main_commands: true,
                commands: Vec::new(),
                password: Password::PlainText(d.password.clone()),
                required_security: SecurityExpression::from_req_security(c.required_security),
                sec_attachments: SecurityExpression::from_req_security(d.attach_level),
                sec_write_message: SecurityExpression::from_req_security(d.req_level_to_enter),
                auto_rejoin: c.auto_rejoin,
                view_members: c.view_members,
                private_uploads: c.private_uploads,
                no_private_msgs: c.private_msgs,
                allow_aliases: d.allow_aliases,
                add_conference_security: c.add_conference_security,
                add_conference_time: c.add_conference_time as u16,
                users_menu: PathBuf::from(&c.users_menu),
                sysop_menu: PathBuf::from(&c.sysop_menu),
                news_file: PathBuf::from(&c.news_file),
                attachment_location: PathBuf::from(&d.attach_loc),
                pub_upload_sort: c.pub_upload_sort,
                pub_upload_dir_file: PathBuf::from(&c.pub_upload_dirfile),
                pub_upload_location: PathBuf::from(&c.pub_upload_location),
                private_upload_sort: c.private_upload_sort,
                private_upload_dir_file: PathBuf::from(&c.private_upload_dirfile),
                private_upload_location: PathBuf::from(&c.private_upload_location),
                command_file: PathBuf::from(&d.cmd_lst),
                intro_file: PathBuf::from(&d.intro),
                doors_menu: PathBuf::from(&c.doors_menu),
                doors_file: PathBuf::from(&c.doors_file),
                blt_menu: PathBuf::from(&c.blt_menu),
                blt_file: PathBuf::from(&c.blt_file),
                survey_menu: PathBuf::from(&c.script_menu),
                survey_file: PathBuf::from(&c.script_file),
                dir_menu: PathBuf::from(&c.dir_menu),
                dir_file: PathBuf::from(&c.dir_file),
                area_menu: PathBuf::from("area"),
                area_file: PathBuf::from("area.toml"),
                areas: AreaList::default(),
                directories: DirectoryList::default(),
                doors: DoorList::default(),
            };
            confs.push(new);
        }
        Self { entries: confs }
    }

    pub fn get(&self, index: usize) -> Option<&Conference> {
        self.entries.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Conference> {
        self.entries.get_mut(index)
    }
}

impl Deref for ConferenceBase {
    type Target = Vec<Conference>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for ConferenceBase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

impl IcyBoardSerializer for ConferenceBase {
    const FILE_TYPE: &'static str = "conferences";
}

impl<'a> UserData for Conference {
    const TYPE_NAME: &'static str = "Conference";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(NAME.clone(), VariableType::String, false);
        registry.add_property(ISPUBLIC.clone(), VariableType::Boolean, false);
        registry.add_property(FILE_AREAS.clone(), VariableType::Boolean, false);
        registry.add_property(MESSAGE_AREAS.clone(), VariableType::Boolean, false);
        registry.add_property(DOORS.clone(), VariableType::Boolean, false);

        registry.add_function(HAS_ACCESS.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(
            GET_FILE_AREA.clone(),
            vec![VariableType::Integer],
            VariableType::UserData(FILE_DIRECTORY_ID as u8),
        );
        registry.add_function(GET_MSG_AREA.clone(), vec![VariableType::Integer], VariableType::UserData(MESSAGE_AREA_ID as u8));
        registry.add_function(GET_DOOR.clone(), vec![VariableType::Integer], VariableType::UserData(DOOR_ID as u8));
    }
}

lazy_static::lazy_static! {
    pub static ref NAME: unicase::Ascii<String> = unicase::Ascii::new("Name".to_string());
    pub static ref ISPUBLIC: unicase::Ascii<String> = unicase::Ascii::new("IsPublic".to_string());
    pub static ref FILE_AREAS: unicase::Ascii<String> = unicase::Ascii::new("Directories".to_string());
    pub static ref DOORS: unicase::Ascii<String> = unicase::Ascii::new("Doors".to_string());
    pub static ref MESSAGE_AREAS: unicase::Ascii<String> = unicase::Ascii::new("Areas".to_string());

    pub static ref HAS_ACCESS: unicase::Ascii<String> = unicase::Ascii::new("HasAccess".to_string());
    pub static ref GET_FILE_AREA: unicase::Ascii<String> = unicase::Ascii::new("GetDir".to_string());
    pub static ref GET_MSG_AREA: unicase::Ascii<String> = unicase::Ascii::new("GetArea".to_string());
    pub static ref GET_DOOR: unicase::Ascii<String> = unicase::Ascii::new("GetDoor".to_string());
}

#[async_trait]
impl UserDataValue for Conference {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *NAME {
            return Ok(VariableValue::new_string(self.name.clone()));
        }
        if *name == *ISPUBLIC {
            return Ok(VariableValue::new_bool(self.required_security.user_can_access(&vm.icy_board_state.session)));
        }
        if *name == *FILE_AREAS {
            return Ok(VariableValue::new_int(self.directories.len() as i32));
        }
        if *name == *MESSAGE_AREAS {
            return Ok(VariableValue::new_int(self.areas.len() as i32));
        }
        if *name == *DOORS {
            return Ok(VariableValue::new_int(self.doors.len() as i32));
        }

        log::error!("Invalid user data call on Conference ({})", name);
        Ok(VariableValue::new_int(-1))
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        // Currently unsupported !
        Ok(())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *HAS_ACCESS {
            let res = self.required_security.user_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        if *name == *GET_FILE_AREA {
            let area = arguments[0].as_int();
            if let Some(res) = self.directories.get(area as usize) {
                vm.user_data.push(Box::new((*res).clone()));
                return Ok(VariableValue {
                    data: VariableData::from_int(0),
                    generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                    vtype: VariableType::UserData(FILE_DIRECTORY_ID as u8),
                });
            }
            log::error!("PPL: File area not found ({})", area);

            vm.user_data.push(Box::new(FileDirectory::default()));
            return Ok(VariableValue {
                data: VariableData::from_int(0),
                generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                vtype: VariableType::UserData(FILE_DIRECTORY_ID as u8),
            });
        }
        if *name == *GET_MSG_AREA {
            let area = arguments[0].as_int();
            if let Some(res) = self.areas.get(area as usize) {
                vm.user_data.push(Box::new((*res).clone()));
                return Ok(VariableValue {
                    data: VariableData::from_int(0),
                    generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                    vtype: VariableType::UserData(MESSAGE_AREA_ID as u8),
                });
            }
            log::error!("PPL: Message area not found ({})", area);

            vm.user_data.push(Box::new(MessageArea::default()));
            return Ok(VariableValue {
                data: VariableData::from_int(0),
                generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                vtype: VariableType::UserData(MESSAGE_AREA_ID as u8),
            });
        }

        if *name == *GET_DOOR {
            let door = arguments[0].as_int();
            if let Some(res) = self.doors.get(door as usize) {
                vm.user_data.push(Box::new((*res).clone()));
                return Ok(VariableValue {
                    data: VariableData::from_int(0),
                    generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                    vtype: VariableType::UserData(DOOR_ID as u8),
                });
            }
            log::error!("PPL: Door not found ({})", door);

            vm.user_data.push(Box::new(Door::default()));
            return Ok(VariableValue {
                data: VariableData::from_int(0),
                generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                vtype: VariableType::UserData(DOOR_ID as u8),
            });
        }
        log::error!("Invalid function call on Conference ({})", name);
        Err("Function not found".into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on Conference ({})", name);
        Err("Function not found".into())
    }
}
