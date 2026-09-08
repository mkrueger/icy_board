pub mod install;

use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::{Args, Subcommand};
use icy_board_engine::{
    Res,
    icy_board::{IcyBoardSerializer, icb_config::IcbConfig, lock::BoardLock, write_atomic},
};
use icy_board_help::{
    Encoding, HelpTheme, RenderOptions,
    catalog::{self, Source},
    render, sha256,
};
use serde::Serialize;

use install::{Artifact, InstallOptions};

#[derive(Args, Debug, PartialEq)]
#[command(subcommand_precedence_over_arg = true)]
pub struct GenHelp {
    #[command(subcommand)]
    command: Option<GenHelpCommand>,

    #[command(flatten)]
    generate: Generate,
}

#[derive(Subcommand, Debug, PartialEq)]
enum GenHelpCommand {
    #[command(about = icy_board_cli::text("icbsetup", "genhelp-check-about"))]
    Check(Selection),
    #[command(about = icy_board_cli::text("icbsetup", "genhelp-export-about"))]
    Export {
        #[arg(help = icy_board_cli::text("icbsetup", "genhelp-destination"))]
        destination: PathBuf,
    },
}

#[derive(Args, Debug, Default, PartialEq)]
struct Selection {
    #[arg(help = icy_board_cli::text("icbsetup", "genhelp-board"))]
    board: Option<PathBuf>,
    #[arg(long, help = icy_board_cli::text("icbsetup", "genhelp-theme"))]
    theme: Option<String>,
    #[arg(long, value_parser = clap::value_parser!(u16).range(40..=79), help = icy_board_cli::text("icbsetup", "genhelp-width"))]
    width: Option<u16>,
    // require_equals keeps `--cp437` zero-arg while still allowing `--cp437=false` for the UTF-8 default.
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true", overrides_with = "cp437", help = icy_board_cli::text("icbsetup", "genhelp-cp437"))]
    cp437: Option<bool>,
    #[arg(long, help = icy_board_cli::text("icbsetup", "genhelp-sources"))]
    sources: Option<PathBuf>,
    #[arg(long, help = icy_board_cli::text("icbsetup", "genhelp-language"))]
    language: Option<String>,
}

#[derive(Args, Debug, Default, PartialEq)]
struct Generate {
    #[command(flatten)]
    selection: Selection,
    #[arg(long, help = icy_board_cli::text("icbsetup", "genhelp-output"))]
    output: Option<PathBuf>,
    #[arg(long, help = icy_board_cli::text("icbsetup", "genhelp-dry-run"))]
    dry_run: bool,
    #[arg(long, help = icy_board_cli::text("icbsetup", "genhelp-adopt"))]
    adopt: bool,
    #[arg(long, help = icy_board_cli::text("icbsetup", "genhelp-replace-modified"))]
    replace_modified: bool,
}

fn text(key: &str) -> String {
    icy_board_cli::text("icbsetup", key)
}

fn error(key: &str, detail: impl std::fmt::Display) -> Box<dyn std::error::Error + Send + Sync> {
    format!("{}: {detail}", text(key)).into()
}

#[derive(Serialize)]
struct Fingerprint<'a> {
    generator: &'a str,
    schema: u32,
    width: usize,
    encoding: Encoding,
    clear_screen: bool,
    theme: &'a HelpTheme,
}

/// The board's own language extension is unknown here; matching it is the sysop's responsibility.
fn language_extension(language: Option<&str>) -> Res<&str> {
    let Some(extension) = language else {
        return Ok("");
    };
    let valid = (1..=32).contains(&extension.len())
        && extension.as_bytes()[0].is_ascii_lowercase()
        && extension.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    if !valid {
        return Err(error("genhelp-invalid-language", extension));
    }
    Ok(extension)
}

fn compile(selection: &Selection) -> Res<Vec<Artifact>> {
    let width = selection.width.unwrap_or(79);
    let encoding = match selection.cp437 {
        Some(true) => Encoding::Cp437,
        _ => Encoding::Utf8,
    };
    let extension = language_extension(selection.language.as_deref())?;
    let theme = match selection.theme.as_deref() {
        None => HelpTheme::preset("classic")?,
        Some(name @ ("classic" | "minimal")) => HelpTheme::preset(name)?,
        // Anything else names a theme file, so a custom theme needs no board setting.
        Some(path) => toml::from_str::<HelpTheme>(&fs::read_to_string(path).map_err(|e| error("genhelp-invalid-theme", format!("{path}: {e}")))?)
            .map_err(|e| error("genhelp-invalid-theme", format!("{path}: {e}")))?,
    };
    theme.validate()?;
    let options = RenderOptions {
        width: usize::from(width),
        encoding,
        theme,
        clear_screen: true,
    };
    let settings_hash = sha256(
        toml::to_string(&Fingerprint {
            generator: concat!("icbsetup-help/", env!("CARGO_PKG_VERSION")),
            schema: 1,
            width: options.width,
            encoding: options.encoding,
            clear_screen: options.clear_screen,
            theme: &options.theme,
        })?
        .as_bytes(),
    );

    let mut artifacts = Vec::new();
    for Source { topic, markdown, source_hash } in catalog::sources(selection.sources.as_deref())? {
        let rendered = render(&markdown, &options).map_err(|e| error("genhelp-render-failed", format!("{topic}: {e}")))?;
        artifacts.push(Artifact {
            name: catalog::output_name(&topic, extension)?,
            bytes: rendered.bytes,
            source_hash,
            settings_hash: settings_hash.clone(),
        });
    }
    Ok(artifacts)
}

pub fn run(genhelp: &GenHelp) -> Res<()> {
    match &genhelp.command {
        Some(GenHelpCommand::Export { destination }) => {
            catalog::export(destination)?;
            println!("{}: {}", text("genhelp-exported"), destination.display());
            Ok(())
        }
        Some(GenHelpCommand::Check(selection)) => run_board(
            selection,
            &InstallOptions {
                dry_run: true,
                ..Default::default()
            },
            false,
        ),
        None => {
            let generate = &genhelp.generate;
            let options = InstallOptions {
                dry_run: generate.dry_run,
                adopt: generate.adopt,
                replace_modified: generate.replace_modified,
            };
            match &generate.output {
                Some(output) => run_output(&generate.selection, output, &options),
                None => run_board(&generate.selection, &options, true),
            }
        }
    }
}

/// Writes into a plain directory: no board, lock, ledger or backups, so nothing can be repaired later.
fn run_output(selection: &Selection, output: &Path, options: &InstallOptions) -> Res<()> {
    if let Some(board) = &selection.board {
        return Err(error("genhelp-board-and-output", board.display()));
    }
    if options.replace_modified {
        return Err(error("genhelp-replace-modified-without-board", "--replace-modified"));
    }
    let artifacts = compile(selection)?;
    if !options.adopt {
        let conflicts: Vec<_> = artifacts
            .iter()
            .filter(|artifact| output.join(&artifact.name).symlink_metadata().is_ok())
            .map(|artifact| artifact.name.as_str())
            .collect();
        if !conflicts.is_empty() {
            return Err(error("genhelp-output-conflicts", conflicts.join(", ")));
        }
    }
    if options.dry_run {
        println!("{}: {}", text("genhelp-would-write"), artifacts.len());
        return Ok(());
    }
    fs::create_dir_all(output)?;
    for artifact in &artifacts {
        write_atomic(output.join(&artifact.name), &artifact.bytes)?;
    }
    println!("{}: {} -> {}", text("genhelp-written"), artifacts.len(), output.display());
    Ok(())
}

fn run_board(selection: &Selection, options: &InstallOptions, require_board: bool) -> Res<()> {
    let config_path = match icy_board_engine::resolve_icyboard_file(&selection.board) {
        Ok(path) => Some(path.canonicalize()?),
        Err(icy_board_engine::IcyBoardFileLookupError::FileNotFound(path)) if require_board || selection.board.is_some() => {
            return Err(error("genhelp-board-not-found", path.display()));
        }
        Err(_) => None,
    };
    let root = config_path
        .as_ref()
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .unwrap_or(std::env::current_dir()?);
    // Reject invalid sources before even creating the board lock file.
    let artifacts = compile(selection)?;
    let _lock = if !options.dry_run { Some(BoardLock::acquire(&root)?) } else { None };
    // Never load IcyBoard: help repair must not depend on users, messages, or other runtime data.
    let config = config_path.as_ref().map(IcbConfig::load).transpose()?.unwrap_or_default();
    if config_path.is_some() {
        if config.paths.help_path.as_os_str().is_empty() {
            return Err(error("genhelp-empty-output", "paths.help_path"));
        }
        for report in install::install(&root, &config.paths.help_path, &artifacts, options)? {
            println!("{report}");
        }
    } else {
        println!("{}: {}", text("genhelp-checked"), artifacts.len());
    }
    Ok(())
}

/// Generate all embedded/overridden English topics for creation.
/// The caller must hold BoardLock; this does not save configuration.
pub fn install_defaults(root: &Path, config: &IcbConfig) -> Res<()> {
    if config.paths.help_path.as_os_str().is_empty() {
        return Err(error("genhelp-empty-output", "paths.help_path"));
    }
    let artifacts = compile(&Selection::default())?;
    for report in install::install(root, &config.paths.help_path, &artifacts, &InstallOptions::default())? {
        println!("{report}");
    }
    Ok(())
}
