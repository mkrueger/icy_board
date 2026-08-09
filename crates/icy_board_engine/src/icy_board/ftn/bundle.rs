use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{Datelike, NaiveDateTime, Weekday};
use jamjam::util::echmoail::EchomailAddress;
use thiserror::Error;
use zip::write::SimpleFileOptions;

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Two characters of the weekday the bundle was made on, followed by one
/// character that only has to make the name unique for that day.
const WEEKDAYS: [&str; 7] = ["mo", "tu", "we", "th", "fr", "sa", "su"];

/// A bundle for the same day and link has to be told apart somehow, and the
/// convention is a single digit followed by the lower case letters.
const COUNTERS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

#[derive(Error, Debug)]
pub enum BundleError {
    #[error("No free bundle name left for today")]
    NoFreeName,

    #[error("The bundle carries a name that is not a name: {0}")]
    UnsafeName(String),
}

/// Bundles are named after the distance between the two systems, which is what
/// keeps the mail for one link apart from the mail for the next.
pub fn bundle_stem(from: &EchomailAddress, to: &EchomailAddress) -> String {
    format!("{:04x}{:04x}", to.net.wrapping_sub(from.net), to.node.wrapping_sub(from.node))
}

pub fn bundle_extension(weekday: Weekday, counter: usize) -> Option<String> {
    let day = WEEKDAYS.get(weekday.num_days_from_monday() as usize)?;
    let counter = *COUNTERS.get(counter)? as char;
    Some(format!("{}{}", day, counter))
}

/// The first name of the day that nobody has taken yet.
pub fn next_bundle(directory: &Path, from: &EchomailAddress, to: &EchomailAddress, when: &NaiveDateTime) -> Res<PathBuf> {
    let stem = bundle_stem(from, to);
    for counter in 0..COUNTERS.len() {
        let Some(extension) = bundle_extension(when.weekday(), counter) else {
            break;
        };
        let candidate = directory.join(format!("{}.{}", stem, extension));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(BundleError::NoFreeName.into())
}

/// Packets are named after the moment they were built, so that a link with more
/// than one of them waiting gets them back in the order they were written.
pub fn packet_name(when: &NaiveDateTime) -> String {
    format!("{:08x}.pkt", when.and_utc().timestamp() as u32)
}

pub fn is_bundle(name: &str) -> bool {
    let Some((_, extension)) = name.rsplit_once('.') else {
        return false;
    };
    let extension = extension.to_ascii_lowercase();
    let mut characters = extension.chars();
    let day: String = characters.by_ref().take(2).collect();
    match characters.next() {
        Some(counter) => WEEKDAYS.contains(&day.as_str()) && COUNTERS.contains(&(counter as u8)) && characters.next().is_none(),
        None => false,
    }
}

pub fn is_packet(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("pkt"))
}

/// Writes the given files into a zip archive, which is what fidonet has meant
/// by an arcmail bundle for long enough that nothing else is expected.
pub fn pack(files: &[PathBuf], into: &Path) -> Res<()> {
    if let Some(parent) = into.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut zip = zip::ZipWriter::new(File::create(into)?);
    for path in files {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return Err(BundleError::UnsafeName(path.display().to_string()).into());
        };
        zip.start_file(name, SimpleFileOptions::default())?;
        zip.write_all(&fs::read(path)?)?;
    }
    zip.finish()?;
    Ok(())
}

/// Unpacks a bundle that arrived from the network, which means every name in it
/// is a claim and not a fact.
pub fn unpack(bundle: &Path, into: &Path) -> Res<Vec<PathBuf>> {
    fs::create_dir_all(into)?;
    let mut archive = zip::ZipArchive::new(File::open(bundle)?)?;
    let mut unpacked = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = safe_name(entry.name()) else {
            log::warn!("Refusing {} out of {}", entry.name(), bundle.display());
            continue;
        };
        let path = into.join(name);
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        fs::write(&path, contents)?;
        unpacked.push(path);
    }
    Ok(unpacked)
}

/// A packet inside a bundle has no directory part, so anything that looks like
/// one is a reason to refuse the entry rather than to repair it.
fn safe_name(name: &str) -> Option<String> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\', ':']) {
        return None;
    }
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn address(text: &str) -> EchomailAddress {
        EchomailAddress::parse(text).unwrap()
    }

    fn sunday() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, 9).unwrap().and_hms_opt(14, 30, 5).unwrap()
    }

    #[test]
    fn test_a_bundle_is_named_after_the_distance_between_the_two_systems() {
        assert_eq!(bundle_stem(&address("21:1/100"), &address("21:1/1")), "0000ff9d");
        assert_eq!(bundle_stem(&address("21:1/1"), &address("21:1/100")), "00000063");
        assert_eq!(bundle_stem(&address("2:240/1120"), &address("2:280/464")), "0028fd70");
    }

    #[test]
    fn test_the_extension_says_which_day_the_bundle_was_made_on() {
        assert_eq!(bundle_extension(Weekday::Sun, 0).unwrap(), "su0");
        assert_eq!(bundle_extension(Weekday::Mon, 35).unwrap(), "moz");
        assert_eq!(bundle_extension(Weekday::Mon, 36), None);
    }

    #[test]
    fn test_a_second_bundle_on_the_same_day_gets_the_next_name() {
        let directory = tempfile::tempdir().unwrap();
        let first = next_bundle(directory.path(), &address("21:1/100"), &address("21:1/1"), &sunday()).unwrap();
        fs::write(&first, b"").unwrap();
        let second = next_bundle(directory.path(), &address("21:1/100"), &address("21:1/1"), &sunday()).unwrap();

        assert_eq!(first.file_name().unwrap(), "0000ff9d.su0");
        assert_eq!(second.file_name().unwrap(), "0000ff9d.su1");
    }

    #[test]
    fn test_what_arrives_in_the_inbound_is_told_apart_by_its_extension() {
        assert!(is_bundle("0000ff9d.su0"));
        assert!(is_bundle("0000FF9D.MO7"));
        assert!(!is_bundle("0000ff9d.su"));
        assert!(!is_bundle("0000ff9d.su00"));
        assert!(!is_bundle("0000ff9d.zip"));
        assert!(is_packet("68970a5d.pkt"));
        assert!(!is_packet("68970a5d.su0"));
    }

    #[test]
    fn test_a_bundle_gives_back_the_packets_that_went_into_it() {
        let directory = tempfile::tempdir().unwrap();
        let packet = directory.path().join("68970a5d.pkt");
        fs::write(&packet, b"not really a packet").unwrap();
        let bundle = directory.path().join("0000ff9d.su0");

        pack(&[packet], &bundle).unwrap();
        let unpacked = unpack(&bundle, &directory.path().join("in")).unwrap();

        assert_eq!(unpacked.len(), 1);
        assert_eq!(unpacked[0].file_name().unwrap(), "68970a5d.pkt");
        assert_eq!(fs::read(&unpacked[0]).unwrap(), b"not really a packet");
    }

    #[test]
    fn test_an_entry_that_climbs_out_of_the_inbound_is_left_in_the_bundle() {
        let directory = tempfile::tempdir().unwrap();
        let bundle = directory.path().join("0000ff9d.su0");
        let mut zip = zip::ZipWriter::new(File::create(&bundle).unwrap());
        zip.start_file("../escaped.pkt", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"nope").unwrap();
        zip.start_file("good.pkt", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"yes").unwrap();
        zip.finish().unwrap();

        let unpacked = unpack(&bundle, &directory.path().join("in")).unwrap();

        assert_eq!(unpacked.len(), 1);
        assert_eq!(unpacked[0].file_name().unwrap(), "good.pkt");
        assert!(!directory.path().join("escaped.pkt").exists());
    }
}
