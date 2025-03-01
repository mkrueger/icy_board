use std::{
    fs,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use codepages::{normalize_file, tables::get_utf8};
use icy_sauce::SauceInformation;
use unrar::Archive;

use crate::file_base::metadata::{MetadataHeader, MetadataType};
pub mod repack;

pub mod bbstro_fingerprint;

pub fn scan_file(path: &Path, extension: &str) -> crate::Result<Vec<MetadataHeader>> {
    let info = Vec::new();
    match extension {
        "ZIP" => scan_zip(info, &path),
        "LHA" | "LZH" => scan_lha(info, path),

        "ANS" | "NFO" | "TXT" | "XB" | "PCB" | "ASC" => scan_sauce(info, path),

        "RAR" => scan_rar(info, path),
        "EXE" | "COM" | "BAT" | "BMP" | "GIF" | "JPG" => Ok(info),

        _ext => Ok(info),
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
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    if reader.seek(SeekFrom::End(-128)).is_ok() {
        let mut sauce = [0u8; 128];
        reader.read_exact(&mut sauce)?;
        if let Ok(Some(_)) = SauceInformation::read(&sauce[..]) {
            info.push(MetadataHeader::new(MetadataType::Sauce, sauce.to_vec()));
        }
    }
    Ok(info)
}

fn scan_zip(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if let Some(outpath) = file.enclosed_name() {
            if let Some(prio) = is_short_desc(&outpath.file_name().unwrap()) {
                if prio <= last_prio {
                    continue;
                }
                last_prio = prio;
                short_descr.clear();
                file.read_to_end(&mut short_descr)?;
            }
        } else {
            println!("Entry {} has a suspicious path", file.name());
            continue;
        };
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

fn scan_lha(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let mut lha_reader = delharc::parse_file(path)?;
    let mut last_prio = -1;
    let mut short_descr = Vec::new();
    loop {
        let header = lha_reader.header();
        let filename = header.parse_pathname();

        if let Some(name) = filename.file_name() {
            if let Some(prio) = is_short_desc(name) {
                if prio <= last_prio {
                    continue;
                }
                last_prio = prio;
                if lha_reader.is_decoder_supported() {
                    short_descr.clear();
                    lha_reader.read_to_end(&mut short_descr)?;
                    lha_reader.crc_check()?;
                } else if header.is_directory() {
                    eprintln!("skipping: an empty directory");
                } else {
                    eprintln!("skipping: has unsupported compression method");
                    break;
                }
            }
        }
        if !lha_reader.next_file()? {
            break;
        }
    }
    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }
    Ok(info)
}

fn scan_rar(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let mut archive = Archive::new(path).open_for_processing()?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    while let Some(header) = archive.read_header()? {
        if !header.entry().is_file() {
            archive = header.skip()?
        } else {
            let name = header.entry().filename.file_name().unwrap_or_default();

            if let Some(prio) = is_short_desc(name) {
                if prio <= last_prio {
                    archive = header.skip()?;
                    continue;
                }
                last_prio = prio;
                short_descr.clear();
                archive = header.extract_to("out.tmp")?;
                short_descr = fs::read("out.tmp")?;
            } else {
                archive = header.skip()?
            }
        }
    }

    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }

    Ok(info)
}
