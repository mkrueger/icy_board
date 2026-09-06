//! Offline, opt-in rule generation from the historical BBS advertisement corpus.
//! Never runs extracted programs or changes source archives / board configuration.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
};

use codepages::{normalize_file, tables::get_utf8};
use dizbase::file_base_scanner::{
    bbstro_fingerprint::{Fingerprint, FingerprintData},
    description_cleaner::{DescriptionBlockRule, DescriptionCleaner, RuleAction},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unarc_rs::unified::{ArchiveFormat, UnifiedArchive};
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const MEMBER_LIMIT: usize = 16 * 1024 * 1024;
const TOTAL_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn corpus_archive_format(data: &[u8]) -> Option<ArchiveFormat> {
    // The general detector also uses weak DOS HYP / pre-ustar TAR heuristics
    // that misidentify text, JPEGs and executables in this historical corpus.
    // Restrict this importer to signature-bearing container formats.
    ArchiveFormat::detect_from_bytes(data).filter(|format| {
        matches!(
            format,
            ArchiveFormat::Zip | ArchiveFormat::SevenZ | ArchiveFormat::Rar | ArchiveFormat::Lha | ArchiveFormat::Arj | ArchiveFormat::Ace
        )
    })
}

#[derive(Serialize, Deserialize)]
struct Sample {
    origin: String,
    sha256: String,
    size: usize,
    kind: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Manifest {
    source: String,
    archives: usize,
    expanded_bytes: u64,
    sources: BTreeMap<String, String>,
    samples: Vec<Sample>,
    errors: Vec<String>,
}

struct Extractor {
    work: PathBuf,
    manifest: Manifest,
}

impl Extractor {
    fn store(&mut self, origin: &str, data: &[u8], kind: &str) -> Result<PathBuf> {
        self.manifest.expanded_bytes += data.len() as u64;
        if self.manifest.expanded_bytes > TOTAL_LIMIT || self.manifest.samples.len() >= 100_000 {
            return Err("global corpus extraction limit exceeded".into());
        }
        let sha256 = digest(data);
        let path = self.work.join("blobs").join(&sha256);
        // Archive names are only metadata: no traversal, collisions, symlinks,
        // executable permission bits or overwriting files through member paths.
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => file.write_all(data)?,
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err.into()),
        }
        self.manifest.samples.push(Sample {
            origin: origin.into(),
            sha256,
            size: data.len(),
            kind: kind.into(),
        });
        Ok(path)
    }

    fn archive(&mut self, path: &Path, origin: &str, format: ArchiveFormat, depth: usize) -> Result<()> {
        if depth > 4 {
            return Err("nested archive depth exceeds 4".into());
        }
        self.manifest.archives += 1;
        let mut archive = UnifiedArchive::open_with_format(BufReader::new(File::open(path)?), format)?;
        if matches!(format, ArchiveFormat::Zip) {
            let zip = zip::ZipArchive::new(BufReader::new(File::open(path)?))?;
            if !zip.comment().is_empty() {
                self.store(&format!("{origin}::ZIP-COMMENT"), zip.comment(), "comment")?;
            }
        }
        let mut total = 0u64;
        let mut count = 0;
        while let Some(entry) = archive.next_entry()? {
            count += 1;
            if count > 10_000 || entry.original_size() > MEMBER_LIMIT as u64 || entry.is_encrypted() {
                return Err("member count / size limit or encryption".into());
            }
            total += entry.original_size();
            if total > 256 * 1024 * 1024 || self.manifest.expanded_bytes + total > TOTAL_LIMIT {
                return Err("expanded archive size limit exceeded".into());
            }
            // read_to bounds streamed output; the archive library can still
            // allocate internally for solid / legacy formats. Use offline only.
            let mut sink = LimitedBytes(Vec::new());
            archive.read_to(&entry, &mut sink)?;
            if sink.0.len() as u64 != entry.original_size() {
                return Err(format!("declared size mismatch: {}", entry.file_name()).into());
            }
            if sink.0.is_empty() && entry.file_name().ends_with(['/', '\\']) {
                continue;
            }
            let member_origin = format!("{origin}::{}", entry.file_name());
            if let Some(nested) = corpus_archive_format(&sink.0) {
                let blob = self.store(&member_origin, &sink.0, "container")?;
                if let Err(err) = self.archive(&blob, &member_origin, nested, depth + 1) {
                    self.manifest.errors.push(format!("{member_origin}: {err}"));
                }
            } else {
                self.store(&member_origin, &sink.0, "member")?;
            }
        }
        Ok(())
    }
}

struct LimitedBytes(Vec<u8>);
impl Write for LimitedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MEMBER_LIMIT {
            return Err(io::Error::other("expanded member exceeds limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    File::open(path)?.take((MEMBER_LIMIT + 1) as u64).read_to_end(&mut data)?;
    if data.len() > MEMBER_LIMIT {
        return Err(format!("source exceeds size limit: {}", path.display()).into());
    }
    Ok(data)
}

fn extract(source: &Path, work: &Path) -> Result<()> {
    let source = source.canonicalize()?;
    // A new work directory is required; never clobber a previous extraction.
    fs::create_dir(work)?;
    fs::create_dir(work.join("blobs"))?;
    let mut extractor = Extractor {
        work: work.into(),
        manifest: Manifest {
            source: source.display().to_string(),
            ..Default::default()
        },
    };
    for entry in WalkDir::new(&source).follow_links(false).sort_by_file_name() {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let origin = entry.path().strip_prefix(&source)?.to_string_lossy().into_owned();
        let data = read_bounded(entry.path())?;
        extractor.manifest.sources.insert(origin.clone(), digest(&data));
        // Historical adverts deliberately use misleading .ZIP/.LHA/.ICE names.
        // Only actual file signatures justify archive decompression.
        if let Some(format) = corpus_archive_format(&data) {
            if let Err(err) = extractor.archive(entry.path(), &origin, format, 0) {
                extractor.manifest.errors.push(format!("{origin}: {err}"));
            }
        } else {
            extractor.store(&origin, &data, "loose")?;
        }
    }
    for (origin, hash) in &extractor.manifest.sources {
        if digest(&read_bounded(&source.join(origin))?) != *hash {
            return Err(format!("source changed during extraction: {origin}").into());
        }
    }
    fs::write(work.join("manifest.toml"), toml::to_string_pretty(&extractor.manifest)?)?;
    println!(
        "{} source files, {} archives, {} samples, {} extraction errors; sources unchanged",
        extractor.manifest.sources.len(),
        extractor.manifest.archives,
        extractor.manifest.samples.len(),
        extractor.manifest.errors.len()
    );
    for error in &extractor.manifest.errors {
        eprintln!("{error}");
    }
    Ok(())
}

fn member_selection(sample: &Sample) -> bool {
    let origin = sample.origin.to_lowercase();
    let name = origin.rsplit("::").next().unwrap().rsplit(['/', '\\']).next().unwrap();
    // Some boards advertise through filenames of 1/6-byte placeholder files.
    // Hashing those contents would match unrelated placeholders everywhere.
    if sample.size < 32 || matches!(sample.kind.as_str(), "comment" | "container") {
        return false;
    }
    if name.starts_with("readme") || matches!(name, "file_id.diz" | "desc.sdi" | "scene.org.txt") {
        return false;
    }
    if sample.kind == "loose" {
        // Only board adverts, not root collection docs, group ads or website ads.
        return (origin.starts_with("bac-v10/") && origin.matches('/').count() >= 2)
            || (origin.starts_with("bbs_ads-2020-10-04/") && origin.matches('/').count() >= 3 && !origin.contains("/_groups/") && !origin.contains("/_misc/"));
    }
    // Packed collections also contain general-purpose support files and docs.
    // Keep a conservative extension allowlist for the actual intros / adverts.
    matches!(
        name.rsplit('.').next().unwrap(),
        "exe" | "com" | "smc" | "swc" | "sfc" | "bbs" | "ad" | "ans" | "asc"
    )
}

fn literal_lines(data: &[u8], ansi: &Regex, colors: &Regex) -> Option<Vec<String>> {
    if data.len() > 64 * 1024 || data.iter().any(|b| *b < 32 && !matches!(*b, 9 | 10 | 13 | 26 | 27)) {
        return None;
    }
    // Same byte decoding as the production matcher, NOT guessed Amiga Unicode.
    let lines: Vec<_> = data
        .split(|b| *b == b'\n')
        .map(|line| {
            let normalized = normalize_file(line);
            if normalized.starts_with(&[0xef, 0xbb, 0xbf]) && std::str::from_utf8(&normalized[3..]).is_err() {
                return None;
            }
            let decoded = get_utf8(&normalized);
            Some(
                colors
                    .replace_all(&ansi.replace_all(&decoded, ""), "")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase(),
            )
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect();
    let letters = lines.iter().flat_map(|line| line.chars()).filter(|c| c.is_alphabetic()).count();
    (lines.len() >= 2 && lines.len() <= 200 && letters >= 24).then_some(lines)
}

#[derive(Serialize, Default)]
struct Catalog {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fingerprint: Vec<Fingerprint>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    description_rule: Vec<DescriptionBlockRule>,
}

fn generate(work: &Path, output: &Path) -> Result<()> {
    let manifest: Manifest = toml::from_str(&fs::read_to_string(work.join("manifest.toml"))?)?;
    fs::create_dir(output)?;
    let mut files = Catalog::default();
    let mut descriptions = Catalog::default();
    let mut selected = BTreeMap::<String, Vec<&Sample>>::new();
    // Raw ZIP comments remain extraction evidence, never cleanup rules.
    let mut comment_samples = BTreeSet::new();
    let mut blocks = BTreeMap::<Vec<String>, Vec<&Sample>>::new();
    let ansi = Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]")?;
    let colors = Regex::new(r"(?i)@X[0-9a-f]{2}")?;
    for sample in &manifest.samples {
        if member_selection(sample) {
            selected.entry(sample.sha256.clone()).or_default().push(sample);
        }
        if sample.kind == "comment" {
            comment_samples.insert(&sample.sha256);
        }
    }
    let mut provenance = String::from("sha256\tsize\tselection\torigin (escaped)\n");
    for sample in &manifest.samples {
        let selection = if selected.contains_key(&sample.sha256) { "member-rule" } else { &sample.kind };
        provenance.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            sample.sha256,
            sample.size,
            selection,
            sample.origin.escape_default()
        ));
    }
    for (hash, samples) in &selected {
        let data = read_bounded(&work.join("blobs").join(hash))?;
        if digest(&data) != *hash {
            return Err("modified extraction blob".into());
        }
        files.fingerprint.push(Fingerprint::new(samples[0].origin.clone(), &data));
        // Infer blocks only from dedicated loose adverts, never FILE_ID.DIZ
        // describing an intro release, or arbitrary executable companions.
        if samples.iter().any(|s| s.kind == "loose") {
            if let Some(lines) = literal_lines(&data, &ansi, &colors) {
                blocks.entry(lines).or_default().extend(samples);
            }
        }
    }
    for (lines, samples) in &blocks {
        descriptions.description_rule.push(DescriptionBlockRule {
            id: format!("corpus-block-{}", digest(lines.join("\n").as_bytes())),
            literal_lines: lines.clone(),
            action: RuleAction::ReportOnly,
            ..Default::default()
        });
        provenance.push_str(&format!(
            "# description {}: {}\n",
            digest(lines.join("\n").as_bytes()),
            samples[0].origin.escape_default()
        ));
    }
    for (name, catalog) in [("upload_ad_files.toml", &files), ("upload_ad_descriptions.toml", &descriptions)] {
        fs::write(
            output.join(name),
            format!(
                "# Generated corpus TEST catalog; see README.md before enabling.\n{}",
                toml::to_string_pretty(catalog)?
            ),
        )?;
    }
    fs::write(output.join("provenance.tsv"), provenance)?;
    let rules = FingerprintData::load_split(&output.join("upload_ad_files.toml"), &output.join("upload_ad_descriptions.toml"))?;
    let mut member_positives = 0;
    for sample in &manifest.samples {
        if selected.contains_key(&sample.sha256) {
            let data = read_bounded(&work.join("blobs").join(&sample.sha256))?;
            assert!(rules.is_match("renamed-unrelated-filename.bin", &data), "{}", sample.origin);
            member_positives += 1;
        }
    }
    let control = b"Unrelated software description.\r\nVersion 1.0 - documentation and source included.\r\n";
    assert!(!rules.is_match("acid_slam.txt", control));
    assert!(!rules.is_match("acid_slam2.txt", control));
    assert_eq!(rules.clean_description("FILE_ID.DIZ", control, 8).content, control);
    assert!(rules.clean_description("FILE_ID.DIZ", control, 8).changes.is_empty());
    let mut ambiguous = BTreeSet::new();
    for (rule, (_, samples)) in descriptions.description_rule.iter().zip(&blocks) {
        let single = DescriptionCleaner::new(std::slice::from_ref(rule))?;
        for sample in samples {
            let mut data = control.to_vec();
            data.extend_from_slice(&read_bounded(&work.join("blobs").join(&sample.sha256))?);
            let result = single.clean("FILE_ID.DIZ", &data, 8);
            assert_eq!(result.changes.len(), 1, "{}", sample.origin);
            assert_eq!(result.content, data);
        }
        let mut data = control.to_vec();
        data.extend_from_slice(&read_bounded(&work.join("blobs").join(&samples[0].sha256))?);
        let result = rules.clean_description("FILE_ID.DIZ", &data, 8);
        assert_eq!(result.content, data);
        if result.needs_review {
            ambiguous.insert(rule.id.clone());
        } else {
            assert_eq!(result.changes.len(), 1);
        }
        assert!(rules.clean_description("README.TXT", &data, 8).changes.is_empty());
    }
    let mut report = format!(
        "# Generated BBS advertisement test catalogs\n\n\
        Source: local user-supplied ads collection. No source files changed; no programs executed.\n\n\
        ## Results\n\n\
        - Source files: {}\n- Archives opened (including nested): {}\n- Extracted/loose samples and comments: {}\n\
        - Unique exact member rules: {}\n- Matching member occurrences, tested under unrelated filenames: {}\n\
        - Unique literal description suffix rules: {}\n- Unique raw ZIP comment samples (provenance only): {}\n\
        - Description candidates with overlapping suffix matches: {}\n- Extraction errors: {}\n\n\
        ## Safety and scope\n\n\
        These are separate **test catalogs**, not installed or merged into the shipped defaults.\n\
        Member rules use SHA-256 plus size, without filename patterns: numbered collection\n\
        variants remain separate if their bytes differ. Identical content is deduplicated.\n\
        Member rules have no report-only mode: enabling that catalog removes matching files.\n\
        Exact matches are evidence of identical corpus bytes, not proof that removal is appropriate\n\
        in every upload (for example, an intentionally uploaded historical intro collection).\n\n\
        Packed-member selection uses EXE/COM/SMC/SWC/SFC/BBS/AD/ANS/ASC; arbitrary support\n\
        files, NFO/TXT/DOC companions and release FILE_ID.DIZ are not automatically selected.\n\
        Files shorter than 32 bytes are excluded (filename-art placeholders are not distinctive content).\n\
        Dedicated loose board-ad collections are included, excluding collection metadata,\n\
        group ads and misc website/card ads. Selection is heuristic, not a manual content audit.\n\
        All samples and exclusions are traceable through provenance.tsv and the work manifest.\n\n\
        Description rules contain complete normalized blocks, suffix only, **report_only**.\n\
        They are tested appended to a synthetic unrelated FILE_ID.DIZ; this does not establish\n\
        that these whole adverts occur as footers in real release descriptions. Overlapping\n\
        blocks require review and never delete bytes. No wildcard phone-number or filename rules.\n\
        Raw ZIP comments are retained only in extraction blobs and provenance; no comment\n\
        rules are generated or validated. Upload archive comments use an independent\n\
        Preserve (default), Remove or Replace mode, not advertisement matching.\n\n\
        bac-v10 readme states that phone numbers were masked and line endings edited.\n\
        Its exact hashes only match those edited versions, not the unedited historical originals.\n\
        Text decoding deliberately follows the engine's UTF-8/CP437 behavior even for Amiga ads;\n\
        no implicit encoding, line-ending or masking variants are invented.\n\n\
        Extraction uses content-addressed non-executable blob files, never archive paths.\n\
        Limits: 16 MiB/member, 256 MiB/archive, 2 GiB total, 10,000 members/archive, depth 4.\n\
        Legacy/solid decompressor internal allocations are not sandboxed.\n\n\
        ## Validation\n\n\
        Both catalogs loaded through FingerprintData::load_split(member_path, description_path). Every selected member occurrence\n\
        matched despite renaming; every description source matched its isolated rule and remained\n\
        byte-identical. The combined description\n\
        catalog was checked for overlap. Synthetic unrelated text/filename controls did not match.\n\
        This is corpus-positive/synthetic-negative validation, not a production false-positive study.\n\n\
        ## Outputs\n\n\
        Only upload_ad_files.toml and upload_ad_descriptions.toml are rule catalogs.\n\
        provenance.tsv and this README accompany them; there is no comment rule file.\n\
        Earlier reports describing comment-rule matches remain historical results, not current validation.\n\n",
        manifest.sources.len(),
        manifest.archives,
        manifest.samples.len(),
        selected.len(),
        member_positives,
        descriptions.description_rule.len(),
        comment_samples.len(),
        ambiguous.len(),
        manifest.errors.len()
    );
    if !manifest.errors.is_empty() {
        report.push_str("## Extraction errors (not silently ignored)\n\n");
        for error in &manifest.errors {
            report.push_str(&format!("- {}\n", error.escape_default()));
        }
    }
    if !ambiguous.is_empty() {
        report.push_str("\n## Overlapping description candidates\n\n");
        for id in ambiguous {
            report.push_str(&format!("- {id}\n"));
        }
    }
    fs::write(output.join("README.md"), report)?;
    println!(
        "Validated two rule catalogs: {} member rules ({} occurrences), {} description blocks. Raw ZIP comment samples (provenance only): {}. Output: {}",
        selected.len(),
        member_positives,
        descriptions.description_rule.len(),
        comment_samples.len(),
        output.display()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.as_slice() {
        [_, mode, source, work] if mode == "extract" => extract(Path::new(source), Path::new(work)),
        [_, mode, work, output] if mode == "generate" => generate(Path::new(work), Path::new(output)),
        _ => Err("usage: ad_corpus_rules extract SOURCE NEW_WORK_DIR | generate WORK_DIR NEW_OUTPUT_DIR".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_preserves_sources_variants_and_untrusted_member_names() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        let ads = source.join("bac-v10/amiga_charset");
        fs::create_dir_all(&ads)?;
        let first = b"Example board advertisement\r\nAll original programs available here\r\n";
        let second = b"Example board advertisement\r\nAnother completely different version\r\n";
        fs::write(ads.join("acid_slam.txt"), first)?;
        fs::write(ads.join("acid_slam2.txt"), second)?;
        fs::write(ads.join("acid_slam3.txt"), first)?;
        let archives = source.join("bbstros/bbs");
        fs::create_dir_all(&archives)?;
        let mut zip = zip::ZipWriter::new(File::create(archives.join("intro.zip"))?);
        zip.start_file("../../../outside.EXE", zip::write::SimpleFileOptions::default())?;
        zip.write_all(b"Not executed, only hashed: first intro version")?;
        zip.set_comment("Example archive comment")?;
        zip.finish()?;
        // The same member name in different archives must preserve both versions.
        let mut zip = zip::ZipWriter::new(File::create(archives.join("intro2.zip"))?);
        zip.start_file("../../../outside.EXE", zip::write::SimpleFileOptions::default())?;
        zip.write_all(b"Not executed, only hashed: second intro version")?;
        zip.finish()?;
        let work = temp.path().join("work");
        extract(&source, &work)?;
        assert!(!temp.path().join("outside.EXE").exists());
        assert_eq!(fs::read(ads.join("acid_slam.txt"))?, first);
        let output = temp.path().join("rules");
        generate(&work, &output)?;
        let catalog: toml::Value = toml::from_str(&fs::read_to_string(output.join("upload_ad_files.toml"))?)?;
        assert_eq!(catalog["fingerprint"].as_array().unwrap().len(), 4);
        let outputs: BTreeSet<_> = fs::read_dir(&output)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<io::Result<_>>()?;
        assert_eq!(
            outputs,
            ["README.md", "provenance.tsv", "upload_ad_files.toml", "upload_ad_descriptions.toml"]
                .into_iter()
                .map(std::ffi::OsString::from)
                .collect()
        );
        let rules = FingerprintData::load_split(&output.join("upload_ad_files.toml"), &output.join("upload_ad_descriptions.toml"))?;
        assert!(rules.is_match("unrelated.bin", first));
        assert_eq!(rules.clean_description("FILE_ID.DIZ", first, 8).content, first);
        let manifest: Manifest = toml::from_str(&fs::read_to_string(work.join("manifest.toml"))?)?;
        let comment = manifest.samples.iter().find(|sample| sample.kind == "comment").unwrap();
        assert_eq!(fs::read(work.join("blobs").join(&comment.sha256))?, b"Example archive comment");
        assert!(fs::read_to_string(output.join("provenance.tsv"))?.contains("ZIP-COMMENT"));
        let report = fs::read_to_string(output.join("README.md"))?;
        assert!(report.contains("Unique raw ZIP comment samples (provenance only): 1"));
        assert!(report.contains("Only upload_ad_files.toml and upload_ad_descriptions.toml are rule catalogs."));
        assert!(extract(&source, &work).is_err());
        assert!(generate(&work, &output).is_err());
        let lines = literal_lines(first, &Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]")?, &Regex::new(r"(?i)@X[0-9a-f]{2}")?).unwrap();
        let cleaner = DescriptionCleaner::new(&[DescriptionBlockRule {
            id: "test-auto-clean".into(),
            literal_lines: lines,
            action: RuleAction::AutoClean,
            ..Default::default()
        }])?;
        let description = [b"Original software description\r\n".as_slice(), first].concat();
        let cleaned = cleaner.clean("FILE_ID.DIZ", &description, 8);
        assert_eq!(cleaned.content, b"Original software description\r\n");
        assert!(!cleaned.needs_review);
        let middle = [description.as_slice(), b"Unrelated final line\r\n"].concat();
        assert!(cleaner.clean("FILE_ID.DIZ", &middle, 8).changes.is_empty());
        Ok(())
    }

    #[test]
    fn archive_detection_does_not_trust_advert_filename_extensions() {
        assert!(corpus_archive_format(b"Call this BBS! Named ISDN.zip or BREAKIT.ICE.").is_none());
        assert_eq!(corpus_archive_format(b"PK\x03\x04........"), Some(ArchiveFormat::Zip));
        assert!(corpus_archive_format(b"HPadvertising, not a HYP archive").is_none());
        let mut executable = vec![0; 512];
        executable[..2].copy_from_slice(b"MZ");
        assert!(corpus_archive_format(&executable).is_none());
    }

    #[test]
    fn bounded_output_and_conservative_selection() {
        let mut sink = LimitedBytes(vec![0; MEMBER_LIMIT]);
        assert!(sink.write_all(b"x").is_err());
        for (name, expected) in [
            ("INTRO.EXE", true),
            ("BOARD.AD", true),
            ("README.TXT", false),
            ("FILE_ID.DIZ", false),
            ("PLAYER.DLL", false),
        ] {
            assert_eq!(
                member_selection(&Sample {
                    origin: format!("bbstros/a.zip::{name}"),
                    sha256: String::new(),
                    size: 100,
                    kind: "member".into()
                }),
                expected
            );
        }
    }
}
