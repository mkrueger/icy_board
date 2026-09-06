//! Shared CLI localization, independent of the board engine and terminal UI.
//!
//! Keep option names and machine-readable output stable. Only descriptions,
//! headings and parser diagnostics are localized. Programs retain their own
//! version switches and no-argument behavior.
use std::{collections::HashMap, ffi::OsString, sync::OnceLock};

use clap::{Arg, ArgAction, Command, CommandFactory, FromArgMatches};
use clap_i18n_richformatter::{ClapI18nRichFormatter, init_clap_rich_formatter_localizer};
use i18n_embed::{DesktopLanguageRequester, fluent::FluentLanguageLoader};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "i18n"]
struct Localizations;

const DOMAINS: &[&str] = &[
    "common",
    "pplc",
    "ppld",
    "icboard",
    "icbsetup",
    "icbsm",
    "icbfile",
    "icbmailer",
    "mkicbtxt",
    "mkicbmnu",
];

static LOADERS: OnceLock<HashMap<&'static str, FluentLanguageLoader>> = OnceLock::new();
static FORMATTER: OnceLock<()> = OnceLock::new();

/// Translate application-owned help using the same desktop locale as the formatter.
pub fn text(domain: &str, key: &str) -> String {
    let loaders = LOADERS.get_or_init(|| {
        let languages = DesktopLanguageRequester::requested_languages();
        DOMAINS
            .iter()
            .map(|&domain| {
                let loader = FluentLanguageLoader::new(domain, "en".parse().unwrap());
                i18n_embed::select(&loader, &Localizations, &languages).expect("embedded CLI translations must load");
                loader.set_use_isolating(false);
                (domain, loader)
            })
            .collect()
    });
    loaders.get(domain).expect("unknown CLI translation domain").get(key)
}

/// Build a command with localized help at every subcommand level.
pub fn command<T: CommandFactory>() -> Command {
    FORMATTER.get_or_init(init_clap_rich_formatter_localizer);
    localize(T::command())
}

fn localize(mut command: Command) -> Command {
    command = command
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .help_template(format!(
            "{{before-help}}{{about-with-newline}}\n{} {{usage}}\n\n{{all-args}}{{after-help}}",
            text("common", "usage")
        ))
        .subcommand_help_heading(text("common", "commands"))
        .subcommand_value_name(text("common", "command"));

    let arguments: Vec<_> = command.get_arguments().map(|arg| (arg.get_id().clone(), arg.is_positional())).collect();
    for (id, positional) in arguments {
        command = command.mut_arg(id, |mut arg| {
            // argh permits repeated switches but rejects repeated scalar options.
            if matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse) {
                let id = arg.get_id().clone();
                arg = arg.overrides_with(id);
            }
            // argh consumes the next token as an option value even if it starts
            // with '-'; positional filenames still require '--' in that case.
            if !positional && arg.get_num_args().map_or(arg.get_action().takes_values(), |range| range.max_values() > 0) {
                arg = arg.allow_hyphen_values(true);
            }
            arg.help_heading(text("common", if positional { "arguments" } else { "options" }))
        });
    }

    command
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::Help)
                .help(text("common", "help"))
                .help_heading(text("common", "options")),
        )
        .mut_subcommands(localize)
}

// argh accepts both `tool help command` and `tool command help`, as well as
// `tool --help command`. Normalize these before clap sees the help action.
// Never reinterpret an option value or a filename following `--` as help.
fn normalize_help(command: &Command, arguments: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut arguments = arguments.into_iter();
    let mut result = Vec::new();
    if let Some(program) = arguments.next() {
        result.push(program);
    }
    let mut current = command;
    let mut help = false;
    let mut takes_value = false;
    for argument in arguments.by_ref() {
        if takes_value {
            takes_value = false;
            result.push(argument);
            continue;
        }
        if argument == "--" {
            if help {
                result.push("--help".into());
                help = false;
            }
            result.push(argument);
            result.extend(arguments);
            break;
        }
        if argument == "help" || argument == "--help" || argument == "-h" {
            help = true;
            continue;
        }
        if let Some(value) = argument.to_str() {
            if let Some(subcommand) = current.find_subcommand(value) {
                current = subcommand;
            } else if value.starts_with('-') {
                takes_value = current.get_arguments().any(|arg| {
                    let matches =
                        arg.get_long().is_some_and(|long| value == format!("--{long}")) || arg.get_short().is_some_and(|short| value == format!("-{short}"));
                    matches && arg.get_num_args().map_or(arg.get_action().takes_values(), |range| range.max_values() > 0)
                });
            }
        }
        result.push(argument);
    }
    if help {
        result.push("--help".into());
    }
    result
}

/// Parse supplied arguments (including argv[0]), without terminating the process.
pub fn try_parse_from<T, I, S>(arguments: I) -> Result<T, clap::error::Error<ClapI18nRichFormatter>>
where
    T: CommandFactory + FromArgMatches,
    I: IntoIterator<Item = S>,
    S: Into<OsString> + Clone,
{
    let mut command = command::<T>();
    let arguments = normalize_help(&command, arguments.into_iter().map(Into::into));
    command
        .try_get_matches_from_mut(arguments)
        .and_then(|matches| T::from_arg_matches(&matches).map_err(|error| error.format(&mut command)))
        .map_err(|error| error.apply::<ClapI18nRichFormatter>())
}

/// Parse the process command line with localized rich errors.
///
/// Preserve argh's exit status: help succeeds, invalid arguments exit with 1
/// (not clap's default 2). clap still selects the correct stdout/stderr stream.
pub fn parse<T: CommandFactory + FromArgMatches>() -> T {
    try_parse_from(std::env::args_os()).unwrap_or_else(|error| {
        let status = i32::from(error.use_stderr());
        let _ = error.print();
        std::process::exit(status);
    })
}

#[cfg(test)]
mod localization_tests {
    use std::collections::BTreeSet;

    use fluent_bundle::{FluentBundle, FluentResource};
    use fluent_syntax::ast::Entry;

    use super::{DOMAINS, Localizations};

    #[test]
    fn all_cli_catalogs_have_matching_keys_and_resolve_without_fallback() {
        let mut checked_files = BTreeSet::new();
        for domain in DOMAINS {
            let mut english_keys = BTreeSet::new();
            for locale in ["en", "de"] {
                let path = format!("{locale}/{domain}.ftl");
                let data = Localizations::get(&path).unwrap_or_else(|| panic!("missing {path}"));
                checked_files.insert(path.clone());
                let source = String::from_utf8(data.data.to_vec()).unwrap();
                let resource = FluentResource::try_new(source).unwrap_or_else(|(_, errors)| panic!("{path}: {errors:?}"));
                let mut bundle = FluentBundle::new(vec![locale.parse().unwrap()]);
                bundle.set_use_isolating(false);
                bundle
                    .add_resource(&resource)
                    .unwrap_or_else(|errors| panic!("{path}: duplicate entries: {errors:?}"));
                let mut keys = BTreeSet::new();
                for entry in resource.entries() {
                    let Entry::Message(message) = entry else { continue };
                    keys.insert(message.id.name.to_string());
                    let value = message.value.as_ref().expect("CLI help must have a value");
                    let mut errors = Vec::new();
                    let text = bundle.format_pattern(value, None, &mut errors);
                    assert!(errors.is_empty(), "{path}/{}: {errors:?}", message.id.name);
                    assert!(!text.trim().is_empty(), "{path}/{} is empty", message.id.name);
                    assert!(!text.contains("TODO"), "{path}/{} is unfinished", message.id.name);
                    assert!(!text.contains(['\u{2068}', '\u{2069}']), "{path}/{} has bidi isolates", message.id.name);
                }
                assert!(!keys.is_empty(), "{path} is empty");
                if locale == "en" {
                    english_keys = keys;
                } else {
                    assert_eq!(english_keys, keys, "{domain}: German and English keys differ");
                }
            }
        }
        let embedded_files: BTreeSet<_> = Localizations::iter().map(|path| path.into_owned()).collect();
        assert_eq!(checked_files, embedded_files, "every embedded CLI catalog must be registered and tested");
    }
}
