//! Offline, copy-only corpus exercise of the upload processor and publisher.
//! Usage: upload_corpus prepare|process|finish BOARD SOURCE RULES
//! `prepare` durably records SHA-256/size baselines before copying or processing.
//! `process` repacks into quarantine; `finish` batch-scans, publishes clean
//! results, verifies the untouched source and writes comparable-pair metrics.
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};

use icy_board_engine::{
    Res,
    icy_board::{
        file_directory::{DirectoryList, FileDirectory},
        icb_config::{UploadProcessingConfig, UploadPublishPolicy},
        lock::BoardLock,
        upload_processor::UploadProcessor,
        upload_publish::publish_quarantine_record,
        upload_quarantine::{QuarantineStatus, UploadQuarantine},
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
struct Input {
    relative: PathBuf,
    area: PathBuf,
    bytes: u64,
    sha256: String,
}

#[derive(Serialize, Deserialize)]
struct Baseline {
    source: PathBuf,
    areas: Vec<PathBuf>,
    files: Vec<Input>,
    excluded_root_files: Vec<Input>,
}

struct BoardLog(Mutex<File>);
impl log::Log for BoardLog {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            let mut file = self.0.lock().unwrap();
            // Escape even diagnostics produced by archive libraries.
            writeln!(
                file,
                "{} {} [upload-corpus] {:?}",
                chrono::Local::now().to_rfc3339(),
                record.level(),
                record.args().to_string()
            )
            .unwrap();
            file.flush().unwrap();
        }
    }
    fn flush(&self) {
        self.0.lock().unwrap().flush().unwrap();
    }
}

fn hash(path: &Path) -> Res<String> {
    let mut input = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write_new(path: &Path, text: &str) -> Res<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn config(board: &Path, run: &Path) -> UploadProcessingConfig {
    let mut config = UploadProcessingConfig {
        publish_policy: UploadPublishPolicy::ManualApproval,
        quarantine_path: run.join("quarantine"),
        advertisement_rules: board.join("upload_ad_rules.toml"),
        remove_advertisements: true,
        repack_to_zip: true,
        compression_level: 9,
        ..Default::default()
    };
    // No publication here: finish() must obtain an explicit ClamAV verdict.
    config.scanner.enabled = false;
    config
}

fn prepare(board: &Path, source: &Path, rules: &Path, run: &Path) -> Res<()> {
    fs::create_dir(run)?; // Refuse to overwrite an earlier experiment.
    let mut baseline = Baseline {
        source: source.to_path_buf(),
        areas: Vec::new(),
        files: Vec::new(),
        excluded_root_files: Vec::new(),
    };
    for entry in walkdir::WalkDir::new(source).min_depth(1).sort_by_file_name() {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?.to_path_buf();
        if entry.file_type().is_symlink() {
            return Err(format!("symlink refused: {relative:?}").into());
        }
        if entry.file_type().is_dir() {
            baseline.areas.push(relative);
        } else if entry.file_type().is_file() {
            let area = relative.parent().ok_or("missing area")?.to_path_buf();
            let input = Input {
                relative,
                area,
                bytes: entry.metadata()?.len(),
                sha256: hash(entry.path())?,
            };
            if input.area.as_os_str().is_empty() {
                println!("EXCLUDED ROOT FILE: {:?} (not in a requested source directory)", input.relative);
                baseline.excluded_root_files.push(input);
            } else {
                baseline.files.push(input);
            }
        }
    }
    write_new(&run.join("baseline.toml"), &toml::to_string(&baseline)?)?;
    println!(
        "BASELINE: {} areas, {} files, {} bytes",
        baseline.areas.len(),
        baseline.files.len(),
        baseline.files.iter().map(|f| f.bytes).sum::<u64>()
    );

    let conference_path = board.join("main/conferences.toml");
    let config_path = board.join("icboard.toml");
    let old_conferences = fs::read_to_string(&conference_path)?;
    let old_config = fs::read_to_string(&config_path)?;
    write_new(&run.join("conferences.before.toml"), &old_conferences)?;
    write_new(&run.join("icboard.before.toml"), &old_config)?;
    let mut conferences: toml::Value = toml::from_str(&old_conferences)?;
    let conference = conferences
        .get_mut("conference")
        .and_then(|v| v.as_array_mut())
        .and_then(|v| v.get_mut(1))
        .ok_or("conference 1 missing")?;
    let old_dirs = conference.get("dir_file").and_then(|v| v.as_str()).ok_or("directory list missing")?;
    let mut dirs: DirectoryList = toml::from_str(&fs::read_to_string(board.join(old_dirs))?)?;
    for (i, area) in baseline.areas.iter().enumerate() {
        let area_root = run.join("areas").join(format!("{:03}", i + 1));
        fs::create_dir_all(area_root.join("files"))?;
        fs::create_dir_all(area_root.join("metadata"))?;
        dirs.push(FileDirectory {
            name: format!("Archives: {}", area.display()),
            path: area_root.join("files"),
            metadata_path: area_root.join("metadata"),
            ..Default::default()
        });
    }
    write_new(&run.join("directories.toml"), &toml::to_string_pretty(&dirs)?)?;
    conference["dir_file"] = toml::Value::String(run.join("directories.toml").to_string_lossy().into_owned());
    // The inherited static General-only menu would hide the added areas.
    let mut menu = String::from("\r\nFile areas - archive corpus test\r\n\r\n");
    for (index, area) in dirs.iter().enumerate() {
        menu.push_str(&format!("{:3}. {}\r\n", index + 1, area.name));
    }
    write_new(&run.join("directories.pcb"), &menu)?;
    conference["dir_menu"] = toml::Value::String(run.join("directories.pcb").to_string_lossy().into_owned());
    if board.join("upload_ad_rules.toml").exists() {
        fs::copy(board.join("upload_ad_rules.toml"), run.join("rules.before.toml"))?;
    }
    fs::copy(rules, board.join("upload_ad_rules.toml"))?;
    fs::copy(rules, run.join("rules.used.toml"))?;
    let mut board_config: toml::Value = toml::from_str(&old_config)?;
    let mut processing = config(board, run);
    processing.publish_policy = UploadPublishPolicy::AfterProcessing;
    processing.scanner.enabled = true;
    board_config["upload_processing"] = toml::Value::try_from(processing)?;
    fs::write(&conference_path, toml::to_string_pretty(&conferences)?)?;
    fs::write(&config_path, toml::to_string_pretty(&board_config)?)?;

    let quarantine = UploadQuarantine::new(run.join("quarantine"));
    let mut mapping = OpenOptions::new().write(true).create_new(true).open(run.join("mapping.tsv"))?;
    for (index, input) in baseline.files.iter().enumerate() {
        let i = baseline.areas.iter().position(|area| area == &input.area).ok_or("area missing")?;
        let area_root = run.join("areas").join(format!("{:03}", i + 1));
        let copy = tempfile::NamedTempFile::new_in(run)?;
        fs::copy(source.join(&input.relative), copy.path())?;
        if hash(copy.path())? != input.sha256 {
            return Err("source changed during copy".into());
        }
        let record = quarantine.enqueue(
            copy.path(),
            input.relative.file_name().ok_or("filename missing")?.to_string_lossy().into_owned(),
            area_root.join("files"),
            area_root.join("metadata"),
            "Archive corpus test".into(),
            vec![],
        )?;
        writeln!(mapping, "{index}\t{}", record.id)?;
        mapping.flush()?;
    }
    mapping.sync_all()?;
    write_new(&run.join("prepared"), "complete\n")?;
    println!("PREPARED: original corpus untouched; copies in quarantine; conference 1 configured");
    Ok(())
}

fn mapping(run: &Path) -> Res<Vec<(usize, String)>> {
    fs::read_to_string(run.join("prepared"))?;
    fs::read_to_string(run.join("mapping.tsv"))?
        .lines()
        .map(|line| {
            let (index, id) = line.split_once('\t').ok_or("bad mapping")?;
            Ok((index.parse()?, id.into()))
        })
        .collect()
}

async fn process(board: &Path, run: &Path) -> Res<()> {
    let config = config(board, run);
    let quarantine = UploadQuarantine::new(config.quarantine_path.clone());
    let processor = UploadProcessor::new(config);
    for (n, (_, id)) in mapping(run)?.iter().enumerate() {
        if quarantine.load(id)?.status == QuarantineStatus::Pending {
            processor.process(id).await?;
        }
        if n % 250 == 0 {
            println!("PROCESSED {n}");
        }
    }
    write_new(&run.join("processed"), "complete\n")?;
    Ok(())
}

fn scan_verdict(line: &str) -> Option<(&str, bool)> {
    if let Some(path) = line.strip_suffix(": OK") {
        return Some((path, true));
    }
    let (path, message) = line.rsplit_once(": ")?;
    if message.ends_with(" FOUND") || message.ends_with(" ERROR") {
        Some((path, false))
    } else {
        None
    }
}

fn finish(run: &Path) -> Res<()> {
    fs::read_to_string(run.join("processed"))?;
    let baseline: Baseline = toml::from_str(&fs::read_to_string(run.join("baseline.toml"))?)?;
    let mapping = mapping(run)?;
    let quarantine = UploadQuarantine::new(run.join("quarantine"));
    let mut file_list = String::new();
    for (_, id) in &mapping {
        let record = quarantine.load(id)?;
        if record.status == QuarantineStatus::Published {
            return Err("finish already started: inspect records before retry".into());
        }
        let path = quarantine.payload_path(&record);
        if path.to_string_lossy().contains(['\n', '\r']) {
            return Err("newline in scanner path".into());
        }
        file_list.push_str(&format!("{}\n", path.display()));
    }
    write_new(&run.join("scan-files.txt"), &file_list)?;
    let output = OpenOptions::new().write(true).create_new(true).open(run.join("clamav.log"))?;
    let version = Command::new("clamscan").arg("--version").output()?;
    write_new(&run.join("clamav-version.txt"), &String::from_utf8_lossy(&version.stdout))?;
    println!("Scanning {} processed/quarantined files with ClamAV", mapping.len());
    let status = Command::new("clamscan")
        .args([
            "--no-summary",
            "--stdout",
            "--alert-exceeds-max=yes",
            "--max-filesize=512M",
            "--max-scansize=2048M",
            "--max-recursion=32",
        ])
        .arg(format!("--file-list={}", run.join("scan-files.txt").display()))
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .status()?;
    let scan_text = fs::read_to_string(run.join("clamav.log"))?;
    let mut verdicts = HashMap::new();
    for line in scan_text.lines() {
        if let Some((path, clean)) = scan_verdict(line) {
            let entry = verdicts.entry(path.to_string()).or_insert((true, String::new()));
            entry.0 &= clean;
            if !clean {
                entry.1 = line.to_string();
            }
        }
    }
    let scan_ok = matches!(status.code(), Some(0 | 1));
    let mut outcomes = OpenOptions::new().write(true).create_new(true).open(run.join("outcomes.tsv"))?;
    writeln!(outcomes, "source\tstatus\tbefore_bytes\tafter_bytes\tid\treport")?;
    let mut totals: BTreeMap<PathBuf, (u64, u64, u64, u64)> = baseline.areas.iter().map(|a| (a.clone(), (0, 0, 0, 0))).collect();
    for (index, id) in &mapping {
        let input = &baseline.files[*index];
        let mut record = quarantine.load(id)?;
        let payload = quarantine.payload_path(&record).to_string_lossy().into_owned();
        let verdict = verdicts.get(&payload);
        let clean = scan_ok && verdict.is_some_and(|v| v.0);
        let message = if clean {
            "batch virus scanner: clean".to_string()
        } else {
            format!(
                "batch virus scanner: blocked: {}",
                verdict
                    .map(|v| v.1.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("missing verdict or scanner operational failure")
            )
        };
        record.processing_report.push(message.clone());
        quarantine.save(&record)?;
        if !clean {
            log::warn!("upload {:?} id {:?}: {:?}", input.relative, id, message);
            if record.status == QuarantineStatus::AwaitingApproval {
                record = quarantine.transition(id, &[QuarantineStatus::AwaitingApproval], QuarantineStatus::NeedsReview, "corpus", &message)?;
            }
        } else if record.status == QuarantineStatus::AwaitingApproval {
            match publish_quarantine_record(&quarantine, id, "corpus", "ClamAV batch clean; test corpus publication") {
                Ok(published) => record = published,
                Err(error) => {
                    log::warn!("publication {:?}: {:?}", input.relative, error.to_string());
                    record = quarantine.load(id)?;
                }
            }
        }
        let after = if record.status == QuarantineStatus::Published {
            fs::metadata(record.destination.join(&record.original_name))?.len()
        } else {
            fs::metadata(quarantine.payload_path(&record))?.len()
        };
        let total = totals.get_mut(&input.area).unwrap();
        if record.status == QuarantineStatus::Published {
            total.0 += 1;
            total.1 += input.bytes;
            total.2 += after;
        } else {
            total.3 += 1;
        }
        writeln!(
            outcomes,
            "{:?}\t{:?}\t{}\t{}\t{}\t{:?}",
            input.relative,
            record.status,
            input.bytes,
            after,
            id,
            record.processing_report.join("; ")
        )?;
        outcomes.flush()?;
    }
    outcomes.sync_all()?;
    println!("Verifying original SHA-256 baseline");
    for input in baseline.files.iter().chain(&baseline.excluded_root_files) {
        let source = baseline.source.join(&input.relative);
        if fs::metadata(&source)?.len() != input.bytes || hash(&source)? != input.sha256 {
            return Err(format!("SOURCE CHANGED: {:?}", input.relative).into());
        }
    }
    let mut report = String::from(
        "# Upload corpus results\n\nDeflate level 9. Savings combine advertisement/comment cleanup and recompression.\nOnly published before/after pairs are included; retained quarantine data and catalog overhead are excluded.\nClamAV signatures were old; a clean result does not establish current malware freedom.\nOriginal source sizes and SHA-256 verified unchanged after processing.\n\n| Area | Published | Review | Before bytes | After bytes | Savings |\n|---|---:|---:|---:|---:|---:|\n",
    );
    let mut grand = (0, 0, 0, 0);
    for (area, t) in totals {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.2}% |\n",
            area.display(),
            t.0,
            t.3,
            t.1,
            t.2,
            savings(t.1, t.2)
        ));
        grand.0 += t.0;
        grand.1 += t.1;
        grand.2 += t.2;
        grand.3 += t.3;
    }
    report.push_str(&format!(
        "| **TOTAL** | {} | {} | {} | {} | **{:.2}%** |\n",
        grand.0,
        grand.3,
        grand.1,
        grand.2,
        savings(grand.1, grand.2)
    ));
    write_new(&run.join("report.md"), &report)?;
    println!(
        "DONE: published {}, review {}, before {}, after {}, savings {:.2}%",
        grand.0,
        grand.3,
        grand.1,
        grand.2,
        savings(grand.1, grand.2)
    );
    Ok(())
}

fn savings(before: u64, after: u64) -> f64 {
    if before == 0 {
        0.0
    } else {
        (before as f64 - after as f64) * 100.0 / before as f64
    }
}

#[tokio::main]
async fn main() -> Res<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: upload_corpus prepare|process|finish BOARD SOURCE RULES".into());
    }
    let board = fs::canonicalize(&args[2])?;
    let source = fs::canonicalize(&args[3])?;
    let rules = fs::canonicalize(&args[4])?;
    if board.starts_with(&source) || source.starts_with(&board) {
        return Err("board and source must be separate".into());
    }
    let _lock = BoardLock::acquire(&board)?;
    let logger = Box::leak(Box::new(BoardLog(Mutex::new(
        OpenOptions::new().append(true).create(true).open(board.join("icboard.log"))?,
    ))));
    log::set_logger(logger).map_err(|error| error.to_string())?;
    log::set_max_level(log::LevelFilter::Info);
    let run = board.join("upload-corpus");
    match args[1].as_str() {
        "prepare" => prepare(&board, &source, &rules, &run),
        "process" => process(&board, &run).await,
        "finish" => finish(&run),
        _ => Err("unknown phase".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_requires_an_explicit_clean_verdict() {
        assert_eq!(scan_verdict("/tmp/a: OK"), Some(("/tmp/a", true)));
        assert_eq!(scan_verdict("/tmp/a: Test.Signature FOUND"), Some(("/tmp/a", false)));
        assert_eq!(scan_verdict("/tmp/a: Can't read ERROR"), Some(("/tmp/a", false)));
        assert_eq!(scan_verdict("LibClamAV Warning: old database"), None);
        assert_eq!(scan_verdict("/tmp/a: scan incomplete"), None);
    }

    #[test]
    fn savings_include_growth_and_empty_input() {
        assert_eq!(savings(100, 75), 25.0);
        assert_eq!(savings(100, 125), -25.0);
        assert_eq!(savings(0, 0), 0.0);
    }
}
