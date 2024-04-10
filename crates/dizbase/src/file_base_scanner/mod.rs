use std::{
    fs,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use codepages::{normalize_file, tables::get_utf8};
use icy_sauce::SauceInformation;
use unrar::Archive;
use walkdir::WalkDir;

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
            let hash = murmurhash64::murmur_hash64a(&file, 0);
            info = info.with_hash(hash);
        }

        if let Some(extension) = path.path().extension() {
            match scan_file(info, &path.path(), extension.to_ascii_uppercase()) {
                Ok(i) => {
                    info = i;
                }
                Err(err) => {
                    eprintln!("{}", err);
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

        ext => {
            println!("Unknown extension {:?}", ext);
            Ok(info)
        }
    }
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

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let extract = if let Some(_outpath) = file.enclosed_name() {
            file.name().eq_ignore_ascii_case("FILE_ID.DIZ") || file.name().eq_ignore_ascii_case("DESC.SDI")
        } else {
            println!("Entry {} has a suspicious path", file.name());
            continue;
        };
        if extract {
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;
            return Ok(info.with_file_id(get_file_id(content)));
        }
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

fn scan_lha(info: FileInfo, path: &Path) -> crate::Result<FileInfo> {
    let mut lha_reader = delharc::parse_file(path)?;
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
