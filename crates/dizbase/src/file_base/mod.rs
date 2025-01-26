use std::{
    fs::{self, OpenOptions},
    io::{BufReader, Read, Seek},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use thiserror::Error;

use crate::{extensions, file_base_scanner::scan_file};

use self::{
    file_header::FileHeader,
    metadata::{MetadaType, MetadataHeader},
    pattern::Pattern,
};

pub mod base_header_info;
pub mod file_header;
pub mod metadata;
pub mod pattern;

#[derive(Error, Debug)]
pub enum FileBaseError {
    #[error("Invalid header signature (needs to start with 'ICFB')")]
    InvalidHeaderSignature,

    #[error("Invalid search token")]
    InvalidSearchToken,
}

const HDR_SIGNATURE: [u8; 4] = [b'I', b'C', b'F', b'B'];

pub struct FileEntry {
    pub file_name: String,
    pub full_path: PathBuf,
    pub metadata: Option<Vec<MetadataHeader>>,
    pub file_date: Option<DateTime<Local>>,
    pub file_size: Option<u64>,
}
impl FileEntry {
    pub fn new(full_path: PathBuf) -> Self {
        let file_name = full_path.file_name().unwrap().to_str().unwrap().to_string();
        Self {
            file_name,
            full_path,
            metadata: None,
            file_date: None,
            file_size: None,
        }
    }

    pub fn date(&mut self) -> DateTime<Local> {
        if let Some(date) = self.file_date {
            return date;
        }
        self.read_metadata();
        self.file_date.unwrap()
    }

    pub fn size(&mut self) -> u64 {
        if let Some(size) = self.file_size {
            return size;
        }
        self.read_metadata();
        self.file_size.unwrap()
    }

    pub fn name(&self) -> &str {
        &self.file_name
    }

    pub fn get_metadata(&mut self) -> crate::Result<&Vec<MetadataHeader>> {
        if self.metadata.is_none() {
            if let Some(ext) = self.full_path.extension() {
                self.metadata = Some(scan_file(&self.full_path, &ext.to_ascii_uppercase().to_string_lossy())?);
            } else {
                self.metadata = Some(Vec::new());
            }
        }
        Ok(self.metadata.as_ref().unwrap())
    }

    fn read_metadata(&mut self) {
        self.file_date = Some(Local::now());
        self.file_size = Some(0);
        if let Ok(metadata) = fs::metadata(&self.full_path) {
            if let Ok(system_time) = metadata.modified() {
                self.file_date = Some(system_time.into());
            }
            self.file_size = Some(metadata.len());
        }
    }
}

pub struct FileBase {
    base_path: PathBuf,
    pub file_headers: Vec<FileEntry>,
}

impl FileBase {
    pub fn open<P: AsRef<Path>>(file_base: P) -> crate::Result<Self> {
        let dir = file_base.as_ref();
        if dir.is_dir() {
            let mut file_headers = Vec::new();
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    file_headers.push(FileEntry::new(path));
                }
            }
            Ok(Self {
                base_path: file_base.as_ref().into(),
                file_headers,
            })
        } else {
            Err(FileBaseError::InvalidHeaderSignature.into())
        }
    }

    pub fn find_files(&mut self, search: &str) -> crate::Result<Vec<&mut FileEntry>> {
        let Ok(pattern) = Pattern::new(search) else {
            return Err(FileBaseError::InvalidSearchToken.into());
        };

        Ok(self.file_headers.par_iter_mut().filter(|header| pattern.matches(&header.file_name)).collect())
    }

    pub fn find_newer_files(&mut self, timestamp: DateTime<Local>) -> crate::Result<Vec<&mut FileEntry>> {
        let mut res: Vec<&mut FileEntry> = Vec::new();
        for header in self.file_headers.iter_mut() {
            if header.date() > timestamp {
                res.push(header);
            }
        }
        Ok(res)
    }

    pub fn find_files_with_pattern(&self, str: &str) -> crate::Result<Vec<&mut FileEntry>> {
        let lc = str.to_lowercase();
        let _bytes = lc.as_bytes();
        Ok(Vec::new())
        /*
        Ok(self
            .file_headers
            .par_iter()
            .filter(|header| {
                if let Ok(metadata) = self.read_metadata(header) {
                    for m in metadata {
                        if m.metadata_type == MetadaType::FileID {
                            if find_match(m.data, bytes) {
                                return true;
                            }
                        }
                    }
                }
                false
            })
            .collect())*/
    }

    pub fn get_headers(&self) -> &Vec<FileEntry> {
        &self.file_headers
    }

    pub fn read_metadata(&self, header: &FileHeader) -> crate::Result<Vec<MetadataHeader>> {
        let metadata_file_name = self.base_path.with_extension(extensions::FILE_METADATA);
        let file = OpenOptions::new().read(true).open(metadata_file_name)?;

        let mut reader = BufReader::new(file);
        reader.seek(std::io::SeekFrom::Start(header.metadata_offset()))?;

        let mut size = [0; 4];
        reader.read_exact(&mut size)?;
        let size = u32::from_le_bytes(size);
        let mut data = vec![0; size as usize];
        reader.read_exact(&mut data)?;
        let mut result = Vec::new();
        let mut data = &data[..];

        while !data.is_empty() {
            let meta_type = data[0];
            let meta_len = u32::from_le_bytes(data[1..5].try_into().unwrap());
            let end = meta_len as usize + 5;
            let metadata = data[5..end].to_vec();
            data = &data[end..];
            result.push(MetadataHeader::new(MetadaType::from_data(meta_type), metadata));
        }
        Ok(result)
    }
}
