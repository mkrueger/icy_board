use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use chrono::{DateTime, Utc};
use icy_net::crc::get_crc32;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use thiserror::Error;

use crate::extensions;

use self::{
    base_header_info::{base_header_attributes, FileBaseHeaderInfo},
    file_header::FileHeader,
    file_info::FileInfo,
    metadata::{MetadaType, MetadataHeader},
    pattern::Pattern,
};

pub mod base_header_info;
pub mod file_header;
pub mod file_info;
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

pub struct FileBase {
    file_name: PathBuf,
    header_info: FileBaseHeaderInfo,
    locked: AtomicBool,
    file_headers: Vec<FileHeader>,
}

impl FileBase {
    /// Epens an existing file base with base path (without any extension)
    pub fn open<P: AsRef<Path>>(file_base: P) -> crate::Result<Self> {
        let index_file_name = file_base.as_ref().with_extension(extensions::FILE_INDEX);
        let header_info = FileBaseHeaderInfo::load(&mut File::open(index_file_name)?)?;
        Ok(Self {
            file_name: file_base.as_ref().into(),
            header_info,
            locked: AtomicBool::new(false),
            file_headers: Vec::new(),
        })
    }

    /// Creates a new file base
    pub fn create<P: AsRef<Path>>(file_name: P) -> crate::Result<Self> {
        let index_file_name = file_name.as_ref().with_extension(extensions::FILE_INDEX);
        FileBaseHeaderInfo::create(&index_file_name, 0, 0)?;
        fs::write(file_name.as_ref().with_extension(extensions::FILE_METADATA), "")?;
        Self::open(file_name)
    }

    /// Creates a new password protected file base
    pub fn create_with_password<P: AsRef<Path>>(file_name: P, password: &str) -> crate::Result<Self> {
        let index_file_name = file_name.as_ref().with_extension(extensions::FILE_INDEX);
        FileBaseHeaderInfo::create(&index_file_name, Self::get_pw_hash(password), base_header_attributes::PASSWORD)?;
        fs::write(file_name.as_ref().with_extension(extensions::FILE_METADATA), "")?;
        Self::open(file_name)
    }

    pub fn get_filename(&self) -> &Path {
        &self.file_name
    }

    pub fn date_created(&self) -> Option<DateTime<Utc>> {
        self.header_info.date_created()
    }

    /// True, if a password is required to access this msg base
    pub fn needs_password(&self) -> bool {
        self.header_info.attributes & base_header_info::base_header_attributes::PASSWORD != 0
    }

    /// Checks if a password is valid.
    pub fn is_password_valid(&self, password: &str) -> bool {
        self.header_info.password() == Self::get_pw_hash(password)
    }

    /// Locks the file base.
    /// User is responsible for locking/unlocking.
    ///
    /// Note that locking is just process only.
    pub fn lock(&self) {
        while self.locked.swap(true, std::sync::atomic::Ordering::Acquire) {
            std::hint::spin_loop();
        }
    }

    /// Unlocks the file base
    pub fn unlock(&self) {
        self.locked.store(false, std::sync::atomic::Ordering::Release);
    }

    pub fn read_header(&self, header_number: u64) -> crate::Result<FileHeader> {
        let index_file_name = self.file_name.with_extension(extensions::FILE_INDEX);
        let offset = FileBaseHeaderInfo::HEADER_SIZE + header_number * FileHeader::HEADER_SIZE as u64;
        let mut file = File::open(index_file_name)?;
        file.seek(std::io::SeekFrom::Start(offset))?;

        let mut reader = BufReader::new(file);

        FileHeader::read(&mut reader)
    }

    /// Returns lowercased crc hash of the password
    fn get_pw_hash(password: &str) -> u32 {
        get_crc32(password.to_lowercase().as_bytes())
    }

    pub fn iter(&self) -> impl Iterator<Item = crate::Result<FileHeader>> {
        let index_file_name = self.file_name.with_extension(extensions::FILE_INDEX);
        let mut f = File::open(index_file_name).unwrap();
        let len = f.metadata().unwrap().len();
        f.seek(std::io::SeekFrom::Start(FileBaseHeaderInfo::HEADER_SIZE)).unwrap();
        FileBaseMessageIter {
            reader: BufReader::new(f),
            len,
        }
    }

    pub fn write_info(&self, info: &FileInfo) -> crate::Result<()> {
        let mut header = info.create_header();
        let metadata = info.create_metadata();

        let metadata_file = self.file_name.with_extension(extensions::FILE_METADATA);
        let mut file = OpenOptions::new().append(true).open(metadata_file)?;

        header.metadata_offset = file.seek(std::io::SeekFrom::End(0))?;
        file.write_all(&(metadata.len() as u32).to_le_bytes())?;
        file.write_all(&metadata)?;

        let index_file_name = self.file_name.with_extension(extensions::FILE_INDEX);
        let file = OpenOptions::new().append(true).open(index_file_name)?;
        let mut writer = BufWriter::new(file);
        header.write(&mut writer)?;

        Ok(())
    }

    pub fn load_headers(&mut self) -> crate::Result<()> {
        self.file_headers.clear();
        for header in self.iter().flatten() {
            self.file_headers.push(header);
        }
        Ok(())
    }

    pub fn find_files(&self, search: &str) -> crate::Result<Vec<&FileHeader>> {
        let Ok(pattern) = Pattern::new(search) else {
            return Err(FileBaseError::InvalidSearchToken.into());
        };

        Ok(self.file_headers.par_iter().filter(|header| pattern.matches(&header.name)).collect())
    }

    pub fn find_newer_files(&self, timestamp: u64) -> crate::Result<Vec<&FileHeader>> {
        Ok(self.file_headers.par_iter().filter(|header| header.file_date > timestamp).collect())
    }
    pub fn find_files_with_pattern(&self, str: &str) -> crate::Result<Vec<&FileHeader>> {
        let lc = str.to_lowercase();
        let bytes = lc.as_bytes();

        Ok(self
            .file_headers
            .par_iter()
            .filter(|header| {
                if let Ok(metadata) = self.read_metadata(header) {
                    for m in metadata {
                        if m.metadata_type == MetadaType::FileID {
                            println!("found file id for {}", header.name);
                            if find_match(m.data, bytes) {
                                return true;
                            }
                        }
                    }
                }
                false
            })
            .collect())
    }

    pub fn get_headers(&self) -> &Vec<FileHeader> {
        &self.file_headers
    }

    pub fn read_metadata(&self, header: &FileHeader) -> crate::Result<Vec<MetadataHeader>> {
        let metadata_file_name = self.file_name.with_extension(extensions::FILE_METADATA);
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

fn find_match(data: Vec<u8>, pattern: &[u8]) -> bool {
    let mut data = &data.to_ascii_lowercase()[..];
    while data.len() > pattern.len() {
        if data.starts_with(pattern) {
            return true;
        }
        data = &data[1..];
    }
    false
}

struct FileBaseMessageIter {
    reader: BufReader<File>,
    len: u64,
}

impl Iterator for FileBaseMessageIter {
    type Item = crate::Result<FileHeader>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(pos) = self.reader.stream_position() {
            if pos >= self.len {
                return None;
            }
            Some(FileHeader::read(&mut self.reader))
        } else {
            None
        }
    }
}
