use std::{
    ops::{Deref, DerefMut},
    path::Path,
    str::FromStr,
};

use crate::{
    Res,
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{VariableType, VariableValue},
    parser::load_with_encoding,
};

use super::{IcyBoardSerializer, security_expr::SecurityExpression};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

mod callinfo_bbs;
mod chain_txt;
mod curruser_bbs;
mod door32_sys;
mod door_sys;
mod doorfile_sr;
mod dorinfo_x;
pub mod dos;
mod exitinfo_bbs;
mod jumper_dat;
pub mod pcboard;
mod sfdoors_dat;
mod tribbs_sys;

const DOOR_COM_PORT: u8 = 1;
const DOOR_BPS_RATE: u32 = 57600;

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BBSLink {
    pub system_code: String,
    pub auth_code: String,
    pub sheme_code: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum DoorServerAccount {
    BBSLink(BBSLink),
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum DoorType {
    #[default]
    Local,
    BBSlink,
    Dos,
}

impl std::fmt::Display for DoorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoorType::Local => write!(f, "Local"),
            DoorType::BBSlink => write!(f, "BBSlink"),
            DoorType::Dos => write!(f, "Dos"),
        }
    }
}

impl DoorType {
    pub fn iter() -> impl Iterator<Item = DoorType> {
        vec![DoorType::Local, DoorType::BBSlink, DoorType::Dos].into_iter()
    }
}

impl FromStr for DoorType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Local" => Ok(DoorType::Local),
            "BBSlink" => Ok(DoorType::BBSlink),
            "Dos" => Ok(DoorType::Dos),
            _ => Err(format!("Invalid DoorType: {s}")),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq, Debug)]
pub enum DropFile {
    #[default]
    None,
    PCBoard,

    /// Common Door.Sys format
    DoorSys,
    /// Mystic BBS
    Door32Sys,
    /// RBBS/QuickBBS
    DorInfo,
    /// WWIV
    CallInfo,
    /// Solar Realms doorfile.sr format
    DoorFileSR,
    /// `RyBBS`
    CurruserBBS,
    /// Chain.TXT format from the WWIV software.
    ChainTXT,
    /// `TriBBS` doorfile format
    TriBBSSYS,
    /// `SpitFire` BBS
    SFDoorsDAT,
    /// `QuickBBS` + `RemoteAccess` 2.62 extensions
    ExitInfoBBS,
    /// 2AM BBS
    JumperDat,
    // USERINFO.DAT WildCat!
    // INFO.BBS  Phoenix BBS
}

impl DropFile {
    pub fn iter() -> impl Iterator<Item = DropFile> {
        [
            DropFile::None,
            DropFile::PCBoard,
            DropFile::DoorSys,
            DropFile::Door32Sys,
            DropFile::DorInfo,
            DropFile::CallInfo,
            DropFile::DoorFileSR,
            DropFile::CurruserBBS,
            DropFile::ChainTXT,
            DropFile::TriBBSSYS,
            DropFile::SFDoorsDAT,
            DropFile::ExitInfoBBS,
            DropFile::JumperDat,
        ]
        .into_iter()
    }
}

impl std::fmt::Display for DropFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DropFile::None => "None",
            DropFile::PCBoard => "PCBOARD.SYS + USER.SYS",
            DropFile::DoorSys => "DOOR.SYS",
            DropFile::Door32Sys => "DOOR32.SYS",
            DropFile::DorInfo => "DORINFOx.DEF",
            DropFile::CallInfo => "CALLINFO.BBS",
            DropFile::DoorFileSR => "DOORFILE.SR",
            DropFile::CurruserBBS => "CURRUSER.BBS",
            DropFile::ChainTXT => "CHAIN.TXT",
            DropFile::TriBBSSYS => "TRIBBS.SYS",
            DropFile::SFDoorsDAT => "SFDOORS.DAT",
            DropFile::ExitInfoBBS => "EXITINFO.BBS",
            DropFile::JumperDat => "JUMPER.DAT",
        })
    }
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Door {
    pub name: String,
    pub description: String,
    pub password: String,

    /// Set when the door is handed to a PPE, so the object can report where it sits.
    #[serde(skip)]
    pub number: usize,

    #[serde(skip)]
    pub valid: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "SecurityExpression::is_empty")]
    #[serde_as(as = "DisplayFromStr")]
    pub securiy_level: SecurityExpression,

    pub door_type: DoorType,
    pub path: String,
    #[serde(default)]
    pub use_shell_execute: bool,

    #[serde(default)]
    pub drop_file: DropFile,

    #[serde(default)]
    pub dos_command: String,

    #[serde(default = "default_dos_memory_mb")]
    pub dos_memory_mb: u32,

    /// Maximum wall-clock runtime for this DOS door; zero leaves it unlimited.
    #[serde(default)]
    pub dos_max_runtime_seconds: u32,
}

fn default_dos_memory_mb() -> u32 {
    64
}
impl Door {
    pub async fn create_drop_file(&self, state: &super::state::IcyBoardState, path: &std::path::Path, door_number: usize) -> Res<()> {
        match self.drop_file {
            DropFile::None => Ok(()),
            DropFile::PCBoard => pcboard::create_pcboard(state, path).await,
            DropFile::DoorSys => door_sys::create_door_sys(state, path).await,
            DropFile::Door32Sys => door32_sys::create_door32_sys(state, path),
            DropFile::DorInfo => dorinfo_x::create_dorinfo(state, path).await,
            DropFile::CallInfo => callinfo_bbs::create_callinfo_bbs(state, path, door_number).await,
            DropFile::DoorFileSR => doorfile_sr::create_doorfile_sr(state, path),
            DropFile::CurruserBBS => curruser_bbs::create_curruser_bbs(state, path),
            DropFile::ChainTXT => chain_txt::create_chain_txt(state, path).await,
            DropFile::TriBBSSYS => tribbs_sys::create_tribbs_sys(state, path).await,
            DropFile::SFDoorsDAT => sfdoors_dat::create_sfdoors_dat(state, path).await,
            DropFile::ExitInfoBBS => exitinfo_bbs::create_exitinfo_bbs(state, path).await,
            DropFile::JumperDat => jumper_dat::create_jumper_dat(state, path).await,
        }
    }
}

impl UserData for Door {
    const TYPE_NAME: &'static str = "Door";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(NAME.clone(), VariableType::String, false);
        registry.add_property(NUMBER.clone(), VariableType::Integer, false);
        registry.add_property(VALID.clone(), VariableType::Boolean, false);
        registry.add_property(DESCRIPTION.clone(), VariableType::String, false);
        registry.add_property(PATH.clone(), VariableType::String, false);
        registry.add_property(PASSWORD.clone(), VariableType::Password, false);
        registry.add_function(HAS_ACCESS.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for Door {
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
        if *name == *DESCRIPTION {
            return Ok(VariableValue::new_string(self.description.clone()));
        }
        if *name == *PATH {
            return Ok(VariableValue::new_string(self.path.clone()));
        }
        if *name == *PASSWORD {
            return Ok(VariableValue::new_password(crate::icy_board::user_base::Password::new_protected(
                &self.password,
            )));
        }
        log::error!("Invalid user data call on Door ({name})");
        Ok(VariableValue::new_int(-1))
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("DOOR property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *HAS_ACCESS {
            let res = self.securiy_level.session_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        log::error!("Invalid function call on Door ({name})");
        Err("Function not found".into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on Door ({name})");
        Err("Function not found".into())
    }
}

pub static NAME: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Name".to_string()));
pub static NUMBER: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Number".to_string()));
pub static VALID: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Valid".to_string()));
pub static DESCRIPTION: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Description".to_string()));
pub static PATH: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Path".to_string()));
pub static PASSWORD: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("Password".to_string()));
pub static HAS_ACCESS: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("HasAccess".to_string()));

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct DoorList {
    #[serde(rename = "account")]
    pub accounts: Vec<DoorServerAccount>,

    #[serde(rename = "door")]
    pub doors: Vec<Door>,
}

impl DoorList {
    pub fn import_pcboard<P: AsRef<Path>>(path: &P) -> Res<Self> {
        let text = load_with_encoding(path, crate::parser::Encoding::CP437)?;

        let mut result = Self::default();
        for (nr, line) in text.lines().enumerate() {
            let split: Vec<&str> = line.split(',').collect();
            if split.len() < 8 {
                log::error!("Invalid DOOR.LST {} line: {}:{}", path.as_ref().display(), nr + 1, line);
                continue;
            }
            let file = split[0];
            let password = split[1];
            let security = split[2];
            let user_sys = split[3] != "0";
            let door_sys = split[4] != "0";
            let path = split[5];
            // let _login= split[6] != "0";
            let use_shell = split[7] != "N";
            // let per_use=  split[8].parse::<f32>().unwrap_or_default();
            // let charges_minute=  split[9].parse::<f32>().unwrap_or_default();
            // let os_2= split[10] != "0";

            let door = Door {
                number: 0,
                valid: false,
                name: file.to_string(),
                description: file.to_string(),
                password: password.to_string(),
                securiy_level: SecurityExpression::from_str(security)?,
                door_type: DoorType::Local,
                path: path.to_string(),
                use_shell_execute: use_shell,
                drop_file: if door_sys {
                    DropFile::DoorSys
                } else if user_sys {
                    DropFile::CurruserBBS
                } else {
                    DropFile::None
                },
                dos_command: String::new(),
                dos_memory_mb: default_dos_memory_mb(),
                dos_max_runtime_seconds: 0,
            };
            result.doors.push(door);
        }

        Ok(result)
    }
}

impl Deref for DoorList {
    type Target = Vec<Door>;
    fn deref(&self) -> &Self::Target {
        &self.doors
    }
}

impl DerefMut for DoorList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.doors
    }
}

impl IcyBoardSerializer for DoorList {
    const FILE_TYPE: &'static str = "doors";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_file_choices_cover_every_serialized_variant() {
        let choices = DropFile::iter().collect::<Vec<_>>();
        assert_eq!(choices.len(), 13);
        for choice in &choices {
            assert_eq!(choices.iter().filter(|candidate| *candidate == choice).count(), 1);
            assert!(!choice.to_string().is_empty());
        }
    }

    #[test]
    fn door_security_and_drop_file_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("doors.toml");
        let mut list = DoorList::default();
        list.doors.push(Door {
            name: "BRE".into(),
            securiy_level: SecurityExpression::from_str("U_SEC() >= 50").unwrap(),
            drop_file: DropFile::DoorSys,
            ..Door::default()
        });

        list.save(&path).unwrap();
        let loaded = DoorList::load(&path).unwrap();
        assert!(loaded.doors[0].securiy_level == list.doors[0].securiy_level);
        assert_eq!(loaded.doors[0].drop_file, DropFile::DoorSys);
    }
}
