use std::{
    fs,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{PPEExpr, VariableType, VariableValue},
    tables::export_cp437_string,
    Res,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{is_false, security::RequiredSecurity, user_base::Password, IcyBoardError, IcyBoardSerializer, PCBoardRecordImporter};

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub enum SortOrder {
    NoSort,
    #[default]
    FileName,
    FileDate,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

/// A survey is a question and answer pair.
/// PCBoard calles them "Questionnairies" but we call them surveys.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct FileDirectory {
    pub name: String,
    pub file_base: PathBuf,
    pub path: PathBuf,

    pub password: Password,

    #[serde(default)]
    pub sort_order: SortOrder,
    #[serde(default)]
    pub sort_direction: SortDirection,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub has_new_files: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub is_readonly: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub is_free: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub allow_ul_pwd: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub list_security: RequiredSecurity,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub download_security: RequiredSecurity,

    #[serde(default)]
    #[serde(skip_serializing_if = "RequiredSecurity::is_empty")]
    pub upload_security: RequiredSecurity,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct DirectoryList {
    #[serde(rename = "area")]
    areas: Vec<FileDirectory>,
}

impl DirectoryList {
    const PATH_SIZE: usize = 0x1E;
    const NAME_SIZE: usize = 0x23;

    pub(crate) fn export_pcboard(&self, dir_file: &PathBuf) -> Res<()> {
        let mut buf = Vec::with_capacity(Self::RECORD_SIZE * self.areas.len());

        for area in &self.areas {
            buf.extend(export_cp437_string(&area.file_base.to_string_lossy(), Self::PATH_SIZE, b' '));
            buf.extend(export_cp437_string(&area.path.to_string_lossy(), Self::PATH_SIZE, b' '));
            buf.extend(export_cp437_string(&area.name, Self::NAME_SIZE, b' '));
            let sort_order = match area.sort_order {
                SortOrder::NoSort => 0,
                SortOrder::FileName => match area.sort_direction {
                    SortDirection::Ascending => 1,
                    SortDirection::Descending => 3,
                },
                SortOrder::FileDate => match area.sort_direction {
                    SortDirection::Ascending => 2,
                    SortDirection::Descending => 4,
                },
            };
            buf.push(sort_order);
        }
        fs::write(dir_file, &buf)?;
        Ok(())
    }
}

impl Deref for DirectoryList {
    type Target = Vec<FileDirectory>;
    fn deref(&self) -> &Self::Target {
        &self.areas
    }
}

impl DerefMut for DirectoryList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.areas
    }
}

impl PCBoardRecordImporter<FileDirectory> for DirectoryList {
    const RECORD_SIZE: usize = Self::PATH_SIZE * 2 + Self::NAME_SIZE + 1;

    fn push(&mut self, value: FileDirectory) {
        self.areas.push(value);
    }

    fn load_pcboard_record(data: &[u8]) -> Res<FileDirectory> {
        let file_base = PathBuf::from(crate::tables::import_cp437_string(&data[..Self::PATH_SIZE], true));
        let data = &data[Self::PATH_SIZE..];
        let path = PathBuf::from(crate::tables::import_cp437_string(&data[..Self::PATH_SIZE], true));
        let data = &data[Self::PATH_SIZE..];
        let name = crate::tables::import_cp437_string(&data[..Self::NAME_SIZE], true);
        let data = &data[Self::NAME_SIZE..];

        let (sort_order, sort_direction) = match data[0] {
            0 => (SortOrder::NoSort, SortDirection::Ascending),
            1 => (SortOrder::FileName, SortDirection::Ascending),
            2 => (SortOrder::FileDate, SortDirection::Ascending),
            3 => (SortOrder::FileName, SortDirection::Descending),
            4 => (SortOrder::FileDate, SortDirection::Descending),
            _ => return Err(IcyBoardError::InvalidDirListSortOrder(data[0]).into()),
        };

        Ok(FileDirectory {
            name,
            file_base,
            path,
            sort_order,
            sort_direction,
            password: Password::default(),

            has_new_files: false,
            is_readonly: false,
            is_free: false,
            allow_ul_pwd: false,
            list_security: RequiredSecurity::default(),
            download_security: RequiredSecurity::default(),
            upload_security: RequiredSecurity::default(),
        })
    }
}

impl IcyBoardSerializer for DirectoryList {
    const FILE_TYPE: &'static str = "file areas";
}

impl UserData for FileDirectory {
    const TYPE_NAME: &'static str = "Directory";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(NAME.clone(), VariableType::String, false);

        registry.add_function(HAS_ACCESS.clone(), Vec::new(), VariableType::Boolean);
    }
}

lazy_static::lazy_static! {
    pub static ref NAME: unicase::Ascii<String> = unicase::Ascii::new("Name".to_string());
    pub static ref HAS_ACCESS: unicase::Ascii<String> = unicase::Ascii::new("HasAccess".to_string());
}

#[async_trait]
impl UserDataValue for FileDirectory {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *NAME {
            return Ok(VariableValue::new_string(self.name.clone()));
        }
        log::error!("Invalid user data call on FileDirectory ({})", name);
        Ok(VariableValue::new_int(-1))
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        log::error!("Invalid user data set on FileDirectory ({})", name);
        Ok(())
    }

    async fn call_function(&self, vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[PPEExpr]) -> crate::Res<VariableValue> {
        if *name == *HAS_ACCESS {
            let res = self.list_security.user_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        log::error!("Invalid function call on FileDirectory ({})", name);
        Err("Function not found".into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[PPEExpr]) -> crate::Res<()> {
        log::error!("Invalid method call on FileDirectory ({})", name);
        Err("Function not found".into())
    }
}
