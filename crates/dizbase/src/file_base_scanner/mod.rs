use std::{
    fs,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use codepages::{normalize_file, tables::get_utf8};
use icy_net::crc::get_crc32;
use icy_sauce::SauceInformation;
use unrar::Archive;
use walkdir::WalkDir;
pub mod repack;

pub mod bbstro_fingerprint;

use crate::file_base::{file_info::FileInfo, FileBase};

pub fn scan_file_directory(scan_dir: &PathBuf, out_file_base: &PathBuf) -> crate::Result<()> {
    if !scan_dir.is_dir() {
        return Err("Input is not a directory".into());
    }

    let base = FileBase::create(out_file_base)?;
    //  let paths = fs::read_dir(scan_dir)?;
    for path in WalkDir::new(scan_dir).into_iter().filter_map(|e| e.ok()) {
        if path.path().is_dir() {
            continue;
        }
        let mut info = FileInfo::new(path.path().file_name().unwrap().to_string_lossy().to_string());

        if let Ok(file) = fs::read(&path.path()) {
            info = info.with_size(file.len() as u64);
            let hash = get_crc32(&file);
            info = info.with_hash(hash);
        }

        if let Some(extension) = path.path().extension() {
            match scan_file(info, &path.path(), extension.to_ascii_uppercase()) {
                Ok(i) => {
                    info = i;
                }
                Err(err) => {
                    eprintln!("{}:{}", path.path().display(), err);
                    continue;
                }
            }
        }

        base.write_info(&info)?;
    }
    Ok(())
}

fn scan_file(info: FileInfo, path: &Path, extension: std::ffi::OsString) -> crate::Result<FileInfo> {
    match extension.to_str() {
        Some("ZIP") => scan_zip(info, &path),
        Some("LHA") | Some("LZH") => scan_lha(info, path),

        Some("ANS") | Some("NFO") | Some("TXT") => scan_sauce(info, path),

        Some("RAR") => scan_rar(info, path),
        Some("EXE") | Some("COM") | Some("BAT") | Some("BMP") | Some("GIF") | Some("JPG") => Ok(info),

        ext => {
            println!("Unknown extension {:?}", ext);
            Ok(info)
        }
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

fn scan_sauce(info: FileInfo, path: &Path) -> crate::Result<FileInfo> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    if reader.seek(SeekFrom::End(-128)).is_ok() {
        let mut sauce = [0u8; 128];
        reader.read_exact(&mut sauce)?;
        if let Ok(Some(_)) = SauceInformation::read(&sauce[..]) {
            return Ok(info.with_sauce(sauce.to_vec()));
        }
    }
    Ok(info)
}

fn scan_zip(info: FileInfo, path: &Path) -> crate::Result<FileInfo> {
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
    if short_descr.is_empty() {
        Ok(info)
    } else {
        Ok(info.with_file_id(get_file_id(short_descr)))
    }
}

fn get_file_id(mut content: Vec<u8>) -> String {
    while content.ends_with(b"\r") || content.ends_with(b"\n") || content.ends_with(b" ") || content.ends_with(b"\t") || content.ends_with(&[0x1A]) {
        content.pop();
    }
    let file_id = normalize_file(&content);
    get_utf8(&file_id)
}

fn scan_lha(info: FileInfo, path: &Path) -> crate::Result<FileInfo> {
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
    if short_descr.is_empty() {
        Ok(info)
    } else {
        Ok(info.with_file_id(get_file_id(short_descr)))
    }
}

fn scan_rar(info: FileInfo, path: &Path) -> crate::Result<FileInfo> {
    let mut archive = Archive::new(path).open_for_processing()?;

    let mut lha_reader = delharc::parse_file(path)?;

    while let Some(header) = archive.read_header()? {
        archive = if header.entry().is_file() {
            let name = header.entry().filename.to_string_lossy();
            if name.eq_ignore_ascii_case("FILE_ID.DIZ") || name.eq_ignore_ascii_case("DESC.SDI") {
                header.extract_to("out.tmp")?;
                let content = fs::read("out.tmp")?;
                return Ok(info.with_file_id(get_file_id(content)));
            }
            header.skip()?
        } else {
            header.skip()?
        };
    }

    loop {
        let header = lha_reader.header();
        let filename = header.parse_pathname();

        if let Some(name) = filename.file_name() {
            if name.eq_ignore_ascii_case("FILE_ID.DIZ") || name.eq_ignore_ascii_case("DESC.SDI") {
                if lha_reader.is_decoder_supported() {
                    let mut content = Vec::new();
                    lha_reader.read_to_end(&mut content)?;
                    lha_reader.crc_check()?;
                    return Ok(info.with_file_id(get_file_id(content)));
                } else if header.is_directory() {
                    eprintln!("skipping: an empty directory");
                } else {
                    eprintln!("skipping: has unsupported compression method");
                }
            }
        }

        if !lha_reader.next_file()? {
            break;
        }
    }
    Ok(info)
}
