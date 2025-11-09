use std::{
    ffi::OsStr,
    fs,
    io::{BufReader, Read},
    path::Path,
};

use codepages::{normalize_file, tables::get_utf8};
use icy_sauce::SauceRecord;
use unarc_rs::{
    arc::arc_archive::ArcArchieve, arj::arj_archive::ArjArchieve, hyp::hyp_archive::HypArchieve, sq::sq_archive::SqArchieve, zoo::zoo_archive::ZooArchieve,
};
use unrar::Archive;

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
        "ZIP" => scan_zip(info, &path),
        "LHA" | "LZH" => scan_lha(info, path),

        "ANS" | "NFO" | "TXT" | "XB" | "PCB" | "ASC" => scan_sauce(info, path),

        "RAR" => scan_rar(info, path),
        "EXE" | "COM" | "BAT" | "BMP" | "GIF" | "JPG" => Ok(info),

        "ARJ" => scan_arj(info, path),
        "ARC" => scan_arc(info, path),
        "ZOO" => scan_zoo(info, path),
        "SQZ" => scan_sqz(info, path),
        "HYP" => scan_hyp(info, path),
        "SQ" | "SQ2" | "QQQ" => scan_sq(info, path),

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
    if let Ok(Some(sauce)) = SauceRecord::from_path(path) {
        info.push(MetadataHeader::new(MetadataType::Sauce, sauce.to_bytes_without_eof()));
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

fn scan_arj(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let file = fs::File::open(path)?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    let mut archieve = ArjArchieve::new(file)?;
    while let Ok(Some(header)) = archieve.get_next_entry() {
        if let Some(prio) = is_short_desc(OsStr::new(&header.name)) {
            if prio <= last_prio {
                continue;
            }
            last_prio = prio;
            short_descr = archieve.read(&header)?;
        }
    }

    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }

    Ok(info)
}

fn scan_arc(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let file = fs::File::open(path)?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    let mut archieve = ArcArchieve::new(file)?;
    while let Ok(Some(header)) = archieve.get_next_entry() {
        if let Some(prio) = is_short_desc(OsStr::new(&header.name)) {
            if prio <= last_prio {
                continue;
            }
            last_prio = prio;
            short_descr = archieve.read(&header)?;
        }
    }

    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }

    Ok(info)
}

fn scan_zoo(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let file = fs::File::open(path)?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    let mut archieve = ZooArchieve::new(file)?;
    while let Ok(Some(header)) = archieve.get_next_entry() {
        if let Some(prio) = is_short_desc(OsStr::new(&header.name)) {
            if prio <= last_prio {
                continue;
            }
            last_prio = prio;
            short_descr = archieve.read(&header)?;
        }
    }

    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }

    Ok(info)
}

fn scan_sqz(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let file = fs::File::open(path)?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    let mut archieve = ZooArchieve::new(file)?;
    while let Ok(Some(header)) = archieve.get_next_entry() {
        if let Some(prio) = is_short_desc(OsStr::new(&header.name)) {
            if prio <= last_prio {
                continue;
            }
            last_prio = prio;
            short_descr = archieve.read(&header)?;
        }
    }

    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }

    Ok(info)
}

fn scan_sq(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let file = fs::File::open(path)?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    let mut archieve = SqArchieve::new(file)?;
    while let Ok(Some(header)) = archieve.get_next_entry() {
        if let Some(prio) = is_short_desc(OsStr::new(&header.name)) {
            if prio <= last_prio {
                continue;
            }
            last_prio = prio;
            short_descr = archieve.read(&header)?;
        }
    }

    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }

    Ok(info)
}

fn scan_hyp(mut info: Vec<MetadataHeader>, path: &Path) -> crate::Result<Vec<MetadataHeader>> {
    let file = fs::File::open(path)?;
    let mut short_descr = Vec::new();
    let mut last_prio = -1;

    let mut archieve = HypArchieve::new(file)?;
    while let Ok(Some(header)) = archieve.get_next_entry() {
        if let Some(prio) = is_short_desc(OsStr::new(&header.name)) {
            // only uncompressed files are supported, so it's likely to fail.
            if let Ok(buffer) = archieve.read(&header) {
                if prio <= last_prio {
                    continue;
                }
                last_prio = prio;
                short_descr = buffer;
            }
        }
    }

    if !short_descr.is_empty() {
        info.push(MetadataHeader::new(MetadataType::FileID, get_file_id(short_descr).as_bytes().to_vec()));
    }

    Ok(info)
}
