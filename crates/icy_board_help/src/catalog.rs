use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::Path,
};

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use rust_embed::RustEmbed;
use serde::Deserialize;

use crate::{Result, invalid, sha256};

#[derive(RustEmbed)]
#[folder = "data/en/"]
struct Embedded;

const CATALOG_TEXT: &str = include_str!("../data/catalog.toml");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub topic: String,
    pub markdown: String,
    /// SHA-256 of this source's exact UTF-8 Markdown bytes.
    pub source_hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    version: u32,
    topics: Vec<Topic>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Topic {
    id: String,
    locales: Vec<String>,
    source_hash: String,
    #[serde(default)]
    legacy_source_hash: String,
    #[serde(default)]
    translations: BTreeMap<String, Translation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Translation {
    upstream_hash: String,
    status: String,
    #[serde(default)]
    legacy_source_hash: String,
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn validate_topic(topic: &str) -> Result<()> {
    if !topic.starts_with("hlp")
        || !(4..=64).contains(&topic.len())
        || !topic
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'!' | b'@' | b'_' | b'-'))
    {
        return Err(invalid(format!("Invalid help topic: {topic:?}")));
    }
    Ok(())
}

/// `extension` is the language suffix, without a dot; English normally uses "".
pub fn output_name(topic: &str, extension: &str) -> Result<String> {
    validate_topic(topic)?;
    if !extension.is_empty()
        && (extension.len() > 32
            || !extension.as_bytes()[0].is_ascii_alphabetic()
            || !extension.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')))
    {
        return Err(invalid(format!("Invalid help language extension: {extension:?}")));
    }
    Ok(if extension.is_empty() {
        format!("{topic}.pcb")
    } else {
        format!("{topic}.{extension}.pcb")
    })
}

fn parse_catalog(text: &str) -> Result<Catalog> {
    let catalog: Catalog = toml::from_str(text)?;
    if catalog.version != 1 {
        return Err(invalid(format!("Unsupported help catalog version: {}", catalog.version)));
    }
    let mut ids = BTreeSet::new();
    for topic in &catalog.topics {
        validate_topic(&topic.id)?;
        if !ids.insert(&topic.id) {
            return Err(invalid(format!("Duplicate help topic: {}", topic.id)));
        }
        if !valid_hash(&topic.source_hash) || (!topic.legacy_source_hash.is_empty() && !valid_hash(&topic.legacy_source_hash)) {
            return Err(invalid(format!("Invalid source fingerprint for {}", topic.id)));
        }
        let mut locales = BTreeSet::new();
        for locale in &topic.locales {
            if !matches!(locale.as_str(), "en" | "de") || !locales.insert(locale.as_str()) {
                return Err(invalid(format!("Invalid or duplicate locale for {}: {locale}", topic.id)));
            }
        }
        if !locales.contains("en") {
            return Err(invalid(format!("Missing English locale for {}", topic.id)));
        }
        for (locale, translation) in &topic.translations {
            if locale != "de" || !locales.contains(locale.as_str()) {
                return Err(invalid(format!("Unexpected translation metadata: {}/{locale}", topic.id)));
            }
            let valid = match translation.status.as_str() {
                "reviewed" => valid_hash(&translation.upstream_hash),
                "legacy-unreviewed" | "unreviewed" => translation.upstream_hash.is_empty(),
                _ => false,
            };
            if !valid || (!translation.legacy_source_hash.is_empty() && !valid_hash(&translation.legacy_source_hash)) {
                return Err(invalid(format!("Invalid translation review metadata: {}/{locale}", topic.id)));
            }
        }
    }
    Ok(catalog)
}

fn embedded_text(name: &str) -> Result<String> {
    if name == "catalog.toml" {
        return Ok(CATALOG_TEXT.to_owned());
    }
    let asset = Embedded::get(name).ok_or_else(|| invalid(format!("Missing embedded help source: {name}")))?;
    Ok(std::str::from_utf8(&asset.data)?.to_owned())
}

fn body_word_count(events: &[Event<'_>]) -> usize {
    let mut body_depth = 0;
    let mut heading = false;
    let mut words = 0;
    for event in events {
        match event {
            Event::Start(Tag::Heading { .. }) => heading = true,
            Event::End(TagEnd::Heading(_)) => heading = false,
            // Tight list items have no paragraph tags; headings inside items still do not count.
            Event::Start(Tag::Paragraph | Tag::Item | Tag::CodeBlock(_)) => body_depth += 1,
            Event::End(TagEnd::Paragraph | TagEnd::Item | TagEnd::CodeBlock) => body_depth -= 1,
            Event::Text(text) | Event::Code(text) if body_depth > 0 && !heading => {
                // Whitespace and punctuation-only decoration are not explanatory text.
                words += text.split_whitespace().filter(|word| word.chars().any(char::is_alphanumeric)).count();
            }
            _ => {}
        }
    }
    words
}

fn validate_markdown(path: &str, markdown: &str) -> Result<()> {
    let events: Vec<_> = Parser::new(markdown).collect();
    if !matches!(events.first(), Some(Event::Start(Tag::Heading { level: HeadingLevel::H1, .. })))
        || events
            .iter()
            .filter(|e| matches!(e, Event::Start(Tag::Heading { level: HeadingLevel::H1, .. })))
            .count()
            != 1
        || !events
            .iter()
            .skip(1)
            .take_while(|e| !matches!(e, Event::End(TagEnd::Heading(HeadingLevel::H1))))
            .any(|e| matches!(e, Event::Text(text) | Event::Code(text) if !text.trim().is_empty()))
    {
        return Err(invalid(format!("{path}: expected exactly one nonempty level-one title at the start")));
    }
    if body_word_count(&events) == 0 {
        return Err(invalid(format!(
            "{path}: expected meaningful body text in a paragraph, list item or code block"
        )));
    }
    crate::render(
        markdown,
        &crate::RenderOptions {
            encoding: crate::Encoding::Utf8,
            ..Default::default()
        },
    )
    .map_err(|e| invalid(format!("{path}: {e}")))?;
    Ok(())
}

fn reject_symlink_components(path: &Path) -> Result<()> {
    for component in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        match fs::symlink_metadata(component) {
            Ok(meta) if meta.file_type().is_symlink() => return Err(invalid(format!("Symlink paths are not allowed: {}", component.display()))),
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn source_topic(name: &str, known: &BTreeSet<String>) -> Result<String> {
    let topic = name.strip_suffix(".md").ok_or_else(|| invalid(format!("Unexpected help source: {name}")))?;
    validate_topic(topic)?;
    if !known.contains(topic) {
        return Err(invalid(format!("Unknown help topic: {name}")));
    }
    Ok(topic.to_owned())
}

fn local_files(root: &Path, known: &BTreeSet<String>) -> Result<BTreeMap<String, String>> {
    reject_symlink_components(root)?;
    let root = root.canonicalize()?;
    if !root.is_dir() {
        return Err(invalid("Help overrides must be a directory"));
    }
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name().into_string().map_err(|_| invalid("Non-UTF-8 override filename"))?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            return Err(invalid(format!(
                "Help sources are now a flat directory of topic Markdown files; remove the {name:?} subdirectory and place catalog.toml and hlp*.md directly in the source directory"
            )));
        }
        if !kind.is_file() {
            return Err(invalid(format!("Help overrides must be regular files, not symlinks or directories: {name}")));
        }
        if name != "catalog.toml" {
            source_topic(&name, known)?;
        }
        files.insert(name, fs::read_to_string(entry.path())?);
    }
    Ok(files)
}

/// Load only English sources, replacing supplied topics with overrides from a flat source directory.
/// A local catalog may contain a topic subset and unused translation metadata.
pub fn sources(overrides: Option<&Path>) -> Result<Vec<Source>> {
    let catalog = parse_catalog(&embedded_text("catalog.toml")?)?;
    let known: BTreeSet<_> = catalog.topics.iter().map(|topic| topic.id.clone()).collect();
    let mut markdowns = BTreeMap::new();
    for topic in &catalog.topics {
        let name = format!("{}.md", topic.id);
        let markdown = embedded_text(&name)?;
        if sha256(markdown.as_bytes()) != topic.source_hash {
            return Err(invalid(format!("Embedded English fingerprint mismatch: {name}")));
        }
        markdowns.insert(name, markdown);
    }
    for name in Embedded::iter() {
        if !markdowns.contains_key(name.as_ref()) {
            return Err(invalid(format!("Uncatalogued embedded help file: {name}")));
        }
    }
    if let Some(root) = overrides {
        let mut local = local_files(root, &known)?;
        if let Some(text) = local.remove("catalog.toml") {
            for topic in parse_catalog(&text)?.topics {
                if !known.contains(&topic.id) {
                    return Err(invalid(format!("Unknown override topic: {}", topic.id)));
                }
            }
        }
        markdowns.extend(local);
    }
    let mut result = Vec::with_capacity(markdowns.len());
    for (name, markdown) in markdowns {
        let topic = source_topic(&name, &known)?;
        validate_markdown(&name, &markdown)?;
        let source_hash = sha256(markdown.as_bytes());
        result.push(Source { topic, markdown, source_hash });
    }
    result.sort_by(|a, b| a.topic.cmp(&b.topic));
    Ok(result)
}

/// Export English Markdown and the unchanged catalog as a flat bundle without replacing existing files.
pub fn export(destination: &Path) -> Result<()> {
    sources(None)?;
    reject_symlink_components(destination)?;
    match fs::symlink_metadata(destination) {
        Ok(meta) => {
            if !meta.is_dir() || fs::read_dir(destination)?.next().is_some() {
                return Err(invalid("Export destination must be a nonexistent or empty directory"));
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => fs::create_dir_all(destination)?,
        Err(e) => return Err(e.into()),
    }
    let mut names: Vec<_> = Embedded::iter().map(|name| name.into_owned()).collect();
    names.push("catalog.toml".to_owned());
    names.sort();
    for name in names {
        let mut file = OpenOptions::new().write(true).create_new(true).open(destination.join(&name))?;
        file.write_all(embedded_text(&name)?.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "icy-help-catalog-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn catalog_loads_only_68_english_sources() {
        let sources = sources(None).unwrap();
        assert_eq!(sources.len(), 68);
        assert_eq!(Embedded::iter().count(), 68);
        assert!(Embedded::iter().all(|path| !path.contains('/') && path.ends_with(".md")));
        assert!(Embedded::get("de/hlpa.md").is_none());
        assert!(embedded_text("de/hlpa.md").is_err());
        for source in &sources {
            assert_eq!(source.source_hash, sha256(source.markdown.as_bytes()));
            crate::render(&source.markdown, &crate::RenderOptions::default()).unwrap();
        }
    }

    #[test]
    fn every_builtin_source_has_substantive_body_text() {
        let sources = sources(None).unwrap();
        assert_eq!(sources.len(), 68);
        for source in sources {
            let events: Vec<_> = Parser::new(&source.markdown).collect();
            let words = body_word_count(&events);
            assert!(words >= 10, "{}: only {words} meaningful body words", source.topic);
        }
    }

    #[test]
    fn catalog_covers_original_pcboard_help_and_icy_board_additions() {
        // Golden suffixes from pcboard/pcb-main/SOURCE/DISPLAY/HELP.C, displayhelpfile().
        let suffixes = [
            "brd", "open", "chat", "cmenu", "endr", "flag", "fscrn", "lang", "r", "news", "qwk", "reg", "rep", "rm", "sec", "sel", "srch", "test", "ts",
            "users", "who", "!", "alias",
        ];
        assert_eq!(suffixes.len(), 23);
        let original: BTreeSet<_> = ('a'..='z')
            .map(|letter| format!("hlp{letter}"))
            .chain((1..=15).map(|number| format!("hlp{number}")))
            .chain(suffixes.into_iter().map(|suffix| format!("hlp{suffix}")))
            .collect();
        // HLP_MSG maps to HLPR, already included in the letter range.
        assert_eq!(original.len(), 63);
        let actual: BTreeSet<_> = parse_catalog(CATALOG_TEXT).unwrap().topics.into_iter().map(|topic| topic.id).collect();
        assert!(
            original.is_subset(&actual),
            "Missing PCBoard help: {:?}",
            original.difference(&actual).collect::<Vec<_>>()
        );
        let additions = BTreeSet::from(["hlp16", "hlp@", "hlp@w", "hlparea", "hlpppe"].map(str::to_owned));
        assert!(original.is_disjoint(&additions));
        let expected: BTreeSet<_> = original.union(&additions).cloned().collect();
        assert_eq!(expected.len(), 68);
        assert_eq!(actual, expected);
        let embedded: BTreeSet<_> = Embedded::iter().map(|name| name.strip_suffix(".md").unwrap().to_owned()).collect();
        assert_eq!(embedded, expected);
    }

    #[test]
    fn output_names_reject_traversal_and_suffix_injection() {
        assert_eq!(output_name("hlpa", "").unwrap(), "hlpa.pcb");
        assert_eq!(output_name("hlpa", "de").unwrap(), "hlpa.de.pcb");
        assert_eq!(output_name("hlpa", "ger").unwrap(), "hlpa.ger.pcb");
        assert_eq!(output_name("hlp!", "de").unwrap(), "hlp!.de.pcb");
        assert_eq!(output_name("hlp@w", "").unwrap(), "hlp@w.pcb");
        for topic in ["../hlpa", "hlpa/evil", "hlpa\\evil", "hlpa.pcb", "/hlpa", "hlpa\0", "hlpa:"] {
            assert!(output_name(topic, "de").is_err(), "{topic:?}");
        }
        for extension in ["../ger", ".ger", "ger/evil", "ger\\evil", "ger.pcb", "ger\0"] {
            assert!(output_name("hlpa", extension).is_err(), "{extension:?}");
        }
    }

    #[test]
    fn partial_overrides_fall_back_to_embedded_sources() {
        let temp = Temp::new();
        fs::write(temp.0.join("hlparea.md"), "# Local help\n\nText.\n").unwrap();
        let updated = sources(Some(&temp.0)).unwrap();
        assert_eq!(updated.len(), 68);
        assert_eq!(updated.iter().find(|s| s.topic == "hlparea").unwrap().markdown, "# Local help\n\nText.\n");
        let original = sources(None).unwrap();
        assert!(original.iter().filter(|s| s.topic != "hlparea").all(|s| updated.contains(s)));
        fs::write(temp.0.join("hlpunknown.md"), "# Unknown\n").unwrap();
        assert!(sources(Some(&temp.0)).is_err());
    }

    #[test]
    fn subdirectories_in_the_source_directory_are_rejected() {
        let temp = Temp::new();
        export(&temp.0).unwrap();
        for name in ["en", "de", "nested"] {
            let directory = temp.0.join(name);
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("hlpa.md"), "# Local help\n").unwrap();
            let error = sources(Some(&temp.0)).unwrap_err().to_string();
            assert!(error.contains("flat directory of topic Markdown files"), "{error}");
            assert!(error.contains(name), "{error}");
            fs::remove_dir_all(&directory).unwrap();
        }
        assert!(sources(Some(&temp.0)).is_ok());
    }

    #[test]
    fn unexpected_markdown_and_invalid_titles_are_rejected() {
        let temp = Temp::new();
        fs::write(temp.0.join("stray.md"), "# Stray\n").unwrap();
        assert!(sources(Some(&temp.0)).is_err());
        fs::remove_file(temp.0.join("stray.md")).unwrap();
        fs::write(temp.0.join("hlpa.txt"), "# Stray\n").unwrap();
        assert!(sources(Some(&temp.0)).is_err());
        fs::remove_file(temp.0.join("hlpa.txt")).unwrap();
        for text in ["No title", "#\n", "# First\n\n# Second", "body\n\n# Title", "```\n# Not a title\n```"] {
            fs::write(temp.0.join("hlpa.md"), text).unwrap();
            assert!(sources(Some(&temp.0)).is_err(), "{text:?}");
        }
    }

    #[test]
    fn markdown_without_meaningful_body_is_rejected() {
        let temp = Temp::new();
        for body in [
            "",
            " \n\t\n",
            "## Instructions\n\n### More instructions\n",
            "## `Command`\n",
            "- ## Heading inside a list\n",
            "> ## Heading inside a quote\n",
            "<!-- Explanatory words hidden in a comment -->\n",
            "---\n\n***\n\n___\n",
            "... !!! ---\n",
            "**...**\n",
            "&nbsp;\n",
            "-\n-\n",
            "1.\n2.\n",
            "- ...\n  - !!!\n",
            ">\n",
            "` `\n",
            "`---`\n",
            "```text\n```\n",
            "```text\n  \n\t\n```\n",
            "```text\n--- ***\n```\n",
        ] {
            let markdown = format!("# Valid title\n\n{body}");
            fs::write(temp.0.join("hlpa.md"), &markdown).unwrap();
            let error = sources(Some(&temp.0)).unwrap_err().to_string();
            assert!(error.contains("hlpa.md: expected meaningful body text"), "{body:?}: {error}");
        }
    }

    #[test]
    fn meaningful_paragraph_list_and_code_bodies_are_accepted() {
        for body in [
            "Use this command.\n",
            "## Instructions\n\nUse this command.\n",
            "**Read** the instructions.\n",
            "- Read the instructions.\n- Return to the board.\n",
            "- Read the instructions.\n\n- Return to the board.\n",
            "1. Read the instructions.\n   - Return to the board.\n",
            "- ## Instructions\n\n  Read the instructions.\n",
            "> Read the instructions.\n",
            "`HELP`\n",
            "```text\nHELP\n```\n",
            "Überprüfen Sie die Einstellungen.\n",
        ] {
            let markdown = format!("# Valid title\n\n{body}");
            validate_markdown("hlpa.md", &markdown).unwrap_or_else(|error| panic!("{body:?}: {error}"));
        }
    }

    #[test]
    fn exported_bundle_roundtrips_and_refuses_existing_files() {
        let temp = Temp::new();
        let destination = temp.0.join("bundle");
        export(&destination).unwrap();
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 69);
        assert!(fs::read_dir(&destination).unwrap().all(|entry| entry.unwrap().file_type().unwrap().is_file()));
        assert!(!destination.join("en").exists() && !destination.join("de").exists());
        assert!(destination.join("hlpa.md").is_file());
        assert_eq!(fs::read_to_string(destination.join("catalog.toml")).unwrap(), CATALOG_TEXT);
        assert_eq!(sources(None).unwrap(), sources(Some(&destination)).unwrap());
        assert!(export(&destination).is_err());
        fs::write(temp.0.join("file"), "preserve").unwrap();
        assert!(export(&temp.0.join("file")).is_err());
        assert_eq!(fs::read_to_string(temp.0.join("file")).unwrap(), "preserve");
        let empty = temp.0.join("empty");
        fs::create_dir(&empty).unwrap();
        export(&empty).unwrap();
    }

    #[test]
    fn override_catalog_keeps_translation_metadata_optional() {
        let temp = Temp::new();
        let original = sources(None).unwrap();
        let hash = &original.iter().find(|s| s.topic == "hlpa").unwrap().source_hash;
        let metadata = format!(
            "version = 1\n[[topics]]\nid = \"hlpa\"\nlocales = [\"en\", \"de\"]\nsource_hash = \"{hash}\"\n[topics.translations.de]\nstatus = \"reviewed\"\nupstream_hash = \"{hash}\"\n"
        );
        fs::write(temp.0.join("catalog.toml"), &metadata).unwrap();
        assert_eq!(sources(Some(&temp.0)).unwrap(), original);
        let changed = "# Changed English\n\nUpdated command instructions.\n";
        fs::write(temp.0.join("hlpa.md"), changed).unwrap();
        let updated = sources(Some(&temp.0)).unwrap();
        assert_eq!(updated.len(), 68);
        assert_eq!(updated.iter().find(|s| s.topic == "hlpa").unwrap().source_hash, sha256(changed.as_bytes()));
        fs::write(
            temp.0.join("catalog.toml"),
            metadata.replace("status = \"reviewed\"", "status = \"legacy-unreviewed\""),
        )
        .unwrap();
        assert!(sources(Some(&temp.0)).is_err());
    }

    #[test]
    fn catalog_preserves_and_validates_source_provenance() {
        let catalog = parse_catalog(CATALOG_TEXT).unwrap();
        assert_eq!(catalog.topics.len(), 68);
        let numeric: BTreeSet<_> = (1..=16).map(|number| format!("hlp{number}")).collect();
        let new: Vec<_> = catalog.topics.iter().filter(|topic| numeric.contains(&topic.id)).collect();
        assert_eq!(new.len(), 16);
        assert!(
            new.iter()
                .all(|topic| topic.locales == ["en"] && topic.legacy_source_hash.is_empty() && topic.translations.is_empty())
        );
        let legacy: Vec<_> = catalog.topics.iter().filter(|topic| !numeric.contains(&topic.id)).collect();
        assert_eq!(legacy.len(), 52);
        assert!(legacy.iter().all(|topic| valid_hash(&topic.legacy_source_hash)));
        let translations: Vec<_> = catalog.topics.iter().flat_map(|topic| topic.translations.values()).collect();
        assert_eq!(translations.len(), 20);
        assert!(translations.iter().all(|translation| {
            translation.status == "legacy-unreviewed" && translation.upstream_hash.is_empty() && valid_hash(&translation.legacy_source_hash)
        }));
        let topic = &catalog.topics[0];
        for hash in [&topic.source_hash, &topic.legacy_source_hash, &topic.translations["de"].legacy_source_hash] {
            assert!(parse_catalog(&CATALOG_TEXT.replacen(hash, "invalid", 1)).is_err());
        }
        assert!(parse_catalog(&CATALOG_TEXT.replacen("locales = [\"en\", \"de\"]", "locales = [\"de\"]", 1)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_files_and_directories_are_rejected() {
        use std::os::unix::fs::symlink;
        let temp = Temp::new();
        let outside = Temp::new();
        fs::write(outside.0.join("hlpa.md"), "# Outside\n").unwrap();
        symlink(outside.0.join("hlpa.md"), temp.0.join("hlpa.md")).unwrap();
        assert!(sources(Some(&temp.0)).is_err());
        fs::remove_file(temp.0.join("hlpa.md")).unwrap();
        symlink(&outside.0, temp.0.join("linked")).unwrap();
        assert!(sources(Some(&temp.0)).is_err());
        assert!(export(&temp.0.join("linked/new")).is_err());
        assert!(sources(Some(&temp.0.join("linked"))).is_err());
    }
}
