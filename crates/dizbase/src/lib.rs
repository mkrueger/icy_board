use std::{fs, io::BufReader, path::Path};

use unrar::Archive;
use zip::DateTime;

pub mod file_base;
pub mod file_base_scanner;
mod macros;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod extensions {
    /// filename.FMD - Lastread information
    pub const FILE_METADATA: &str = "fmd";
}

pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub date: DateTime,
}

pub fn scan_file_contents(path: &std::path::PathBuf) -> crate::Result<Vec<FileInfo>> {
    let extension = path.extension().unwrap_or_default().to_ascii_uppercase().to_string_lossy().to_string();
    match extension.as_str() {
        "ZIP" => scan_zip(path),
        "LHA" | "LZH" => scan_lha(path),

        "RAR" => scan_rar(path),

        ext => {
            println!("Unknown extension {:?}", ext);
            Err("evlvevl".into())
        }
    }
}

fn scan_zip(path: &Path) -> crate::Result<Vec<FileInfo>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut info = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let Some(outpath) = file.enclosed_name() else {
            continue;
        };
        info.push(FileInfo {
            name: outpath.file_name().unwrap_or_default().to_string_lossy().to_string(),
            size: file.size(),
            compressed_size: file.compressed_size(),
            date: file.last_modified().unwrap_or_default(),
        });
    }
    Ok(info)
}

fn scan_lha(path: &Path) -> crate::Result<Vec<FileInfo>> {
    let mut lha_reader = delharc::parse_file(path)?;
    let mut info = Vec::new();
    loop {
        let header = lha_reader.header();
        let filename = header.parse_pathname();

        info.push(FileInfo {
            name: filename.file_name().unwrap_or_default().to_string_lossy().to_string(),
            size: header.original_size,
            compressed_size: header.compressed_size,
            date: unsafe { DateTime::from_msdos_unchecked(header.last_modified as u16, (header.last_modified >> 16) as u16) },
        });

        if !lha_reader.next_file()? {
            break;
        }
    }
    Ok(info)
}

fn scan_rar(path: &Path) -> crate::Result<Vec<FileInfo>> {
    let mut archive = Archive::new(path).open_for_processing()?;
    let mut info = Vec::new();

    while let Some(header) = archive.read_header()? {
        if header.entry().is_file() {
            info.push(FileInfo {
                name: header.entry().filename.file_name().unwrap_or_default().to_string_lossy().to_string(),
                size: header.entry().unpacked_size,
                compressed_size: 0,
                date: unsafe { DateTime::from_msdos_unchecked(header.entry().file_time as u16, (header.entry().file_time >> 16) as u16) },
            });
        }
        archive = header.skip()?;
    }
    Ok(info)
}
