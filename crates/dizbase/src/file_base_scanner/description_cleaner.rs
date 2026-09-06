use codepages::{normalize_file, tables::get_utf8};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    #[default]
    ReportOnly,
    Review,
    AutoClean,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockPosition {
    Prefix,
    #[default]
    Suffix,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DescriptionBlockRule {
    pub id: String,
    #[serde(default = "default_description_member_pattern")]
    pub member_pattern: String,
    #[serde(default)]
    pub position: BlockPosition,
    /// Optional literal ASCII start marker for a suffix attached to an existing
    /// line. Match it in the original bytes, then validate all normalized lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_start: Option<String>,
    /// Regular expressions applied to normalized lines; mutually exclusive with literal_lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    /// Whole-line literal matches after the same normalization as the description.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub literal_lines: Vec<String>,
    #[serde(default)]
    pub action: RuleAction,
}

impl Default for DescriptionBlockRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            member_pattern: default_description_member_pattern(),
            position: BlockPosition::default(),
            inline_start: None,
            lines: Vec::new(),
            literal_lines: Vec::new(),
            action: RuleAction::default(),
        }
    }
}

fn default_description_member_pattern() -> String {
    "(?i)(^|[/\\\\])(desc\\.sdi|file_id\\.(diz|ans|pcb))$".to_string()
}

enum LineMatcher {
    Regex(Regex),
    Literal(String),
}

impl LineMatcher {
    fn is_match(&self, line: &str) -> bool {
        match self {
            Self::Regex(pattern) => pattern.is_match(line),
            Self::Literal(text) => text == line,
        }
    }
}

struct CompiledDescriptionRule {
    id: String,
    member_pattern: Regex,
    position: BlockPosition,
    inline_start: Option<String>,
    lines: Vec<LineMatcher>,
    action: RuleAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DescriptionChange {
    pub rule_id: String,
    pub action: RuleAction,
    pub first_line: usize,
    pub last_line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DescriptionCleanResult {
    pub content: Vec<u8>,
    pub changes: Vec<DescriptionChange>,
    pub needs_review: bool,
}

pub struct DescriptionCleaner {
    rules: Vec<CompiledDescriptionRule>,
    ansi: Regex,
}

impl DescriptionCleaner {
    pub fn new(rules: &[DescriptionBlockRule]) -> crate::Result<Self> {
        let mut cleaner = Self {
            rules: Vec::with_capacity(rules.len()),
            ansi: Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]")?,
        };
        for rule in rules {
            if rule.lines.is_empty() == rule.literal_lines.is_empty() {
                return Err(format!("Description rule '{}': specify exactly one nonempty list of lines or literal_lines", rule.id).into());
            }
            let lines = if rule.literal_lines.is_empty() {
                rule.lines
                    .iter()
                    .map(|line| Regex::new(line).map(LineMatcher::Regex))
                    .collect::<Result<_, _>>()?
            } else {
                let mut lines = Vec::with_capacity(rule.literal_lines.len());
                for (index, line) in rule.literal_lines.iter().enumerate() {
                    let normalized = cleaner.normalize_text(line);
                    if normalized.is_empty() || line.contains(['\r', '\n', '\x1a']) {
                        return Err(format!(
                            "Description rule '{}': literal_lines entry {} must contain nonempty text on one line without CR, LF or DOS EOF",
                            rule.id,
                            index + 1
                        )
                        .into());
                    }
                    lines.push(LineMatcher::Literal(normalized));
                }
                lines
            };
            cleaner.rules.push(CompiledDescriptionRule {
                id: rule.id.clone(),
                member_pattern: Regex::new(&rule.member_pattern)?,
                position: rule.position,
                inline_start: rule.inline_start.clone(),
                lines,
                action: rule.action,
            });
        }
        Ok(cleaner)
    }

    pub fn clean(&self, member_name: &str, content: &[u8], max_passes: usize) -> DescriptionCleanResult {
        let mut content = content.to_vec();
        let mut changes = Vec::new();
        let mut needs_review = false;
        if !self.rules.iter().any(|rule| rule.member_pattern.is_match(member_name)) {
            return DescriptionCleanResult {
                content,
                changes,
                needs_review,
            };
        }

        for _ in 0..max_passes {
            let lines = raw_lines(&content);
            let normalized: Vec<(usize, String)> = lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| {
                    let normalized = self.normalize(line);
                    (!normalized.is_empty()).then_some((index, normalized))
                })
                .collect();
            let matches: Vec<(&CompiledDescriptionRule, usize, usize, usize)> = self
                .rules
                .iter()
                .filter(|rule| rule.member_pattern.is_match(member_name))
                .flat_map(|rule| self.match_block(rule, &normalized, &lines))
                .collect();

            if matches.is_empty() {
                break;
            }
            if matches.len() != 1 {
                needs_review = true;
                break;
            }

            let (rule, first_line, last_line, first_byte) = matches[0];
            changes.push(DescriptionChange {
                rule_id: rule.id.clone(),
                action: rule.action,
                first_line: first_line + 1,
                last_line: last_line + 1,
            });
            match rule.action {
                RuleAction::ReportOnly => break,
                RuleAction::Review => {
                    needs_review = true;
                    break;
                }
                RuleAction::AutoClean => match rule.position {
                    BlockPosition::Prefix => content = lines[last_line + 1..].concat(),
                    BlockPosition::Suffix => {
                        let mut kept = lines[..first_line].concat();
                        if first_byte > 0 {
                            let first = lines[first_line];
                            kept.extend_from_slice(&first[..first_byte]);
                            // Preserve the description's original line ending,
                            // without re-encoding its frame, colors or whitespace.
                            if first.ends_with(b"\r\n") {
                                kept.extend_from_slice(b"\r\n");
                            } else if first.ends_with(b"\n") {
                                kept.push(b'\n');
                            }
                        }
                        content = kept;
                    }
                },
            }
        }

        if !content.is_empty() && max_passes > 0 && changes.len() == max_passes {
            let normalized: Vec<(usize, String)> = raw_lines(&content)
                .iter()
                .enumerate()
                .filter_map(|(index, line)| {
                    let normalized = self.normalize(line);
                    (!normalized.is_empty()).then_some((index, normalized))
                })
                .collect();
            if self
                .rules
                .iter()
                .filter(|rule| rule.member_pattern.is_match(member_name))
                .any(|rule| !self.match_block(rule, &normalized, &raw_lines(&content)).is_empty())
            {
                needs_review = true;
            }
        }
        if changes.iter().any(|change| change.action == RuleAction::AutoClean) && raw_lines(&content).iter().all(|line| self.normalize(line).is_empty()) {
            needs_review = true;
        }

        DescriptionCleanResult {
            content,
            changes,
            needs_review,
        }
    }

    fn match_block<'a>(
        &self,
        rule: &'a CompiledDescriptionRule,
        normalized: &[(usize, String)],
        raw: &[&[u8]],
    ) -> Vec<(&'a CompiledDescriptionRule, usize, usize, usize)> {
        if let Some((rule, first, last)) = match_rule(rule, normalized) {
            return vec![(rule, first, last, 0)];
        }
        let Some(marker) = rule.inline_start.as_ref().filter(|marker| !marker.is_empty() && marker.is_ascii()) else {
            return Vec::new();
        };
        if rule.position != BlockPosition::Suffix || rule.lines.is_empty() || normalized.len() < rule.lines.len() {
            return Vec::new();
        }
        let candidate = &normalized[normalized.len() - rule.lines.len()..];
        if !rule.lines[1..].iter().zip(&candidate[1..]).all(|(pattern, (_, line))| pattern.is_match(line)) {
            return Vec::new();
        }
        let first = candidate[0].0;
        let last = candidate[candidate.len() - 1].0;
        raw[first]
            .windows(marker.len())
            .enumerate()
            .filter_map(|(offset, bytes)| {
                (bytes.eq_ignore_ascii_case(marker.as_bytes()) && rule.lines[0].is_match(&self.normalize(&raw[first][offset..])))
                    .then_some((rule, first, last, offset))
            })
            .collect()
    }

    fn normalize(&self, line: &[u8]) -> String {
        let line = line.strip_suffix(b"\n").unwrap_or(line);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let normalized = normalize_file(line);
        let text = get_utf8(&normalized);
        self.normalize_text(&text)
    }

    fn normalize_text(&self, text: &str) -> String {
        let text = self.ansi.replace_all(text, "");
        let chars: Vec<char> = text.chars().collect();
        let mut without_colors = String::new();
        let mut index = 0;
        while index < chars.len() {
            if index + 3 < chars.len()
                && chars[index] == '@'
                && matches!(chars[index + 1], 'x' | 'X')
                && chars[index + 2].is_ascii_hexdigit()
                && chars[index + 3].is_ascii_hexdigit()
            {
                index += 4;
            } else {
                without_colors.push(chars[index]);
                index += 1;
            }
        }
        without_colors.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
    }
}

fn raw_lines(content: &[u8]) -> Vec<&[u8]> {
    content.split_inclusive(|byte| *byte == b'\n').collect()
}

fn match_rule<'a>(rule: &'a CompiledDescriptionRule, lines: &[(usize, String)]) -> Option<(&'a CompiledDescriptionRule, usize, usize)> {
    if rule.lines.is_empty() || lines.len() < rule.lines.len() {
        return None;
    }
    let start = match rule.position {
        BlockPosition::Prefix => 0,
        BlockPosition::Suffix => lines.len() - rule.lines.len(),
    };
    let candidate = &lines[start..start + rule.lines.len()];
    if !rule.lines.iter().zip(candidate).all(|(pattern, (_, line))| pattern.is_match(line)) {
        return None;
    }
    Some((rule, candidate.first()?.0, candidate.last()?.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suffix_rule(id: &str, lines: &[&str]) -> DescriptionBlockRule {
        DescriptionBlockRule {
            id: id.to_string(),
            lines: lines.iter().map(|line| line.to_string()).collect(),
            action: RuleAction::AutoClean,
            ..Default::default()
        }
    }

    fn literal_rule(lines: &[&str]) -> DescriptionBlockRule {
        DescriptionBlockRule {
            id: "literal-footer".into(),
            literal_lines: lines.iter().map(|line| line.to_string()).collect(),
            action: RuleAction::AutoClean,
            ..Default::default()
        }
    }

    #[test]
    fn literal_lines_match_whole_lines_without_regex_interpretation() {
        let cleaner = DescriptionCleaner::new(&[literal_rule(&[r"[BOARD] .* + ? (ad) $ ^ \ end"])]).unwrap();
        let input = b"Product\n[BOARD] .* + ? (ad) $ ^ \\ end\n";
        let result = cleaner.clean("FILE_ID.DIZ", input, 8);
        assert_eq!(b"Product\n", result.content.as_slice());
        assert_eq!(1, result.changes.len());
        for text in [
            "Product\nB anything ad end\n",
            "Product\nprefix [BOARD] .* + ? (ad) $ ^ \\ end\n",
            "Product\n[BOARD] .* + ? (ad) $ ^ \\ end extra\n",
            "Product\n[BOARD] .* + ? (ad) $ ^ \\ end\nLegitimate tail\n",
        ] {
            let result = cleaner.clean("FILE_ID.DIZ", text.as_bytes(), 8);
            assert_eq!(text.as_bytes(), result.content);
            assert!(result.changes.is_empty());
            assert!(!result.needs_review);
        }
    }

    #[test]
    fn literal_lines_normalize_both_sides_and_preserve_raw_description_bytes() {
        let cleaner = DescriptionCleaner::new(&[literal_rule(&[" @X0F\x1b[31m GRÜSSE   vom\tBoard ", "[Download + Support]"])]).unwrap();
        for ad in [
            "  grüsse VOM board\r\n\r\n\t[download + support]  \r\n".as_bytes(),
            b"@X0Fgr\x81sse vom board\r\n \r\n\x1b[32m[DOWNLOAD + SUPPORT]\x1b[0m\r\n".as_slice(),
        ] {
            let kept = b"@X0B\xda\xc4\xbf  Original text \r\n";
            let input = [kept.as_slice(), ad].concat();
            let result = cleaner.clean("FILE_ID.DIZ", &input, 8);
            assert_eq!(kept.as_slice(), result.content);
            assert!(!result.needs_review);
            let result = cleaner.clean("UNRELATED.TXT", &input, 8);
            assert_eq!(input, result.content);
            assert!(result.changes.is_empty());
        }
    }

    #[test]
    fn literal_prefix_requires_complete_block_and_keeps_the_rest() {
        let mut rule = literal_rule(&["[Board]", "Visit us!"]);
        rule.position = BlockPosition::Prefix;
        let cleaner = DescriptionCleaner::new(&[rule]).unwrap();
        let result = cleaner.clean("DESC.SDI", b"[board]\nVisit us!\nProduct\n", 8);
        assert_eq!(b"Product\n", result.content.as_slice());
        assert_eq!((1, 2), (result.changes[0].first_line, result.changes[0].last_line));
        for input in [b"[Board]\nProduct\n".as_slice(), b"Product\n[Board]\nVisit us!\n".as_slice()] {
            let result = cleaner.clean("DESC.SDI", input, 8);
            assert_eq!(input, result.content);
            assert!(result.changes.is_empty());
        }
    }

    #[test]
    fn literal_inline_footer_requires_the_complete_block_at_the_end() {
        let mut rule = literal_rule(&["[Board] + Support?", "Visit us!"]);
        rule.inline_start = Some("[Board]".into());
        let cleaner = DescriptionCleaner::new(&[rule]).unwrap();
        let result = cleaner.clean("FILE_ID.DIZ", b"Product\r\n@X0F :---: [BOARD] + Support?\r\nVisit us!\r\n", 8);
        assert_eq!(b"Product\r\n@X0F :---: \r\n", result.content.as_slice());
        assert!(!result.needs_review);
        for input in [
            b"Product\n :---: [BOARD] + Support?\nUnrelated text\n".as_slice(),
            b"Product\n :---: [BOARD] + Support?\nVisit us!\nLegitimate tail\n".as_slice(),
            b"Product\n :---: [BOARD] + Support? extra\nVisit us!\n".as_slice(),
        ] {
            let result = cleaner.clean("FILE_ID.DIZ", input, 8);
            assert_eq!(input, result.content);
            assert!(result.changes.is_empty());
        }
    }

    #[test]
    fn literal_actions_ambiguity_and_pass_limits_keep_existing_safety_checks() {
        let input = b"Product\n[Board]\n";
        for action in [RuleAction::ReportOnly, RuleAction::Review, RuleAction::AutoClean] {
            let mut rule = literal_rule(&["[Board]"]);
            rule.action = action;
            let result = DescriptionCleaner::new(&[rule]).unwrap().clean("FILE_ID.DIZ", input, 8);
            assert_eq!(
                if action == RuleAction::AutoClean {
                    b"Product\n".as_slice()
                } else {
                    input.as_slice()
                },
                result.content
            );
            assert_eq!(action == RuleAction::Review, result.needs_review);
            assert_eq!(action, result.changes[0].action);
        }
        let rule = literal_rule(&["[Board]"]);
        let ambiguous = DescriptionCleaner::new(&[rule.clone(), suffix_rule("regex", &[r"^\[board\]$"])]).unwrap();
        let result = ambiguous.clean("FILE_ID.DIZ", input, 8);
        assert_eq!(input.as_slice(), result.content);
        assert!(result.needs_review);
        assert!(result.changes.is_empty());
        let cleaner = DescriptionCleaner::new(&[rule]).unwrap();
        let result = cleaner.clean("FILE_ID.DIZ", b"Product\n[Board]\n[Board]\n", 1);
        assert_eq!(input.as_slice(), result.content);
        assert!(result.needs_review);
        let result = cleaner.clean("FILE_ID.DIZ", b"[Board]\n", 8);
        assert!(result.content.is_empty());
        assert!(result.needs_review);
        let result = cleaner.clean("FILE_ID.DIZ", input, 0);
        assert_eq!(input.as_slice(), result.content);
        assert!(result.changes.is_empty());
    }

    #[test]
    fn literal_rules_reject_ambiguous_empty_and_multiline_definitions() {
        let mut both = literal_rule(&["Advertisement"]);
        both.lines.push("^advertisement$".into());
        assert!(DescriptionCleaner::new(&[both]).is_err());
        assert!(DescriptionCleaner::new(&[literal_rule(&[])]).is_err());
        for text in ["", " \t ", "@X0F\x1b[31m", "one\ntwo", "one\rtwo", "one\x1atwo"] {
            let error = DescriptionCleaner::new(&[literal_rule(&[text])]).err().unwrap().to_string();
            assert!(error.contains("literal-footer"), "{error}");
            assert!(error.contains("literal_lines"), "{error}");
        }
    }

    #[test]
    fn removes_a_shogunat_footer_without_reencoding_the_description() {
        let cleaner = DescriptionCleaner::new(&[suffix_rule(
            "shogunat-footer",
            &[r"^.*fucking fast shareware.*$", r"^.*shogunat 030 746 67 93.*$"],
        )])
        .unwrap();
        let input = b"Product description\r\n@X0F == FuCkInG FaSt ShArEwArE ==\r\n== SHoGuNaT 030 746 67 93 ==\r\n";
        let result = cleaner.clean("FILE_ID.DIZ", input, 4);
        assert_eq!(b"Product description\r\n", result.content.as_slice());
        assert_eq!(1, result.changes.len());
        assert!(!result.needs_review);
    }

    #[test]
    fn removes_stacked_liquid_and_courier_footers_from_the_outside_in() {
        let cleaner = DescriptionCleaner::new(&[
            suffix_rule("courier", &[r"^intercepted by the sonic team$", r"^couriering by the cure$"]),
            suffix_rule(
                "liquid-whq",
                &[r"^i was first on liquid's whq, @user@$", r"^why don't ya ask for japanese boards$"],
            ),
        ])
        .unwrap();
        let input =
            b"Product\nIntercepted By The SONiC Team\nCouriering by The CuRe\nI WAS FIRST ON LiQUiD'S WHQ, @USER@\nWHY DON'T YA ASK FOR JAPANESE BOARDS\n";
        let result = cleaner.clean("FILE_ID.DIZ", input, 4);
        assert_eq!(b"Product\n", result.content.as_slice());
        assert_eq!(
            vec!["liquid-whq", "courier"],
            result.changes.iter().map(|change| change.rule_id.as_str()).collect::<Vec<_>>()
        );
        assert!(!result.needs_review);
    }

    #[test]
    fn does_not_clean_a_noncanonical_diz_member() {
        let cleaner = DescriptionCleaner::new(&[suffix_rule("shogunat-footer", &[r"^shogunat 030 746 67 93$"])]).unwrap();
        let input = b"SHoGuNaT 030 746 67 93\n";
        let result = cleaner.clean("P!-STRIP.DIZ", input, 4);
        assert_eq!(input, result.content.as_slice());
        assert!(result.changes.is_empty());
    }

    #[test]
    fn report_only_records_without_modifying() {
        let mut rule = suffix_rule("group-footer", &[r"^release group$"]);
        rule.action = RuleAction::ReportOnly;
        let cleaner = DescriptionCleaner::new(&[rule]).unwrap();
        let input = b"Product\nRelease Group\n";
        let result = cleaner.clean("FILE_ID.DIZ", input, 4);
        assert_eq!(input, result.content.as_slice());
        assert_eq!(RuleAction::ReportOnly, result.changes[0].action);
        assert!(!result.needs_review);
    }

    #[test]
    fn empty_and_whitespace_files_without_a_match_do_not_require_review() {
        let cleaner = DescriptionCleaner::new(&[suffix_rule("footer", &[r"^advertisement$"])]).unwrap();
        for name in ["EMPTY.DAT", "SETTINGS.CFG", "FILE_ID.DIZ"] {
            for bytes in [b"".as_slice(), b" \r\n\t\n".as_slice()] {
                let result = cleaner.clean(name, bytes, 8);
                assert_eq!(bytes, result.content.as_slice());
                assert!(result.changes.is_empty());
                assert!(!result.needs_review);
            }
        }
        let result = cleaner.clean("FILE_ID.DIZ", b"Advertisement\r\n", 8);
        assert!(result.needs_review, "removing the entire description still requires review");
    }

    #[test]
    fn shipped_liquid_rule_preserves_the_raw_inline_prefix_and_line_ending() {
        let catalog = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/upload_ad_descriptions.toml");
        let empty = std::path::Path::new("");
        let rules = super::super::bbstro_fingerprint::FingerprintData::load_split(empty, &catalog).unwrap();
        for prefix in [
            b" :-----------------------------:".as_slice(),
            b"@X0F \x1b[31m\xda----\xbf  ".as_slice(),
            "Produkt: ü ".as_bytes(),
        ] {
            for ending in [b"\r\n".as_slice(), b"\n".as_slice()] {
                let mut input = b"Product description\r\n".to_vec();
                input.extend_from_slice(prefix);
                let mut expected = input.clone();
                expected.extend_from_slice(ending);
                input.extend_from_slice(b"I WAS FIRST ON LiQUiD'S WHQ, @USER@");
                input.extend_from_slice(ending);
                input.extend_from_slice(b"WHY DON'T YA ASK FOR JAPANESE BOARDS");
                input.extend_from_slice(ending);
                let result = rules.clean_description("FILE_ID.DIZ", &input, 8);
                assert_eq!(expected, result.content);
                assert!(!result.needs_review);
                assert_eq!(1, result.changes.len());
                assert_eq!("liquid-whq-footer", result.changes[0].rule_id);
                let noncanonical = rules.clean_description("P!-STRIP.DIZ", &input, 8);
                assert_eq!(input, noncanonical.content);
                assert!(noncanonical.changes.is_empty());
            }
        }
    }

    #[test]
    fn inline_footer_requires_the_complete_block_at_the_end() {
        let mut rule = suffix_rule("liquid", &[r"^i was first on liquid's whq, @user@$", r"^why don't ya ask for japanese boards$"]);
        rule.inline_start = Some("I WAS FIRST ON".into());
        let cleaner = DescriptionCleaner::new(&[rule.clone()]).unwrap();
        for input in [
            b"Product\n :---:I WAS FIRST ON LiQUiD'S WHQ, @USER@\nUnrelated text\n".as_slice(),
            b"Product\n :---:I WAS FIRST ON LiQUiD'S WHQ, @USER@\nWHY DON'T YA ASK FOR JAPANESE BOARDS\nLegitimate final line\n".as_slice(),
        ] {
            let result = cleaner.clean("FILE_ID.DIZ", input, 8);
            assert_eq!(input, result.content.as_slice());
            assert!(result.changes.is_empty());
        }
        let input = b"Product\n :---:I WAS FIRST ON LiQUiD'S WHQ, @USER@\nWHY DON'T YA ASK FOR JAPANESE BOARDS\n";
        let ambiguous = DescriptionCleaner::new(&[rule.clone(), rule]).unwrap().clean("FILE_ID.DIZ", input, 8);
        assert!(ambiguous.needs_review);
        assert_eq!(input, ambiguous.content.as_slice());
    }

    #[test]
    fn ambiguous_rules_require_review_without_modifying() {
        let cleaner = DescriptionCleaner::new(&[suffix_rule("first", &[r"^same footer$"]), suffix_rule("second", &[r"^same footer$"])]).unwrap();
        let input = b"Product\nSame footer\n";
        let result = cleaner.clean("FILE_ID.DIZ", input, 4);
        assert_eq!(input, result.content.as_slice());
        assert!(result.needs_review);
        assert!(result.changes.is_empty());
    }
}
