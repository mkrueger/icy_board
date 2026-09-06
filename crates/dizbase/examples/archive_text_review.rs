//! Read-only analysis of archive_text_audit output; never installs cleanup rules.
use std::{collections::BTreeMap, fs, path::Path};

use dizbase::file_base_scanner::bbstro_fingerprint::FingerprintData;
use regex::Regex;
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Deliberately broad discovery queries, NOT safe whole-member deletion rules.
const MARKERS: &[(&str, &str)] = &[
    ("clipper-workshop", r"clipper workshop bbs"),
    ("cutting-edge", r"cutting edge online"),
    ("pacific-coast-micro", r"pacific coast micro"),
    ("bbs-archives", r"the bbs archives"),
    ("buggerer-deluxe", r"this annoying advert was created by buggerer deluxe"),
];
const NAMES: &[&str] = &[
    "workshop.bbs",
    "ceos.ad",
    "pcm.805",
    "pcm.nfo",
    "pcmicro.bbs",
    "archives.bbs",
    "out.ad",
    "tcs_ad.txt",
];

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\t', "\\t").replace('\r', "\\r").replace('\n', "\\n")
}

fn review(audit: &Path, output: &Path) -> Result<()> {
    // New sibling/output directory only. A completed audit must be supplied.
    let summary = fs::read_to_string(audit.join("summary.tsv"))?;
    let db = Connection::open_with_flags(audit.join("audit.sqlite"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut text_by_hash = BTreeMap::new();
    let mut groups = BTreeMap::<String, Vec<String>>::new();
    let markers: Vec<_> = MARKERS.iter().map(|(id, pattern)| (*id, Regex::new(pattern).unwrap())).collect();
    let mut statement = db.prepare("SELECT DISTINCT normalized FROM samples WHERE stored=1")?;
    for row in statement.query_map([], |r| r.get::<_, String>(0))? {
        let hash = row?;
        let text = fs::read_to_string(audit.join("normalized").join(&hash))?;
        let mut matched = false;
        for (id, marker) in &markers {
            if marker.is_match(&text) {
                groups.entry((*id).into()).or_default().push(hash.clone());
                matched = true;
            }
        }
        if matched {
            text_by_hash.insert(hash, text);
        }
    }
    let mut variants = String::from(
        "# Textvarianten ausgewählter Dateinamen\n\nKeine Löschfreigabe: auch Originalwerbung, Supporttexte und beliebige gleichnamige Dateien können enthalten sein. Normalisierte Texte sind nur Vergleichsansichten, keine Originalbytes.\n",
    );
    let mut findings = String::from("origin\traw_sha256\tnormalized_sha256\tbasename\tencoding\tclassification\tmarkers\n");
    let mut counts = BTreeMap::<String, [usize; 3]>::new();
    let mut names = String::from("basename\toccurrences\traw_variants\tnormalized_variants\tdigit_fold_groups\n");
    let mut named = db.prepare("SELECT count(*),count(DISTINCT raw),count(DISTINCT normalized),count(DISTINCT digit_fold) FROM samples WHERE basename=?1")?;
    for name in NAMES {
        let values: [i64; 4] = named.query_row([name], |r| Ok([r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?]))?;
        names.push_str(&format!("{name}\t{}\t{}\t{}\t{}\n", values[0], values[1], values[2], values[3]));
        variants.push_str(&format!(
            "\n## {name}\n\n{} Vorkommen; {} Bytevarianten; {} normalisierte Texte; {} Gruppen nach Ziffernersetzung.\n",
            values[0], values[1], values[2], values[3]
        ));
        let mut stmt = db.prepare(
            "SELECT normalized,min(origin),count(*),min(size),max(size) FROM samples WHERE basename=?1 GROUP BY normalized ORDER BY count(*) DESC,normalized",
        )?;
        for row in stmt.query_map([name], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })? {
            let (hash, origin, n, min, max) = row?;
            let text = fs::read_to_string(audit.join("normalized").join(&hash))?;
            variants.push_str(&format!(
                "\n### {hash}\n\n{n} Vorkommen; {min}–{max} Bytes. Beispiel: {}\n\n<pre>{}</pre>\n",
                html(&origin),
                html(&text)
            ));
        }
    }
    let mut stmt = db.prepare("SELECT origin,raw,normalized,basename,encoding,kind FROM samples ORDER BY origin")?;
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    })? {
        let (origin, raw, norm, name, encoding, kind) = row?;
        let matched: Vec<_> = groups.iter().filter(|(_, hashes)| hashes.contains(&norm)).map(|(id, _)| id.as_str()).collect();
        for id in &matched {
            let count = counts.entry((*id).into()).or_default();
            count[0] += 1;
            count[1] += usize::from(kind == "doc-like");
            count[2] += usize::from(kind == "canonical-description");
        }
        if !matched.is_empty() || NAMES.contains(&name.as_str()) {
            findings.push_str(&format!(
                "{}\t{raw}\t{norm}\t{}\t{encoding}\t{kind}\t{}\n",
                escape(&origin),
                escape(&name),
                matched.join(",")
            ));
        }
    }
    let mut marker_report = String::from(
        "# Breite Suchmarker: ausdrücklich keine Löschregeln\n\nDoc-like ist nur eine Dateinamenheuristik, kein Inhaltsurteil. Auch Treffer außerhalb dieser Gruppe können Originaldateien sein.\n\n| Marker | Vorkommen | Normalisierte Texte | Doc-like | Kanonische Beschreibung |\n|---|---:|---:|---:|---:|\n",
    );
    for (id, _) in MARKERS {
        let count = counts.get(*id).copied().unwrap_or_default();
        marker_report.push_str(&format!(
            "| {id} | {} | {} | {} | {} |\n",
            count[0],
            groups.get(*id).map_or(0, Vec::len),
            count[1],
            count[2]
        ));
    }
    let mut dynamic = String::from(
        "# Beleg: Buggerer Deluxe\n\nDieser Suchmarker belegt die Generator-Selbstauskunft. Er ist keine aktivierte Löschregel. Fundstellen und Rohhashes stehen in findings.tsv.\n",
    );
    for hash in groups.get("buggerer-deluxe").into_iter().flatten() {
        dynamic.push_str(&format!("\n## {hash}\n\n<pre>{}</pre>\n", html(&text_by_hash[hash])));
    }
    fs::create_dir(output)?;
    for (file, text) in [
        ("scan-summary.tsv", summary),
        ("families.tsv", names),
        ("variants.md", variants),
        ("findings.tsv", findings),
        ("discovery-markers.md", marker_report),
        ("dynamic-generator.md", dynamic),
    ] {
        fs::write(output.join(file), text)?;
    }
    println!("Research reports written to {}; production rules and archives unchanged", output.display());
    Ok(())
}

fn html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Run the production matcher against original bytes, not research-normalized text.
fn review_text_rules(audit: &Path, catalog: &Path, output: &Path) -> Result<()> {
    let rules = FingerprintData::load_split(catalog, Path::new(""))?;
    let db = Connection::open_with_flags(audit.join("audit.sqlite"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut candidates = BTreeMap::new();
    let mut raw_count = 0;
    let mut controls = 0;
    let mut statement = db.prepare("SELECT DISTINCT raw FROM samples WHERE stored=1")?;
    for row in statement.query_map([], |r| r.get::<_, String>(0))? {
        let hash = row?;
        let raw = fs::read(audit.join("raw").join(&hash))?;
        if format!("{:x}", Sha256::digest(&raw)) != hash {
            return Err(format!("modified raw blob {hash}").into());
        }
        raw_count += 1;
        let matches = rules.match_text_member("renamed.bin", &raw);
        if matches.is_empty() {
            continue;
        }
        if matches.len() != 1 {
            return Err(format!("ambiguous text rules for {hash}").into());
        }
        // Reject extra original documentation and hidden EOF payloads, even
        // when all of the known advertisement remains present.
        for changed in [
            [b"Original manual\n".as_slice(), raw.as_slice()].concat(),
            [raw.as_slice(), b"\nOriginal manual"].concat(),
            [raw.as_slice(), b"\x1ahidden data"].concat(),
        ] {
            if !rules.match_text_member("renamed.bin", &changed).is_empty() {
                return Err(format!("negative control matched for {hash}").into());
            }
            controls += 1;
        }
        let utf8 = match std::str::from_utf8(&raw) {
            Ok(text) => text.to_owned(),
            Err(_) => raw.iter().map(|b| codepages::tables::CP437_TO_UNICODE[*b as usize]).collect(),
        };
        if rules.match_text_member("renamed.bin", utf8.as_bytes()).len() != 1 {
            return Err(format!("UTF-8 control did not match for {hash}").into());
        }
        controls += 1;
        candidates.insert(hash, raw);
    }
    let mut counts = BTreeMap::<String, [usize; 3]>::new();
    let mut findings = String::from("origin\traw_sha256\trule_id\taction\tencoding\tlegacy_rule_takes_precedence\n");
    let mut seen = 0;
    let mut statement = db.prepare("SELECT origin,raw,basename,kind FROM samples WHERE stored=1 ORDER BY origin")?;
    for row in statement.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
    })? {
        let (origin, hash, name, kind) = row?;
        seen += 1;
        let Some(raw) = candidates.get(&hash) else {
            continue;
        };
        for matched in rules.match_text_member(&name, raw) {
            let legacy = rules.is_match(&name, raw);
            let count = counts.entry(matched.rule_id.clone()).or_default();
            count[0] += 1;
            count[1] += usize::from(legacy);
            count[2] += usize::from(kind == "doc-like");
            findings.push_str(&format!(
                "{}\t{hash}\t{}\t{:?}\t{}\t{legacy}\n",
                escape(&origin),
                escape(&matched.rule_id),
                matched.action,
                matched.encoding
            ));
        }
    }
    let mut report = format!(
        "# Production text-rule trial\n\nRead-only raw-blob replay: {seen} stored occurrences, {raw_count} SHA-256-verified raw blobs; {controls} generated negative/UTF-8 checks passed. No source archives or configured board files changed.\n\nThese are template matches, not unconditional removal decisions. Legacy byte rules run first. Doc-like is a filename heuristic, not a content verdict. Scan exclusions from the original audit still apply.\n\n| Rule | Template matches | Legacy precedence | Doc-like names |\n|---|---:|---:|---:|\n"
    );
    for (id, count) in counts {
        report.push_str(&format!("| {id} | {} | {} | {} |\n", count[0], count[1], count[2]));
    }
    report.push_str(&format!("\nCatalog SHA-256: `{:x}`\n", Sha256::digest(fs::read(catalog)?)));
    fs::create_dir(output)?;
    fs::write(output.join("README.md"), &report)?;
    fs::write(output.join("findings.tsv"), findings)?;
    println!("{report}");
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 4 && args[0] == "--text-rules" {
        return review_text_rules(Path::new(&args[1]), Path::new(&args[2]), Path::new(&args[3]));
    }
    if args.len() != 2 {
        return Err("usage: archive_text_review AUDIT NEW_REPORT_DIRECTORY | --text-rules AUDIT MEMBER_CATALOG NEW_REPORT_DIRECTORY".into());
    }
    review(Path::new(&args[0]), Path::new(&args[1]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_escapes_untrusted_metadata() {
        assert_eq!(escape("a\\b\tc\nd\r"), "a\\\\b\\tc\\nd\\r");
        assert_eq!(html("<a&b>"), "&lt;a&amp;b&gt;");
        for (_, pattern) in MARKERS {
            Regex::new(pattern).unwrap();
        }
    }
}
