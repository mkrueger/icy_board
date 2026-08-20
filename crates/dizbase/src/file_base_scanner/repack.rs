use std::{
    collections::HashSet,
    fs,
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use unarc_rs::unified::{ArchiveFormat, UnifiedArchive};
use zip::write::ExtendedFileOptions;

use super::bbstro_fingerprint::FingerprintData;

/// What `repack_file` is allowed to do to the archive it is handed.
pub struct RepackOptions {
    pub lowercase_names: bool,
    /// Do all the work and report it, but leave the directory as it was.
    pub dry_run: bool,
}

impl Default for RepackOptions {
    fn default() -> Self {
        Self {
            lowercase_names: true,
            dry_run: false,
        }
    }
}

pub enum Repacked {
    /// Nothing was touched, and the reason is worth telling the operator.
    Skipped(&'static str),
    /// Already a zip under the right name, carrying nothing the fingerprints object to.
    Unchanged,
    Converted {
        name: String,
        removed: Vec<String>,
        before: u64,
        after: u64,
    },
}

/// Rewrites one archive as a zip and drops the members the fingerprints recognise.
///
/// The new archive is written beside the old one and moved into place in one step, so an
/// interrupted run leaves either the old file or the new one, never neither.
pub fn repack_file(path: &Path, fingerprints: &FingerprintData, options: &RepackOptions) -> crate::Result<Repacked> {
    let Some(format) = ArchiveFormat::from_path(path) else {
        return Ok(Repacked::Skipped("not an archive"));
    };
    let Some(target) = target_path(path, options.lowercase_names) else {
        return Ok(Repacked::Skipped("no usable file name"));
    };
    let renamed = path != target;
    // A case insensitive volume answers to both names with the same file.
    let in_place = renamed && target.exists() && is_same_file(path, &target);
    if renamed && target.exists() && !in_place {
        return Err(format!("{} is already there", target.display()).into());
    }
    let directory = path.parent().unwrap_or(Path::new("."));

    let metadata = fs::metadata(path)?;
    let before = metadata.len();
    let modified = metadata.modified().ok();

    let mut archive = UnifiedArchive::open_with_format(BufReader::new(fs::File::open(path)?), format)?;
    let temporary = tempfile::NamedTempFile::new_in(directory)?;
    let mut zip = zip::ZipWriter::new(BufWriter::new(temporary.as_file()));
    let mut removed = Vec::new();
    let mut written = HashSet::new();

    while let Some(entry) = archive.next_entry()? {
        if entry.is_encrypted() {
            return Ok(Repacked::Skipped("needs a password"));
        }
        let Some(name) = member_name(entry.name()) else {
            continue;
        };
        // Losing a member to a gap in a decoder would be worse than leaving the archive alone.
        let content = archive.read(&entry)?;
        if fingerprints.is_match(&name, &content) {
            removed.push(name);
            continue;
        }
        if !written.insert(name.clone()) {
            return Err(format!("{} holds '{}' twice", path.display(), name).into());
        }
        let mut file_options = zip::write::FileOptions::<ExtendedFileOptions>::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(9));
        if let Some(time) = entry.modified_time()
            && let Ok(time) = zip::DateTime::from_date_and_time(time.year(), time.month(), time.day(), time.hour(), time.minute(), time.second())
        {
            file_options = file_options.last_modified_time(time);
        }
        zip.start_file(name, file_options)?;
        zip.write_all(&content)?;
    }
    zip.finish()?.flush()?;

    if !renamed && removed.is_empty() && format == ArchiveFormat::Zip {
        return Ok(Repacked::Unchanged);
    }

    let after = temporary.as_file().metadata()?.len();
    let result = Repacked::Converted {
        name: file_name(&target),
        removed,
        before,
        after,
    };
    if options.dry_run {
        return Ok(result);
    }

    // The board sorts by date, so a repack must not make an old file look new.
    if let Some(modified) = modified {
        let _ = temporary.as_file().set_modified(modified);
    }
    let _ = temporary.as_file().set_permissions(metadata.permissions());
    temporary.persist(&target)?;
    if renamed && !in_place {
        fs::remove_file(path)?;
    }
    Ok(result)
}

fn target_path(path: &Path, lowercase: bool) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let stem = if lowercase { stem.to_ascii_lowercase() } else { stem.to_string() };
    if stem.is_empty() {
        return None;
    }
    Some(path.with_file_name(format!("{}.zip", stem)))
}

#[cfg(unix)]
fn is_same_file(left: &Path, right: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (fs::metadata(left), fs::metadata(right)) {
        (Ok(left), Ok(right)) => left.dev() == right.dev() && left.ino() == right.ino(),
        _ => false,
    }
}

/// Windows volumes do not tell `FOO.ZIP` and `foo.zip` apart.
#[cfg(not(unix))]
fn is_same_file(left: &Path, right: &Path) -> bool {
    match (left.file_name().and_then(|n| n.to_str()), right.file_name().and_then(|n| n.to_str())) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        _ => false,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()
}

/// Keeps a member that calls itself `../../etc/passwd` from deciding where it lands.
fn member_name(name: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in name.split(['/', '\\']) {
        match part {
            "" | "." | ".." => continue,
            _ => parts.push(part),
        }
    }
    // A drive letter is a prefix, not a directory anyone wants back.
    if parts.first().is_some_and(|first| first.len() == 2 && first.ends_with(':')) {
        parts.remove(0);
    }
    if parts.is_empty() { None } else { Some(parts.join("/")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_member_cannot_climb_out_of_the_archive() {
        assert_eq!(member_name("../../etc/passwd").as_deref(), Some("etc/passwd"));
    }

    #[test]
    fn test_a_member_keeps_the_directory_it_came_from() {
        assert_eq!(member_name("docs\\readme.txt").as_deref(), Some("docs/readme.txt"));
    }

    #[test]
    fn test_a_member_that_is_only_separators_is_dropped() {
        assert_eq!(member_name("../.."), None);
    }

    #[test]
    fn test_a_dos_drive_is_not_a_directory() {
        assert_eq!(member_name("c:\\dos\\run.exe").as_deref(), Some("dos/run.exe"));
    }

    #[test]
    fn test_an_archive_is_renamed_to_a_lower_case_zip() {
        assert_eq!(target_path(Path::new("/files/GAME.LHA"), true), Some(PathBuf::from("/files/game.zip")));
    }

    #[test]
    fn test_the_case_of_a_name_can_be_left_alone() {
        assert_eq!(target_path(Path::new("/files/GAME.LHA"), false), Some(PathBuf::from("/files/GAME.zip")));
    }

    #[test]
    fn test_a_file_is_the_same_as_itself() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(is_same_file(file.path(), file.path()));
    }

    #[test]
    fn test_two_files_are_not_the_same() {
        let left = tempfile::NamedTempFile::new().unwrap();
        let right = tempfile::NamedTempFile::new().unwrap();
        assert!(!is_same_file(left.path(), right.path()));
    }
}
