//! Whole-member text templates. Deliberately separate from legacy raw-byte fingerprints.
use std::collections::HashSet;

use codepages::tables::CP437_TO_UNICODE;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::description_cleaner::RuleAction;

const HARD_MAX_BYTES: usize = 128 * 1024;
const HARD_MAX_LINES: usize = 512;
fn default_bytes() -> usize {
    16 * 1024
}

/// Every line must match, in order. Leading/trailing blank lines are ignored.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextMemberRule {
    pub id: String,
    #[serde(default)]
    pub action: RuleAction,
    #[serde(default = "default_bytes")]
    pub max_bytes: usize,
    pub lines: Vec<TextMemberLine>,
}

/// Literal lines are normalized like the input; regex lines see normalized text
/// and are implicitly anchored at both ends. Rust regex syntax, no backtracking.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextMemberLine {
    Literal(String),
    Regex(String),
}

#[derive(Clone, Debug)]
pub struct TextMemberMatch {
    pub rule_id: String,
    pub action: RuleAction,
    pub encoding: &'static str,
    pub sha256: String,
}

enum LineMatcher {
    Literal(String),
    Regex(Regex),
}
struct CompiledRule {
    id: String,
    action: RuleAction,
    max_bytes: usize,
    lines: Vec<LineMatcher>,
}

pub struct TextMemberMatcher {
    rules: Vec<CompiledRule>,
    colors: Regex,
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

impl TextMemberMatcher {
    pub fn new(rules: &[TextMemberRule]) -> crate::Result<Self> {
        let colors = Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]|(?i:@x[0-9a-f]{2})")?;
        let mut ids = HashSet::new();
        let mut compiled = Vec::new();
        for rule in rules {
            let invalid = |reason: &str| format!("Invalid text member rule '{}': {reason}", rule.id);
            if rule.id.trim().is_empty() || !ids.insert(rule.id.clone()) {
                return Err(invalid("id must be nonempty and unique").into());
            }
            if !(1..=HARD_MAX_BYTES).contains(&rule.max_bytes) || !(1..=HARD_MAX_LINES).contains(&rule.lines.len()) {
                return Err(invalid("max_bytes must be 1..=131072; lines must contain 1..=512 entries").into());
            }
            let mut lines = Vec::new();
            let mut literal_letters = 0;
            for line in &rule.lines {
                lines.push(match line {
                    TextMemberLine::Literal(text) => {
                        if text.chars().any(char::is_control) {
                            return Err(invalid("literal lines must not contain control characters").into());
                        }
                        let text = normalize_line(&colors.replace_all(text, ""));
                        literal_letters += text.chars().filter(|c| c.is_alphabetic()).count();
                        LineMatcher::Literal(text)
                    }
                    TextMemberLine::Regex(pattern) => LineMatcher::Regex(Regex::new(&format!(r"\A(?:{pattern})\z")).map_err(|err| invalid(&err.to_string()))?),
                });
            }
            if literal_letters < 12 {
                return Err(invalid("templates need at least 12 literal letters as stable identity; regex-only rules are not supported").into());
            }
            compiled.push(CompiledRule {
                id: rule.id.clone(),
                action: rule.action,
                max_bytes: rule.max_bytes,
                lines,
            });
        }
        Ok(Self { rules: compiled, colors })
    }

    pub fn matches(&self, name: &str, raw: &[u8]) -> Vec<TextMemberMatch> {
        let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
        // Canonical descriptions may be cleaned by description rules, never by
        // these whole-member text rules, even when they contain only an ad.
        if super::is_short_desc(std::ffi::OsStr::new(basename)).is_some()
            || self.rules.is_empty()
            || raw.len() > HARD_MAX_BYTES
            || !self.rules.iter().any(|rule| raw.len() <= rule.max_bytes)
        {
            return Vec::new();
        }
        let Some((text, encoding)) = self.normalize(raw) else {
            return Vec::new();
        };
        let lines: Vec<_> = text.lines().collect();
        let matching: Vec<_> = self
            .rules
            .iter()
            .filter(|rule| {
                raw.len() <= rule.max_bytes
                    && lines.len() == rule.lines.len()
                    && lines.iter().zip(&rule.lines).all(|(text, matcher)| match matcher {
                        LineMatcher::Literal(literal) => text == literal,
                        LineMatcher::Regex(regex) => regex.is_match(text),
                    })
            })
            .collect();
        if matching.is_empty() {
            return Vec::new();
        }
        let sha256 = format!("{:x}", Sha256::digest(raw));
        matching
            .into_iter()
            .map(|rule| TextMemberMatch {
                rule_id: rule.id.clone(),
                action: rule.action,
                encoding,
                sha256: sha256.clone(),
            })
            .collect()
    }

    fn normalize(&self, raw: &[u8]) -> Option<(String, &'static str)> {
        // Reject executable headers and unknown control sequences, rather than
        // interpreting arbitrary binaries or cursor movement as visible text.
        if raw.starts_with(b"MZ") || raw.starts_with(b"\x7fELF") {
            return None;
        }
        let (body, tail) = match raw.iter().position(|b| *b == 0x1a) {
            Some(i) => (&raw[..i], &raw[i + 1..]),
            None => (raw, &[][..]),
        };
        if tail.iter().any(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x1a)) {
            return None;
        }
        if body.iter().any(|b| *b < 32 && !matches!(b, b'\t' | b'\r' | b'\n' | 0x1b)) {
            return None;
        }
        let (decoded, encoding) = if let Some(payload) = body.strip_prefix(b"\xef\xbb\xbf") {
            (std::str::from_utf8(payload).ok()?.to_owned(), "utf8-bom")
        } else if let Ok(text) = std::str::from_utf8(body) {
            (text.to_owned(), if body.is_ascii() { "ascii" } else { "utf8" })
        } else {
            (body.iter().map(|b| CP437_TO_UNICODE[*b as usize]).collect(), "cp437-inferred")
        };
        // Only SGR color escapes are allowed. CSI cursor/erase instructions must
        // not conceal unmatched material. @X colors are ordinary PCBoard text.
        let sgr = sgr_regex();
        let decoded = sgr.replace_all(&decoded, "");
        if decoded.chars().any(|c| c.is_control() && !matches!(c, '\t' | '\r' | '\n')) {
            return None;
        }
        let decoded = self.colors.replace_all(&decoded, "");
        let decoded = decoded.replace("\r\n", "\n").replace('\r', "\n");
        if decoded.split('\n').count() > HARD_MAX_LINES {
            return None;
        }
        let normalized = decoded.split('\n').map(normalize_line).collect::<Vec<_>>().join("\n");
        Some((normalized.trim_matches('\n').to_string(), encoding))
    }
}

fn sgr_regex() -> &'static Regex {
    static SGR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*m").unwrap());
    &SGR
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> Vec<TextMemberRule> {
        vec![TextMemberRule {
            id: "example".into(),
            action: RuleAction::ReportOnly,
            max_bytes: 1024,
            lines: vec![
                TextMemberLine::Literal("Example Board Grüße".into()),
                TextMemberLine::Regex(r"uploaded [0-9]{2}-[0-9]{2}-[0-9]{4}".into()),
            ],
        }]
    }
    #[test]
    fn matches_full_templates_in_both_encodings_and_ignores_names() {
        let matcher = TextMemberMatcher::new(&rules()).unwrap();
        for raw in [
            b"\r\nExample Board Gr\x81\xe1e\r\nuploaded 01-02-1994\r\n\x1a".to_vec(),
            "\u{feff}\x1b[31mEXAMPLE  BOARD Grüße\x1b[0m\n@X0fuploaded 12-31-2026\n".as_bytes().to_vec(),
        ] {
            let matches = matcher.matches("renamed/weird.nfo", &raw);
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].action, RuleAction::ReportOnly);
            assert_eq!(matches[0].sha256, format!("{:x}", Sha256::digest(&raw)));
        }
    }
    #[test]
    fn rejects_extra_material_controls_bad_bom_and_protected_descriptions() {
        let matcher = TextMemberMatcher::new(&rules()).unwrap();
        let good = "Example Board Grüße\nuploaded 01-02-1994";
        for text in [
            format!("manual\n{good}"),
            format!("{good}\nmanual"),
            format!("{good}\x1asecret"),
            format!("{good}\x1b[2J"),
            format!("{good}\0"),
            good.replace("01-02-1994", "01-02-1994 extra"),
        ] {
            assert!(matcher.matches("x.txt", text.as_bytes()).is_empty());
        }
        assert!(matcher.matches("x", b"\xef\xbb\xbf\xff").is_empty());
        for name in ["FILE_ID.DIZ", "dir\\file_id.ans", "dir/desc.sdi", "file_id.pcb"] {
            assert!(matcher.matches(name, good.as_bytes()).is_empty());
        }
    }
    #[test]
    fn validates_rules_and_preserves_ambiguities() {
        let mut rules = rules();
        rules.push(rules[0].clone());
        assert!(TextMemberMatcher::new(&rules).is_err());
        rules[1].id = "other".into();
        assert_eq!(
            TextMemberMatcher::new(&rules)
                .unwrap()
                .matches("x", "Example Board Grüße\nuploaded 01-02-1994".as_bytes())
                .len(),
            2
        );
        rules[0].max_bytes = HARD_MAX_BYTES + 1;
        assert!(TextMemberMatcher::new(&rules).is_err());
        rules[0].max_bytes = 1;
        rules[0].lines = vec![TextMemberLine::Regex(".*".into())];
        assert!(TextMemberMatcher::new(&rules).is_err());
        rules[0].lines = vec![TextMemberLine::Literal("Example Board Grüße".into()), TextMemberLine::Regex("[".into())];
        assert!(TextMemberMatcher::new(&rules).is_err());
    }
}
