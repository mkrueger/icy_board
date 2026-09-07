.. _toml-configuration-reference:

TOML configuration reference
============================

This is the on-disk format reference for IcyBoard's configuration and persisted
TOML data. It describes the readers implemented in this version of IcyBoard,
including exact field names, value types, required fields, loading defaults,
enum values and examples. It is not the specification of PCBoard's binary
configuration files. Import those with ``icbsetup`` rather than renaming them
or translating their labels into TOML keys.

Start with ``icbsetup`` to create a complete board. Use these chapters when
editing the generated files or writing tools which read or produce them.
Examples marked **fragment** must be merged into the indicated record or
table; they are not replacements for the complete configuration.

.. toctree::
   :maxdepth: 2

   board
   components
   networks
   events
   tools
   data

Which format do I need?
-----------------------

The configured path selects a document; its basename does not define the
schema. Setup/import may choose different names. These are common names and
the configuration fields which select them:

.. list-table:: File-format index
   :header-rows: 1
   :widths: 27 38 35

   * - File / purpose
     - Selected by
     - Specification
   * - ``icboard.toml``
     - Main board configuration supplied to the tools
     - :doc:`board`
   * - ``commands.toml`` (native CMD.LST replacement)
     - ``paths.command_file`` or conference ``command_file``
     - :doc:`../customizing/adding_commands`
   * - ``conferences.toml``
     - ``paths.conferences``
     - :doc:`components`
   * - Message-area and file-directory lists
     - Conference ``area_file`` and ``dir_file``
     - :doc:`components`; both use ``[[area]]``, with different schemas
   * - Language, security-level and transfer-protocol lists
     - ``paths.language_file``, ``paths.pwrd_sec_level_file``, ``paths.protocol_data_file``
     - :doc:`components`
   * - Door, bulletin and survey lists
     - Conference ``doors_file``, ``blt_file``, ``survey_file``
     - :doc:`components`
   * - Interactive menus (normally ``*.mnu``)
     - Menu action or conference menu
     - :ref:`component-menus`; native menus are TOML despite their extension
   * - Accounting rates
     - ``accounting.cfg_file``
     - :doc:`components`
   * - ``security_tables.toml``
     - Beside the configured user database
     - :doc:`components` and :doc:`data`
   * - FTN, QWKnet and ZCONNECT configuration
     - ``paths.ftn_file``, ``paths.qwknet_file``, ``paths.zconnect_file``
     - :doc:`networks`
   * - Event schedule
     - ``event.event_file``
     - :doc:`events`
   * - Advertisement-rule catalogs
     - ``upload_processing.advertisement_file_rules``, ``advertisement_description_rules``;
       also tool-selected combined catalogs
     - :doc:`tools`
   * - ``ppl.toml``
     - PPL project manifest
     - :doc:`tools`
   * - ``users.toml``, statistics, ICBTEXT
     - ``paths.user_file``, ``paths.statistics_file``, ``paths.icbtext``
     - :doc:`data`; ICBTEXT has additional header/detection requirements
   * - Quarantine records, event history and network checkpoints
     - Managed by the respective runtime subsystem
     - :doc:`data`, :doc:`events` and :doc:`networks`; not ordinary settings

The group file is a colon-separated text format, **not TOML**. Legacy PCBoard
CMD.LST, CNAMES and other binary records are also not TOML. Repository build
manifests such as Cargo/rustup/rustfmt configuration belong to their external
tools; see the ownership inventory in :doc:`tools`.

TOML rules that matter here
---------------------------

* **Keys are case-sensitive.** The command field is ``keyword``, not
  ``Keyword``. Some historical misspellings are part of the actual schema;
  copy the documented spelling rather than correcting it.
* **Strings must be quoted.** Write ``keyword = "USER"``, never
  ``Keyword = USER``. This applies to paths, enum names and security
  expressions too. Booleans and numeric values are normally unquoted;
  native TOML local-time fields are also unquoted where documented.
* A single record uses ``[table]``; repeated records use ``[[record]]``.
  Nested action arrays belong to the most recently opened command record.
  Table scope continues until the next header: root keys must precede
  headers, not be appended after a nested table.
* A missing value, an empty string and an empty array are different.
  There is no TOML ``null``. Rust constructor defaults and omitted output
  fields do not imply that the corresponding input field is optional.
* Unknown keys are often silently ignored. A successful TOML parse is not
  proof that the board understood a setting. Enum spelling and semantic
  validation are specific to each reader.
* Ordinary board-relative paths start at the main board configuration's
  directory, **not** the directory of a referenced list. Command execution
  and some subsystem paths have their own rules; see the individual pages.

Safe editing and validation
---------------------------

Back up the configuration before editing. Stop the board, mailer and editors
before changing mutable user data or runtime state; live processes can retain
old values or overwrite manual edits. Never delete checkpoints to fix a
syntax error: they may contain replay protection or pending transactions.

Use a TOML-aware editor to check syntax, then use the owning IcyBoard editor
or loader to check the actual format. Required fields, enum values, file
references and subsystem validation are additional to TOML syntax. The
reference explicitly separates format requirements from current runtime
limitations; stored fields are not a promise that every associated feature
is implemented.

Saving through the tools can normalize enum values and omit default fields.
Generic serialization does not preserve formatting, comments or unknown
keys. Do not rely on round-tripping an undocumented extension.