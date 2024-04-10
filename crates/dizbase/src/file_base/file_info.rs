use bstr::BString;
use chrono::Utc;

use super::{
    file_header::FileHeader,
    metadata::{MetadaType, MetadataHeader},
};

pub struct FileInfo {
    header: FileHeader,
    metadata: Vec<MetadataHeader>,
}

impl FileInfo {
    /// Creates a new file info
    pub fn new(name: String) -> Self {
        let file_date = Utc::now().timestamp() as u64;
        let header = FileHeader {
            name,
            file_date,
            size: 0,
            hash: 0,
            dl_counter: 0,
            metadata_offset: 0,
            long_description_offset: 0,
            attribute: 0,
        };
        Self { header, metadata: Vec::new() }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.header.size = size;
        self
    }

    pub fn with_hash(mut self, hash: u64) -> Self {
        self.header.hash = hash;
        self
    }

    pub fn with_date(mut self, file_date: u64) -> Self {
        self.header.file_date = file_date;
        self
    }

    pub fn with_dl_counter(mut self, dl_counter: u64) -> Self {
        self.header.dl_counter = dl_counter;
        self
    }

    pub fn with_attribute(mut self, attribute: u8) -> Self {
        self.header.attribute = attribute;
        self
    }

    pub fn create_header(&self) -> FileHeader {
        self.header.clone()
    }

    // Metadata

    pub fn with_uploader(mut self, uploader: String) -> Self {
        self.metadata.push(MetadataHeader::new(MetadaType::UploaderName, uploader.into_bytes()));
        self
    }

    pub fn with_password(mut self, password: String) -> Self {
        self.metadata.push(MetadataHeader::new(MetadaType::Password, password.into_bytes()));
        self
    }

    pub fn with_tags(mut self, tags: String) -> Self {
        self.metadata.push(MetadataHeader::new(MetadaType::Tags, tags.into_bytes()));
        self
    }

    pub fn with_sauce(mut self, data: Vec<u8>) -> Self {
        self.metadata.push(MetadataHeader::new(MetadaType::Sauce, data));
        self
    }

    pub fn with_file_id(mut self, file_id: String) -> Self {
        self.metadata.push(MetadataHeader::new(MetadaType::FileID, file_id.as_bytes().to_vec()));
        self
    }

    pub(crate) fn create_metadata(&self) -> Vec<u8> {
        let mut len = 0;
        for data in &self.metadata {
            len += 8 + data.data.len();
        }
        let mut metadata = Vec::with_capacity(len);
        for data in &self.metadata {
            metadata.push(data.metadata_type.to_data());
            metadata.extend((data.data.len() as u32).to_le_bytes());
            metadata.extend_from_slice(&data.data);
        }
        metadata
    }
}
