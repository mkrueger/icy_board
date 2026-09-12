use codepages::tables::CP437_TO_UNICODE;
use icy_board_help::{Encoding, HelpTheme, RenderOptions, Role, document, render, sha256};

const ENGLISH: &str = "# Help\n\nRead **messages** and use `Enter` to continue. Visit [the project](https://example.org/help).\n\n## Commands\n\n- Read messages\n  - Browse older messages\n  - Search by subject\n- Write a message\n\n> Keep your password *private*.\n";
const GERMAN: &str = "# Hilfe\n\nLies **Nachrichten** und drücke `Enter`, um fortzufahren. Grüße für alle Gäste.\n\n## Befehle\n\n1. Nachrichten lesen\n   - Ältere Nachrichten öffnen\n2. Eine Nachricht schreiben\n\n> Halte dein Passwort *geheim*.\n";

fn decoded(bytes: &[u8], encoding: Encoding) -> String {
    match encoding {
        Encoding::Cp437 => bytes.iter().map(|&byte| CP437_TO_UNICODE[byte as usize]).collect(),
        Encoding::Utf8 => String::from_utf8(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).expect("UTF-8 BOM").to_vec()).unwrap(),
    }
}

// This validates the generator grammar, not the engine's display-file execution path.
fn visible_generated(bytes: &[u8], encoding: Encoding) -> String {
    let source = decoded(bytes, encoding);
    let mut remaining = source.as_str();
    let mut text = String::new();
    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix("@@XFF") {
            text.push('@');
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("@CLS@") {
            remaining = rest;
        } else if remaining.starts_with("@X") {
            assert!(remaining.as_bytes()[2..4].iter().all(u8::is_ascii_hexdigit));
            remaining = &remaining[4..];
        } else {
            let ch = remaining.chars().next().unwrap();
            assert_ne!(ch, '@', "Unexpected executable macro in generator output");
            text.push(ch);
            remaining = &remaining[ch.len_utf8()..];
        }
    }
    text.replace("\r\n", "\n")
}

#[test]
fn defaults_and_theme_serialization() {
    let options = RenderOptions::default();
    assert_eq!(options.width, 79);
    assert_eq!(options.encoding, Encoding::Utf8);
    assert_eq!(Encoding::default(), Encoding::Utf8);
    assert!(options.clear_screen);
    assert_eq!(options.theme, HelpTheme::preset("classic").unwrap());
    for preset in ["classic", "minimal"] {
        let theme = HelpTheme::preset(preset).unwrap();
        let serialized = toml::to_string(&theme).unwrap();
        assert_eq!(toml::from_str::<HelpTheme>(&serialized).unwrap(), theme);
    }
    assert_eq!(toml::from_str::<HelpTheme>("").unwrap(), HelpTheme::default());
    assert!(toml::from_str::<HelpTheme>("script = '@USER@'").is_err());
    assert!(HelpTheme::preset("unknown").is_err());
    #[derive(serde::Deserialize)]
    struct Config {
        encoding: Encoding,
    }
    assert_eq!(toml::from_str::<Config>("encoding = 'cp437'").unwrap().encoding, Encoding::Cp437);
    assert_eq!(toml::from_str::<Config>("encoding = 'utf8'").unwrap().encoding, Encoding::Utf8);
}

#[test]
fn both_languages_fit_every_supported_width_and_encoding() {
    for source in [ENGLISH, GERMAN] {
        for width in 40..=79 {
            for encoding in [Encoding::Cp437, Encoding::Utf8] {
                let options = RenderOptions {
                    width,
                    encoding,
                    ..RenderOptions::default()
                };
                let output = render(source, &options).unwrap();
                let visible = visible_generated(&output.bytes, encoding);
                assert!(visible.lines().all(|line| line.chars().count() <= width), "width {width}: {visible}");
                assert!(output.plain_text.lines().all(|line| line.chars().count() <= width));
                for line in decoded(&output.bytes, encoding).split("\r\n") {
                    assert!(line.starts_with("@X"), "unguarded line: {line}");
                }
            }
        }
    }
    assert!(render(GERMAN, &RenderOptions::default()).unwrap().plain_text.contains("Grüße"));
}

#[test]
fn themes_change_bytes_but_not_semantic_plain_text() {
    for source in [ENGLISH, GERMAN] {
        let classic = render(source, &RenderOptions::default()).unwrap();
        let minimal = render(
            source,
            &RenderOptions {
                theme: HelpTheme::preset("minimal").unwrap(),
                ..RenderOptions::default()
            },
        )
        .unwrap();
        assert_ne!(classic.bytes, minimal.bytes);
        assert_eq!(classic.plain_text, minimal.plain_text);
        assert_eq!(visible_generated(&minimal.bytes, Encoding::Utf8), minimal.plain_text);
        assert!(visible_generated(&classic.bytes, Encoding::Utf8).contains("===\n"));
    }
}

#[test]
fn all_roles_remain_semantic_until_serialization() {
    let source = "# Title\n\n## Heading\n\nBody *emphasis* and `code`.\n\n> Note\n";
    let document = document::compile(source, 79, &HelpTheme::default()).unwrap();
    for role in [Role::Title, Role::Heading, Role::Body, Role::Emphasis, Role::Code, Role::Note, Role::Border] {
        assert!(document.lines.iter().flat_map(|line| &line.spans).any(|span| span.role == role), "{role:?}");
    }
}

#[test]
fn literal_macros_and_dispatch_characters_cannot_become_commands() {
    let source = "!door.ppe\n\n$include\n\n%script\n\n@USER@ @CLS@ @X01 @X00 @XFF @@ @ @URL:evil@\n\n`@USER@`\n\n```\n!door.ppe\n$include\n%script\n@USER@ @CLS@ @X01\n```\n";
    let options = RenderOptions {
        theme: HelpTheme::preset("minimal").unwrap(),
        ..RenderOptions::default()
    };
    let rendered = render(source, &options).unwrap();
    let wire = decoded(&rendered.bytes, options.encoding);
    assert_eq!(wire.matches("@CLS@").count(), 1);
    assert!(!wire.contains("@USER@"));
    assert!(!wire.contains("@X01"));
    assert!(wire.contains("@@XFFUSER@@XFF"));
    assert_eq!(visible_generated(&rendered.bytes, options.encoding), rendered.plain_text);
    assert!(rendered.plain_text.contains("@USER@ @CLS@ @X01"));
    for line in wire.split("\r\n") {
        assert!(line.starts_with("@X"));
    }
}

#[test]
fn escaping_happens_after_long_word_wrapping() {
    let text = format!("{}@USER@{}", "a".repeat(38), "b".repeat(91));
    let options = RenderOptions {
        width: 40,
        theme: HelpTheme::preset("minimal").unwrap(),
        ..RenderOptions::default()
    };
    let result = render(&text, &options).unwrap();
    assert_eq!(result.plain_text.lines().collect::<String>(), text);
    assert_eq!(visible_generated(&result.bytes, Encoding::Utf8), result.plain_text);
    assert!(result.plain_text.lines().all(|line| line.chars().count() <= 40));
}

#[test]
fn wrapping_preserves_words_and_inline_code_spaces() {
    let text = "One two three four five six seven eight nine ten eleven twelve thirteen fourteen.";
    let options = RenderOptions {
        width: 40,
        ..RenderOptions::default()
    };
    let result = render(text, &options).unwrap();
    assert_eq!(
        result.plain_text.split_whitespace().collect::<Vec<_>>(),
        text.split_whitespace().collect::<Vec<_>>()
    );
    assert!(render("Use `a  b` now.", &options).unwrap().plain_text.contains("a  b"));
    assert_eq!(
        render("A soft\nbreak and a hard  \nbreak.", &options).unwrap().plain_text,
        "A soft break and a hard\nbreak.\n"
    );
}

#[test]
fn fenced_code_preserves_whitespace_and_expands_tabs_at_four_cells() {
    let text = "```text\n  first  \n\n\tsecond\n a\tb\n```";
    let result = render(text, &RenderOptions::default()).unwrap();
    assert_eq!(result.plain_text, "  first  \n\n    second\n a  b\n");
    assert_eq!(visible_generated(&result.bytes, Encoding::Utf8), result.plain_text);
    assert_eq!(render("```\na\n\n\n```", &RenderOptions::default()).unwrap().plain_text, "a\n\n\n");
    assert!(render("a\tb", &RenderOptions::default()).is_err());
    assert!(render("\toutside code", &RenderOptions::default()).is_err());
    assert!(render("`a\tb`", &RenderOptions::default()).is_ok());
}

#[test]
fn oversized_code_is_an_error_not_silently_wrapped() {
    let options = RenderOptions {
        width: 40,
        ..RenderOptions::default()
    };
    assert!(render(&format!("```\n{}\n```", "x".repeat(40)), &options).is_ok());
    let err = render(&format!("```\n{}\n```", "x".repeat(41)), &options).unwrap_err();
    assert!(err.to_string().contains("Code line"));
    assert!(render(&format!("> ```\n> {}\n> ```", "x".repeat(39)), &options).is_err());
}

#[test]
fn nested_lists_keep_hanging_indentation_and_numbers() {
    let source = "3. First entry\n   - Nested entry with many words that should wrap onto a continuation line\n   - Next nested entry\n4. Last entry\n";
    let options = RenderOptions {
        width: 40,
        ..RenderOptions::default()
    };
    let text = render(source, &options).unwrap().plain_text;
    assert!(text.contains("3. First entry\n"), "{text}");
    assert!(text.contains("   - Nested entry"), "{text}");
    assert!(text.lines().any(|line| line.starts_with("     ")), "{text}");
    assert!(text.contains("4. Last entry\n"), "{text}");
}

#[test]
fn tables_use_stacked_label_value_rows() {
    for source in [
        "| Command | Description |\n| --- | --- |\n| `R` | Read **messages** |\n| `W` | Write a message |\n",
        "| Befehl | Beschreibung |\n| --- | --- |\n| `R` | Nachrichten **lesen** |\n| `W` | Nachricht schreiben |\n",
    ] {
        let result = render(
            source,
            &RenderOptions {
                width: 40,
                ..RenderOptions::default()
            },
        )
        .unwrap();
        assert!(result.plain_text.contains(": R\n"));
        assert!(result.plain_text.contains(": W\n"));
        assert!(!result.plain_text.contains('|'));
        assert!(result.plain_text.lines().all(|line| line.chars().count() <= 40));
    }
}

#[test]
fn links_print_labels_and_urls_and_escape_entity_macros() {
    let source = "[Site](https://example.org/path)\n\n<https://example.org/>\n\n&#64;USER&#64;";
    let result = render(source, &RenderOptions::default()).unwrap();
    assert!(result.plain_text.contains("Site (https://example.org/path)"));
    assert!(result.plain_text.contains("https://example.org/\n"));
    assert_eq!(result.plain_text.matches("https://example.org/").count(), 2);
    assert!(result.plain_text.contains("@USER@"));
    assert!(!decoded(&result.bytes, Encoding::Utf8).contains("@USER@"));
}

#[test]
fn unsupported_markdown_is_rejected() {
    for source in [
        "<script>alert(1)</script>",
        "text <b>bold</b>",
        "<!-- hidden -->",
        "![image](picture.png)",
        "note[^1]\n\n[^1]: footnote",
        "~~struck out~~",
        "- [x] task",
        "    indented code",
        "---",
        "# Title {#id}",
        "Term\n: Definition",
        "---\nmetadata: value\n---\n\nText",
        "Math $x+y$.",
    ] {
        assert!(render(source, &RenderOptions::default()).is_err(), "Accepted {source:?}");
    }
}

#[test]
fn source_and_entity_controls_and_non_cell_characters_are_rejected() {
    for source in [
        "a\x1bb",
        "a\x00b",
        "a\x07b",
        "a\x08b",
        "a\x1ab",
        "a\x7fb",
        "a\u{0085}b",
        "a&#27;b",
        "a&#10;b",
        "e\u{0301}",
        "漢字",
        "🙂",
        "a\u{200b}b",
        "a\u{202e}b",
    ] {
        for encoding in [Encoding::Cp437, Encoding::Utf8] {
            assert!(
                render(
                    source,
                    &RenderOptions {
                        encoding,
                        ..RenderOptions::default()
                    }
                )
                .is_err(),
                "Accepted {source:?}"
            );
        }
    }
}

#[test]
fn cp437_is_lossless_and_utf8_has_a_bom() {
    let source = "ÄÖÜ äöü ß é";
    let legacy = RenderOptions {
        encoding: Encoding::Cp437,
        ..RenderOptions::default()
    };
    let cp = render(source, &legacy).unwrap();
    assert!(!cp.bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    assert!(cp.bytes.contains(&0x8E));
    assert!(visible_generated(&cp.bytes, Encoding::Cp437).contains(source));
    let utf = render(source, &RenderOptions::default()).unwrap();
    assert!(utf.bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    assert!(visible_generated(&utf.bytes, Encoding::Utf8).contains(source));
    assert_eq!(cp.plain_text, utf.plain_text);
    let err = render("€", &legacy).unwrap_err();
    assert!(err.to_string().contains("CP437"));
    assert!(render("€", &RenderOptions::default()).is_ok());
}

#[test]
fn invalid_geometry_and_special_or_blinking_colors_are_rejected() {
    for width in [0, 39, 80, usize::MAX] {
        assert!(
            render(
                "text",
                &RenderOptions {
                    width,
                    ..RenderOptions::default()
                }
            )
            .is_err()
        );
    }
    for attribute in [0x00, 0x80, 0xFF] {
        let mut theme = HelpTheme::default();
        theme.body = attribute;
        assert!(
            render(
                "text",
                &RenderOptions {
                    theme,
                    ..RenderOptions::default()
                }
            )
            .is_err()
        );
    }
    let mut theme = HelpTheme::default();
    theme.margin = usize::MAX;
    assert!(
        render(
            "text",
            &RenderOptions {
                theme,
                ..RenderOptions::default()
            }
        )
        .is_err()
    );
    let mut theme = HelpTheme::default();
    theme.margin = 19;
    assert!(
        render(
            "> > > too deep",
            &RenderOptions {
                width: 40,
                theme,
                ..RenderOptions::default()
            }
        )
        .is_err()
    );
}

#[test]
fn margins_reserve_both_edges() {
    let mut theme = HelpTheme::default();
    theme.margin = 3;
    let options = RenderOptions {
        width: 40,
        theme,
        ..RenderOptions::default()
    };
    let result = render(ENGLISH, &options).unwrap();
    for line in visible_generated(&result.bytes, Encoding::Utf8).lines().filter(|line| !line.is_empty()) {
        assert!(line.starts_with("   "));
        assert!(line.chars().count() <= 37);
    }
}

#[test]
fn clear_screen_is_optional_reset_is_unconditional_and_crlf_is_normalized() {
    for clear_screen in [true, false] {
        for source in ["", "plain\n\ntext", "plain\r\n\r\ntext", "plain\r\rtext"] {
            let result = render(
                source,
                &RenderOptions {
                    clear_screen,
                    ..RenderOptions::default()
                },
            )
            .unwrap();
            let wire = decoded(&result.bytes, Encoding::Utf8);
            assert_eq!(wire.matches("@CLS@").count(), usize::from(clear_screen));
            assert!(wire.ends_with("@X07"));
            if !source.is_empty() {
                assert_eq!(result.plain_text, "plain\n\ntext\n");
            }
        }
    }
}

#[test]
fn hashes_are_stable_lowercase_sha256() {
    assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(sha256(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}
