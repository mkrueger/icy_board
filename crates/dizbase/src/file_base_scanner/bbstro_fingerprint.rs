use bstr::ByteSlice;
use codepages::{normalize_file, tables::get_utf8};
use icy_net::crc::get_crc32;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, path::Path};
use walkdir::WalkDir;

use super::description_cleaner::{DescriptionBlockRule, DescriptionCleanResult, DescriptionCleaner, RuleAction};

/// A fingerprint whose pattern has been compiled once instead of once per file.
struct Matcher {
    pattern: Regex,
    keywords: Vec<String>,
}

struct ArchiveCommentMatcher {
    id: String,
    sha256: String,
    file_size: u64,
    keywords: Vec<String>,
    action: RuleAction,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ArchiveCommentRule {
    pub id: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sha256: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_null_64")]
    pub file_size: u64,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub action: RuleAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveCommentResult {
    pub content: Vec<u8>,
    pub rule_ids: Vec<String>,
    pub needs_review: bool,
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

    #[serde(default)]
    #[serde(rename = "archive_comment_rule")]
    archive_comment_rules: Vec<ArchiveCommentRule>,

    #[serde(skip)]
    legacy_checksums: HashSet<(u32, u64)>,

    #[serde(skip)]
    sha256s: HashSet<(String, u64)>,

    #[serde(skip)]
    matchers: Vec<Matcher>,

    #[serde(skip)]
    description_cleaner: Option<DescriptionCleaner>,

    #[serde(skip)]
    archive_comment_matchers: Vec<ArchiveCommentMatcher>,
}

impl FingerprintData {
    fn index(&mut self) {
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
                log::error!("Description rules contain an unusable pattern: {err}");
                None
            }
        };
        self.archive_comment_matchers = self
            .archive_comment_rules
            .iter()
            .filter(|rule| !rule.sha256.is_empty() || !rule.keywords.is_empty())
            .map(|rule| ArchiveCommentMatcher {
                id: rule.id.clone(),
                sha256: rule.sha256.to_ascii_lowercase(),
                file_size: rule.file_size,
                keywords: rule.keywords.iter().map(|keyword| keyword.to_lowercase()).collect(),
                action: rule.action,
            })
            .collect();
    }

    pub fn is_empty(&self) -> bool {
        self.legacy_checksums.is_empty()
            && self.sha256s.is_empty()
            && self.matchers.is_empty()
            && self.description_rules.is_empty()
            && self.archive_comment_rules.is_empty()
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

    /// Load each category from its configured file using the combined TOML schema.
    /// Empty paths disable that category. Other categories are ignored, unless
    /// they are the only recognized rules in a file (a likely configuration error).
    /// Empty catalogs are accepted. Read, parse and selected-pattern errors include
    /// the category and path; the combined `load` API retains its legacy behavior.
    pub fn load_split(member_rules: &Path, description_rules: &Path, comment_rules: &Path) -> crate::Result<Self> {
        fn load_category<T>(path: &Path, category: &str, select: fn(FingerprintData) -> Vec<T>) -> crate::Result<Vec<T>> {
            if path.as_os_str().is_empty() {
                return Ok(Vec::new());
            }
            let text = fs::read_to_string(path).map_err(|err| format!("Failed to load {category} rules from '{}': {err}", path.display()))?;
            let data: FingerprintData = toml::from_str(&text).map_err(|err| format!("Failed to parse {category} rules from '{}': {err}", path.display()))?;
            let has_rules = !data.finger_prints.is_empty() || !data.description_rules.is_empty() || !data.archive_comment_rules.is_empty();
            let selected = select(data);
            if selected.is_empty() && has_rules {
                return Err(format!("No {category} rules in '{}': file contains only other rule categories", path.display()).into());
            }
            Ok(selected)
        }

        let mut result = Self {
            finger_prints: load_category(member_rules, "member", |data| data.finger_prints)?,
            description_rules: load_category(description_rules, "description", |data| data.description_rules)?,
            archive_comment_rules: load_category(comment_rules, "archive comment", |data| data.archive_comment_rules)?,
            ..Default::default()
        };
        // Validate selected patterns before index() can log and skip an invalid rule.
        for rule in &result.finger_prints {
            if !rule.keywords.is_empty() {
                Regex::new(&rule.pattern).map_err(|err| format!("Invalid member rule '{}' in '{}': {err}", rule.name, member_rules.display()))?;
            }
        }
        DescriptionCleaner::new(&result.description_rules).map_err(|err| format!("Invalid description rules in '{}': {err}", description_rules.display()))?;
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

    /// Whether an archive member is one of the intros that keep travelling with the files.
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

    pub fn clean_archive_comment(&self, content: &[u8]) -> ArchiveCommentResult {
        let sha256 = format!("{:x}", Sha256::digest(content));
        let normalized = get_utf8(&normalize_file(content)).to_lowercase();
        let matches: Vec<&ArchiveCommentMatcher> = self
            .archive_comment_matchers
            .iter()
            .filter(|rule| {
                let hash_matches = !rule.sha256.is_empty() && rule.sha256 == sha256 && (rule.file_size == 0 || rule.file_size == content.len() as u64);
                let keywords_match = !rule.keywords.is_empty() && rule.keywords.iter().all(|keyword| normalized.contains(keyword));
                hash_matches || keywords_match
            })
            .collect();
        if matches.len() != 1 {
            return ArchiveCommentResult {
                content: content.to_vec(),
                rule_ids: matches.iter().map(|rule| rule.id.clone()).collect(),
                needs_review: matches.len() > 1,
            };
        }
        let rule = matches[0];
        ArchiveCommentResult {
            content: if rule.action == RuleAction::AutoClean { Vec::new() } else { content.to_vec() },
            rule_ids: vec![rule.id.clone()],
            needs_review: rule.action == RuleAction::Review,
        }
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

    #[test]
    fn test_an_exact_archive_comment_rule_removes_only_its_comment() {
        let comment = b"The BBS Archives";
        let fingerprints = data(&format!(
            "[[archive_comment_rule]]\nid = \"bbs-archives\"\nsha256 = \"{:x}\"\nfile_size = {}\naction = \"auto_clean\"\n",
            Sha256::digest(comment),
            comment.len()
        ));
        let result = fingerprints.clean_archive_comment(comment);
        assert!(result.content.is_empty());
        assert_eq!(vec!["bbs-archives"], result.rule_ids);

        let other = fingerprints.clean_archive_comment(b"Author's original comment");
        assert_eq!(b"Author's original comment", other.content.as_slice());
        assert!(other.rule_ids.is_empty());
    }

    const MEMBER_RULE: &str = "[[fingerprint]]\nname = 'member-ad'\npattern = '^AD[.]TXT$'\nkeywords = ['member-ad']\n";
    const DESCRIPTION_RULE: &str = "[[description_rule]]\nid = 'description-ad'\nlines = ['^description-ad$']\naction = 'auto_clean'\n";
    const COMMENT_RULE: &str = "[[archive_comment_rule]]\nid = 'comment-ad'\nkeywords = ['comment-ad']\naction = 'auto_clean'\n";

    fn assert_categories(rules: &FingerprintData, enabled: [bool; 3]) {
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
        let comment = rules.clean_archive_comment(b"comment-ad");
        assert_eq!(enabled[2], !comment.rule_ids.is_empty());
        assert_eq!(if enabled[2] { b"".as_slice() } else { b"comment-ad".as_slice() }, comment.content);
        assert!(!comment.needs_review);
    }

    #[test]
    fn split_distinct_files_produce_usable_rules() {
        let dir = tempfile::tempdir().unwrap();
        let paths = [
            dir.path().join("members.toml"),
            dir.path().join("descriptions.toml"),
            dir.path().join("comments.toml"),
        ];
        for (path, text) in paths.iter().zip([MEMBER_RULE, DESCRIPTION_RULE, COMMENT_RULE]) {
            fs::write(path, text).unwrap();
        }
        let rules = FingerprintData::load_split(&paths[0], &paths[1], &paths[2]).unwrap();
        assert_categories(&rules, [true; 3]);
    }

    #[test]
    fn split_same_combined_path_works_and_combined_save_load_remains_usable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("combined.toml");
        fs::write(&path, [MEMBER_RULE, DESCRIPTION_RULE, COMMENT_RULE].concat()).unwrap();
        let rules = FingerprintData::load_split(&path, &path, &path).unwrap();
        assert_categories(&rules, [true; 3]);
        let saved = dir.path().join("saved.toml");
        rules.save(&saved).unwrap();
        assert_categories(&FingerprintData::load(&saved).unwrap(), [true; 3]);
    }

    #[test]
    fn split_empty_paths_disable_each_category_and_isolate_combined_categories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("combined.toml");
        fs::write(&path, [MEMBER_RULE, DESCRIPTION_RULE, COMMENT_RULE].concat()).unwrap();
        for mask in 0..8 {
            let enabled = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0];
            let paths = enabled.map(|enable| if enable { path.as_path() } else { Path::new("") });
            let rules = FingerprintData::load_split(paths[0], paths[1], paths[2]).unwrap();
            assert_categories(&rules, enabled);
            assert_eq!(mask == 0, rules.is_empty());
        }
    }

    #[test]
    fn split_only_uses_the_selected_category_from_each_file() {
        let dir = tempfile::tempdir().unwrap();
        let paths = [dir.path().join("a.toml"), dir.path().join("b.toml"), dir.path().join("c.toml")];
        for (index, path) in paths.iter().enumerate() {
            let text = [MEMBER_RULE, DESCRIPTION_RULE, COMMENT_RULE].concat().replace("-ad", &format!("-ad-{index}"));
            fs::write(path, text).unwrap();
        }
        let rules = FingerprintData::load_split(&paths[0], &paths[1], &paths[2]).unwrap();
        for index in 0..3 {
            assert_eq!(index == 0, rules.is_match("AD.TXT", format!("member-ad-{index}").as_bytes()));
            let description = rules.clean_description("FILE_ID.DIZ", format!("Product\ndescription-ad-{index}\n").as_bytes(), 4);
            assert_eq!(index == 1, !description.changes.is_empty());
            let comment = rules.clean_archive_comment(format!("comment-ad-{index}").as_bytes());
            assert_eq!(index == 2, !comment.rule_ids.is_empty());
        }
    }

    #[test]
    fn split_missing_malformed_and_wrong_category_files_have_contextual_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.toml");
        let empty = Path::new("");
        for (index, category) in ["member", "description", "archive comment"].iter().enumerate() {
            let mut paths = [empty; 3];
            paths[index] = &path;
            let wrong_category = if index == 0 { DESCRIPTION_RULE } else { MEMBER_RULE };
            for text in [None, Some("[[broken"), Some("fingerprint = 'wrong type'"), Some(wrong_category)] {
                if let Some(text) = text {
                    fs::write(&path, text).unwrap();
                }
                let error = FingerprintData::load_split(paths[0], paths[1], paths[2]).err().expect("must fail").to_string();
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
        for text in [
            "",
            "# intentionally empty\n",
            "fingerprint = []\ndescription_rule = []\narchive_comment_rule = []\n",
        ] {
            fs::write(&path, text).unwrap();
            assert!(FingerprintData::load_split(&path, &path, &path).unwrap().is_empty());
        }
    }

    #[test]
    fn split_invalid_selected_patterns_error_instead_of_being_silently_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.toml");
        let empty = Path::new("");
        for (text, category, paths) in [
            (MEMBER_RULE.replace("^AD[.]TXT$", "["), "member", [&path as &Path, empty, empty]),
            (DESCRIPTION_RULE.replace("^description-ad$", "["), "description", [empty, &path, empty]),
            (format!("{DESCRIPTION_RULE}member_pattern = '['\n"), "description", [empty, &path, empty]),
        ] {
            fs::write(&path, text).unwrap();
            let error = FingerprintData::load_split(paths[0], paths[1], paths[2]).err().expect("must fail").to_string();
            assert!(error.contains(category), "{error}");
            assert!(error.contains(&path.display().to_string()), "{error}");
        }
    }

    #[test]
    fn test_the_shipped_advertisement_rules_load() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let paths = [
            assets.join("upload_ad_files.toml"),
            assets.join("upload_ad_descriptions.toml"),
            assets.join("upload_ad_comments.toml"),
        ];
        // Check the assets themselves are split, not just filtered by load_split.
        for (index, path) in paths.iter().enumerate() {
            let catalog = FingerprintData::load(path).unwrap();
            assert_eq!(if index == 0 { 12 } else { 0 }, catalog.finger_prints.len());
            assert_eq!(if index == 1 { 2 } else { 0 }, catalog.description_rules.len());
            assert_eq!(if index == 2 { 1 } else { 0 }, catalog.archive_comment_rules.len());
        }
        let rules = FingerprintData::load_split(&paths[0], &paths[1], &paths[2]).unwrap();
        assert_eq!(12, rules.sha256s.len());
        assert_eq!(2, rules.description_rules.len());
        assert_eq!(1, rules.archive_comment_matchers.len());
        assert!(!rules.is_empty());
    }
}
