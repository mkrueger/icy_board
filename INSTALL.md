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
| `icy_board_linux_<version>.zip` | Linux x86_64 |
| `icy_board_windows_<version>.zip` | Windows x86_64 |
| `icy_board_osx_aarch64-apple-darwin_<version>.zip` | macOS, Apple Silicon |
| `icy_board_osx_x86_64-apple-darwin_<version>.zip` | macOS, Intel |

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

The programs are then in `target/release/`. On a Raspberry Pi the OpenSSL
development package has to be there as well: `sudo apt-get install libssl-dev`.

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
| `icyboard-ppl` | The PPL language server, for the editor support below. |

## PPL in your editor

Editor support is two separate pieces, and most editors want both:

- a **tree-sitter grammar** for syntax highlighting, folding, indentation and
  the outline
- the **language server** `icyboard-ppl` for diagnostics, completion, hover,
  signature help, go to definition and references

`.pps` is the source extension. Which piece an editor needs, and how it gets it,
differs:

| Editor | Grammar | Language server |
| :--- | :--- | :--- |
| VS Code | in the extension | in the extension, or from `PATH` |
| Zed | built by the editor | downloaded by the extension |
| Helix, Neovim | built once, locally | configured by hand or by the script |
| Anything else with LSP | not available | run `icyboard-ppl` over stdio |

### VS Code

Download the `.vsix` for your platform from the
[releases](https://github.com/mkrueger/icy_board/releases) and install it:

```sh
code --install-extension icyboard-ppl-<version>-<platform>.vsix
```

The Extensions view does the same through *Install from VSIX...* in its `...`
menu. The platform packages carry the server; the package without a platform in
its name expects `icyboard-ppl` on your `PATH`.

### Zed

Open the extension list and install **PPL**. It brings the grammar and fetches
the language server from the IcyBoard releases on the first `.pps` file, so
there is nothing else to install.

While the extension is still on its way into Zed's registry, install it from its
[repository](https://github.com/mkrueger/zed-ppl) instead: clone it, then run
`zed: install dev extension` and select the clone.

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

### Other editors

Anything that speaks LSP can use the server directly. It talks over stdio and
takes no arguments, so an entry naming the `icyboard-ppl` binary for `.pps`
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
