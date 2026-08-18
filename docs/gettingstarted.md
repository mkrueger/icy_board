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

## Configure the essentials

From the board directory, run:

```sh
icbsetup
```

Set the board and sysop names, choose a permanent sysop password, inspect the
network listeners and confirm the node count. Options that the runtime does not
use yet are greyed out and explain why.

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
2. `E` enters a message.
3. `R` reads it back and tries the reply and scan commands.
4. `F` opens the file directories.
5. `V` shows the caller settings.
6. `G` logs off.

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
