use std::{ffi::OsStr, fs, io::BufReader, path::Path};

use codepages::{normalize_file, tables::get_utf8};
use icy_sauce::SauceRecord;
use unarc_rs::unified::{ArchiveFormat, UnifiedArchive};

use crate::file_base::{
    FileBase,
    metadata::{MetadataHeader, MetadataType},
};
pub mod repack;

pub mod bbstro_fingerprint;

pub fn scan_file(path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let mut info = Vec::new();
    let hash = FileBase::get_hash(path)?;
    info.push(MetadataHeader {
        metadata_type: MetadataType::Hash,
        data: hash.to_le_bytes().to_vec(),
    });

    let Some(extension) = path.extension() else {
        return Ok(info);
    };
    let extension = extension.to_string_lossy().to_uppercase();

    match extension.as_str() {
        "ANS" | "NFO" | "TXT" | "XB" | "PCB" | "ASC" => scan_sauce(info, path),
        "EXE" | "COM" | "BAT" | "BMP" | "GIF" | "JPG" => Ok(info),
        _ => scan_archive(info, path),
    }
}

const FILE_DESCR: [&str; 4] = ["desc.sdi", "file_id.diz", "file_id.ans", "file_id.pcb"];

fn is_short_desc(name: &std::ffi::OsStr) -> Option<i32> {
    for (i, descr) in FILE_DESCR.iter().enumerate() {
        if name.eq_ignore_ascii_case(descr) {
            return Some(i as i32);
        }
    }
    None
}

fn scan_sauce(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    if let Ok(Some(sauce)) = SauceRecord::from_path(path) {
        info.push(MetadataHeader::new(MetadataType::Sauce, sauce.to_bytes_without_eof()));
    }
    Ok(info)
}

fn scan_archive(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let Some(format) = ArchiveFormat::from_path(path) else {
        return Ok(info);
    };
    let mut archive = UnifiedArchive::open_with_format(BufReader::new(fs::File::open(path)?), format)?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    while let Some(entry) = archive.next_entry()? {
        let Some(prio) = is_short_desc(OsStr::new(entry.file_name())) else {
            continue;
        };
        if prio <= last_prio {
            continue;
        }
        // Some members use a compression method the format's decoder does not cover.
        let Ok(data) = archive.read(&entry) else {
            continue;
        };
        last_prio = prio;
        short_descr = data;
    }

    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }

    Ok(info)
}

fn get_file_id(mut content: Vec<u8>) -> String {
    while content.ends_with(b"\r") || content.ends_with(b"\n") || content.ends_with(b" ") || content.ends_with(b"\t") || content.ends_with(&[0x1A]) {
        content.pop();
    }
    let file_id = normalize_file(&content);
    get_utf8(&file_id)
}
