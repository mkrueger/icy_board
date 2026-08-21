<!-- download-guide -->
### Which file do I want?

Take **one** row. Every download already contains what that row needs.

| I want to | File |
| :--- | :--- |
| **Run a bulletin board** — the board and all its tools | `icyboard-#TAG-<platform>.zip` |
| **Write PPL in VS Code** | `ppl-vscode-#TAG-<platform>.vsix` |
| **Write PPL in Helix, Neovim, Zed or another LSP editor** | `ppl-lsp-#TAG-<platform>.tar.gz` (`.zip` on Windows) |
| **Read the manual** | `icyboard-manual-#TAG.pdf` |

`<platform>` is one of:

| Platform | Your machine |
| :--- | :--- |
| `linux-x64` | Linux on a 64-bit PC |
| `windows-x64` | Windows on a 64-bit PC |
| `macos-arm64` | Mac with Apple Silicon (M1 and later) |
| `macos-x64` | Mac with an Intel processor |

**You do not need more than one.** `icyboard-*.zip` already holds the `ppl-lsp`
language server next to the board in `bin/`, and each `ppl-vscode-*` package
holds the server for its platform. `ppl-lsp-*` on its own is for editors other
than VS Code when you are not installing the board.

`ppl-vscode-#TAG-no-server.vsix` is the exception: it carries **no** server and
expects `ppl-lsp` on your `PATH` already. Take it only if your platform is not
in the list above.
<!-- /download-guide -->
