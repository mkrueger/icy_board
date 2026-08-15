use zed_extension_api::{self as zed, LanguageServerId, Result, settings::LspSettings};

const SERVER_ID: &str = "icyboard-ppl";
const SERVER_BINARY: &str = "icyboard-ppl";

struct PplExtension;

impl zed::Extension for PplExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(&mut self, _id: &LanguageServerId, worktree: &zed::Worktree) -> Result<zed::Command> {
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

        // The server ships with IcyBoard, so it is taken from the user's PATH.
        let command = worktree
            .which(SERVER_BINARY)
            .ok_or_else(|| format!("`{SERVER_BINARY}` was not found in PATH. Install IcyBoard, or point `lsp.{SERVER_ID}.binary.path` at the executable."))?;

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
