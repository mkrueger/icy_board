# Migrating from PCBoard

Icy Board can import a PCBoard installation, but migration is not an in-place
upgrade. The importer reads the old board and writes a separate Icy Board tree.
It never modifies the source installation.

The goal is to preserve what callers and PPEs observe while replacing the parts
that belong to DOS: drive letters, binary setup files, PCBoard message bases and
DIR databases. A simple board may need little more than path review. A heavily
customized board with PPE-specific data files needs a deliberate test pass.

Before starting, read [known limitations](known_limitations.md) and
[differences and improvements](differences.md).

## 1. Inventory the old board

Keep a runnable copy of the original. Record at least:

- the `PCBOARD.DAT` used by each node
- every drive letter and network path referenced by setup or conference files
- third-party PPEs and the data files they read or write
- doors and their drop-file requirements
- transfer protocols and external commands
- message networks, conference numbers and security assumptions

The original board is the best acceptance test. If a prompt or PPE behaves
differently, compare the same action on both systems.

## 2. Dry-run the import

Run the importer without keeping its output:

```sh
icbsetup import /path/to/PCBOARD.DAT migrated-board --dry-run
```

A directory may be given instead; `icbsetup` will locate `PCBOARD.DAT` in it.
The dry run reports paths it could not resolve and writes nothing to the chosen
destination.

Map DOS drives to their mounted locations with repeatable `--map` options:

```sh
icbsetup import ~/dos/PCB migrated-board --dry-run \
  --map 'C:\PCB=/home/sysop/dos/PCB' \
  --map 'D:\FILES=/srv/bbs/files'
```

Use quotes so the shell leaves backslashes alone. Prefer mapping a drive or a
stable root over adding one special case per file.

## 3. Import into a new directory

The destination must not exist:

```sh
icbsetup import ~/dos/PCB migrated-board \
  --map 'C:\PCB=/home/sysop/dos/PCB' \
  --map 'D:\FILES=/srv/bbs/files'
```

Read `importlog.txt` and `import_report.txt` before editing anything. They show
what was converted and which paths still need attention.

## 4. Validate paths

Icy Board is designed around paths relative to `icboard.toml`. Relative paths
make a board movable and avoid carrying DOS drive letters into the new setup.

```sh
icbsetup check migrated-board/icboard.toml
```

The checker understands display-file variants such as `welcome`,
`welcomeg.ans` and language or security suffixes. On case-sensitive systems it
reports a DOS-style case mismatch separately from a missing file.

To offer to create missing directories:

```sh
icbsetup check --create-dirs migrated-board/icboard.toml
```

Directories inside the board default to yes. Absolute paths outside the board
default to no because they may be typographical errors.

## 5. Review deliberate storage changes

Some old tools cannot be carried over because the storage they access changed:

| PCBoard | Icy Board | Migration consequence |
| :--- | :--- | :--- |
| PCBoard message bases | JAM | Import or recreate bases; tools that open the old files directly cannot work unchanged. |
| DIR databases | SQLite-backed file areas | Use `icbfile` to import and normalize the file base. |
| Binary setup files | TOML | Use `icbsetup`; do not copy old setup files over the generated configuration. |
| Plain-text passwords | Argon2id/bcrypt by default | PPEs that read passwords need review or the explicitly insecure compatibility fallback. |

See [File areas](icbfile.md) for archive and description import.

PCBoard's one-message-base-per-conference model remains valid after import: it
becomes the default message area. Additional named areas can then be added to a
conference without changing old PPE calls. Icy Board's personal mail inbox is a
new, separate JAM base and therefore has no PCBoard database to import.

## 6. Review every PPE

The PPL runtime targets PCBoard compatibility, but no importer can infer what a
third-party PPE has hard-coded. Check each PPE for:

- DOS drive letters and backslashes in data files
- assumptions about 8.3 or uppercase file names
- direct reads or writes of PCBoard user, message, menu or DIR formats
- calls to DOS or assembler functions
- assumptions that a conference contains only one message area
- reads of the caller's plain-text password

Run the PPE, not only its startup path. Exercise its save path, error path and
sysop-only actions too. An existing PPE that stays within the documented PPL
and PCBoard APIs is expected to run; a failure is a compatibility bug worth
reporting.

## 7. Test as caller and sysop

Before opening a listener to the network:

1. Start with `icboard --localon` and inspect `icboard.log`.
2. Join every conference and open each message and file area.
3. Enter, reply to, scan and delete a test message.
4. Upload and download through every configured protocol.
5. Run doors, PPE replacements and scheduled events.
6. Connect through the same network protocol callers will use.
7. Compare important prompt flows with the original board.

The detailed command status is in [feature parity](feature_parity.md). Exact
known mismatches are tracked in the [command audit](../compat/COMMAND_AUDIT.md).

## Report useful failures

Importer reports from real installations are especially valuable. Include the
source PCBoard version, the unresolved-path report, the relevant configuration
shape and the smallest PPE or action that reproduces the problem. Remove user
records, passwords and private message content before attaching files.
