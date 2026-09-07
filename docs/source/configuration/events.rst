Event configuration and schedule format
=======================================

Timed events have two configuration layers: the board's ``[event]`` options
and a separate schedule containing ``[[event]]`` records. They are not the
same table. This page specifies all persisted options, schedule fields and
the generated history format, including the custom TOML validation.

Board options: ``[event]``
--------------------------------------------------------------------------------

The board configuration requires the ``event`` table during deserialization;
the individual fields inside it may all be omitted. Thus ``[event]`` alone
is valid as part of an otherwise complete board file, but omitting the whole
table is not equivalent. Merge this fragment into that file:

.. code-block:: toml

   [event]
   enabled = true
   event_file = "main/events.toml"
   suspend_minutes = 5
   disallow_uploads = true
   minutes_uploads_disallowed = 10

.. list-table:: All global event options
   :header-rows: 1
   :widths: 28 22 50

   * - Key
     - Type; omitted value
     - Meaning
   * - ``enabled``
     - Boolean; ``false``
     - Enable automatic scheduling. Does not prevent schedule loading or confirmed manual runs.
   * - ``event_file``
     - Path string; ``""``
     - Separate schedule filename; relative to board root. Empty means an empty schedule.
   * - ``suspend_minutes``
     - ``u16``; ``0``
     - Minutes before a maintenance/fixed start to close admission and suspend callers.
   * - ``disallow_uploads``
     - Boolean; ``false``
     - Enable the maintenance/fixed advance upload restriction.
   * - ``minutes_uploads_disallowed``
     - ``u16``; ``0``
     - Minutes before that start to refuse new uploads, when the switch is true.

``event_dat_path`` is an accepted input alias for ``event_file``; saves use
``event_file``. Do not supply both names in one table. There is no
``paths.events_file`` setting. The constructor also starts with an empty
event path; a setup-created board may explicitly assign ``main/events.toml``.
That setup choice is not a serde default.

Both minute counts accept integers 0--65,535, not negative values, floats or
quoted numbers. Zero means the scheduled start itself, not an unlimited
advance window or a global upload ban. Restrictions count back from each
occurrence, possibly into the preceding day. They apply only to
``execution = "maintenance"`` with ``mode = "fixed"``: not to Slide, Idle
or Online. Suspension also caps session time for callers arriving shortly
before the event; already-running transfers and managed writers must drain
cooperatively.

Unlike the schedule below, ``EventOptions`` does not reject unknown fields.
A misspelled option may silently keep its default. At board load a missing
or invalid nonempty schedule file is logged and replaced by an empty schedule
in memory; it is not a successful schedule load. A later save can overwrite
the damaged file. Preserve a copy before editing or saving after such errors.

Separate schedule file
----------------------

The only allowed root key is ``event``, an array of event tables. It defaults
to an empty array, so an empty file and the following are equivalent:

.. code-block:: toml

   event = []

For a populated schedule, repeat ``[[event]]``. Do not wrap the records in
``[events]`` or use ``[[events]]``. Unknown root keys and unknown keys in any
event record are rejected. Duplicate IDs reject the entire list, including
disabled entries. Validation runs on deserialization and on schedule save.

.. list-table:: Every persisted ``[[event]]`` field
   :header-rows: 1
   :widths: 25 25 50

   * - Key
     - Type; omitted value
     - Meaning and allowed values
   * - ``id``
     - String; deterministic legacy ID
     - Stable identity for scheduling/history. Explicit IDs must have 1--128 ASCII letters, digits, hyphens or underscores and be unique in the list.
   * - ``description``
     - String; ``""``
     - Human-readable label; no scheduling meaning or uniqueness requirement.
   * - ``enabled``
     - Boolean; ``true``
     - Per-record automatic scheduling switch, in addition to the global switch.
   * - ``time``
     - TOML local time; ``00:00:00``
     - Start on selected local weekdays, with whole-second precision.
   * - ``days``
     - String; ``"YYYYYYY"``
     - Exactly seven uppercase ``Y``/``N`` bytes, Sunday first.
   * - ``mode``
     - String enum; ``"fixed"``
     - Exactly ``"fixed"``, ``"slide"`` or ``"idle"``. Caller policy, not execution type.
   * - ``command``
     - String; ``""``
     - Foreground shell command, with board root as working directory.
   * - ``end_time``
     - Optional TOML local time; absent
     - Inclusive latest start on the same calendar day; must be at or after ``time``.
   * - ``interval_minutes``
     - Optional ``u32``; absent
     - Positive start-anchored recurrence interval in wall-clock minutes; 1--4,294,967,295.
   * - ``warning_minutes``
     - Optional ``u32``; absent
     - Positive elapsed-command warning threshold in minutes; 1--4,294,967,295. Never a kill timeout.
   * - ``execution``
     - String enum; ``"maintenance"``
     - Exactly ``"maintenance"`` or ``"online"``; controls board drain/reload behavior.

No individual event field is required syntactically: even an empty event
record receives defaults and a legacy ID. That record is enabled, daily at
midnight, maintenance/fixed, with an empty command. Do not use empty records
as disabled placeholders. TOML has no null: omit optional keys rather than
writing ``null``, ``""`` or zero. Zero interval/warning values are rejected,
as are negative or fractional numbers.

Local time and weekday encoding
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Use unquoted TOML local times such as ``time = 03:05:00``. These are not
strings and not dates or timestamps. Accepted components are hours 0--23,
minutes 0--59 and seconds 0--59. The custom reader rejects a date component,
an offset/timezone and nonzero fractional seconds. A syntactically supported
zero fraction is not nonzero and therefore passes that check, but saves use
whole-second times. For portable files, always write ``HH:MM:SS``. The setup
editor's ability to accept ``HH:MM`` is not a promise that a bare TOML
``HH:MM`` value will parse.

The mask positions are **Sunday, Monday, Tuesday, Wednesday, Thursday, Friday,
Saturday**. ``Y`` selects a day and ``N`` deselects it:

.. list-table:: Valid masks
   :header-rows: 1

   * - Value
     - Selected days
   * - ``"YYYYYYY"``
     - Every day (default).
   * - ``"NYYYYYN"``
     - Monday through Friday.
   * - ``"NYNNNNN"``
     - Monday only.
   * - ``"NNNNNNY"``
     - Saturday only.
   * - ``"YNNNNNY"``
     - Saturday and Sunday.
   * - ``"NNNNNNN"``
     - No automatic occurrences; valid, and still manually runnable.

Lowercase letters, spaces, an array of weekday names, numeric bitmasks or
strings with other than seven bytes are invalid.

Scheduling and latest-start behavior
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Without ``interval_minutes``, there is one start at ``time`` on each selected
day. An ``end_time`` alone allows that occurrence to wait; it does not create
additional occurrences. With an interval, slots are ``time + n * interval``
starting with ``n = 0`` on each selected day. Slots stop at the inclusive
``end_time`` second or at 23:59:59 when no end is supplied. They reset to
``time`` on the next selected day, not to the previous day's last slot and
not to the previous command's completion time.

For example, 03:05 with a 17-minute interval and end 04:00 yields 03:05, 03:22,
03:39 and 03:56. Equal start/end values are valid and permit just one slot.
An end earlier than start is rejected: overnight windows are not inferred.
Split such work into separate records with distinct IDs and the appropriate
weekday masks. There are no cron expressions, dates, monthly masks, per-node
schedules or configurable timezone fields.

All slots use the host's local wall clock. A daylight-saving gap is skipped;
an ambiguous time uses the earlier instant (first fold). A retained occurrence
with an end expires once the local latest-start bound is passed, including
after midnight. Without an explicit end a retained occurrence may wait beyond
midnight. **End bounds limit starting, never running duration.**

The scheduler scans forward from startup: it does not catch up time while
the board was offline. Observed due occurrences are retained while busy.
For interval events only the newest due, unclaimed slot per ID survives a
backlog; older unclaimed slots become ``superseded`` in history without an
attempted start. Latest-start expiry takes precedence as ``expired``; Idle
with callers takes precedence as ``skipped_busy``. Claimed/running work and
manual requests are not displaced. Simultaneous scheduled starts are ordered
by the records' file order, but reordering does not change their IDs.

Action and caller policy
~~~~~~~~~~~~~~~~~~~~~~~~

There is no ``action`` table or action enum. The action is the string
``command`` plus the separate ``execution`` and ``mode`` choices.

.. list-table:: Execution/caller combinations
   :header-rows: 1
   :widths: 18 18 64

   * - Execution
     - Mode
     - Behavior
   * - ``maintenance``
     - ``fixed``
     - Close admission at the global suspension time; suspend/drain callers. Start no earlier than the slot and only after sessions and managed services drain.
   * - ``maintenance``
     - ``slide``
     - At the due occurrence close admission and wait for existing callers to finish naturally, subject to latest-start expiry.
   * - ``maintenance``
     - ``idle``
     - Skip the occurrence when callers are online at a due check; otherwise perform maintenance.
   * - ``online``
     - ``fixed``
     - Run with callers without closing admission or stopping services.
   * - ``online``
     - ``slide``
     - Wait for no callers without closing admission, subject to latest-start expiry.
   * - ``online``
     - ``idle``
     - Skip if callers are online; otherwise run without maintenance.

Maintenance stops managed services, releases the board-data lock, runs the
foreground command, reacquires the lock, reloads board-owned data and
restarts listeners/services in the same process. It is not an OS process
restart. A command failure is recorded and still followed by reload; a reload
failure keeps the board offline and retries loading, not the command.
Fixed does not mean force-killing live writers to meet a deadline.

On Unix, commands are handed to ``sh -c``; on Windows, to ``cmd /C``.
Standard input is null and stdout/stderr are captured together in a unique
file under board-root ``event_logs``. Paths in a command are interpreted by
the shell from the board root; the command field is not an argv array.
The executable/script must exist and be runnable at execution time, but the
configuration validator does not check that. An empty command spawns no
shell; empty maintenance still drains, reloads and restarts the board.

.. warning::
   Online execution is not a sandbox or a mail-safe mode. Commands must not
   modify live board data, need interactive input, or create background
   descendants. Keep message-base packing and mailer writes in maintenance.
   Independently launched mailers are not automatically coordinated by the
   event lifecycle. Only one foreground event command runs at a time;
   maintenance waits for online work to finish before releasing the board
   lock. Neither ``end_time`` nor ``warning_minutes`` kills a hung command.

At the warning threshold the scheduler logs ``Running long`` (and marks online
status). There is no forced timeout, retry-count or background-execution key.
A stuck command, session or managed service can hold maintenance indefinitely.
The normal call-wait server owns the scheduler; standalone local/PPE execution
modes do not start it.

Stable identity and manual execution
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Explicit IDs need not be UUIDs: ``nightly-maintenance`` is valid. Use stable,
meaningful IDs when hand-writing schedules. UUID-v4 IDs are generated by the
Rust event constructor and for new/duplicated editor records, not by missing-ID
deserialization. Ordinary edits and reordering preserve the ID; a duplicate
must get a different ID and therefore separate history.

If ``id`` is omitted, the loader serializes a canonical, fully defaulted
record with an empty ID, hashes it with SHA-256, and prefixes the lowercase
hex digest with ``legacy-``. Loading does not modify the schedule file; the
next save persists this fallback. Whitespace, key order and explicitly
spelling an existing default do not change it. Changing record contents
before the ID has been saved changes its identity on the next load. Two
identical ID-less records deliberately collide and fail list validation;
they are not assigned random replacements. An explicit empty ID is invalid,
not a request for fallback.

Confirmed manual requests can run with global scheduling disabled or a record
disabled, and override day/start/end restrictions. They still obey execution
and caller policy: maintenance/fixed drains immediately, slide closes
admission and waits, idle skips busy callers; online follows the matrix
above. They receive independent history keys rather than retrying an old
occurrence. Duplicate queued/active IDs and unavailable runtime states can
reject a manual request.

Complete schedule example
-------------------------

This is a complete separate schedule file. The first record demonstrates
every persisted field. The second is deliberately disabled, demonstrating
that an offline script can be prepared without scheduling it immediately.
Supply the scripts before enabling or manually running them.

.. code-block:: toml

   [[event]]
   id = "weekday-maintenance"
   description = "Weekday maintenance window"
   enabled = true
   time = 03:05:00
   days = "NYYYYYN"
   mode = "slide"
   command = "scripts/maintain.sh"
   end_time = 04:00:00
   interval_minutes = 17
   warning_minutes = 10
   execution = "maintenance"

   [[event]]
   id = "saturday-pack"
   description = "Pack message bases"
   enabled = false
   time = 04:30:00
   days = "NNNNNNY"
   mode = "fixed"
   command = "scripts/pack.sh"
   execution = "maintenance"

An independent minimal, safe online example uses a foreground command that
does not access board data. This is another complete schedule, not text to
append after the preceding record without its ``[[event]]`` header:

.. code-block:: toml

   [[event]]
   id = "daily-clock-log"
   description = "Record system time"
   time = 12:00:00
   command = "date"
   execution = "online"
   mode = "fixed"

Here ``enabled`` defaults true and ``days`` defaults to every day. Omitting
interval, end and warning disables those optional settings.

Generated history TOML
----------------------

The board-root ``event_history.toml`` is a runtime journal, **not** the
schedule. The scheduler owns an independent exclusive ``.event_history.lock``
lease across maintenance lock release. Do not edit, delete or rotate the
journal as a way to trim logs: occurrence keys are the replay barrier.
History/log retention is unbounded; rotate log files separately while retaining
history keys. No filename/retention knobs are persisted in event configuration.

The root has exactly ``version`` (required ``u32``, validator requires ``1``)
and ``entries`` (required array of tables). Unlike the schedule, a missing
entries key is not an empty list. The empty persisted shape is:

.. code-block:: toml

   version = 1
   entries = []

.. list-table:: Every ``[[entries]]`` field
   :header-rows: 1
   :widths: 23 27 50

   * - Key
     - Type; omitted value
     - Meaning
   * - ``key``
     - String; **required**
     - Unique occurrence key; scheduled or manual form below.
   * - ``event_id``
     - String; **required**
     - Stable ID with the same 1--128 ASCII identifier rules as the schedule.
   * - ``description``
     - String; **required**
     - Description snapshot, not a lookup into the current schedule.
   * - ``scheduled_for``
     - RFC3339 datetime string; **required**
     - UTC occurrence time. Unlike schedule times, this is a quoted string.
   * - ``start``
     - Optional RFC3339 datetime string; absent
     - Attempted-start time persisted before spawn; not proof a process ran.
   * - ``finish``
     - Optional RFC3339 datetime string; absent
     - Completion/outcome time; required by validation for every non-pending result.
   * - ``result``
     - String enum; **required**
     - Exact outcome spelling listed below.
   * - ``exit_code``
     - Optional ``i32``; absent
     - Process exit status when available; -2,147,483,648 through 2,147,483,647.
   * - ``log_file``
     - Optional string; absent
     - Planned board-relative log path; must occur together with ``start``.
   * - ``manual``
     - Boolean; **required**
     - Distinguishes explicit requests from scheduled occurrences.
   * - ``execution``
     - String enum; **required**
     - ``"maintenance"`` or ``"online"``; no omission default in history.
   * - ``detail``
     - Optional string; absent
     - Human-readable failure/recovery detail.

``result`` accepts exactly these snake-case strings:
``"pending"``, ``"success"``, ``"nonzero_exit"``, ``"spawn_error"``,
``"wait_error"``, ``"interrupted"``, ``"skipped_busy"``, ``"expired"`` and
``"superseded"``. Capitalized UI labels are not TOML enum values. Older
binaries without ``superseded`` support cannot read journals containing it.

Both root and entry structs reject unknown fields. Further validation requires:

* Unique occurrence ``key`` values and a valid nonempty ``event_id``.
* For scheduled entries, ``key`` must equal the event ID, ``@``, and
  ``scheduled_for`` formatted by the implementation as UTC RFC3339. The
  generated zero offset is ``+00:00``; it is not interchangeable with ``Z``
  inside that key string. Datetime string inputs themselves can use RFC3339
  offsets and are normalized to UTC.
* For manual entries, the key must begin with ``manual@``. The writer appends
  a fresh UUID; the validator checks the prefix, not UUID syntax.
* ``finish`` must be absent if and only if ``result = "pending"``. ``start``
  and ``log_file`` must either both be present or both absent.
* A log path must start with ``event_logs/``. Its remaining filename must end
  with ``.log``, contain only ASCII letters, digits, hyphens or dots, and
  contain no ``..``. An actual log need not exist after preparation failure.

The validator does not enforce chronological ordering of timestamps or infer
whether a shell really executed from an attempted start. A generated completed
entry with every field present has this shape:

.. code-block:: toml

   version = 1

   [[entries]]
   key = "weekday-maintenance@2026-09-07T03:05:00+00:00"
   event_id = "weekday-maintenance"
   description = "Weekday maintenance window"
   scheduled_for = "2026-09-07T03:05:00+00:00"
   start = "2026-09-07T03:05:01+00:00"
   finish = "2026-09-07T03:05:12+00:00"
   result = "success"
   exit_code = 0
   log_file = "event_logs/8b220030-d4c5-4c16-9b96-9f7de25fa020.log"
   manual = false
   execution = "maintenance"
   detail = "Completed"

A pending claim is durably recorded before process spawning. On scheduler
startup leftover pending entries become interrupted, with a finish timestamp,
and are not automatically retried. Skips, expiry and superseded occurrences
have no attempted start or command log. Journal read-only views do not recover
pending claims. Journal write failure closes admission and requires repair
and restart rather than command replay; there is no exactly-once guarantee
for shell side effects across a process/power failure.

Validation summary
------------------

Before enabling a hand-written schedule, check the following:

* The board has one ``[event]`` options table pointing at the intended file;
  that file uses ``[[event]]`` records, not a copied options table.
* Times are local-time literals, days are seven uppercase Y/N characters,
  optional durations are omitted or positive, and no end precedes its start.
* Every explicit ID is stable, valid and unique, including disabled records.
  Persist generated legacy IDs before making external content changes.
* Execution and mode are chosen separately. Scripts exist, remain foreground,
  require no input, and do not write live data in online mode.
* Treat load errors as errors, despite the board's empty-list fallback, and
  retain journal keys when managing logs or changing schedules.

These rules follow the engine's custom ``EventFields``/``EventListFields``
deserializers, ``BoardEvent::validate()``, event-history validator and call-wait
scheduler. See :doc:`../events` for editor controls and runtime operation, and
:doc:`networks` for mailer configuration used by maintenance scripts.