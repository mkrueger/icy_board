#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetadaType {
    /// Unknown meta data type
    Unknown(u8),
    /// The uploader of the file.
    UploaderName,
    /// The password of the file  (Murmur64a hash)
    Password,
    /// A csv list of tags
    Tags,
    /// Short description (FILE_ID.DIZ/DESC.SDI)
    FileID,
    /// Sauce (128 bytes)
    Sauce,
}
impl MetadaType {
    pub fn from_data(data: u8) -> Self {
        match data {
            1 => MetadaType::UploaderName,
            2 => MetadaType::Password,
            3 => MetadaType::Tags,
            4 => MetadaType::FileID,
            5 => MetadaType::Sauce,
            _ => MetadaType::Unknown(data),
        }
    }

    pub fn to_data(&self) -> u8 {
        match self {
            MetadaType::Unknown(data) => *data,
            MetadaType::UploaderName => 1,
            MetadaType::Password => 2,
            MetadaType::Tags => 3,
            MetadaType::FileID => 4,
            MetadaType::Sauce => 5,
        }
    }
}

pub struct MetadataHeader {
    pub metadata_type: MetadaType,
    pub data: Vec<u8>,
}
impl MetadataHeader {
    pub fn new(metadata_type: MetadaType, data: Vec<u8>) -> Self {
        Self { metadata_type, data }
    }

    pub fn get_type(&self) -> MetadaType {
        self.metadata_type
    }
}
