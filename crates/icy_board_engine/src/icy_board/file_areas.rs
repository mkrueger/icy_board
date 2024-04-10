use std::{
    fs,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use icy_ppe::{tables::export_cp437_string, Res};
use serde::{Deserialize, Serialize};

use super::{
    is_false, security::RequiredSecurity, user_base::Password, IcyBoardError, IcyBoardSerializer,
    PCBoardRecordImporter,
};

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
pub struct FileArea {
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
pub struct FileAreaList {
    #[serde(rename = "area")]
    areas: Vec<FileArea>,
}

impl FileAreaList {
    const PATH_SIZE: usize = 0x1E;
    const NAME_SIZE: usize = 0x23;

    pub(crate) fn export_pcboard(&self, dir_file: &PathBuf) -> Res<()> {
        let mut buf = Vec::with_capacity(Self::RECORD_SIZE * self.areas.len());

        for area in &self.areas {
            buf.extend(export_cp437_string(
                &area.file_base.to_string_lossy(),
                Self::PATH_SIZE,
                b' ',
            ));
            buf.extend(export_cp437_string(
                &area.path.to_string_lossy(),
                Self::PATH_SIZE,
                b' ',
            ));
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

impl Deref for FileAreaList {
    type Target = Vec<FileArea>;
    fn deref(&self) -> &Self::Target {
        &self.areas
    }
}

impl DerefMut for FileAreaList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.areas
    }
}

impl PCBoardRecordImporter<FileArea> for FileAreaList {
    const RECORD_SIZE: usize = Self::PATH_SIZE * 2 + Self::NAME_SIZE + 1;

    fn push(&mut self, value: FileArea) {
        self.areas.push(value);
    }

    fn load_pcboard_record(data: &[u8]) -> Res<FileArea> {
        let file_base = PathBuf::from(icy_ppe::tables::import_cp437_string(
            &data[..Self::PATH_SIZE],
            true,
        ));
        let data = &data[Self::PATH_SIZE..];
        let path = PathBuf::from(icy_ppe::tables::import_cp437_string(
            &data[..Self::PATH_SIZE],
            true,
        ));
        let data = &data[Self::PATH_SIZE..];
        let name = icy_ppe::tables::import_cp437_string(&data[..Self::NAME_SIZE], true);
        let data = &data[Self::NAME_SIZE..];

        let (sort_order, sort_direction) = match data[0] {
            0 => (SortOrder::NoSort, SortDirection::Ascending),
            1 => (SortOrder::FileName, SortDirection::Ascending),
            2 => (SortOrder::FileDate, SortDirection::Ascending),
            3 => (SortOrder::FileName, SortDirection::Descending),
            4 => (SortOrder::FileDate, SortDirection::Descending),
            _ => return Err(IcyBoardError::InvalidDirListSortOrder(data[0]).into()),
        };

        Ok(FileArea {
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

impl IcyBoardSerializer for FileAreaList {
    const FILE_TYPE: &'static str = "surveys";
}
