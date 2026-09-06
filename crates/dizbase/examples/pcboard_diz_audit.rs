//! Read-only PCBoard archive description audit; output is always a new directory.
use codepages::{normalize_file, tables::get_utf8};
use dizbase::file_base_scanner::description_cleaner::{DescriptionBlockRule, DescriptionCleaner, RuleAction};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Read},
    path::Path,
};
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Serialize, Deserialize)]
struct Diz {
    origin: String,
    hash: String,
}
#[derive(Default, Serialize, Deserialize)]
struct Audit {
    source: String,
    archives: usize,
    nested_archives: usize,
    sources: BTreeMap<String, String>,
    errors: Vec<String>,
    diz: Vec<Diz>,
}

fn sha(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn inspect(data: &[u8], origin: &str, output: &Path, depth: usize, audit: &mut Audit) -> Result<()> {
    if depth > 4 {
        return Err("nested depth limit".into());
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(data))?;
    if zip.len() > 10_000 {
        return Err("archive member limit".into());
    }
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_owned();
        let base = name.rsplit(['/', '\\']).next().unwrap().to_lowercase();
        let is_diz = base.ends_with(".diz") || matches!(base.as_str(), "desc.sdi" | "file_id.ans" | "file_id.pcb");
        let nested = base.ends_with(".zip");
        if !is_diz && !nested {
            continue;
        }
        let limit = if nested { 16 * 1024 * 1024 } else { 256 * 1024 };
        if entry.size() > limit {
            return Err(format!("size limit: {name}").into());
        }
        let mut bytes = Vec::new();
        entry.by_ref().take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err("expanded size limit".into());
        }
        let member_origin = format!("{origin}::{name}");
        if nested {
            audit.nested_archives += 1;
            if let Err(err) = inspect(&bytes, &member_origin, output, depth + 1, audit) {
                audit.errors.push(format!("{member_origin}: {err}"));
            }
        } else {
            let hash = sha(&bytes);
            fs::write(output.join("raw").join(&hash), &bytes)?;
            audit.diz.push(Diz { origin: member_origin, hash });
        }
    }
    Ok(())
}

fn scan(source: &Path, output: &Path) -> Result<()> {
    fs::create_dir(output)?;
    fs::create_dir(output.join("raw"))?;
    let mut audit = Audit {
        source: source.display().to_string(),
        ..Default::default()
    };
    for entry in WalkDir::new(source).follow_links(false).sort_by_file_name() {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(source)?.to_string_lossy().into_owned();
        if !relative.split('/').next().unwrap().to_lowercase().contains("pcboard") || !relative.to_lowercase().ends_with(".zip") {
            continue;
        }
        let bytes = fs::read(entry.path())?;
        audit.archives += 1;
        audit.sources.insert(relative.clone(), sha(&bytes));
        if let Err(err) = inspect(&bytes, &relative, output, 0, &mut audit) {
            audit.errors.push(format!("{relative}: {err}"));
        }
    }
    let mut sections = BTreeMap::<String, String>::new();
    let mut frequency = BTreeMap::<String, usize>::new();
    let mut seen = std::collections::BTreeSet::new();
    for diz in &audit.diz {
        if !seen.insert(&diz.hash) {
            continue;
        }
        let bytes = fs::read(output.join("raw").join(&diz.hash))?;
        let normalized = normalize_file(&bytes);
        if normalized.starts_with(&[0xef, 0xbb, 0xbf]) && std::str::from_utf8(&normalized[3..]).is_err() {
            audit.errors.push(format!("{}: invalid UTF8 BOM", diz.origin));
            continue;
        }
        let text = get_utf8(&normalized);
        let section = sections.entry(diz.origin.split('/').next().unwrap().to_owned()).or_default();
        section.push_str(&format!("\n### {}\nHASH {}\n```text\n{}\n```\n", diz.origin, diz.hash, text));
        for line in text.lines() {
            let line = line.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
            if !line.is_empty() {
                *frequency.entry(line).or_default() += 1;
            }
        }
    }
    for (category, text) in sections {
        fs::write(output.join(format!("{category}.md")), text)?;
    }
    let mut frequency: Vec<_> = frequency.into_iter().collect();
    frequency.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    fs::write(
        output.join("lines.tsv"),
        frequency.iter().map(|(line, count)| format!("{count}\t{line}\n")).collect::<String>(),
    )?;
    for (relative, hash) in &audit.sources {
        assert_eq!(sha(&fs::read(source.join(relative))?), *hash, "source changed");
    }
    fs::write(output.join("audit.toml"), toml::to_string_pretty(&audit)?)?;
    println!(
        "{} archives + {} nested, {} descriptions ({} unique), {} errors; source hashes unchanged",
        audit.archives,
        audit.nested_archives,
        audit.diz.len(),
        seen.len(),
        audit.errors.len()
    );
    for error in &audit.errors {
        eprintln!("{error}");
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.as_slice() {
        [_, mode, audit, board_corpus, output] if mode == "compare-live" => compare_live(Path::new(audit), Path::new(board_corpus), Path::new(output)),
        [_, mode, audit, defaults, corpus, report] if mode == "curate" => curate(Path::new(audit), Path::new(defaults), Path::new(corpus), Path::new(report)),
        [_, source, output] => scan(Path::new(source), Path::new(output)),
        _ => Err("usage: pcboard_diz_audit SOURCE NEW_OUTPUT | curate AUDIT DEFAULT_RULES CORPUS_RULES NEW_REPORT_DIRECTORY | compare-live AUDIT BOARD_CORPUS NEW_OUTPUT".into()),
    }
}

/// Read published area copies, accepting only the original bytes or the exact
/// result of the old board rules. Never opens the board database for writing.
fn compare_live(audit_dir: &Path, board_corpus: &Path, output: &Path) -> Result<()> {
    let original: Audit = toml::from_str(&fs::read_to_string(audit_dir.join("audit.toml"))?)?;
    let baseline: toml::Value = toml::from_str(&fs::read_to_string(board_corpus.join("baseline.toml"))?)?;
    let baseline_files = baseline["files"].as_array().ok_or("missing baseline files")?;
    for (relative, hash) in &original.sources {
        let file = baseline_files
            .iter()
            .find(|f| f["relative"].as_str() == Some(relative))
            .ok_or("source absent from board baseline")?;
        if file["sha256"].as_str() != Some(hash) {
            return Err(format!("board baseline source differs: {relative}").into());
        }
    }
    let old: Rules = toml::from_str(&fs::read_to_string(board_corpus.join("rules.used.toml"))?)?;
    let old_cleaner = DescriptionCleaner::new(&old.description_rule)?;
    let mut known = BTreeMap::<String, Vec<String>>::new();
    for sample in &original.diz {
        let raw = fs::read(audit_dir.join("raw").join(&sample.hash))?;
        assert_eq!(sha(&raw), sample.hash);
        let cleaned = old_cleaner.clean(sample.origin.rsplit("::").next().unwrap(), &raw, 8);
        known
            .entry(sample.origin.clone())
            .or_default()
            .extend([sample.hash.clone(), sha(&cleaned.content)]);
    }
    let directories: toml::Value = toml::from_str(&fs::read_to_string(board_corpus.join("directories.toml"))?)?;
    let directories = directories.get("area").and_then(toml::Value::as_array).ok_or("missing file areas")?;
    fs::create_dir(output)?;
    fs::create_dir(output.join("raw"))?;
    let mut live = Audit {
        source: board_corpus.display().to_string(),
        ..Default::default()
    };
    let mut areas = 0;
    for directory in directories {
        let category = directory["name"].as_str().ok_or("missing area name")?.trim_start_matches("Archives: ");
        if !category.to_lowercase().contains("pcboard") {
            continue;
        }
        areas += 1;
        let path = Path::new(directory["path"].as_str().ok_or("missing area path")?);
        if !path.is_absolute() {
            return Err("expected absolute published area path".into());
        }
        for entry in WalkDir::new(path).follow_links(false).sort_by_file_name() {
            let entry = entry?;
            if !entry.file_type().is_file() || !entry.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
                continue;
            }
            let bytes = fs::read(entry.path())?;
            let origin = format!("{category}/{}", entry.path().strip_prefix(path)?.display());
            live.archives += 1;
            live.sources.insert(entry.path().display().to_string(), sha(&bytes));
            if let Err(err) = inspect(&bytes, &origin, output, 0, &mut live) {
                live.errors.push(format!("{origin}: {err}"));
            }
        }
    }
    if areas != 8 {
        return Err(format!("expected eight PCBoard areas, found {areas}").into());
    }
    let mut differences = String::from("origin\tsha256\n");
    let mut count = 0;
    for sample in &live.diz {
        if !known.get(&sample.origin).is_some_and(|hashes| hashes.contains(&sample.hash)) {
            count += 1;
            differences.push_str(&format!("{}\t{}\n", sample.origin, sample.hash));
        }
    }
    for (path, hash) in &live.sources {
        assert_eq!(sha(&fs::read(path)?), *hash, "live source changed");
    }
    fs::write(output.join("audit.toml"), toml::to_string_pretty(&live)?)?;
    fs::write(output.join("differences.tsv"), differences)?;
    println!(
        "{} baseline source hashes verified; {areas} live areas, {} archives + {} nested, {} descriptions; {count} differing descriptions; {} errors; live hashes unchanged",
        original.sources.len(),
        live.archives,
        live.nested_archives,
        live.diz.len(),
        live.errors.len()
    );
    for error in &live.errors {
        eprintln!("{error}");
    }
    Ok(())
}

#[derive(Default, Deserialize, Serialize)]
struct Rules {
    #[serde(default)]
    description_rule: Vec<DescriptionBlockRule>,
}

// Manually reviewed boundaries in actual release descriptions, NOT entries
// copied from STRIP.DIZ dictionaries or guessed from telephone number patterns.
// (id, source suffix, first-line marker, number of lines, report-only)
const SELECTIONS: &[(&str, &str, &str, usize, bool)] = &[
    ("critical-strike-transit", "911lv1b.zip::FILE_ID.DIZ", "HΣY LÄMεΓ", 6, false),
    ("unknown-realm-transit", "2_2ddownx.zip::FILE_ID.DIZ", "This File Passed Thru", 2, false),
    ("eyes-cream-transit", "caz_2dentr.zip::FILE_ID.DIZ", ".,-", 5, false),
    ("lords-couriering-94", "blot_ul.zip::FILE_ID.DIZ", "L.O.R.D.S.", 1, false),
    ("white-sands-transit", "art1bpwa.zip::FILE_ID.DIZ", "Leeched from White Sands", 2, false),
    ("blood-pool-transit", "lur98pwa.zip::FILE_ID.DIZ", "BLooD PooL", 1, false),
    ("spaezm-couriers", "ciafonlr.zip::FILE_ID.DIZ", "SPÆZM", 1, false),
    ("scimitar-spreader", "infopwa.zip::FILE_ID.DIZ", "SCIMITAR", 1, false),
    ("rts-couriers", "dod_2ddl1c.zip::FILE_ID.DIZ", "Brought to you by RTS", 1, false),
    ("limpy-crack-mark", "dod_2ddl1c.zip::FILE_ID.DIZ", "CRACKED BY", 1, true),
    ("ambient-hauze-transit", "dod_2due12.zip::FILE_ID.DIZ", "PASSED THRU", 1, false),
    ("risc-couriering-94", "dod_2dup32.zip::FILE_ID.DIZ", "RiSC COURiERiNG", 1, false),
    ("digital-delusions-transit", "lspd_2dnup.zip::FILE_ID.DIZ", "Passed Through Digital", 1, false),
    ("digital-delusions-transit-315", "flx_2deblt.zip::FILE_ID.DIZ", "Passed Thru Digital", 1, false),
    ("high-tech-couriers-95", "lspd_2dnup.zip::file_id.diz", "HiGH TeCh", 1, false),
    ("high-tech-couriers-slash", "lspdex1.zip::FILE_ID.DIZ", "High Tech Couriers", 1, false),
    ("high-tech-couriers-cross", "ftatul16.zip::FILE_ID.DIZ", "HiGH TeCH", 1, false),
    ("distinct-courier", "lspderv1.zip::FILE_ID.DIZ", "Courier'd", 1, false),
    ("ufp-couriering", "lspd_2dvfb.zip::FILE_ID.DIZ", "Dístributéd By", 1, false),
    ("fate97-courier", "food_21bll.zip::FILE_ID.DIZ", "fATE97", 1, false),
    ("fate95-courier", "ror_2dmail.zip::FILE_ID.DIZ", "fATE95", 1, false),
    ("fate95-courier-slash", "ror_2dslam.zip::FILE_ID.DIZ", "fATE95", 1, false),
    ("rts-whq-transit", "fsn_2dnuu.zip::FILE_ID.DIZ", "Scortched", 1, false),
    ("metro-couriers-95", "gnxzmrf1.zip::FILE_ID.DIZ", "metro couriers", 1, false),
    ("rod-bbs-transit", "nwread2.zip::FILE_ID.DIZ", "Leeched", 1, false),
    (
        "wild-thing-transit-box",
        "jm_mf_10.zip::FILE_ID.DIZ",
        "┌───────────────────────────────────┐",
        3,
        false,
    ),
    ("wild-thing-euro-connection", "bcwm051.zip::FILE_ID.DIZ", "┌»»LεεCHεD", 2, false),
    ("holland-number-one-transit", "dma_2dwall.zip::FILE_ID.DIZ", "went trough holland", 1, false),
    ("kort-whq-upload", "bcpg10.zip::FILE_ID.DIZ", "4 Ñodes KORT", 3, false),
    ("bsbbs-phone", "elt_2dnfse.zip::FILE_ID.DIZ", "bsbbs.1.", 1, false),
    ("bsbbs-phone-obfuscated", "bpc_2dub10.zip::FILE_ID.DIZ", "b@X04sb", 1, false),
    // These named promotional blocks may belong to the original release's
    // distribution/support group: record them but don't delete automatically.
    ("nostrum-nine-promotion", "cr_2ddisc.zip::FILE_ID.DIZ", "+31(o)317", 2, true),
    ("nexus-project-promotion", "imn_2daloh.zip::FILE_ID.DIZ", "iMMUNE!  tRADERS", 2, true),
    ("reckless-life-promotion", "nkpage12.zip::FILE_ID.DIZ", "RΣ¢KLΣS$", 1, true),
    ("mtnt-hacker-courier", "mtntmes2.zip::FILE_ID.DIZ", "Couriered by", 2, true),
    ("disembodied-lands-promotion", "wkd_2dlam1.zip::FILE_ID.DIZ", "Disembodied Voices", 2, true),
    ("nuclear-insemination-promotion", "fsn_2dnuu.zip::FILE_ID.DIZ", "Nuclear Insemination", 1, true),
];

fn decoded_lines(bytes: &[u8]) -> Vec<String> {
    bytes
        .split_inclusive(|b| *b == b'\n')
        .map(|line| get_utf8(&normalize_file(line)).trim_end_matches('\n').to_owned())
        .collect()
}

fn curate(audit_dir: &Path, defaults: &Path, corpus: &Path, report_dir: &Path) -> Result<()> {
    let audit: Audit = toml::from_str(&fs::read_to_string(audit_dir.join("audit.toml"))?)?;
    let default_text = fs::read_to_string(defaults)?;
    let corpus_text = fs::read_to_string(corpus)?;
    let mut rules: Rules = toml::from_str(&default_text)?;
    let corpus_rules: Rules = toml::from_str(&corpus_text)?;
    let original_ids: std::collections::BTreeSet<_> = rules.description_rule.iter().map(|r| r.id.clone()).collect();
    let ansi = Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]")?;
    let colors = Regex::new(r"(?i)@X[0-9a-f]{2}")?;
    for &(id, origin, marker, count, report_only) in SELECTIONS {
        if original_ids.contains(id) {
            continue;
        }
        let sample = audit
            .diz
            .iter()
            .find(|d| d.origin.ends_with(origin))
            .ok_or_else(|| format!("missing source {origin}"))?;
        let raw = fs::read(audit_dir.join("raw").join(&sample.hash))?;
        let lines = decoded_lines(&raw);
        let first = lines
            .iter()
            .position(|l| l.contains(marker))
            .ok_or_else(|| format!("missing marker {id}: {marker}"))?;
        let mut literals: Vec<_> = lines[first..first + count]
            .iter()
            .map(|l| {
                colors
                    .replace_all(&ansi.replace_all(l, ""), "")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        // The dot separator belongs to the inserted White Sands block.
        if id == "white-sands-transit" {
            literals.insert(0, ".".into());
        }
        let mut rule = DescriptionBlockRule {
            id: id.into(),
            literal_lines: literals,
            action: if report_only { RuleAction::ReportOnly } else { RuleAction::AutoClean },
            ..Default::default()
        };
        if id == "critical-strike-transit" {
            rule.lines = rule
                .literal_lines
                .iter()
                .map(|line| format!("^{}$", regex::escape(&line.to_lowercase())))
                .collect();
            rule.lines[3] = r"^│ üplφadσd φ∩ [0-9]{2}\.[0-9]{2}\.[0-9]{2} at [0-9]{2}:[0-9]{2}$".into();
            rule.literal_lines.clear();
        }
        rules.description_rule.push(rule);
    }
    let cleaner = DescriptionCleaner::new(&rules.description_rule)?;
    let mut combined = corpus_rules.description_rule.clone();
    for rule in &rules.description_rule {
        if !combined.iter().any(|r| r.id == rule.id) {
            combined.push(rule.clone());
        }
    }
    let combined_cleaner = DescriptionCleaner::new(&combined)?;
    let mut occurrences = BTreeMap::<String, Vec<String>>::new();
    let mut changes_tsv = String::from("origin\tsha256\trule\taction\tfirst_line\tlast_line\n");
    let mut ambiguities = Vec::new();
    let mut cleaned_count = 0;
    let mut negative_count = 0;
    let mut changed_text = String::new();
    for sample in &audit.diz {
        let bytes = fs::read(audit_dir.join("raw").join(&sample.hash))?;
        assert_eq!(sha(&bytes), sample.hash);
        let name = sample.origin.rsplit("::").next().unwrap();
        let result = cleaner.clean(name, &bytes, 8);
        let merged_result = combined_cleaner.clean(name, &bytes, 8);
        if merged_result.needs_review && !result.needs_review {
            return Err(format!("new combined-catalog ambiguity: {}", sample.origin).into());
        }
        if merged_result.content != result.content {
            return Err(format!("combined catalog changes additional bytes: {}", sample.origin).into());
        }
        if result.needs_review {
            ambiguities.push(sample.origin.clone());
        }
        if result.content != bytes {
            cleaned_count += 1;
            assert!(!result.content.is_empty(), "entire description removed: {}", sample.origin);
            assert!(
                !cleaner
                    .clean(name, &result.content, 8)
                    .changes
                    .iter()
                    .any(|c| c.action == RuleAction::AutoClean),
                "not idempotent"
            );
            changed_text.push_str(&format!(
                "\n## {}\n\nBefore:\n```text\n{}\n```\nAfter:\n```text\n{}\n```\n",
                sample.origin,
                get_utf8(&normalize_file(&bytes)),
                get_utf8(&normalize_file(&result.content))
            ));
        }
        if result.changes.is_empty() && !result.needs_review {
            negative_count += 1;
        }
        for change in result.changes {
            occurrences.entry(change.rule_id.clone()).or_default().push(sample.origin.clone());
            changes_tsv.push_str(&format!(
                "{}\t{}\t{}\t{:?}\t{}\t{}\n",
                sample.origin, sample.hash, change.rule_id, change.action, change.first_line, change.last_line
            ));
        }
    }
    // Check each manually selected block in isolation as well. Report-only
    // suffixes deliberately stop cleanup of earlier stacked ads.
    for &(id, origin, marker, count, _) in SELECTIONS {
        let rule = rules.description_rule.iter().find(|r| r.id == id).unwrap();
        let sample = audit.diz.iter().find(|d| d.origin.ends_with(origin)).unwrap();
        let bytes = fs::read(audit_dir.join("raw").join(&sample.hash))?;
        let raw_lines: Vec<_> = bytes.split_inclusive(|b| *b == b'\n').collect();
        let text = decoded_lines(&bytes);
        let first = text.iter().position(|l| l.contains(marker)).unwrap();
        let start = if id == "white-sands-transit" { first - 1 } else { first };
        let block = raw_lines[start..first + count].concat();
        let control = b"Unrelated original program description\r\n";
        let input = [control.as_slice(), &block].concat();
        let single = DescriptionCleaner::new(std::slice::from_ref(rule))?;
        let result = single.clean("FILE_ID.DIZ", &input, 8);
        assert_eq!(result.changes.len(), 1, "isolated positive {id}");
        assert!(!result.needs_review, "isolated review {id}");
        assert_eq!(
            result.content,
            if rule.action == RuleAction::AutoClean {
                control.to_vec()
            } else {
                input.clone()
            },
            "byte preservation {id}"
        );
        assert!(single.clean("README.TXT", &input, 8).changes.is_empty());
        let middle = [input.as_slice(), b"\r\nUnrelated final program line\r\n"].concat();
        assert!(single.clean("FILE_ID.DIZ", &middle, 8).changes.is_empty(), "middle-of-description {id}");
    }
    // Write only after every rule compilation and corpus check succeeds.
    fs::create_dir(report_dir)?;
    let mut report = format!(
        "# PCBoard DIZ advertising audit\n\nSource: {} (all eight PCBoard categories, original archives read-only).\n\n{} archives, {} nested archives; {} extracted description/dictionary occurrences.\n{} archive/read errors are listed below, so completeness is limited to readable members.\n\n{} descriptions cleaned, {} unchanged without a match; {} review cases.\n\nOnly complete suffixes are matched. Original author artwork, technical details, registration terms and support contacts are not generalized into removal rules. Report-only promotional blocks remain unchanged.\n\n",
        audit.source,
        audit.archives,
        audit.nested_archives,
        audit.diz.len(),
        audit.errors.len(),
        cleaned_count,
        negative_count,
        ambiguities.len()
    );
    for rule in &rules.description_rule {
        let matches = occurrences.get(&rule.id).cloned().unwrap_or_default();
        report.push_str(&format!(
            "## {}\n\nAction: {:?}; matched occurrences: {}.\n\n```toml\n{}\n```\n",
            rule.id,
            rule.action,
            matches.len(),
            toml::to_string_pretty(rule)?
        ));
        if matches.is_empty() {
            report.push_str("No end-to-end match in readable descriptions; may be behind another report-only marker or only historically known.\n");
        }
        for source in matches {
            report.push_str(&format!("- {source}\n"));
        }
        if let Some((_, source, _, _, _)) = SELECTIONS.iter().find(|s| s.0 == rule.id) {
            report.push_str(&format!("\nReviewed source block: {source}.\n"));
        }
    }
    report.push_str("\n## Unreadable / damaged archives\n\n");
    for error in &audit.errors {
        report.push_str(&format!("- {error}\n"));
    }
    report.push_str("\n## Matcher review cases\n\n");
    for source in ambiguities {
        report.push_str(&format!("- {source}\n"));
    }
    fs::write(report_dir.join("README.md"), report)?;
    fs::write(report_dir.join("matches.tsv"), changes_tsv)?;
    fs::write(report_dir.join("before-after.md"), changed_text)?;
    let additions = Rules {
        description_rule: rules.description_rule.iter().filter(|r| !original_ids.contains(&r.id)).cloned().collect(),
    };
    if !additions.description_rule.is_empty() {
        fs::write(
            defaults,
            format!(
                "{default_text}\n\n# Reviewed PCBoard release DIZ transit advertisements; see pcboard_diz_audit/README.md.\n{}",
                toml::to_string_pretty(&additions)?
            ),
        )?;
    }
    let missing = Rules {
        description_rule: rules
            .description_rule
            .iter()
            .filter(|r| !corpus_rules.description_rule.iter().any(|old| old.id == r.id))
            .cloned()
            .collect(),
    };
    if !missing.description_rule.is_empty() {
        fs::write(
            corpus,
            format!(
                "{corpus_text}\n\n# Reviewed PCBoard release DIZ rules, including the previously shipped footers.\n{}",
                toml::to_string_pretty(&missing)?
            ),
        )?;
    }
    println!(
        "{} curated rules; {} changed descriptions; {} negative controls; {} new rules appended to defaults / {} to corpus",
        rules.description_rule.len(),
        cleaned_count,
        negative_count,
        additions.description_rule.len(),
        missing.description_rule.len()
    );
    Ok(())
}
