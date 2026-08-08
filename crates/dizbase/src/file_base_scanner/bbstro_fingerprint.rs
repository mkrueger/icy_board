use bstr::ByteSlice;
use icy_net::crc::get_crc32;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};
use walkdir::WalkDir;

/// A fingerprint whose pattern has been compiled once instead of once per file.
struct Matcher {
    pattern: Regex,
    keywords: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Fingerprint {
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pattern: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_32")]
    crc: u32,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    file_size: u64,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    keywords: Vec<String>,
}

fn is_null_64(b: impl std::borrow::Borrow<u64>) -> bool {
    *b.borrow() == 0
}

fn is_null_32(b: impl std::borrow::Borrow<u32>) -> bool {
    *b.borrow() == 0
}

impl Fingerprint {
    pub fn new(file_name: String, crc: u32, crc_file_size: u64) -> Self {
        Self {
            name: file_name,
            pattern: String::new(),
            keywords: Vec::new(),
            crc,
            file_size: crc_file_size,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct FingerprintData {
    #[serde(default)]
    #[serde(rename = "fingerprint")]
    finger_prints: Vec<Fingerprint>,

    #[serde(skip)]
    crcs: HashSet<u32>,

    #[serde(skip)]
    matchers: Vec<Matcher>,
}

impl FingerprintData {
    fn index(&mut self) {
        self.crcs = HashSet::from_iter(self.finger_prints.iter().map(|f| f.crc));
        self.matchers.clear();
        for f in &self.finger_prints {
            if f.keywords.is_empty() {
                continue;
            }
            match Regex::new(&f.pattern) {
                Ok(pattern) => self.matchers.push(Matcher {
                    pattern,
                    keywords: f.keywords.clone(),
                }),
                Err(err) => log::error!("Fingerprint '{}' has an unusable pattern: {}", f.name, err),
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.crcs.is_empty() && self.matchers.is_empty()
    }

    pub fn load<P: AsRef<Path>>(path: &P) -> crate::Result<Self> {
        match fs::read_to_string(path) {
            Ok(txt) => match toml::from_str::<FingerprintData>(&txt) {
                Ok(mut result) => {
                    result.index();
                    Ok(result)
                }
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e.into()),
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: &P) -> crate::Result<()> {
        match toml::to_string(self) {
            Ok(txt) => match fs::write(path, txt) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e.into()),
        }
    }

    pub fn scan_fingerprint_dir<P: AsRef<Path>>(path: &P) -> crate::Result<Self> {
        let mut finger_prints = Vec::new();

        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                continue;
            }
            let data = fs::read(entry.path())?;
            let Some(file_name) = entry.path().file_name().and_then(|n| n.to_str()) else {
                log::warn!("Skipping file with a non utf-8 name: {}", entry.path().display());
                continue;
            };
            let fingerprint = Fingerprint::new(file_name.to_string(), get_crc32(&data), data.len() as u64);

            finger_prints.push(fingerprint);
        }

        let mut result = Self {
            finger_prints,
            ..Default::default()
        };
        result.index();
        Ok(result)
    }

    /// Whether an archive member is one of the intros that keep travelling with the files.
    pub fn is_match(&self, name: &str, content: &[u8]) -> bool {
        if self.crcs.contains(&get_crc32(content)) {
            return true;
        }
        self.matchers
            .iter()
            .any(|m| m.pattern.is_match(name) && m.keywords.iter().all(|keyword| content.contains_str(keyword.as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(toml: &str) -> FingerprintData {
        let mut result: FingerprintData = toml::from_str(toml).unwrap();
        result.index();
        result
    }

    #[test]
    fn test_a_known_checksum_is_a_match_whatever_the_member_is_called() {
        let content = b"the same bytes as ever";
        let fingerprints = data(&format!("[[fingerprint]]\nname = \"intro\"\ncrc = {}\n", get_crc32(content)));
        assert!(fingerprints.is_match("whatever.ans", content));
    }

    #[test]
    fn test_an_unknown_file_is_left_alone() {
        let fingerprints = data("[[fingerprint]]\nname = \"intro\"\ncrc = 1\n");
        assert!(!fingerprints.is_match("readme.txt", b"nothing anyone has seen"));
    }

    #[test]
    fn test_a_pattern_alone_does_not_condemn_a_member() {
        let fingerprints = data("[[fingerprint]]\npattern = \"\\\\.ans$\"\nkeywords = [\"ACiD\"]\n");
        assert!(!fingerprints.is_match("art.ans", b"a drawing and nothing else"));
    }

    #[test]
    fn test_a_pattern_and_its_keyword_together_do() {
        let fingerprints = data("[[fingerprint]]\npattern = \"\\\\.ans$\"\nkeywords = [\"ACiD\"]\n");
        assert!(fingerprints.is_match("art.ans", b"brought to you by ACiD"));
    }

    #[test]
    fn test_every_keyword_has_to_be_there() {
        let fingerprints = data("[[fingerprint]]\npattern = \"\\\\.ans$\"\nkeywords = [\"ACiD\", \"1995\"]\n");
        assert!(!fingerprints.is_match("art.ans", b"brought to you by ACiD"));
    }

    #[test]
    fn test_a_member_the_pattern_misses_survives_its_keyword() {
        let fingerprints = data("[[fingerprint]]\npattern = \"\\\\.ans$\"\nkeywords = [\"ACiD\"]\n");
        assert!(!fingerprints.is_match("art.txt", b"brought to you by ACiD"));
    }

    #[test]
    fn test_an_unusable_pattern_does_not_take_the_others_down() {
        let fingerprints = data("[[fingerprint]]\npattern = \"[\"\nkeywords = [\"x\"]\n\n[[fingerprint]]\npattern = \"\\\\.ans$\"\nkeywords = [\"ACiD\"]\n");
        assert!(fingerprints.is_match("art.ans", b"brought to you by ACiD"));
    }
}
