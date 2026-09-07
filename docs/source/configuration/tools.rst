Tool and project TOML files
===========================

This page describes the tool-owned formats implemented by this checkout. Board,
component, network and scheduled-event configuration have separate specifications.
The file's reader, not its extension, determines its format. Keys and enum strings
below are case-sensitive. A Rust ``Default`` implementation does not make fields
optional in TOML: only an optional field or a Serde default does that.

Inventory and ownership
-----------------------

* ``ppl.toml`` is the PPL compiler/language-server project manifest.
* ``upload_ad_files.toml``, ``upload_ad_descriptions.toml``, the older combined
  ``upload_ad_rules.toml``, and ``bbstros.toml`` use the same dizbase rule-catalog
  schema. Their names are conventions, not format identifiers.
* ``icbfile`` reads component file-directory lists and advertisement catalogs.
  It has no separate ``icbfile.toml`` settings loader.
* ``icbmailer`` loads a board and its FTN, QWKnet or ZCONNECT configuration.
  It has no separate ``icbmailer.toml`` settings loader.
* ``mkicbmnu`` edits TOML menu documents, normally named ``*.mnu``. It does not
  persist a separate editor-settings TOML document. ``mkicbtxt`` edits the
  specially detected ICBTEXT format described in the data-file reference.
* Cargo, rustup, rustfmt, Python packaging and localization TOML in this repository
  belong to those external tools; they are not additional board formats.
* Audit-example manifests and baselines are generated research data, not live
  configuration. Their inventory appears at the end of this page.

PPL project manifest: ppl.toml
--------------------------------

``Workspace::load`` reads UTF-8 TOML directly. ``Workspace::save`` writes ordinary
TOML without an IcyBoard header. The root contains one required ``[package]``
table, optional ``[compiler]``, ``[data]`` and ``[formatting]`` tables, and an
optional ``[dependencies]`` map (empty by default). The filename, discovered
source files and dependency-resolution caches are runtime-only fields, not keys.

The normal source tree is ``src/`` relative to the manifest. Discovery is recursive,
accepts lowercase ``.pps`` files and extensionless files, and orders ``main``
stems first. There is no manifest ``sources``, ``include`` or ``exclude`` key.

Package
~~~~~~~

``[package]`` has exactly these supported fields:

.. list-table:: Package fields
   :header-rows: 1
   :widths: 22 25 53

   * - Key
     - TOML type / omission
     - Meaning
   * - ``name``
     - String; required
     - Output package name; the compiler uses it to form the PPE filename.
   * - ``version``
     - String; required
     - Semantic version parsed by ``semver::Version``, for example ``"0.1.0"``.
       A bare number or abbreviated ``"1.0"`` is not a semantic version here.
   * - ``runtime``
     - Unsigned 16-bit integer; optional
     - PPE runtime target. Effective default is currently 400.
   * - ``authors``
     - Array of strings; optional
     - Author metadata. Absence and an empty array are both usable.

The supported runtime targets are ``100``, ``200``, ``300``, ``310``, ``320``,
``330``, ``340`` and ``400``. The manifest deserializer only checks the integer
type; the supported-version list describes compiler support, not a TOML enum.
The default workspace constructed in code has an empty name and version
``0.1.0``; loading a manifest still requires both keys.

Output is under ``target/pcboard_15.0`` for 100, ``pcboard_15.10`` for 200,
``pcboard_15.20`` for 300, ``pcboard_15.21`` for 310, ``pcboard_15.22`` for 320,
``pcboard_15.30`` for 330 and ``pcboard_15.40`` for 340. Other target numbers use
``target/icboard``. These paths are relative to the manifest directory.

Compiler
~~~~~~~~

Both members of ``[compiler]`` are optional:

* ``language_version``: unsigned 16-bit integer. Supported language versions are
  ``100``, ``200``, ``300``, ``310``, ``320``, ``330``, ``340``, ``350``, ``400``.
  Language 350 is not an additional PPE runtime target. In the workspace API,
  omission means the lesser of the effective runtime and the latest language
  version (currently 400).
* ``defines``: array of strings, such as ``["DEBUG", "LIMIT=10"]``. These seed
  the preprocessor; they are not a TOML name/value table. Omission supplies no
  additional manifest definitions. The CLI's semicolon-separated ``--defines``
  option replaces this array when supplied.

Define names must begin with an ASCII letter or underscore and continue with
ASCII letters, digits or underscores. A bare name means boolean true; a
``NAME=value`` definition must evaluate to a preprocessor boolean or integer,
not an arbitrary string value.

``pplc`` also considers command-line options, source ``$LANGVERSION`` declarations
and ``PPL_LANG_VERSION``. Explicit CLI runtime/language choices override the
manifest; a source language declaration must agree with an explicit selection.
The environment supplies a language version only when none is explicit. Do not
interpret the workspace API's fallback as overriding source or CLI selection.

Data files
~~~~~~~~~~

``[data]`` contains only the optional string arrays ``text_files``, ``art_files``
and ``binary_files``. These are literal paths, not glob patterns, relative to the
manifest. Each is copied to the corresponding relative location under the target
directory, creating parent directories. Use relative paths without ``..``:
the copying code joins paths directly and is not a sandbox.

* ``text_files``: encoding-detected input, written as CP437 for runtime <= 340
  and UTF-8 for newer runtimes.
* ``art_files``: the same text conversion, except a lowercase ``.icy`` extension
  is decoded as Icy art and exported as PCBoard art with a ``.pcb`` extension.
* ``binary_files``: bytes copied unchanged.

Dependencies' source libraries are resolved transitively; their ``[data]`` arrays
are not automatically merged into the root package's data-copy phase.

Formatting
~~~~~~~~~~

``[formatting]`` is supported even though the workspace field is private in Rust.
All four keys have deserialization defaults:

.. list-table:: Formatting options
   :header-rows: 1

   * - Key
     - Type
     - Default
     - Meaning
   * - ``space_around_binop``
     - Boolean
     - ``true``
     - Put spaces around binary operators.
   * - ``use_tabs``
     - Boolean
     - ``false``
     - Use tabs rather than spaces for indentation.
   * - ``indent_size``
     - Nonnegative platform-sized integer
     - ``4``
     - Indentation width.
   * - ``max_blank_lines``
     - Nonnegative platform-sized integer
     - ``2``
     - Limit consecutive blank lines between statements; the formatter preserves
       paragraph separation rather than joining paragraphs together.

There is no ``binop_separator`` option: that proposed enum is commented out.

Dependencies
~~~~~~~~~~~~

Each key under ``[dependencies]`` names a source-library import. Its value is a
table (inline or ``[dependencies.name]``), not a version string. Every dependency
allows only ``path``, ``git``, ``rev``, ``branch`` and ``tag``; unknown fields are
rejected. Each of these five values is an optional string/path at deserialization,
but resolution imposes the following constraints:

* Exactly one of ``path`` and ``git`` must be present.
* ``path`` identifies a directory containing ``ppl.toml``, relative to the
  manifest that declares it. A path dependency cannot use ``rev``, ``branch``
  or ``tag``.
* ``git`` is a Git repository URL/path with ``ppl.toml`` at its repository root.
  At most one of ``rev`` (commit/ref), ``branch`` or ``tag`` may be supplied.
  Omission selects the repository's default branch.
* Git URL/ref strings starting with ``-`` are rejected. Resolution invokes Git
  with terminal prompting disabled.
* Git checkouts live under the root package's
  ``target/ppl-dependencies/git/<sanitized-name>-<digest>`` directory. Moving
  branches/default HEAD are refreshed when possible, with cached-checkout
  fallback on a fetch failure. Revision/tag selections reuse their cache.

There is no registry dependency, version constraint, feature list, subdirectory
selector or serialized lockfile in this schema. The dependency name provides an
import mapping; internal module names and canonical-path visitation state are
not serialized. Unlike dependency tables, the outer workspace/package/compiler/
data/formatting structs do not reject unknown keys, so a misspelling there may
be silently ignored.

Example (the local library directory must exist when building):

.. code-block:: toml

   [package]
   name = "welcome"
   version = "0.1.0"
   authors = ["Example Sysop"]
   runtime = 400

   [compiler]
   language_version = 400
   defines = ["DEBUG", "LIMIT=10"]

   [data]
   text_files = ["data/help.txt"]
   art_files = ["data/logo.icy"]
   binary_files = ["data/table.bin"]

   [formatting]
   space_around_binop = true
   use_tabs = false
   indent_size = 4
   max_blank_lines = 2

   [dependencies]
   helpers = { path = "../helpers" }

Advertisement rule catalogs (dizbase)
---------------------------------------

The root ``FingerprintData`` document has three optional arrays of tables:
``[[fingerprint]]``, ``[[description_rule]]`` and ``[[text_member_rule]]``.
All default to empty. There is no outer ``[rules]`` wrapper, schema-version key,
include directive or nested rule-file reference. ``#`` comments are ordinary
TOML comments and are not retained by serialization.

Board upload processing uses a split loader:

* ``upload_processing.advertisement_file_rules`` selects fingerprints and
  whole-member text rules (default filename ``upload_ad_files.toml``).
* ``upload_processing.advertisement_description_rules`` selects description
  rules (default filename ``upload_ad_descriptions.toml``).
* An empty configured path disables that category. Paths are resolved using the
  board's path handling, not relative to the catalog. An empty catalog is valid.
* Nonselected categories in a file are ignored, except that a file containing
  only the wrong recognized category is an error. The same combined catalog
  can be selected for both paths.
* Selected regexes/templates are validated before processing. A missing file,
  TOML error or invalid selected rule fails loading with category/path context.

The older combined ``FingerprintData::load`` API, used by ``icbfile --fingerprints``,
selects all three categories. It rejects invalid text-member templates, but
legacy fingerprint regex errors and description-cleaner errors are logged by its
indexing phase rather than consistently returned as load errors. Use the split
board loader's stricter behavior as a reason to validate catalogs before rollout.

Fingerprints
~~~~~~~~~~~~

All fields in ``[[fingerprint]]`` are optional; empty strings/lists and numeric
zeroes are omitted when written:

.. list-table:: Fingerprint fields
   :header-rows: 1
   :widths: 19 27 54

   * - Key
     - Type / default
     - Matching behavior
   * - ``name``
     - String / ``""``
     - Human-readable provenance/label, not an exact filename constraint.
   * - ``pattern``
     - String / ``""``
     - Rust regex applied to the member name/path. Used only if ``keywords``
       is nonempty. The empty regex matches every name.
   * - ``crc``
     - Unsigned 32-bit integer / ``0``
     - Legacy CRC32 of raw member bytes, active only together with nonzero
       ``file_size`` and an empty ``sha256``.
   * - ``file_size``
     - Unsigned 64-bit integer / ``0``
     - Exact raw byte length. Zero disables this record's checksum match.
   * - ``sha256``
     - String / ``""``
     - Raw-content SHA-256 hex digest, normalized to ASCII lowercase for lookup.
       A nonempty digest plus nonzero size supersedes this record's CRC branch.
   * - ``keywords``
     - Array of strings / ``[]``
     - Every keyword's UTF-8 bytes must occur literally in the raw member,
       case-sensitively. Not regexes and not normalized display text.

Checksum matching ignores filenames. Regex/keyword matching is a separate OR
branch: a matching pattern and all keywords suffice even if this record's hash
does not match. Patterns are not implicitly anchored or case-insensitive; add
``^``, ``$`` or ``(?i)`` deliberately. The digest is not syntactically validated
as a 64-digit hash during loading; malformed strings simply fail to match.

These legacy records have no ``id`` or ``action`` field. A match authorizes
member removal when advertisement removal is enabled. Do not attach
``action = "report_only"`` to a fingerprint and expect it to protect the member:
unknown fields here are ignored. Prefer whole-member templates for reviewable
text rules, or generate exact fingerprints from verified samples.

Description blocks
~~~~~~~~~~~~~~~~~~

``[[description_rule]]`` recognizes a leading/trailing block inside a canonical
description, not an arbitrary substring anywhere in a file:

.. list-table:: Description-rule fields
   :header-rows: 1
   :widths: 22 28 50

   * - Key
     - Type / omission
     - Meaning
   * - ``id``
     - String; required
     - Identifier included in findings. This rule type does not enforce
       nonempty/unique IDs.
   * - ``member_pattern``
     - Regex string; default shown below
     - Select archive member names/paths.
   * - ``position``
     - String; default ``"suffix"``
     - Exactly ``"prefix"`` or ``"suffix"``.
   * - ``inline_start``
     - Optional string; absent by default
     - Literal ASCII marker for a suffix beginning partway through a raw line.
       Only a nonempty ASCII marker on a suffix rule is used; matching of the
       marker is ASCII case-insensitive, then line patterns are validated.
   * - ``lines``
     - Array of regex strings; default ``[]``
     - Ordered patterns applied to normalized nonblank lines. Regexes are not
       implicitly anchored; use ``^...$`` for whole-line matching.
   * - ``literal_lines``
     - Array of strings; default ``[]``
     - Ordered whole-line literal matches, normalized like the input.
   * - ``action``
     - String; default ``"report_only"``
     - ``"report_only"``, ``"review"`` or ``"auto_clean"``.

The default member regex, expressed as a TOML literal string, is:

.. code-block:: toml

   member_pattern = '(?i)(^|[/\\])(desc\.sdi|file_id\.(diz|ans|pcb))$'

Exactly one of ``lines`` and ``literal_lines`` must be nonempty. Literal entries
must normalize to nonempty text and must not contain CR, LF or DOS EOF. Matching
normalizes encoding, strips ANSI CSI and PCBoard ``@Xhh`` colors, collapses
whitespace and lowercases text; blank normalized lines do not participate.
Literal patterns receive the same normalization. Regex patterns themselves are
not lowercased. ``inline_start`` is searched in original bytes; keep it ASCII.

``report_only`` records a finding without removing it; ``review`` requires
review; ``auto_clean`` removes the matched prefix/suffix while retaining original
bytes of the kept description. Multiple matching rules/positions cause review
rather than a guessed deletion. The repacker makes at most eight cleanup passes;
remaining matches or a description emptied by cleaning require review. That pass
count is not a catalog setting.

Whole-member text templates
~~~~~~~~~~~~~~~~~~~~~~~~~~~

``[[text_member_rule]]`` recognizes an entire small text member regardless of its
filename. Canonical ``DESC.SDI`` and ``FILE_ID.DIZ``, ``FILE_ID.ANS``,
``FILE_ID.PCB`` descriptions are excluded; use description rules for those.

.. list-table:: Whole-member fields
   :header-rows: 1

   * - Key
     - Type / omission
     - Validation
   * - ``id``
     - String; required
     - Must be nonblank and unique among text-member rules.
   * - ``action``
     - String; default ``"report_only"``
     - ``"report_only"``, ``"review"`` or ``"auto_clean"``.
   * - ``max_bytes``
     - Nonnegative platform-sized integer; default ``16384``
     - Raw member byte ceiling; must be 1 through 131072 inclusive.
   * - ``lines``
     - Array of one-key tables; required
     - 1 through 512 entries. Each is ``{ literal = "..." }`` or
       ``{ regex = '...' }``. These are tagged enum values, not plain strings.

Unknown fields on a text-member rule are rejected. There is no ``name``,
``member_pattern``, encoding override, hash selector or position setting.
Templates need at least twelve alphabetic characters across their normalized
literal entries; regex-only rules are rejected. Literal entries cannot contain
control characters. Regex entries use Rust regex syntax and are implicitly
anchored at both ends.

Input accepts ASCII, UTF-8 (including BOM), or inferred CP437. SGR color escapes
and ``@Xhh`` colors are removed; whitespace is collapsed and text lowercased.
CR/LF forms are normalized; leading/trailing blank lines are ignored, but internal
blank lines still count. Every remaining line must match in order, with no extra
material. Executable signatures, unexpected controls, cursor/erase escapes,
oversized inputs and nonwhitespace after DOS EOF are rejected as candidates.
Multiple matches or a ``review`` action require review; ``auto_clean`` permits
whole-member deletion; ``report_only`` does not.

Example combined catalog
~~~~~~~~~~~~~~~~~~~~~~~~

Start uncertain text rules in reporting mode. This sample intentionally contains
no speculative checksum fingerprint:

.. code-block:: toml

   [[description_rule]]
   id = "example-footer"
   position = "suffix"
   literal_lines = ["Downloaded from Example Board"]
   action = "report_only"

   [[text_member_rule]]
   id = "example-board-card"
   action = "report_only"
   max_bytes = 16384
   lines = [
       { literal = "Welcome to Example Board" },
       { regex = 'uploaded [0-9]{2}-[0-9]{2}-[0-9]{4}' },
   ]

Archive comments are not another rule kind
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

There is no ``[[comment_rule]]`` or ``clean_archive_comments`` setting in the
current catalog. ZIP archive-comment handling is independent of advertisement
matching and is selected in the board's ``[upload_processing]`` table:

* ``archive_comment_mode``: ``"preserve"`` (default), ``"remove"`` or
  ``"replace"``.
* ``replacement_archive_comment``: string, default ``""``; used only by
  ``"replace"``. The board passes its UTF-8 bytes unchanged. An empty replacement
  clears the comment.

Preservation keeps original raw ZIP comment bytes. Other archive formats do not
expose comments through this repacking interface, so conversion does not transfer
their comments. The library rejects replacement ZIP comments above 65535 bytes.
``RepackOptions`` is an in-memory Rust structure, not a separate TOML config file.
Likewise ``advertisement_file`` selects a static member or trusted PPE generator,
not another TOML rules document.

Other installed tools
---------------------

icbfile
~~~~~~~

The file-directory-list argument is decoded by ``DirectoryList::load`` using the
component ``[[area]]`` schema, including when named ``file_areas.toml``. A plain
directory argument opens its file base instead. Area selection by number is
zero-based; names are matched case-insensitively. Relative directory/metadata
paths are resolved against the list's parent; empty metadata paths use ``dir``
inside the directory. The file-base database itself is not TOML.

The ``fingerprints`` subcommand writes a catalog, defaulting to ``bbstros.toml``;
each scanned file produces ``name``, ``crc``, ``file_size`` and ``sha256``.
``repack --fingerprints`` reads the combined catalog above. Compression, dry-run,
case handling and archive resource limits are CLI options, not saved tool keys.
Import/export formats such as PCBoard DIR and FILES.BBS are not TOML either.

icbmailer
~~~~~~~~~

The CLI argument named ``config`` is a board file passed to ``IcyBoard::load``.
The FTN path then uses the board's loaded FTN configuration; QWK operations use
``paths.qwknet_file`` and ZCONNECT operations use ``paths.zconnect_file``.
Network link/hub fields belong to those network documents, not to a second
mailer-settings schema. Outbound ``scan.toml``, per-hub state and FREQ usage are
machine-managed checkpoints, described in the data-file reference.

mkicbmnu and mkicbtxt
~~~~~~~~~~~~~~~~~~~~~~~

``mkicbmnu`` forces its input extension to ``.mnu`` and reads/writes the component
``Menu`` TOML schema. It locates a board configuration in a parent directory for
context. ``--create`` and ``--full-screen`` are command-line options, not
serialized preferences. There is no standalone settings manifest for either
editor. ``mkicbtxt`` uses ICBTEXT's custom loader/writer, not ``IcyBoardSerializer``.

Repository TOML owned by external tools
-----------------------------------------

These files configure development tools only. Their complete schemas are defined
by those tools, not by IcyBoard's Rust board serializers:

.. list-table:: Development-tool inventory
   :header-rows: 1
   :widths: 34 66

   * - File family
     - Owner and contents in this checkout
   * - Root/crate/fuzz ``Cargo.toml``
     - Cargo package/workspace metadata, features, dependencies, targets and
       profiles. Not related to PPL ``[dependencies]`` despite similar syntax.
   * - ``.cargo/config.toml``
     - Cargo ``[env]`` sets ``RUST_TEST_THREADS = { value = "4", force = false }``;
       ``[alias]`` defines ``test-low = ["test", "--jobs", "4"]``.
   * - ``rust-toolchain.toml``
     - rustup ``[toolchain]``: string ``channel = "1.96.0"`` and string-array
       ``components = ["rustfmt", "clippy"]``.
   * - ``rustfmt.toml``
     - rustfmt root keys ``max_width = 160``, ``use_small_heuristics = "Default"``
       and ``use_field_init_shorthand = true``. Not PPL formatting options.
   * - ``crates/ppl-lsp/i18n.toml`` and ``crates/icy_board_tui/i18n.toml``
     - Localization tooling: root ``fallback_language = "en"`` and
       ``[fluent] assets_dir = "i18n"``. Fluent translation assets are not TOML.
   * - ``crates/tree-sitter-ppl/pyproject.toml``
     - Python packaging: ``[build-system]`` requirements/backend; ``[project]``
       name, description, version, keywords, classifiers, authors,
       requires-python, license.text and readme; ``[project.urls] Homepage``;
       ``[project.optional-dependencies] core``; ``[tool.cibuildwheel]`` build
       selector and build-frontend. These are setuptools/PEP/cibuildwheel keys.

Example/audit-generated TOML
------------------------------

The workspace's TOML reader/writer inventory also finds the following research
artifacts. They are not discovered by a live board, nor stable user configuration
APIs. Do not copy an arbitrary audit manifest into a board config path.
All fields listed here are required on deserialization unless noted.

* ``ad_corpus_rules`` writes/reads ``manifest.toml``: ``source`` string,
  ``archives`` nonnegative platform-sized count, ``expanded_bytes`` unsigned
  64-bit byte count, ``sources`` string-to-string map, ``errors`` string array,
  and ``samples`` array. Each sample has ``origin``, ``sha256`` and ``kind``
  strings and ``size`` nonnegative platform-sized byte count. The ``kind`` field
  is a string, not a Serde enum.
* ``pcboard_diz_audit`` writes/reads ``audit.toml``: ``source`` string,
  ``archives`` and ``nested_archives`` nonnegative platform-sized counts,
  ``sources`` string map, ``errors`` string array, and ``diz`` array with
  required ``origin`` and ``hash`` strings in every element.
* ``upload_corpus`` writes/reads ``baseline.toml``: ``source`` path string,
  ``areas`` path-string array, and ``files`` and ``excluded_root_files`` arrays.
  Every element of either array has ``relative`` and ``area`` path strings,
  ``bytes`` unsigned 64-bit byte count and ``sha256`` string.
* Generated ``upload_ad_files.toml`` and ``upload_ad_descriptions.toml`` remain
  catalogs of the rule kinds specified above; generation does not create a new
  board schema. ``rules.used.toml`` is a saved rule snapshot. Example programs
  also produce component ``directories.toml`` and modify/copy board and
  conference TOML using their existing schemas.

This inventory was checked against direct ``toml::from_str``/``to_string``/
``to_string_pretty``/``Value`` conversions and implementations of
``IcyBoardSerializer``, not inferred solely from files ending in ``.toml``.