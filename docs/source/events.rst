Timed events
============

For the current implementation and its PCBoard source comparison, see the
`timed-event source review <../events.md>`_. It covers the built-in editor,
caller announcements, same-process listener restart/full board reload, and
remaining parity differences and the current validation scope.

A timed event runs a foreground command on selected weekdays, once or at a
start-anchored interval. Maintenance commands, such as message-base packing or
mailer live writes, close admission and drain managed writers before execution.
Online commands leave the board running and must not modify live board files.
Execution type and Fixed/Slide/Idle caller policy are separate settings.

The event file
--------------

The events live in their own file. Its name is set in ICBSetup under
*General* → *Event setup* → *Name/Location of Event File*; a new board gets
``main/events.toml``. Pressing F2 on that entry opens the built-in event list
editor; F4 browses for a path. A missing file is created only on Save. Enter
opens a record, Insert adds a disabled record, Delete removes it, F5 duplicates
it, and PageUp/PageDown reorder it. In the detail form Enter moves down, F2
applies the staged edits, and Esc cancels them. In the list F2 saves and closes;
Esc prompts if there are unsaved changes.

F6 in the list opens read-only history for the selected event ID. Up/Down select
entries, Home/End select newest/oldest, PageUp/PageDown scroll details, F6 refreshes,
F1 shows help and Esc closes. This view never runs events or recovers pending
claims. Missing history is empty; read errors are reported without replacement.
Blank latest-start, interval and warning fields disable those optional settings.
Left/Right or Shift-Tab/Tab cycle mode and execution; Enter only moves down.

.. code-block:: toml

   [[event]]
   id = "nightly-maintenance"
   description = "Nightly maintenance"
   enabled = true
   time = 03:00:00
   end_time = 04:00:00
   days = "YYYYYYY"
   mode = "fixed"
   execution = "maintenance"
   warning_minutes = 30
   command = "scripts/nightly.sh"

   [[event]]
   id = "monday-mail-poll"
   description = "Monday fidonet polls"
   time = 05:30:00
   end_time = 06:30:00
   interval_minutes = 30
   days = "NYNNNNN"
   mode = "slide"
   execution = "maintenance"
   command = "icbmailer poll"

``id``
   Stable identity for scheduling and history. New/duplicated editor records get
   fresh UUID-v4 IDs; edits and reordering preserve them. Missing legacy IDs are
   derived from a SHA-256 hash of canonical, fully defaulted contents and persisted
   on the next save. External field edits followed by reload before ID persistence
   change the legacy identity; whitespace/reordering does not. Duplicate IDs, including identical legacy
   records, are rejected and need distinct IDs. A duplicate has separate history.

``description``
   Shown in the log. Purely informational.

``enabled``
   Controls automatic scheduling; defaults to ``true``. Confirmed manual runs can
   override it.

``time``
   Local wall-clock start time, a TOML local-time value as above. The editor accepts
   ``HH:MM`` or ``HH:MM:SS``. DST gaps are skipped; ambiguous slots use the earlier
   instant.

``end_time``
   Optional inclusive latest start, through the whole specified second, on the
   scheduled day. It must be at or after ``time``; overnight windows are rejected.
   Waiting scheduled occurrences expire after this bound. It never stops a running
   command. Without a bound, retained work may wait beyond midnight.

``interval_minutes``
   Optional positive integer. Runs at ``time + n * interval_minutes`` on each
   selected weekday, ending at ``end_time`` or the end of that day. Completion time
   does not shift the slots. Omit for one slot per selected day. Monthly/date-mask
   schedules are not implemented.

   On backlog, only the newest due, not-yet-claimed scheduled occurrence per
   stable event ID is retained. Older due, unclaimed slots are journaled as
   ``Superseded`` without an attempted start, command log or execution. Future
   slots remain scheduled. Manual starts and events without an interval are
   unchanged; claimed or running jobs are never displaced or cancelled.
   An exceeded ``end_time`` takes precedence as ``Expired``; Idle with active
   callers still produces ``SkippedBusy`` rather than ``Superseded``. This does
   not change the startup policy: there is no catch-up for offline time.

``warning_minutes``
   Optional positive elapsed-command duration. Logs ``Running long`` and marks
   Online status, but never kills the command or its descendants.

``days``
   Seven characters, ``Y`` or ``N``, starting with Sunday. ``YYYYYYY`` is every day,
   ``NNNNNNY`` is Saturdays only. Defaults to every day.

``mode``
   Caller policy. The following describes ``execution = "maintenance"``:

   ``fixed``
      From the suspension time, admission closes and callers are asked to
      disconnect. The command starts no earlier than its scheduled time and
      only after all sessions and managed services finish draining. This is
      the default, not a hard deadline enforced by killing active writers.

   ``slide``
      The event waits for the last caller to hang up. No new caller gets in while it
      waits, subject to latest-start expiry when ``end_time`` is set. Stalled
      sessions are not force-killed.

   ``idle``
      The occurrence is skipped when somebody is online at a due check. The next
      attempt is the next scheduled slot, possibly another interval that day.

   ``execution``
      ``maintenance`` (default) drains callers according to mode, stops managed
      services, releases the board lock, runs the command, reloads board data and
      restarts services. ``online`` does none of that: Fixed runs with callers,
      Slide waits for no callers without closing admission, and Idle skips if busy.
      Online imposes no event session/upload restrictions and retains the board lock.

      **Online is not a sandbox.** Arbitrary shell commands cannot be prevented from
      modifying board files. Online commands must not write live board data, require
      interactive input or launch background work. Keep mailer live writes in
      Maintenance: raw ``icbmailer`` does not acquire ``BoardLock`` and has not been
      verified safe online.

``command``
   Handed to ``sh -c`` (``cmd /C`` on Windows) with the board directory as the working
   directory, with null stdin and stdout/stderr captured together in a unique log
   under the board-root ``event_logs`` directory. Commands have no forced time
   limit and must remain foreground. An empty Maintenance command still performs
   the drain, board reload and listener restart. The event cannot
   synchronize with an independently running external scheduler merely by having
   an empty command.

Clearing the board
------------------

Three settings in ICBSetup control Maintenance Fixed suspension and upload restrictions.
They count backwards from the event time and do not impose advance restrictions
on Slide, Idle or Online events.

*Minutes prior to event to suspend the system*
   From this moment on nobody may log on any more. A caller reaching the login guard
   is shown ``Access Denied - Upcoming Event Pending ...`` and disconnected; a
   connection refused at node admission may not reach that notice. The callers who are
   already online are shown ``Awaiting Event Timer - All activity suspended ...`` and
   dropped.

   The same number also caps the time of a session that starts shortly before the event:
   a caller logging on ten minutes before the suspend period gets ten minutes, not their
   usual limit, and PPL's ``ADJTIME`` may then only take time away, never give it back.
   ``EVTTIMEADJ()`` returns true for such a session. After successful authentication
   or new-user setup, an event-adjusted caller gets the time-adjustment text and
   an Enter acknowledgement before continuing. Strict suspension-time refusal
   differs from PCBoard's login grace period.

*Disallow uploads prior to event* and *Minutes prior to event uploads disallowed*
   Together they refuse new uploads once the configured Fixed-event timestamp
   is reached. The caller is shown ``Uploads Are Currently Disabled``. This is
   not PCBoard's adjustment-flag/minutes-left policy; zero means the event time
   here, not an immediate ban for every event-adjusted caller. Already-running
   transfers still have to finish or cooperate with shutdown.

Command and reload lifecycle
----------------------------

For Maintenance, the normal call-wait server keeps one scheduler alive across
listener restarts.
It retains due occurrences instead of repeatedly asking only for a future event.
After sessions finish and their threads are joined, main stops/joins listeners
and live web administration, releases the board directory lock, and allows the
shell command to run. It then reacquires the lock, reloads and replaces all
board-owned state, resolves paths, and starts listeners from the new configuration.
Session-owned mail handles have already been dropped.

This is a listener/service restart in the same process, not OS ``exec``. A failed
reload leaves the board offline and retries loading, not the completed command.
Shell failures are logged and still followed by reload. Command completion uses
runtime logs, not the localized EventRan/EventFinished texts.

All enabled Telnet, SSH, secure WebSocket and web-admin endpoints must prepare
and bind successfully before any service task starts. Preparation fails as a
unit: already-bound listeners are dropped on failure. Initial startup returns
the error; after maintenance or an operator tool, the restart helper keeps
admission closed and retains ``BoardLock`` while retrying listener preparation
every two seconds. These retries never rerun the event or reload configuration.
Call-wait displays the actual listener error and a dedicated automatic-retry hint,
not reload-repair instructions. Release an occupied port, or correct configuration
and restart the process; disk configuration changes alone are not loaded by this
retry loop.

Only one foreground event command runs at a time. Maintenance may gate/drain
callers during an Online command, but waits for it before stopping services or
releasing the board lock. Stalled sessions, commands or admin requests can hold
maintenance offline indefinitely; detached descendants are not tracked. Standalone
``--localon``, ``--ppe`` and ``--runppe`` do not start this background scheduler.

History and recovery
--------------------

The board-root ``event_history.toml`` is a durable journal, separate from board
configuration and protected by its own exclusive lock. Scheduled keys combine
stable event ID and UTC scheduled timestamp; manual runs have unique keys.
Pending claims are synced and atomically replaced on disk before spawn. On
scheduler startup leftover ``pending`` entries become ``interrupted`` and are
never automatically retried. Startup scans forward: there is no downtime catch-up
and no exactly-once guarantee for command effects.

History includes busy skips, expiry, superseded slots, results, attempted start/finish timestamps,
exit codes and log paths. Attempted start is not proof that a process ran; a
planned log may be missing after a preparation error. Journal failure closes
admission and requires repair/restart, not command replay. History and logs have
unbounded retention: rotate logs separately and retain journal occurrence keys
as the replay barrier.

``Superseded`` appears as **Superseded** in English and **Überholt** in German
in setup history and the runtime Events menu. It marks an older unclaimed
interval slot replaced by a newer due slot, not a started or aborted command.
Its journal entry has no attempted-start timestamp or log path.

New binaries read old journals unchanged. Older binaries do not recognize the
serialized result ``superseded``: journals containing it are not backward
compatible with those binaries. Account for this before downgrading.

Manual execution at call-wait
----------------------------

F6 opens the runtime Events menu. Up/Down and Home/End select; R or F5 refreshes
the loaded-board list and read-only history snapshot (not the event file on disk);
Esc or F6 closes. Enter opens a default-No confirmation. Left/Right or Tab toggles,
Enter accepts, Esc cancels. Pressing Enter twice alone does not run the event.

A confirmed manual request overrides global scheduling off, disabled status,
weekday, start and end bounds, but not execution/caller policy. Maintenance Fixed
drains immediately; Slide closes admission and waits naturally; Idle skips if
busy. Online follows its policy above. Requests are revalidated and duplicate
queued/active IDs or unavailable maintenance/error states are refused. Each
accepted manual run is separately journaled, not a retry of an old occurrence.

Enabling events at all
----------------------

Automatic scheduling requires *Event enabled* in ICBSetup. When it is off the event
file is still loaded; confirmed manual requests remain possible.

Importing from PCBoard
----------------------

``PCBOARD.DAT`` only knows the single daily event, not the ``EVENT.DAT`` list. The
importer therefore writes one daily event with the time from ``PCBOARD.DAT``, in
``slide`` mode when PCBoard's *slide event* flag was set, and leaves its command empty -
PCBoard ran ``EVENT.BAT``, which will not do anything useful here. The suspend period and
the upload settings are carried over unchanged.

The importer does not read ``EVENT.DAT`` or translate DOS batch files. Same-day end
times, intervals and durable history are supported, but monthly/date masks,
PCBoard per-node last-run dates, per-node scheduling, OS/2 events and Fido/mail-hour
event modes are not. This does not mean FTN is unsupported: ``icbmailer`` provides
mail operations, but independently launched workers are not owned by the BBS
event restart handshake and must not write concurrently with the live board.
