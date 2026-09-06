//! Validate both workspace Fluent catalogs without relying on the host locale
//! or allowing the language loader to hide missing translations with fallback.
use std::collections::{BTreeMap, BTreeSet};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use fluent_syntax::ast::{CallArguments, Entry, Expression, InlineExpression, Pattern, PatternElement};

const TUI_EN: &str = include_str!("../i18n/en/icy_board_tui.ftl");
const TUI_DE: &str = include_str!("../i18n/de/icy_board_tui.ftl");
const LSP_EN: &str = include_str!("../../ppl-lsp/i18n/en/ppl_lsp.ftl");
const LSP_DE: &str = include_str!("../../ppl-lsp/i18n/de/ppl_lsp.ftl");

#[derive(Debug, Default, PartialEq, Eq)]
struct Placeholders {
    variables: BTreeSet<String>,
    references: BTreeSet<String>,
}

impl Placeholders {
    fn pattern(&mut self, pattern: &Pattern<&str>) {
        for element in &pattern.elements {
            if let PatternElement::Placeable { expression } = element {
                self.expression(expression);
            }
        }
    }

    fn expression(&mut self, expression: &Expression<&str>) {
        match expression {
            Expression::Inline(inline) => self.inline(inline),
            Expression::Select { selector, variants } => {
                self.inline(selector);
                for variant in variants {
                    self.pattern(&variant.value);
                }
            }
        }
    }

    fn arguments(&mut self, arguments: &CallArguments<&str>) {
        for argument in &arguments.positional {
            self.inline(argument);
        }
        for argument in &arguments.named {
            self.inline(&argument.value);
        }
    }

    fn inline(&mut self, inline: &InlineExpression<&str>) {
        match inline {
            InlineExpression::VariableReference { id } => {
                self.variables.insert(id.name.to_string());
            }
            InlineExpression::MessageReference { id, attribute } => {
                self.references
                    .insert(format!("{}{}", id.name, attribute.as_ref().map_or(String::new(), |a| format!(".{}", a.name))));
            }
            InlineExpression::TermReference { id, attribute, arguments } => {
                self.references
                    .insert(format!("-{}{}", id.name, attribute.as_ref().map_or(String::new(), |a| format!(".{}", a.name))));
                if let Some(arguments) = arguments {
                    self.arguments(arguments);
                }
            }
            InlineExpression::FunctionReference { id, arguments } => {
                self.references.insert(format!("{}()", id.name));
                self.arguments(arguments);
            }
            InlineExpression::Placeable { expression } => self.expression(expression),
            InlineExpression::StringLiteral { .. } | InlineExpression::NumberLiteral { .. } => {}
        }
    }
}

fn parse(source: &str, name: &str) -> FluentResource {
    FluentResource::try_new(source.to_string()).unwrap_or_else(|(_, errors)| panic!("{name}: invalid Fluent syntax: {errors:?}"))
}

fn patterns<'a>(resource: &'a FluentResource) -> BTreeMap<String, &'a Pattern<&'a str>> {
    let mut patterns = BTreeMap::new();
    for entry in resource.entries() {
        let (id, value, attributes) = match entry {
            Entry::Message(message) => (message.id.name.to_string(), message.value.as_ref(), &message.attributes),
            Entry::Term(term) => (format!("-{}", term.id.name), Some(&term.value), &term.attributes),
            _ => continue,
        };
        if let Some(value) = value {
            assert!(patterns.insert(id.clone(), value).is_none(), "Duplicate key: {id}");
        }
        for attribute in attributes {
            let key = format!("{id}.{}", attribute.id.name);
            assert!(patterns.insert(key.clone(), &attribute.value).is_none(), "Duplicate attribute: {key}");
        }
    }
    patterns
}

fn check_catalog(english: &str, german: &str) {
    let english = parse(english, "en");
    let german = parse(german, "de");
    let en_patterns = patterns(&english);
    let de_patterns = patterns(&german);
    let en_keys: BTreeSet<_> = en_patterns.keys().collect();
    let de_keys: BTreeSet<_> = de_patterns.keys().collect();
    assert_eq!(en_keys, de_keys, "German and English must have exactly the same keys and attributes");

    let mut arguments = FluentArgs::new();
    for (key, en_pattern) in &en_patterns {
        let mut en = Placeholders::default();
        en.pattern(en_pattern);
        let mut de = Placeholders::default();
        de.pattern(de_patterns[key]);
        assert_eq!(en, de, "Placeholders differ for {key}");
        for variable in en.variables {
            arguments.set(variable, "1");
        }
    }

    for (locale, resource) in [("en", &english), ("de", &german)] {
        let mut bundle = FluentBundle::new(vec![locale.parse().unwrap()]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .unwrap_or_else(|errors| panic!("{locale}: duplicate Fluent entries: {errors:?}"));
        for (key, pattern) in patterns(resource) {
            let mut errors = Vec::new();
            let text = bundle.format_pattern(pattern, Some(&arguments), &mut errors);
            assert!(errors.is_empty(), "{locale}/{key}: {errors:?}");
            assert!(!text.trim().is_empty(), "{locale}/{key}: empty translation");
            assert!(!text.trim().eq_ignore_ascii_case("TODO"), "{locale}/{key}: untranslated placeholder");
            assert!(!text.contains(['\u{2068}', '\u{2069}']), "{locale}/{key}: unexpected bidi isolates");
        }
    }
}

#[test]
fn german_tui_catalog_is_complete_and_formats_without_errors() {
    check_catalog(TUI_EN, TUI_DE);
}

#[test]
fn german_lsp_catalog_is_complete_and_formats_without_errors() {
    check_catalog(LSP_EN, LSP_DE);
}

#[test]
fn scanner_help_displays_literal_file_placeholder() {
    for (locale, source) in [("en", TUI_EN), ("de", TUI_DE)] {
        let mut bundle = FluentBundle::new(vec![locale.parse().unwrap()]);
        bundle.add_resource(parse(source, locale)).unwrap();
        for suffix in ["status", "help", "invalid"] {
            let key = format!("upload_processing_scanner_arguments-{suffix}");
            let message = bundle.get_message(&key).unwrap();
            let mut errors = Vec::new();
            let text = bundle.format_pattern(message.value().unwrap(), None, &mut errors);
            assert!(errors.is_empty(), "{locale}/{key}: {errors:?}");
            assert!(text.contains("{file}"), "{locale}/{key}: {text}");
        }
    }
}
