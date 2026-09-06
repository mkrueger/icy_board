//! Portable regressions for the reviewed PCBoard catalog; no local corpus required.
use dizbase::file_base_scanner::description_cleaner::{DescriptionBlockRule, DescriptionCleaner, RuleAction};
use serde::Deserialize;

#[derive(Deserialize)]
struct Rules {
    description_rule: Vec<DescriptionBlockRule>,
}

fn catalogs() -> (Rules, Rules) {
    (
        toml::from_str(include_str!("../../../assets/upload_ad_descriptions.toml")).unwrap(),
        toml::from_str(include_str!("../../../assets/ad_corpus_rules/upload_ad_descriptions.toml")).unwrap(),
    )
}

#[test]
fn curated_literal_blocks_preserve_original_bytes_and_require_boundaries() {
    let (defaults, combined) = catalogs();
    let cleaners = [
        DescriptionCleaner::new(&defaults.description_rule).unwrap(),
        DescriptionCleaner::new(&combined.description_rule).unwrap(),
    ];
    let original = b"\xda Original program description\r\n";
    for rule in defaults
        .description_rule
        .iter()
        .filter(|r| !r.literal_lines.is_empty() && r.inline_start.is_none())
    {
        let block = format!("{}\r\n", rule.literal_lines.join("\r\n"));
        let input = [original.as_slice(), block.as_bytes()].concat();
        for cleaner in &cleaners {
            let result = cleaner.clean("subdir/FILE_ID.DIZ", &input, 8);
            assert!(!result.needs_review, "{}", rule.id);
            assert_eq!(result.changes.len(), 1, "{}", rule.id);
            assert_eq!(result.changes[0].rule_id, rule.id);
            assert_eq!(
                result.content,
                if rule.action == RuleAction::AutoClean {
                    original.to_vec()
                } else {
                    input.clone()
                }
            );
            for name in ["README.TXT", "STRIP.DIZ", "P!-STRIP.DIZ"] {
                assert!(cleaner.clean(name, &input, 8).changes.is_empty(), "{}: {name}", rule.id);
            }
            let middle = [input.as_slice(), b"Original final line\r\n"].concat();
            assert!(cleaner.clean("FILE_ID.DIZ", &middle, 8).changes.is_empty(), "{}", rule.id);
        }
        // Every literal must match completely, not merely contain a board name.
        let mut altered = rule.literal_lines.clone();
        altered.last_mut().unwrap().push_str(" changed");
        let altered = format!("Original program\r\n{}\r\n", altered.join("\r\n"));
        let single = DescriptionCleaner::new(std::slice::from_ref(rule)).unwrap();
        assert!(single.clean("FILE_ID.DIZ", altered.as_bytes(), 8).changes.is_empty(), "{}", rule.id);
    }
}

#[test]
fn critical_strike_requires_complete_block_but_allows_other_upload_times() {
    let (defaults, combined) = catalogs();
    let original = b"\xb3 Program box and author credits\r\n";
    for rules in [defaults, combined] {
        let cleaner = DescriptionCleaner::new(&rules.description_rule).unwrap();
        for timestamp in ["04.30.95 AT 23:42", "12.01.96 AT 01:09"] {
            let block = format!(
                "φ HΣY LÄMεΓ! φ\r\n┌────────────────────────── · · · · · ·\r\n│ THiS FiLE PASSΣD THRU (ΓiTi[AL STΓiKε\r\n│ ÜPLφADΣD φ∩ {timestamp}\r\n│ CALL NΘW - (2o1)535-3902\r\n└────────────────────────── · · · · · ·\r\n"
            );
            let input = [original.as_slice(), block.as_bytes()].concat();
            let result = cleaner.clean("FILE_ID.DIZ", &input, 8);
            assert_eq!(result.content, original);
            assert_eq!(result.changes[0].rule_id, "critical-strike-transit");
            assert!(!result.needs_review);
            let incomplete = block.split_once('\n').unwrap().1;
            for negative in [
                block.replace("535-3902", "535-9999"),
                incomplete.to_owned(),
                block.replace(timestamp, "yesterday"),
            ] {
                let input = [original.as_slice(), negative.as_bytes()].concat();
                assert!(cleaner.clean("FILE_ID.DIZ", &input, 8).changes.is_empty());
            }
        }
    }
}

#[test]
fn stacked_ads_stop_at_report_only_and_liquid_preserves_inline_border() {
    let (defaults, _) = catalogs();
    let cleaner = DescriptionCleaner::new(&defaults.description_rule).unwrap();
    let block = |id: &str| {
        format!(
            "{}\r\n",
            defaults.description_rule.iter().find(|r| r.id == id).unwrap().literal_lines.join("\r\n")
        )
    };
    let original = b"\xc0\xc4 Program description\r\n";
    let ads = format!("{}{}", block("digital-delusions-transit"), block("high-tech-couriers-95"));
    let input = [original.as_slice(), ads.as_bytes()].concat();
    let result = cleaner.clean("FILE_ID.DIZ", &input, 8);
    assert_eq!(result.content, original);
    assert_eq!(result.changes.len(), 2);
    assert!(cleaner.clean("FILE_ID.DIZ", &result.content, 8).changes.is_empty());

    let ads = format!("{}{}", block("rts-couriers"), block("limpy-crack-mark"));
    let input = [original.as_slice(), ads.as_bytes()].concat();
    let result = cleaner.clean("FILE_ID.DIZ", &input, 8);
    assert_eq!(result.content, input);
    assert_eq!(result.changes.len(), 1);
    assert_eq!(result.changes[0].action, RuleAction::ReportOnly);

    let border = b"Original program\r\n\xc0\xc4\xd9";
    let input = [border.as_slice(), block("liquid-whq-footer").as_bytes()].concat();
    let result = cleaner.clean("FILE_ID.DIZ", &input, 8);
    // The original first-line CRLF belongs to the preserved box border.
    assert_eq!(result.content, [border.as_slice(), b"\r\n"].concat());
    assert_eq!(result.changes[0].rule_id, "liquid-whq-footer");
}

#[test]
fn author_support_registration_and_generic_dictionary_markers_are_not_ads() {
    let (defaults, combined) = catalogs();
    for rules in [defaults, combined] {
        let cleaner = DescriptionCleaner::new(&rules.description_rule).unwrap();
        for original in [
            "pcboard 15.2x (C) POPIT SOFTWARES\r\nMessage tools\r\nU/L: POPIT 714-733-3521\r\n",
            "Programed By Cyberspace Entertainment at ALTERED EGO\r\n",
            "Original author: PWA\r\nSupport: The Wizard's BBS\r\nRegistration $20\r\n",
            "STRiP/DiZ TEST FiLE_iD :)\r\nCOURIER\r\nUPLOADED BY\r\nSPREAD BY\r\n",
            "Archive verification status\r\nCRC OK\r\nF-PROT tested\r\n",
        ] {
            let result = cleaner.clean("FILE_ID.DIZ", original.as_bytes(), 8);
            assert_eq!(result.content, original.as_bytes());
            assert!(result.changes.is_empty());
            assert!(!result.needs_review);
        }
    }
}
