use std::{fs, io::BufReader};

use unarc_rs::{
    date_time::DosDateTime,
    unified::{ArchiveFormat, UnifiedArchive},
};

pub mod file_base;
pub mod file_base_scanner;

mod macros;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub date: DosDateTime,
}

pub fn scan_file_contents(path: &std::path::PathBuf) -> crate::Result<Vec<FileInfo>> {
    let Some(format) = ArchiveFormat::from_path(path) else {
        return Err(format!("Unsupported archive format: {}", path.display()).into());
    };
    let mut archive = UnifiedArchive::open_with_format(BufReader::new(fs::File::open(path)?), format)?;
    let mut info = Vec::new();

    while let Some(entry) = archive.next_entry()? {
        info.push(FileInfo {
            name: entry.file_name().to_string(),
            size: entry.original_size(),
            compressed_size: entry.compressed_size(),
            date: entry.modified_time().unwrap_or(DosDateTime::new(0)),
        });
    }

    Ok(info)
}


