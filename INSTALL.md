# Installing IcyBoard

IcyBoard is a set of command line programs. There is no installer and nothing is
written outside the directory you unpack it into, because the board works with
relative paths.

## What runs it

| System | Prebuilt | From source |
| :--- | :--- | :--- |
| Linux x86_64 | yes | yes |
| Windows x86_64 | yes | yes |
| macOS, Apple Silicon and Intel | yes | yes |
| Raspberry Pi and other Linux | no | yes |

A board needs a terminal of at least 80x25 for the setup tools.

## Getting the programs

Download the archive for your system from the
[releases](https://github.com/mkrueger/icy_board/releases):

| File | For |
| :--- | :--- |
| `icyboard-<version>-linux-x64.zip` | Linux x86_64 |
| `icyboard-<version>-windows-x64.zip` | Windows x86_64 |
| `icyboard-<version>-macos-arm64.zip` | macOS, Apple Silicon |
| `icyboard-<version>-macos-x64.zip` | macOS, Intel |

The archive holds a `bin/` directory with every program. Unpack it wherever you
like and put `bin/` on your `PATH`, so the tools find each other and you can call
them from a board directory.

Or build it yourself, which works on anything Rust supports and needs a
[Rust toolchain](https://rustup.rs):

```sh
git clone https://github.com/mkrueger/icy_board
cd icy_board
cargo build --release
```

The programs are then in `target/release/`. To build only the PPL compiler,
decompiler and language server, without the board's audio dependencies, use:

```sh
cargo build --release -p pplc -p ppld -p ppl-lsp
```

With CMake 4, a full build may fail in the bundled Opus library with a
`cmake_minimum_required` compatibility error. The release builds use CMake's
policy compatibility setting for that dependency:

```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build --release
```

On a Raspberry Pi the OpenSSL development package has to be there as well:
`sudo apt-get install libssl-dev`.

### macOS

The build needs `cmake` and `pkg-config` for the bundled Opus audio codec.
Install them with [Homebrew](https://brew.sh):

```sh
brew install cmake pkg-config
```

The Opus source shipped with the `audiopus_sys` crate still declares an old
`cmake_minimum_required`, which recent CMake (4.x) rejects. Build with the
compatibility flag set:

```sh
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build --release
```

## Your first board

```sh
icbsetup create mybbs     # writes a complete board into mybbs/
cd mybbs
icboard                   # call waiting screen, telnet on port 1337
```

`icboard` reads the `icboard.toml` of the directory it is started in; `ICB_PATH`
can name that directory instead. `icboard --localon` logs you in as the sysop
right away.

An existing PCBoard installation can be brought over instead. The original is
only read, never changed:

```sh
icbsetup import /path/to/PCBOARD.DAT mybbs
```

The import gets you a starting point, not a finished board. Follow the
[migration guide](docs/migration.md), and read
[known limitations](docs/known_limitations.md) before moving a board you care
about.

## The programs

| Program | What it is for |
| :--- | :--- |
| `icboard` | The board itself. Started in the directory that holds `icboard.toml`. |
| `icbsetup` | Creates a board, imports a PCBoard one, and edits every setting. Start here. |
| `icbsm` | User and group editor, packs the user file and runs the bulk edits. |
| `mkicbtxt` | Edits the system messages, which is how most of the board is reworded. |
| `mkicbmnu` | Edits menus. |
| `icbfile` | Brings a file base into shape - see [icbfile](docs/icbfile.md). |
| `icbmailer` | FTN mail: scan, poll and toss. |
| `pplc`, `ppld` | PPL compiler and decompiler - see [PPL](docs/ppl.md). |
| `ppl-lsp` | The PPL language server, for the editor support below. |

## PPL in your editor

Editor support is two separate pieces, and most editors want both:

- a **tree-sitter grammar** for syntax highlighting, folding, indentation and
  the outline
- the **language server** `ppl-lsp` for diagnostics, completion, hover,
  signature help, go to definition and references

`.pps` is the source extension. Which piece an editor needs, and how it gets it,
differs:

| Editor | Grammar | Language server |
| :--- | :--- | :--- |
| VS Code | in the extension | in the extension, or from `PATH` |
| Zed | built by the editor | downloaded by the extension |
| Helix, Neovim | built once, locally | configured by hand or by the script |
| Anything else with LSP | not available | run `ppl-lsp` over stdio |

### VS Code

Download the `.vsix` for your platform from the
[releases](https://github.com/mkrueger/icy_board/releases) and install it:

```sh
code --install-extension ppl-vscode-<version>-<platform>.vsix
```

The Extensions view does the same through *Install from VSIX...* in its `...`
menu. The platform packages carry the server, so nothing else is needed;
`ppl-vscode-<version>-no-server.vsix` is only for platforms that have no package
of their own and expects `ppl-lsp` on your `PATH`.

[![A PPL project in VS Code](assets/editor_vscode.png)](assets/editor_vscode.png)

The extension supplies PPL highlighting and the language client. The integrated
terminal can build or run the PPE beside its source.

### Zed

The extension is not in Zed's registry yet, so it is installed from its
[repository](https://github.com/mkrueger/zed-ppl):

```sh
git clone https://github.com/mkrueger/zed-ppl
```

Then run `zed: install dev extension` from the command palette, or press
*Install Dev Extension* in the extension list, and select the clone. Zed builds
the extension and the grammar itself, so a [Rust toolchain](https://rustup.rs)
has to be there; the first build takes a few seconds.

Open a `.pps` file afterwards. The grammar highlights it, and the language server
is fetched from the newest IcyBoard release when it is first needed. An
`ppl-lsp` on your `PATH` is taken instead, so a local build wins over the
downloaded one.

[![PPL diagnostics in Zed](assets/editor_zed.png)](assets/editor_zed.png)

Zed combines the tree-sitter grammar with language-server diagnostics, hover
information, completion and navigation.

### Helix and Neovim

From a source checkout, one script does the whole job - it builds the parser,
puts the queries where the editor looks for them, builds the language server and
writes the configuration:

```sh
tools/setup-editor.sh          # every editor found
tools/setup-editor.sh helix
tools/setup-editor.sh neovim
```

[tree-sitter-ppl](crates/tree-sitter-ppl/README.md) spells out what that amounts
to, for anyone who would rather do it by hand.

[![PPL completion in Helix](assets/editor_helix.png)](assets/editor_helix.png)

Helix shows completion details from `ppl-lsp` while its tree-sitter grammar
handles highlighting, indentation, folding and text objects.

### Other editors

Anything that speaks LSP can use the server directly. It talks over stdio and
takes no arguments, so an entry naming the `ppl-lsp` binary for `.pps`
files is all it needs.

## Where to read on

- [Documentation index](docs/README.md) - guides, compatibility references and
  PPL/tooling documentation
- [Getting started](docs/gettingstarted.md) - the board, its directories and the
  first steps
- [Migrating from PCBoard](docs/migration.md) - dry-run import, drive maps, PPE
  review and validation
- [Known limitations](docs/known_limitations.md) - what is missing and what
  behaves differently than PCBoard did
- [PPL](docs/ppl.md) - the language, the compiler and what IcyBoard added
