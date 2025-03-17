use std::{fs, io::BufReader, path::Path};

use unarc_rs::{
    arc::arc_archive::ArcArchieve, arj::arj_archive::ArjArchieve, hyp::hyp_archive::HypArchieve, sq::sq_archive::SqArchieve, sqz::sqz_archive::SqzArchieve,
    zoo::zoo_archive::ZooArchieve,
};
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
        "ARJ" => scan_arj(path),
        "ARC" => scan_arc(path),
        "ZOO" => scan_zoo(path),
        "SQZ" => scan_sqz(path),
        "HYP" => scan_hyp(path),
        "SQ" | "SQ2" | "QQQ" => scan_sq(path),

        ext => {
            println!("Unknown extension {:?}", ext);
            Err("evlvevl".into())
        }
    }
}

fn scan_arj(path: &std::path::PathBuf) -> crate::Result<Vec<FileInfo>> {
    let file = fs::File::open(path)?;
    let mut archieve = ArjArchieve::new(file)?;
    let mut info = Vec::new();

    while let Ok(Some(header)) = archieve.get_next_entry() {
        let (date, time) = header.date_time_modified.into();
        info.push(FileInfo {
            name: header.name.clone(),
            size: header.original_size as u64,
            compressed_size: header.compressed_size as u64,
            date: unsafe { DateTime::from_msdos_unchecked(date, time) },
        });
    }

    Ok(info)
}

fn scan_arc(path: &std::path::PathBuf) -> crate::Result<Vec<FileInfo>> {
    let file = fs::File::open(path)?;
    let mut archieve = ArcArchieve::new(file)?;
    let mut info = Vec::new();

    while let Ok(Some(header)) = archieve.get_next_entry() {
        let (date, time) = header.date_time.into();
        info.push(FileInfo {
            name: header.name.clone(),
            size: header.original_size as u64,
            compressed_size: header.compressed_size as u64,
            date: unsafe { DateTime::from_msdos_unchecked(date, time) },
        });
    }

    Ok(info)
}

fn scan_zoo(path: &std::path::PathBuf) -> crate::Result<Vec<FileInfo>> {
    let file = fs::File::open(path)?;
    let mut archieve = ZooArchieve::new(file)?;
    let mut info = Vec::new();

    while let Ok(Some(header)) = archieve.get_next_entry() {
        let (date, time) = header.date_time.into();
        info.push(FileInfo {
            name: header.name.clone(),
            size: header.org_size as u64,
            compressed_size: header.size_now as u64,
            date: unsafe { DateTime::from_msdos_unchecked(date, time) },
        });
    }

    Ok(info)
}

fn scan_sqz(path: &std::path::PathBuf) -> crate::Result<Vec<FileInfo>> {
    let file = fs::File::open(path)?;
    let mut archieve = SqzArchieve::new(file)?;
    let mut info = Vec::new();

    while let Ok(Some(header)) = archieve.get_next_entry() {
        let (date, time) = header.date_time.into();
        info.push(FileInfo {
            name: header.name.clone(),
            size: header.original_size as u64,
            compressed_size: header.compressed_size as u64,
            date: unsafe { DateTime::from_msdos_unchecked(date, time) },
        });
    }

    Ok(info)
}

fn scan_sq(path: &std::path::PathBuf) -> crate::Result<Vec<FileInfo>> {
    let file = fs::File::open(path)?;
    let mut archieve = SqArchieve::new(file)?;
    let mut info = Vec::new();

    while let Ok(Some(header)) = archieve.get_next_entry() {
        info.push(FileInfo {
            name: header.name.clone(),
            size: 0,
            compressed_size: 0,
            date: unsafe { DateTime::from_msdos_unchecked(0, 0) },
        });
    }

    Ok(info)
}

fn scan_hyp(path: &std::path::PathBuf) -> crate::Result<Vec<FileInfo>> {
    let file = fs::File::open(path)?;
    let mut archieve = HypArchieve::new(file)?;
    let mut info = Vec::new();

    while let Ok(Some(header)) = archieve.get_next_entry() {
        let (date, time) = header.date_time.into();
        info.push(FileInfo {
            name: header.name.clone(),
            size: header.original_size as u64,
            compressed_size: header.compressed_size as u64,
            date: unsafe { DateTime::from_msdos_unchecked(date, time) },
        });
    }

    Ok(info)
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
