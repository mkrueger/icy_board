Main board configuration (icboard.toml)
===========================================

This is the on-disk reference for the main board configuration, as read by
``IcbConfig``. It covers every persisted field, including nested listener,
upload-processing, HTTP, and screen-color settings. Referenced files have
their own formats; a path in this file does not embed their contents.

Loading and defaults
--------------------

The file is UTF-8 TOML. Keys and enum strings are case-sensitive. There is no
``[icyboard]`` wrapper and no required format-version key. ``icyboard`` is
the serializer's diagnostic file-type name, not a TOML section.

**Required** in the tables means that omission causes a deserialization
error, even if the feature is disabled. **Missing** describes what happens
when that key is absent from a table that is present. **New** means the
value supplied by ``IcbConfig::new()`` / ``IcbConfig::default()``; setup and
import tools can subsequently replace it. New values are *not* generally
loading defaults: deriving or implementing Rust ``Default`` does not make
a field optional in Serde.

Only ``upload_processing``, ``ppl_http``, and ``qwk_settings`` may be omitted
as whole top-level tables. Each then receives its complete New values.
Every other top-level table below is required. An empty required table is
valid only when all its fields have loading defaults. Unknown keys are
currently ignored rather than rejected, so a misspelling can silently lose
a setting. Supplying both a canonical key and its alias is not supported.
Saving rewrites the representation and does not preserve comments or
unknown keys.

Types used in this reference:

.. list-table:: TOML value types
   :header-rows: 1
   :widths: 20 80

   * - Type
     - Representation and limits
   * - boolean
     - Unquoted ``true`` or ``false``.
   * - string / path
     - Quoted TOML string. Paths are strings, not tables. ``""`` is an empty
       path, not an instruction to use the New path.
   * - integer (u8 / u16 / u32)
     - Nonnegative integer, at most 255 / 65535 / 4294967295 respectively.
   * - integer (u64 / usize)
     - Nonnegative integer; Rust u64 or platform-sized unsigned capacity.
       Keep portable TOML values within signed 64-bit range
       (0 through 9223372036854775807), and within the target's ``usize``
       range where applicable.
   * - integer (i32 / i64)
     - Signed integer, within the corresponding 32-bit / 64-bit range.
   * - time
     - Native TOML local time, for example ``08:30:00``, **without quotes**.
       Midnight is ``00:00:00``. Do not substitute a date-only value:
       the conversion assumes a time component and can panic if absent.
   * - security expression
     - A TOML **string**, including numeric thresholds such as ``"10"``.
       See the security-expression details below.
   * - color
     - A TOML **string**, such as ``"@X1F"`` or ``"#aabbcc"``.
       See the color details below.

Ordinary board paths are resolved relative to the directory containing the
main configuration; absolute paths remain absolute and empty paths remain
empty. This is not shell expansion. Executable/command settings are
separate: notably the upload scanner is launched directly, with its own
argument array and ordinary process executable lookup. Prefer absolute
paths for external programs, TLS files, and other process-level resources
when launching the board from different working directories.

Top-level layout
----------------

.. list-table:: Root entries
   :header-rows: 1
   :widths: 29 20 51

   * - Key
     - Type / missing
     - Contents
   * - ``func_keys``
     - array of 10 strings; required
     - Function-key definitions F1 through F10, in order. New: ten empty
       strings. Put this before any table header to keep it at the root.
   * - ``board``, ``sysop``
     - tables; required
     - Board identity and operator settings.
   * - ``new_user_settings``, ``message``, ``file_transfer``
     - tables; required
     - Registration, messages, and transfers.
   * - ``upload_processing``
     - table; New values if absent
     - Publication, archive rewriting, quarantine, and scanner.
   * - ``system_control``, ``switches``, ``limits``, ``options``
     - tables; required
     - Operation switches, caller limits, and logging/page options.
   * - ``ppl_http``
     - table; New values if absent
     - HTTP boundary for PPE requests.
   * - ``event``, ``accounting``
     - tables; required
     - Event scheduler and accounting configuration references.
   * - ``qwk_settings``
     - table; New values if absent
     - QWK packet identity and capture ceilings.
   * - ``login_server``
     - table; required
     - Telnet, SSH, secure WebSocket, and modem definitions.
   * - ``sysop_sec``, ``user_sec``
     - tables; required
     - Security thresholds and expressions. These are the serialized names,
       not ``sysop_command_level`` or ``user_command_level``.
   * - ``paths``, ``colors``, ``subs``
     - tables; required
     - Paths, board display colors, and subscriptions. The serialized names
       are not ``color_configuration`` or ``subscription_info``.

Board and operator
------------------

.. list-table:: [board]
   :header-rows: 1
   :widths: 29 16 18 37

   * - Field
     - Type
     - Missing / New
     - Meaning
   * - ``name``
     - string
     - required / ``"IcyBoard"``
     - Board name.
   * - ``location``, ``operator``, ``notice``, ``capabilities``
     - string each
     - required / ``""`` each
     - Board location, operator, notice, and EMSI capability text.
   * - ``allow_iemsi``
     - boolean
     - false / true
     - Permit IEMSI login negotiation.
   * - ``date_format``
     - string
     - required / ``"%m/%d/%y"``
     - Local date-format string (strftime-style), not an enum.
   * - ``num_nodes``
     - integer (u16)
     - required / 4
     - Maximum active node count.
   * - ``who_include_city``, ``who_show_alias``
     - boolean each
     - false / true each
     - Include city and show aliases in WHO output.
   * - ``web_admin``
     - table
     - complete defaults / same
     - Optional nested administration listener, detailed below.

.. list-table:: [board.web_admin]
   :header-rows: 1
   :widths: 27 17 24 32

   * - Field
     - Type
     - Missing = New
     - Meaning
   * - ``enabled``
     - boolean
     - false
     - Start the web administration service.
   * - ``address``
     - string
     - ``"127.0.0.1"``
     - Bind address.
   * - ``port``
     - integer (u16)
     - 8787
     - TCP port.
   * - ``allow_remote``
     - boolean
     - false
     - Permit a non-loopback bind. This is not an authentication setting.

.. list-table:: [sysop]
   :header-rows: 1
   :widths: 29 17 24 30

   * - Field
     - Type
     - Missing / New
     - Meaning
   * - ``name``
     - string
     - required / ``"SYSOP"``
     - Operator display name.
   * - ``password``
     - password string
     - empty plaintext / same
     - Local operator password; see encoding below. Empty is omitted on save.
   * - ``require_password_to_exit``
     - boolean
     - false / false
     - Require local password to exit/drop to DOS. False is omitted on save.
   * - ``use_real_name``
     - boolean
     - false / false
     - Use the operator's real name rather than SYSOP. False is omitted on save.
   * - ``external_editor``
     - string
     - required / ``"nano"``
     - External text editor.
   * - ``graphics_editor``
     - string
     - ``"icy_draw"`` / same
     - External graphics editor.
   * - ``config_color_theme``
     - string
     - required / ``"DEFAULT1"``
     - Configuration UI theme selector; a string, not a Serde enum.
   * - ``config_color_configuration``
     - table
     - DEFAULT_1 palette / same
     - Custom configuration-screen attributes, detailed below.

Password encoding
~~~~~~~~~~~~~~~~~

``password`` is one TOML string, not a tagged table. After TOML unescaping:

* A value beginning ``bcrypt:`` stores the remainder as a bcrypt hash.
* A value beginning ``$argon2`` stores the entire Argon2 PHC hash string.
* A value whose first and last characters are double quotes stores the
  text *inside* those quotes as plaintext.
* Any other string is accepted as legacy plaintext, unchanged.

The writer adds those inner double quotes to plaintext. Thus the following
are equivalent TOML fragments showing the password **representation only**;
these are not recommended operator credentials or complete ``[sysop]`` tables:

.. code-block:: toml

   [sysop]
   password = '"example-only"'

.. code-block:: toml

   [sysop]
   password = "\"example-only\""

Do not paste a bare bcrypt hash without the ``bcrypt:`` prefix: it is read
as plaintext. Hash syntax is not verified by deserialization. The reader
also does not lowercase supplied plaintext; normal password verification
lowercases input. Use the administration tools to generate stored passwords.
``system_control.password_storage_method`` controls the storage policy and
does not change this string format. There is no distinct on-disk
``Protected`` password tag.

Configuration-screen palette
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``[sysop.config_color_configuration]`` has exactly one field: ``colors``, a
**required array of exactly 23 integers (u8)**. An absent whole table gets
DEFAULT_1; a present table with no ``colors`` does not. Each integer is a DOS
foreground/background attribute, not one of the ``IcbColor`` strings used
in ``[colors]``.

The array order is: Outer Box; Status Information; Headings and Screen Titles;
Menu Box; Menu Title; Menu Selections; Selected Menu Item; Menu Descriptions;
Unavailable Menu Item; Highlighted Unavailable Item; Questions; Answers;
Current Input Field; Display-only Fields; Special Instructions; Help Box;
Help Title; Help Subtitle; Help Text; Help Description; F1 Help Key;
Scroll Bar; Scroll Position.

Fragment containing the complete DEFAULT_1 nested table:

.. code-block:: toml

   [sysop.config_color_configuration]
   colors = [0x01, 0x03, 0x0C, 0x04, 0x0E, 0x0A, 0x3E, 0x4E,
             0x07, 0x30, 0x0A, 0x03, 0x4F, 0x07, 0x60, 0x20,
             0x2F, 0x2E, 0x20, 0x4F, 0x0F, 0x70, 0x0F]

New users
---------

Every field in ``[new_user_settings]`` is required except
``auto_register_conferences``. Grouped names below are separate keys of the
same type with the same New value, not an array.

.. list-table:: [new_user_settings]
   :header-rows: 1
   :widths: 39 15 15 31

   * - Field
     - Type
     - New
     - Meaning
   * - ``sec_level``
     - integer (u8)
     - 10
     - New-user security level.
   * - ``new_user_groups``
     - string
     - ``"new_users"``
     - New-user group assignment text, not a TOML array.
   * - ``allow_one_name_users``
     - boolean
     - false
     - Allow registration with one name.
   * - ``use_newask_and_builtin``
     - boolean
     - false
     - Ask the configured new-user survey in addition to built-in questions.
   * - ``ask_city_or_state``
     - boolean
     - true
     - Ask city/state.
   * - ``ask_address``, ``ask_verification``
     - boolean each
     - false each
     - Ask postal address and verification text.
   * - ``ask_business_phone``, ``ask_home_phone``, ``ask_comment``
     - boolean each
     - true each
     - Ask business/home telephone and user comment.
   * - ``ask_clr_msg``
     - boolean
     - true
     - Ask whether to clear the screen between messages.
   * - ``ask_xfer_protocol``, ``ask_fse``
     - boolean each
     - true each
     - Ask transfer protocol and full-screen editor preference.
   * - ``ask_date_format``
     - boolean
     - false
     - Ask date format.
   * - ``ask_alias``, ``ask_gender``, ``ask_birthdate``
     - boolean each
     - false each
     - Ask alias, gender, and birth date.
   * - ``ask_email``, ``ask_web_address``
     - boolean each
     - false each
     - Ask email and web address.
   * - ``ask_use_short_descr``
     - boolean
     - true
     - Ask short file-description preference.
   * - ``auto_register_conferences``
     - boolean
     - true
     - Missing: true. Register in accessible public conferences.

Messages
--------

.. list-table:: [message]
   :header-rows: 1
   :widths: 40 14 16 30

   * - Field
     - Type
     - Missing / New
     - Meaning
   * - ``max_msg_lines``
     - integer (u16)
     - required / 100
     - Maximum message lines.
   * - ``scan_all_mail_at_login``
     - boolean
     - required / true
     - Scan all mail on login.
   * - ``disable_message_scan_prompt``
     - boolean
     - required / true
     - Suppress the message-scan prompt.
   * - ``allow_esc_codes``
     - boolean
     - required / false
     - Allow ESC codes in messages.
   * - ``allow_carbon_copy``
     - boolean
     - required / true
     - Allow carbon copies.
   * - ``validate_to_name``
     - boolean
     - required / true
     - Validate the recipient name.
   * - ``default_quick_personal_scan``
     - boolean
     - required / true
     - Default quick personal scan setting.
   * - ``default_scan_all_selected_confs_at_login``
     - boolean
     - required / true
     - Default login scan across selected conferences.
   * - ``prompt_to_read_mail``
     - boolean
     - required / true
     - Offer to read waiting mail.
   * - ``force_comments_to_main``
     - boolean
     - false / false
     - Enter comments to the operator in the main board.
   * - ``update_last_read_pointer``
     - boolean
     - true / true
     - Advance the last-read pointer when reading a message.

File transfers and processing
-----------------------------

.. list-table:: [file_transfer]
   :header-rows: 1
   :widths: 34 15 18 33

   * - Field
     - Type
     - Missing / New
     - Meaning / unit
   * - ``disallow_batch_uploads``
     - boolean
     - required / false
     - Disallow batch uploads.
   * - ``promote_to_batch_transfers``
     - boolean
     - required / true
     - Promote eligible transfers to batch mode.
   * - ``upload_credit_time``
     - integer (u32)
     - required / 100
     - Time-credit rate in tenths: 10 is 1x, 100 is 10x; not a percentage.
   * - ``upload_credit_bytes``
     - integer (u32)
     - required / 0
     - Byte-credit rate in tenths: 10 is 1x; zero gives no byte credit.
   * - ``verify_files_uploaded``
     - boolean
     - required / true
     - Verify uploaded files.
   * - ``upload_descr_lines``
     - integer (u8)
     - required / 20
     - Upload-description line setting.
   * - ``display_uploader``
     - boolean
     - required / false
     - Display uploader identity.
   * - ``strip_colors_in_descriptions``
     - boolean
     - false / false
     - Strip colors from FILE_ID.DIZ descriptions.
   * - ``disable_drive_size_check``
     - boolean
     - required / false
     - Bypass the upload free-space check.
   * - ``stop_uploads_free_space``
     - integer (u32)
     - required / 1024
     - Minimum free KiB (1024-byte units); zero disables the threshold.

Byte credit is uploaded bytes multiplied by ``upload_credit_bytes``, divided
by 10. Time credit is uploaded bytes divided by measured characters/bytes
per second, then multiplied by ``upload_credit_time`` and divided by 10;
the result is seconds. A zero transfer rate gives zero time credit. Integer
division truncates; this is not the same as multiplying first in all cases.

.. list-table:: [upload_processing]
   :header-rows: 1
   :widths: 33 14 23 30

   * - Field
     - Type
     - Missing / New
     - Meaning / unit
   * - ``publish_policy``
     - string enum
     - ``"immediate"`` / same
     - ``"immediate"``, ``"after_processing"``, or ``"manual_approval"``.
   * - ``notify_sysop``
     - boolean
     - false / false
     - Notify the operator about received uploads.
   * - ``remove_advertisements``
     - boolean
     - false / false
     - Remove archive members and description blocks identified by rules.
   * - ``repack_to_zip``
     - boolean
     - false / false
     - Repack supported archives as ZIP; recompress existing ZIPs.
   * - ``advertisement_file_rules``
     - path
     - ``"upload_ad_files.toml"`` / same
     - Complete-member rules file. Empty disables this rule category.
   * - ``advertisement_description_rules``
     - path
     - ``"upload_ad_descriptions.toml"`` / same
     - Description-block rules file. Empty disables this rule category.
   * - ``quarantine_path``
     - path
     - ``""`` / ``"quarantine/uploads"``
     - Quarantine storage directory. **Different missing-field and New values.**
   * - ``advertisement_file``
     - path
     - ``""`` / ``""``
     - Empty disables insertion. Otherwise one static file (inserted under
       its basename), or a trusted generator when its extension is .ppe,
       case-insensitively. One literal path, not a shell command or list.
   * - ``archive_comment_mode``
     - string enum
     - ``"preserve"`` / same
     - ``"preserve"``, ``"remove"``, or ``"replace"``. Independent of ad removal.
   * - ``replacement_archive_comment``
     - string
     - ``""`` / ``""``
     - Exact UTF-8 replacement text in replace mode; empty clears the comment.
   * - ``compression_level``
     - integer (i64)
     - required / 9
     - ZIP compression level 0 through 9; range checked when repacking.
   * - ``max_members``
     - integer (usize)
     - required / 10000
     - Maximum archive member count.
   * - ``max_member_size``
     - integer (u64)
     - required / 536870912
     - Maximum expanded bytes per member (512 MiB).
   * - ``max_expanded_size``
     - integer (u64)
     - required / 2147483648
     - Maximum total expanded bytes (2 GiB).
   * - ``max_compression_ratio``
     - integer (u64)
     - required / 1000
     - Maximum per-member expanded/compressed byte ratio, dimensionless.
   * - ``scanner``
     - table
     - complete scanner defaults / same
     - Optional nested scanner settings.

``immediate`` publishes directly, without the quarantine processing path;
setting scanner/repack switches alone does not turn it into a processed
publication. ``after_processing`` uses processing before automatic
publication; ``manual_approval`` leaves approval to the operator. Processing
failures require review instead of publishing the failed result. For
``preserve``, source ZIP comments retain their raw bytes; comments from
other archive formats are not carried over by the archive reader.

An absent ``[upload_processing]`` is backward-compatible. A present empty
table is **not**: its five required numeric fields are still mandatory.
Likewise, omitting the whole scanner gets its default executable and
arguments, whereas adding a partial scanner table has the rules below.

.. list-table:: [upload_processing.scanner]
   :header-rows: 1
   :widths: 25 18 25 32

   * - Field
     - Type
     - Missing / New
     - Meaning
   * - ``enabled``
     - boolean
     - false / false
     - Run the external scanner.
   * - ``executable``
     - path
     - required / ``"clamscan"``
     - Program to execute directly, not through a shell.
   * - ``arguments``
     - array of strings
     - ``[]`` / ``["--no-summary", "{file}"]``
     - Must contain a standalone ``"{file}"`` element when scanning.
       Only that exact element is replaced with the payload path.
   * - ``timeout_seconds``
     - integer (u64)
     - required / 120
     - Timeout in seconds, including output draining; zero is not unlimited.
   * - ``clean_exit_code``
     - integer (i32)
     - required / 0
     - Scanner exit code indicating a clean file.
   * - ``infected_exit_code``
     - integer (i32)
     - required / 1
     - Scanner exit code indicating infection. Other codes are failures.

Fragment with a complete processing configuration and an enabled scanner
(the executable must also be installed):

.. code-block:: toml

   [upload_processing]
   publish_policy = "after_processing"
   quarantine_path = "quarantine/uploads"
   compression_level = 9
   max_members = 10000
   max_member_size = 536870912
   max_expanded_size = 2147483648
   max_compression_ratio = 1000

   [upload_processing.scanner]
   enabled = true
   executable = "clamscan"
   arguments = ["--no-summary", "{file}"]
   timeout_seconds = 120
   clean_exit_code = 0
   infected_exit_code = 1

System controls, switches, and limits
------------------------------------------

All fields of ``[system_control]`` below are booleans with New value false,
except ``password_storage_method``. The Missing column distinguishes the
required older controls from optional newer controls.

.. list-table:: [system_control]
   :header-rows: 1
   :widths: 39 15 46

   * - Field
     - Missing
     - Meaning
   * - ``disable_ns_logon``
     - required
     - Disable the non-stop logon option.
   * - ``disable_full_record_updating``
     - required
     - Restrict W-command record changes to passwords.
   * - ``allow_alias_change``
     - required
     - Allow alias changes.
   * - ``is_multi_lingual``
     - required
     - Enable multiple languages.
   * - ``is_closed_board``
     - required
     - Closed-board / NewAsk operation.
   * - ``enforce_daily_time_limit``
     - required
     - Enforce daily rather than session time limits.
   * - ``allow_password_failure_comment``
     - required
     - Allow a comment after password failure.
   * - ``guard_logoff``
     - required
     - Confirm G-command logoff; BYE bypasses confirmation.
   * - ``password_storage_method``
     - ``"bcrypt"``
     - String enum: ``"bcrypt"`` (New), ``"argon2"``, or ``"plain"``.
       Plaintext storage is for legacy compatibility, not recommended.
   * - ``confirm_caller_name``
     - false
     - Confirm the matched caller name during login.
   * - ``reread_sec_level_on_join``
     - false
     - Re-read level limits after a conference changes the caller's level.
   * - ``enforce_transfer_limits``
     - false
     - Enforce ratios, daily allowances, and total download limits.

.. list-table:: [switches]
   :header-rows: 1
   :widths: 36 18 16 30

   * - Field
     - Type
     - Missing / New
     - Meaning
   * - ``default_graphics_at_login``
     - boolean
     - false / true
     - Default to graphics on login.
   * - ``non_graphics``
     - boolean
     - false / false
     - Disable graphics/colors.
   * - ``exclude_local_calls_stats``
     - boolean
     - false / true
     - Exclude local sessions from statistics.
   * - ``display_news_behavior``
     - string enum
     - required / ``"Y"``
     - ``"Y"``: only newer news; ``"N"``: once per day;
       ``"A"``: always; ``"X"``: never at login.
   * - ``disable_registration_edits``
     - boolean
     - false / false
     - Disable automatic filtering of logon-prompt input.
   * - ``disable_high_ascii_filter``
     - boolean
     - false / false
     - Disable the high-ASCII input filter.
   * - ``display_userinfo_at_login``
     - boolean
     - false / false
     - Display user information on login.
   * - ``force_intro_on_join``
     - boolean
     - false / false
     - Force conference introduction display on joining.
   * - ``scan_new_blt``
     - boolean
     - false / true
     - Scan for new bulletins.
   * - ``capture_grp_chat_session``
     - boolean
     - false / false
     - Capture group-chat sessions.
   * - ``allow_handle_in_grpchat``
     - boolean
     - false / false
     - Allow handles in group chat.

.. list-table:: [limits]
   :header-rows: 1
   :widths: 37 17 16 30

   * - Field
     - Type
     - Missing / New
     - Meaning / unit
   * - ``keyboard_timeout``
     - integer (u16)
     - 0 / 5
     - Remote inactivity timeout in minutes; zero disables it.
   * - ``max_number_upload_descr_lines``
     - integer (u16)
     - 0 / 20
     - Maximum upload-description lines; distinct from
       ``file_transfer.upload_descr_lines``.
   * - ``min_pwd_length``
     - integer (u8)
     - 0 / 0
     - Minimum password length; zero disables the minimum.
   * - ``password_expire_days``
     - integer (u16)
     - 0 / 0
     - Password validity in days; zero disables expiry.
   * - ``password_expire_warn_days``
     - integer (u16)
     - 0 / 0
     - Days before expiry to warn.
   * - ``sysop_start``, ``sysop_stop``
     - time each
     - ``00:00:00`` / same
     - Start/end of permitted operator paging times, local time.

All zero numeric values and midnight times in ``[limits]`` are omitted on
save. The table itself remains required; empty is valid.

All ``[options]`` fields are optional within the required table:

.. list-table:: [options]
   :header-rows: 1
   :widths: 33 17 18 32

   * - Field
     - Type
     - Missing / New
     - Meaning
   * - ``give_user_password_to_doors``
     - boolean
     - false / false
     - Permit passing user passwords to doors.
   * - ``call_log``
     - boolean
     - false / true
     - Enable caller logging.
   * - ``page_bell``
     - boolean
     - false / true
     - Sound the operator paging bell.
   * - ``page_notification_command``
     - string
     - ``""`` / ``""``
     - Optional shell command when paging begins; receives
       ``ICB_PAGE_NODE`` and ``ICB_PAGE_USER`` environment variables.
   * - ``alarm``
     - boolean
     - false / false
     - Operator alarm switch.
   * - ``log_caller_number``
     - boolean
     - false / false
     - Include session caller number in the caller log.
   * - ``log_connect_string``
     - boolean
     - false / false
     - Include connection information in the caller log.
   * - ``log_security_level``
     - boolean
     - false / false
     - Include caller security level in the caller log.

PPE HTTP policy
---------------

Every key in ``[ppl_http]`` is optional. Missing and New values are identical,
whether the table is absent or present. These limits apply to PPE HTTP
requests, not to the web administration listener.

.. list-table:: [ppl_http]
   :header-rows: 1
   :widths: 30 17 18 35

   * - Field
     - Type
     - Missing = New
     - Meaning / unit
   * - ``destination_policy``
     - string enum
     - ``"public"``
     - ``"disabled"``: deny requests; ``"allowlist"``: restrict to
       allowed origins; ``"public"``: public destinations.
   * - ``allowed_origins``
     - array of strings
     - ``[]``
     - Exact HTTP(S) origins, e.g. ``["https://example.org:8443"]``.
       No credentials, path other than /, query, fragment, or wildcard.
   * - ``max_response_bytes``
     - integer (usize)
     - 16777216
     - Maximum response body bytes (16 MiB).
   * - ``max_request_bytes``
     - integer (usize)
     - 1048576
     - Maximum request body bytes (1 MiB).
   * - ``connect_timeout_seconds``
     - integer (u64)
     - 5
     - Connection timeout in seconds.
   * - ``request_timeout_seconds``
     - integer (u64)
     - 30
     - Request timeout in seconds.
   * - ``max_redirects``
     - integer (usize)
     - 3
     - Maximum redirect count.
   * - ``max_concurrent_requests``
     - integer (usize)
     - 16
     - Board-wide concurrent requests.
   * - ``max_concurrent_per_node``
     - integer (usize)
     - 2
     - Concurrent requests per node.
   * - ``max_headers``
     - integer (usize)
     - 64
     - Header-count ceiling.
   * - ``max_header_bytes``
     - integer (usize)
     - 65536
     - Header-size ceiling in bytes (64 KiB).
   * - ``allow_http``
     - boolean
     - true
     - Allow unencrypted HTTP in addition to HTTPS.

The runtime clamps both timeout settings and both concurrency settings to
at least 1; zero does not disable HTTP or mean unlimited. The request
timeout also includes waiting for a concurrency slot. ``max_headers`` and
``max_header_bytes`` constrain both request and response headers; the
HTTP/2 header-list setting additionally caps the byte value at u32 maximum.
``public`` rejects a hostname if any resolved address is non-public.
``allowlist`` can explicitly permit private/local services, so treat changes
to it as security-sensitive. Invalid origin strings are not rejected by
Serde but cannot match at runtime. Each redirect destination is checked.

Events, accounting, and subscriptions
------------------------------------------

All fields in the required ``[event]`` table are optional; New and Missing
values are identical.

.. list-table:: [event]
   :header-rows: 1
   :widths: 34 17 15 34

   * - Field
     - Type
     - Missing = New
     - Meaning
   * - ``enabled``
     - boolean
     - false
     - Enable scheduled events.
   * - ``event_file``
     - path
     - ``""``
     - Event definitions file. Read alias: ``event_dat_path``;
       saves use ``event_file``.
   * - ``suspend_minutes``
     - integer (u16)
     - 0
     - Pre-event suspension window, minutes.
   * - ``disallow_uploads``
     - boolean
     - false
     - Disallow uploads in the configured pre-event window.
   * - ``minutes_uploads_disallowed``
     - integer (u16)
     - 0
     - Upload restriction window, minutes before an event.

All fields in the required ``[accounting]`` table are optional; New and
Missing values are identical. The loaded runtime ``accounting_config`` is
explicitly skipped by Serde: do not put an accounting configuration object
under that key. ``cfg_file`` references its separate file.

.. list-table:: [accounting]
   :header-rows: 1
   :widths: 33 17 18 32

   * - Field
     - Type
     - Missing = New
     - Meaning
   * - ``enabled``
     - boolean
     - false
     - Enable accounting.
   * - ``use_money``
     - boolean
     - false
     - Use monetary units instead of accounting credits.
   * - ``concurrent_tracking``
     - boolean
     - false
     - Concurrent tracking switch.
   * - ``ignore_empty_sec_level``
     - boolean
     - false
     - Ignore empty security-level accounting entries.
   * - ``peak_usage_start``, ``peak_usage_end``
     - time each
     - ``00:00:00`` each
     - Local peak-period start and end.
   * - ``peak_days_of_week``
     - string
     - ``"NNNNNNN"``
     - Seven Y/N characters, Sunday first through Saturday last.
       Uppercase Y selects a day. ``"NYYYYYN"`` selects Monday-Friday.
   * - ``peak_holiday_list_file``
     - path
     - ``""``
     - Holiday list.
   * - ``cfg_file``
     - path
     - ``""``
     - Accounting rates/configuration file.
   * - ``tracking_file``
     - path
     - ``""``
     - Accounting tracking file.
   * - ``info_file``, ``warning_file``, ``logoff_file``
     - path each
     - ``""`` each
     - Accounting information, warning, and logoff displays.

The day-mask parser does not validate seven-character Y/N input: only
uppercase Y sets bits, and excess characters are unsafe. Always supply
exactly seven characters. This is not an integer mask or array of weekdays.

.. list-table:: [subs]
   :header-rows: 1
   :widths: 30 17 20 33

   * - Field
     - Type
     - Missing / New
     - Meaning
   * - ``is_enabled``
     - boolean
     - false / false
     - Enable subscription mode.
   * - ``subscription_length``
     - integer (u32)
     - 0 / 365
     - New subscription period in days.
   * - ``default_expired_level``
     - integer (u8)
     - 0 / 10
     - Security level after expiration.
   * - ``warning_days``
     - integer (u32)
     - 0 / 30
     - Days before expiration to warn.

False and zero values in ``[subs]`` are omitted on save; the table itself
is required. An empty table therefore does not restore a 365-day period.

QWK settings
------------

Omitting ``[qwk_settings]`` supplies all New values. If the table is present,
its identity and screen paths are mandatory, even when empty.

.. list-table:: [qwk_settings]
   :header-rows: 1
   :widths: 35 16 19 30

   * - Field
     - Type
     - Missing / New
     - Meaning
   * - ``bbs_name``
     - string
     - required / ``""``
     - Packet BBS name.
   * - ``bbs_city_and_state``
     - string
     - required / ``""``
     - Packet BBS location.
   * - ``bbs_phone_number``
     - string
     - required / ``""``
     - Packet contact telephone.
   * - ``bbs_sysop_name``
     - string
     - required / ``""``
     - Packet operator name.
   * - ``bbs_id``
     - string
     - required / ``""``
     - Packet BBS identifier.
   * - ``welcome_screen``, ``goodbye_screen``
     - path each
     - required / ``""`` each
     - QWK welcome and goodbye screens.
   * - ``news_sceen``
     - path
     - required / ``""``
     - QWK news screen. The spelling **sceen** is the actual serialized key;
       ``news_screen`` is not an alias.
   * - ``max_msgs``
     - integer (u16)
     - 600 / 600
     - System ceiling on a user's total captured message count.
   * - ``max_msgs_per_conf``
     - integer (u16)
     - 200 / 200
     - Per-conference message capture ceiling.

Login servers
-------------

``[login_server]`` requires ``telnet``, ``ssh``, ``secure_websocket``, and
``modems``. The first three are nested tables; ``modems`` is an array of
tables (empty ``modems = []`` is valid). There is **no persisted**
``login_server.websocket`` field, despite the standalone Websocket Rust
type. Do not use a plain WebSocket section as a substitute for
``secure_websocket``.

The next table applies separately to ``[login_server.telnet]``,
``[login_server.ssh]``, and ``[login_server.secure_websocket]``:

.. list-table:: Common listener fields
   :header-rows: 1
   :widths: 22 16 16 46

   * - Field
     - Type
     - Missing
     - New / meaning
   * - ``is_enabled``
     - boolean
     - required
     - Telnet: true; SSH and secure WebSocket: false.
   * - ``port``
     - integer (u16)
     - required
     - Telnet: 23; SSH: 22; secure WebSocket: 8811. TCP port number.
   * - ``address``
     - string
     - ``""``
     - New: ``""``. Bind address; empty uses the listener's wildcard
       ``0.0.0.0`` binding, not loopback-only. Empty is omitted on save.
   * - ``display_file``
     - path
     - ``""``
     - New: ``""``. Connection-specific display file; empty is omitted on save.

Additionally, ``[login_server.secure_websocket]`` requires both ``cert_pem``
and ``key_pem`` (path strings; New ``""`` each), intended to name the TLS
certificate and private-key PEM files. They are required **even when disabled**.

.. warning::

  These PEM paths are persisted but currently unused by the listener. The
  secure-WebSocket connection handler attempts a client-side TLS/WebSocket
  handshake instead of a server-side handshake. Supplying valid certificates
  does not make this listener a working secure-WebSocket server. Leave it
  disabled until server-side TLS acceptance is implemented.

Each ``[[login_server.modems]]`` entry has the following required fields.
The Modem Rust type has empty/zero defaults, but those do not make any
field optional. A new board has no entries.

.. list-table:: Modem entry
   :header-rows: 1
   :widths: 31 20 49

   * - Field
     - Type
     - Meaning (Rust Modem default)
   * - ``device``
     - string
     - Serial device (``""``).
   * - ``baud_rate``
     - integer (usize)
     - Serial baud rate (0).
   * - ``init_string1``, ``init_string2``
     - string each
     - First and second modem initialization strings (``""`` each).
   * - ``answer_string``
     - string
     - Answer command (``""``).
   * - ``dialout_string``
     - string
     - Dial-out command (``""``).

Fragment containing the complete login-server section, using a loopback
Telnet endpoint for a local test:

.. code-block:: toml

   [login_server]
   modems = []

   [login_server.telnet]
   is_enabled = true
   address = "127.0.0.1"
   port = 2323

   [login_server.ssh]
   is_enabled = false
   port = 22

   [login_server.secure_websocket]
   is_enabled = false
   port = 8811
   cert_pem = ""
   key_pem = ""

Security configuration
----------------------

Security-expression representation
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

With the sole exception of the numeric ``sysop_sec.sysop`` field, every
security key below is a **required string**, not an integer. For example,
``cmd_d = "50"`` requires level 50 or above. ``"0"`` permits any security
level; ``"true"`` and ``"false"`` give unconditional allow/deny. Use levels
0 through 255: the runtime casts integer results to u8.

The parser supports parentheses, unary ``!``, ``&``, ``|``, and the
comparisons ``<``, ``<=``, ``>``, ``>=``. Parenthesize mixed boolean
expressions explicitly. Function names are evaluated case-insensitively:
``U_SEC()`` returns security level, ``U_AGE()`` age in years,
``U_GROUP("name")`` group membership, ``TIME()`` local time,
``TIME_LEFT()`` remaining minutes, and ``DOW()`` day number (Sunday 0 through
Saturday 6). String literals
inside expressions are restricted to nonempty letters, digits, and
underscores. Time literals in expressions are ``HH:MM``; these are part of
the surrounding string, unlike the native TOML times in ``[limits]``.

.. warning::

   Security-expression deserialization is permissive, not a security
   validator. Parse errors fall back to ``"0"`` (open access), and parsing
   can ignore trailing tokens. Although equality operators exist in the
   expression model/tokenizer, the current comparison parser does not
   consume ``==`` or ``!=``; do not rely on them in configuration. The
   expression writer also formats time constants with seconds, unlike the
   parser's minute-only token. Recheck nontrivial expressions after a save
   and reload. A malformed string need not make the configuration load fail.

Fragment (other required security fields deliberately omitted):

.. code-block:: toml

   [user_sec]
   cmd_d = "50"
   cmd_e = "U_SEC() >= 10"
   cmd_open_door = '(U_SEC() >= 20) & U_GROUP("members")'

Operator security
~~~~~~~~~~~~~~~~~

All fields in ``[sysop_sec]`` are required. ``sysop`` is integer (u8), New
100, the operator security level. **Every other key** in the following
table is a security-expression string with New value ``"110"``.

.. list-table:: [sysop_sec] expression fields
   :header-rows: 1
   :widths: 54 46

   * - Field
     - Permission
   * - ``read_all_comments``
     - Read all operator comments.
   * - ``read_all_mail``
     - Read all mail.
   * - ``copy_move_messages``
     - Copy or move messages.
   * - ``enter_color_codes_in_messages``
     - Enter message color codes.
   * - ``edit_any_message``
     - Edit any message.
   * - ``not_update_msg_read``
     - Avoid updating message-read state.
   * - ``use_broadcast_command``
     - Use broadcast.
   * - ``view_private_uploads``
     - View private uploads.
   * - ``enter_generic_messages``
     - Enter generic messages.
   * - ``edit_message_headers``
     - Edit message headers.
   * - ``protect_unprotect_messages``
     - Protect/unprotect messages.
   * - ``overwrite_files_on_uploads``
     - Overwrite existing files during upload.
   * - ``set_pack_out_date_on_messages``
     - Set message pack-out date.
   * - ``see_all_return_receipts``
     - See all return receipts.
   * - ``sec_1_view_caller_log``
     - Operator command 1: caller log.
   * - ``sec_2_view_usr_list``
     - Command 2: user list.
   * - ``sec_3_pack_renumber_msg``
     - Command 3: pack/renumber messages.
   * - ``sec_4_recover_deleted_msg``
     - Command 4: recover deleted messages.
   * - ``sec_5_list_message_hdr``
     - Command 5: message headers.
   * - ``sec_6_view_any_file``
     - Command 6: view files; also directory command access.
   * - ``sec_7_user_maint``
     - Command 7: user maintenance.
   * - ``sec_8_pack_usr_file``
     - Command 8: pack users.
   * - ``sec_9_exit_to_dos``
     - Command 9: exit to DOS threshold.
   * - ``sec_10_shelled_dos_func``
     - Command 10: shelled functions; used for RunPPE access.
   * - ``sec_11_view_other_nodes``
     - Command 11: node list.
   * - ``sec_12_logoff_alt_node``
     - Command 12: log off another node.
   * - ``sec_13_view_alt_node_callers``
     - Command 13: another node's caller log.
   * - ``sec_14_drop_alt_node_to_dos``
     - Command 14: drop another node to DOS threshold.

These are persisted permission settings; their presence does not imply that
every historical DOS operation is implemented on every host.

Caller security
~~~~~~~~~~~~~~~

Every field in ``[user_sec]`` is a **required security-expression string**
with New value ``"10"``. There is no persisted ``cmd_g`` field.

.. list-table:: [user_sec]
   :header-rows: 1
   :widths: 37 63

   * - Field
     - Command/permission
   * - ``cmd_a``
     - Abandon conference.
   * - ``cmd_b``
     - Bulletin list.
   * - ``cmd_c``
     - Comment to operator.
   * - ``cmd_d``
     - Download, flag files, and batch download command access.
   * - ``cmd_e``
     - Enter/reply to messages and write email.
   * - ``cmd_f``
     - File directories.
   * - ``cmd_h``
     - Help.
   * - ``cmd_i``
     - Initial welcome.
   * - ``cmd_j``
     - Join/select conferences and change message area.
   * - ``cmd_k``
     - Delete messages.
   * - ``cmd_l``
     - Locate files.
   * - ``cmd_m``
     - Toggle graphics.
   * - ``cmd_n``
     - New-file scan.
   * - ``cmd_o``
     - Page operator.
   * - ``cmd_p``
     - Page length.
   * - ``cmd_q``
     - Quick message scan.
   * - ``cmd_r``
     - Read messages/email, text search, QWK, and memorized messages.
   * - ``cmd_s``
     - Surveys.
   * - ``cmd_t``
     - Transfer protocol.
   * - ``cmd_u``
     - Upload and batch upload command access.
   * - ``cmd_v``
     - View settings.
   * - ``cmd_w``
     - Write settings.
   * - ``cmd_x``
     - Expert mode.
   * - ``cmd_y``
     - Your-mail scan.
   * - ``cmd_z``
     - Zippy directory scan.
   * - ``cmd_chat``
     - Group chat.
   * - ``cmd_open_door``
     - Open a door.
   * - ``cmd_test_file``
     - Test a file.
   * - ``cmd_show_user_list``
     - User list.
   * - ``cmd_who``
     - Who is online.
   * - ``batch_file_transfer``
     - Batch-mode eligibility, not the command-level download/upload gate.
   * - ``edit_own_messages``
     - Edit own messages.

Paths
-----

Every ``[paths]`` field is a TOML path string. All are required **except**
``ftn_file``, ``qwknet_file``, ``zconnect_file``, and ``transfer_log``, whose
missing-field value is ``""``. Empty strings must still be written for
required unused paths. The New column is the constructor value, not a
fallback for missing required paths.

.. list-table:: [paths]
   :header-rows: 1
   :widths: 29 30 41

   * - Field
     - New
     - Target
   * - ``help_path``
     - ``"art/help/"``
     - Help-file directory.
   * - ``security_file_path``
     - ``"art/secmsgs/"``
     - Security-level login display directory.
   * - ``email_msgbase``
     - ``"main/email"``
     - Private email message-base location.
   * - ``command_display_path``
     - ``"art/cmd_display/"``
     - Pre-command displays, named after commands.
   * - ``tmp_work_path``
     - ``"tmp/"``
     - Temporary working directory.
   * - ``icbtext``
     - ``"main/icbtext.toml"``
     - Board prompt/text definitions.
   * - ``conferences``
     - ``"main/conferences.toml"``
     - Conference configuration.
   * - ``welcome``, ``newuser``, ``closed``
     - ``""`` each
     - Welcome, new-user, and closed-board displays.
   * - ``expire_warning``, ``expired``
     - ``""`` each
     - Subscription warning and expired displays.
   * - ``conf_join_menu``
     - ``""``
     - Conference join menu.
   * - ``chat_intro_file``, ``chat_menu``, ``chat_actions_menu``
     - ``""`` each
     - Chat introduction, menu, and actions menu.
   * - ``no_ansi``
     - ``""``
     - Non-ANSI warning display.
   * - ``trashcan_upload_files``
     - ``""``
     - Rejected upload-name patterns.
   * - ``trashcan_user``, ``trashcan_email``, ``trashcan_passwords``
     - ``""`` each
     - Rejected users, email addresses, and passwords.
   * - ``vip_users``
     - ``""``
     - VIP user list.
   * - ``protocol_data_file``
     - ``""``
     - Transfer protocol definitions.
   * - ``pwrd_sec_level_file``
     - ``""``
     - Security-level definitions and limits.
   * - ``command_file``
     - ``""``
     - Board commands.
   * - ``statistics_file``
     - ``""``
     - Statistics data.
   * - ``language_file``
     - ``""``
     - Language definitions.
   * - ``group_file``
     - ``""``
     - User group definitions.
   * - ``ftn_file``
     - ``""``
     - FTN network configuration; optional key.
   * - ``qwknet_file``
     - ``""``
     - QWK network configuration; optional key, distinct from qwk_settings.
   * - ``zconnect_file``
     - ``""``
     - ZCONNECT network configuration; optional key.
   * - ``user_file``
     - ``"main/users.toml"``
     - User base location.
   * - ``caller_log``
     - ``"caller.log"``
     - Caller log.
   * - ``transfer_log``
     - ``"transfer.log"``
     - Completed-transfer log; optional key, Missing is empty, not this New path.
   * - ``logon_survey``, ``logon_answer``
     - ``""`` each
     - Logon survey and answer output.
   * - ``logoff_survey``, ``logoff_answer``
     - ``""`` each
     - Logoff survey and answer output.
   * - ``newask_survey``, ``newask_answer``
     - ``""`` each
     - New-user survey and answer output.

Board display colors
--------------------

All ``[colors]`` fields are required ``IcbColor`` strings. Canonical DOS
attributes are ``"@X00"`` through ``"@XFF"``, combining foreground and
background bits. RGB colors accept exactly six hexadecimal digits with
an optional leading # (for example ``"aabbcc"`` or ``"#AABBCC"``); the
writer emits lowercase ``"#aabbcc"``. Integers, RGB arrays/tables, short
``#abc`` colors, and names such as ``"red"`` are not this format.

.. warning::

   The color reader uses unchecked conversions. Invalid strings can panic
   instead of returning a normal configuration error. In particular, the
   internal ``None`` color serializes as ``""`` but the reader does not
   accept that empty value. Use a valid explicit color, not an empty
   string, and use canonical ``@X`` prefixes rather than relying on the
   reader's permissive prefix handling.

.. list-table:: [colors] (all strings, all required)
   :header-rows: 1
   :widths: 30 18 52

   * - Field
     - New
     - Display role
   * - ``default``
     - ``"@X07"``
     - Default board color.
   * - ``msg_hdr_date``
     - ``"@X1F"``
     - Message DATE header.
   * - ``msg_hdr_to``
     - ``"@X3F"``
     - Message TO header.
   * - ``msg_hdr_from``
     - ``"@X3F"``
     - Message FROM header.
   * - ``msg_hdr_subj``
     - ``"@X3F"``
     - Message subject header.
   * - ``msg_hdr_read``
     - ``"@X3E"``
     - Message READ header.
   * - ``msg_hdr_conf``
     - ``"@X3E"``
     - Message conference header.
   * - ``file_name``
     - ``"@X0E"``
     - Filename.
   * - ``file_size``
     - ``"@X02"``
     - File size.
   * - ``file_date``
     - ``"@X04"``
     - File date.
   * - ``file_description``
     - ``"@X0B"``
     - File description.
   * - ``file_head``
     - ``"@X06"``
     - File-list header.
   * - ``file_text``
     - ``"@X06"``
     - File-list text.
   * - ``file_duplicate``
     - ``"@X03"``
     - Duplicate file. Read alias: ``file_description_low``;
       saves use ``file_duplicate``.
   * - ``file_deleted``
     - ``"@X0F"``
     - Deleted file.
   * - ``file_offline``
     - ``"@X05"``
     - Offline file.
   * - ``file_new_file``
     - ``"@X8F"``
     - New-file indicator.

Validation checklist
--------------------

* Start from a generated complete configuration. The examples here are
  **fragments**, not minimal whole-board files. Valid TOML syntax alone
  does not satisfy required Serde fields or create referenced files.
* Preserve root/table scope: root ``func_keys`` must not accidentally
  become a key in the last opened table; ``modems = []`` belongs directly
  to ``[login_server]``.
* Distinguish absent parent tables from missing children, especially for
  ``upload_processing``, its ``scanner``, ``qwk_settings``, and the
  configuration-screen palette.
* Check spelling, especially ``news_sceen``, ``sysop_sec``, ``user_sec``,
  ``subs``, and ``secure_websocket``. Unknown settings can be ignored.
* Validate expressions for actual access behavior and after round trips;
  parser errors can grant access. Use valid color strings and native TOML
  local times to avoid unchecked conversion failures.
* Required fields remain required for disabled listeners and scanners.
  Runtime validation additionally needs usable files, available bind
  addresses/ports, scanner arguments and exit codes,
  supported compression levels, and valid HTTP destinations. Deserializing
  a value is not proof that the corresponding feature can operate; in
  particular, the secure-WebSocket listener has the limitation above.