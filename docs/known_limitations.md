# Known limitations

What icy_board does not do, as of the beta. This is the list to read before
moving a board over, so nothing here is a surprise at two in the morning.

Three companion documents say the same thing in more detail:
[compat/OPTIONS_AUDIT.md](../compat/OPTIONS_AUDIT.md) for the switches a sysop
can set but the board ignores, [compat/COMMAND_AUDIT.md](../compat/COMMAND_AUDIT.md)
for where a command answers differently than PCBoard did, and
[differences.md](differences.md) for the deliberate departures.

For an existing installation, use the [migration guide](migration.md) as the
procedure and this page as its risk checklist.

## Not implemented

| Area | What is missing |
| :--- | :--- |
| Modem | Callers reach the board over telnet, SSH and websockets. There is no serial or modem support, and no FOSSIL driver. |
| Accounting | Activity charging, enforcement, peak/holiday rates, credit macros, display files and setup controls are implemented; see the [operator guide](accounting.md). No per-file NoTime/FSEC monetary transfer-time refund, transactional ledger, crash-atomic account/audit commit or cross-process funds reservation. Currency display is fixed US-dollar style, not locale-selected. Audit failures are logged without undoing posted charges. |
| Upload credits | Uploading earns configured byte credit but not time credit, and uploads are not test-extracted. The configured free-space threshold is enforced before a transfer starts. |
| FTN | icy_board is a leaf or point over BinkP: scan, poll, toss, routing and AreaFix work, with AKA and link setup in ICBSetup. There is no BinkP answering side, and incoming netmail arrives in a single dump base rather than a per-user base. |
| Web | There is no browser-based caller frontend or caller API. A running board can host the optional token-protected web administration interface, and a sysop may enable policy-controlled outbound HTTP for PPL 4.00 scripts. |
| Sysop numeric commands | Commands `9`, `10`, `14` and `15` are missing. The level named for command 10 protects `PPE` instead; commands `1`, `2`, `3`, `4`, `5`, `6`, `7`, `8`, `11`, `12`, `13` and `16` work. |
| Message reader | Inside the read loop, export (`X`), `EDIT`, `FORWARD`, `VIEW` and the capture actions (`C`, `D`, `Z`) are recognised and answered but do not run. |
| ICBSM | Editing users and groups, sorting and packing the user file, the bulk edits over a selection of users and the security level tables. Reports, index files and the user info file of the original have no equivalent here. |
| Command help | Generation installs 68 substantive English topics, including sysop help `1`–`16`. Help for unimplemented commands `9`, `10`, `14` and `15` documents compatibility limits, not new functionality. Translated output is a file-naming workflow over sysop-supplied sources, not a shipped localization. See [Command help](#command-help). |

## Works, but not the way PCBoard did it

| Area | What to expect |
| :--- | :--- |
| Config files | TOML, editable in any text editor. Old formats are written out again for PPEs that read them, but a PPE that writes one will not be heard. |
| Message bases | JAM. Tools that read PCBoard's old base will not work. |
| DIR files | Binary, they carry the metadata the archives do not. |
| Encoding | Structural text is UTF-8. Display files with a UTF-8 BOM are UTF-8; display files without it are read as CP437. See [differences.md](differences.md). |
| Passwords | Hashed by default. The plain text fallback exists for PPEs that read the password and is a security risk. |
| Email password recovery | Optional and off by default; requires hashed account credentials and an existing email address. Stored mailbox ownership is not verified, sysops/inactive accounts cannot recover, and there is no per-IP throttle or retry spool. SMTP TLS does not protect mailbox storage or cleartext Telnet entry. See [configuration](configuration/board.md#optional-email-password-recovery). |
| Access | Security level, group and age instead of a single level. |
| Events | Weekday schedules support positive start-anchored intervals, inclusive same-day latest starts, Maintenance/Online execution, stable IDs, durable history and confirmed manual runs. Monthly/date masks, overnight windows, PCBoard per-node scheduling/last-run dates and Fido/mail-hour modes are absent; `EVENT.DAT` is not imported. See [events.md](events.md). |

### Event operation

- Maintenance drains managed writers and reloads the board; Online does neither
	and **must not modify live board files**. Arbitrary shell commands are not
	sandboxed. Keep mailer live writes in Maintenance: raw `icbmailer` does not
	acquire `BoardLock`, and online safety has not been verified. Independently
	launched workers are outside the event restart handshake.
- Commands run foreground without a forced time limit; `warning_minutes` only
	logs a warning. Stalled commands or uncooperative sessions can delay maintenance
	indefinitely; background descendants are not tracked.
- The atomic journal prevents replay of claimed occurrences, but startup does
	not catch up downtime or retry interrupted/failed commands. It is not an
	exactly-once guarantee. Journal failures close admission pending repair/restart.
	History and command-log retention are unbounded; rotate logs separately and
	retain journal keys.

### Command help

New boards install generated English help without language suffixes;
imported/custom help is not automatically replaced. Generation changes neither
general artwork, admin TUI help nor
ICBTEXT. Original `.icy` assets remain as migration provenance, not the source
for new-board help rendering.

All 68 English topics now contain substantive help. The fourteen former
title-only placeholders have been completed and numeric topics `hlp1` through
`hlp16` added. This covers the fixed file-name mappings in the original PCBoard
[help dispatcher](../pcboard/pcb-main/SOURCE/DISPLAY/HELP.C), including sysop
commands `1`–`15`, plus Icy Board's command `16` and its additional topics.
Arbitrary custom help names still need sysop-supplied files. Virtual `HLPMORE`
and `HLPXFRMORE` pagination help remains in ICBTEXT rather than generated files.
Commands `9`, `10`, `14` and `15` are still unimplemented; their help topics
explain compatibility limitations and do not enable those commands.

Title-only sources, including local overrides, are rejected by the source loader.
Check and generation fail before writing outputs or installation bookkeeping.
The 20 legacy German Markdown sources under `crates/icy_board_help/data/de/`
and catalog translation metadata remain unreviewed in the repository for a
future, separate localization feature. German sources are not embedded,
exported or generated, and no shipped translation is installed.

`icbsetup genhelp --language ger` only suffixes the generated file names. It
does not select a translated source, is not checked against the board's language
definitions and performs no translation-coverage validation: the sysop exports
the flat English source bundle, translates it and passes it back with
`--sources`. Existing German help files and custom variants on installed boards
are neither modified nor deleted. Runtime fallback, custom variant precedence and
the CLI's German UI localization remain unchanged.

Generated width is fixed (40–79 columns), not responsive to each caller, and the
embedded English topics do not render below 68 columns.
Generated help is UTF-8 with the required BOM by default; `--cp437` writes the
legacy encoding and rejects characters it cannot represent. The renderer retains
support for German and other content within these encoding constraints. Verify
actual terminal output and pagination locally; there is no preview command.

Generation settings are command line flags only. There is no board configuration
for them, so a theme, width, encoding, source directory or language chosen once
has to be passed again on the next run; a leftover `[help_generation]` section in
an older `icboard.toml` is ignored.

Installation into a board requires an offline board lock. Managed local edits are
protected; `--adopt` applies to unmanaged files, while `--replace-modified`
explicitly replaces edited managed files. Conflicts abort the whole batch; single
topics cannot be selected. Atomic replacement is per file, not per batch:
interrupted transactions require journal/backup recovery on the next apply.
Backups are retained without automatic rotation. `icbsetup genhelp --output DIR`
skips the board entirely and therefore keeps no lock, ledger or backups, so its
output cannot be repaired or re-adopted later. See the
[command help guide](gettingstarted.md#command-help) for the commands, the
generation flags, source overrides and the recovery locations under `main/`.

## PPL 400 Beta

The implemented language, format and API contracts are documented in
[New PPL](new_ppl.md), the [language overview](ppl.md), the
[compiler guide](pplc.md) and the [PPE format](ppe_format.md).
These limits are part of the beta scope, not promises that the omitted features
will arrive before release.

### Compatibility and Loader

- Runtime 400 targets Icy Board, not an original PCBoard.
- Recompile unreleased 400 files written in the old container and older beta
	programs using unmarked array formals. Classic PPE compatibility is backed by
	the checked-in fixtures, not by a claim that every third-party PPE was tested.
- The loader validates container and section budgets, but `Executable::read_file`
	reads the whole file before those checks. The 64 MiB container budget does
	not cap that initial allocation. Install PPEs from trusted sources.
- Debug data can preserve symbol names, not source positions, source text or a
	separate debug file. Content identity is not authentication or encryption.
- The current release validation ran on Linux, where Zstd was built and tested.
	Windows/macOS execution, including Zstd support, remains unverified; no new
	loader fuzz acceptance was run.

### Files and Data

- PPE file sharing coordinates cooperating channels in one BBS process only.
	External tools, DOS doors, other processes and direct `DELETE`/`RENAME`/`COPY`
	operations do not participate. Read/modify/write requires the stable separate
	lock-file protocol in [record file I/O](new_ppl.md#record-file-io).
- Positional record files have no automatic schema migration or fingerprint.
	Record I/O supports fixed value layouts, not host objects or dynamic fields,
	even though those fields are supported in executable record layouts.
- The tested temporary-file/rename workflow is not a power-loss guarantee.
	`FFLUSH` does not promise `fsync`; cancellation can leave temporary files.
- File searches need an existing index, inspect bounded description prefixes,
	and may return an empty page with `HasMore=TRUE`. Pages are not one stable
	transaction and follow row ID, not display sort order. Marking is separate
	from the normal BBS download command; no typed transfer-result API is added.
- `Board.Users` remains a full array snapshot, built on first collection access.
	It has no userbase search/pagination API. Metadata reads do not build that array.

### Terminals and Messages

- String lengths and slicing count Unicode scalars, not display cells. UTF-8
	output uses grapheme-aware cells; CP437 boundaries retain substitution and
	their one-cell model. There is no public cell-width/cropping API or
	`Terminal.WriteText` safe-output helper. `StripATX` is not a sanitizer.
- Logical resize events currently cover Telnet NAWS and existing ANSI/board
	resizes, not every transport or pixel-only resize. Optional graphics, audio,
	fonts and input modes depend on the actual terminal's capabilities.
- E1 and E3 were accepted for the implemented scope. A remote protocol transfer
	from the browser and a complete versioned SyncTERM/icy_term/ANSI client matrix
	remain unverified, as do JXL Paint and differing fonts/cell metrics. Paint has
	no save/audio feature and selects its backend only at startup.
- Terminal cleanup is best-effort after disconnect and does not guarantee
	restoration after a process crash or forced cancellation. PPE file-channel
	reservations have their own cancellation-safe release; that is not a guarantee
	that terminal reset sequences reached the client.
- Low-level `Area.Read`, `Find` and `Msg.Text` are not the interactive reader's
	permission filter and do not mark messages read. Caller-facing PPEs must apply
	visibility rules and existing read-marker operations themselves; the
	not-for-display header flag has no dedicated object member. Use the `Session`
	methods for the board's interactive post/reply/edit workflow.
- `ledit` is an 80x25 editor limited to 200 lines of 76 characters. Real DOS
	editor compatibility remains installation-specific: ICE's full quote workflow
	and EXITINFO layout are not established. The ignored external-editor and
	LiQUiD package tests were not rerun in the current general regression suite.

### Deferred Additions

`TRY`/`CATCH`/`FINALLY` and `DEFER` are not implemented; use `ON ERROR` and
explicit application cleanup. JSON objects, additional file objects, UTC
timestamp types, asynchronous HTTP, further object-based session navigation
and door execution are not part of this beta scope. Existing procedural BBS
operations remain available. No new API is implied by these omissions.

## Import

Importing a PCBoard installation is best effort. Simple installations come over
well; the more a board relied on PPEs, absolute paths or drive letters, the more
hand work is left. `icbsetup import --dry-run` reports what it could not
resolve before anything is written, and `--map` translates a drive to a
directory. Every PPE has to be looked at one by one. The complete sequence is
in [Migrating from PCBoard](migration.md).

The importer is the part that most needs real installations to test against. If
one of yours does not come over, that is worth a bug report more than anything
else on this page.

## What is not planned

DOS, FOSSIL drivers, the PPE DOS and assembler functions, and printer support.
The machine underneath them is gone.
