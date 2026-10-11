# Board concepts and terminology

Icy Board keeps the caller-facing conventions of PCBoard while using native
processes and different storage formats. Understanding the boundaries below
helps avoid treating an imported PCBoard file as an active Icy Board database.

## Board root and configuration

A **board** is an installation with a main configuration, users, messages, file
areas, artwork and runtime state. It is separate from the source checkout and
from the directory containing the program binaries.

The main configuration is normally `icboard.toml`. Most board-relative paths
start at its containing directory, not the current shell directory and not
the directory of a referenced component file. An absolute path can point
outside the board root, so copying the board directory is not necessarily a
complete backup.

Use [configuration formats](../configuration/README.md) to distinguish the
main configuration from conference lists, area lists, runtime state and tool
configuration. A setting that contains a filename references a separate
document; its fields do not belong inside the main configuration.

## Nodes, callers and sessions

A **node** is a caller connection slot. A **session** is a caller's current
interaction with the board; the **user record** is the persistent account.
Disconnecting ends a session, not the account.

The call-waiting screen runs alongside network listeners. Local **User** and
**Sysop** logins do not put the network side into a DOS-style busy mode.
`icboard --localon` is not a loopback-only networking option.
See [local testing](administration.md#before-the-first-call) and the
[call-waiting reference](../source/icy_board.rst).

## Conferences and message areas

A **conference** groups menus, message areas, file directories, bulletins,
surveys and doors. `J` joins a conference; `A` abandons it.

A **message area** is one discussion base inside a conference. Icy Board
supports multiple areas per conference, each with a name, access expression,
JAM base and optional network association. `AREA` changes the current area.
The conference number and message-area identifier are not interchangeable.

Messages live in **JAM** databases, not PCBoard's old message-base format.
Message numbers and last-read positions belong to their area. Older PPE calls
use compatibility behavior for the default area; newer code should use the
[area-aware PPL API](../new_ppl.md).

Selection for scanning is distinct from access. Selecting a conference does
not grant permission to read it. Likewise, knowing a message number does not
override private-message or group-password checks.

See [conference and area configuration](../configuration/components.md),
[message reading](../../crates/icy_board_help/data/en/hlpr.md) and
[conference selection](../../crates/icy_board_help/data/en/hlpsel.md).

## Personal mail

The **personal inbox** is separate from public conference areas:

- `@` opens the inbox.
- `@W` writes personal mail.
- `Y` includes personal-mail checking.

Private messages in conference areas, personal inbox mail and FTN netmail
are related concepts but are not the same storage or delivery path.
In particular, incoming FTN netmail is not a general per-user inbox gateway;
check the [mailer limitations](../known_limitations.md).

Message security controls visibility; it is not message encryption.
Read the [message-security reference](../../crates/icy_board_help/data/en/hlpsec.md)
before using sender-password or group-password messages.

## File directories, archives and metadata

A **file directory** or **file area** contains downloadable files and a database
of descriptions and attributes. The physical file and its database entry are
different things: putting an archive into a directory does not automatically
complete an import of its metadata.

Use `icbfile` for scanning, importing listings and maintaining metadata.
An imported `DIR` or `FILES.BBS` listing is input to that process, not a live
replacement for the native database.

An archive can contain a `FILE_ID.DIZ` description. The
[file-area guide](../icbfile.md) explains which descriptions are authoritative
and what a scan can overwrite. [Upload processing](../upload_processing.md)
adds quarantine, scanner and publication rules before an upload is made
available to callers.

## Access, registration and subscriptions

Access may depend on security level, group membership, age, conference
registration and the action being attempted. A numeric threshold is only
one kind of [security expression](../configuration/board.md#security-expression-representation).

Conference registration flags include subscription-sensitive registration,
registration independent of expiry, lockout, scan selection and temporary
conference sysop status. See the
[registration flags](../../crates/icy_board_help/data/en/hlpreg.md).

Subscriptions govern expiry and associated access. [Accounting](../accounting.md)
governs monetary credits and charges. File ratios and transfer ceilings are
another independent set of limits. Increasing one does not necessarily bypass
the others.

## Display files, prompts, menus and help

| Surface | Purpose | Where to start |
| --- | --- | --- |
| Display file | Artwork or text sent to the caller; may contain macros. | [Display conventions](../source/customizing/display_files.rst) |
| `@` macro | Expands caller/board values or performs an output-control action. | [Macro reference](../macros.md) |
| ICBTEXT | Numbered system prompts and messages. | [Text editor](tools.md#mkicbtxt-system-prompts) |
| Menu | Presentation and actions attached to command choices. | [Menu editor](../mkicbmnu.md) |
| Command list | Adds or overrides dispatch at the command prompt. | [Command configuration](../configuration/commands.md) |
| Command help | Per-topic help installed for callers. | [Help generation](../gettingstarted.md#command-help) |

A UTF-8 display file requires a UTF-8 BOM; without it, legacy display content
is interpreted as CP437. This is different from structural TOML text, which
is UTF-8. Keep both English and any configured translated variants in mind
when changing visible output.

## PPL, PPE and doors

**PPL** is the source language; a **PPE** is compiled executable bytecode.
`pplc` compiles and `ppld` decompiles. Source-language version and bytecode
runtime version are separate compatibility decisions.

A **door** is an external program or supported remote-door service. Drop files
describe the caller to the external program. A DOS executable is not a PPE,
and native DOS-door support has its own image, licensing and runtime
requirements.

Start with [PPL](../ppl.md), [door configuration](../configuration/components.md#doors-and-bbslink-accounts)
or [native DOS doors](../native_dos_doors.md).

## PCBoard terminology crosswalk

| PCBoard term | Icy Board equivalent / boundary |
| --- | --- |
| `PCBOARD.DAT` | Import source; use `icboard.toml` for the active board. |
| `PCBSETUP` | `icbsetup`. |
| `PCBSM` | `icbsm`; original reports and index files are not all reproduced. |
| `MKPCBTXT` / `PCBTEXT` | `mkicbtxt` / native ICBTEXT data. |
| `MKPCBMNU` | `mkicbmnu`. |
| `CMD.LST` | [Native command lists and legacy import](../configuration/commands.md). |
| `CNAMES` | Imported conference configuration; see [components](../configuration/components.md). |
| `PWRD` | Security-level time and transfer-limit tables. |
| `DIR` / `FILES.BBS` | Importable file listings; maintain native areas with `icbfile`. |
| Questionnaire | Survey. |
| FOSSIL / modem node | Not the ordinary network listener model; DOS-door emulation is a separate feature. |

For deliberate differences rather than terminology, read
[differences and improvements](../differences.md).
