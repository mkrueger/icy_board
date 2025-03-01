#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetadataType {
    /// Unknown meta data type
    Unknown(u8),
    /// The full file name of the file
    FilePath,
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
impl MetadataType {
    pub fn from_data(data: u8) -> Self {
        match data {
            0 => MetadataType::FilePath,
            1 => MetadataType::UploaderName,
            2 => MetadataType::Password,
            3 => MetadataType::Tags,
            4 => MetadataType::FileID,
            5 => MetadataType::Sauce,
            _ => MetadataType::Unknown(data),
        }
    }

    pub fn to_data(&self) -> u8 {
        match self {
            MetadataType::Unknown(data) => *data,
            MetadataType::FilePath => 0,
            MetadataType::UploaderName => 1,
            MetadataType::Password => 2,
            MetadataType::Tags => 3,
            MetadataType::FileID => 4,
            MetadataType::Sauce => 5,
        }
    }
}

#[derive(Clone)]
pub struct MetadataHeader {
    pub metadata_type: MetadataType,
    pub data: Vec<u8>,
}
impl MetadataHeader {
    pub fn new(metadata_type: MetadataType, data: Vec<u8>) -> Self {
        Self { metadata_type, data }
    }

    pub fn get_type(&self) -> MetadataType {
        self.metadata_type
    }
}
