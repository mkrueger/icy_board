.. _component-configuration:

Component configuration files
=============================

This reference describes the TOML files for conferences, commands, menus,
message areas, file directories, languages, security levels, transfer
protocols, doors, bulletins, surveys, accounting rates, and user-maintenance
security tables. Board-wide settings and network configuration are separate
from these component files.

The schemas below follow the Rust serializers, not the labels in setup or
the legacy PCBoard formats. In particular, ``[[area]]`` is used by **both**
message-area lists and file-directory lists, in separate files.

Reading the schemas
-------------------

* Keys and ordinary enum strings are case-sensitive. Retain the historical
  spellings ``bullettin``, ``securiy_level``, ``sheme_code``, and
  ``lighbar_display``; corrected English spellings are not substitutes.
* **Required** means that omitting the key fails deserialization, even if
  Rust's ``Default`` constructor can create the corresponding field. An
  optional field's stated default applies when the key is absent from TOML.
  ``skip_serializing_if`` alone does not make a field optional on input.
* A path is a TOML string, not a table. Prefer board-root-relative paths such
  as ``"conferences/main/area.toml"`` or absolute paths. Component-list paths
  are resolved against the board root, not the directory containing the list.
  A required path may be ``""`` for deserialization, but that does not create
  a usable file or directory.
* ``u8``, ``u16``, ``u32``, and ``u64`` mean nonnegative TOML integers fitting
  the corresponding Rust unsigned type; ``i32`` is a signed 32-bit integer.
  TOML integer representation may further restrict the largest ``u64`` values.
  ``f64`` means a floating-point number; examples use ``0.0``. Booleans are
  unquoted ``true`` or ``false``. Strings, characters, paths, enums and
  security expressions are quoted.
* Arrays of records use ``[[record]]``. An explicitly empty list is
  ``record = []`` at the root. Root keys must appear before any table header
  if they are to remain root keys. There is no ``[conferences]`` or
  ``[directories]`` wrapper around a list.
* These structs do not generally reject unknown keys. A misspelled optional
  key may therefore appear to load while having no effect. Use the exact
  keys below, not successful TOML parsing alone, as a schema check.
* Examples marked **standalone** deserialize as the named component. They
  do not provision the referenced art, message bases, directories, executables,
  or service credentials. Examples marked **fragment** belong inside another
  configuration and are not complete board configurations.

File selection and root keys
----------------------------

.. list-table:: Component files
   :header-rows: 1
   :widths: 22 43 35

   * - Component
     - Selected by
     - Root shape / empty-file semantics
   * - Conferences
     - Board ``paths.conferences``
     - ``[[conference]]``; absent list means empty
   * - Global commands
     - Board ``paths.command_file``
     - ``[[command]]``; list required (``command = []`` is valid)
   * - Conference commands
     - Conference ``command_file``
     - Same ``[[command]]`` schema
   * - Interactive menu
     - A ``Menu`` action, or conference menu with a ``.mnu`` extension
     - One root record; ``title`` required; nested ``[[commands]]``
   * - Message areas
     - Conference ``area_file``
     - ``[[area]]``; list required
   * - File directories
     - Conference ``dir_file``
     - ``[[area]]``; list required, **not** ``[[directory]]``
   * - Languages
     - Board ``paths.language_file``
     - Required ``date_formats``; optional ``[[language]]``
   * - Security levels
     - Board ``paths.pwrd_sec_level_file``
     - ``[[level]]``; absent list means empty
   * - Transfer protocols
     - Board ``paths.protocol_data_file``
     - ``[[protocol]]``; absent list means empty
   * - Doors
     - Conference ``doors_file``
     - Both ``account`` and ``door`` lists required
   * - Bulletins
     - Conference ``blt_file``
     - ``[[bullettin]]``; list required
   * - Surveys
     - Conference ``survey_file``
     - ``[[survey]]``; list required
   * - Accounting rates
     - Board ``accounting.cfg_file``
     - One root record, all 15 fields required
   * - Maintenance security tables
     - ``security_tables.toml`` beside the configured user file
     - Four optional arrays: ``file_ratio``, ``byte_ratio``, ``uploads``, ``downloads``

An empty list is not the same as a missing file. Board loading requires the
configured global command, language, protocol, security-level and conference
files. Conference sublists are loaded if the configured path is a file;
parse failures are logged and leave the corresponding in-memory list absent
or empty. Accounting load failure is logged when accounting is enabled.

.. _component-security-expressions:

Security expressions and passwords
----------------------------------

Security-expression fields in the component records below use
``DisplayFromStr``: the TOML value is a **string**, for example
``required_security = "20"``, not an integer or a nested expression table.
Omission defaults to the integer expression ``0``; an explicitly empty string
also parses as ``0``. An integer result is interpreted as a minimum security
level (keep constants in 0--255); a boolean result grants or denies access.
Thus ``"0"`` and ``"true"`` permit access, while ``"false"`` denies it.

The writer omits expressions equal to boolean ``true``. Despite its name,
``SecurityExpression::is_empty`` does **not** identify the default integer
``0``, which may be written explicitly on save.

Supported practical forms include:

* ``"U_SEC() >= 20"``: current security level.
* ``"(U_SEC() >= 20) & (U_AGE() >= 18)"``: minimum security and age.
* ``'U_GROUP("sysop") | (U_SEC() >= 100)'``: exact session group membership
  or minimum level. TOML literal quotes avoid escaping the inner quotes.
* ``"TIME_LEFT() > 5"``: remaining session minutes.
* ``"DOW() < 5"``: day-of-week test (Sunday is 0, Saturday is 6).
* ``"!false"`` and parenthesized expressions.

Function names are evaluated without regard to case. The recognized functions
are ``U_SEC()``, ``U_AGE()``, ``U_GROUP(string)``, ``TIME()``,
``TIME_LEFT()``, and ``DOW()``. ``TIME()`` uses local time. String literals
inside expressions are limited by the lexer to nonempty ASCII letters,
digits and underscores. Group names are compared as stored, including case.

.. warning::

   This is not a general-purpose expression language. The current parser
   implements ``<``, ``<=``, ``>``, ``>=``, ``!``, ``&``, ``|``, and parentheses.
   Although equality operators exist in the internal enum, the parser does
   not consume ``==`` or ``!=`` as comparisons. It also does not reliably
   reject unconsumed trailing tokens. Do not use equality, arithmetic,
   arbitrary function names, or malformed argument lists for access control.
   Parenthesize mixed boolean expressions: ``&`` and ``|`` share a
   right-recursive parsing level rather than conventional precedence.

   The lexer accepts time literals as ``HH:MM``, but the writer emits
   ``HH:MM:SS``. Time-literal expressions are not reliably round-trip-safe;
   do not use them in configuration that will be saved by setup. The
   component fields use the fallible ``FromStr`` adapter, but the security
   type's direct deserializer elsewhere can replace a parse failure with
   ``0``. A file that loads is not proof that its security policy is correct.

Conference and file-directory ``password`` fields use the custom ``Password``
string format. The loader accepts ``password = ""`` or a legacy plain string,
a string whose *contents* have surrounding double quotes, a ``bcrypt:``
prefixed hash, or a hash beginning ``$argon2``. The writer encloses plaintext
in an extra pair of double quotes inside the TOML string, as shown below.
No ``[password.PlainText]`` table is used. Protected PPE passwords use the
plaintext storage representation if
serialized. Door and security-level passwords are ordinary strings instead.

Password-field fragment (the exact saved plaintext representation):

.. code-block:: toml

   password = '"secret"'

Use setup-generated password hashes for nonempty secrets where supported;
these files should not be publicly readable.

.. _component-conferences:

Conferences
-----------

Each ``[[conference]]`` record represents one conference. Its position is its
zero-based conference number; the first is Main Board (0). ``name`` is a
required string. The following are **all** remaining serialized fields.

.. list-table:: Conference fields
   :header-rows: 1
   :widths: 40 18 42

   * - Keys
     - Type / missing default
     - Purpose
   * - ``is_public``, ``is_read_only``, ``echo_mail_in_conference``
     - bool / false
     - Public access, read-only conference and echo-mail flags
   * - ``password``
     - Password string / empty
     - Conference password
   * - ``required_security``
     - expression / ``"0"``
     - Conference access requirement
   * - ``sec_attachments``, ``sec_write_message``, ``sec_request_rr``, ``sec_carbon_copy``
     - expression / ``"0"``
     - Attachment, posting, return-receipt and carbon-copy access
   * - ``carbon_list_limit``
     - u8 / 0
     - Carbon-copy recipient limit
   * - ``auto_rejoin``, ``allow_view_conf_members``
     - bool / false
     - Rejoin and membership-view flags
   * - ``private_uploads``, ``private_msgs``, ``disallow_private_msgs``
     - bool / false
     - Private upload/message policy flags
   * - ``allow_aliases``, ``show_intro_in_scan``
     - bool / false
     - Alias and intro-in-scan flags
   * - ``add_conference_security``
     - i32 / 0
     - Conference security adjustment
   * - ``add_conference_time``
     - u16 / 0
     - Additional conference time
   * - ``use_main_commands``
     - bool / false
     - Stored compatibility flag; current command lookup does not consult it
   * - ``record_origin``, ``prompt_for_routing``, ``long_to_names``, ``force_echomail``
     - bool / false
     - Message origin, routing, recipient-name and echo-mail flags
   * - ``conference_type``
     - enum / ``"Normal"``
     - See exact values below
   * - ``users_menu``, ``sysop_menu``, ``news_file``, ``attachment_location``
     - path / required
     - User/sysop menus, conference news and attachment storage
   * - ``pub_upload_location``, ``private_upload_location``
     - path / required
     - Public and private upload directories
   * - ``pub_upload_metadata``, ``private_upload_metadata``
     - path / empty
     - Upload metadata/index paths
   * - ``pub_upload_sort``, ``private_upload_sort``
     - u8 / 0
     - Legacy numeric upload sort codes, not ``SortOrder`` strings
   * - ``command_file``, ``intro_file``
     - path / required
     - Conference command list and introduction display file
   * - ``doors_menu``, ``doors_file``
     - path / required
     - Door menu and door-list TOML
   * - ``blt_menu``, ``blt_file``
     - path / required
     - Bulletin menu and bulletin-list TOML
   * - ``survey_menu``, ``survey_file``
     - path / required
     - Survey menu and survey-list TOML
   * - ``dir_menu``, ``dir_file``
     - path / required
     - File-directory menu and directory-list TOML
   * - ``area_menu``, ``area_file``
     - path / required
     - Message-area menu and area-list TOML
   * - ``charge_time``, ``charge_msg_read``, ``charge_msg_write``
     - f64 / 0.0
     - Conference time/read/write charges

``conference_type`` values are exactly ``Normal``, ``InternetEmail``,
``InternetUsenet``, ``UsnetModeratedNewsgroup``, ``UsnetPublicNewsgroup``,
and ``FidoConference``. The ``Usnet`` spelling is intentional in the format.
Legacy conversion maps these to numbers 0 through 5 respectively. The upload
sort fields accept any ``u8`` at the serde layer; the traditional codes are
0 unsorted, 1 filename ascending, 2 date ascending, 3 filename descending,
and 4 date descending. Their presence is not a guarantee that every imported
conference policy is implemented in every runtime operation.

The in-memory ``number``, ``valid``, ``commands``, ``areas``, ``directories``,
``doors``, ``bulletins``, and ``surveys`` fields are skipped, not nested TOML
tables. Configure their corresponding external list paths instead.

Standalone conference file, including every required field:

.. code-block:: toml

   [[conference]]
   name = "Main Board"
   is_public = true
   auto_rejoin = true
   use_main_commands = true
   users_menu = "conferences/main/brdm"
   sysop_menu = "conferences/main/brds"
   news_file = "conferences/main/news"
   attachment_location = "conferences/main/attach"
   pub_upload_location = "conferences/main/upload"
   private_upload_location = ""
   command_file = ""
   intro_file = ""
   doors_menu = "conferences/main/door"
   doors_file = "conferences/main/door.toml"
   blt_menu = "conferences/main/blt"
   blt_file = "conferences/main/blt.toml"
   survey_menu = "conferences/main/survey"
   survey_file = "conferences/main/survey.toml"
   dir_menu = "conferences/main/dir"
   dir_file = "conferences/main/dir.toml"
   area_menu = "conferences/main/area"
   area_file = "conferences/main/area.toml"

Setup creates a Main Board with ``is_public``, ``auto_rejoin``, and
``use_main_commands`` true and supplies art/list paths. Those are **setup
choices**, not the missing-field defaults. Setup also creates a General
message area and file directory, two bulletins, two surveys, and an empty
door list. A bare ``Conference::default()`` does none of that provisioning.

Commands
--------

Global and conference command lists have the required root array
``[[command]]``, with optional ``[[command.actions]]`` records beneath each
command. Menu files instead use ``[[commands]]`` and
``[[commands.actions]]``. The command fields, action fields, complete enum
vocabulary, security behavior, legacy import and working examples are
specified in :ref:`adding-commands`.

.. _component-menus:

Interactive menus
-----------------

A menu is a single root ``Menu`` record, normally stored with a ``.mnu``
extension even though the native content is TOML. A legacy PCBoard MNU text
file must be imported, not passed directly to the TOML loader.

.. list-table:: Menu schema
   :header-rows: 1
   :widths: 25 30 45

   * - Key
     - Type / missing default
     - Meaning
   * - ``title``
     - string / required
     - Menu title
   * - ``display_file``
     - path / empty
     - Display art behind the menu
   * - ``help_file``
     - path / empty
     - Stored help-file setting
   * - ``force_display``
     - bool / false
     - Stored force-display setting
   * - ``menu_type``
     - enum / ``"Hotkey"``
     - ``Hotkey``, ``Lightbar``, or ``Command``
   * - ``pass_through``
     - bool / false
     - Stored pass-through setting
   * - ``prompt``
     - string / empty
     - Prompt printed before input
   * - ``prompts``
     - array of two-string arrays / empty
     - Imported alternate prompt selector/text pairs
   * - ``commands``
     - array of Command records / empty
     - Full schema in :ref:`adding-commands`

For example, ``prompts = [["expert", "Command? "]]`` has the correct tuple
shape; it is not a table keyed by language. Currently the runner uses
``prompt``, not ``prompts``, and does not implement ``help_file``,
``force_display`` or ``pass_through`` as controls. It displays the art each
cycle and falls back to board command lookup even when ``pass_through`` is
false. ``Hotkey`` returns after one accepted character; other modes wait for
Enter. Arrow-key movement and selection actions use command positions.

Although an empty command array deserializes, the interactive runner indexes
the current command during input. Supply at least one usable command rather
than relying on an empty menu at runtime. Position values are zero-based
``[x, y]`` arrays, not strings or ``{ x = ..., y = ... }`` tables.

Standalone menu (provision the referenced art separately):

.. code-block:: toml

   title = "Information"
   display_file = "art/information"
   menu_type = "Command"
   prompt = "R = rules, Q = return: "

   [[commands]]
   keyword = "R"
   display = "Rules"
   lighbar_display = "> Rules"
   position = [2, 4]
   [[commands.actions]]
   command_type = "DisplayFile"
   parameter = "conferences/main/rules"

   [[commands]]
   keyword = "Q"
   display = "Return"
   position = [2, 5]
   [[commands.actions]]
   command_type = "QuitMenu"

Message areas
-------------

``area_file`` selects an ``AreaList`` with required ``area`` array. Each
``[[area]]`` contains:

.. list-table:: Message-area fields
   :header-rows: 1
   :widths: 43 23 34

   * - Keys
     - Type / missing default
     - Meaning
   * - ``name``, ``path``
     - string, path / required
     - Display name and JAM message-base path/prefix
   * - ``is_read_only``, ``allow_aliases``
     - bool / required
     - Posting and alias policy; neither may be omitted
   * - ``qwk_name``
     - string / empty
     - QWK area name
   * - ``qwk_conference_number``
     - u16 / 0
     - QWK conference-number setting
   * - ``ftn_area_tag``, ``ftn_origin``
     - string / empty
     - FTN echo tag and per-area origin override (empty uses board origin)
   * - ``req_level_to_enter``, ``req_level_to_list``, ``req_level_to_save_attach``
     - expression / ``"0"``
     - Enter/list/save-attachment access

``number`` and ``valid`` are runtime-only. Empty ``ftn_area_tag`` leaves an
area local. Setup's General area has empty optional strings, number setting
0, false policy flags, and a provisioned JAM base; those files do not appear
merely by deserializing a path.

Standalone message-area file:

.. code-block:: toml

   [[area]]
   name = "General"
   path = "conferences/main/messages/general"
   is_read_only = false
   allow_aliases = false
   req_level_to_enter = "10"

File directories
----------------

``dir_file`` selects a ``DirectoryList``. Its required root array is also
``area``: use ``[[area]]``, **not** ``[[directory]]``, ``[[directories]]`` or
``[[file_directory]]``.

.. list-table:: File-directory fields
   :header-rows: 1
   :widths: 35 27 38

   * - Keys
     - Type / missing default
     - Meaning
   * - ``name``, ``path``
     - string, path / required
     - Display name and directory holding downloadable files
   * - ``password``
     - Password string / required
     - Use ``""`` for no password
   * - ``metadata_path``
     - path / empty
     - File-base metadata/index name, not the download directory
   * - ``sort_order``
     - enum / ``"FileName"``
     - ``NoSort``, ``FileName``, ``FileDate``
   * - ``sort_direction``
     - enum / ``"Ascending"``
     - ``Ascending``, ``Descending``
   * - ``ftn_area_tag``
     - string / empty
     - File-echo tag matched to incoming TIC ``Area``; empty disables that association
   * - ``has_new_files``, ``is_free``
     - bool / false
     - New-file marker and free-download flag
   * - ``list_security``, ``download_security``
     - expression / ``"0"``
     - Listing and download access

``number`` and ``valid`` are skipped. The custom ``SortOrder::from_str``
fallback used elsewhere is not the derived TOML enum deserializer: invalid
TOML enum strings fail, rather than silently choosing ``NoSort``.
Setup creates General with default filename/ascending sorting and an empty
metadata path. PCBoard directory import instead assigns ``path/dir`` as the
metadata name. Do not confuse these two sources of defaults.

Standalone directory file:

.. code-block:: toml

   [[area]]
   name = "General files"
   path = "conferences/main/general/files/dir00"
   metadata_path = "conferences/main/general/files/dir00/dir"
   password = ""
   sort_order = "FileDate"
   sort_direction = "Descending"
   list_security = "10"
   download_security = "20"

Languages
---------

The root ``date_formats`` is a **required** array of two-string arrays
``[display label, chrono strftime format]``. ``language`` is an optional array
of records, defaulting to empty. Each ``[[language]]`` requires **all five**
fields: ``description`` (string), ``locale`` (string), ``extension`` (string),
``yes_char`` (one Unicode character encoded as a string), and ``no_char``
(one Unicode character encoded as a string). Empty or multi-character
yes/no strings are invalid. Use the extension suffix without a leading dot.

``SupportedLanguages::default()`` supplies nine date-format pairs: the label
orders ``MM/DD/YY``, ``DD/MM/YY``, ``YY/MM/DD``, followed by the same three
orders using dots and hyphens. Their formats use ``%m``, ``%d``, ``%y`` in
the corresponding order. This constructor does **not** make a missing
``date_formats`` key valid. Setup adds English with ``Y``/``N`` and empty
``locale`` and ``extension``; ``en_US`` is not its serialized default.

Standalone language file:

.. code-block:: toml

   date_formats = [["MM/DD/YY", "%m/%d/%y"], ["DD.MM.YY", "%d.%m.%y"]]

   [[language]]
   description = "English"
   locale = "en_US"
   extension = ""
   yes_char = "Y"
   no_char = "N"

Security levels
---------------

``[[level]]`` records form the optional root ``level`` array. **Every field
is optional**; strings default to empty, integers to zero, and booleans to
false. Saving suppresses these empty/zero/false values.

.. list-table:: Security-level fields
   :header-rows: 1
   :widths: 45 15 40

   * - Keys
     - Type
     - Meaning
   * - ``description``, ``password``
     - string
     - Description and optional plain-string level password
   * - ``security``
     - u8
     - Exact security level to match
   * - ``time_per_day``, ``calls_per_day``
     - u32
     - Daily time and call allowances
   * - ``base_baud_rate``
     - u32
     - Download-allowance scaling reference; 0 leaves allowance unchanged
   * - ``batch_limit``
     - u32
     - Files per batch; 0 uses system default
   * - ``uldl_ratio_tenths``, ``uldl_kb_ratio_tenths``
     - u32
     - Download/upload file and byte ratios in tenths; 0 disables
   * - ``daily_file_limit``, ``daily_file_kb_limit``
     - u64
     - Daily file and kilobyte allowances
   * - ``file_limit``, ``file_kb_limit``
     - u64
     - File and kilobyte ratio-limit settings
   * - ``file_credit``, ``file_kb_credit``
     - u64
     - File and kilobyte credits
   * - ``enforce_time_limit``, ``allow_alias``, ``enforce_read_mail``
     - bool
     - Time enforcement, alias and required-mail-reading flags
   * - ``demo_account``, ``enabled``
     - bool
     - Exact serialized names (not ``is_demo_account`` / ``is_enabled``).
       ``enabled`` is PWRD accounting Y (enforce), not a general level switch.
   * - ``accounting_tracking``
     - bool
     - PWRD accounting T: tracking without balance enforcement. Takes precedence
       over ``enabled`` and requires a nonempty board tracking path.

Input aliases ``uldl_ratio`` and ``uldl_kb_ratio`` are accepted for
``uldl_ratio_tenths`` and ``uldl_kb_ratio_tenths``. The writer uses the latter
names; do not specify both an alias and its canonical key. A ratio of 5.0
is stored as ``50``. ``daily_file_kb_limit = 32767`` means unlimited;
zero blocks non-free downloads, not unlimited downloads.

Level lookup selects the **first** exact security match whose password is
empty or matches without regard to ASCII case. It is not a highest-level
less-than-or-equal search. An empty list is not populated automatically.

Global accounting must also be enabled. With T selected but an empty tracking
path, accounting is off even if Y is set. With neither flag (or no level match),
automatic accounting is off. Both active modes require a valid rate configuration.

Setup creates levels 10, 20, 100 and 110 with time allowances 60, 90, 540
and 999, and both ratio fields 10, 90, 150 and 250 respectively. All four
allow aliases; level 110 has daily kilobyte allowance 32767. Other fields,
including ``enabled``, retain the struct defaults (false/zero/empty).

Standalone security-level file:

.. code-block:: toml

   [[level]]
   description = "Regular caller"
   security = 20
   enabled = true
  accounting_tracking = false
   time_per_day = 90
   allow_alias = true
   daily_file_kb_limit = 32767

Transfer protocols
------------------

The optional root ``protocol`` array contains ``[[protocol]]`` records:

.. list-table:: Transfer-protocol fields
   :header-rows: 1

   * - Key
     - Type / missing default
   * - ``enabled``
     - bool / **true** (serialized only when false)
   * - ``batch``, ``bidirectional``
     - bool / false
   * - ``char_code``, ``description``
     - string / empty
   * - ``send_command``, ``recv_command``
     - custom protocol string / **required**

The protocol type has a custom string serializer, not a tagged enum. Its
internal values are ``"@asc"``, ``"@xmodem"``, ``"@xmodemcrc"``,
``"@xmodem1k"``, ``"@xmodem1kg"``, ``"@ymodem"``, ``"@ymodemg"``,
``"@zmodem"``, and ``"@zmodem8k"``. These ``@`` names are parsed
case-insensitively. ``""`` selects None; an unknown string starting with
``@`` also selects None. Any other string represents an external command.
In particular, ``"ZModem"`` is **not** the native spelling of Zmodem.
The protocol factory currently returns a no-transfer implementation for
None, ASCII and External: accepting an external command string does not
mean that the external program will be executed.

The Rust protocol-type default is Zmodem, but missing ``send_command`` or
``recv_command`` still fails. ``Protocol::default()`` has ``enabled`` false,
whereas omitted TOML ``enabled`` is true. Setup's generated list enables
A/ASCII, X/Xmodem, C/CRC, O/1K, F/1K-G, Y/Ymodem, G/Ymodem-G, Z/Zmodem,
8/Zmodem-8k and N/None. Y, G, Z, 8 and N are marked batch; none is marked
bidirectional. An absent root list simply yields an empty list instead.

Standalone protocol file:

.. code-block:: toml

   [[protocol]]
   char_code = "Z"
   description = "Zmodem"
   batch = true
   send_command = "@zmodem"
   recv_command = "@zmodem"

Doors and BBSLink accounts
------------------------------

A door file requires both root arrays: ``account`` and ``door``. For local
doors use ``account = []`` before the first ``[[door]]``. An empty standalone
door list is ``account = []`` followed by ``door = []``.

.. list-table:: Door fields
   :header-rows: 1
   :widths: 34 25 41

   * - Key
     - Type / missing default
     - Meaning
   * - ``name``, ``description``, ``password``
     - string / required
     - Door keyword/name, description and plain password (empty disables password)
   * - ``securiy_level``
     - expression / ``"0"``
     - Access expression; preserve the missing ``t`` in the key
   * - ``door_type``
     - enum / required
     - Exactly ``Local``, ``BBSlink`` or ``Dos``
   * - ``path``
     - string / required
     - Local executable/PPE, DOS door root, or BBSLink door code
   * - ``use_shell_execute``
     - bool / false
     - Local launch setting
   * - ``drop_file``
     - enum / ``"None"``
     - Drop-file format; exact values below
   * - ``dos_command``
     - string / empty
     - Command line run inside the DOS environment
   * - ``dos_memory_mb``
     - u32 / **64**
     - DOS guest memory setting in MiB
   * - ``dos_max_runtime_seconds``
     - u32 / **3600**
     - DOS wall-clock runtime limit; explicit 0 uses the safe runtime default
   * - ``charge_per_use``, ``charge_per_minute``
     - f64 / **0.0** each
     - Finite, non-negative accounting units per successful launch/connect and
       per elapsed minute (rounded up at 30 seconds). Zero is free.

Door and invoking-command rates can both apply; neither replaces global or
conference time. Admission checks the per-use charge plus one minute, not a
minimum bill or a funds reservation. A failed launch is not charged; errors
after launch still settle elapsed usage. The same optional ``charge_per_use``
and ``charge_per_minute`` keys exist on Command records (including menu commands),
not on CommandAction records. A multi-action command charges once per invocation.
Both rates are omitted on save when zero. Setup's Commands and Doors detail forms
expose both fields and validate on save. See :ref:`adding-commands` for the complete
command schema and binary CMD.LST rate layout. Logoff waits for enclosing
command/door usage to settle before final account persistence and summaries.

``number`` and ``valid`` are skipped. A Rust ``Door::default()`` uses Local
and zero-valued DOS integer fields; that is distinct from TOML's required
``door_type`` and custom missing-field defaults 64 and 3600. Setup initially
writes an empty door list, not a runnable sample door.

``drop_file`` values are exactly ``None``, ``PCBoard``, ``DoorSys``,
``Door32Sys``, ``DorInfo``, ``CallInfo``, ``DoorFileSR``, ``CurruserBBS``,
``ChainTXT``, ``TriBBSSYS``, ``SFDoorsDAT``, ``ExitInfoBBS``, and ``JumperDat``.
The filenames shown in setup (for example DOOR.SYS) are display labels,
not the serialized enum strings.

Standalone local-door file (the executable must be installed separately):

.. code-block:: toml

   account = []

   [[door]]
   name = "GAME"
   description = "Local game"
   password = ""
   securiy_level = "20"
   door_type = "Local"
   path = "doors/game/run"
   drop_file = "Door32Sys"

``DoorServerAccount`` is an **externally tagged** enum with the single
variant ``BBSLink``. Each account record therefore contains a nested
``BBSLink`` table. Its three fields ``system_code``, ``auth_code``, and
``sheme_code`` are all required strings, with no missing defaults. The
account tag is ``BBSLink`` but the door type string is ``BBSlink``. There is
no ``type = "BBSLink"`` discriminator and no correctly-spelled
``scheme_code`` alias. The runtime uses the first account for BBSLink doors;
there is no per-door account-index field. Do not configure a BBSLink door
with an empty account list.

Standalone BBSLink file (placeholder credentials deserialize but cannot
authenticate):

.. code-block:: toml

   [[account]]
   [account.BBSLink]
   system_code = "YOUR_SYSTEM_CODE"
   auth_code = "YOUR_AUTH_CODE"
   sheme_code = "YOUR_SCHEME_CODE"

   [[door]]
   name = "REMOTE"
   description = "BBSLink game"
   password = ""
   door_type = "BBSlink"
   path = "remote-game-code"

Bulletins and surveys
---------------------

The bulletin root array is required and named ``bullettin`` (two ``t``
characters). Each ``[[bullettin]]`` has exactly ``path`` (required path) and
``required_security`` (optional expression, default ``"0"``). There is no
name or description field. Numbering follows list order. Setup supplies
Rules and History display-file entries.

Standalone bulletin file:

.. code-block:: toml

   [[bullettin]]
   path = "conferences/main/rules"
   required_security = "0"

The survey root array is required and named ``survey``. Each ``[[survey]]``
has exactly ``survey_file`` and ``answer_file`` (both required paths), plus
``required_security`` (optional expression, default ``"0"``). The first
path selects the questionnaire/display/PPE, the second its answer output.
Setup creates two entries, one display-based and one PPE-based. ``Script``
command actions select surveys by their one-based list number.

Standalone survey file:

.. code-block:: toml

   [[survey]]
   survey_file = "conferences/main/script1.pcb"
   answer_file = "conferences/main/script1.answer"
   required_security = "10"

Accounting rates
----------------

The accounting-rate file is a single root record, **not** an
``[accounting]`` table (that table belongs to the board configuration).
Every field below is a required ``f64``. Rust's default constructor and
setup both produce all-zero rates, but omitting a rate from a hand-written
file fails deserialization.

* ``new_user_balance``: one-time registration grant for new users only;
  does not fund an existing user even when their account was previously absent.
* ``warn_level``: balance warning threshold.
* ``charge_per_logon``: per-logon charge.
* ``charge_per_time``, ``charge_per_peak_time``,
  ``charge_per_group_chat_time``: normal and peak units per online minute;
  group-chat units per rounded elapsed minute, additional to online time.
* ``charge_per_msg_read``, ``charge_per_msg_read_captured``: read and
  captured-message read charges.
* ``charge_per_msg_written``, ``charge_per_msg_write_echoed``,
  ``charge_per_msg_write_private``: ordinary, echoed and private posting charges.
* ``charge_per_download_file``, ``charge_per_download_bytes``: download
  file and **KiB** rates (1 KiB = 1024 bytes), despite the legacy bytes key.
* ``pay_back_for_upload_file``, ``pay_back_for_upload_bytes``: upload
  file and **KiB** credits. Positive values increase balance, not decrease it.

Enabling accounting, peak-time settings, display files and tracking files are
separate board settings. All rate fields must be finite; negative global rates
are supported for intentional adjustments (unlike command/door rates). Setup
validates before saving and reports malformed existing files instead of
substituting a zero draft. F1 help explains every rate.

Successful downloads and accepted uploads post **whole KiB rounded down per
file**. Download preflight uses fractional KiB plus estimated normal-rate online
time; this is not an upload-receive affordability check. Successful manual-approval
intake (``AwaitingApproval``) earns upload credit before publication or approval;
later administrative rejection does not automatically reverse it. Scanner-rejected
uploads and failed publication earn no credit.
Free files/directories waive file/KiB debits, not time. NoTime/FSEC monetary
transfer-time refunds are not implemented. Read/capture rates add the conference
read surcharge. Writes select private, else echoed, else ordinary rate, then
add the conference write surcharge. Conference time also adds to global time.

Global time bills calendar-minute boundaries; balance preview waives one normal
minute (or one peak minute if no normal minute exists), final settlement does not.
Activity and conference minutes round up at 30 seconds. Peak windows include both
HH:MM endpoints and can cross midnight. Each minute uses its local date's
Sunday-first day mask and MM-DD-YY holiday patterns with uppercase X wildcards.
See the operator guide ``docs/accounting.md`` for a funded minimal setup and the
sum-versus-maximum debit policy; the all-zero file below alone charges nothing.

PCBoard's corresponding binary import is 15 little-endian 64-bit floats
(120 bytes), not TOML; use import rather than renaming that file.

Standalone all-zero accounting-rate file:

.. code-block:: toml

   new_user_balance = 0.0
   warn_level = 0.0
   charge_per_logon = 0.0
   charge_per_time = 0.0
   charge_per_peak_time = 0.0
   charge_per_group_chat_time = 0.0
   charge_per_msg_read = 0.0
   charge_per_msg_read_captured = 0.0
   charge_per_msg_written = 0.0
   charge_per_msg_write_echoed = 0.0
   charge_per_msg_write_private = 0.0
   charge_per_download_file = 0.0
   charge_per_download_bytes = 0.0
   pay_back_for_upload_file = 0.0
   pay_back_for_upload_bytes = 0.0

User-maintenance security tables
--------------------------------

These tables are separate from the login/security-level definitions above.
The file is named ``security_tables.toml`` beside the user file; there is
no separate board path selector. The root fields ``file_ratio``,
``byte_ratio``, ``uploads``, and ``downloads`` are each optional arrays of
``TableEntry`` records and each defaults to empty. Each entry requires
``value`` (f64 threshold) and ``security`` (u8 target level). There is no
outer ``[security_tables]`` table and no serialized ``TableKind`` enum.

File/byte ratios are **uploads divided by downloads**, not the download/upload
ratios in tenths used in security-level records. ``uploads`` and ``downloads``
use file counts, not bytes. A zero download count is treated as a denominator
of one, so its ratio equals the upload count. Applying a table sorts thresholds ascending,
uses the highest reached threshold (or the first when below all thresholds),
and only adjusts selected users whose current level occurs somewhere in that
table. Merely loading a table does not run a maintenance operation.

The maintenance loader returns empty defaults when the file is absent or
cannot be parsed. Selection criteria, packing options, ``SecurityField``,
``ExpirationChange`` and counter operations in the maintenance module are
runtime types, **not additional TOML fields** in this file.

Standalone maintenance security-table file:

.. code-block:: toml

   byte_ratio = []
   uploads = []
   downloads = []

   [[file_ratio]]
   value = 0.0
   security = 10

   [[file_ratio]]
   value = 1.0
   security = 20

Source and compatibility boundary
---------------------------------

This specification is based on ``conferences.rs``, ``commands.rs``,
``menu.rs``, ``message_area.rs``, ``file_directory.rs``, ``language.rs``,
``sec_levels.rs``, ``xfer_protocols.rs``, ``doors/mod.rs``, ``bulletins.rs``,
``surveys.rs``, ``accounting_cfg.rs``, and ``user_maintenance.rs`` in the
engine's ``icy_board`` module. Custom encodings were checked against
``security_expr.rs``, ``user_base.rs`` and the network protocol serializer;
setup defaults against the setup creator, and menu behavior against the
runtime menu runner. Legacy import formats and Rust display labels are not
alternative native TOML schemas.