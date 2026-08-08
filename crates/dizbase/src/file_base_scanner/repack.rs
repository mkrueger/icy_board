use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

use unarc_rs::unified::{ArchiveFormat, UnifiedArchive};
use walkdir::WalkDir;
use zip::write::ExtendedFileOptions;

use super::bbstro_fingerprint::FingerprintData;

pub fn repack_files(scan_dir: &PathBuf, fingerprints: FingerprintData) -> crate::Result<()> {
    for path in WalkDir::new(scan_dir).into_iter().filter_map(|e| e.ok()) {
        if path.path().is_dir() {
            continue;
        }

        if let Err(err) = repack_file(path.path(), &fingerprints) {
            eprintln!("Error while repacking {}:{}", path.path().display(), err);
            continue;
        }
    }

    Ok(())
}

fn repack_file(path: &std::path::Path, fingerprints: &FingerprintData) -> crate::Result<()> {
    if ArchiveFormat::from_path(path).is_none() {
        return Ok(());
    }
    let dir = tempfile::tempdir()?;
    unpack(path, dir.path())?;

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

fn unpack(path: &std::path::Path, dest_path: &std::path::Path) -> crate::Result<()> {
    let Some(format) = ArchiveFormat::from_path(path) else {
        return Ok(());
    };
    let mut archive = UnifiedArchive::open_with_format(BufReader::new(fs::File::open(path)?), format)?;

    fs::create_dir_all(dest_path)?;
    while let Some(entry) = archive.next_entry()? {
        // Flattening the member name keeps a crafted archive from escaping the temp dir.
        let content = archive.read(&entry)?;
        fs::write(dest_path.join(entry.file_name()), content)?;
    }
    Ok(())
}
