use std::fs;

use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, GithubReleaseOptions, LanguageServerId, LanguageServerInstallationStatus, Os, Result, settings::LspSettings,
};

const SERVER_ID: &str = "icyboard-ppl";
const REPOSITORY: &str = "mkrueger/icy_board";

struct PplExtension {
    server: Option<String>,
}

/// Where the server for this platform is packed and what it is called there.
struct Package {
    asset: String,
    binary: &'static str,
    file_type: DownloadedFileType,
}

fn package() -> Result<Package> {
    let (os, arch) = zed::current_platform();
    let target = match (os, arch) {
        (Os::Linux, Architecture::X8664) => "x86_64-unknown-linux-gnu",
        (Os::Mac, Architecture::Aarch64) => "aarch64-apple-darwin",
        (Os::Mac, Architecture::X8664) => "x86_64-apple-darwin",
        (Os::Windows, Architecture::X8664) => "x86_64-pc-windows-msvc",
        _ => {
            return Err("There is no prebuilt PPL language server for this platform. Build `icyboard-ppl` from source and put it on your PATH.".to_string());
        }
    };

    Ok(match os {
        Os::Windows => Package {
            asset: format!("icyboard-ppl-{target}.zip"),
            binary: "icyboard-ppl.exe",
            file_type: DownloadedFileType::Zip,
        },
        _ => Package {
            asset: format!("icyboard-ppl-{target}.tar.gz"),
            binary: "icyboard-ppl",
            file_type: DownloadedFileType::GzipTar,
        },
    })
}

impl PplExtension {
    /// Fetches the server from the newest release that carries one, and keeps it
    /// until a release brings a newer one.
    fn download(&mut self, id: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.server
            && fs::metadata(path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(path.clone());
        }

        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::CheckingForUpdate);
        let release = zed::latest_github_release(
            REPOSITORY,
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let package = package()?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == package.asset)
            .ok_or_else(|| format!("Release {} of IcyBoard has no {}.", release.version, package.asset))?;

        let directory = format!("icyboard-ppl-{}", release.version);
        let path = format!("{directory}/{}", package.binary);
        if fs::metadata(&path).is_err() {
            zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Downloading);
            zed::download_file(&asset.download_url, &directory, package.file_type)?;
            zed::make_file_executable(&path)?;
            remove_older_downloads(&directory);
        }

        self.server = Some(path.clone());
        Ok(path)
    }
}

fn remove_older_downloads(keep: &str) {
    let Ok(entries) = fs::read_dir(".") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("icyboard-ppl-") && name != keep {
            fs::remove_dir_all(entry.path()).ok();
        }
    }
}

impl zed::Extension for PplExtension {
    fn new() -> Self {
        Self { server: None }
    }

    fn language_server_command(&mut self, id: &LanguageServerId, worktree: &zed::Worktree) -> Result<zed::Command> {
        let configured = LspSettings::for_worktree(SERVER_ID, worktree).ok().and_then(|settings| settings.binary);

        if let Some(binary) = configured
            && let Some(path) = binary.path
        {
            return Ok(zed::Command {
                command: path,
                args: binary.arguments.unwrap_or_default(),
                env: worktree.shell_env(),
            });
        }

        // An installed IcyBoard is used as it is; otherwise the server is fetched.
        let command = match worktree.which(SERVER_ID) {
            Some(path) => path,
            None => self.download(id).inspect_err(|error| {
                zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::Failed(error.clone()));
            })?,
        };

        Ok(zed::Command {
            command,
            args: Vec::new(),
            env: worktree.shell_env(),
        })
    }

    fn language_server_workspace_configuration(&mut self, _id: &LanguageServerId, worktree: &zed::Worktree) -> Result<Option<zed::serde_json::Value>> {
        Ok(LspSettings::for_worktree(SERVER_ID, worktree).ok().and_then(|settings| settings.settings))
    }
}

zed::register_extension!(PplExtension);
