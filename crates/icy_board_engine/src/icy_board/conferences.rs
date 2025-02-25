use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{GenericVariableData, VariableData, VariableType, VariableValue},
    icy_board::{doors::Door, file_directory::FileDirectory},
    parser::{DOOR_ID, FILE_DIRECTORY_ID, MESSAGE_AREA_ID},
};

use super::{
    IcyBoardSerializer,
    bulletins::BullettinList,
    commands::Command,
    doors::DoorList,
    file_directory::DirectoryList,
    is_false, is_null_8, is_null_16, is_null_f64, is_null_i32,
    message_area::{AreaList, MessageArea},
    pcbconferences::{PcbAdditionalConferenceHeader, PcbConferenceHeader},
    security_expr::SecurityExpression,
    surveys::SurveyList,
    user_base::Password,
};

#[derive(Default, Clone, Serialize, Deserialize)]
pub enum ConferenceType {
    #[default]
    Normal,
    InternetEmail,
    InternetUsenet,
    UsnetModeratedNewsgroup,
    UsnetPublicNewsgroup,
    FidoConference,
}

impl ConferenceType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Normal,
            1 => Self::InternetEmail,
            2 => Self::InternetUsenet,
            3 => Self::UsnetModeratedNewsgroup,
            4 => Self::UsnetPublicNewsgroup,
            5 => Self::FidoConference,
            _ => Self::Normal,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::InternetEmail => 1,
            Self::InternetUsenet => 2,
            Self::UsnetModeratedNewsgroup => 3,
            Self::UsnetPublicNewsgroup => 4,
            Self::FidoConference => 5,
        }
    }
}

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
    #[serde(skip_serializing_if = "is_false")]
    pub echo_mail_in_conference: bool,

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
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub sec_request_rr: SecurityExpression,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub sec_carbon_copy: SecurityExpression,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub carbon_list_limit: u8,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub auto_rejoin: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub allow_view_conf_members: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub private_uploads: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub private_msgs: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub disallow_private_msgs: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub allow_aliases: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub show_intro_in_scan: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_i32")]
    pub add_conference_security: i32,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_16")]
    pub add_conference_time: u16,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub use_main_commands: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub record_origin: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub prompt_for_routing: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub long_to_names: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub force_echomail: bool,

    #[serde(default)]
    pub conference_type: ConferenceType,

    pub users_menu: PathBuf,
    pub sysop_menu: PathBuf,
    pub news_file: PathBuf,
    pub attachment_location: PathBuf,

    /// Sort type for public upload DIR file
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub pub_upload_sort: u8,
    pub pub_upload_location: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub private_upload_sort: u8,
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
    pub areas: Option<AreaList>,

    #[serde(skip)]
    pub directories: Option<DirectoryList>,

    #[serde(skip)]
    pub doors: Option<DoorList>,

    #[serde(skip)]
    pub bulletins: Option<BullettinList>,

    #[serde(skip)]
    pub surveys: Option<SurveyList>,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_f64")]
    pub charge_time: f64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_f64")]
    pub charge_msg_read: f64,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_f64")]
    pub charge_msg_write: f64,
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
            let general_area: MessageArea = MessageArea {
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
                allow_view_conf_members: c.view_members,
                private_uploads: c.private_uploads,
                private_msgs: c.private_msgs,
                allow_aliases: d.allow_aliases,
                echo_mail_in_conference: c.echo_mail,
                add_conference_security: c.add_conference_security,
                add_conference_time: c.add_conference_time as u16,
                users_menu: PathBuf::from(&c.users_menu),
                sysop_menu: PathBuf::from(&c.sysop_menu),
                news_file: PathBuf::from(&c.news_file),
                attachment_location: PathBuf::from(&d.attach_loc),
                pub_upload_sort: c.pub_upload_sort,
                pub_upload_location: PathBuf::from(&c.pub_upload_location),
                private_upload_sort: c.private_upload_sort,
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
                areas: None,
                directories: None,
                doors: None,
                bulletins: None,
                surveys: None,
                show_intro_in_scan: d.show_intro_on_ra,
                sec_request_rr: SecurityExpression::from_req_security(d.ret_receipt_level),
                sec_carbon_copy: SecurityExpression::from_req_security(d.carbon_level),
                carbon_list_limit: d.carbon_limit,
                charge_time: d.charge_time as f64,
                charge_msg_read: d.charge_msg_read as f64,
                charge_msg_write: d.charge_msg_write as f64,
                disallow_private_msgs: d.no_private_msgs,
                record_origin: d.record_origin,
                prompt_for_routing: d.prompt_for_routing,
                long_to_names: d.long_to_names,
                force_echomail: d.force_echo,
                conference_type: ConferenceType::from_u8(d.conf_type),
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
            if let Some(res) = &self.directories {
                return Ok(VariableValue::new_int(res.len() as i32));
            }
            return Ok(VariableValue::new_int(0));
        }
        if *name == *MESSAGE_AREAS {
            if let Some(res) = &self.areas {
                return Ok(VariableValue::new_int(res.len() as i32));
            }
            return Ok(VariableValue::new_int(0));
        }
        if *name == *DOORS {
            if let Some(res) = &self.doors {
                return Ok(VariableValue::new_bool(!res.is_empty()));
            }
            return Ok(VariableValue::new_int(0));
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
            if let Some(dir) = &self.directories {
                let area = arguments[0].as_int();
                if let Some(res) = dir.get(area as usize) {
                    vm.user_data.push(Box::new((*res).clone()));
                    return Ok(VariableValue {
                        data: VariableData::from_int(0),
                        generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                        vtype: VariableType::UserData(FILE_DIRECTORY_ID as u8),
                    });
                }
                log::error!("PPL: File area not found ({})", area);
            }

            vm.user_data.push(Box::new(FileDirectory::default()));
            return Ok(VariableValue {
                data: VariableData::from_int(0),
                generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                vtype: VariableType::UserData(FILE_DIRECTORY_ID as u8),
            });
        }
        if *name == *GET_MSG_AREA {
            let area = arguments[0].as_int();
            if let Some(areas) = &self.areas {
                if let Some(res) = areas.get(area as usize) {
                    vm.user_data.push(Box::new((*res).clone()));
                    return Ok(VariableValue {
                        data: VariableData::from_int(0),
                        generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                        vtype: VariableType::UserData(MESSAGE_AREA_ID as u8),
                    });
                }
                log::error!("PPL: Message area not found ({})", area);
            }

            vm.user_data.push(Box::new(MessageArea::default()));
            return Ok(VariableValue {
                data: VariableData::from_int(0),
                generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                vtype: VariableType::UserData(MESSAGE_AREA_ID as u8),
            });
        }

        if *name == *GET_DOOR {
            let door = arguments[0].as_int();
            if let Some(doors) = &self.doors {
                if let Some(res) = doors.get(door as usize) {
                    vm.user_data.push(Box::new((*res).clone()));
                    return Ok(VariableValue {
                        data: VariableData::from_int(0),
                        generic_data: GenericVariableData::UserData(vm.user_data.len() - 1),
                        vtype: VariableType::UserData(DOOR_ID as u8),
                    });
                }
                log::error!("PPL: Door not found ({})", door);
            }

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
