//! File requests. A node names the files it wants in a `.REQ` file, and the
//! ones it is allowed to have are put in its outbound for the next session.

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use jamjam::util::echomail::EchomailAddress;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use super::{Context, FtnConfig, FtnLink};
use crate::Res;

/// A directory a request may be answered from, and the password a node has to
/// name to reach it.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct FreqPath {
    pub path: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
}

/// A name a node can ask for that stands for a file of the sysop's choosing,
/// so the asking side never has to know what the file is really called.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct FreqMagic {
    pub name: String,
    pub file: PathBuf,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
}

/// The byte limits `PCBoard` applies to one session and one day.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FreqLimits {
    /// Zero means unlimited.
    pub session_bytes: u64,
    pub daily_bytes: u64,
}

impl Default for FreqLimits {
    fn default() -> Self {
        Self {
            session_bytes: 10 * 1024 * 1024,
            daily_bytes: 50 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DailyUsage {
    day: String,
    #[serde(default)]
    nodes: BTreeMap<String, u64>,
}

#[serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct FtnFreq {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub limits: FreqLimits,

    #[serde(rename = "path", default)]
    pub paths: Vec<FreqPath>,

    #[serde(rename = "magic", default)]
    pub magic: Vec<FreqMagic>,

    /// Nodes that get nothing, whatever they ask for.
    #[serde_as(as = "Vec<DisplayFromStr>")]
    #[serde(rename = "deny", default)]
    pub deny: Vec<EchomailAddress>,
}

/// Why a node is not getting what it asked for. The wording reaches the sysop,
/// so it says what to change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotAName,
    NeedsPassword,
    NotFound,
    SessionLimit,
    DailyLimit,
    AlreadyWaiting,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAName => write!(f, "that is a path, not a file name, and a request may only name a file"),
            Self::NeedsPassword => write!(f, "the password it came with does not open any path or magic name that offers this"),
            Self::NotFound => write!(f, "no configured FREQ path holds a file of that name"),
            Self::SessionLimit => write!(f, "the request is already at the session byte limit"),
            Self::DailyLimit => write!(f, "the node is already at its daily byte limit"),
            Self::AlreadyWaiting => write!(f, "a file of that name is already waiting in the node's outbound"),
        }
    }
}

#[derive(Debug, Default)]
pub struct FreqReport {
    /// Request files that were read.
    pub requests: usize,
    pub served: usize,
    pub bytes: u64,

    /// What was asked for and could not be sent, with the reason.
    pub refused: Vec<(String, String)>,

    /// Requests from a node that is not a configured link, by file name.
    pub unknown: Vec<String>,
    pub failed: Vec<(PathBuf, String)>,
}

/// A name a node may ask for. Anything that could reach out of a configured
/// path is refused before it is ever used.
fn is_plain_name(wanted: &str) -> bool {
    !wanted.is_empty() && wanted.len() <= 100 && !wanted.contains(['/', '\\', ':', '\0']) && wanted != "." && wanted != ".." && !wanted.starts_with('.')
}

/// `*` stands for any run of characters and `?` for one, which is what a node
/// on the other side of a request expects.
pub(super) fn matches_mask(name: &str, mask: &str) -> bool {
    let name: Vec<char> = name.to_ascii_uppercase().chars().collect();
    let mask: Vec<char> = mask.to_ascii_uppercase().chars().collect();
    let (mut n, mut m) = (0, 0);
    let (mut star, mut back) = (None, 0);
    while n < name.len() {
        if m < mask.len() && (mask[m] == '?' || mask[m] == name[n]) {
            n += 1;
            m += 1;
        } else if m < mask.len() && mask[m] == '*' {
            star = Some(m);
            back = n;
            m += 1;
        } else if let Some(position) = star {
            m = position + 1;
            back += 1;
            n = back;
        } else {
            return false;
        }
    }
    mask[m..].iter().all(|c| *c == '*')
}

impl FtnFreq {
    /// The files behind one requested name. The directory is listed and its
    /// entries are matched, so a name from the network never becomes a path.
    pub fn resolve(&self, wanted: &str, password: &str) -> Result<Vec<PathBuf>, Refusal> {
        if !is_plain_name(wanted) {
            return Err(Refusal::NotAName);
        }
        let mut locked = false;

        for magic in &self.magic {
            if !magic.name.eq_ignore_ascii_case(wanted) {
                continue;
            }
            if !magic.password.is_empty() && !magic.password.eq_ignore_ascii_case(password) {
                locked = true;
                continue;
            }
            let found = configured_mask(&magic.file);
            return if found.is_empty() { Err(Refusal::NotFound) } else { Ok(found) };
        }

        let mut found = Vec::new();
        for offered in &self.paths {
            if !offered.password.is_empty() && !offered.password.eq_ignore_ascii_case(password) {
                locked = true;
                continue;
            }
            let Ok(entries) = fs::read_dir(&offered.path) else {
                continue;
            };
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if matches_mask(&name, wanted) {
                    found.push(entry.path());
                }
            }
        }
        if found.is_empty() {
            return Err(if locked { Refusal::NeedsPassword } else { Refusal::NotFound });
        }
        found.sort();
        Ok(found)
    }

    pub fn denies(&self, node: &EchomailAddress) -> bool {
        self.deny.iter().any(|denied| denied == node)
    }
}

/// The node a request file belongs to. Fidonet names it after the net and node
/// it came from, four hex digits each.
fn requester(stem: &str) -> Option<(u16, u16)> {
    if stem.len() != 8 || !stem.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((u16::from_str_radix(&stem[..4], 16).ok()?, u16::from_str_radix(&stem[4..], 16).ok()?))
}

fn identified_requester(stem: &str) -> Option<EchomailAddress> {
    if stem.len() != 20 || !stem.starts_with('z') || &stem[5..6] != "n" || &stem[10..11] != "f" || &stem[15..16] != "p" {
        return None;
    }
    Some(EchomailAddress::new(
        u16::from_str_radix(&stem[1..5], 16).ok()?,
        u16::from_str_radix(&stem[6..10], 16).ok()?,
        u16::from_str_radix(&stem[11..15], 16).ok()?,
        u16::from_str_radix(&stem[16..20], 16).ok()?,
    ))
}

/// Renames a request received in a known session so later processing does not
/// have to trust the name the remote supplied.
pub fn identify_received(path: &Path, node: &EchomailAddress) -> Res<PathBuf> {
    let target = path.with_file_name(format!("z{:04x}n{:04x}f{:04x}p{:04x}.req", node.zone, node.net, node.node, node.point));
    if target == path {
        return Ok(target);
    }
    if target.exists() {
        let request = fs::read(path).context(|| format!("Cannot read the received request {}", path.display()))?;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&target)
            .context(|| format!("Cannot append to the request {}", target.display()))?;
        file.write_all(&request)
            .context(|| format!("Cannot append {} to {}", path.display(), target.display()))?;
        fs::remove_file(path).context(|| format!("Cannot remove the merged request {}", path.display()))?;
    } else {
        fs::rename(path, &target).context(|| format!("Cannot identify the request {} as coming from {}", path.display(), node))?;
    }
    Ok(target)
}

/// Answers the requests waiting in the inbound by putting what they asked for
/// into the outbound of the node that asked.
pub fn serve(config: &FtnConfig) -> Res<FreqReport> {
    let mut report = FreqReport::default();
    if !config.options.enabled || !config.freq.enabled || !config.inbound.is_dir() {
        return Ok(report);
    }
    let mut requests = Vec::new();
    for entry in fs::read_dir(&config.inbound).context(|| format!("Cannot read the inbound {}", config.inbound.display()))? {
        let entry = entry.context(|| format!("Cannot read the inbound {}", config.inbound.display()))?;
        let path = entry.path();
        if entry.file_type()?.is_file() && path.extension().is_some_and(|kind| kind.eq_ignore_ascii_case("req")) {
            requests.push(path);
        }
    }
    requests.sort();
    let usage_path = config.outbound.join("freq_usage.toml");
    let today = chrono::Local::now().date_naive().to_string();
    let mut usage = fs::read_to_string(&usage_path)
        .ok()
        .and_then(|text| toml::from_str::<DailyUsage>(&text).ok())
        .filter(|usage| usage.day == today)
        .unwrap_or_else(|| DailyUsage {
            day: today,
            nodes: BTreeMap::new(),
        });

    for request in requests {
        match answer(config, &request, &mut report, &mut usage) {
            Ok(true) => fs::remove_file(&request).context(|| format!("Cannot remove the request {} now that it is answered", request.display()))?,
            Ok(false) => {}
            Err(err) => report.failed.push((request.clone(), err.to_string())),
        }
    }
    if report.served > 0 {
        fs::create_dir_all(&config.outbound).context(|| format!("Cannot create the outbound {}", config.outbound.display()))?;
        fs::write(&usage_path, toml::to_string_pretty(&usage)?).context(|| format!("Cannot save FREQ usage to {}", usage_path.display()))?;
    }
    Ok(report)
}

fn answer(config: &FtnConfig, request: &Path, report: &mut FreqReport, usage: &mut DailyUsage) -> Res<bool> {
    let name = request.file_name().unwrap_or_default().to_string_lossy().to_string();
    let Some(stem) = request.file_stem().map(|stem| stem.to_string_lossy()) else {
        report.unknown.push(name);
        return Ok(false);
    };
    let link = if let Some(address) = identified_requester(&stem) {
        config.links.iter().find(|link| link.address == address)
    } else if let Some((net, node)) = requester(&stem) {
        // A legacy request file carries no zone, so net/node must identify
        // exactly one configured link before it can safely own an outbound.
        let mut links = config.links.iter().filter(|link| link.address.net == net && link.address.node == node);
        let first = links.next();
        if links.next().is_some() { None } else { first }
    } else {
        None
    };
    let Some(link) = link else {
        report.unknown.push(name);
        return Ok(false);
    };
    if config.freq.denies(&link.address) {
        report.refused.push((name, format!("{} is on the FREQ deny list", link.to_5d())));
        return Ok(true);
    }
    report.requests += 1;

    let text = fs::read_to_string(request).context(|| format!("Cannot read the request {}", request.display()))?;
    let outbound = config.outbound_for(link);

    let mut session_bytes = 0u64;
    let daily_bytes = usage.nodes.entry(link.address.to_string()).or_default();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        // A node names the password for a locked path after the file name.
        let (wanted, password) = match line.split_once(|c: char| c.is_whitespace() || c == '!') {
            Some((wanted, password)) => (wanted.trim(), password.trim()),
            None => (line, ""),
        };
        match config.freq.resolve(wanted, password) {
            Ok(found) => {
                for file in found {
                    let size = file.metadata().map_or(0, |data| data.len());
                    if exceeds(config.freq.limits.session_bytes, session_bytes, size) {
                        report.refused.push((wanted.to_string(), Refusal::SessionLimit.to_string()));
                        break;
                    }
                    if exceeds(config.freq.limits.daily_bytes, *daily_bytes, size) {
                        report.refused.push((wanted.to_string(), Refusal::DailyLimit.to_string()));
                        break;
                    }
                    fs::create_dir_all(&outbound).context(|| format!("Cannot create the outbound {}", outbound.display()))?;
                    let target = outbound.join(file.file_name().unwrap_or_default());
                    if target.exists() {
                        report.refused.push((wanted.to_string(), Refusal::AlreadyWaiting.to_string()));
                        continue;
                    }
                    fs::copy(&file, &target).context(|| format!("Cannot put {} in the outbound of {}", file.display(), link.to_5d()))?;
                    session_bytes += size;
                    *daily_bytes += size;
                    report.served += 1;
                    report.bytes += size;
                }
            }
            Err(refusal) => report.refused.push((wanted.to_string(), refusal.to_string())),
        }
    }
    Ok(true)
}

fn exceeds(limit: u64, used: u64, size: u64) -> bool {
    limit != 0 && used.saturating_add(size) > limit
}

fn configured_mask(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let Some(mask) = path.file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    if !mask.contains(['*', '?']) {
        return Vec::new();
    }
    let Some(directory) = path.parent() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| matches_mask(&entry.file_name().to_string_lossy(), mask))
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found
}

/// The name a request file carries when this board asks another node.
pub fn request_name(to: &EchomailAddress) -> String {
    format!("{:04x}{:04x}.req", to.net, to.node)
}

/// Asks `link` for the named files in the next session.
pub fn request(config: &FtnConfig, link: &FtnLink, wanted: &[String]) -> Res<PathBuf> {
    let outbound = config.outbound_for(link);
    fs::create_dir_all(&outbound).context(|| format!("Cannot create the outbound {}", outbound.display()))?;
    let path = outbound.join(request_name(&link.address));
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .context(|| format!("Cannot open the request {}", path.display()))?;
    file.write_all((wanted.join("\r\n") + "\r\n").as_bytes())
        .context(|| format!("Cannot write the request {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn freq(directory: &Path) -> FtnFreq {
        FtnFreq {
            enabled: true,
            limits: FreqLimits::default(),
            paths: vec![FreqPath {
                path: directory.join("files"),
                password: String::new(),
            }],
            magic: Vec::new(),
            deny: Vec::new(),
        }
    }

    fn offer(directory: &Path, name: &str, content: &str) {
        let files = directory.join("files");
        fs::create_dir_all(&files).unwrap();
        fs::write(files.join(name), content).unwrap();
    }

    #[test]
    fn test_a_requested_file_is_found_in_a_configured_path() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");

        let found = freq(directory.path()).resolve("READ.ME", "").unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), "READ.ME");
    }

    #[test]
    fn test_a_request_is_answered_whatever_case_it_is_written_in() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");

        assert!(freq(directory.path()).resolve("read.me", "").is_ok());
    }

    /// A name from the network must never become a path, or a node could ask
    /// for anything on the disk.
    #[test]
    fn test_a_request_cannot_climb_out_of_the_configured_paths() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");
        fs::write(directory.path().join("secret.txt"), "no").unwrap();
        let freq = freq(directory.path());

        for wanted in ["../secret.txt", "..\\secret.txt", "/etc/passwd", "C:\\secret.txt", ".."] {
            assert_eq!(freq.resolve(wanted, "").unwrap_err(), Refusal::NotAName, "{wanted}");
        }
    }

    #[test]
    fn test_a_file_next_to_the_configured_path_is_not_offered() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");
        fs::write(directory.path().join("secret.txt"), "no").unwrap();

        assert_eq!(freq(directory.path()).resolve("secret.txt", "").unwrap_err(), Refusal::NotFound);
    }

    #[test]
    fn test_a_mask_names_every_file_that_fits_it() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "A.ZIP", "1");
        offer(directory.path(), "B.ZIP", "2");
        offer(directory.path(), "C.TXT", "3");

        let found = freq(directory.path()).resolve("*.ZIP", "").unwrap();

        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_a_magic_name_stands_for_the_file_behind_it() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "LIST0905.ZIP", "1");
        let mut freq = freq(directory.path());
        freq.magic.push(FreqMagic {
            name: "FILES".to_string(),
            file: directory.path().join("files/LIST0905.ZIP"),
            password: String::new(),
        });

        let found = freq.resolve("files", "").unwrap();

        assert_eq!(found[0].file_name().unwrap(), "LIST0905.ZIP");
    }

    #[test]
    fn test_a_magic_name_can_stand_for_a_configured_file_mask() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "LIST0904.ZIP", "1");
        offer(directory.path(), "LIST0905.ZIP", "2");
        offer(directory.path(), "README.TXT", "3");
        let mut freq = freq(directory.path());
        freq.magic.push(FreqMagic {
            name: "FILES".to_string(),
            file: directory.path().join("files/LIST*.ZIP"),
            password: String::new(),
        });

        let found = freq.resolve("FILES", "").unwrap();

        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|path| path.extension().unwrap() == "ZIP"));
    }

    #[test]
    fn test_a_locked_path_only_answers_the_node_that_knows_the_password() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");
        let mut freq = freq(directory.path());
        freq.paths[0].password = "secret".to_string();

        assert_eq!(freq.resolve("READ.ME", "").unwrap_err(), Refusal::NeedsPassword);
        assert!(freq.resolve("READ.ME", "SECRET").is_ok());
    }

    #[test]
    fn test_a_request_file_names_the_node_that_wrote_it() {
        assert_eq!(requester("00010064"), Some((1, 100)));
        assert_eq!(requester("nothex11"), None);
        assert_eq!(requester("0001"), None);
    }

    #[test]
    fn test_a_received_request_is_bound_to_the_full_session_address() {
        let directory = tempfile::tempdir().unwrap();
        let offered = directory.path().join("remote.req");
        fs::write(&offered, "FILES\r\n").unwrap();
        let address = EchomailAddress::new(21, 1, 100, 3);

        let identified = identify_received(&offered, &address).unwrap();

        assert_eq!(identified.file_name().unwrap(), "z0015n0001f0064p0003.req");
        assert_eq!(identified_requester(identified.file_stem().unwrap().to_str().unwrap()), Some(address));
        assert!(!offered.exists());
    }

    #[test]
    fn test_the_name_of_a_request_says_who_it_is_for() {
        assert_eq!(request_name(&EchomailAddress::new(21, 1, 100, 0)), "00010064.req");
    }

    #[test]
    fn test_asking_twice_keeps_both_outgoing_requests() {
        let directory = tempfile::tempdir().unwrap();
        let config = FtnConfig {
            outbound: directory.path().join("outbound"),
            ..FtnConfig::default()
        };
        let link = FtnLink {
            address: EchomailAddress::new(21, 1, 1, 0),
            ..Default::default()
        };

        request(&config, &link, &["FILES".to_string()]).unwrap();
        let path = request(&config, &link, &["NODEDIFF".to_string()]).unwrap();

        assert_eq!(fs::read_to_string(path).unwrap(), "FILES\r\nNODEDIFF\r\n");
    }

    fn served(directory: &Path, wanted: &str) -> (FtnConfig, FreqReport) {
        let mut config = FtnConfig {
            inbound: directory.join("inbound"),
            outbound: directory.join("outbound"),
            freq: freq(directory),
            ..FtnConfig::default()
        };
        config.options.enabled = true;
        config.links.push(FtnLink {
            address: EchomailAddress::new(21, 1, 1, 0),
            ..Default::default()
        });
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00010001.req"), wanted).unwrap();

        let report = serve(&config).unwrap();
        (config, report)
    }

    #[test]
    fn test_an_answered_request_puts_the_file_in_the_outbound_of_the_node() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");

        let (config, report) = served(directory.path(), "READ.ME\r\n");

        assert_eq!(report.requests, 1);
        assert_eq!(report.served, 1);
        assert!(config.outbound_for(&config.links[0]).join("READ.ME").exists());
        // The request is answered once, so it does not travel again.
        assert!(!config.inbound.join("00010001.req").exists());
    }

    #[test]
    fn test_a_denied_node_gets_nothing_it_asks_for() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");
        let mut config = FtnConfig {
            inbound: directory.path().join("inbound"),
            outbound: directory.path().join("outbound"),
            freq: freq(directory.path()),
            ..FtnConfig::default()
        };
        config.options.enabled = true;
        config.freq.deny.push(EchomailAddress::new(21, 1, 1, 0));
        config.links.push(FtnLink {
            address: EchomailAddress::new(21, 1, 1, 0),
            ..Default::default()
        });
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00010001.req"), "READ.ME\r\n").unwrap();

        let report = serve(&config).unwrap();

        assert_eq!(report.served, 0);
        assert_eq!(report.refused.len(), 1);
    }

    #[test]
    fn test_a_request_stops_at_the_session_byte_limit() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "A.ZIP", "1");
        offer(directory.path(), "B.ZIP", "2");
        offer(directory.path(), "C.ZIP", "3");
        let mut config = FtnConfig {
            inbound: directory.path().join("inbound"),
            outbound: directory.path().join("outbound"),
            freq: freq(directory.path()),
            ..FtnConfig::default()
        };
        config.options.enabled = true;
        config.freq.limits.session_bytes = 2;
        config.freq.limits.daily_bytes = 0;
        config.links.push(FtnLink {
            address: EchomailAddress::new(21, 1, 1, 0),
            ..Default::default()
        });
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00010001.req"), "*.ZIP\r\n").unwrap();

        let report = serve(&config).unwrap();

        assert_eq!(report.served, 2);
        assert_eq!(report.refused[0].1, Refusal::SessionLimit.to_string());
    }

    #[test]
    fn test_separate_requests_share_the_nodes_daily_byte_limit() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "A.ZIP", "1");
        offer(directory.path(), "B.ZIP", "2");
        let mut config = FtnConfig {
            inbound: directory.path().join("inbound"),
            outbound: directory.path().join("outbound"),
            freq: freq(directory.path()),
            ..FtnConfig::default()
        };
        config.options.enabled = true;
        config.freq.limits.session_bytes = 0;
        config.freq.limits.daily_bytes = 1;
        config.links.push(FtnLink {
            address: EchomailAddress::new(21, 1, 1, 0),
            ..Default::default()
        });
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00010001.req"), "A.ZIP\r\n").unwrap();
        assert_eq!(serve(&config).unwrap().served, 1);
        fs::remove_file(config.outbound_for(&config.links[0]).join("A.ZIP")).unwrap();
        fs::write(config.inbound.join("00010001.req"), "B.ZIP\r\n").unwrap();

        let report = serve(&config).unwrap();

        assert_eq!(report.served, 0);
        assert_eq!(report.refused[0].1, Refusal::DailyLimit.to_string());
    }

    #[test]
    fn test_a_request_from_a_node_we_do_not_know_is_left_alone() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");
        let mut config = FtnConfig {
            inbound: directory.path().join("inbound"),
            outbound: directory.path().join("outbound"),
            freq: freq(directory.path()),
            ..FtnConfig::default()
        };
        config.options.enabled = true;
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00630063.req"), "READ.ME\r\n").unwrap();

        let report = serve(&config).unwrap();

        assert_eq!(report.served, 0);
        assert_eq!(report.unknown, vec!["00630063.req".to_string()]);
        assert!(config.inbound.join("00630063.req").exists());
    }

    #[test]
    fn test_an_existing_outbound_file_is_not_overwritten_by_a_request() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "new");
        let mut config = FtnConfig {
            inbound: directory.path().join("inbound"),
            outbound: directory.path().join("outbound"),
            freq: freq(directory.path()),
            ..FtnConfig::default()
        };
        config.options.enabled = true;
        config.links.push(FtnLink {
            address: EchomailAddress::new(21, 1, 1, 0),
            ..Default::default()
        });
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00010001.req"), "READ.ME\r\n").unwrap();
        let outbound = config.outbound_for(&config.links[0]);
        fs::create_dir_all(&outbound).unwrap();
        fs::write(outbound.join("READ.ME"), "already waiting").unwrap();

        let report = serve(&config).unwrap();

        assert_eq!(report.served, 0);
        assert_eq!(report.refused[0].1, Refusal::AlreadyWaiting.to_string());
        assert_eq!(fs::read_to_string(outbound.join("READ.ME")).unwrap(), "already waiting");
    }

    #[test]
    fn test_an_ambiguous_net_and_node_does_not_choose_a_zone_at_random() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");
        let mut config = FtnConfig {
            inbound: directory.path().join("inbound"),
            outbound: directory.path().join("outbound"),
            freq: freq(directory.path()),
            ..FtnConfig::default()
        };
        config.options.enabled = true;
        for zone in [1, 21] {
            config.links.push(FtnLink {
                address: EchomailAddress::new(zone, 1, 1, 0),
                ..Default::default()
            });
        }
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00010001.req"), "READ.ME\r\n").unwrap();

        let report = serve(&config).unwrap();

        assert_eq!(report.served, 0);
        assert_eq!(report.unknown, vec!["00010001.req".to_string()]);
        assert!(config.inbound.join("00010001.req").exists());
    }

    #[test]
    fn test_nothing_is_served_while_freq_is_switched_off() {
        let directory = tempfile::tempdir().unwrap();
        offer(directory.path(), "READ.ME", "hello");
        let (_, report) = served(directory.path(), "READ.ME\r\n");
        assert_eq!(report.served, 1);

        let mut config = FtnConfig {
            inbound: directory.path().join("inbound"),
            freq: freq(directory.path()),
            ..FtnConfig::default()
        };
        config.options.enabled = true;
        config.freq.enabled = false;
        fs::create_dir_all(&config.inbound).unwrap();
        fs::write(config.inbound.join("00010001.req"), "READ.ME\r\n").unwrap();

        assert_eq!(serve(&config).unwrap().requests, 0);
    }
}
