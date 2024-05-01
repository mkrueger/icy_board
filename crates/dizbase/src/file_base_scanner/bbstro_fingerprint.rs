use bstr::ByteSlice;
use icy_net::crc::get_crc32;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};
use walkdir::WalkDir;

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
    secial_finger_prints: Vec<Fingerprint>,
}

impl FingerprintData {
    fn update_crcs(&mut self) {
        self.crcs = HashSet::from_iter(self.finger_prints.iter().map(|f| f.crc));
        for f in &self.finger_prints {
            if !f.keywords.is_empty() {
                self.secial_finger_prints.push(f.clone());
            }
        }
    }
    pub fn load<P: AsRef<Path>>(path: &P) -> crate::Result<Self> {
        match fs::read_to_string(path) {
            Ok(txt) => match toml::from_str::<FingerprintData>(&txt) {
                Ok(mut result) => {
                    result.update_crcs();
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
            println!("scan {}…", entry.path().display());
            let data = fs::read(entry.path())?;
            let Some(file_name) = entry.path().file_name().unwrap().to_str() else {
                println!("no utf8 file name.");
                continue;
            };
            let fingerprint = Fingerprint::new(file_name.to_string(), get_crc32(&data), data.len() as u64);

            finger_prints.push(fingerprint);
        }

        let crcs = HashSet::from_iter(finger_prints.iter().map(|f| f.crc));

        Ok(Self {
            finger_prints,
            crcs,
            secial_finger_prints: Vec::new(),
        })
    }

    pub fn is_match(&self, path: &Path, content: &[u8]) -> bool {
        let crc = get_crc32(content);
        if self.crcs.contains(&crc) {
            return true;
        }
        for f in &self.secial_finger_prints {
            if f.keywords.is_empty() {
                continue;
            }
            let re = Regex::new(&f.pattern).unwrap();
            if !re.is_match(&path.file_name().unwrap().to_string_lossy()) {
                continue;
            }

            for keyword in &f.keywords {
                if !content.contains_str(keyword.as_bytes()) {
                    continue;
                }
            }
            return true;
        }
        false
    }
}
