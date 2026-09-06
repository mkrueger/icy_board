use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

use dizbase::file_base_scanner::{
    bbstro_fingerprint::FingerprintData,
    repack::{RepackOptions, Repacked, repack_file},
};

const TEXT: &[u8] = b"Example Board advertisement\r\nuploaded 01-02-1994\r\n";
fn rule(id: &str, action: &str) -> String {
    format!(
        r#"
[[text_member_rule]]
id = '{id}'
action = '{action}'
max_bytes = 1024
lines = [
  {{ literal = 'Example Board advertisement' }},
  {{ regex = 'uploaded [0-9]{{2}}-[0-9]{{2}}-[0-9]{{4}}' }},
]
"#
    )
}
fn load(dir: &Path, text: &str) -> FingerprintData {
    let path = dir.join("rules.toml");
    fs::write(&path, text).unwrap();
    FingerprintData::load_split(&path, Path::new("")).unwrap()
}
fn archive(path: &Path, content: &[u8]) {
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, bytes) in [("random-name.nfo", content), ("README.TXT", b"Original software manual".as_slice())] {
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.set_raw_comment(b"original comment".to_vec().into_boxed_slice()).unwrap();
    zip.finish().unwrap();
}
fn options() -> RepackOptions {
    RepackOptions {
        lowercase_names: false,
        recompress: false,
        ..Default::default()
    }
}

#[test]
fn text_only_catalog_load_save_disable_and_validation() {
    let dir = tempfile::tempdir().unwrap();
    let rules = load(dir.path(), &rule("example", "report_only"));
    assert!(!rules.is_empty());
    assert!(!rules.is_match("x", TEXT), "the legacy bool must not auto-remove report-only text");
    assert_eq!(rules.match_text_member("x", TEXT).len(), 1);
    let saved = dir.path().join("saved.toml");
    rules.save(&saved).unwrap();
    assert_eq!(FingerprintData::load(&saved).unwrap().match_text_member("x", TEXT).len(), 1);
    assert!(FingerprintData::load_split(Path::new(""), &saved).is_err());
    assert!(FingerprintData::load_split(Path::new(""), Path::new("")).unwrap().is_empty());
    let invalid = rule("example", "report_only").replace("uploaded [0-9]{2}-[0-9]{2}-[0-9]{4}", "[");
    fs::write(&saved, invalid).unwrap();
    assert!(FingerprintData::load_split(&saved, Path::new("")).is_err());
    assert!(FingerprintData::load(&saved).is_err());
}

#[test]
fn report_only_retains_archive_bytes_without_forcing_review_or_repack() {
    let dir = tempfile::tempdir().unwrap();
    let rules = load(dir.path(), &rule("example", "report_only"));
    let path = dir.path().join("test.zip");
    archive(&path, TEXT);
    let before = fs::read(&path).unwrap();
    let Repacked::Reported { text_members } = repack_file(&path, &rules, &options()).unwrap() else {
        panic!("expected report")
    };
    assert_eq!(text_members.len(), 1);
    assert!(text_members[0].to_string().contains("sha256="));
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut opts = options();
    opts.recompress = true;
    let Repacked::Converted { text_members, removed, .. } = repack_file(&path, &rules, &opts).unwrap() else {
        panic!("expected repack")
    };
    assert_eq!(text_members.len(), 1);
    assert!(removed.is_empty());
    let mut zip = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
    let mut kept = Vec::new();
    zip.by_name("random-name.nfo").unwrap().read_to_end(&mut kept).unwrap();
    assert_eq!(kept, TEXT);
    assert_eq!(zip.comment(), b"original comment");
}

#[test]
fn auto_clean_is_switch_controlled_and_dry_run_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let rules = load(dir.path(), &rule("example", "auto_clean"));
    let path = dir.path().join("test.zip");
    archive(&path, TEXT);
    let before = fs::read(&path).unwrap();
    let mut opts = options();
    opts.remove_advertisements = false;
    assert!(matches!(repack_file(&path, &rules, &opts).unwrap(), Repacked::Unchanged));
    opts.remove_advertisements = true;
    opts.dry_run = true;
    assert!(matches!(repack_file(&path, &rules, &opts).unwrap(), Repacked::Converted { .. }));
    assert_eq!(fs::read(&path).unwrap(), before);
    opts.dry_run = false;
    let Repacked::Converted { removed, text_members, .. } = repack_file(&path, &rules, &opts).unwrap() else {
        panic!("expected removal")
    };
    assert_eq!(removed, ["random-name.nfo"]);
    assert_eq!(text_members.len(), 1);
    let mut zip = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
    assert!(zip.by_name("random-name.nfo").is_err());
    assert!(zip.by_name("README.TXT").is_ok());
}

#[test]
fn review_ambiguity_and_extra_material_preserve_original() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.zip");
    archive(&path, TEXT);
    let before = fs::read(&path).unwrap();
    for text in [rule("example", "review"), rule("one", "auto_clean") + &rule("two", "report_only")] {
        let rules = load(dir.path(), &text);
        let Repacked::NeedsReview { reason } = repack_file(&path, &rules, &options()).unwrap() else {
            panic!("expected review")
        };
        assert!(reason.contains("sha256="));
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    let rules = load(dir.path(), &rule("example", "auto_clean"));
    for extra in [b"Original documentation\n".as_slice(), b"\x1ahidden material"] {
        let bytes = [TEXT, extra].concat();
        archive(&path, &bytes);
        let before = fs::read(&path).unwrap();
        assert!(matches!(repack_file(&path, &rules, &options()).unwrap(), Repacked::Unchanged));
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn shipped_templates_match_audit_examples_but_not_extra_documentation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = FingerprintData::load_split(&root.join("assets/upload_ad_files.toml"), Path::new("")).unwrap();
    let variants = include_str!("../../../assets/archive_text_audit/variants.md");
    let dynamic = include_str!("../../../assets/archive_text_audit/dynamic-generator.md");
    for (source, hash, id) in [
        (
            variants,
            "ec3248d64a1c16f212bff4c24d7310fd24a41a934c82ce517744fc500def47c1",
            "clipper-workshop-filenet-nine-nodes",
        ),
        (
            variants,
            "402b6f789261495dad1b11a5b5b10927435c4acd2ffb96deaa719501ed99c902",
            "cutting-edge-online-extended-offer",
        ),
        (
            variants,
            "b1761e8d0edefc60353e57f91c05d31f9196ec47b46eaeeb94297e49cf8f95bd",
            "bbs-archives-passed-banner",
        ),
        (
            dynamic,
            "4cc5133e44c4e8cb1917c80b6e817957be17873d2ab4d773cb973ea793021f66",
            "buggerer-stadium-footer-102",
        ),
    ] {
        let text = source
            .split_once(hash)
            .unwrap()
            .1
            .split_once("<pre>")
            .unwrap()
            .1
            .split_once("</pre>")
            .unwrap()
            .0
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        let cp437: Vec<_> = text
            .chars()
            .map(|c| {
                if c.is_ascii() {
                    c as u8
                } else {
                    codepages::tables::CP437_TO_UNICODE.iter().position(|v| *v == c).unwrap() as u8
                }
            })
            .collect();
        for bytes in [text.as_bytes(), cp437.as_slice()] {
            let matches = rules.match_text_member("renamed.nfo", bytes);
            assert_eq!(matches.len(), 1, "{id}");
            assert_eq!(matches[0].rule_id, id);
            assert_eq!(matches[0].action, dizbase::file_base_scanner::description_cleaner::RuleAction::AutoClean);
            assert!(rules.match_text_member("README.TXT", &[bytes, b"\nOriginal manual"].concat()).is_empty());
        }
        if id == "buggerer-stadium-footer-102" {
            let changed = text.replace(
                "cia-ftr3.zip 6426 bytes uploaded at 19:17 on 01-28-94 on node 3",
                "newfile.zip 99999 bytes uploaded at 08:25 on 09-06-26 on node 12",
            );
            assert_ne!(changed, text);
            assert_eq!(rules.match_text_member("different.txt", changed.as_bytes()).len(), 1);
        }
    }
}
