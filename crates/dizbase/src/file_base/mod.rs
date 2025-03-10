use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, Write},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use chrono::Utc;
use file_header::FileAttributes;
use thiserror::Error;
use twox_hash::XxHash3_64;

use crate::{extensions, file_base_scanner::scan_file};

use self::{
    file_header::FileHeader,
    metadata::{MetadataHeader, MetadataType},
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

    #[error("Directory {0} is not a directory")]
    DirIsNoDir(PathBuf),

    #[error("Can't open metadata file")]
    CantOpenMetadata,

    #[error("File {0} not found")]
    FileNotFound(String),

    #[error("No extension found")]
    NoExtension,
}

const HDR_SIGNATURE: [u8; 4] = [b'I', b'C', b'F', b'B'];

pub struct FileBase {
    meta_data_path: PathBuf,
    dir: PathBuf,
    name_map: HashMap<String, usize>,
    file_headers: Vec<FileHeader>,
}

impl Deref for FileBase {
    type Target = Vec<FileHeader>;
    fn deref(&self) -> &Self::Target {
        &self.file_headers
    }
}

impl DerefMut for FileBase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file_headers
    }
}

impl FileBase {
    pub fn open<P: AsRef<Path>>(dir: &Path, meta_data_path: P) -> crate::Result<Self> {
        let mut file_headers: Vec<FileHeader> = Vec::new();
        let mut name_map = HashMap::new();

        if let Ok(file) = fs::OpenOptions::new().read(true).open(meta_data_path.as_ref()) {
            let mut reader = BufReader::new(file);
            while let Ok(entry) = FileHeader::read(&mut reader) {
                name_map.insert(entry.name.clone(), file_headers.len());
                file_headers.push(entry);
            }
        }

        let mut res = Self {
            dir: dir.to_path_buf(),
            meta_data_path: meta_data_path.as_ref().into(),
            name_map,
            file_headers,
        };
        if let Err(err) = res.scan_path() {
            log::error!("Filebase error scanning path: {}", err);
        }
        Ok(res)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn scan_path(&mut self) -> crate::Result<()> {
        if self.dir.is_dir() {
            let old_lent = self.file_headers.len();
            for entry in fs::read_dir(&self.dir)? {
                let path = entry?.path();
                let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
                if self.name_map.contains_key(&file_name) {
                    continue;
                }
                if path.is_file() {
                    self.name_map.insert(file_name.clone(), self.file_headers.len());
                    let mut date = Utc::now();
                    let mut size = 0;
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(system_time) = metadata.modified() {
                            date = system_time.into();
                        }
                        size = metadata.len();
                    }

                    let header = FileHeader {
                        name: file_name,
                        date,
                        size,
                        dl_counter: 0,
                        metadata_offset: u64::MAX,
                        attribute: FileAttributes::NONE,
                    };
                    self.file_headers.push(header);
                }
            }
            if old_lent != self.file_headers.len() {
                self.save()?;
            }
            Ok(())
        } else {
            Err(FileBaseError::DirIsNoDir(self.dir.clone()).into())
        }
    }

    pub fn read_metadata(&mut self, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        let header = if let Some(index) = self.name_map.get(&file_name).clone() {
            if self.file_headers[*index].metadata_offset() == u64::MAX {
                let res = self.create_header(*index, &path);
                return res;
            }
            &self.file_headers[*index]
        } else {
            return Err(FileBaseError::FileNotFound(file_name).into());
        };
        self.get_metadata(header)
    }

    pub fn add_file(&mut self, path: &Path, metadata: Vec<MetadataHeader>) -> crate::Result<()> {
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        if self.name_map.contains_key(&file_name) {
            return Err(FileBaseError::FileNotFound(file_name).into());
        }
        let mut date = Utc::now();
        let mut size = 0;
        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(system_time) = metadata.modified() {
                date = system_time.into();
            }
            size = metadata.len();
        }

        let header = FileHeader {
            name: file_name.clone(),
            date,
            size,
            dl_counter: 0,
            metadata_offset: u64::MAX,
            attribute: FileAttributes::NONE,
        };
        self.name_map.insert(file_name.clone(), self.file_headers.len());
        self.file_headers.push(header);
        self.write_metadata(path, metadata)?;
        Ok(())
    }

    pub fn write_metadata(&mut self, path: &Path, metadata: Vec<MetadataHeader>) -> crate::Result<()> {
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        let header = if let Some(index) = self.name_map.get(&file_name).clone() {
            &mut self.file_headers[*index]
        } else {
            return Err(FileBaseError::FileNotFound(file_name).into());
        };
        let metadata_file_name = self.meta_data_path.with_extension(extensions::FILE_METADATA);
        let Ok(mut file) = OpenOptions::new().append(true).create(true).open(&metadata_file_name) else {
            log::error!("Error opening metadata file: {}", metadata_file_name.display());
            return Err(FileBaseError::CantOpenMetadata.into());
        };
        header.metadata_offset = file.seek(std::io::SeekFrom::Current(0))?;
        file.write(&[metadata.len() as u8])?;
        for meta in &metadata {
            file.write(&[meta.get_type().to_data()])?;
            let len = meta.data.len() as u32;
            file.write(&len.to_le_bytes())?;
            file.write(&meta.data)?;
        }
        self.save()?;
        Ok(())
    }

    pub fn get_hash(path: &Path) -> crate::Result<u64> {
        let data = fs::read(&path)?;
        let hash = XxHash3_64::oneshot(&data);
        Ok(hash)
    }

    fn create_header(&mut self, index: usize, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
        match scan_file(&path) {
            Ok(meta_data) => {
                let metadata_file_name = self.meta_data_path.with_extension(extensions::FILE_METADATA);
                let md_len = if let Ok(md) = fs::metadata(&metadata_file_name) { md.len() } else { 0 };
                let Ok(mut file) = OpenOptions::new().append(true).create(true).open(&metadata_file_name) else {
                    log::error!("Error opening metadata file: {}", metadata_file_name.display());
                    return Err(FileBaseError::CantOpenMetadata.into());
                };
                self.file_headers[index].metadata_offset = md_len;
                file.write(&[meta_data.len() as u8])?;
                for meta in &meta_data {
                    file.write(&[meta.get_type().to_data()])?;
                    let len = meta.data.len() as u32;
                    file.write(&len.to_le_bytes())?;
                    file.write(&meta.data)?;
                }
                self.save()?;
                Ok(meta_data)
            }
            Err(err) => {
                log::error!("Error scanning file {}: {}", path.display(), err);
                Ok(Vec::new())
            }
        }
    }

    pub fn save(&self) -> crate::Result<()> {
        match OpenOptions::new().write(true).create(true).open(&self.meta_data_path) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                for header in &self.file_headers {
                    header.write(&mut writer)?;
                }
            }
            Err(err) => {
                log::error!("Error saving filebase to {}: {}", self.meta_data_path.display(), err);
            }
        }
        Ok(())
    }

    fn get_metadata(&self, header: &FileHeader) -> crate::Result<Vec<MetadataHeader>> {
        let metadata_file_name: PathBuf = self.meta_data_path.with_extension(extensions::FILE_METADATA);
        let Ok(file) = OpenOptions::new().read(true).open(&metadata_file_name) else {
            log::error!("Error opening metadata file: {}", metadata_file_name.display());
            return Err(FileBaseError::CantOpenMetadata.into());
        };

        let mut reader = BufReader::new(file);
        reader.seek(std::io::SeekFrom::Start(header.metadata_offset()))?;

        let mut size = [0; 1];
        reader.read_exact(&mut size)?;
        let size = size[0] as usize;
        let mut data = [0; 5];
        let mut result = Vec::new();
        for _ in 0..size {
            reader.read_exact(&mut data)?;
            let metadata_type = MetadataType::from_data(data[0]);
            let data_len = u32::from_le_bytes(data[1..5].try_into().unwrap()) as usize;

            let mut data = vec![0; data_len];
            reader.read_exact(&mut data)?;
            let header = MetadataHeader { metadata_type, data };
            result.push(header);
        }

        Ok(result)
    }

    pub fn full_path(&self, entry: &FileHeader) -> PathBuf {
        self.dir.join(&entry.name)
    }
}
