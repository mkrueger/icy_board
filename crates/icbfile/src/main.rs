use std::{
    fs,
    path::{Path, PathBuf},
    process::exit,
};

use argh::FromArgs;
use dizbase::file_base::{FileBase, file_header::FileHeader};
use icy_board_engine::icy_board::{IcyBoardSerializer, file_directory::DirectoryList};

mod listing;

use listing::{Entry, format_files_bbs, parse_files_bbs, parse_pcboard_dir};

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(FromArgs)]
/// Convert and maintain icy_board file bases
struct Cli {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    Areas(Areas),
    List(List),
    Scan(Scan),
    Check(Check),
    Import(Import),
    Export(Export),
    Set(Set),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "areas")]
/// list the areas defined in a file_areas.toml
struct Areas {
    #[argh(positional)]
    /// path to the area list
    areas: PathBuf,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "list")]
/// list the files in an area with their descriptions
struct List {
    #[argh(positional)]
    /// a file directory, or a file_areas.toml together with --area
    target: PathBuf,

    #[argh(option, short = 'a')]
    /// area name or index when the target is a file_areas.toml
    area: Option<String>,

    #[argh(switch, short = 'l')]
    /// show size, date and download count as well
    long: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "scan")]
/// derive descriptions from the archives in an area
struct Scan {
    #[argh(positional)]
    /// a file directory, or a file_areas.toml together with --area
    target: PathBuf,

    #[argh(option, short = 'a')]
    /// area name or index when the target is a file_areas.toml
    area: Option<String>,

    #[argh(switch, short = 'f')]
    /// re-derive descriptions that were imported or edited by hand
    force: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "check")]
/// report entries whose file is missing or whose size no longer matches
struct Check {
    #[argh(positional)]
    /// a file directory, or a file_areas.toml together with --area
    target: PathBuf,

    #[argh(option, short = 'a')]
    /// area name or index when the target is a file_areas.toml
    area: Option<String>,

    #[argh(switch)]
    /// drop entries whose file is gone
    prune: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "import")]
/// read descriptions from PCBoard DIR listings or FILES.BBS files
struct Import {
    #[argh(positional)]
    /// a file directory, or a file_areas.toml together with --area
    target: PathBuf,

    #[argh(positional)]
    /// the listings to read
    listings: Vec<PathBuf>,

    #[argh(option, short = 'a')]
    /// area name or index when the target is a file_areas.toml
    area: Option<String>,

    #[argh(option, short = 'f', default = "Format::Auto")]
    /// listing format: auto, pcboard or filesbbs
    format: Format,

    #[argh(switch, short = 'n')]
    /// report what would change without writing anything
    dry_run: bool,

    #[argh(switch)]
    /// replace descriptions that are already there
    overwrite: bool,

    #[argh(switch)]
    /// also import entries whose file is not in the directory
    keep_missing: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "export")]
/// write the descriptions of an area back out as a FILES.BBS
struct Export {
    #[argh(positional)]
    /// a file directory, or a file_areas.toml together with --area
    target: PathBuf,

    #[argh(option, short = 'a')]
    /// area name or index when the target is a file_areas.toml
    area: Option<String>,

    #[argh(option, short = 'o')]
    /// write to this file instead of stdout, encoded as cp437
    output: Option<PathBuf>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
/// change the description or the flags of a single file
struct Set {
    #[argh(positional)]
    /// a file directory, or a file_areas.toml together with --area
    target: PathBuf,

    #[argh(positional)]
    /// the file to change
    file: String,

    #[argh(option, short = 'a')]
    /// area name or index when the target is a file_areas.toml
    area: Option<String>,

    #[argh(option, short = 'd')]
    /// the new description
    desc: Option<String>,

    #[argh(option)]
    /// download costs no time: true or false
    free: Option<bool>,

    #[argh(option)]
    /// file cannot be downloaded: true or false
    locked: Option<bool>,
}

enum Format {
    Auto,
    PcBoard,
    FilesBbs,
}

impl std::str::FromStr for Format {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Format::Auto),
            "pcboard" | "dir" => Ok(Format::PcBoard),
            "filesbbs" | "files.bbs" | "bbs" => Ok(Format::FilesBbs),
            _ => Err(format!("unknown format '{}', expected auto, pcboard or filesbbs", s)),
        }
    }
}

fn main() {
    reset_sigpipe();
    let cli: Cli = argh::from_env();
    if let Err(err) = run(cli) {
        eprintln!("error: {}", err);
        exit(1);
    }
}

/// Rust ignores SIGPIPE, which turns `icbfile list | head` into a panic.
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn run(cli: Cli) -> Res<()> {
    match cli.command {
        Command::Areas(cmd) => areas(&cmd.areas),
        Command::List(cmd) => list(open(&cmd.target, &cmd.area)?, cmd.long),
        Command::Scan(cmd) => scan(open(&cmd.target, &cmd.area)?, cmd.force),
        Command::Check(cmd) => check(open(&cmd.target, &cmd.area)?, cmd.prune),
        Command::Import(cmd) => import(&cmd),
        Command::Export(cmd) => export(open(&cmd.target, &cmd.area)?, cmd.output.as_deref()),
        Command::Set(cmd) => set(&cmd),
    }
}

/// Resolves either a plain directory or one area out of a `file_areas.toml`, so that the
/// database ends up exactly where the board expects to find it.
fn open(target: &Path, area: &Option<String>) -> Res<FileBase> {
    if target.is_dir() {
        if area.is_some() {
            return Err("--area only applies when the target is a file_areas.toml".into());
        }
        return FileBase::open(target, target.join("dir"));
    }
    if !target.is_file() {
        return Err(format!("{} is neither a directory nor a file", target.display()).into());
    }

    let list = DirectoryList::load(&target).map_err(|err| format!("can't read {}: {}", target.display(), err))?;
    let Some(selector) = area else {
        return Err(format!("{} is an area list, pick one of its areas with --area", target.display()).into());
    };
    let directory = select_area(&list, selector)?;
    let base = target.parent().unwrap_or(Path::new("."));
    let path = resolve(base, &directory.path);
    let metadata_path = if directory.metadata_path.as_os_str().is_empty() {
        path.join("dir")
    } else {
        resolve(base, &directory.metadata_path)
    };
    FileBase::open(&path, metadata_path)
}

fn select_area<'a>(list: &'a DirectoryList, selector: &str) -> Res<&'a icy_board_engine::icy_board::file_directory::FileDirectory> {
    if let Ok(index) = selector.parse::<usize>() {
        return list
            .get(index)
            .ok_or_else(|| format!("no area with index {}, the list has {}", index, list.len()).into());
    }
    list.iter()
        .find(|area| area.name.eq_ignore_ascii_case(selector))
        .ok_or_else(|| format!("no area named '{}'", selector).into())
}

fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { base.join(path) }
}

fn areas(path: &Path) -> Res<()> {
    let list = DirectoryList::load(&path).map_err(|err| format!("can't read {}: {}", path.display(), err))?;
    for (index, area) in list.iter().enumerate() {
        println!("{:>3}  {:<35} {}", index, area.name, area.path.display());
    }
    Ok(())
}

fn list(mut base: FileBase, long: bool) -> Res<()> {
    let headers: Vec<FileHeader> = base.to_vec();
    for header in &headers {
        let path = base.full_path(header);
        let description = base.description(&path)?.unwrap_or_default();
        if long {
            println!(
                "{:<12} {:>10} {} {:>5} dl {}",
                header.name,
                header.size,
                header.date.format("%Y-%m-%d"),
                header.dl_counter,
                if path.exists() { "" } else { "MISSING" }
            );
        } else {
            println!("{:<12} {:>10}  {}", header.name, header.size, header.date.format("%m-%d-%y"));
        }
        for line in description.lines() {
            println!("             | {}", line);
        }
    }
    println!("\n{} file(s)", headers.len());
    Ok(())
}

fn scan(mut base: FileBase, force: bool) -> Res<()> {
    let headers: Vec<FileHeader> = base.to_vec();
    let mut described = 0;
    for header in &headers {
        let path = base.full_path(header);
        if !path.exists() {
            continue;
        }
        base.rescan(&path, force)?;
        if base.description(&path)?.is_some_and(|d| !d.is_empty()) {
            described += 1;
        }
    }
    println!("scanned {} file(s), {} have a description", headers.len(), described);
    Ok(())
}

fn check(mut base: FileBase, prune: bool) -> Res<()> {
    let headers: Vec<FileHeader> = base.to_vec();
    let mut missing = Vec::new();
    for header in &headers {
        let path = base.full_path(header);
        if !path.exists() {
            missing.push(header.name.clone());
            continue;
        }
        let (size, _) = (fs::metadata(&path)?.len(), ());
        if size != header.size {
            println!("size changed: {} is {} bytes, listed as {}", header.name, size, header.size);
        }
    }

    for name in &missing {
        println!("missing: {}", name);
        if prune {
            base.remove_file(&base.dir().join(name))?;
        }
    }
    println!(
        "{} file(s), {} missing{}",
        headers.len(),
        missing.len(),
        if prune && !missing.is_empty() { " (removed)" } else { "" }
    );
    Ok(())
}

fn import(cmd: &Import) -> Res<()> {
    if cmd.listings.is_empty() {
        return Err("no listing given".into());
    }
    let mut base = open(&cmd.target, &cmd.area)?;
    let mut imported = 0;
    let mut skipped = 0;
    let mut unknown = 0;

    for listing in &cmd.listings {
        let data = fs::read(listing).map_err(|err| format!("can't read {}: {}", listing.display(), err))?;
        let entries = match cmd.format {
            Format::PcBoard => parse_pcboard_dir(&data),
            Format::FilesBbs => parse_files_bbs(&data),
            Format::Auto => {
                if looks_like_pcboard_dir(&data) {
                    parse_pcboard_dir(&data)
                } else {
                    parse_files_bbs(&data)
                }
            }
        };
        println!("{}: {} entries", listing.display(), entries.len());

        for entry in &entries {
            let path = base.dir().join(&entry.name);
            if !path.exists() {
                unknown += 1;
                if !cmd.keep_missing {
                    println!("  skipped, not in the directory: {}", entry.name);
                    continue;
                }
                if !cmd.dry_run {
                    place_holder(&mut base, &path, entry)?;
                }
            }
            if !cmd.overwrite && base.is_authored(&path).unwrap_or(false) {
                skipped += 1;
                continue;
            }
            if cmd.dry_run {
                println!("  would set {}: {}", entry.name, first_line(&entry.description));
            } else {
                base.set_description(&path, &entry.description)?;
                apply_free(&mut base, &path, entry.free)?;
            }
            imported += 1;
        }
    }

    if cmd.dry_run {
        println!(
            "\ndry run: {} description(s) would be set, {} kept, {} not in the directory",
            imported, skipped, unknown
        );
    } else {
        base.save()?;
        println!("\nimported {} description(s), kept {}, {} not in the directory", imported, skipped, unknown);
    }
    Ok(())
}

/// An entry whose file is not on disk is recorded anyway so that the description is not
/// lost while the volume holding it is elsewhere.
fn place_holder(base: &mut FileBase, path: &Path, entry: &Entry) -> Res<()> {
    base.add_file(path, Vec::new())?;
    let name = &entry.name;
    if let Some(header) = base.iter_mut().find(|header| header.name == *name) {
        header.size = entry.size.unwrap_or(0);
        if let Some(date) = entry.date {
            header.date = date;
        }
    }
    Ok(())
}

fn apply_free(base: &mut FileBase, path: &Path, free: bool) -> Res<()> {
    if !free {
        return Ok(());
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(());
    };
    if let Some(header) = base.iter_mut().find(|header| header.name == name) {
        header.set_free(true);
    }
    Ok(())
}

/// The fixed size and date columns are what tells a DIR listing apart from a FILES.BBS.
fn looks_like_pcboard_dir(data: &[u8]) -> bool {
    let text = codepages::tables::get_utf8(data);
    let mut lines = 0;
    let mut matching = 0;
    for line in text.lines().take(50) {
        if line.trim().is_empty() {
            continue;
        }
        lines += 1;
        let chars: Vec<char> = line.chars().collect();
        if chars.get(31) == Some(&'|') {
            matching += 1;
            continue;
        }
        if chars.len() > 31 && chars[21] == ' ' && chars[22] == ' ' && chars[25] == '-' && chars[28] == '-' {
            matching += 1;
        }
    }
    lines > 0 && matching * 2 >= lines
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}

fn export(mut base: FileBase, output: Option<&Path>) -> Res<()> {
    let headers: Vec<FileHeader> = base.to_vec();
    let mut entries = Vec::new();
    for header in &headers {
        let description = base.description(&base.full_path(header))?.unwrap_or_default();
        entries.push((header.name.clone(), description));
    }
    let text = format_files_bbs(&entries);

    match output {
        Some(path) => {
            codepages::tables::write_cp437(&path, &text)?;
            println!("wrote {} entries to {}", entries.len(), path.display());
        }
        None => print!("{}", text),
    }
    Ok(())
}

fn set(cmd: &Set) -> Res<()> {
    let mut base = open(&cmd.target, &cmd.area)?;
    let path = base.dir().join(&cmd.file);
    if base.iter().all(|header| header.name != cmd.file) {
        return Err(format!("{} is not in this area", cmd.file).into());
    }

    if let Some(description) = &cmd.desc {
        base.set_description(&path, description)?;
    }
    if let Some(free) = cmd.free
        && let Some(header) = base.iter_mut().find(|header| header.name == cmd.file)
    {
        header.set_free(free);
    }
    if let Some(locked) = cmd.locked
        && let Some(header) = base.iter_mut().find(|header| header.name == cmd.file)
    {
        header.set_locked(locked);
    }
    base.save()?;
    println!("updated {}", cmd.file);
    Ok(())
}
