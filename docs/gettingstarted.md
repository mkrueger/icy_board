# Getting started

This guide takes a new board from an empty directory to a tested local call.
For packages and source builds, start with [INSTALL.md](../INSTALL.md). To move
an existing PCBoard installation, use the [migration guide](migration.md)
instead.

## Create a board

```sh
icbsetup create mybbs
cd mybbs
```

`icbsetup` creates a complete board, prints a random initial sysop password and
does not write outside `mybbs/`. Keep the password until the first login.

The generated board uses paths relative to `icboard.toml`. You can move or back
up the whole directory without rewriting drive letters.

New boards generate command help from the embedded Markdown catalog and record
ownership for later regeneration. This installs 68 English topics with no
language suffix. See [Command help](#command-help) for customization,
translated output and coverage limits.

## Configure the essentials

From the board directory, run:

```sh
icbsetup
```

Set the board and sysop names, choose a permanent sysop password, inspect the
network listeners and confirm the node count. Options that the runtime does not
use yet are greyed out and explain why.

On an editable filename or directory field, **F4** opens the file browser:

- **Enter** selects a file or opens a directory; **Ctrl-S** selects the current directory.
- Use the arrow keys and Page Up/Down to browse, **Backspace** for the parent directory,
  and **.** to show or hide hidden files.
- **Esc** cancels without changing the field. The next Esc returns through the menus.

Browsing starts at the current path, or its nearest existing directory. Relative paths
are resolved against the board directory. Selections within it remain relative when
the original field was relative or empty; selections outside it use absolute paths.
Choosing a path updates the field, not the filesystem, and uses the normal save dialog.
You can still type new filenames manually: **F2** edits an existing file and **F3**
creates a missing file where those shortcuts are offered.

Escape returns through the menus. When something changed, the exit dialog has
the answers PCBSetup had:

- **Yes** saves, validates configured paths and offers to create missing
  directories.
- **Quick** saves without the path check.
- **No** discards the changes.

The same validation is available without opening the editor:

```sh
icbsetup check icboard.toml
```

## Make the first call

```sh
icboard
```

The call-waiting screen starts the network listeners. Pick **Sysop** and press
Enter for a local session, or skip the screen entirely with:

```sh
icboard --localon
```

From another terminal, the generated board accepts telnet on port 1337:

```sh
telnet localhost 1337
```

The TUIs require a terminal of at least 80 columns by 25 rows.

## Walk the board once

Use the first local call as a smoke test:

1. `J` joins a conference.
2. If the conference has several message areas, use the area-change command to
  select one; areas organize discussions without requiring another conference.
3. `E` enters a message and `R` reads it back.
4. `@W` sends personal mail; `@` opens the caller's inbox and `Y` includes it in
  the personal-mail scan.
5. `F` opens the file directories.
6. `V` shows the caller settings, then `G` logs off.

Read `icboard.log` afterwards. It is the first place to look when a display
file, PPE, protocol or data file does not load.

## Make it your board

PCBoard boards were defined by their data and artwork rather than by one fixed
theme. Icy Board keeps that model:

| Tool or directory | What to change |
| :--- | :--- |
| `icbsetup` | Board, node, listener, conference, security, event and transfer settings |
| `mkicbtxt` | Prompts and system messages |
| `mkicbmnu` | Menus and their commands |
| `icbsm` | Users, groups, bulk maintenance and user-file packing |
| `art/` | PCB, ANSI, Avatar, RIP and plain display files |
| `art/help/` | Command help |
| `conferences/` | Per-conference menus, message areas, file areas and scripts |

Display files may be CP437 or UTF-8. UTF-8 files need the UTF-8 BOM so the
runtime can distinguish them from legacy CP437 without guessing. Files without
that BOM are read as CP437.

Use extensions such as `.pcb`, `.ans`, `.avt`, `.rip` and `.asc`. The runtime
also understands PCBoard's graphics, language and security variants when a
configuration names the extensionless base file.

## Command help

`icbsetup genhelp` manages only BBS command help, not general artwork, admin TUI
help or ICBTEXT prompts. New-board creation uses the same generator as these
maintenance commands; PCBoard import preserves existing help instead.

- `icbsetup genhelp` generates and installs into the board's `paths.help_path`.
  Add `--dry-run` to report a proposed installation without writing.
- `icbsetup genhelp check` validates sources and settings. With a board found,
  it also preflights ownership conflicts and reports shadowing display variants.
- `icbsetup genhelp export help-sources` writes the embedded catalog and the
  per-topic English Markdown into a nonexistent or empty directory.

Generation and check take an optional **positional** board: a board directory or
an `icboard.toml` file. Without it, normal discovery applies (current directory,
then `ICB_PATH`); generation requires a board, while check falls back to built-in
defaults. Both also accept `--sources DIRECTORY`, `--language EXT`,
`--theme classic|minimal|FILE`, `--width 40..79` and `--cp437[=BOOL]`. Generated
files are UTF-8 with a BOM unless `--cp437` selects the legacy encoding.
Generation additionally accepts `--output DIRECTORY`, `--dry-run`, `--adopt` and
`--replace-modified`. Export takes only its destination.

### Generating without a board

`icbsetup genhelp --output out` renders the topics into a plain directory. That
mode takes no board lock, writes no ledger and keeps no backups, so nothing can
be repaired later; it also rejects a positional board and `--replace-modified`.
The directory is created when missing. Existing files with generated names abort
the whole batch unless `--adopt` is given. `--dry-run` writes nothing and does
not create the directory.

### Sources and settings

Help sources are one flat bundle: `catalog.toml` beside `hlpa.md`, `hlpb.md` and
the rest. Use an exported bundle as editable sources, or supply only the changed
topic files in an override directory. Omitted topics keep their embedded sources;
a local catalog is optional and may describe a subset. Locale subdirectories such
as `en/` or `de/` are rejected. Start each Markdown file with one level-one
title followed by substantive help content; title-only sources and overrides are
errors, not warnings. Check and generation reject them before writing outputs
or installation bookkeeping. Keep layout and colors in the theme, not raw
display controls. Run check after editing: unsupported Markdown, characters
and overlong examples are errors.

### Translated help

`--language` sets the language suffix of the generated file names:

```sh
icbsetup genhelp export help-sources
# translate help-sources/hlp*.md in place
icbsetup genhelp . --sources help-sources --language ger
```

That writes `hlpa.ger.pcb` and so on; without `--language`, names stay
unsuffixed (`hlpa.pcb`). The extension is 1 to 32 lowercase letters or digits and
must start with a letter. Help generation never reads the board's language
definitions, so matching the extension the board actually uses is the sysop's
responsibility. `--language` only names the output; it does not select a
translated source, so pass the translated sources as well.

The exported English catalog preserves source/provenance hashes. The 20 legacy
German Markdown sources under `crates/icy_board_help/data/de/` and catalog translation metadata
remain in the repository for a future, separate localization feature; they are
not embedded, exported or generated. There is no translation-coverage validation
in help generation. The installation ledger records effective source,
rendering-settings and output hashes.

There is no board configuration for help generation. Every setting is a command
line flag and applies to that invocation only; nothing is stored in
`icboard.toml`. A leftover `[help_generation]` section from an older board is
ignored and does not stop the board from loading.

| Flag | Default and meaning |
| :--- | :--- |
| `--theme` | `classic`; also `minimal`, or the path to a theme TOML file |
| `--width` | `79`; fixed generated width from 40 through 79, not per-caller reflow |
| `--cp437[=BOOL]` | UTF-8 **with BOM**, also in `.pcb` files; `--cp437` writes legacy CP437, `--cp437=false` keeps UTF-8 |
| `--sources` | Embedded sources; an override directory replaces the topics it contains |
| `--language` | Unsuffixed output names |

Narrow widths are validated, not reflowed: the embedded English topics contain
code lines that need at least 68 columns, and a width below that fails the whole
batch instead of truncating.

The `--cp437` value form needs the equals sign; `--cp437 false` is not accepted,
because the space form leaves `false` as the positional board. CP437 output
rejects characters it cannot represent, which fails the whole batch.

`--sources` and `--output` are relative to the invoking directory, not to the
board root; absolute paths are supported. A `--theme` value other than `classic`
or `minimal` is read as a theme TOML file. An unreadable, malformed or invalid
theme file fails before anything is written. Its supported fields are DOS
attributes `title`, `heading`, `body`, `emphasis`, `code`, `note`, `border`
(1 through 127), `margin` (0 through 19) and `decoration` (boolean).

Output name suffixes come from `--language` alone, never from board language
definitions or the CLI UI language. Existing German help files and custom
language variants on installed boards are neither modified nor deleted. Runtime
language fallback and custom variant precedence remain unchanged, as does the
CLI's German UI localization.

### Safe regeneration

Take the board offline before generating: installation into a board and new-board
creation hold `BoardLock`. Check, dry runs, export and `--output` generation do
not acquire the lock or write board files. There is no startup regeneration or
online update mode.

Unchanged managed files can be updated; local edits are protected by their
recorded hashes. For legacy/imported files, use `--adopt` to explicitly replace
and take ownership of unmanaged output. Identical unmanaged files remain
unclaimed unless adopted. Already-managed local edits instead require
`--replace-modified`; `--adopt` is not a force flag for them. Inspect `--dry-run`
first. Conflicts abort the whole batch before new installation writes: there is
no per-topic selection. Unknown or shadowing artwork is not deleted; warnings may
require manual cleanup.

Bookkeeping lives under the board's `main/`: `help-generation.toml` is the
ledger, `help-generation.pending.toml` the active transaction journal, and
`help-generation.backups/transaction-*/` retains prior bytes and a
`transaction.toml` manifest mapping numbered `.before` backups to output names.
Writes are atomic per file, **not** for the whole batch. On failure the installer
attempts rollback; after interruption the next apply rolls back the pending
transaction before planning new work. Use the same output directory for recovery;
dry-run reports recovery without performing it. Changed recovery targets or
damaged backups block recovery rather than overwriting unknown data. Keep the
journal and backups until recovery succeeds. Backups are retained, not rotated
automatically. The ledger records absolute output locations, so after moving a
board inspect dry-run and explicitly adopt the relocated files if needed.

### Coverage

All 68 English topics contain substantive help: the fourteen former title-only
placeholders have been completed, and sixteen numeric sysop topics, `hlp1`
through `hlp16`, have been added. The bundle covers the fixed file-name mappings
in the original PCBoard [help dispatcher](../pcboard/pcb-main/SOURCE/DISPLAY/HELP.C),
including sysop commands `1` through `15`, plus Icy Board's command `16` and its
additional topics. Arbitrary custom help names still require sysop-supplied files.
Virtual `HLPMORE` and `HLPXFRMORE` pagination help remains in ICBTEXT, not in
generated files.

Help coverage does not imply new command implementations: topics for `9`, `10`,
`14` and `15` explain compatibility limitations; those numeric commands remain
unimplemented. The 20 legacy German translations retained in the repository are
unreviewed and are not embedded, exported or generated. Original `.icy` help
assets remain in the source tree as migration provenance. See
[Known limitations](known_limitations.md#command-help).

## Board layout

The important generated paths are:

| Path | Purpose |
| :--- | :--- |
| `icboard.toml` | Main configuration and paths to the other data files |
| `icboard.log` | Runtime log |
| `art/` | Display, menu, help and command artwork |
| `main/` | Users, conferences, commands, languages, protocols and security data |
| `conferences/` | Conference-specific message, file and display data |
| `tmp/` | Generated compatibility and work files |

Most individual locations can be changed in `icboard.toml`; the tools are
preferable to hand-editing until the board has been tested.

## What to read next

- [Differences and improvements](differences.md) explains how Icy Board
  modernizes PCBoard and which old tools that affects.
- [Known limitations](known_limitations.md) is the pre-production checklist.
- [File areas](icbfile.md) covers importing and maintaining file bases.
- [PPL and PPEs](ppl.md) covers the runtime and toolchain.
- [Feature status](feature_parity.md) is the detailed compatibility matrix.
