use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{Context, FtnConfig, FtnLink};

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Everything waiting to be handed over, in the order the links are configured
/// and the files are named. Pcboard kept one queue file and numbered its
/// records from one, and a ppe that walks the queue counts on that order
/// holding still between two calls.
pub fn entries(config: &FtnConfig) -> Res<Vec<PathBuf>> {
    let mut files = Vec::new();
    for link in &config.links {
        let directory = config.outbound_for(link);
        if !directory.is_dir() {
            continue;
        }
        let mut waiting = Vec::new();
        for entry in fs::read_dir(&directory).context(|| format!("Cannot read the outbound {} of {}", directory.display(), link.to_5d()))? {
            let entry = entry.context(|| format!("Cannot read the outbound {} of {}", directory.display(), link.to_5d()))?;
            if entry.file_type()?.is_file() {
                waiting.push(entry.path());
            }
        }
        waiting.sort();
        files.append(&mut waiting);
    }
    Ok(files)
}

/// Puts a file into the outbound of the link that answers to an address.
pub fn add(config: &FtnConfig, address: &str, file: &Path) -> Res<PathBuf> {
    let Some(link) = config
        .links
        .iter()
        .find(|link| link.address.to_string().eq_ignore_ascii_case(address) || link.to_5d().eq_ignore_ascii_case(address))
    else {
        let known: Vec<String> = config.links.iter().map(FtnLink::to_5d).collect();
        return Err(if known.is_empty() {
            format!("Nothing can be sent to {address}: ftn.toml configures no links at all").into()
        } else {
            format!(
                "Nothing can be sent to {address}: no link answers to that address. Configured are {}",
                known.join(", ")
            )
            .into()
        });
    };
    let Some(name) = file.file_name() else {
        return Err(format!(
            "{} cannot be sent: a file to hand over needs a name, and this path ends in a directory",
            file.display()
        )
        .into());
    };
    let directory = config.outbound_for(link);
    fs::create_dir_all(&directory).context(|| format!("Cannot create the outbound {} of {}", directory.display(), link.to_5d()))?;
    let target = directory.join(name);
    fs::copy(file, &target).context(|| format!("Cannot copy {} into the outbound of {}", file.display(), link.to_5d()))?;
    Ok(target)
}

pub fn get(config: &FtnConfig, record: usize) -> Res<Option<PathBuf>> {
    let Some(index) = record.checked_sub(1) else {
        return Ok(None);
    };
    Ok(entries(config)?.get(index).cloned())
}

pub fn remove(config: &FtnConfig, record: usize) -> Res<bool> {
    let Some(file) = get(config, record)? else {
        return Ok(false);
    };
    fs::remove_file(&file).context(|| format!("Cannot remove {} from the outbound", file.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::ftn::FtnLink;
    use jamjam::util::echomail::EchomailAddress;

    fn config(directory: &Path) -> FtnConfig {
        FtnConfig {
            outbound: directory.join("outbound"),
            links: vec![
                FtnLink {
                    address: EchomailAddress::parse("21:1/1").unwrap(),
                    domain: "fsxnet".to_string(),
                    ..Default::default()
                },
                FtnLink {
                    address: EchomailAddress::parse("21:1/2").unwrap(),
                    domain: "fsxnet".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn test_a_file_is_queued_for_the_link_that_answers_to_the_address() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let file = directory.path().join("readme.txt");
        fs::write(&file, b"hello").unwrap();

        add(&config, "21:1/2@fsxnet", &file).unwrap();

        assert_eq!(entries(&config).unwrap().len(), 1);
        assert_eq!(get(&config, 1).unwrap().unwrap().file_name().unwrap(), "readme.txt");
        assert!(config.outbound.join("21.1.2/readme.txt").is_file());
    }

    #[test]
    fn test_an_address_nobody_is_linked_to_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        let file = directory.path().join("readme.txt");
        fs::write(&file, b"hello").unwrap();

        assert!(add(&config, "21:1/999", &file).is_err());
    }

    #[test]
    fn test_the_queue_is_numbered_from_one_and_the_records_can_be_deleted() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path());
        for (link, name) in [("21:1/1", "a.txt"), ("21:1/2", "b.txt")] {
            let file = directory.path().join(name);
            fs::write(&file, b"x").unwrap();
            add(&config, link, &file).unwrap();
        }

        assert!(get(&config, 0).unwrap().is_none());
        assert_eq!(get(&config, 1).unwrap().unwrap().file_name().unwrap(), "a.txt");
        assert!(remove(&config, 1).unwrap());
        assert_eq!(get(&config, 1).unwrap().unwrap().file_name().unwrap(), "b.txt");
        assert!(!remove(&config, 2).unwrap());
    }
}
