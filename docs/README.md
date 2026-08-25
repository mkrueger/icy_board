# Icy Board documentation

The [project README](../README.md) explains what Icy Board is and what it can
do. This directory holds the shorter, task-oriented documentation. The full
handbook is built from [`docs/source`](source/) and ships as HTML and PDF.

## Run a board

| Start here | Use it for |
| :--- | :--- |
| [Getting started](gettingstarted.md) | Create a board, configure it, make a local call and find the important files. |
| [Installation](../INSTALL.md) | Prebuilt packages, source builds and editor setup. |
| [Migrating from PCBoard](migration.md) | Dry-run an import, map DOS drives, inspect PPEs and validate the result. |
| [File areas](icbfile.md) | Import, normalize and maintain file bases. |

## Understand compatibility

| Reference | Answers |
| :--- | :--- |
| [Feature status](feature_parity.md) | Which PCBoard commands and subsystems work today? |
| [Known limitations](known_limitations.md) | What is missing or incomplete in the beta? |
| [Differences and improvements](differences.md) | What changed deliberately, why, and what may break? |
| [Command audit](../compat/COMMAND_AUDIT.md) | Where does a command still answer differently from PCBoard? |
| [Options audit](../compat/OPTIONS_AUDIT.md) | Which setup options are active, inactive or intentionally different? |

“PCBoard compatible” here means that callers, display files, commands, macros
and PPEs should behave as they did on PCBoard 15.4 unless a difference is
documented. It does not mean preserving DOS, modem control or the old on-disk
databases. The compatibility harness uses the PCBoard source and a real board
under DOSBox as an oracle; see [`compat/README.md`](../compat/README.md).

## Customize and extend

| Guide | Use it for |
| :--- | :--- |
| [PPL and PPEs](ppl.md) | Language overview, runtime compatibility and the toolchain. |
| [PPL compiler](pplc.md) | Projects, language versions, output and diagnostics. |
| [New in PPL 3.50 and 4.x](new_ppl.md) | Version matrix for loops, initializers, constants, enums, routine parameters, records and board objects. |
| [New `@` macros](new_macros.md) | Macros beyond PCBoard's set. |
| [PPE format](ppe_format.md) | Binary format reference for tooling authors. |
| [PPL 4.00 API review](ppl400_api_review.md) | Why the 4.00 object API looks the way it does, and what is still open. |

PPL editor support consists of the tree-sitter grammar for syntax and the
`ppl-lsp` language server for diagnostics, completion, hover, navigation,
references and formatting. [Installation](../INSTALL.md#ppl-in-your-editor)
contains the setup for VS Code, Zed, Helix and Neovim.

## Project status

- [Roadmap](roadmap.md) lists work that remains after the first beta.
- The [handbook](source/index.rst) covers operation, events, FTN mail and the
  complete PPL reference.
- Bugs found while importing a real PCBoard installation are especially useful;
  the importer cannot learn unusual drive layouts and PPE conventions from
  synthetic boards alone.
