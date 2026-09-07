<a id="toml-configuration-reference"></a>

# TOML configuration reference

This is the on-disk format reference for IcyBoard's configuration and persisted
TOML data. It describes exact field names, types, required fields, loading
defaults, enum values and examples. These Markdown chapters mirror the
[handbook reference](../source/configuration/index.rst) for reading directly
in the repository. Keep both versions synchronized when changing a format.

This is not the specification of PCBoard's binary configuration files. Import
those with `icbsetup` rather than renaming them or translating their labels
into TOML keys. Start with `icbsetup` to create a complete board, then use this
reference to edit generated files or implement readers and writers.
Examples marked **fragment** belong inside the indicated existing record or
table; they are not replacements for the complete configuration.

## Chapters

- [Main board configuration](board.md)
- [Component files](components.md)
- [Adding and overriding commands / CMD.LST](commands.md)
- [Network configuration](networks.md)
- [Events and event history](events.md)
- [Tool and project TOML files](tools.md)
- [User, text and runtime data](data.md)

## Which format do I need?

The configured path selects a document; its basename does not define the schema.
Setup/import may choose different names.

| File / purpose | Selected by | Specification |
| --- | --- | --- |
| `icboard.toml` | Main board configuration supplied to the tools | [Board](board.md) |
| `commands.toml` (native CMD.LST replacement) | `paths.command_file` or conference `command_file` | [Commands](commands.md) |
| `conferences.toml` | `paths.conferences` | [Components](components.md) |
| Message-area and file-directory lists | Conference `area_file` and `dir_file` | [Components](components.md); both use `[[area]]`, with different schemas |
| Language, security-level and transfer-protocol lists | `paths.language_file`, `paths.pwrd_sec_level_file`, `paths.protocol_data_file` | [Components](components.md) |
| Door, bulletin and survey lists | Conference `doors_file`, `blt_file`, `survey_file` | [Components](components.md) |
| Interactive menus (normally `*.mnu`) | Menu action or conference menu | [Menus](components.md#component-menus); native menus are TOML despite their extension |
| Accounting rates | `accounting.cfg_file` | [Components](components.md) |
| `security_tables.toml` | Beside the configured user database | [Components](components.md) and [data](data.md) |
| FTN, QWKnet and ZCONNECT configuration | `paths.ftn_file`, `paths.qwknet_file`, `paths.zconnect_file` | [Networks](networks.md) |
| Event schedule | `event.event_file` | [Events](events.md) |
| Advertisement-rule catalogs | `upload_processing.advertisement_file_rules`, `advertisement_description_rules`; also tool-selected combined catalogs | [Tools](tools.md) |
| `ppl.toml` | PPL project manifest | [Tools](tools.md) |
| `users.toml`, statistics, ICBTEXT | `paths.user_file`, `paths.statistics_file`, `paths.icbtext` | [Data](data.md); ICBTEXT has additional header/detection requirements |
| Quarantine records, event history and network checkpoints | Managed by the respective runtime subsystem | [Data](data.md), [events](events.md) and [networks](networks.md); not ordinary settings |

The group file is colon-separated text, **not TOML**. Legacy PCBoard CMD.LST,
CNAMES and other binary records are also not TOML. Repository build manifests
such as Cargo/rustup/rustfmt configuration belong to external tools; see the
[ownership inventory](tools.md).

## TOML rules that matter here

- **Keys are case-sensitive.** The command field is `keyword`, not `Keyword`.
  Historical misspellings may be part of the schema; copy the documented spelling.
- **Strings must be quoted.** Write `keyword = "USER"`, never `Keyword = USER`.
  Paths, enum names and security expressions are strings too. Booleans and
  numeric values are normally unquoted; native TOML local-time fields are
  also unquoted where documented.
- A single record uses `[table]`; repeated records use `[[record]]`. Nested
  action arrays belong to the most recently opened command record. Table
  scope continues until the next header: put root keys before headers.
- A missing value, an empty string and an empty array are different. TOML has
  no `null`. Constructor defaults and omitted output fields do not imply
  that the corresponding input field is optional.
- Unknown keys are often silently ignored. Parsing successfully does not prove
  that the board understood a setting. Enum spelling and semantic validation
  are specific to each reader.
- Ordinary board-relative paths start at the main configuration's directory,
  **not** the directory of a referenced list. Command execution and some
  subsystem paths have different rules; consult their chapters.

## Safe editing and validation

Back up the configuration before editing. Stop the board, mailer and editors
before changing mutable user data or runtime state: live processes can retain
old values or overwrite manual edits. Never delete checkpoints to fix syntax;
they may contain replay protection or pending transactions.

Check syntax with a TOML-aware editor, then check the actual format using the
owning IcyBoard editor or loader. Required fields, enum values, file references
and subsystem validation go beyond TOML syntax. Stored fields do not promise
that every associated feature is implemented; the chapters document current
runtime limitations separately.

Saving can normalize enums and omit default fields. Generic serialization
does not preserve formatting, comments or unknown keys. Do not rely on
round-tripping an undocumented extension.