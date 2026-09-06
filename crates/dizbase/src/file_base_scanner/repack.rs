use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use unarc_rs::unified::{ArchiveFormat, UnifiedArchive};
use zip::write::ExtendedFileOptions;

use super::{bbstro_fingerprint::FingerprintData, description_cleaner::DescriptionChange};
use super::{description_cleaner::RuleAction, text_member::TextMemberMatch};

// Bound stacked advertisement removal without exposing an implementation detail
// as a SysOp setting.
const MAX_DESCRIPTION_CLEAN_PASSES: usize = 8;

/// How to handle the archive comment, independently of advertisement removal.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveCommentMode {
    /// Preserve the original raw ZIP comment; other formats do not expose comments.
    #[default]
    Preserve,
    Remove,
    /// Use the exact replacement bytes, including an empty replacement.
    Replace,
}

/// What `repack_file` is allowed to do to the archive it is handed.
pub struct RepackOptions {
    pub lowercase_names: bool,
    /// Remove recognized ad members and description blocks.
    pub remove_advertisements: bool,
    /// Rewrite even an otherwise unchanged ZIP using the requested compression.
    pub recompress: bool,
    pub compression_level: i64,
    pub max_members: usize,
    pub max_member_size: u64,
    pub max_expanded_size: u64,
    pub max_compression_ratio: u64,
    pub additions: Vec<ArchiveAddition>,
    pub archive_comment_mode: ArchiveCommentMode,
    /// Used only in Replace mode; Preserve and Remove ignore these bytes.
    pub replacement_archive_comment: Vec<u8>,
    /// Do all the work and report it, but leave the directory as it was.
    pub dry_run: bool,
}

impl Default for RepackOptions {
    fn default() -> Self {
        Self {
            lowercase_names: true,
            remove_advertisements: true,
            recompress: true,
            compression_level: 9,
            max_members: 10_000,
            max_member_size: 512 * 1024 * 1024,
            max_expanded_size: 2 * 1024 * 1024 * 1024,
            max_compression_ratio: 1_000,
            additions: Vec::new(),
            archive_comment_mode: ArchiveCommentMode::Preserve,
            replacement_archive_comment: Vec::new(),
            dry_run: false,
        }
    }
}

#[derive(Clone)]
pub struct ArchiveAddition {
    pub name: String,
    pub content: Vec<u8>,
}

pub struct CleanedDescription {
    pub name: String,
    pub changes: Vec<DescriptionChange>,
}

pub struct TextMemberFinding {
    pub name: String,
    pub matched: TextMemberMatch,
}

impl std::fmt::Display for TextMemberFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "text member {:?}: rule {:?}, {:?}, {}, sha256={}",
            self.name, self.matched.rule_id, self.matched.action, self.matched.encoding, self.matched.sha256
        )
    }
}

pub enum Repacked {
    /// Nothing was touched, and the reason is worth telling the operator.
    Skipped(&'static str),
    /// Already a zip under the right name, carrying nothing the fingerprints object to.
    Unchanged,
    /// Report-only findings without an archive rewrite or a forced quarantine.
    Reported {
        text_members: Vec<TextMemberFinding>,
    },
    NeedsReview {
        reason: String,
    },
    Converted {
        name: String,
        removed: Vec<String>,
        added: Vec<String>,
        cleaned_descriptions: Vec<CleanedDescription>,
        text_members: Vec<TextMemberFinding>,
        archive_comment_changed: bool,
        before: u64,
        after: u64,
    },
}

/// Rewrites one archive as a zip and drops the members the fingerprints recognise.
///
/// The new archive is written beside the old one and moved into place in one step, so an
/// interrupted run leaves either the old file or the new one, never neither.
pub fn repack_file(path: &Path, fingerprints: &FingerprintData, options: &RepackOptions) -> crate::Result<Repacked> {
    if !(0..=9).contains(&options.compression_level) {
        return Err(format!("invalid zip compression level {} (expected 0..=9)", options.compression_level).into());
    }
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
    let original_comment = if format == ArchiveFormat::Zip {
        let archive = zip::ZipArchive::new(BufReader::new(fs::File::open(path)?))?;
        archive.comment().to_vec()
    } else {
        Vec::new()
    };
    // ZIP comments can be read and preserved. UnifiedArchive does not expose
    // comments from other formats, so they are not transferred on conversion.
    let comment = match options.archive_comment_mode {
        ArchiveCommentMode::Preserve => original_comment.as_slice(),
        ArchiveCommentMode::Remove => &[],
        ArchiveCommentMode::Replace => options.replacement_archive_comment.as_slice(),
    };
    let comment_changed = comment != original_comment.as_slice();

    let mut archive = UnifiedArchive::open_with_format(BufReader::new(fs::File::open(path)?), format)?;
    let temporary = tempfile::NamedTempFile::new_in(directory)?;
    let mut zip = zip::ZipWriter::new(BufWriter::new(temporary.as_file()));
    let mut removed = Vec::new();
    let mut cleaned_descriptions = Vec::new();
    let mut text_members = Vec::new();
    let mut written = HashSet::new();
    let mut additions: HashMap<String, &ArchiveAddition> = HashMap::new();
    for addition in &options.additions {
        let Some(name) = member_name(&addition.name) else {
            return Err(format!("addition '{}' has no usable member name", addition.name).into());
        };
        // Description cleanup is deliberately one-way: advertisements may be
        // removed, but never injected through the generic additions list either.
        let basename = name.rsplit('/').next().unwrap_or(&name);
        if super::is_short_desc(std::ffi::OsStr::new(basename)).is_some() {
            return Ok(Repacked::NeedsReview {
                reason: format!(
                    "configured addition '{}' is a protected description file; adding or replacing descriptions is not supported",
                    name
                ),
            });
        }
        let key = name.to_ascii_lowercase();
        if additions.insert(key, addition).is_some() {
            return Err(format!("addition '{}' is configured more than once", name).into());
        }
    }
    let mut added = Vec::new();
    let mut member_count = 0usize;
    let mut expanded_size = 0u64;

    if !comment.is_empty() {
        zip.set_raw_comment(comment.to_vec().into_boxed_slice())?;
    }

    while let Some(entry) = archive.next_entry()? {
        member_count += 1;
        if member_count > options.max_members {
            return Ok(Repacked::NeedsReview {
                reason: format!("archive contains more than {} members", options.max_members),
            });
        }
        if entry.original_size() > options.max_member_size {
            return Ok(Repacked::NeedsReview {
                reason: format!("member '{}' expands beyond {} bytes", entry.name(), options.max_member_size),
            });
        }
        expanded_size = match expanded_size.checked_add(entry.original_size()) {
            Some(size) if size <= options.max_expanded_size => size,
            _ => {
                return Ok(Repacked::NeedsReview {
                    reason: format!("archive expands beyond {} bytes", options.max_expanded_size),
                });
            }
        };
        if entry.original_size() > 0
            && (entry.compressed_size() == 0 || entry.original_size() > entry.compressed_size().saturating_mul(options.max_compression_ratio))
        {
            return Ok(Repacked::NeedsReview {
                reason: format!("member '{}' exceeds compression ratio {}:1", entry.name(), options.max_compression_ratio),
            });
        }
        if entry.is_encrypted() {
            return Ok(Repacked::NeedsReview {
                reason: format!("member '{}' needs a password", entry.name()),
            });
        }
        let Some(name) = member_name(entry.name()) else {
            continue;
        };
        // Losing a member to a gap in a decoder would be worse than leaving the archive alone.
        let content = archive.read(&entry)?;
        if content.len() as u64 > options.max_member_size || content.len() as u64 > entry.original_size() {
            return Ok(Repacked::NeedsReview {
                reason: format!("member '{}' expanded beyond its declared size", entry.name()),
            });
        }
        if options.remove_advertisements && fingerprints.is_match(&name, &content) {
            removed.push(name);
            continue;
        }
        if options.remove_advertisements {
            let matches = fingerprints.match_text_member(&name, &content);
            if matches.len() > 1 || matches.iter().any(|m| m.action == RuleAction::Review) {
                return Ok(Repacked::NeedsReview {
                    reason: format!(
                        "text member {:?} needs review (ambiguous or review rule): {}",
                        name,
                        matches
                            .iter()
                            .map(|m| format!("{} sha256={}", m.rule_id, m.sha256))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
            if let Some(matched) = matches.into_iter().next() {
                let remove = matched.action == RuleAction::AutoClean;
                text_members.push(TextMemberFinding { name: name.clone(), matched });
                if remove {
                    removed.push(name);
                    continue;
                }
            }
        }
        let description_result = if options.remove_advertisements {
            fingerprints.clean_description(&name, &content, MAX_DESCRIPTION_CLEAN_PASSES)
        } else {
            super::description_cleaner::DescriptionCleanResult {
                content,
                changes: Vec::new(),
                needs_review: false,
            }
        };
        if description_result.needs_review {
            return Ok(Repacked::NeedsReview {
                reason: format!("description '{}' needs review", name),
            });
        }
        if !description_result.changes.is_empty() {
            cleaned_descriptions.push(CleanedDescription {
                name: name.clone(),
                changes: description_result.changes,
            });
        }
        if !written.insert(name.clone()) {
            return Err(format!("{} holds '{}' twice", path.display(), name).into());
        }
        if additions.contains_key(&name.to_ascii_lowercase()) {
            return Ok(Repacked::NeedsReview {
                reason: format!("configured addition '{}' already exists", name),
            });
        }
        let content = description_result.content.as_slice();
        let mut file_options = zip::write::FileOptions::<ExtendedFileOptions>::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(options.compression_level));
        if let Some(time) = entry.modified_time()
            && let Ok(time) = zip::DateTime::from_date_and_time(time.year(), time.month(), time.day(), time.hour(), time.minute(), time.second())
        {
            file_options = file_options.last_modified_time(time);
        }
        zip.start_file(name, file_options)?;
        zip.write_all(content)?;
    }
    for addition in additions.into_values() {
        let name = member_name(&addition.name).expect("addition names were validated");
        zip.start_file(
            &name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(options.compression_level)),
        )?;
        zip.write_all(&addition.content)?;
        added.push(name);
    }
    zip.finish()?.flush()?;

    if !options.recompress
        && !renamed
        && removed.is_empty()
        && added.is_empty()
        && cleaned_descriptions.is_empty()
        && !comment_changed
        && format == ArchiveFormat::Zip
    {
        return Ok(if text_members.is_empty() {
            Repacked::Unchanged
        } else {
            Repacked::Reported { text_members }
        });
    }

    let after = temporary.as_file().metadata()?.len();
    let result = Repacked::Converted {
        name: file_name(&target),
        removed,
        added,
        cleaned_descriptions,
        text_members,
        archive_comment_changed: comment_changed,
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
    use sha2::{Digest, Sha256};
    use std::io::Read;

    fn test_archive(path: &Path, comment: &[u8], members: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        if !comment.is_empty() {
            zip.set_raw_comment(comment.to_vec().into_boxed_slice()).unwrap();
        }
        for (name, content) in members {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
    }

    fn rules(toml: &str) -> FingerprintData {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), toml).unwrap();
        FingerprintData::load(&file.path()).unwrap()
    }

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

    #[test]
    fn advertisement_switch_controls_members_and_footers_but_preserves_comments() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        let exact = b"known advertisement";
        let comment = b"known advertising comment";
        let fingerprints = rules(&format!(
            "[[fingerprint]]\nname = 'known'\nsha256 = '{:x}'\nfile_size = {}\n\n[[fingerprint]]\npattern = '^VARIANT[.]AD$'\nkeywords = ['visit our board']\n\n[[description_rule]]\nid = 'footer'\nlines = ['^visit our board$']\naction = 'auto_clean'\n",
            Sha256::digest(exact),
            exact.len()
        ));
        for enabled in [false, true] {
            test_archive(
                &path,
                comment,
                &[
                    ("KNOWN.AD", exact),
                    ("VARIANT.AD", b"visit our board today"),
                    ("PROGRAM.TXT", b"payload"),
                    ("FILE_ID.DIZ", b"Product\nvisit our board\n"),
                ],
            );
            let result = repack_file(
                &path,
                &fingerprints,
                &RepackOptions {
                    remove_advertisements: enabled,
                    ..Default::default()
                },
            )
            .unwrap();
            let Repacked::Converted {
                removed,
                cleaned_descriptions,
                archive_comment_changed,
                ..
            } = result
            else {
                panic!("expected repacked archive");
            };
            assert_eq!(if enabled { 2 } else { 0 }, removed.len());
            assert_eq!(usize::from(enabled), cleaned_descriptions.len());
            assert!(!archive_comment_changed);
            let mut archive = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
            assert_eq!(comment, archive.comment());
            let mut description = String::new();
            archive.by_name("FILE_ID.DIZ").unwrap().read_to_string(&mut description).unwrap();
            assert_eq!(if enabled { "Product\n" } else { "Product\nvisit our board\n" }, description);
            assert_eq!(!enabled, archive.by_name("KNOWN.AD").is_ok());
            assert_eq!(!enabled, archive.by_name("VARIANT.AD").is_ok());
            assert!(archive.by_name("PROGRAM.TXT").is_ok());
        }
    }

    #[test]
    fn archive_comment_mode_defaults_and_serialization() {
        #[derive(Serialize, Deserialize, Default)]
        struct Config {
            #[serde(default)]
            mode: ArchiveCommentMode,
        }
        assert_eq!(ArchiveCommentMode::Preserve, ArchiveCommentMode::default());
        assert_eq!(ArchiveCommentMode::Preserve, RepackOptions::default().archive_comment_mode);
        assert!(RepackOptions::default().replacement_archive_comment.is_empty());
        assert_eq!(ArchiveCommentMode::Preserve, toml::from_str::<Config>("").unwrap().mode);
        for (mode, name) in [
            (ArchiveCommentMode::Preserve, "preserve"),
            (ArchiveCommentMode::Remove, "remove"),
            (ArchiveCommentMode::Replace, "replace"),
        ] {
            let encoded = toml::to_string(&Config { mode }).unwrap();
            assert_eq!(format!("mode = \"{name}\"\n"), encoded);
            assert_eq!(mode, toml::from_str::<Config>(&encoded).unwrap().mode);
        }
        assert!(toml::from_str::<Config>("mode = 'invalid'").is_err());
    }

    #[test]
    fn archive_comment_modes_are_independent_of_advertisement_removal_and_recompression() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        let raw = b"Release\0\xff\x80\r\n@X0F".as_slice();
        for archive_comment_mode in [ArchiveCommentMode::Preserve, ArchiveCommentMode::Remove, ArchiveCommentMode::Replace] {
            for remove_advertisements in [false, true] {
                for recompress in [false, true] {
                    for original in [b"".as_slice(), raw] {
                        for replacement in [b"".as_slice(), raw, b"New\0\xfe\r\n".as_slice()] {
                            test_archive(&path, original, &[("PROGRAM.TXT", b"payload")]);
                            let before = fs::read(&path).unwrap();
                            let expected = match archive_comment_mode {
                                ArchiveCommentMode::Preserve => original,
                                ArchiveCommentMode::Remove => b"".as_slice(),
                                ArchiveCommentMode::Replace => replacement,
                            };
                            let result = repack_file(
                                &path,
                                &FingerprintData::default(),
                                &RepackOptions {
                                    archive_comment_mode,
                                    replacement_archive_comment: replacement.to_vec(),
                                    remove_advertisements,
                                    recompress,
                                    ..Default::default()
                                },
                            )
                            .unwrap();
                            let changed = expected != original;
                            if recompress || changed {
                                let Repacked::Converted {
                                    archive_comment_changed,
                                    removed,
                                    added,
                                    cleaned_descriptions,
                                    ..
                                } = result
                                else {
                                    panic!("expected conversion for {archive_comment_mode:?}, recompress={recompress}, changed={changed}");
                                };
                                assert_eq!(changed, archive_comment_changed);
                                assert!(removed.is_empty() && added.is_empty() && cleaned_descriptions.is_empty());
                            } else {
                                assert!(matches!(result, Repacked::Unchanged));
                                assert_eq!(before, fs::read(&path).unwrap());
                            }
                            let mut archive = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
                            assert_eq!(expected, archive.comment());
                            assert_eq!(1, archive.len());
                            let mut payload = Vec::new();
                            archive.by_name("PROGRAM.TXT").unwrap().read_to_end(&mut payload).unwrap();
                            assert_eq!(b"payload", payload.as_slice());
                            assert_eq!(1, fs::read_dir(directory.path()).unwrap().count());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn archive_comment_modes_dry_run_leave_source_and_directory_untouched() {
        for name in ["upload.zip", "UPLOAD.ZIP"] {
            for archive_comment_mode in [ArchiveCommentMode::Preserve, ArchiveCommentMode::Remove, ArchiveCommentMode::Replace] {
                for remove_advertisements in [false, true] {
                    let directory = tempfile::tempdir().unwrap();
                    let path = directory.path().join(name);
                    test_archive(&path, b"original\xff", &[("PROGRAM.TXT", b"payload")]);
                    let before = fs::read(&path).unwrap();
                    let result = repack_file(
                        &path,
                        &FingerprintData::default(),
                        &RepackOptions {
                            archive_comment_mode,
                            replacement_archive_comment: b"replacement\xfe".to_vec(),
                            remove_advertisements,
                            recompress: false,
                            dry_run: true,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    if name == "upload.zip" && archive_comment_mode == ArchiveCommentMode::Preserve {
                        assert!(matches!(result, Repacked::Unchanged));
                    } else {
                        let Repacked::Converted { archive_comment_changed, .. } = result else {
                            panic!("expected dry-run conversion");
                        };
                        assert_eq!(archive_comment_mode != ArchiveCommentMode::Preserve, archive_comment_changed);
                    }
                    assert_eq!(before, fs::read(&path).unwrap());
                    assert_eq!(1, fs::read_dir(directory.path()).unwrap().count());
                    if name != "upload.zip" {
                        assert!(!directory.path().join("upload.zip").exists());
                    }
                }
            }
        }
    }

    #[test]
    fn oversized_replacement_is_rejected_only_in_replace_mode_without_touching_source() {
        for name in ["upload.zip", "UPLOAD.ZIP"] {
            for archive_comment_mode in [ArchiveCommentMode::Preserve, ArchiveCommentMode::Remove, ArchiveCommentMode::Replace] {
                for remove_advertisements in [false, true] {
                    for dry_run in [false, true] {
                        let directory = tempfile::tempdir().unwrap();
                        let path = directory.path().join(name);
                        test_archive(&path, b"original", &[("PROGRAM.TXT", b"payload")]);
                        let before = fs::read(&path).unwrap();
                        let result = repack_file(
                            &path,
                            &FingerprintData::default(),
                            &RepackOptions {
                                archive_comment_mode,
                                replacement_archive_comment: vec![b'X'; usize::from(u16::MAX) + 1],
                                remove_advertisements,
                                recompress: false,
                                dry_run,
                                ..Default::default()
                            },
                        );
                        if archive_comment_mode == ArchiveCommentMode::Replace {
                            assert!(result.is_err(), "overlength ZIP comment must be rejected");
                        } else {
                            assert!(result.is_ok(), "unused replacement must not be validated");
                        }
                        if dry_run || archive_comment_mode == ArchiveCommentMode::Replace {
                            assert_eq!(before, fs::read(&path).unwrap());
                            if name != "upload.zip" {
                                assert!(!directory.path().join("upload.zip").exists());
                            }
                        }
                        assert_eq!(1, fs::read_dir(directory.path()).unwrap().count());
                    }
                }
            }
        }
    }

    #[test]
    fn maximum_length_raw_zip_comment_can_be_replaced_and_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        test_archive(&path, b"original", &[("PROGRAM.TXT", b"payload")]);
        let comment = vec![0xff; usize::from(u16::MAX)];
        for archive_comment_mode in [ArchiveCommentMode::Replace, ArchiveCommentMode::Preserve] {
            let result = repack_file(
                &path,
                &FingerprintData::default(),
                &RepackOptions {
                    archive_comment_mode,
                    replacement_archive_comment: comment.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
            let Repacked::Converted { archive_comment_changed, .. } = result else {
                panic!("expected conversion");
            };
            assert_eq!(archive_comment_mode == ArchiveCommentMode::Replace, archive_comment_changed);
            let archive = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
            assert_eq!(comment.as_slice(), archive.comment());
        }
    }

    #[test]
    fn footer_cleanup_is_internally_limited_to_eight_passes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        let fingerprints = rules("[[description_rule]]\nid = 'footer'\nlines = ['^advertisement$']\naction = 'auto_clean'\n");
        for count in [8, 9] {
            let description = format!("Product\n{}", "advertisement\n".repeat(count));
            test_archive(&path, &[], &[("FILE_ID.DIZ", description.as_bytes())]);
            let before = fs::read(&path).unwrap();
            let result = repack_file(&path, &fingerprints, &RepackOptions::default()).unwrap();
            if count == 8 {
                let Repacked::Converted { cleaned_descriptions, .. } = result else {
                    panic!("eight passes must succeed");
                };
                assert_eq!(8, cleaned_descriptions[0].changes.len());
            } else {
                assert!(matches!(result, Repacked::NeedsReview { .. }));
                assert_eq!(before, fs::read(&path).unwrap());
            }
        }
    }

    #[test]
    fn test_repack_cleans_description_and_removes_comment_without_touching_payload() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("UPLOAD.ZIP");
        let comment = b"The BBS Archives";
        test_archive(&path, comment, &[("FILE_ID.DIZ", b"Product\r\nLiQUiD WHQ\r\n"), ("PROGRAM.EXE", b"payload")]);
        let fingerprints = rules("[[description_rule]]\nid = \"liquid\"\nlines = [\"^liquid whq$\"]\naction = \"auto_clean\"\n");

        let result = repack_file(
            &path,
            &fingerprints,
            &RepackOptions {
                archive_comment_mode: ArchiveCommentMode::Remove,
                ..Default::default()
            },
        )
        .unwrap();
        let Repacked::Converted {
            name,
            cleaned_descriptions,
            archive_comment_changed,
            ..
        } = result
        else {
            panic!("archive was not converted");
        };
        assert_eq!("upload.zip", name);
        assert_eq!("liquid", cleaned_descriptions[0].changes[0].rule_id);
        assert!(archive_comment_changed);

        let mut archive = zip::ZipArchive::new(fs::File::open(directory.path().join(name)).unwrap()).unwrap();
        assert!(archive.comment().is_empty());
        let mut description = Vec::new();
        archive.by_name("FILE_ID.DIZ").unwrap().read_to_end(&mut description).unwrap();
        assert_eq!(b"Product\r\n", description.as_slice());
        let mut payload = Vec::new();
        archive.by_name("PROGRAM.EXE").unwrap().read_to_end(&mut payload).unwrap();
        assert_eq!(b"payload", payload.as_slice());
    }

    #[test]
    fn test_recompression_is_applied_to_an_existing_lowercase_zip() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        zip.start_file(
            "PAYLOAD.TXT",
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        let payload = vec![b'A'; 4096];
        zip.write_all(&payload).unwrap();
        zip.finish().unwrap();
        let before = fs::read(&path).unwrap();
        let rules = FingerprintData::default();
        let result = repack_file(
            &path,
            &rules,
            &RepackOptions {
                recompress: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches!(result, Repacked::Unchanged));
        assert_eq!(before, fs::read(&path).unwrap());
        let result = repack_file(
            &path,
            &rules,
            &RepackOptions {
                compression_level: 9,
                dry_run: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches!(result, Repacked::Converted { .. }));
        assert_eq!(before, fs::read(&path).unwrap());
        repack_file(
            &path,
            &rules,
            &RepackOptions {
                compression_level: 9,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(fs::metadata(&path).unwrap().len() < before.len() as u64);
        let mut archive = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
        let mut member = archive.by_name("PAYLOAD.TXT").unwrap();
        assert_eq!(zip::CompressionMethod::Deflated, member.compression());
        let mut actual = Vec::new();
        member.read_to_end(&mut actual).unwrap();
        assert_eq!(payload, actual);
    }

    #[test]
    fn test_repack_sends_an_oversized_member_to_review_without_replacing_archive() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        test_archive(&path, &[], &[("LARGE.DAT", b"too large")]);
        let before = fs::read(&path).unwrap();
        let options = RepackOptions {
            max_member_size: 4,
            ..Default::default()
        };
        let result = repack_file(&path, &FingerprintData::default(), &options).unwrap();
        assert!(matches!(result, Repacked::NeedsReview { .. }));
        assert_eq!(before, fs::read(&path).unwrap());
    }

    #[test]
    fn test_description_cleaning_leaves_empty_unrelated_members_alone() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        test_archive(
            &path,
            &[],
            &[("FILE_ID.DIZ", b"Product description"), ("EMPTY.DAT", b""), ("SETTINGS.CFG", b" \r\n")],
        );
        let fingerprints = rules("[[description_rule]]\nid = 'footer'\nlines = ['^advertisement$']\naction = 'auto_clean'\n");
        assert!(matches!(
            repack_file(&path, &fingerprints, &RepackOptions::default()).unwrap(),
            Repacked::Converted { .. }
        ));
        let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        assert_eq!(3, archive.len());
        assert_eq!(0, archive.by_name("EMPTY.DAT").unwrap().size());
        let mut content = Vec::new();
        archive.by_name("SETTINGS.CFG").unwrap().read_to_end(&mut content).unwrap();
        assert_eq!(b" \r\n", content.as_slice());
    }

    #[test]
    fn test_additions_cannot_insert_or_replace_archive_descriptions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        for existing_description in [false, true] {
            let mut members: Vec<(&str, &[u8])> = vec![("PROGRAM.EXE", b"payload")];
            if existing_description {
                members.push(("FILE_ID.DIZ", b"Original description\r\n"));
            }
            test_archive(&path, b"original comment", &members);
            let original = fs::read(&path).unwrap();
            for name in [
                "FILE_ID.DIZ",
                "file_id.diz",
                "subdir/FiLe_Id.DiZ",
                "subdir\\FILE_ID.DIZ",
                "DESC.SDI",
                "FILE_ID.ANS",
                "FILE_ID.PCB",
            ] {
                let options = RepackOptions {
                    additions: vec![ArchiveAddition {
                        name: name.to_string(),
                        content: b"Board advertisement".to_vec(),
                    }],
                    ..Default::default()
                };
                let result = repack_file(&path, &FingerprintData::default(), &options).unwrap();
                let Repacked::NeedsReview { reason } = result else {
                    panic!("description addition {name} was accepted");
                };
                assert!(reason.contains("protected description file"));
                assert_eq!(original, fs::read(&path).unwrap());
            }
        }
    }

    #[test]
    fn test_repack_adds_board_advertisement_and_comment_but_preserves_description() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upload.zip");
        test_archive(&path, b"old comment", &[("FILE_ID.DIZ", b"old description"), ("PROGRAM.EXE", b"payload")]);
        let options = RepackOptions {
            additions: vec![ArchiveAddition {
                name: "ICYBOARD.TXT".to_string(),
                content: b"Visit this board".to_vec(),
            }],
            archive_comment_mode: ArchiveCommentMode::Replace,
            replacement_archive_comment: b"IcyBoard".to_vec(),
            ..Default::default()
        };

        let result = repack_file(&path, &FingerprintData::default(), &options).unwrap();
        let Repacked::Converted { name, added, .. } = result else {
            panic!("archive was not converted");
        };
        assert_eq!(vec!["ICYBOARD.TXT"], added);

        let mut archive = zip::ZipArchive::new(fs::File::open(directory.path().join(name)).unwrap()).unwrap();
        assert_eq!(b"IcyBoard", archive.comment());
        let mut description = Vec::new();
        archive.by_name("FILE_ID.DIZ").unwrap().read_to_end(&mut description).unwrap();
        assert_eq!(b"old description", description.as_slice());
        let mut advertisement = Vec::new();
        archive.by_name("ICYBOARD.TXT").unwrap().read_to_end(&mut advertisement).unwrap();
        assert_eq!(b"Visit this board", advertisement.as_slice());
    }
}
