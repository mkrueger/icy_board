use bstr::ByteSlice;
use icy_net::crc::get_crc32;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, path::Path};
use walkdir::WalkDir;

use super::description_cleaner::{DescriptionBlockRule, DescriptionCleanResult, DescriptionCleaner};
use super::text_member::{TextMemberMatch, TextMemberMatcher, TextMemberRule};

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
    #[serde(skip_serializing_if = "String::is_empty")]
    sha256: String,

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
    pub fn new(file_name: String, content: &[u8]) -> Self {
        Self {
            name: file_name,
            pattern: String::new(),
            keywords: Vec::new(),
            crc: get_crc32(content),
            file_size: content.len() as u64,
            sha256: format!("{:x}", Sha256::digest(content)),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct FingerprintData {
    #[serde(default)]
    #[serde(rename = "fingerprint")]
    finger_prints: Vec<Fingerprint>,

    #[serde(default)]
    #[serde(rename = "description_rule")]
    description_rules: Vec<DescriptionBlockRule>,

    #[serde(default, rename = "text_member_rule", skip_serializing_if = "Vec::is_empty")]
    text_member_rules: Vec<TextMemberRule>,

    #[serde(skip)]
    text_member_matcher: Option<TextMemberMatcher>,

    #[serde(skip)]
    legacy_checksums: HashSet<(u32, u64)>,

    #[serde(skip)]
    sha256s: HashSet<(String, u64)>,

    #[serde(skip)]
    matchers: Vec<Matcher>,

    #[serde(skip)]
    description_cleaner: Option<DescriptionCleaner>,
}

impl FingerprintData {
    fn index(&mut self) {
        self.text_member_matcher = match TextMemberMatcher::new(&self.text_member_rules) {
            Ok(matcher) => Some(matcher),
            Err(err) => {
                log::error!("Invalid text member rules: {err}");
                None
            }
        };
        self.legacy_checksums = self
            .finger_prints
            .iter()
            .filter(|fingerprint| fingerprint.sha256.is_empty() && fingerprint.crc != 0 && fingerprint.file_size != 0)
            .map(|fingerprint| (fingerprint.crc, fingerprint.file_size))
            .collect();
        self.sha256s = self
            .finger_prints
            .iter()
            .filter(|fingerprint| !fingerprint.sha256.is_empty() && fingerprint.file_size != 0)
            .map(|fingerprint| (fingerprint.sha256.to_ascii_lowercase(), fingerprint.file_size))
            .collect();
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
        self.description_cleaner = match DescriptionCleaner::new(&self.description_rules) {
            Ok(cleaner) => Some(cleaner),
            Err(err) => {
                log::error!("Invalid description rules: {err}");
                None
            }
        };
    }

    pub fn is_empty(&self) -> bool {
        self.legacy_checksums.is_empty()
            && self.sha256s.is_empty()
            && self.matchers.is_empty()
            && self.description_rules.is_empty()
            && self.text_member_rules.is_empty()
    }

    pub fn load<P: AsRef<Path>>(path: &P) -> crate::Result<Self> {
        match fs::read_to_string(path) {
            Ok(txt) => match toml::from_str::<FingerprintData>(&txt) {
                Ok(mut result) => {
                    TextMemberMatcher::new(&result.text_member_rules)?;
                    result.index();
                    Ok(result)
                }
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e.into()),
        }
    }

    /// Load each category from its configured file using the combined TOML schema.
    /// Empty paths disable that category. Other categories are ignored, unless
    /// they are the only recognized rules in a file (a likely configuration error).
    /// Empty catalogs are accepted. Read, parse and selected-pattern errors include
    /// the category and path; the combined `load` API retains its legacy behavior.
    pub fn load_split(member_rules: &Path, description_rules: &Path) -> crate::Result<Self> {
        fn load_category(path: &Path, category: &str, member: bool) -> crate::Result<FingerprintData> {
            if path.as_os_str().is_empty() {
                return Ok(FingerprintData::default());
            }
            let text = fs::read_to_string(path).map_err(|err| format!("Failed to load {category} rules from '{}': {err}", path.display()))?;
            let data: FingerprintData = toml::from_str(&text).map_err(|err| format!("Failed to parse {category} rules from '{}': {err}", path.display()))?;
            let has_members = !data.finger_prints.is_empty() || !data.text_member_rules.is_empty();
            let has_descriptions = !data.description_rules.is_empty();
            if (if member { !has_members } else { !has_descriptions }) && (has_members || has_descriptions) {
                return Err(format!("No {category} rules in '{}': file contains only other rule categories", path.display()).into());
            }
            Ok(data)
        }

        let members = load_category(member_rules, "member", true)?;
        let descriptions = load_category(description_rules, "description", false)?;
        let mut result = Self {
            finger_prints: members.finger_prints,
            text_member_rules: members.text_member_rules,
            description_rules: descriptions.description_rules,
            ..Default::default()
        };
        // Validate selected patterns before index() can log and skip an invalid rule.
        for rule in &result.finger_prints {
            if !rule.keywords.is_empty() {
                Regex::new(&rule.pattern).map_err(|err| format!("Invalid member rule '{}' in '{}': {err}", rule.name, member_rules.display()))?;
            }
        }
        DescriptionCleaner::new(&result.description_rules).map_err(|err| format!("Invalid description rules in '{}': {err}", description_rules.display()))?;
        TextMemberMatcher::new(&result.text_member_rules).map_err(|err| format!("Invalid text member rules in '{}': {err}", member_rules.display()))?;
        result.index();
        Ok(result)
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
            let fingerprint = Fingerprint::new(file_name.to_string(), &data);

            finger_prints.push(fingerprint);
        }

        let mut result = Self {
            finger_prints,
            ..Default::default()
        };
        result.index();
        Ok(result)
    }

    /// Legacy byte-based rules only. Text templates use `match_text_member` so
    /// callers cannot accidentally turn a report-only finding into a deletion.
    pub fn is_match(&self, name: &str, content: &[u8]) -> bool {
        self.is_exact_match(content) || self.is_pattern_match(name, content)
    }

    pub fn is_exact_match(&self, content: &[u8]) -> bool {
        let legacy_checksum = (get_crc32(content), content.len() as u64);
        if self.legacy_checksums.contains(&legacy_checksum) {
            return true;
        }
        if !self.sha256s.is_empty() {
            let sha256 = format!("{:x}", Sha256::digest(content));
            if self.sha256s.contains(&(sha256, content.len() as u64)) {
                return true;
            }
        }
        false
    }

    pub fn is_pattern_match(&self, name: &str, content: &[u8]) -> bool {
        self.matchers
            .iter()
            .any(|m| m.pattern.is_match(name) && m.keywords.iter().all(|keyword| content.contains_str(keyword.as_bytes())))
    }

    pub fn match_text_member(&self, name: &str, content: &[u8]) -> Vec<TextMemberMatch> {
        self.text_member_matcher
            .as_ref()
            .map_or_else(Vec::new, |matcher| matcher.matches(name, content))
    }

    pub fn clean_description(&self, name: &str, content: &[u8], max_passes: usize) -> DescriptionCleanResult {
        match &self.description_cleaner {
            Some(cleaner) => cleaner.clean(name, content, max_passes),
            None => DescriptionCleanResult {
                content: content.to_vec(),
                changes: Vec::new(),
                needs_review: !self.description_rules.is_empty(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::description_cleaner::RuleAction;
    use super::*;

    fn data(toml: &str) -> FingerprintData {
        let mut result: FingerprintData = toml::from_str(toml).unwrap();
        result.index();
        result
    }

    #[test]
    fn test_a_known_checksum_is_a_match_whatever_the_member_is_called() {
        let content = b"the same bytes as ever";
        let fingerprints = data(&format!(
            "[[fingerprint]]\nname = \"intro\"\ncrc = {}\nfile_size = {}\n",
            get_crc32(content),
            content.len()
        ));
        assert!(fingerprints.is_match("whatever.ans", content));
    }

    #[test]
    fn test_a_legacy_checksum_requires_the_recorded_size() {
        let content = b"the same bytes as ever";
        let fingerprints = data(&format!(
            "[[fingerprint]]\nname = \"intro\"\ncrc = {}\nfile_size = {}\n",
            get_crc32(content),
            content.len() + 1
        ));
        assert!(!fingerprints.is_match("whatever.ans", content));
    }

    #[test]
    fn test_a_sha256_fingerprint_matches_exact_content() {
        let content = b"the same bytes as ever";
        let fingerprints = data(&format!(
            "[[fingerprint]]\nname = \"intro\"\nsha256 = \"{:x}\"\nfile_size = {}\n",
            Sha256::digest(content),
            content.len()
        ));
        assert!(fingerprints.is_match("whatever.ans", content));
        assert!(!fingerprints.is_match("whatever.ans", b"different bytes"));
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

    #[test]
    fn test_description_rules_are_loaded_from_the_fingerprint_file() {
        let fingerprints = data("[[description_rule]]\nid = \"liquid\"\nlines = [\"^liquid whq$\"]\naction = \"auto_clean\"\n");
        let result = fingerprints.clean_description("FILE_ID.DIZ", b"Product\nLiQUiD WHQ\n", 4);
        assert_eq!(b"Product\n", result.content.as_slice());
        assert_eq!("liquid", result.changes[0].rule_id);
    }

    const MEMBER_RULE: &str = "[[fingerprint]]\nname = 'member-ad'\npattern = '^AD[.]TXT$'\nkeywords = ['member-ad']\n";
    const DESCRIPTION_RULE: &str = "[[description_rule]]\nid = 'description-ad'\nlines = ['^description-ad$']\naction = 'auto_clean'\n";
    fn assert_categories(rules: &FingerprintData, enabled: [bool; 2]) {
        assert_eq!(enabled[0], rules.is_match("AD.TXT", b"member-ad"));
        assert!(!rules.is_match("OTHER.TXT", b"member-ad"));
        let description = rules.clean_description("FILE_ID.DIZ", b"Product\ndescription-ad\n", 4);
        assert_eq!(enabled[1], !description.changes.is_empty());
        assert_eq!(
            if enabled[1] {
                b"Product\n".as_slice()
            } else {
                b"Product\ndescription-ad\n".as_slice()
            },
            description.content
        );
        assert!(!description.needs_review);
    }

    #[test]
    fn split_distinct_files_produce_usable_rules() {
        let dir = tempfile::tempdir().unwrap();
        let paths = [dir.path().join("members.toml"), dir.path().join("descriptions.toml")];
        for (path, text) in paths.iter().zip([MEMBER_RULE, DESCRIPTION_RULE]) {
            fs::write(path, text).unwrap();
        }
        let rules = FingerprintData::load_split(&paths[0], &paths[1]).unwrap();
        assert_categories(&rules, [true; 2]);
    }

    #[test]
    fn split_same_combined_path_works_and_combined_save_load_remains_usable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("combined.toml");
        fs::write(&path, [MEMBER_RULE, DESCRIPTION_RULE].concat()).unwrap();
        let rules = FingerprintData::load_split(&path, &path).unwrap();
        assert_categories(&rules, [true; 2]);
        let saved = dir.path().join("saved.toml");
        rules.save(&saved).unwrap();
        assert_categories(&FingerprintData::load(&saved).unwrap(), [true; 2]);
    }

    #[test]
    fn split_empty_paths_disable_each_category_and_isolate_combined_categories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("combined.toml");
        fs::write(&path, [MEMBER_RULE, DESCRIPTION_RULE].concat()).unwrap();
        for mask in 0..4 {
            let enabled = [mask & 1 != 0, mask & 2 != 0];
            let paths = enabled.map(|enable| if enable { path.as_path() } else { Path::new("") });
            let rules = FingerprintData::load_split(paths[0], paths[1]).unwrap();
            assert_categories(&rules, enabled);
            assert_eq!(mask == 0, rules.is_empty());
        }
    }

    #[test]
    fn split_only_uses_the_selected_category_from_each_file() {
        let dir = tempfile::tempdir().unwrap();
        let paths = [dir.path().join("a.toml"), dir.path().join("b.toml")];
        for (index, path) in paths.iter().enumerate() {
            let text = [MEMBER_RULE, DESCRIPTION_RULE].concat().replace("-ad", &format!("-ad-{index}"));
            fs::write(path, text).unwrap();
        }
        let rules = FingerprintData::load_split(&paths[0], &paths[1]).unwrap();
        for index in 0..2 {
            assert_eq!(index == 0, rules.is_match("AD.TXT", format!("member-ad-{index}").as_bytes()));
            let description = rules.clean_description("FILE_ID.DIZ", format!("Product\ndescription-ad-{index}\n").as_bytes(), 4);
            assert_eq!(index == 1, !description.changes.is_empty());
        }
    }

    #[test]
    fn split_missing_malformed_and_wrong_category_files_have_contextual_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.toml");
        let empty = Path::new("");
        for (index, category) in ["member", "description"].iter().enumerate() {
            let mut paths = [empty; 2];
            paths[index] = &path;
            let wrong_category = if index == 0 { DESCRIPTION_RULE } else { MEMBER_RULE };
            for text in [None, Some("[[broken"), Some("fingerprint = 'wrong type'"), Some(wrong_category)] {
                if let Some(text) = text {
                    fs::write(&path, text).unwrap();
                }
                let error = FingerprintData::load_split(paths[0], paths[1]).err().expect("must fail").to_string();
                assert!(error.contains(category), "{error}");
                assert!(error.contains(&path.display().to_string()), "{error}");
                if text.is_some() {
                    fs::remove_file(&path).unwrap();
                }
            }
        }
    }

    #[test]
    fn split_empty_catalogs_are_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.toml");
        for text in ["", "# intentionally empty\n", "fingerprint = []\ndescription_rule = []\n"] {
            fs::write(&path, text).unwrap();
            assert!(FingerprintData::load_split(&path, &path).unwrap().is_empty());
        }
    }

    #[test]
    fn split_invalid_selected_patterns_error_instead_of_being_silently_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.toml");
        let empty = Path::new("");
        for (text, category, paths) in [
            (MEMBER_RULE.replace("^AD[.]TXT$", "["), "member", [&path as &Path, empty]),
            (DESCRIPTION_RULE.replace("^description-ad$", "["), "description", [empty, &path]),
            (format!("{DESCRIPTION_RULE}member_pattern = '['\n"), "description", [empty, &path]),
        ] {
            fs::write(&path, text).unwrap();
            let error = FingerprintData::load_split(paths[0], paths[1]).err().expect("must fail").to_string();
            assert!(error.contains(category), "{error}");
            assert!(error.contains(&path.display().to_string()), "{error}");
        }
    }

    #[test]
    fn literal_description_rules_load_and_round_trip_without_regex_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("literal.toml");
        fs::write(&path, "[[description_rule]]\nid = 'literal'\nliteral_lines = ['[Board] + Support?']\n").unwrap();
        let empty = Path::new("");
        let rules = FingerprintData::load_split(empty, &path).unwrap();
        let input = b"Product\n[BOARD] + Support?\n";
        let result = rules.clean_description("FILE_ID.DIZ", input, 8);
        assert_eq!(input.as_slice(), result.content);
        assert_eq!(RuleAction::ReportOnly, result.changes[0].action);
        rules.save(&path).unwrap();
        let saved = fs::read_to_string(&path).unwrap();
        let value: toml::Value = toml::from_str(&saved).unwrap();
        assert!(value["description_rule"][0].get("lines").is_none());
        assert_eq!(value["description_rule"][0]["literal_lines"][0].as_str(), Some("[Board] + Support?"));
        let reloaded = FingerprintData::load(&path).unwrap();
        assert_eq!(result, reloaded.clean_description("FILE_ID.DIZ", input, 8));
    }

    #[test]
    fn invalid_literal_description_rules_fail_with_file_and_rule_context() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.toml");
        let empty = Path::new("");
        for fields in [
            "literal_lines = ['ad']\nlines = ['^ad$']",
            "literal_lines = []",
            "literal_lines = ['   ']",
            "literal_lines = ['@X0F']",
            "literal_lines = [\"one\\ntwo\"]",
            "",
        ] {
            fs::write(&path, format!("[[description_rule]]\nid = 'invalid-literal'\n{fields}\n")).unwrap();
            let error = FingerprintData::load_split(empty, &path).err().unwrap().to_string();
            assert!(error.contains("invalid-literal"), "{error}");
            assert!(error.contains(&path.display().to_string()), "{error}");
        }
    }

    #[test]
    fn test_the_shipped_advertisement_rules_load() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let paths = [assets.join("upload_ad_files.toml"), assets.join("upload_ad_descriptions.toml")];
        // Check the assets themselves are split, not just filtered by load_split.
        for (index, path) in paths.iter().enumerate() {
            let catalog = FingerprintData::load(path).unwrap();
            assert_eq!(if index == 0 { 12 } else { 0 }, catalog.finger_prints.len());
            assert_eq!(if index == 1 { 39 } else { 0 }, catalog.description_rules.len());
        }
        let rules = FingerprintData::load_split(&paths[0], &paths[1]).unwrap();
        assert_eq!(12, rules.sha256s.len());
        assert_eq!(39, rules.description_rules.len());
        assert_eq!(32, rules.description_rules.iter().filter(|r| r.action == RuleAction::AutoClean).count());
        assert_eq!(7, rules.description_rules.iter().filter(|r| r.action == RuleAction::ReportOnly).count());
        assert!(!rules.is_empty());
    }
}
