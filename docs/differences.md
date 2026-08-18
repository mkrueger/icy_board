# Differences and improvements

Icy Board aims to preserve the behavior a caller, sysop or well-behaved PPE can
observe on PCBoard 15.4. It does not preserve DOS merely because PCBoard used
it, and it does not preserve an old storage format when that would keep the
board tied to old tools and file-system limits.

That gives the project a practical compatibility boundary:

- commands, prompts, display files, `@` macros and PPL behavior are compatibility
  surfaces
- DOS, modem control, printer output and the internal layout of PCBoard's
  databases are not

The exact incomplete features are in [known limitations](known_limitations.md).
This document covers deliberate changes and their consequences.

## Runtime and connectivity

Icy Board is a native Rust application for current operating systems. Callers
connect locally or over telnet, SSH and WebSockets; multiple nodes do not need
separate DOS processes and node-specific copies of every file.

**Why:** the board can run on maintained Linux, macOS and Windows systems,
including a Raspberry Pi, and can use ordinary service managers and remote
administration.

**Compatibility cost:** there is no serial or modem support, FOSSIL driver,
remote DOS shell, assembler bridge or printer output. PPEs that depend on those
facilities need replacement code.

## Paths and file names

Configuration paths are normally relative to the directory containing
`icboard.toml`; absolute paths remain available. Long file names are supported
throughout the native tools and through the PPL file API.

**Why:** a board can be moved, backed up or mounted elsewhere without rewriting
drive letters, and a file area is no longer constrained by 8.3 names.

**Compatibility cost:** a PPE that enforces an 8.3 limit itself still has that
limit. Unix file systems are case-sensitive. The importer and `icbsetup check`
perform case-insensitive diagnostics for old DOS names, but new configuration
should use the spelling that exists on disk.

## Text and encodings

TOML configuration and other structural text are UTF-8. Display files can be
legacy CP437 or UTF-8; a UTF-8 BOM (`EF BB BF`) marks UTF-8, and a file without
that marker is treated as CP437.

**Why:** current editors, source control and non-English text work normally,
while unmodified DOS artwork still displays correctly.

**Compatibility cost:** UTF-8 display files must carry the BOM. This is an
explicit choice rather than an encoding guess, because many CP437 byte streams
also happen to be valid UTF-8.

## Configuration is TOML

Board, conference, menu, protocol, language and security configuration is
stored as named TOML fields rather than opaque binary records. `icbsetup`,
`mkicbmnu` and the other tools remain the supported editors.

**Why:** configuration can be read, diffed, backed up and generated with normal
tools. Unknown or inactive settings are visible rather than buried in a record.

**Compatibility cost:** a PPE that directly writes a PCBoard binary setup file
cannot change the live configuration. Compatibility files are generated for
old PPEs where practical, but they are not a second writable source of truth.

## Messages use JAM

Message areas use JAM, including QWK/QWKE and FTN-backed areas. A conference
may contain several named message areas instead of one message base. Each area
has its own base, access expression and optional FTN area tag; scans cover all
selected areas.

**Why:** JAM is an established open format with existing tooling and does not
carry the proprietary layout of PCBoard's message base. Separating the caller's
conference membership from its message bases lets one subject community carry
several local or networked discussions without creating artificial conferences.

**Compatibility cost:** PPEs and external utilities that open PCBoard message
files directly must be changed. PPEs that use PPL message functions continue to
work through the runtime API and address the default area, so old code does not
need to understand the extension. Area-aware PPL uses `AreaId(conf, area)` or
the 4.00 conference and area objects.

## Personal mail has its own inbox

Icy Board keeps person-to-person mail in a separate JAM base rather than making
it another conference. `@` reads the current caller's inbox, `@W` writes to a
user or alias, and `Y` includes an `E-Mail` line in both quick and long personal
mail scans. Comments to the sysop can be delivered to the same mailbox.

**Why:** a caller has one inbox across conferences, and private correspondence
does not depend on which public area currently happens to be selected.

**Compatibility cost:** PCBoard had no separate inbox base. A PPE that opens
conference message files directly cannot see it; caller commands and PPL's
message API do.

## File areas carry richer metadata

File areas use an indexed database and keep metadata such as uploader and
download counts that an archive cannot provide. Descriptions can be read from
`FILE_ID.DIZ`, and `icbfile` can import and normalize old collections.

**Why:** long names, reliable metadata, indexed operations and modern archive
formats do not fit PCBoard's old DIR records. Icy Board can inspect ZIP and many
legacy archive formats without shelling out to a DOS utility.

**Compatibility cost:** external tools that edit PCBoard DIR files do not edit
the live base. Use `icbfile` or the Icy Board file APIs.

## Passwords are hashed

New boards store passwords with Argon2id or bcrypt instead of storing the
caller's password as readable text.

**Why:** plain-text password storage is not acceptable on an Internet-facing
service.

**Compatibility cost:** a PPE that reads the password field may break. A
plain-text compatibility fallback exists, but enabling it is a security
regression; updating the PPE is the preferred fix.

## Access is more expressive

Access checks can combine a PCBoard-style security level with groups and age
conditions. Security expressions are used for commands, conferences and data
records.

**Why:** a sysop can describe roles and exceptions without spending a scarce
numeric level on every combination.

**Compatibility cost:** imported level checks keep their meaning, but a new
group-based rule has no representation in PCBoard's old files.

## PPL is a maintained toolchain

The runtime targets old PPE behavior, while new source can select later
language versions and use Icy Board additions. The project includes a compiler,
decompiler, formatter, language server and tree-sitter grammar.

**Why:** PPEs can be understood and maintained in current editors instead of
depending on a closed DOS compiler and third-party reverse engineering.

**Compatibility cost:** new language features are versioned and are not
available to a source file that declares an older language version. DOS and
assembler functions remain out of scope.

## Operational improvements

- Configuration and maintenance TUIs work locally and over SSH.
- User packing, sorting and bulk maintenance can run non-interactively.
- Events can invoke current executables and scripts.
- Logs are shared and node-stamped instead of split into DOS node files.
- `icbsetup check` validates every configured path, including display-file
  variants and case mismatches.
- More door drop-file formats are available, including BBSLink integration.

These additions do not change the caller command set. They replace operational
constraints that only existed because the original board ran under DOS.

## Compatibility rule of thumb

An old asset is most likely to work unchanged when it talks to PCBoard through
documented commands, display files, macros and PPL APIs. It needs review when it
opens PCBoard's databases itself, assumes drive letters or 8.3 names, reads a
plain-text password, or invokes DOS directly.

The [migration guide](migration.md) turns that rule into a test checklist. The
[command](../compat/COMMAND_AUDIT.md) and
[options](../compat/OPTIONS_AUDIT.md) audits record exact known differences.
