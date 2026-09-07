User, text and runtime data files
=================================

This reference distinguishes editable TOML data from machine-managed checkpoints
and non-TOML legacy formats. Board/component/network/event configuration is
documented separately. Stop the board and mailer before making offline changes
to mutable data; save a consistent backup. Rewriting a checkpoint can duplicate
mail, lose a queued operation or remove a replay barrier.

Conventions
-----------

Keys are case-sensitive and use the spelling shown below, including historical
misspellings. ``u8``, ``u16`` and ``u32`` denote nonnegative TOML integers bounded
by 255, 65535 and 4294967295. ``i32`` and ``i64`` are signed integers; ``u64`` is
the Rust unsigned 64-bit type, although portable TOML integers are signed 64-bit.
``usize`` is a nonnegative platform-sized integer. Do not depend on a particular
serializer accepting values beyond TOML's signed 64-bit range. ``f64`` fields are
floating-point numbers, best written with a decimal point.

An omitted ``Option`` means no value/table; TOML has no ``null``. A type deriving
Rust ``Default`` does not make its TOML fields optional. The tables below identify
actual deserialization defaults separately from zero-initialized Rust records.
Generic ``IcyBoardSerializer`` files are UTF-8 TOML, saved by atomic replacement;
they have no mandatory magic header. ICBTEXT is the exception described below.

User database: users.toml
---------------------------

The board's ``paths.user_file`` selects this document (normally
``main/users.toml``). It contains a required ``users`` array, represented as
``[[users]]`` entries. An empty database must say ``users = []``; a blank document
does not deserialize as ``UserBase``. Array order is the user-record order; there
is no stored ``id`` field. Do not reorder records casually.

Each user has required ``name``, ``security_level``, ``exp_security_level``,
``protocol`` and ``page_len`` scalars, and required ``password``, ``flags`` and
``stats`` tables. All other user fields are optional or have field defaults as
listed below. Unknown fields are not rejected by these structs, so misspellings
can disappear on the next save.

Identity, contact and display fields
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. list-table:: User scalars and collections
   :header-rows: 1
   :widths: 43 22 35

   * - Exact keys
     - Type / omitted value
     - Meaning
   * - ``name``
     - String; required
     - Login/display name.
   * - ``path``
     - Optional path string
     - Destination used by the individual ``User::save`` method. Not a user ID
       or required database location; that method does nothing if absent.
   * - ``alias``, ``verify_answer``
     - Strings / ``""``
     - Alternate login name and verification answer.
   * - ``city_or_state``, ``city``, ``state``
     - Strings / ``""``
     - Legacy combined location and separate location fields; not aliases of
       one another in serialization.
   * - ``street1``, ``street2``, ``zip``, ``country``
     - Strings / ``""``
     - Postal address.
   * - ``gender``, ``email``, ``web``
     - Strings / ``""``
     - Profile/contact information; gender is not a TOML enum.
   * - ``bus_data_phone``, ``home_voice_phone``
     - Strings / ``""``
     - Business/data and home/voice phone numbers, not numeric quantities.
   * - ``contacts``
     - Array of tables / ``[]``
     - Each entry requires string ``service`` and ``account``. It can be written
       as ``[[users.contacts]]`` or inline tables. The mutation API caps a user
       at 100 contacts; plain Serde deserialization does not enforce that cap.
   * - ``date_format``
     - String / ``""``
     - User date-display format. Empty falls back through board display logic;
       do not confuse it with the timestamp serialization format.
   * - ``language``
     - String / ``""``
     - User language selection, not a fixed enum. Used to select language text.
   * - ``birth_date``, ``expiration_date``, ``date_last_dir_read``
     - UTC timestamp strings / epoch
     - Birth date, account expiration and most recent directory scan date.
   * - ``user_comment``, ``sysop_comment``
     - Strings / ``""``
     - User/sysop notes.
   * - ``custom_comment1``, ``custom_comment2``, ``custom_comment3``,
       ``custom_comment4``, ``custom_comment5``
     - Strings / ``""``
     - Five independent custom notes.
   * - ``security_level``, ``exp_security_level``
     - ``u8``; required
     - Current and expired-account security levels. Zero is accepted, not an
       implicit omission default.
   * - ``protocol``
     - String; required
     - Transfer-protocol selection (normally a configured single-letter code).
       The serializer does not restrict this to A--Z.
   * - ``page_len``
     - ``u16``; required
     - Screen page length in lines.
   * - ``last_conference``
     - ``u16`` / ``0``
     - Last conference number/index.
   * - ``elapsed_time_on``
     - ``u16`` / ``0``
     - Stored elapsed online minutes (legacy field).
   * - ``chat_status``
     - String / ``"Available"``
     - Exactly ``"Available"`` or ``"Unavailable"``.
   * - ``conference_flags``
     - Custom string / empty map
     - Conference flags; syntax below.
   * - ``lastread_ptr_flags``
     - Custom string / empty map
     - Per-conference/area read pointers; syntax below.
   * - ``tpa_records``
     - Array of tables / ``[]``
     - Third-party application data; nested structure below.
   * - ``qwk_config``, ``account``, ``bank``
     - Optional tables
     - Legacy-compatible QWK, accounting and bank information; absence differs
       from a present table filled with zeroes.

The string defaults listed as empty, empty contacts/TPA arrays, and the zero
``last_conference``/``elapsed_time_on`` values are omitted on save. Other required
scalars and the three top-level user timestamps are written even when defaulted.

UTC timestamps and passwords
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``DateTime<Utc>`` values use **quoted RFC3339 strings**, such as
``"2026-09-07T12:00:00Z"``, not unquoted TOML local dates. Missing timestamp fields
with a Serde default become ``1970-01-01T00:00:00Z``. This applies to the three
user dates above, both password dates and both statistics dates below. It is not
the bank date format.

The required ``[users.password]`` table is a ``PasswordInfo`` record:

* ``password``: required string using the special encoding below.
* ``prev_pwd``: array of specially encoded password strings, default ``[]`` and
  omitted when empty. Normal password changes retain the last three historical
  entries; deserialization does not impose that maximum.
* ``last_change``: UTC timestamp string, default epoch.
* ``times_changed``: ``u64`` count, default zero and omitted when zero.
* ``expire_date``: UTC timestamp string, default epoch. Zero-expiration policy
  writes the epoch sentinel; this is distinct from ``users.expiration_date``.

Password encoding examines the *decoded TOML string* in this order:

1. Prefix ``bcrypt:`` selects bcrypt; the remainder is the stored bcrypt hash.
2. Prefix ``$argon2`` selects an Argon2 PHC hash string as stored.
3. A string beginning and ending with literal double-quote characters is a
   plaintext password with that outer pair removed.
4. Other strings are accepted as legacy plaintext.

The writer always wraps plaintext in that additional pair of quote characters,
so a cleartext example would be ``password = '"example"'`` in TOML. This is a
format explanation, not a recommendation to store plaintext. Use the user editor
to generate hashes; keep this file, backups and password history private.
``Protected`` is only an in-memory comparison wrapper and has no separate disk
tag. Password creation/login paths normalize case; the TOML reader itself does
not lowercase existing plaintext.

User flags
~~~~~~~~~~

The required ``[users.flags]`` table may be empty. ``fse_mode`` defaults to
``"Yes"`` and accepts exactly ``"Yes"``, ``"No"`` or ``"Ask"``. It is written
even at its default. The following twelve booleans all default to ``false`` and
are omitted when false:

.. list-table:: Boolean user flags
   :header-rows: 1

   * - Key
     - Meaning
   * - ``expert_mode``
     - Expert command/menu mode.
   * - ``is_dirty``
     - Legacy dirty-record state.
   * - ``msg_clear``
     - Clear screen between messages.
   * - ``has_mail``
     - Stored mail-waiting flag.
   * - ``scroll_msg_body``
     - Message-body scrolling preference.
   * - ``use_short_filedescr``
     - Short file descriptions.
   * - ``long_msg_header``
     - Long message headers.
   * - ``wide_editor``
     - Wide message editor preference.
   * - ``delete_flag``
     - Marked for deletion; not the same as physically removing the record.
   * - ``disabled_flag``
     - Disabled account.
   * - ``use_graphics``
     - Graphics display preference.
   * - ``use_alias``
     - Alias-use preference.

User statistics
~~~~~~~~~~~~~~~

``[users.stats]`` is required, but every member has a deserialization default, so
an empty table is valid. ``first_date_on`` and ``last_on`` are UTC timestamp
strings defaulting to epoch. All remaining fields default to zero and are omitted
when zero:

.. list-table:: Complete user counter inventory
   :header-rows: 1
   :widths: 54 12 34

   * - Keys
     - Type
     - Unit / meaning
   * - ``num_times_on``
     - ``u64``
     - Calls/connections.
   * - ``messages_read``, ``messages_left``
     - ``u64``
     - Message counts.
   * - ``num_sec_viol``, ``num_not_reg``
     - ``u64``
     - Security violations and unregistered-conference attempts.
   * - ``num_reach_dnld_lim``, ``num_file_not_found``
     - ``u64``
     - Download-limit and missing-download incidents.
   * - ``num_password_failures``, ``num_verify_errors``
     - ``u64``
     - Password failures and upload-verification errors.
   * - ``num_sysop_pages``, ``num_group_chats``, ``num_comments``
     - ``u64``
     - Sysop pages, group chats and comments to sysop.
   * - ``num_uploads``, ``num_downloads``
     - ``u64``
     - Lifetime file counts.
   * - ``total_dnld_bytes``, ``total_upld_bytes``
     - ``u64``
     - Lifetime byte counts (not kilobytes).
   * - ``today_num_downloads``, ``today_num_uploads``
     - ``u64``
     - Today's file counts.
   * - ``today_dnld_bytes``
     - ``i64``
     - Today's charged download bytes. Can be negative when upload credit
       exceeds today's usage; do not convert it to an unsigned counter.
   * - ``today_upld_bytes``
     - ``u64``
     - Today's uploaded bytes.
   * - ``total_doors_executed``
     - ``u64``
     - Door execution count.
   * - ``minutes_today``
     - ``u16``
     - Today's online minutes.

Conference flags and read pointers
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Neither field is a nested TOML map. They are custom strings:

* ``conference_flags = "0:5;2:1;"``: semicolon-separated
  ``conference:decimal_bits`` records. The conference is ``usize`` and the bits
  parse as ``u8``. Flags are Registered=1, Expired=2, Selected=4, Sysop=8,
  MailWaiting=16 and NetStatus=32. The writer persists **only bits 1, 2 and 4**;
  the other flags may be accepted on read but are removed on rewrite. Empty
  entries are skipped, and a wholly empty map writes ``""``.
* ``lastread_ptr_flags = "0,0,10,12,1;"``: semicolon-separated
  ``conference,area,last_read,highest_msg_read,include_qwk`` records. The first
  four values are ``usize`` indices/message numbers; the final value parses as
  ``usize`` and is true only when exactly ``1``. The writer emits 0 or 1. A new
  ``LastReadStatus`` defaults to zero pointers and ``include_qwk = true``, but
  omission of this user field produces an empty map rather than fabricated
  entries for all areas.

Malformed entries are silently ignored by both readers. Later duplicates replace
earlier ones. Map iteration order is not stable on save. Conference and area keys
refer to the board's internal zero-based indices; changing configuration order
without migrating pointers can change which area a record describes.

QWK information
~~~~~~~~~~~~~~~

If ``[users.qwk_config]`` is present, **all six fields are required**. There are no
field-level Serde defaults, despite the Rust record's zero/false default:

* ``max_msgs`` (``u16``): personal message-count limit.
* ``max_msgs_per_conf`` (``u16``): per-conference message-count limit.
* ``personal_attach_limit`` (``i32``): legacy personal attachment limit.
* ``public_attach_limit`` (``i32``): legacy public attachment limit.
* ``new_blt_limit`` (``i32``): legacy new-bulletin limit.
* ``new_files`` (boolean): legacy new-files selection.

These are preserved PCBoard QWK preferences, not QWKnet hub configuration. The
settings dialog clamps the two message counts to board-wide limits. The current
native QWK packet writer does not consistently consume these stored preferences;
in particular, the three signed legacy limits have no enforced native unit or
sentinel policy in that writer. Do not assume that zero means unlimited or that
the attachment fields impose a working byte ceiling. PCBoard binary export casts
those three ``i32`` values back to signed 16-bit fields; values outside that range
lose information.

Accounting information
~~~~~~~~~~~~~~~~~~~~~~

If ``[users.account]`` is present, all eighteen fields below are required. The
seventeen ``f64`` values are balances or accumulated monetary/accounting units,
**not rate configuration and not raw byte/minute counters**. Rust-created empty
records use 0.0 and security zero; omission in a present TOML table is an error.

* Balances: ``starting_balance``, ``start_this_session``.
* Debits: ``debit_call``, ``debit_time``, ``debit_msg_read``,
  ``debit_msg_read_capture``, ``debit_msg_write``, ``debit_msg_write_echoed``,
  ``debit_msg_write_private``, ``debit_download_file``, ``debit_download_bytes``,
  ``debit_group_chat``, ``debit_tpu``, ``debit_special``.
* Credits: ``credit_upload_file``, ``credit_upload_bytes``, ``credit_special``.
* ``drop_sec_level``: ``u8``, lower session security ceiling when enforced funds
  run out (unless ``ignore_empty_sec_level``). Does not permanently change the
  user's normal level or promote them.

The site defines accounting value/currency conventions. There is no currency
code, decimal scale or rounding rule stored in this user record. Board accounting
rate tables are a separate component format. Display uses comma-grouped credits
with up to six decimal places and trailing zeros trimmed, or deterministic
US-dollar-style money with exactly two decimals (``$1,234.50``), independent
of the host locale. Display rounding does not change the stored f64 values.

Balance is ``starting_balance - sum(debits) + sum(credits)``, or subtracts only
the maximum debit category with ``concurrent_tracking``. Pending global time
joins ``debit_time`` before that maximum. Positive credits add funds.
``start_this_session`` remembers the opening balance, not a second source of funds.
``CREDNOW`` is session net usage, ``CREDUSED`` cumulative net usage,
``CREDSTART`` the stored starting balance, and ``CREDLEFT`` the enforced balance.

New-user grants apply at registration only. Existing users without an account
start at zero. The current system-manager editor has no monetary funding fields;
use a controlled PPE ``ACCOUNT START_BAL, amount`` adjustment followed by
``PUTUSER`` with a verified selected identity. The statement adds to the balance;
it does not assign the requested final balance. See ``docs/accounting.md`` for
the safe current/alternate-user workflow and persistence boundaries.

Runtime saves merge monetary deltas against the latest shared-board record,
but this is neither a cross-process user-file lock nor a funds reservation.
Audit appends and user saves are separate operations, not a crash-atomic ledger.
Mid-call profile saves (W/LANG) leave accounting active. Logoff finalization waits
for enclosing command/door usage to settle, then posts pending time and persists
the account before displaying final summaries. Settlement or final-save errors
suppress those summaries; terminal output is not a financial durability guarantee.

Bank information
~~~~~~~~~~~~~~~~

Present ``[users.bank]`` requires both ``[users.bank.time_info]`` and
``[users.bank.byte_info]``. Each has exactly the same six required fields:

.. list-table:: BankInfo fields (both subrecords)
   :header-rows: 1

   * - Key
     - Type
     - Meaning
   * - ``last_deposite_date``
     - TOML local date
     - Last deposit date; preserve this exact misspelling.
   * - ``last_withdraw_date``
     - TOML local date
     - Last withdrawal date.
   * - ``last_transaction_amount``
     - ``u32``
     - Last transaction amount.
   * - ``amount_saved``
     - ``u32``
     - Current saved amount.
   * - ``max_withdrawl_per_day``
     - ``u32``
     - Maximum daily withdrawal; preserve this exact misspelling.
   * - ``max_stored_amount``
     - ``u32``
     - Maximum saved amount.

Amounts use the bank's native units: online time (minutes) in ``time_info`` and
transfer bytes in ``byte_info``. The serializer applies no conversion and enforces
no relationship between saved amounts and maxima. Do not infer an unlimited
meaning merely from a zero maximum.

The two dates are custom ``IcbDate`` values, written **unquoted**, for example
``last_deposite_date = 2026-09-07``. A Rust-default date serializes as
``0000-01-01`` (year zero), not the UTC epoch. Missing date keys in a present bank
table are errors. Do not supply a time-only value: the custom converter expects a
date component. This differs from every ``DateTime<Utc>`` field above.

Third-party application records
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``[[users.tpa_records]]`` requires string ``keyword`` and optionally has string
``data`` (default empty, omitted when empty) and ``conferences`` (array, default
empty, omitted when empty). Each ``[[users.tpa_records.conferences]]`` entry
requires ``conference`` (``usize`` conference index) and ``data`` (string).
Application data is stored as text, not interpreted as a nested TOML document.
The mutation API organizes records by keyword/conference, but Serde does not
validate uniqueness during loading.

Minimal valid user-database example
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

This demonstrates required fields, not a ready-to-enable login. The account is
deliberately disabled and has an empty password. Use account-management tools to
set its password/security before enabling it.

.. code-block:: toml

   [[users]]
   name = "Example User"
   security_level = 0
   exp_security_level = 0
   protocol = "N"
   page_len = 24
   conference_flags = "0:5;"
   lastread_ptr_flags = "0,0,0,0,1;"

   [users.password]
   password = '""'

   [users.flags]
   disabled_flag = true
   fse_mode = "Ask"

   [users.stats]

Optional subrecords for a user can be appended before the next ``[[users]]``:

.. code-block:: toml

   [users.qwk_config]
   max_msgs = 100
   max_msgs_per_conf = 50
   personal_attach_limit = 0
   public_attach_limit = 0
   new_blt_limit = 0
   new_files = false

   [users.bank.time_info]
   last_deposite_date = 2026-09-07
   last_withdraw_date = 2026-09-07
   last_transaction_amount = 10
   amount_saved = 30
   max_withdrawl_per_day = 15
   max_stored_amount = 120

   [users.bank.byte_info]
   last_deposite_date = 2026-09-07
   last_withdraw_date = 2026-09-07
   last_transaction_amount = 1024
   amount_saved = 4096
   max_withdrawl_per_day = 2048
   max_stored_amount = 1048576

   [[users.contacts]]
   service = "Matrix"
   account = "@example:example.org"

   [[users.tpa_records]]
   keyword = "MYAPP"
   data = "last-choice=2"

   [[users.tpa_records.conferences]]
   conference = 0
   data = "visited"

Individual user files
~~~~~~~~~~~~~~~~~~~~~~~

``User::save`` can serialize the same record without the ``[[users]]`` wrapper
to its optional ``path``. Nested tables would then be ``[password]``, ``[flags]``,
``[stats]`` and so on. Its ``home_dir`` argument is not used. The apparent code
writing a per-home-directory ``user.toml`` in the board module is commented out;
it is not evidence of an additional active user-database loader. Normal board
loading uses the aggregate ``UserBase`` document.

User-maintenance security tables
----------------------------------

``security_tables.toml`` lives beside the configured user file. It is an editable
ICBSM maintenance input, **not machine-generated usage statistics** and not the
board's security-level-definition file. ``SecurityTables`` has four arrays of
tables, each defaulting to empty: ``file_ratio``, ``byte_ratio``, ``uploads`` and
``downloads``. Every entry requires ``value`` (``f64`` threshold) and ``security``
(``u8`` target level).

Ratios are uploads divided by downloads (file counts or lifetime bytes), not
percentages or PCBoard scaled integers. A zero download denominator is treated
as one, so the ratio then equals the upload count. Upload/download thresholds are file
counts. When applied, thresholds are sorted by value; the highest reached
threshold wins, or the first threshold below all steps. Only users whose current
security appears in the selected table are eligible. An empty table changes no
users. The convenience loader falls back to empty tables if the file is missing
or invalid, so a parse error can otherwise look like a no-op.

.. code-block:: toml

   [[file_ratio]]
   value = 0.0
   security = 10

   [[file_ratio]]
   value = 0.5
   security = 20

   [[uploads]]
   value = 100.0
   security = 30

Board statistics: statistics.toml
-----------------------------------

``paths.statistics_file`` selects a mutable ``Statistics`` document, normally
``main/statistics.toml``. It requires ``last_callers`` (array), ``[today]`` and
``[total]``. ``today_date`` is the only optional root field, a string defaulting
to ``""``; normal writes use the board host's local ``YYYY-MM-DD`` date.

Each ``[[last_callers]]`` requires ``user_name`` and ``time`` strings. ``time`` is
written as UTC RFC3339, but its type is a plain string, not a validated timestamp.
Normal updates retain ten callers. Deserialization does not cap the array.

Both ``[today]`` and ``[total]`` require all six ``u64`` fields: ``calls``,
``messages``, ``uploads``, ``uploads_kb``, ``downloads``, ``downloads_kb``.
Calls/messages/files are counts. The ``*_kb`` fields are whole 1024-byte units:
each completed transfer's total bytes are divided by 1024 with truncation before
adding. They are not the byte-unit user counters. Rust defaults are zero, but
missing individual counters in TOML are errors.

When a new counter event sees a different local date, ``today`` is cleared and
``today_date`` updated. This also happens to an older file that omitted
``today_date``. ``total`` is not reset.

.. code-block:: toml

   today_date = "2026-09-07"
   last_callers = []

   [today]
   calls = 0
   messages = 0
   uploads = 0
   uploads_kb = 0
   downloads = 0
   downloads_kb = 0

   [total]
   calls = 0
   messages = 0
   uploads = 0
   uploads_kb = 0
   downloads = 0
   downloads_kb = 0

ICBTEXT: header-detected text catalog
---------------------------------------

ICBTEXT is **not** loaded by ``IcyBoardSerializer``. ``IcbTextFile::load`` reads
bytes and dispatches by their prefix, regardless of the filename extension.
The writer begins the file with this exact line followed by a blank line:

.. code-block:: text

   # IcyBoard text file v1.0

The current detector checks the first nine bytes (``# IcyBoar``), not the full
version line. Preserve the complete canonical header at byte zero: a BOM, leading
blank line, another comment or removal of the header sends an otherwise valid
TOML document to the PCBoard binary importer instead.

After detection, the content is decoded by the text helper and parsed as a TOML
table. Root table names must be exact symbolic ``IceText`` names, such as
``LeaveComment`` and ``CommentFieldPrompt``, not message numbers, arbitrary
translation keys or ``[[text]]`` entries. Each table supports:

* ``text``: required string. Standard TOML escaping applies; an empty string
  deliberately disables some prompts. Display macros such as ``@USER@`` remain
  text for the runtime to expand.
* ``style``: optional string, default ``"Plain"``. Accepted values are exactly
  ``"Plain"``, ``"Red"``, ``"Green"``, ``"Yellow"``, ``"Blue"``,
  ``"Purple"``, ``"Cyan"``, ``"White"``. The nonplain values select DOS
  light-red/light-green/yellow/light-blue/magenta/light-cyan/white colors.
* ``justify``: optional string, default ``"Left"``. Accepted values are exactly
  ``"Left"``, ``"Right"``, ``"Center"``. The key is not ``justification``.

Invalid symbolic names, missing/nonstring ``text`` and invalid string enum values
fail loading. Extra entry keys are ignored. Nonstring ``style``/``justify`` values
are currently ignored and fall back to defaults, rather than rejected. A valid
symbolic root key whose value is not a table is skipped. Neither permissive
behavior should be used when authoring files.

.. warning::

   Edit a complete catalog generated by ``mkicbtxt`` or shipped with the board.
   Do not build a sparse override file. The current TOML loader fills numbered
   slots and then flattens missing slots, which can shift later entries onto the
   wrong message numbers. Runtime fallback only handles an index beyond the
   loaded vector; it does not repair these holes. Keep all symbolic entries and
   set unwanted text to ``""`` rather than deleting a table. Record zero is
   reserved internally and omitted by the writer.

This is a **fragment showing entry syntax**, not a complete replacement catalog:

.. code-block:: toml

   # IcyBoard text file v1.0

   [LeaveComment]
   text = "Leave a comment for the sysop (Enter)=no"
   style = "White"

   [CommentFieldPrompt]
   text = "Your computer"
   style = "Yellow"
   justify = "Left"

The writer emits entries in numeric order and omits default style/justification.
It escapes controls and non-ASCII characters itself rather than serializing a
Serde struct. Its current Unicode escape implementation uses ``\u`` with a
minimum four-digit width even for non-BMP characters; avoid assuming astral
characters round-trip correctly through an editor save.

Without the IcyBoard prefix, the same loader expects legacy **binary PCBTEXT**:
80-byte records, first byte a color code and the remaining 79 bytes CP437 text.
The first record's text must begin ``PCBoard version`` and file length must be a
multiple of 80. Legacy justification is inferred from hard-coded record numbers.
``save`` writes the IcyBoard format; explicit PCBoard export writes binary.
Renaming PCBTEXT to ``.toml`` does not convert it. Language-specific ICBTEXT
files use the same detection and schema, not a different translation format.

Machine-managed TOML
----------------------

The following formats describe persistent runtime state, not hand-authored
policy. There is no shared runtime-state wrapper or universal ``version`` key.
Keep state with its matching payloads/message bases. Defaults below describe
reader behavior, not a recommendation to erase existing state.

Upload quarantine records
~~~~~~~~~~~~~~~~~~~~~~~~~

Under the configured quarantine root, ``records/<id>.toml`` contains one
``QuarantineRecord`` (no array wrapper). All fields are required except the two
default-empty arrays noted below:

* ``id``, ``original_name``, ``uploader``: strings.
* ``payload_file``: path string relative to the quarantine root, normally
  ``files/<id>.<extension>``; extensionless payloads are possible.
* ``destination``, ``metadata_path``: path strings saved by the enqueue caller
  for publication/file-base metadata. These are not automatically relative to
  the record file.
* ``description``: array of strings (description lines).
* ``uploaded_at``: quoted UTC RFC3339 timestamp.
* ``status``: one of ``"pending"``, ``"processing"``, ``"needs_review"``,
  ``"ready_to_publish"``, ``"awaiting_approval"``, ``"publishing"``,
  ``"published"``, ``"rejected"``; no omission default.
* ``processing_report``: string array, default empty.
* ``decisions``: array of tables, default empty. Every ``[[decisions]]`` entry
  requires ``at`` (UTC timestamp), ``actor`` (string), ``from`` and ``to``
  (status strings from the same enum), and ``note`` (string).

The root also contains payloads in ``files/`` and a non-TOML
``.quarantine.lock``. Interrupted ``processing``/``publishing`` records are moved
to ``needs_review`` at startup with an appended decision. Never edit status to
bypass approval or recovery; use the quarantine-management operations.

Event execution journal
~~~~~~~~~~~~~~~~~~~~~~~

``event_history.toml`` in the board root is a versioned execution journal, not
the configured event list. Root fields ``version`` (``u32``, currently exactly 1)
and ``entries`` (array) are required. An empty journal is
``version = 1`` plus ``entries = []``. Unknown fields at root or entry level are
rejected.

Every ``[[entries]]`` has these fields:

.. list-table:: Journal entry
   :header-rows: 1
   :widths: 24 28 48

   * - Key
     - Type / omission
     - Meaning
   * - ``key``
     - String; required
     - Scheduled occurrence ``<event_id>@<UTC RFC3339>`` or manual
       ``manual@<UUID>``.
   * - ``event_id``
     - String; required
     - Nonempty, at most 128 bytes, ASCII alphanumeric/hyphen/underscore.
   * - ``description``
     - String; required
     - Snapshot of the event description.
   * - ``scheduled_for``
     - UTC timestamp string; required
     - Scheduled instant.
   * - ``start``, ``finish``
     - Optional UTC timestamp strings
     - Attempted start persisted before spawning, and outcome time.
   * - ``result``
     - Enum string; required
     - ``pending``, ``success``, ``nonzero_exit``, ``spawn_error``, ``wait_error``,
       ``interrupted``, ``skipped_busy``, ``expired`` or ``superseded``.
   * - ``exit_code``
     - Optional ``i32``
     - Process exit status when available.
   * - ``log_file``
     - Optional string
     - Planned board-root-relative ``event_logs/<unique-name>.log``.
   * - ``manual``
     - Boolean; required
     - Whether this was a manually requested occurrence.
   * - ``execution``
     - Enum string; required
     - ``"online"`` or ``"maintenance"``.
   * - ``detail``
     - Optional string
     - Additional outcome/recovery detail.

Occurrence keys must be unique; nonmanual keys must match the ID and scheduled
timestamp, and manual keys must start with ``manual@``. ``finish`` is absent
exactly when ``result`` is ``pending``. ``start`` and ``log_file`` are either both
present or both absent. Log names are constrained to the event-log directory and
safe ASCII filenames. A planned log does not prove that a process ran or that the
file was successfully created.

Startup marks pending entries interrupted, rather than replaying them. The
independent ``.event_history.lock`` lease and atomic, synced snapshots protect
claims. Logs may be rotated separately, but deleting/rotating journal entries
removes the replay barrier. This journal is not rewritten by a normal board
configuration save/reload.

FTN scan checkpoint and FREQ usage
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

FTN outbound ``scan.toml`` is a root table with ``serial`` (``u32``, default 0)
and ``[exported]`` (string-to-``u32`` map, default empty). ``serial`` supplies
locally generated message IDs; ``exported`` maps area tags to last-exported/high
message numbers. No ``version`` field exists. A missing file starts from an empty
state; a parse error is reported. A previously unseen area is initialized to its
current high-water mark rather than exporting its whole history. Removing this
file can therefore skip pending content and resets serial identity history.

FTN outbound ``freq_usage.toml`` has required ``day`` (local ``YYYY-MM-DD`` string)
and optional ``[nodes]`` (string-to-``u64`` map, empty default). Keys are node
addresses, values served bytes for that day. Missing, invalid or other-day data
is treated as a new day's empty usage. This permissive recovery can reset a
daily quota; do not use manual edits as ordinary quota configuration.

QWKnet hub checkpoint
~~~~~~~~~~~~~~~~~~~~~

QWKnet writes ``<hub-id>.state.toml`` in its outbound directory, beside the
pending ``<hub-id>.rep`` packet. Its sole field is optional ``[conferences]``, a
string-to-``u32`` map defaulting to empty. Keys are decimal **remote hub conference
numbers**, not local area names; values are message high-water marks. This choice
keeps renaming a local area from resetting the checkpoint. Existing pending
packets are kept for delivery rather than regenerated over the top of them.

.. code-block:: toml

   [conferences]
   "100" = 42
   "200" = 17

This example illustrates state, not an initialization recommendation. Deleting
pointers can cause previously exported messages to be offered again.

ZCONNECT per-link checkpoint
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

ZCONNECT outbound ``<link-id>/scan.toml`` is separate from FTN's same-named
file. It has:

* ``[committed]``: map from uppercase remote-board names (the area's
  ``remote_board`` value) to ``u32`` high-water marks, default empty.
* ``[pending]``: optional table. If present, all its fields are required:
  ``next`` is another string-to-``u32`` checkpoint map, ``messages`` is a
  ``usize`` count, and ``sha256`` is the pending archive's digest string.
* ``retired``: optional string holding the acknowledged archive digest while
  retirement is being completed.

The journal is kept with ``mail.zip`` and ``operation.lock`` in the link spool.
An acknowledgement is made durable before archive retirement; recovery completes
retirement instead of reoffering an acknowledged archive. The archive/checkpoint
digest must match. There is no journal version field. Do not transplant it between
links or edit pointers independently of the packet.

Files that are not TOML
-------------------------

Groups
~~~~~~

``paths.group_file`` uses ``GroupList``'s custom UTF-8 line parser, even when a
site or a test happens to name it ``groups.toml``. Its actual syntax is:

.. code-block:: text

   # A comment
   sysops:System Operators: Sysop
   users:Regular users: Example User, Second User
   empty:No members:

Each record is ``name:description: member, member`` followed by a newline.
Whitespace before fields/members is skipped and trailing member whitespace is
trimmed. A ``#`` before any name starts a whole-line comment, not an inline TOML
comment. There is no quoting/escape syntax for delimiters. Keep a final newline:
the parser commits a record on newline and does not flush an unterminated last
record. Membership lookup compares stored group/member strings exactly. There
are no ``[[group]]``, ``members = [...]`` or TOML defaults.

Other non-TOML or unimplemented formats
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* Native QWK/REP packets are ZIP archives. ``control.dat``/``messages.dat`` use
  QWK wire formats; ``NNN.ndx`` entries are binary BASIC-real block pointers plus
  a conference byte, not TOML indexes. The **QWKnet checkpoint** above is TOML;
  the packet indexes are not.
* JAM message-base files and last-read data use their binary formats. File-base
  metadata handled by dizbase is not a TOML catalog merely because the directory
  configuration is TOML.
* PCBoard ``USERS``/``USERS.INF``, PCBTEXT and legacy statistics imports are binary
  compatibility data. Exporting users to PCBoard does not write ``users.toml``.
* Group lists, FILES.BBS/PCBoard DIR listings, caller/event logs, FTN packet/TIC/
  request data, payload archives and lock files are not generic TOML documents.
* ``accounting.peak_holiday_list_file`` is plain text, one MM-DD-YY pattern per
  line, with uppercase X matching one digit (``12-25-XX`` means every Christmas).
  A matching local date suppresses peak rates for that whole day. Setup's
  conventional ``main/holidays.toml`` filename does **not** make it TOML.
* Accounting tracking selects dBase III for a case-insensitive .DBF extension;
  otherwise it writes fixed-width ASCII with CRLF. Fields are Date, Time, Name,
  NodeNumber, ConfNumber, Activity, SubAct, UnitCost, Quantity and Value. Audit
  amounts use four decimal places; controls become spaces and non-ASCII becomes
  ``?``. A persistent sibling .lock coordinates cooperating writers, not arbitrary
  DBF editors/PPE writes. Errors are logged without rolling back monetary posts.

The tool-file reference inventories audit ``manifest.toml``, ``audit.toml`` and
``baseline.toml`` outputs separately from live board state. No schema here is
inferred from a filename alone.