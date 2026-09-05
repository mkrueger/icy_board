//! File echos. A file travels the network with a `.TIC` beside it that says
//! which area it belongs to, what it is called and what it should look like on
//! arrival. The file is put into the directory carrying that area and the
//! description the TIC brought becomes the one users read.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use dizbase::file_base::FileBase;
use jamjam::util::echomail::EchomailAddress;

use super::{Context, FtnConfig, freq::matches_mask};
use crate::Res;

/// One file directory of this board as the tosser sees it: the tag it carries
/// in the network, the name it is known by here and where its files live.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileArea {
    pub tag: String,
    pub name: String,
    pub path: PathBuf,
    pub metadata_path: PathBuf,
}

/// What a `.TIC` says about the file lying next to it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tic {
    pub area: String,
    pub file: String,
    pub description: String,
    pub from: Option<EchomailAddress>,
    pub origin: Option<EchomailAddress>,
    pub size: Option<u64>,
    pub crc: Option<u32>,
    pub password: String,

    /// A mask naming what this file supersedes, which is how a nodelist area
    /// stays one file long instead of growing a copy a day.
    pub replaces: String,
}

impl Tic {
    pub fn parse(text: &str) -> Self {
        let mut tic = Self::default();
        let mut long = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            let (keyword, value) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
            let value = value.trim();
            match keyword.to_ascii_uppercase().as_str() {
                "AREA" => tic.area = value.to_string(),
                "FILE" => tic.file = value.to_string(),
                "DESC" => tic.description = value.to_string(),
                "LDESC" => long.push(value.to_string()),
                "FROM" => tic.from = EchomailAddress::parse(value),
                "ORIGIN" => tic.origin = EchomailAddress::parse(value),
                "SIZE" => tic.size = value.parse().ok(),
                "CRC" => tic.crc = u32::from_str_radix(value, 16).ok(),
                "PW" => tic.password = value.to_string(),
                "REPLACES" => tic.replaces = value.to_string(),
                _ => {}
            }
        }
        // The long description is the one written for people to read.
        if !long.is_empty() {
            tic.description = long.join("\n");
        }
        tic
    }
}

/// What one run over the file echos waiting in the inbound left behind.
#[derive(Debug, Default)]
pub struct TicReport {
    /// The files that reached an area, and the area each went to.
    pub arrived: Vec<(String, String)>,

    /// Files taken out of an area because an arriving one replaced them.
    pub replaced: Vec<String>,

    /// Tags no directory here carries, and how many files came with each.
    pub unknown: BTreeMap<String, usize>,

    pub failed: Vec<(PathBuf, String)>,
}

/// Puts the files waiting in the inbound into the directories that carry their
/// area.
pub fn toss_tics(config: &FtnConfig, areas: &[FileArea]) -> Res<TicReport> {
    let mut report = TicReport::default();
    if !config.options.enabled || !config.options.process_in || !config.inbound.is_dir() {
        return Ok(report);
    }

    let mut tics = Vec::new();
    for entry in fs::read_dir(&config.inbound).context(|| format!("Cannot read the inbound {}", config.inbound.display()))? {
        let entry = entry.context(|| format!("Cannot read the inbound {}", config.inbound.display()))?;
        let path = entry.path();
        if entry.file_type()?.is_file() && path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("tic")) {
            tics.push(path);
        }
    }
    tics.sort();

    for path in tics {
        log::info!("Tossing the file echo {}", path.display());
        match toss_tic(config, areas, &path, &mut report) {
            Ok(true) => fs::remove_file(&path).context(|| format!("Cannot remove {} now that its file has been tossed", path.display()))?,
            // A file that has not arrived yet, or an area nobody here carries,
            // is worth another run rather than being thrown away.
            Ok(false) => {}
            Err(err) => report.failed.push((path, err.to_string())),
        }
    }
    Ok(report)
}

fn toss_tic(config: &FtnConfig, areas: &[FileArea], path: &Path, report: &mut TicReport) -> Res<bool> {
    let text = fs::read(path).context(|| format!("Cannot read the file echo {}", path.display()))?;
    let tic = Tic::parse(&String::from_utf8_lossy(&text));

    if tic.file.is_empty() {
        return Err(format!("{} names no file, so there is nothing to toss", path.display()).into());
    }
    // A name out of the network never becomes a path, so it cannot reach out of
    // the inbound or out of the directory it is put into.
    if !is_plain_name(&tic.file) {
        return Err(format!("{} names {}, which is not a plain file name", path.display(), tic.file).into());
    }

    let link = tic.from.and_then(|from| config.links.iter().find(|link| link.address == from));
    if let Some(from) = tic.from {
        if config.options.secure && link.is_none() {
            return Err(format!(
                "{} says it comes from {}, which is not a configured link. Add that node under Message Networking > Node Configuration, or turn off Secure Netmail to take file echos from anyone",
                path.display(),
                from
            )
            .into());
        }
        if let Some(link) = link
            && !link.tic_password.is_empty()
            && !link.tic_password.eq_ignore_ascii_case(tic.password.trim())
        {
            return Err(format!(
                "{} carries a password that is not the one tic_password names for {}. Correct it in ftn.toml, or clear it to take file echos as they come",
                path.display(),
                from
            )
            .into());
        }
    }

    let Some(area) = areas
        .iter()
        .find(|area| area.tag.eq_ignore_ascii_case(&tic.area))
        .or_else(|| areas.iter().find(|area| area.name.eq_ignore_ascii_case(&tic.area)))
    else {
        *report.unknown.entry(tic.area.to_uppercase()).or_default() += 1;
        return Ok(false);
    };

    let Some(source) = arrived_as(&config.inbound, &tic.file)? else {
        return Err(format!(
            "{} announces {}, which is not in the inbound. It is waiting for the next call, or the session that brought it was cut short",
            path.display(),
            tic.file
        )
        .into());
    };

    let size = fs::metadata(&source)
        .context(|| format!("Cannot look at {}", source.display()))?
        .len();
    if let Some(announced) = tic.size
        && announced != size
    {
        return Err(format!(
            "{} arrived with {} bytes where {} announces {}, so it came over incomplete",
            source.display(),
            size,
            path.display(),
            announced
        )
        .into());
    }
    if let Some(announced) = tic.crc {
        let found = crc32(&source)?;
        if found != announced {
            return Err(format!(
                "{} arrived with checksum {:08X} where {} announces {:08X}, so it came over damaged",
                source.display(),
                found,
                path.display(),
                announced
            )
            .into());
        }
    }

    fs::create_dir_all(&area.path).context(|| format!("Cannot create the file directory {} of {}", area.path.display(), area.name))?;
    let target = area.path.join(&tic.file);
    let metadata_path = if area.metadata_path.as_os_str().is_empty() {
        area.path.join("dir")
    } else {
        area.metadata_path.clone()
    };
    let mut base = FileBase::open(&area.path, &metadata_path).context(|| format!("Cannot open the file base of {}", area.name))?;

    if !tic.replaces.is_empty() {
        let superseded: Vec<String> = base
            .iter()
            .map(|header| header.name.clone())
            .filter(|name| !name.eq_ignore_ascii_case(&tic.file) && matches_mask(name, &tic.replaces))
            .collect();
        for name in superseded {
            let old = area.path.join(&name);
            fs::remove_file(&old).context(|| format!("Cannot remove {}, which {} replaces", old.display(), tic.file))?;
            base.remove_file(&old)
                .context(|| format!("Cannot take {} out of the file base of {}", name, area.name))?;
            report.replaced.push(name);
        }
    }

    move_file(&source, &target)?;

    // A file that came before is registered with what it looked like then.
    if base.contains_name(&tic.file) {
        base.remove_file(&target)
            .context(|| format!("Cannot take the earlier {} out of the file base of {}", tic.file, area.name))?;
    }
    base.add_file(&target, Vec::new())
        .context(|| format!("Cannot add {} to the file base of {}", tic.file, area.name))?;
    if !tic.description.is_empty() {
        base.set_description(&target, &tic.description)
            .context(|| format!("Cannot store the description of {} in the file base of {}", tic.file, area.name))?;
    }

    report.arrived.push((tic.file.clone(), area.name.clone()));
    Ok(true)
}

/// A name a TIC may bring. Anything that could reach out of the directory it
/// is meant for is refused before it is ever used.
fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 100 && !name.contains(['/', '\\', ':', '\0']) && !name.starts_with('.')
}

/// A mailer keeps the name the sending side used, and the two sides do not
/// have to agree on its case.
fn arrived_as(inbound: &Path, name: &str) -> Res<Option<PathBuf>> {
    let direct = inbound.join(name);
    if direct.is_file() {
        return Ok(Some(direct));
    }
    for entry in fs::read_dir(inbound).context(|| format!("Cannot read the inbound {}", inbound.display()))? {
        let entry = entry.context(|| format!("Cannot read the inbound {}", inbound.display()))?;
        if entry.file_name().to_string_lossy().eq_ignore_ascii_case(name) && entry.file_type()?.is_file() {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

fn crc32(path: &Path) -> Res<u32> {
    let mut file = File::open(path).context(|| format!("Cannot read {}", path.display()))?;
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).context(|| format!("Cannot read {}", path.display()))?;
        if read == 0 {
            return Ok(hasher.finalize());
        }
        hasher.update(&buffer[..read]);
    }
}

/// The inbound and the file directories may lie on different volumes, where a
/// rename cannot reach.
fn move_file(source: &Path, target: &Path) -> Res<()> {
    if fs::rename(source, target).is_ok() {
        return Ok(());
    }
    fs::copy(source, target).context(|| format!("Cannot put {} into {}", source.display(), target.display()))?;
    fs::remove_file(source).context(|| format!("Cannot remove {} now that it has been put into {}", source.display(), target.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::ftn::{FtnAka, FtnLink};

    fn address(text: &str) -> EchomailAddress {
        EchomailAddress::parse(text).unwrap()
    }

    fn config(directory: &Path) -> FtnConfig {
        FtnConfig {
            inbound: directory.join("inbound"),
            outbound: directory.join("outbound"),
            netmail: directory.join("netmail"),
            akas: vec![FtnAka {
                address: address("21:1/100"),
                domain: "fsxnet".to_string(),
            }],
            links: vec![FtnLink {
                address: address("21:1/1"),
                domain: "fsxnet".to_string(),
                host: "hub.example.org".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn area(directory: &Path, tag: &str) -> FileArea {
        FileArea {
            tag: tag.to_string(),
            name: "Nodelists".to_string(),
            path: directory.join("files").join(tag.to_lowercase()),
            metadata_path: PathBuf::new(),
        }
    }

    /// Writes a file into the inbound together with a TIC that describes it.
    fn arrive(config: &FtnConfig, name: &str, content: &[u8], lines: &str) -> PathBuf {
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join(name), content).unwrap();
        let tic = config.inbound.join("00000001.tic");
        let mut text = format!("File {name}\r\nSize {}\r\nCrc {:08X}\r\n", content.len(), crc32fast::hash(content));
        text.push_str(lines);
        fs::write(&tic, text).unwrap();
        tic
    }

    #[test]
    fn test_a_file_reaches_the_directory_carrying_its_area() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![area(directory.path(), "R24NODEL")];
        let tic = arrive(&config, "NODELR24.Z54", b"nodelist", "Area R24NODEL\r\nDesc Nodelist of the day\r\nFrom 21:1/1\r\n");

        let report = toss_tics(&config, &areas).unwrap();

        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert_eq!(report.arrived, vec![("NODELR24.Z54".to_string(), "Nodelists".to_string())]);
        assert!(!tic.exists());
        assert!(!config.inbound.join("NODELR24.Z54").exists());
        let arrived = areas[0].path.join("NODELR24.Z54");
        assert!(arrived.exists());
        let mut base = FileBase::open(&areas[0].path, areas[0].path.join("dir")).unwrap();
        assert_eq!(base.description(&arrived).unwrap().as_deref(), Some("Nodelist of the day"));
    }

    /// A directory that was never given a tag is still known by its name, which
    /// is what a sysop who named it after the echo expects.
    #[test]
    fn test_an_untagged_directory_is_found_by_its_name() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![FileArea {
            tag: String::new(),
            name: "R24NODEL".to_string(),
            ..area(directory.path(), "R24NODEL")
        }];
        arrive(&config, "NODELR24.Z54", b"nodelist", "Area R24NODEL\r\nFrom 21:1/1\r\n");

        let report = toss_tics(&config, &areas).unwrap();

        assert_eq!(report.arrived.len(), 1);
        assert!(areas[0].path.join("NODELR24.Z54").exists());
    }

    #[test]
    fn test_a_tag_no_directory_carries_leaves_both_files_alone() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let tic = arrive(&config, "NODELR24.Z54", b"nodelist", "Area R24NODEL\r\nFrom 21:1/1\r\n");

        let report = toss_tics(&config, &[]).unwrap();

        assert_eq!(report.unknown.get("R24NODEL"), Some(&1));
        assert!(tic.exists());
        assert!(config.inbound.join("NODELR24.Z54").exists());
    }

    #[test]
    fn test_a_file_that_came_over_damaged_is_not_put_into_the_directory() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![area(directory.path(), "R24NODEL")];
        arrive(&config, "NODELR24.Z54", b"nodelist", "Area R24NODEL\r\nFrom 21:1/1\r\n");
        fs::write(config.inbound.join("NODELR24.Z54"), b"nodelisT").unwrap();

        let report = toss_tics(&config, &areas).unwrap();

        assert_eq!(report.failed.len(), 1);
        assert!(report.failed[0].1.contains("damaged"), "{}", report.failed[0].1);
        assert!(!areas[0].path.join("NODELR24.Z54").exists());
    }

    #[test]
    fn test_a_file_that_has_not_arrived_yet_is_waited_for() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![area(directory.path(), "R24NODEL")];
        let tic = arrive(&config, "NODELR24.Z54", b"nodelist", "Area R24NODEL\r\nFrom 21:1/1\r\n");
        fs::remove_file(config.inbound.join("NODELR24.Z54")).unwrap();

        let report = toss_tics(&config, &areas).unwrap();

        assert_eq!(report.failed.len(), 1);
        assert!(tic.exists());
    }

    #[test]
    fn test_a_new_nodelist_takes_the_place_of_the_one_it_replaces() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![area(directory.path(), "R24NODEL")];
        fs::create_dir_all(&areas[0].path).unwrap();
        fs::write(areas[0].path.join("NODELR24.Z53"), b"yesterday").unwrap();
        arrive(&config, "NODELR24.Z54", b"nodelist", "Area R24NODEL\r\nFrom 21:1/1\r\nReplaces NODELR24.*\r\n");

        let report = toss_tics(&config, &areas).unwrap();

        assert_eq!(report.replaced, vec!["NODELR24.Z53".to_string()]);
        assert!(!areas[0].path.join("NODELR24.Z53").exists());
        assert!(areas[0].path.join("NODELR24.Z54").exists());
    }

    #[test]
    fn test_a_file_echo_with_the_wrong_password_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.links[0].tic_password = "secret".to_string();
        let areas = vec![area(directory.path(), "R24NODEL")];
        arrive(&config, "NODELR24.Z54", b"nodelist", "Area R24NODEL\r\nFrom 21:1/1\r\nPw wrong\r\n");

        let report = toss_tics(&config, &areas).unwrap();

        assert_eq!(report.failed.len(), 1);
        assert!(report.failed[0].1.contains("tic_password"), "{}", report.failed[0].1);
        assert!(!areas[0].path.join("NODELR24.Z54").exists());
    }

    #[test]
    fn test_a_file_echo_from_an_unconfigured_node_is_refused_on_a_secure_board() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = config(directory.path());
        config.options.secure = true;
        let areas = vec![area(directory.path(), "R24NODEL")];
        arrive(&config, "NODELR24.Z54", b"nodelist", "Area R24NODEL\r\nFrom 21:99/99\r\n");

        let report = toss_tics(&config, &areas).unwrap();

        assert_eq!(report.failed.len(), 1);
        assert!(!areas[0].path.join("NODELR24.Z54").exists());
    }

    #[test]
    fn test_a_tic_naming_a_path_instead_of_a_file_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let areas = vec![area(directory.path(), "R24NODEL")];
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00000001.tic"), "Area R24NODEL\r\nFile ../../secret\r\nFrom 21:1/1\r\n").unwrap();

        let report = toss_tics(&config, &areas).unwrap();

        assert_eq!(report.failed.len(), 1);
        assert!(report.failed[0].1.contains("plain file name"), "{}", report.failed[0].1);
    }

    #[test]
    fn test_the_long_description_is_what_users_read() {
        let tic = Tic::parse("Area FSX_FILES\r\nDesc short\r\nLdesc first line\r\nLdesc second line\r\n");

        assert_eq!(tic.area, "FSX_FILES");
        assert_eq!(tic.description, "first line\nsecond line");
    }
}
