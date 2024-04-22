use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

use walkdir::WalkDir;
use zip::write::ExtendedFileOptions;

use super::bbstro_fingerprint::FingerprintData;

pub fn repack_files(scan_dir: &PathBuf, fingerprints: FingerprintData) -> crate::Result<()> {
    for path in WalkDir::new(scan_dir).into_iter().filter_map(|e| e.ok()) {
        if path.path().is_dir() {
            continue;
        }

        if let Some(extension) = path.path().extension() {
            if let Err(err) = repack_file(&path.path(), extension.to_ascii_uppercase(), &fingerprints) {
                eprintln!("Error while repacking {}:{}", path.path().display(), err);
                continue;
            }
        }
    }

    Ok(())
}

fn repack_file(path: &std::path::Path, extension: std::ffi::OsString, fingerprints: &FingerprintData) -> crate::Result<()> {
    let dir = tempfile::tempdir()?;

    match extension.to_str() {
        Some("ZIP") => unpack_zip(path, dir.path())?,
        //        Some("LHA") | Some("LZH") => unpack_lha(path, dir.path())?,
        //         Some("RAR") => unpack_rar(path, dir.path())?,
        _ => return Ok(()),
    }

    let file = tempfile::NamedTempFile::new()?;
    pack(dir.path(), file.as_file(), fingerprints)?;
    fs::remove_file(&path)?;
    let new_path = path.with_file_name(path.file_name().unwrap().to_ascii_lowercase()).with_extension("zip");
    fs::copy(file.path(), new_path)?;
    fs::remove_file(file.path())?;
    Ok(())
}

fn pack(src: &std::path::Path, out_file: &File, fingerprints: &FingerprintData) -> crate::Result<()> {
    let mut zip = zip::ZipWriter::new(BufWriter::new(out_file));

    for path in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        if path.path().is_dir() {
            continue;
        }

        let content = fs::read(path.path())?;
        if fingerprints.is_match(path.path(), &content) {
            println!("Removed BBStro {} from archive", path.path().display());
            continue;
        }
        let options = zip::write::FileOptions::<ExtendedFileOptions>::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(9));
        zip.start_file(path.path().file_name().unwrap().to_string_lossy().to_string(), options)?;
        zip.write_all(&content)?;
    }

    Ok(())
}

fn unpack_zip(path: &std::path::Path, dest_path: &std::path::Path) -> crate::Result<()> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let out_path = if let Some(out_path) = file.enclosed_name() {
            out_path.to_path_buf()
        } else {
            println!("Entry {} has a suspicious path", file.name());
            return Ok(());
        };
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        let dest_file = dest_path.join(out_path);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(dest_file, content)?;
    }
    Ok(())
}
