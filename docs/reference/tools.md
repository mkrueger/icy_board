# Programs and command-line tools

This is the entry point for every operator-facing program in the board
toolchain. Run `PROGRAM --help` for the syntax accepted by your installed
binary and `PROGRAM --version` when reporting a problem. The examples below
assume the programs are on `PATH`.

## Choosing a tool

| Program | Purpose | Detailed reference |
| --- | --- | --- |
| `icboard` | Run the board, local sessions and PPEs. | [Below](#icboard-the-server) and [call-waiting screen](../source/icy_board.rst) |
| `icbsetup` | Create, import, configure and check a board; generate help. | [Below](#icbsetup-board-configuration) and [configuration](../configuration/README.md) |
| `icbsm` | Manage users, groups and bulk user-file maintenance. | [Below](#icbsm-users-and-groups) |
| `icbfile` | Maintain file areas, metadata and archives. | [File-area guide](../icbfile.md) |
| `icbmailer` | FTN, QWKnet and ZCONNECT exchange. | [Below](#icbmailer-network-mail) |
| `mkicbtxt` | Edit system prompts and messages. | [Below](#mkicbtxt-system-prompts) |
| `mkicbmnu` | Create, edit, preview and check menus. | [Menu-editor guide](../mkicbmnu.md) |
| `pplc` | Compile, check and format PPL. | [Compiler reference](../pplc.md) |
| `ppld` | Decompile, disassemble and inspect PPE compatibility. | [Decompiler reference](../ppl.md#decompiler) |
| `ppl-lsp` | Provide editor language services over standard input/output. | [Editor installation](../../INSTALL.md#ppl-in-your-editor) |

These are separate programs. Do not assume an option or positional argument
accepted by one tool applies to another.

## Board selection and safe use

`icboard`, `icbsetup` and `icbsm` accept an optional board/configuration path.
Without an explicit path, ordinary discovery checks the current directory and
`ICB_PATH`. Passing a path explicitly is preferable in scheduled jobs.

`icbmailer` takes the board configuration on its subcommands. `icbfile` normally
takes a physical directory or an area-list file, **not** the main board
configuration. `mkicbtxt` takes an ICBTEXT file; `mkicbmnu` takes a menu file
and can receive board context separately.

Stop conflicting writers before offline maintenance. A read-only preview is
different from a mutating operation; the tables below identify important
boundaries. Never use a force, prune, replace or acknowledgement flag merely
to get past an unexplained error.

## icboard: the server

```text
icboard [OPTIONS] [BOARD]
```

| Option | Meaning |
| --- | --- |
| `-f`, `--full-screen` | Use the full terminal rather than the default 80x25 layout. |
| `--localon` | Start a local login. Network listeners still run. |
| `--ppe FILE` | Execute a PPE in the local board context. |
| `--runppe VALUE` | Run with a user login and semicolon-separated parameters; see below. |
| `--key TEXT` | Supply stuffed key characters. |
| `--version` | Print version and exit. |

The `--runppe` value uses
`first;last;PWRD:password;PPE:file.ppe;param1;param2;...`.
It can expose credentials through process arguments or shell history; do not
put a real password in documentation, shared scripts or support logs.

For a first trial, configure loopback listeners before running:

```sh
icbsetup check icboard.toml
icboard --localon icboard.toml
```

The application log belongs to the selected board configuration. See
[administration](administration.md) for startup and shutdown practice.

## icbsetup: board configuration

With no subcommand, `icbsetup [BOARD]` opens the configuration editor.
`-f` / `--full-screen` changes its layout.

| Subcommand | Arguments and important options | Effect |
| --- | --- | --- |
| `create` | `DIRECTORY` | Create a new board in a nonexistent directory; prints its initial sysop password. |
| `import` | `PCBOARD.DAT DIRECTORY`, repeatable `--map`, `--dry-run` | Import a PCBoard installation; follow the [migration procedure](../migration.md). |
| `check` | `[BOARD]`, `--create-dirs` | Validate board configuration and paths; the flag offers to create missing directories. |
| `ppe-convert` | `PATH` | Convert an editable PPE tree's text; back up first and inspect binary data assumptions. |
| `genhelp` | See below. | Generate, check or export caller help. |
| `dos-image` | `DIRECTORY` | Prepare native DOS-door image resources. |
| `dos-copy` | `IMAGE SOURCE DESTINATION` | Copy a host file into a DOS image. |
| `dos-fossil` | `DIRECTORY ARCHIVE`, required `--accept-license`, optional `--door NAME` | Install the licensed FOSSIL archive into the DOS-door setup. |
| `dos-console` | See `icbsetup dos-console --help`. | Enter the DOS maintenance console; see [native DOS doors](../native_dos_doors.md). |

`check` without `--create-dirs` is the normal validation command. It does not
prove that listeners are private, passwords are appropriate or callers can
complete a session.

### Help generation

```text
icbsetup genhelp [OPTIONS] [BOARD]
icbsetup genhelp check [OPTIONS] [BOARD]
icbsetup genhelp export DIRECTORY
```

Generation and checking share:

| Option | Meaning |
| --- | --- |
| `--sources DIRECTORY` | Flat Markdown override bundle; omitted topics use embedded English sources. |
| `--language EXT` | Output filename suffix, not automatic translation. |
| `--theme classic|minimal|FILE` | Built-in theme or a theme file. |
| `--width NUMBER` | 40–79 columns; the complete embedded English set needs at least 68. Default 79. |
| `--cp437[=BOOL]` | CP437 output when true; otherwise UTF-8 with BOM. |

Generation additionally supports `--output DIRECTORY`, `--dry-run`, `--adopt`
and `--replace-modified`. Standalone `--output` does not use a board ownership
ledger or backups and cannot be combined with a positional board or
`--replace-modified`. Export takes only its destination, which must be
nonexistent or empty.

Read the [full ownership and translation workflow](../gettingstarted.md#command-help)
before adopting existing help or replacing locally modified files.

## icbsm: users and groups

```text
icbsm [OPTIONS] [BOARD]
```

Without a batch operation, the program opens the user/group editor. The
interactive manager also exposes selection-based bulk edits and security-level
tables; see [the manager screens](../source/icy_board.rst).

| Option | Meaning |
| --- | --- |
| `-f`, `--full-screen` | Use the full terminal. |
| `--pack` | Remove selected user records instead of opening the editor. |
| `--inactive-days DAYS` | Select inactive users by days since calling. |
| `--never-logged-on` | Select users who have never logged on. |
| `--no-delete-flagged` | Do not select records solely because they are marked for deletion. |
| `--keep-security LEVEL` | Protect users at or above this level; packing defaults to 100. |
| `--pack-locked-out` | Include locked-out users rather than protecting them. |
| `--standardize-phones` | Standardize phone-number formatting. |
| `--undo` | Restore the user file from the tool's previous backup. |
| `--dry-run` | Report the selected records without writing. Use with a batch operation. |
| `--version` | Print version and exit. |

Packing also selects disabled users, protects the first record and normally
keeps locked-out users. Selection/protection conditions are not simply an
unconditional deletion of every account older than the chosen age.

Choose one batch operation per invocation. If combined, `--undo` takes
precedence; otherwise packing takes precedence over phone standardization.
The tool backs up before a write, but a single previous-run backup is not an
archive of all earlier maintenance.

```sh
icbsm --pack --inactive-days 365 --dry-run icboard.toml
icbsm --standardize-phones --dry-run icboard.toml
```

Review the listed accounts before repeating an operation without `--dry-run`.

## icbfile: file-area maintenance

The [full file-area guide](../icbfile.md) defines target selection, area ranges,
metadata precedence, all options and examples.

| Subcommand | Purpose / write boundary |
| --- | --- |
| `areas AREA_LIST` | List configured file areas. |
| `list TARGET` | List indexed files; `--long` includes additional metadata. |
| `scan TARGET` | Read physical files and update metadata; `--all` and `--force` broaden processing. |
| `check TARGET` | Report missing physical files; `--prune` removes stale entries. |
| `import TARGET LISTINGS...` | Import PCBoard DIR or FILES.BBS descriptions; supports `--dry-run`. |
| `export TARGET` | Write a listing, optionally to `--output FILE`. |
| `set TARGET FILE` | Change `--desc`, `--free BOOL` or `--locked BOOL`. |
| `delete TARGET FILE` | Delete the selected file/entry as documented by the file-area guide. |
| `repack TARGET` | Rewrite archives; supports `--dry-run` and resource ceilings. |
| `fingerprints INPUT` | Build an advertisement fingerprint catalog, default output `bbstros.toml`. |

Most operations accept `--area` / `-a` with an area-list target. Imports support
`--format auto|pcboard|filesbbs` (including legacy aliases), `--overwrite`
and `--keep-missing`.

Repacking defaults are compression level 9, 10,000 members, 512 MiB per member,
2 GiB expanded data and maximum compression ratio 1,000. Review these limits
and the fingerprint catalog before modifying valuable archives.

## icbmailer: network mail

`CONFIG` in these commands is the **main board configuration**, not the
referenced FTN/QWKnet/ZCONNECT component file.

| Command | Action |
| --- | --- |
| `links CONFIG` | List FTN links and queued work. |
| `scan CONFIG` | Pack locally written messages for configured FTN links. |
| `poll CONFIG [ADDRESS]` | Call one FTN link, or all when omitted. |
| `toss CONFIG` | Import received FTN mail into bases. |
| `show FILE` | Inspect a packet/bundle; `--text` / `-t` includes message text. |
| `qwk-links CONFIG` | List QWKnet hubs. |
| `qwk-scan CONFIG [HUB]` | Create outbound REP packets. |
| `qwk-toss CONFIG [HUB]` | Import inbound QWK packets. |
| `qwk-poll CONFIG [HUB]` | Scan, exchange and import. |
| `zconnect-links CONFIG [LINK]` | List links and public-board mappings. |
| `zconnect-scan CONFIG [LINK]` | Prepare retryable outbound ZIP packets. |
| `zconnect-toss CONFIG [LINK]` | Import inbound ZIP packets. |
| `zconnect-poll CONFIG [LINK]` | Scan, exchange and import. |
| `zconnect-ack CONFIG LINK` | Acknowledge verified external delivery and retire the pending packet. |

FTN `scan`, `poll` and `toss` accept `--verbose` / `-v`; `poll` also accepts
`--keep` / `-k` to retain delivered outbound files. Optional hub/link identifiers
normally select all configured peers when omitted.

**Do not use `zconnect-ack` to clear an error.** It asserts that an external
offline mailer has successfully delivered the pending packet.

Live database writes must be coordinated with the board. Raw `icbmailer`
does not acquire `BoardLock`; use the offline or maintenance workflow described
in [events](../events.md). See [FTN](../source/mailer.rst),
[QWKnet](../qwknet.md) and [ZCONNECT](../zconnect.md) for complete setup and
interoperability limits.

## mkicbtxt: system prompts

```text
mkicbtxt [OPTIONS] FILE [NEW_TEXT]
```

| Option | Meaning |
| --- | --- |
| `-c`, `--create` | Write a default text file instead of opening the editor. |
| `-i`, `--update NUMBER` | Replace one numbered record using `NEW_TEXT`. |
| `--convert` | Import the loaded text file to a `.toml` output. |
| `--force` | Permit an existing create/convert target to be overwritten, with a backup. |
| `-b`, `--board PATH` | Supply board context for the editor palette. |
| `-f`, `--full-screen` | Use the full terminal. |
| `--version` | Print version and exit. |

Without a noninteractive operation the tool edits the text file. Preserve
record meanings, substitutions and required header/detection rules; see
[ICBTEXT format](../configuration/data.md). These records are separate from
the command-help topics generated by `icbsetup genhelp`.

## mkicbmnu: menu editor

```text
mkicbmnu [--create | --check] [--board BOARD] [--full-screen] FILE
```

`--create` / `-c` creates a new menu; `--check` performs read-only validation
and exits nonzero for errors. They cannot be combined. `--board` / `-b`
provides explicit board context; otherwise the editor looks above the menu
for a board. `--full-screen` / `-f` uses the terminal dimensions.

The tool uses the `.mnu` extension and requires a valid board context. Preview
does not execute menu actions or PPEs. The [menu guide](../mkicbmnu.md) covers
every editor field, action type, shortcut, save/undo workflow and preview limit.

## PPL development tools

### pplc

Compile a source file or package, or use the non-writing diagnostic modes:

```sh
pplc --print-config hello.pps
pplc --check hello.pps
pplc --format --stdout hello.pps
```

`--check` checks both diagnostics and formatting; a formatting difference also
returns nonzero. `--format` without `--stdout` rewrites source.
`--runtime` selects the executable target; `--lang-version` selects source
language. Neither is the same as `--version`.

See [the compiler reference](../pplc.md) for every option, package manifests,
encoding, defines, compression, debug names and configuration precedence.

### ppld

```sh
ppld --check --strict example.ppe
ppld --output example.ppe
```

By default decompilation writes a `.ppd` file. `--output` / `-o` writes source
to stdout and keeps diagnostics on stderr. `--raw` / `-r` disables structured
reconstruction; `--disassemble` / `-d` prints instructions. `--cp437` selects
legacy output encoding; `--style u|l|c` selects keyword case.

`--lang-version` overrides `PPL_LANG_VERSION`, which otherwise overrides the
stored runtime as the output-language default. Original source language,
names and formatting cannot generally be recovered from bytecode.

`--check` reports compatibility findings; add `--strict` if findings must
produce a failing exit status. This does not execute the PPE.
See [the decompiler reference](../ppl.md#decompiler).

### ppl-lsp

The language server speaks LSP over standard input/output and is normally
started by an editor, not interactively by a sysop. It provides diagnostics,
completion, hover, signatures, navigation, references and formatting.
Tree-sitter supplies syntax highlighting separately.

Follow [editor installation](../../INSTALL.md#ppl-in-your-editor) for VS Code,
Zed, Helix and Neovim. Open the project containing `ppl.toml` when working on
a package so the server can see its configuration and other source files.
