use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{VariableType, VariableValue},
    icy_board::state::ppl_collection::{PplAreas, PplDirectories, PplDoors},
    parser::{AREAS_ID, DIRECTORIES_ID, DOORS_ID},
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

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
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

    /// Set when the conference is handed to a PPE, so the object can report its number.
    #[serde(skip)]
    pub number: usize,

    #[serde(skip)]
    pub valid: bool,

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
    pub pub_upload_metadata: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_8")]
    pub private_upload_sort: u8,
    pub private_upload_location: PathBuf,
    #[serde(default)]
    pub private_upload_metadata: PathBuf,

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

    // Shared rather than owned: a conference is handed to a PPE by value, and copying
    // every area and directory along with it made that cost the size of the lists.
    #[serde(skip)]
    pub areas: Option<Arc<AreaList>>,

    #[serde(skip)]
    pub directories: Option<Arc<DirectoryList>>,

    #[serde(skip)]
    pub doors: Option<Arc<DoorList>>,

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
                number: 0,
                valid: false,
                name: "General".to_string(),
                path: PathBuf::from(&c.message_file),
                qwk_name: "General".to_string(),
                qwk_conference_number: 0,
                ftn_area_tag: String::new(),
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
            areas.save(&destination.join("area.toml")).unwrap();

            let new = Conference {
                number: 0,
                valid: false,
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
                add_conference_time: c.add_conference_time,
                users_menu: PathBuf::from(&c.users_menu),
                sysop_menu: PathBuf::from(&c.sysop_menu),
                news_file: PathBuf::from(&c.news_file),
                attachment_location: PathBuf::from(&d.attach_loc),
                pub_upload_sort: c.pub_upload_sort,
                pub_upload_location: PathBuf::from(&c.pub_upload_location),
                pub_upload_metadata: PathBuf::from(&c.pub_upload_location).join("dir"),
                private_upload_sort: c.private_upload_sort,
                private_upload_location: PathBuf::from(&c.private_upload_location),
                private_upload_metadata: PathBuf::from(&c.private_upload_location).join("dir"),
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

impl UserData for Conference {
    const TYPE_NAME: &'static str = "Conference";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(NAME.clone(), VariableType::String, false);
        registry.add_property(NUMBER.clone(), VariableType::Integer, false);
        registry.add_property(VALID.clone(), VariableType::Boolean, false);
        registry.add_property(ISPUBLIC.clone(), VariableType::Boolean, false);
        registry.add_property(IS_READ_ONLY.clone(), VariableType::Boolean, false);
        registry.add_property(ALLOW_ALIASES.clone(), VariableType::Boolean, false);
        registry.add_property(ECHO_MAIL.clone(), VariableType::Boolean, false);
        registry.add_property(AUTO_REJOIN.clone(), VariableType::Boolean, false);
        registry.add_property(PRIVATE_UPLOADS.clone(), VariableType::Boolean, false);
        registry.add_property(PASSWORD.clone(), VariableType::Password, false);
        registry.add_property(FILE_AREAS.clone(), VariableType::UserData(DIRECTORIES_ID as u8), false);
        registry.add_property(MESSAGE_AREAS.clone(), VariableType::UserData(AREAS_ID as u8), false);
        registry.add_property(DOORS.clone(), VariableType::UserData(DOORS_ID as u8), false);

        registry.add_function(HAS_ACCESS.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(CAN_POST.clone(), Vec::new(), VariableType::Boolean);
        registry.add_function(CAN_ATTACH.clone(), Vec::new(), VariableType::Boolean);
    }
}

pub static NAME: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Name".to_string()));
pub static NUMBER: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Number".to_string()));
pub static VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Valid".to_string()));
pub static ISPUBLIC: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("IsPublic".to_string()));
pub static IS_READ_ONLY: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("IsReadOnly".to_string()));
pub static ALLOW_ALIASES: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("AllowAliases".to_string()));
pub static ECHO_MAIL: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("EchoMail".to_string()));
pub static AUTO_REJOIN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("AutoRejoin".to_string()));
pub static PRIVATE_UPLOADS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("PrivateUploads".to_string()));
pub static PASSWORD: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Password".to_string()));
pub static FILE_AREAS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Directories".to_string()));
pub static DOORS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Doors".to_string()));
pub static MESSAGE_AREAS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Areas".to_string()));
pub static HAS_ACCESS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("HasAccess".to_string()));
pub static CAN_POST: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("CanPost".to_string()));
pub static CAN_ATTACH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("CanAttach".to_string()));

#[async_trait(?Send)]
impl UserDataValue for Conference {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *NAME {
            return Ok(VariableValue::new_string(self.name.clone()));
        }
        if *name == *NUMBER {
            return Ok(VariableValue::new_int(self.number as i32));
        }
        if *name == *VALID {
            return Ok(VariableValue::new_bool(self.valid));
        }
        if *name == *ISPUBLIC {
            return Ok(VariableValue::new_bool(self.is_public));
        }
        if *name == *IS_READ_ONLY {
            return Ok(VariableValue::new_bool(self.is_read_only));
        }
        if *name == *ALLOW_ALIASES {
            return Ok(VariableValue::new_bool(self.allow_aliases));
        }
        if *name == *ECHO_MAIL {
            return Ok(VariableValue::new_bool(self.echo_mail_in_conference));
        }
        if *name == *AUTO_REJOIN {
            return Ok(VariableValue::new_bool(self.auto_rejoin));
        }
        if *name == *PRIVATE_UPLOADS {
            return Ok(VariableValue::new_bool(self.private_uploads));
        }
        if *name == *PASSWORD {
            return Ok(VariableValue::new_password(self.password.protected()));
        }
        if *name == *FILE_AREAS {
            return Ok(PplDirectories::new(self.directories.clone().unwrap_or_default()).value());
        }
        if *name == *MESSAGE_AREAS {
            return Ok(PplAreas::new(self.areas.clone().unwrap_or_default()).value());
        }
        if *name == *DOORS {
            return Ok(PplDoors::new(self.doors.clone().unwrap_or_default()).value());
        }

        log::error!("Invalid user data call on Conference ({name})");
        Ok(VariableValue::new_int(-1))
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("CONFERENCE property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *HAS_ACCESS {
            let res = self.required_security.session_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        if *name == *CAN_POST {
            let res = self.sec_write_message.session_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        if *name == *CAN_ATTACH {
            let res = self.sec_attachments.session_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        log::error!("Invalid function call on Conference ({name})");
        Err("Function not found".into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on Conference ({name})");
        Err("Function not found".into())
    }
}
