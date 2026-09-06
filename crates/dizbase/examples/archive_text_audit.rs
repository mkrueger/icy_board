//! Offline research only: SOURCE NEW_OUTPUT DEFAULT_MEMBER_RULES CORPUS_MEMBER_RULES.
//! No execution, cleanup, or archive rewriting. Run on a quiescent corpus.
//! Bounds cover delivered bytes (or larger declared sizes for non-ZIP), NOT library allocations/CPU
//! (notably solid archives and eager legacy readers). This is NOT a sandbox.
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Cursor, Read, Seek, Write},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

use codepages::tables::CP437_TO_UNICODE;
use dizbase::file_base_scanner::bbstro_fingerprint::FingerprintData;
use regex::Regex;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use unarc_rs::unified::{ArchiveFormat, UnifiedArchive};
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const TEXT: usize = 128 * 1024;
const NESTED: usize = 16 * 1024 * 1024;
const ENTRIES: usize = 10_000;
const DEPTH: usize = 3; // top-level is depth zero
const EXPANDED: usize = 128 * 1024 * 1024;
const TOTAL: usize = 2 * 1024 * 1024 * 1024;
const STORED: usize = 1024 * 1024 * 1024;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash_file(path: &Path) -> Result<String> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err("not a regular non-symlink file".into());
    }
    let mut reader = BufReader::new(File::open(path)?);
    let mut sha = Sha256::new();
    let mut buf = [0; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        sha.update(&buf[..n]);
    }
    Ok(format!("{:x}", sha.finalize()))
}
fn esc(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            c if c.is_control() => c.escape_default().collect(),
            c => vec![c],
        })
        .collect()
}
fn path_label(path: &Path) -> String {
    // Lossless even for Unix filenames that are not UTF-8.
    path.as_os_str().as_bytes().escape_ascii().to_string()
}
fn new_file(path: &Path) -> io::Result<BufWriter<File>> {
    Ok(BufWriter::new(OpenOptions::new().write(true).create_new(true).open(path)?))
}
fn signature(b: &[u8]) -> Option<ArchiveFormat> {
    // Do not use the general detector: weak HYP/TAR/ARC heuristics match text.
    if b.starts_with(b"PK\x03\x04") || b.starts_with(b"PK\x05\x06") {
        Some(ArchiveFormat::Zip)
    } else if b.starts_with(b"Rar!\x1a\x07\x00") || b.starts_with(b"Rar!\x1a\x07\x01\x00") {
        Some(ArchiveFormat::Rar)
    } else if b.starts_with(b"7z\xbc\xaf\x27\x1c") {
        Some(ArchiveFormat::SevenZ)
    } else if b.starts_with(b"\x60\xea") {
        Some(ArchiveFormat::Arj)
    } else if b.len() >= 7
        && b[6] == b'-'
        && ((&b[2..5] == b"-lh" && (b[5].is_ascii_digit() || b[5] == b'd')) || (&b[2..5] == b"-lz" && b"45s".contains(&b[5])))
    {
        Some(ArchiveFormat::Lha)
    } else {
        None
    }
}

struct TextRules {
    color: Regex,
    digits: Regex,
}
impl TextRules {
    fn new() -> Self {
        Self {
            color: Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]|(?i:@x[0-9a-f]{2})").unwrap(),
            digits: Regex::new(r"\p{Nd}+").unwrap(),
        }
    }
    fn normalize(&self, raw: &[u8]) -> (String, &'static str, usize) {
        // Unlike get_utf8(), validate the BOM payload before constructing a String.
        let (decoded, encoding) = if let Some(payload) = raw.strip_prefix(b"\xef\xbb\xbf").and_then(|b| std::str::from_utf8(b).ok()) {
            (payload.to_owned(), "utf8-bom")
        } else if let Ok(s) = std::str::from_utf8(raw) {
            (s.to_owned(), if raw.is_ascii() { "ascii" } else { "utf8" })
        } else {
            (raw.iter().map(|b| CP437_TO_UNICODE[*b as usize]).collect(), "cp437-inferred")
        };
        let tail = raw.iter().position(|b| *b == 0x1a).map_or(0, |i| raw.len() - i - 1);
        let prefix = decoded.split('\x1a').next().unwrap_or("").replace('\r', "");
        let stripped = self.color.replace_all(&prefix, "");
        let normalized = stripped
            .split('\n')
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase())
            .collect::<Vec<_>>()
            .join("\n");
        (normalized, encoding, tail)
    }
    fn fold(&self, s: &str) -> String {
        self.digits.replace_all(s, "<n>").into_owned()
    }
}
fn binary_magic(raw: &[u8]) -> bool {
    let binary: &[&[u8]] = &[
        b"MZ",
        b"\x7fELF",
        b"\x89PNG",
        b"GIF87a",
        b"GIF89a",
        b"\xff\xd8\xff",
        b"BM",
        b"II*\0",
        b"MM\0*",
        b"%PDF",
        b"RIFF",
        b"OggS",
        b"fLaC",
        b"\0asm",
    ];
    binary.iter().any(|magic| raw.starts_with(magic))
}
fn text_like(raw: &[u8], text: &str) -> bool {
    if raw.len() < 32 || raw.contains(&0) || binary_magic(raw) {
        return false;
    }
    let controls = raw.iter().filter(|&&b| (b < 32 && !b"\r\n\t\x1b\x1a".contains(&b)) || b == 127).count();
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    controls * 100 <= raw.len() * 5 && letters >= 12 && letters * 100 >= text.chars().count() * 15
}
fn kind(name: &str) -> &'static str {
    if matches!(name, "file_id.diz" | "file_id.ans" | "file_id.pcb" | "desc.sdi") {
        "canonical-description"
    } else if name.starts_with("readme") || name.contains("license") || name.contains("licence") || name.ends_with(".doc") || name.ends_with(".nfo") {
        "doc-like"
    } else {
        "candidate-other"
    }
}

// The first 32 bytes select the member bound by signature, never filename.
// Short writes charge every retained byte, including failed/partial reads.
struct Bounded {
    data: Vec<u8>,
    budget: usize,
    reason: &'static str,
    hit: Option<&'static str>,
}
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.data.len() >= 32 && signature(&self.data).is_none() && (binary_magic(&self.data) || self.data.contains(&0)) {
            self.hit = Some("binary-rejected");
            return Err(io::Error::other("binary prefix; not read further"));
        }
        let cap = if self.data.len() < 32 {
            32
        } else if signature(&self.data).is_some() {
            NESTED
        } else {
            TEXT
        };
        let available = cap.min(self.budget).saturating_sub(self.data.len());
        if available == 0 {
            let reason = if self.data.len() >= self.budget { self.reason } else { "member-size-limit" };
            self.hit = Some(reason);
            return Err(io::Error::other(reason));
        }
        let n = available.min(bytes.len());
        self.data.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Audit {
    out: PathBuf,
    db: Connection,
    errors: BufWriter<File>,
    unsupported: BufWriter<File>,
    samples: BufWriter<File>,
    default: FingerprintData,
    corpus: FingerprintData,
    rules: TextRules,
    counts: BTreeMap<String, usize>,
    scanned: usize,
    stored: usize,
    total_limit: usize,
    store_limit: usize,
    stopped: bool,
}
impl Audit {
    fn new(out: &Path, default: FingerprintData, corpus: FingerprintData) -> Result<Self> {
        fs::create_dir(out)?; // Deliberately refuses existing output, including symlinks.
        fs::create_dir(out.join("raw"))?;
        fs::create_dir(out.join("normalized"))?;
        let db = Connection::open(out.join("audit.sqlite"))?;
        db.execute_batch(
            "PRAGMA temp_store=FILE; PRAGMA cache_size=-8192;
            CREATE TABLE sources(id INTEGER PRIMARY KEY, path BLOB, before TEXT);
            CREATE TABLE samples(origin TEXT, raw TEXT, size INTEGER, encoding TEXT, basename TEXT, kind TEXT,
                known_default INTEGER, known_corpus INTEGER, exact_default INTEGER, exact_corpus INTEGER,
                normalized TEXT, digit_fold TEXT, eof_tail INTEGER, ad_view INTEGER, stored INTEGER);",
        )?;
        let mut errors = new_file(&out.join("errors.tsv"))?;
        writeln!(errors, "origin\tstage\treason\tdetail")?;
        let mut unsupported = new_file(&out.join("unsupported.tsv"))?;
        writeln!(unsupported, "source\treason")?;
        let mut samples = new_file(&out.join("samples.tsv"))?;
        writeln!(
            samples,
            "origin\traw_sha256\tsize\tencoding\tbasename\tkind\tknown_default\tknown_corpus\texact_default\texact_corpus\tnormalized_sha256\tdigit_fold_hash\tdos_eof_tail_bytes\tad_view\tstored"
        )?;
        Ok(Self {
            out: out.into(),
            db,
            errors,
            unsupported,
            samples,
            default,
            corpus,
            rules: TextRules::new(),
            counts: BTreeMap::new(),
            scanned: 0,
            stored: 0,
            total_limit: TOTAL,
            store_limit: STORED,
            stopped: false,
        })
    }
    fn count(&mut self, key: &str) {
        *self.counts.entry(key.into()).or_default() += 1;
    }
    fn event(&mut self, origin: &str, stage: &str, reason: &str, detail: impl std::fmt::Display) -> Result<()> {
        self.count(reason);
        writeln!(self.errors, "{}\t{}\t{}\t{}", esc(origin), esc(stage), esc(reason), esc(&detail.to_string()))?;
        Ok(())
    }
    fn failure(&mut self, origin: &str, stage: &str, error: impl std::fmt::Display) -> Result<()> {
        let text = error.to_string();
        let lower = text.to_ascii_lowercase();
        let category = if lower.contains("password") || lower.contains("encrypt") {
            "encrypted"
        } else {
            "errors"
        };
        self.event(origin, stage, category, text)
    }
    fn store_pair(&mut self, raw: &[u8], norm: &str, rh: &str, nh: &str, origin: &str) -> Result<bool> {
        let pairs = [(self.out.join("raw").join(rh), raw), (self.out.join("normalized").join(nh), norm.as_bytes())];
        let additional: usize = pairs.iter().filter(|(p, _)| !p.exists()).map(|(_, b)| b.len()).sum();
        if additional > self.store_limit - self.stored {
            self.stopped = true;
            self.event(origin, "store", "stored-limit", "whole scan stopped; this occurrence has no guaranteed blobs")?;
            return Ok(false);
        }
        for (path, bytes) in pairs {
            if !path.exists() {
                let mut file = new_file(&path)?;
                file.write_all(bytes)?;
                file.flush()?;
                self.stored += bytes.len();
            }
        }
        Ok(true)
    }
    fn member(&mut self, origin: &str, name: &str, data: Vec<u8>, depth: usize) -> Result<()> {
        if let Some(format) = signature(&data) {
            self.count("nested_recognized");
            if self.stopped {
                return Ok(());
            }
            if depth >= DEPTH {
                return self.event(origin, "nested", "depth-limit", DEPTH);
            }
            return self.archive(Cursor::new(data), origin, format, depth + 1);
        }
        if data.len() > TEXT {
            return self.event(origin, "text", "member-size-limit", data.len());
        }
        let (normalized, encoding, tail) = self.rules.normalize(&data);
        if !text_like(&data, &normalized) {
            self.count("text_rejected");
            return Ok(());
        }
        let filename = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let basename = filename.to_lowercase();
        let (kd, kc) = (self.default.is_match(filename, &data), self.corpus.is_match(filename, &data));
        let (ed, ec) = (self.default.is_exact_match(&data), self.corpus.is_exact_match(&data));
        let (rh, nh, dh) = (hash(&data), hash(normalized.as_bytes()), hash(self.rules.fold(&normalized).as_bytes()));
        let view = kd
            || kc
            || [".ad", ".bbs", ".ans", ".asc"].iter().any(|ext| basename.ends_with(ext))
            || ["passed thru", "leeched", "downloaded from", "this file", "board", "sysop"]
                .iter()
                .any(|phrase| normalized.contains(phrase));
        let stored = self.store_pair(&data, &normalized, &rh, &nh, origin)?;
        let kind = kind(&basename);
        self.db
            .prepare_cached("INSERT INTO samples VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)")?
            .execute(params![
                origin,
                rh,
                data.len() as i64,
                encoding,
                basename,
                kind,
                kd,
                kc,
                ed,
                ec,
                nh,
                dh,
                tail as i64,
                view,
                stored
            ])?;
        writeln!(
            self.samples,
            "{}\t{rh}\t{}\t{encoding}\t{}\t{kind}\t{kd}\t{kc}\t{ed}\t{ec}\t{nh}\t{dh}\t{tail}\t{view}\t{stored}",
            esc(origin),
            data.len(),
            esc(&basename)
        )?;
        self.count("text_occurrences");
        Ok(())
    }
    fn selected_read(&mut self, origin: &str, used: &mut usize, charge: usize, read: impl FnOnce(&mut Bounded) -> Result<()>) -> Result<Option<Vec<u8>>> {
        let (global, local) = (self.total_limit - self.scanned, EXPANDED - *used);
        // UnifiedArchive::read_to currently buffers through read() before writing.
        // Reserve/charge the declared size even when our writer rejects a prefix.
        if charge > global.min(local) {
            self.stopped |= global <= local;
            self.event(origin, "read-preflight", if global <= local { "global-limit" } else { "archive-limit" }, charge)?;
            return Ok(None);
        }
        self.count("members_read_attempted");
        let mut sink = Bounded {
            data: Vec::new(),
            budget: global.min(local),
            reason: if global <= local { "global-limit" } else { "archive-limit" },
            hit: None,
        };
        let result = read(&mut sink);
        let charged = sink.data.len().max(charge);
        self.scanned += charged;
        *used += charged;
        if self.scanned >= self.total_limit {
            self.stopped = true;
            self.event(origin, "read", "global-limit", "whole scan stopped; verification still runs")?;
        }
        if let Err(err) = result {
            if let Some(hit) = sink.hit {
                if hit != "global-limit" {
                    self.event(origin, "read", hit, format!("retained {} bytes; {err}", sink.data.len()))?;
                }
            } else {
                self.failure(origin, "read", err)?;
            }
            return Ok(None);
        }
        self.count("members_read");
        Ok(Some(sink.data))
    }
    fn archive<R: Read + Seek>(&mut self, reader: R, origin: &str, format: ArchiveFormat, depth: usize) -> Result<()> {
        self.count(if depth == 0 { "archives_top" } else { "archives_nested" });
        let mut used = 0;
        if format == ArchiveFormat::Zip {
            let mut zip = match zip::ZipArchive::new(reader) {
                Ok(z) => z,
                Err(e) => return self.open_failure(origin, depth, "zip-open", e),
            };
            if zip.len() > ENTRIES {
                self.event(origin, "enumerate", "entry-limit", format!("{} entries; visiting first {ENTRIES}", zip.len()))?;
            }
            for index in 0..zip.len().min(ENTRIES) {
                if self.stopped {
                    break;
                }
                if used >= EXPANDED {
                    self.event(origin, "read", "archive-limit", used)?;
                    break;
                }
                self.count("members_seen");
                let indexed = format!("{origin}![{index}]");
                let mut entry = match zip.by_index(index) {
                    Ok(e) => e,
                    Err(e) => {
                        self.failure(&indexed, "zip-entry", e)?;
                        continue;
                    }
                };
                if entry.is_dir() {
                    self.count("directories");
                    continue;
                }
                let name = entry.name().to_owned();
                let member_origin = format!("{indexed}:{}", esc(&name));
                let size = entry.size();
                let data = self.selected_read(&member_origin, &mut used, 0, |sink| {
                    io::copy(&mut entry, sink)?;
                    Ok(())
                })?;
                if let Some(data) = data {
                    if data.len() as u64 != size {
                        self.event(&member_origin, "read", "errors", "declared size mismatch")?;
                    } else {
                        self.member(&member_origin, &name, data, depth)?;
                    }
                }
            }
        } else {
            let mut archive = match UnifiedArchive::open_with_format(reader, format) {
                Ok(a) => a,
                Err(e) => return self.open_failure(origin, depth, "unified-open", e),
            };
            for index in 0..=ENTRIES {
                if self.stopped {
                    break;
                }
                if used >= EXPANDED {
                    self.event(origin, "read", "archive-limit", used)?;
                    break;
                }
                // A failed header cannot reliably advance a sequential archive; abort that archive only.
                let entry = match archive.next_entry() {
                    Ok(Some(e)) => e,
                    Ok(None) => break,
                    Err(e) => {
                        self.failure(&format!("{origin}![{index}]"), "next-entry-abort-archive", e)?;
                        break;
                    }
                };
                if index == ENTRIES {
                    self.event(origin, "enumerate", "entry-limit", ENTRIES)?;
                    break;
                }
                self.count("members_seen");
                let name = entry.name().to_owned();
                let member_origin = format!("{origin}![{index}]:{}", esc(&name));
                if entry.is_directory() {
                    self.count("directories");
                    continue;
                }
                if entry.is_encrypted() {
                    self.event(&member_origin, "read", "encrypted", "not read")?;
                    continue;
                }
                if entry.original_size() > NESTED as u64 || entry.compressed_size() > NESTED as u64 {
                    self.event(
                        &member_origin,
                        "read-preflight",
                        "member-size-limit",
                        format!("declared original={}, compressed={}", entry.original_size(), entry.compressed_size()),
                    )?;
                    continue;
                }
                let data = self.selected_read(&member_origin, &mut used, entry.original_size() as usize, |sink| {
                    archive.read_to(&entry, sink)?;
                    Ok(())
                })?;
                if let Some(data) = data {
                    let unknown_size =
                        entry.original_size() == 0 && matches!(format, ArchiveFormat::Sq | ArchiveFormat::Z | ArchiveFormat::Gz | ArchiveFormat::Bz2);
                    if !unknown_size && data.len() as u64 != entry.original_size() {
                        self.event(&member_origin, "read", "errors", "declared size mismatch")?;
                    } else {
                        self.member(&member_origin, &name, data, depth)?;
                    }
                }
            }
        }
        Ok(())
    }
    fn open_failure(&mut self, origin: &str, depth: usize, stage: &str, error: impl std::fmt::Display) -> Result<()> {
        if depth == 0 {
            self.count("unreadable_source_archives");
            writeln!(self.unsupported, "{}\t{}: {}", esc(origin), stage, esc(&error.to_string()))?;
        }
        self.failure(origin, stage, error)
    }
    fn source(&mut self, path: &Path) -> Result<()> {
        if !fs::symlink_metadata(path)?.is_file() {
            return Err("source no longer regular".into());
        }
        let mut reader = BufReader::new(File::open(path)?);
        let mut prefix = [0; 32];
        let n = reader.read(&mut prefix)?;
        reader.rewind()?;
        let format = signature(&prefix[..n]).or_else(|| {
            if prefix.starts_with(b"MZ") || path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
                None
            } else {
                ArchiveFormat::from_path(path)
            }
        });
        if let Some(format) = format {
            self.archive(reader, &path_label(path), format, 0)
        } else {
            self.count("unsupported_sources");
            writeln!(
                self.unsupported,
                "{}\tno supported signature/extension; SFX not searched",
                esc(&path_label(path))
            )?;
            Ok(())
        }
    }
    fn run(&mut self, source: &Path) -> Result<()> {
        // Inventory and hash ALL originals before extraction, even if a later cap stops scanning.
        self.db.execute_batch("BEGIN")?;
        for item in WalkDir::new(source).follow_links(false).follow_root_links(false).sort_by_file_name() {
            let entry = match item {
                Ok(e) => e,
                Err(e) => {
                    self.failure(&path_label(source), "walk", e)?;
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                self.count(if entry.file_type().is_symlink() {
                    "symlinks_skipped"
                } else {
                    "source_directories"
                });
                continue;
            }
            self.count("source_files");
            let before = match hash_file(entry.path()) {
                Ok(h) => h,
                Err(e) => {
                    self.failure(&path_label(entry.path()), "hash-before", e)?;
                    String::new()
                }
            };
            self.db.execute(
                "INSERT INTO sources(path,before) VALUES (?1,?2)",
                params![entry.path().as_os_str().as_bytes(), before],
            )?;
        }
        self.db.execute_batch("COMMIT; BEGIN")?;
        let count: i64 = self.db.query_row("SELECT count(*) FROM sources", [], |r| r.get(0))?;
        for id in 1..=count {
            if self.stopped {
                break;
            }
            let (path, before) = self.original(id)?;
            if before.is_empty() {
                continue;
            }
            self.count("sources_attempted");
            if let Err(e) = self.source(&path) {
                self.failure(&path_label(&path), "source", e)?;
            }
        }
        let mut hashes = new_file(&self.out.join("sources.tsv"))?;
        writeln!(hashes, "source\tbefore_sha256\tafter_sha256\tunchanged")?;
        for id in 1..=count {
            let (path, before) = self.original(id)?;
            let after = match hash_file(&path) {
                Ok(h) => h,
                Err(e) => {
                    self.failure(&path_label(&path), "hash-after", e)?;
                    String::new()
                }
            };
            let same = !before.is_empty() && before == after;
            if same {
                self.count("sources_verified");
            } else {
                self.event(&path_label(&path), "verify", "source-verification-failed", "hash mismatch or unreadable source")?;
            }
            writeln!(hashes, "{}\t{before}\t{after}\t{same}", esc(&path_label(&path)))?;
        }
        hashes.flush()?;
        self.db.execute_batch("COMMIT")?;
        self.reports()
    }
    fn original(&self, id: i64) -> Result<(PathBuf, String)> {
        let (path, sha): (Vec<u8>, String) = self
            .db
            .query_row("SELECT path,before FROM sources WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok((OsString::from_vec(path).into(), sha))
    }
    fn reports(&mut self) -> Result<()> {
        // SQLite keeps occurrence/grouping memory bounded; all text remains searchable.
        for key in [
            "archives_top",
            "archives_nested",
            "nested_recognized",
            "members_seen",
            "members_read_attempted",
            "members_read",
            "text_occurrences",
            "text_rejected",
            "binary-rejected",
            "encrypted",
            "errors",
            "unsupported_sources",
            "entry-limit",
            "member-size-limit",
            "depth-limit",
            "archive-limit",
            "global-limit",
            "stored-limit",
        ] {
            self.counts.entry(key.into()).or_default();
        }
        for (file, column, having) in [
            ("basenames.tsv", "basename", "1"),
            ("raw_groups.tsv", "raw", "1"),
            ("normalization_groups.tsv", "normalized", "count(DISTINCT raw)>1"),
            ("numeric_groups.tsv", "digit_fold", "count(DISTINCT normalized)>1"),
        ] {
            let mut out = new_file(&self.out.join(file))?;
            writeln!(
                out,
                "group\toccurrences\tunique_raw\tunique_normalized\tunique_numeric\tunique_basenames\tknown_default\tknown_corpus\trepresentative_origin\trepresentative_raw"
            )?;
            let sql = format!(
                "SELECT {column},count(*),count(DISTINCT raw),count(DISTINCT normalized),count(DISTINCT digit_fold),count(DISTINCT basename),sum(known_default),sum(known_corpus),min(origin),raw FROM samples GROUP BY {column} HAVING {having} ORDER BY count(*) DESC,{column}"
            );
            let mut statement = self.db.prepare(&sql)?;
            let mut rows = statement.query([])?;
            while let Some(row) = rows.next()? {
                write!(out, "{}", esc(&row.get::<_, String>(0)?))?;
                for i in 1..8 {
                    write!(out, "\t{}", row.get::<_, i64>(i)?)?;
                }
                writeln!(out, "\t{}\t{}", esc(&row.get::<_, String>(8)?), row.get::<_, String>(9)?)?;
            }
            out.flush()?;
        }
        let mut candidates = new_file(&self.out.join("candidates.md"))?;
        writeln!(
            candidates,
            "# Manual advertisement review\n\nHeuristic view ONLY, not verified advertisements or cleanup rules. All accepted text is in samples.tsv / audit.sqlite.\n\nDigit folding replaces ALL decimal runs (including dates AND phones) with <n>; manually inspect raw evidence. Rule keywords match RAW bytes and patterns match names only. exact_* uses the catalog API (legacy CRC+size OR SHA256+size). CP437 is inferred.\n\nLimits: depth 3, 16 MiB nested container, 128 KiB text, 10,000 entries/archive, 128 MiB delivered bytes/archive, 2 GiB globally; 1 GiB unique raw+normalized blobs (metadata excluded). Partial/failed reads count. Library internal allocation/CPU and eager decompression are NOT bounded by the writer; not a sandbox. No panic recovery. Raw blobs retain CR/EOF/tails; normalization strips colors/CR, collapses whitespace per line and stops at DOS EOF.\n\nSources must remain quiescent; hashes verify inventoried files, not concurrent additions. Origins use source![index]:member nesting, TSV backslash escapes, and byte-escaped source paths. Duplicate entry indices are preserved.\n\nTop 100 candidate basename groups (counts include all occurrences of that name):\n"
        )?;
        writeln!(
            candidates,
            "Non-ZIP read_to currently buffers the whole member before calling the writer. Declared original/compressed sizes over 16 MiB are skipped before reading, and the larger of declared size / delivered bytes is charged. Unknown sizes and dishonest headers remain library-level risks. Patterns receive the original-case basename; contents are never normalized for catalog matching.\n"
        )?;
        let mut statement = self.db.prepare("SELECT basename,count(*),count(DISTINCT raw),count(DISTINCT normalized),min(origin),raw FROM samples GROUP BY basename HAVING max(ad_view)=1 ORDER BY count(*) DESC,basename LIMIT 100")?;
        let mut rows = statement.query([])?;
        // HTML-escape metadata so untrusted archive names cannot inject Markdown links/HTML.
        while let Some(row) = rows.next()? {
            let safe = |s: String| esc(&s).replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('`', "&#96;");
            writeln!(
                candidates,
                "<pre>{}: {} occurrences, {} raw / {} normalized\n{}\nraw/{}</pre>\n",
                safe(row.get(0)?),
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                safe(row.get(4)?),
                row.get::<_, String>(5)?
            )?;
        }
        candidates.flush()?;
        let partial = self.stopped
            || self.counts.iter().any(|(k, v)| {
                *v > 0 && (k.ends_with("limit") || matches!(k.as_str(), "errors" | "encrypted" | "unsupported_sources" | "source-verification-failed"))
            });
        let mut summary = new_file(&self.out.join("summary.tsv"))?;
        writeln!(summary, "metric\tvalue")?;
        writeln!(
            summary,
            "status\t{}\nwhole_scan_stopped\t{}\nscanned_bytes\t{}\nstored_blob_bytes\t{}",
            if partial { "partial" } else { "complete-within-text-policy" },
            self.stopped,
            self.scanned,
            self.stored
        )?;
        for (key, value) in &self.counts {
            writeln!(summary, "{key}\t{value}")?;
        }
        summary.flush()?;
        self.samples.flush()?;
        self.errors.flush()?;
        self.unsupported.flush()?;
        println!(
            "{}: {} sources, {} text occurrences; {} scanned / {} stored bytes. Reports: {}",
            if partial { "PARTIAL" } else { "COMPLETE within text policy" },
            self.counts.get("source_files").unwrap_or(&0),
            self.counts.get("text_occurrences").unwrap_or(&0),
            self.scanned,
            self.stored,
            self.out.display()
        );
        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: archive_text_audit SOURCE NEW_OUTPUT DEFAULT_MEMBER_RULES CORPUS_MEMBER_RULES".into());
    }
    let source = Path::new(&args[0]);
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        return Err("SOURCE must not be a symlink".into());
    }
    let source = source.canonicalize()?;
    let output = Path::new(&args[1]);
    let parent = output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).canonicalize()?;
    let output = parent.join(output.file_name().ok_or("output must name a new directory")?);
    if output.starts_with(&source) {
        return Err("NEW_OUTPUT must be outside SOURCE".into());
    }
    let mut audit = Audit::new(
        &output,
        FingerprintData::load(&Path::new(&args[2]))?,
        FingerprintData::load(&Path::new(&args[3]))?,
    )?;
    audit.run(&source)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in entries {
            z.start_file(
                *name,
                zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap().into_inner()
    }
    fn audit(out: &Path) -> Audit {
        Audit::new(out, FingerprintData::default(), FingerprintData::default()).unwrap()
    }
    const AD: &[u8] = b"This file passed thru Example Board on 1995-01-02.\r\nCall 5551234 for more information.\r\n";
    #[test]
    fn encoding_colors_eof_and_digits() {
        let rules = TextRules::new();
        let cp = b"\x1b[31m@X0FCaf\x82  BOARD\r\nThis file dated 1995.\x1aTAIL";
        let (a, enc, tail) = rules.normalize(cp);
        let (b, _, _) = rules.normalize("café board\nthis file dated 1995.".as_bytes());
        assert_eq!(a, b);
        assert_eq!(enc, "cp437-inferred");
        assert_eq!(tail, 4);
        assert_eq!(rules.fold(&a), rules.fold(&b.replace("1995", "2026")));
        assert_eq!(rules.normalize(b"\xef\xbb\xbfvalid").1, "utf8-bom");
        for invalid in [&b"\xef\xbb\xbf\xff"[..], &b"\xef\xbb"[..]] {
            assert_eq!(rules.normalize(invalid).1, "cp437-inferred");
        }
        assert_eq!(rules.fold("phone 1234 date 2026-01-02"), "phone <n> date <n>-<n>-<n>");
        assert!(!text_like(b"MZ This file is not actually readable text", "this file has letters"));
        assert_eq!(signature(b"some normal text here"), None);
    }
    #[test]
    fn synthetic_corpus_unchanged_renames_and_groups() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fs::create_dir(&source).unwrap();
        let default = dir.path().join("rules.toml");
        fs::write(&default, format!("[[fingerprint]]\nsha256 = {:?}\nfile_size = {}\n", hash(AD), AD.len())).unwrap();
        let raw = zip(&[
            ("../renamed.module", AD),
            ("README", AD),
            ("FILE_ID.DIZ", AD),
            ("different.ad", &String::from_utf8_lossy(AD).replace("1995", "1996").into_bytes()),
        ]);
        fs::write(source.join("one.zip"), &raw).unwrap();
        let inner = zip(&[("another.name", AD)]);
        fs::write(source.join("nested.zip"), zip(&[("no-extension", &inner)])).unwrap();
        std::os::unix::fs::symlink(source.join("one.zip"), source.join("link.zip")).unwrap();
        fs::write(source.join("unknown.exe"), b"MZ fake unsupported executable").unwrap();
        let out = dir.path().join("out");
        let mut a = Audit::new(&out, FingerprintData::load(&default).unwrap(), FingerprintData::load(&default).unwrap()).unwrap();
        a.run(&source).unwrap();
        assert_eq!(fs::read(source.join("one.zip")).unwrap(), raw);
        assert_eq!(a.counts["sources_verified"], 3);
        assert_eq!(a.counts["text_occurrences"], 5);
        assert_eq!(a.counts["archives_nested"], 1);
        assert_eq!(a.counts["symlinks_skipped"], 1);
        let known: i64 =
            a.db.query_row("SELECT count(*) FROM samples WHERE exact_default=1 AND exact_corpus=1", [], |r| r.get(0))
                .unwrap();
        assert_eq!(known, 4);
        assert!(fs::read_to_string(out.join("numeric_groups.tsv")).unwrap().lines().count() > 1);
        assert!(!dir.path().join("renamed.module").exists());
        assert!(Audit::new(&out, FingerprintData::default(), FingerprintData::default()).is_err());
    }
    #[test]
    fn limits_charge_partial_reads_stop_and_verify() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("input.zip");
        fs::write(&source, zip(&[("a", AD), ("b", AD)])).unwrap();
        let mut a = audit(&dir.path().join("out"));
        a.total_limit = 40;
        a.run(&source).unwrap();
        assert!(a.stopped);
        assert_eq!(a.scanned, 40);
        assert_eq!(a.counts["sources_verified"], 1);
        assert!(fs::read_to_string(a.out.join("summary.tsv")).unwrap().contains("partial"));
        let mut b = audit(&dir.path().join("store"));
        b.store_limit = 1;
        b.run(&source).unwrap();
        assert!(b.stopped);
        assert_eq!(b.stored, 0);
        assert_eq!(b.counts["text_occurrences"], 1);
        let mut c = audit(&dir.path().join("member"));
        c.archive(
            Cursor::new(zip(&[("too-big.txt", &vec![b'a'; TEXT + 1]), ("later.txt", AD)])),
            "test",
            ArchiveFormat::Zip,
            0,
        )
        .unwrap();
        assert_eq!(c.counts["member-size-limit"], 1);
        assert_eq!(c.counts["text_occurrences"], 1);
        let mut used = EXPANDED - 5;
        c.selected_read("limit", &mut used, 0, |w| {
            w.write_all(AD)?;
            Ok(())
        })
        .unwrap();
        assert_eq!(used, EXPANDED);
        assert_eq!(c.counts["archive-limit"], 1);
    }
    #[test]
    fn corrupt_entry_continues_and_depth_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = audit(&dir.path().join("out"));
        let mut data = zip(&[("bad", AD), ("good", AD)]);
        let offset = data.windows(AD.len()).position(|w| w == AD).unwrap();
        data[offset] ^= 1;
        a.archive(Cursor::new(data), "corrupt", ArchiveFormat::Zip, 0).unwrap();
        assert_eq!(a.counts["errors"], 1);
        assert_eq!(a.counts["text_occurrences"], 1);
        let mut nested = zip(&[("text", AD)]);
        for _ in 0..4 {
            nested = zip(&[("nested", &nested)]);
        }
        a.archive(Cursor::new(nested), "deep", ArchiveFormat::Zip, 0).unwrap();
        assert_eq!(a.counts["depth-limit"], 1);
    }
    #[test]
    fn encodings_group_across_archive_member_names_without_changing_matching() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = audit(&dir.path().join("out"));
        let cp = b"@X0FCaf\x82 BOARD\r\nThis file contains enough readable words.";
        let utf = "café board\nthis file contains enough readable words.".as_bytes();
        let rules = dir.path().join("patterns.toml");
        fs::write(&rules, "[[fingerprint]]\npattern='^FIRST$'\nkeywords=['BOARD']\n").unwrap();
        a.default = FingerprintData::load(&rules).unwrap();
        a.archive(
            Cursor::new(zip(&[("path/FIRST", cp), ("other.module", utf)])),
            "encoding",
            ArchiveFormat::Zip,
            0,
        )
        .unwrap();
        let groups: (i64, i64, i64) =
            a.db.query_row(
                "SELECT count(DISTINCT raw),count(DISTINCT normalized),sum(known_default) FROM samples",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(groups, (2, 1, 1));
        a.reports().unwrap();
        assert_eq!(fs::read_to_string(a.out.join("normalization_groups.tsv")).unwrap().lines().count(), 2);
    }
    #[test]
    fn writer_nested_and_entry_caps_and_nonzip_accounting() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = audit(&dir.path().join("out"));
        let mut nested = vec![0; NESTED + 1];
        nested[..4].copy_from_slice(b"PK\x03\x04");
        let mut used = 0;
        assert!(
            a.selected_read("nested", &mut used, 0, |w| {
                w.write_all(&nested)?;
                Ok(())
            })
            .unwrap()
            .is_none()
        );
        assert_eq!(used, NESTED);
        let mut binary = vec![0; 4096];
        binary[..2].copy_from_slice(b"MZ");
        a.selected_read("nonzip", &mut used, binary.len(), |w| {
            w.write_all(&binary)?;
            Ok(())
        })
        .unwrap();
        assert_eq!(used, NESTED + binary.len());
        // Hand-built valid stored TAR exercises the real non-ZIP read_to path.
        let mut tar = vec![0u8; 2048];
        tar[..8].copy_from_slice(b"text.mod");
        tar[100..108].copy_from_slice(b"0000644\0");
        tar[124..136].copy_from_slice(format!("{:011o}\0", AD.len()).as_bytes());
        tar[148..156].fill(b' ');
        tar[156] = b'0';
        tar[257..263].copy_from_slice(b"ustar\0");
        let sum: usize = tar[..512].iter().map(|b| *b as usize).sum();
        tar[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        tar[512..512 + AD.len()].copy_from_slice(AD);
        a.archive(Cursor::new(tar), "tar", ArchiveFormat::Tar, 0).unwrap();
        assert_eq!(a.counts["text_occurrences"], 1);
        let entries: Vec<_> = (0..ENTRIES + 1).map(|i| format!("{i}.txt")).collect();
        let data: Vec<_> = entries.iter().map(|s| (s.as_str(), &b""[..])).collect();
        a.archive(Cursor::new(zip(&data)), "entries", ArchiveFormat::Zip, 0).unwrap();
        assert_eq!(a.counts["entry-limit"], 1);
    }
}
