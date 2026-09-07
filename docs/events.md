# Timed events: source review, 2026-09-07

This describes the current working-tree implementation, including the event editor
and scheduler/restart changes. The PCBoard comparison is grounded in the bundled
sources; executed regression results and remaining validation gaps are recorded
at the end.

## Configuration and editor

The supported `BoardEvent` record contains `id`, `description`, `enabled`, `time`,
`days`, `mode`, `command`, `end_time`, `interval_minutes`, `warning_minutes`, and
`execution`. The TOML list uses `[[event]]` records. Defaults are enabled,
midnight, all weekdays, `fixed`, `maintenance`, empty description/command, and no
optional end, interval or warning. Disabled records and an empty weekday mask do
not schedule automatically. This is not the full PCBoard event record.

- `end_time` is an inclusive latest-start bound through that whole local second,
  on the scheduled day, and must be at or after `time`; overnight windows are
  rejected. It expires waiting scheduled work, never terminates a running command.
- `interval_minutes`, when present, is a positive integer: slots are anchored at
  `time + n × interval_minutes` on each selected weekday, not at the previous
  completion. Slots end at `end_time`, or at the end of the day when omitted.
  Without an interval there is one slot per selected day. There is no monthly or
  date-mask schedule.
- `warning_minutes`, when present, is a positive integer. After that elapsed
  command duration the runtime logs **Running long**; it is not a kill timeout.
- `execution` selects `maintenance` (drain/reload) or `online` (no drain/reload),
  independently of Fixed/Slide/Idle caller policy, detailed below.

Global `EventOptions` control `enabled`, `event_file`, `suspend_minutes`,
`disallow_uploads`, and `minutes_uploads_disallowed`. Suspension and upload lead
times apply to **Maintenance Fixed only**. Turning global events off stops
automatic scheduling without discarding the list.

Sources: [event model](../crates/icy_board_engine/src/icy_board/events.rs),
[configuration](../crates/icy_board_engine/src/icy_board/icb_config.rs#L1055-L1070).

In ICBSetup choose **General → E, Event setup**, select **Name/Location of Event
File**, then press **F2** for the built-in list editor. F2 is a path-field shortcut,
not a global shortcut from every setting. **F4** browses for the path, relative to
the board root; the browser owns keys until it closes. Editing the command does
not invoke a path picker: it is shell text, including arguments and quoting.

| Context | Actual keys/actions |
| :--- | :--- |
| List selection | Up/Down or k/j; Home/End select first/last. |
| List editing | Enter opens details; Insert adds a **disabled** record after the selection; Delete removes it; F5 duplicates it, including its enabled flag. |
| List ordering | PageUp/PageDown move the selected record one position, rather than paging the view. |
| List saving | F2 saves and closes on success. Esc closes an unchanged list, otherwise asks whether to save. |
| Save question | Left/Right change the choice; Enter accepts; Esc cancels the question and returns to the list. |
| Details | Up/Down move fields; Enter moves to the next field, **not** Apply. F2 validates and applies to the staged list; Esc discards the detail edits. F1 shows help. |
| Enabled field | Space, Left/Right, Tab/Shift-Tab toggle. |
| Mode field | Left/Right or Shift-Tab/Tab cycle Fixed, Slide, Idle. |
| Execution field | Left/Right or Shift-Tab/Tab cycle Maintenance, Online. |
| List history | F6 opens read-only history for the selected stable ID. |
| History view | Up/Down select an entry; Home/End select newest/oldest; PageUp/PageDown scroll details; F6 refreshes; F1 shows help; Esc closes. |

Time input must be `HH:MM` or `HH:MM:SS`, with valid two-digit components. Days
must be exactly seven Y/N characters, **Sunday first**; lowercase is normalized.
All-N is accepted but never schedules automatically. Blank latest-start, interval
and warning fields disable those options; minute values must be positive integers.
Description and command use scrolling text fields with a 48-column editing width,
not a 48-character value limit; long Unicode
commands and arguments survive edits and persistence. The editor stages changes:
Apply is not a disk save. A missing list opens
empty and is created only on Save; malformed/unreadable lists produce an error,
not an empty replacement. Save errors retain the working copy for retry. Global
setup changes still have their own save flow; saving a list is not a live runtime
file-watch reload.

New and duplicated records receive fresh UUID-v4 IDs; editing and reordering
preserve IDs. Legacy records without an ID get a deterministic SHA-256-derived
ID from their fully defaulted canonical contents, persisted on the next save.
Whitespace and reordering do not change it, but external field edits followed by
reload before ID persistence do. Identical legacy records collide; assign distinct IDs before saving/loading
them together. Duplicate IDs are rejected. History follows IDs, not row positions;
a duplicate does not inherit the original's history. The setup history viewer
reads the board-root journal without claiming its lock, recovering pending work,
or running commands; missing history is empty and read errors are reported.

Sources: [setup entry](../crates/icbsetup/src/tabs/general/event_setup.rs),
[general menu](../crates/icbsetup/src/tabs/general/mod.rs),
[list/detail editor](../crates/icbsetup/src/editors/events.rs),
[shared field keys](../crates/icy_board_tui/src/config_menu.rs#L846-L1007),
[save dialog](../crates/icy_board_tui/src/save_changes_dialog.rs),
[save helper](../crates/icbsetup/src/editors/mod.rs#L50-L66).

## Scheduling and execution policy

The current `Schedule` retains occurrences across ticks using a monotonic scan
watermark and pending `(stable ID, run_at)` entries. It ticks at one-second
intervals, retains due work even when execution is delayed, orders ties by file
position, and removes an occurrence when taken or when a busy Idle is skipped.
Pending entries pick up command/description/mode/execution/restriction changes
after reload; removed, disabled, or retimed entries are dropped. Backward clock movement does
not move the watermark backward. Session restrictions prefer the next Maintenance
Fixed window, so a nearer Online or Slide/Idle event does not hide it.

For `execution = "maintenance"`:

| Mode | Admission and existing callers | Execution |
| :--- | :--- | :--- |
| `fixed` | Close admissions and repeatedly request caller shutdown from `run_at - suspend_minutes`. Apply time/upload limits before then. | Run no earlier than `run_at`, only after every node and service has drained. This is not a hard real-time forced kill. |
| `slide` | No Fixed-style advance restrictions. At due time close admissions; let existing callers finish naturally. | Wait until empty, subject to `end_time` expiry if set. Without an end bound, retained work can wait beyond midnight. |
| `idle` | No advance restrictions. If a caller is online at a due check, consume/skip this occurrence. | If empty, close admission atomically and run; otherwise wait for the next scheduled slot (possibly another interval that day), not an arbitrary later idle moment. |

For `execution = "online"`, Fixed runs with callers present, Slide waits for no
callers **without closing admission**, and Idle skips a busy occurrence. Online
does not impose session/upload restrictions, stop services, release `BoardLock`,
or reload live board state. Only one foreground event command runs at a time;
maintenance can gate/drain callers while Online runs but waits for its completion
before stopping services and releasing the lock.

**Online is an administrator safety contract, not a sandbox:** arbitrary shell
commands cannot be prevented from writing board files. Online commands must not
modify live board data, require interactive input, or launch background work.
Keep mailer live writes in Maintenance: the raw `icbmailer` does not acquire
`BoardLock`, so holding that lock does not make a concurrent mailer safe. Online
mailer safety has not been verified.

Time is the host's local wall clock. Nonexistent DST times are skipped; ambiguous
times choose the earlier instant. A new process scans forward from startup:
there is **no catch-up for downtime**. The durable journal below prevents replay
of claimed occurrences; it does not guarantee exactly-once command effects.

Sources: [scheduler and regression-test definitions](../crates/icboard/src/event_scheduler.rs),
[occurrence calculation](../crates/icy_board_engine/src/icy_board/events.rs).

## History, logs and manual execution

The board-root event_history.toml journal records scheduled occurrences by stable
ID plus UTC scheduled timestamp, and each manual request by a fresh run ID.
An independent exclusive journal lock survives release of `BoardLock`. Pending
claims are persisted **before spawn**, using a synced temporary file and atomic
replacement (with directory sync on Unix). On scheduler startup, leftover
`pending` claims become `interrupted`, never automatically retried. Busy Idle
skips, expired slots, success, nonzero exits, spawn/wait errors, timestamps, exit
codes and log paths are recorded. An attempted-start timestamp is not proof that
a process actually ran. Journal failures latch admission closed and require
repair/restart, not command replay.

Commands run foreground through `sh -c` (`cmd /C` on Windows), in the board root,
with null stdin and stdout/stderr captured together in a unique file under
event_logs. Journal and logs have **unbounded retention**. Rotate logs separately;
do not delete the journal's occurrence keys to trim logs, because they are the
replay barrier. Lost history cannot prove previous execution. Commands have no
forced time limit; `warning_minutes` only logs a warning (and marks Online status).

At runtime, **F6 on call-wait opens the Events menu**. Up/Down and Home/End select
a record; **R or F5** refreshes the loaded-board list and read-only history snapshot;
**Esc or F6** closes. Enter opens a **default-No** confirmation; Left/Right or Tab
toggles the choice, Enter accepts, and Esc cancels. A second Enter alone does not
run anything. The menu shows execution/mode, queued/active state, latest result
and log path; refresh is not an event-file reload.

Confirmed manual runs override global scheduling off, a disabled record, weekday,
start and end restrictions, but retain execution and caller policy: Maintenance
Fixed drains immediately, Slide closes admission and waits naturally, Idle skips
if busy; Online follows the policy above. Requests are revalidated, bounded and
refused for duplicate queued/active IDs or unavailable maintenance/error states.
They are separate journaled runs, not retries of old scheduled occurrences.

Sources: [journal](../crates/icy_board_engine/src/icy_board/event_history.rs),
[runtime menu](../crates/icboard/src/event_screen.rs),
[call-wait keys](../crates/icboard/src/call_wait_screen.rs).

## Drain → command → reload → listener restart

This lifecycle applies to **Maintenance**, not Online execution.

1. `BBS.event_maintenance` closes both network and local node admission. The online
   check and gate update share the BBS lock with `spawn_node`, avoiding an
   empty-board/admission race. Node allocation installs its thread handle
   atomically; reservations are not mistaken for completed sessions.
2. Fixed sends `BBSMessage::Shutdown` using channel snapshots outside the BBS lock.
   A full queue is retried on later ticks rather than awaited while holding the
   lock. The session disables MORE counting, attempts the waiting notice and bell,
   persists its current user without final-time accounting, and hangs up even if
   writing the notice failed. Normal session exit performs final accounting where
   reached. The scheduler waits for thread completion/join, not just a closed socket.
3. Once empty and due, the scheduler sets `event_restart_requested`. Call-wait/main
   stops and joins its listener generation and live web administration service.
   SSH transport tasks are tracked and disconnected; HTTP shutdown is graceful.
   Main releases its `BoardLock` and acknowledges `event_listeners_stopped`.
4. Only then does the scheduler run the foreground command with captured output
  as described above, provided its scheduled latest-start bound has not expired.
  An empty command still performs the maintenance/reload cycle.
  Spawn failure or nonzero exit is logged;
   disk changes are reloaded even after a failed command.
5. Reacquire `BoardLock`, call `IcyBoard::load()` and `resolve_paths()`, and replace
   the complete object behind the existing shared board `Arc`. Resize the empty
   node table without replacing its shared identity. Clear the restart request.
6. Main starts listeners/admin from the newly loaded configuration and recreates
   call-wait UI state. The original scheduler continues and recomputes the gate
   for tied/pending events before admitting callers again.

This is a **same-process service restart**, not OS `exec`, process replacement,
or an executable upgrade. The scheduler and shared BBS identities survive.
Tracked services here are Telnet, SSH, secure WebSocket and live web admin; the
plain-WebSocket start block is commented out.

Call-wait displays the event description and maintenance phase: waiting for the
scheduled time, draining callers, stopping services, running, reloading, or
restarting. During the handshake it redraws every 250 ms without cancelling the
operation. Reload/lock errors show repair instructions and keep admission closed;
a stalled operation is not treated as permission to reopen the board unsafely.

Sources: [runtime handshake](../crates/icboard/src/event_scheduler.rs),
[main service ownership/restart](../crates/icboard/src/main.rs#L174-L463),
[node admission and joins](../crates/icy_board_engine/src/icy_board/bbs.rs),
[SSH transport lifecycle](../crates/icboard/src/bbs/ssh.rs),
[graceful admin shutdown](../crates/icbadmin/src/lib.rs),
[shutdown handler](../crates/icy_board_engine/src/icy_board/state/mod.rs#L3955-L3972).

### Reload ownership, locks and failure handling

Reload replaces **all board-owned state**, not just event settings: configuration,
users, conferences and their loaded data, display text, languages, protocols,
security levels, groups, statistics, commands, FTN/QWKnet/ZCONNECT configuration,
events, and the board-owned PPL HTTP service/cache. Paths are resolved again.
JAM/email handles belong to session operations, not a permanent `IcyBoard` mail
handle: drained/joined sessions have dropped those handles. New mail operations
open the newly configured paths. This is not an in-place refresh of live mail
handles, nor a reset of every process-global object.

`BoardLock` is an advisory cross-process directory lock, reference-counted inside
the process so board/admin can share it. It is released only after managed writers
drain and before the command, allowing cooperating offline tools to acquire it.
It cannot protect against arbitrary external writers which ignore that lock.
The board/BBS mutexes are not held across shell execution. Operator-launched board
tools have a separate admission interlock; event scheduling pauses during that
operator-maintenance window.

If lock reacquisition or a required reload fails, the board stays offline and
retries once per second. Reload errors release the lock for offline repair. The
**completed/attempted command is not repeated** by this retry loop. An enabled,
nonempty event-file path is loaded strictly during maintenance reload, unlike
ordinary startup's logged empty-list fallback. Other existing loader fallbacks
remain (for example statistics, groups, FTN and QWKnet); full replacement does not
mean every ancillary load error is fatal. Listener bind failures are logged, but
there is no automatic bind-retry/rollback transaction guaranteeing every configured
endpoint reopened successfully.

Sources: [board fields and loader](../crates/icy_board_engine/src/icy_board/mod.rs),
[email path/opening](../crates/icy_board_engine/src/icy_board/state/functions.rs#L728-L745),
[directory lock](../crates/icy_board_engine/src/icy_board/lock.rs),
[reload/retry logic](../crates/icboard/src/event_scheduler.rs),
[service startup](../crates/icboard/src/main.rs).

## Caller notices, logon and new users versus PCBoard

Advance time/upload restrictions and shutdown notices concern **Maintenance
Fixed**; admission guards also honor other maintenance gates. Online does not
apply these restrictions.

- **Shortened allowance:** Fixed caps the total session allowance relative to
  `login_date`, including when security limits are reapplied; zero means unlimited,
  so the cap is at least one minute. `time_adjusted_for_event` records an actual
  reduction. `EVTTIMEADJ()` exposes it; positive `ADJTIME` is refused once set,
  while negative adjustments remain possible. Slide/Idle alone do not set it.
  Initial login applies this after loading the user's security allowance, not to
  the anonymous default; callers whose ordinary time expires before suspension
  do not receive a spurious adjustment announcement.
- **Announcement:** after successful existing-user authentication, IcyBoard
  reapplies the cap, displays `IceText::TimeAdjusted` if the flag is set and calls
  `press_enter()` before joining the saved conference/logon questions. New users
  use the same announcement after account creation/current-user setup, before
  news/logon questions. It is not a periodic countdown or a notice for every event.
- **Login refusal:** the welcome display precedes the first login guard. Further
  guards run in the name loop, after password success, on entry to registration,
  and after registration questions before account publication. They check both
  the maintenance gate (including due Slide) and Fixed suspension, display/log
  `DeniedAccessForEvent`, and hang up. Connections rejected by node admission may
  never reach this localized login notice.
- **Interrupted registration:** required-input loops stop on `request_logoff`;
  the final guard avoids publishing the still-staged account after observed
  shutdown/suspension. This is a guard at explicit checkpoints, not an atomic
  transaction spanning every registration await. The outer registration path can
  also display `RefusedToRegister` when `new_user()` returns false; do not promise
  an event-only transcript for all interruptions.
- **Shutdown and completion:** Fixed sends `WaitingForEvent` before hangup; the
  command start/result is logged using runtime log messages. Existing
  `EventRan`/`_EventFinished` text identifiers are not wired into this scheduler as
  localized completion announcements. Maintenance has no callers left to announce
  to after the command; Online completion also uses runtime logs/history.

PCBoard's [LOGIN.C](../pcboard/pcb-main/SOURCE/NODE/LOGIN.C#L1214-L1219) explicitly
shows `TXT_TIMEADJUSTED` and pauses so INTRO cannot erase it. Its new-user path
calls `checksessionandkbdtime()` before registration, and that helper denies a
session with at most ten seconds remaining using the event-specific text when
adjusted ([login/new-user checks](../pcboard/pcb-main/SOURCE/NODE/LOGIN.C#L134-L205)).
These are the source basis for the notices, **not identical admission policy**:
[MISC.C](../pcboard/pcb-main/SOURCE/MAIN/MISC.C#L145-L181) grants a new-time login
inside suspension roughly half the suspend buffer plus ten seconds (OS/2 has
integer-minute rounding), and adds half the buffer when less than two minutes
remain before suspension. IcyBoard instead refuses login strictly during
suspension; its one-minute minimum cap is not PCBoard's grace policy.

IcyBoard sources: [login/registration](../crates/icboard/src/menu_runner/login.rs),
[time cap](../crates/icy_board_engine/src/icy_board/state/mod.rs#L1942-L1974),
[ADJTIME](../crates/icy_board_engine/src/vm/statements/predefined_procedures.rs#L333-L341),
[EVTTIMEADJ](../crates/icy_board_engine/src/vm/expressions/predefined_functions.rs#L2321-L2323).

## Remaining parity and operational boundaries

- **Import:** the importer reads only the single daily time/slide flag and global
  event settings from PCBOARD.DAT. It writes one all-days record when the time is
  present, with an empty command. It does **not import EVENT.DAT** or translate DOS
  batch files. See [importer](../crates/icbsetup/src/import/mod.rs#L749-L805).
- **PCBoard model:** no monthly/date/wildcard mask, PCBoard per-node last-date
  database, per-node scheduling/batch selection, OS/2 flag, Fido-verb event or
  Fido mail-hour event.
  [EVENT.H](../pcboard/pcb-main/SOURCE/H/EVENT.H) defines `E` expedite, `S` slide,
  `I` idle, `N` none, `F` Fido and `M` mail; `F`/`M` are **not** aliases for
  IcyBoard Fixed/Slide. [EVENT.C](../pcboard/pcb-main/SOURCE/MAIN/EVENT.C) processes
  date/end windows, node-specific batch fallbacks and per-node last dates;
  `eventminutes()`/`timeforevent()` reserve advance suspension for E/M. Its
  `performevent()`/`prepevent()` prepare a batch and exit/recycle for ordinary
  events, with special Fido mail-hour behavior, rather than IcyBoard's shell and
  same-process reload. Matching three mode names does not establish full parity.
- **Uploads differ:** IcyBoard rejects file, attachment and editor uploads when
  the retained Maintenance Fixed window reaches `run_at - minutes_uploads_disallowed`, provided
  upload blocking is enabled. This ignores the caller's adjustment flag; zero
  means the scheduled instant. PCBoard instead requires
  `TimeAdjustedForEvent && EventStopUplds`, then rejects when its setting is zero
  **or** `minutesleft() <= MinPriorToEvent`. Zero there blocks any event-adjusted
  caller immediately. PCBoard's source itself notes the missed-unadjusted-caller
  case. See [TRANSFER.C](../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L4388-L4405),
  [file uploads](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/u_upload_file.rs#L339-L349),
  [attachments](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/message_attachment.rs#L170-L180),
  [editor uploads](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/editor/upload.rs#L67-L76).
- **Cooperative drain:** stalled transfers, doors or PPEs that do not service node
  shutdown cannot safely be force-killed while they may still write. Such sessions
  can delay even Fixed indefinitely when no latest-start bound expires the waiting
  occurrence. Undrained admin requests can also hold maintenance offline. The
  foreground-command and timeout boundaries are described above.
- **Standalone local CLI:** `--localon`, `--ppe` and `--runppe` return/exit before
  scheduler startup. They do not run the background scheduler. Local sessions
  launched from the normal call-wait server are different: they share its gate
  and active scheduler. See [startup branches](../crates/icboard/src/main.rs#L174-L218).
- **FTN scope:** the reviewed `icboard` startup/service list does not start a
  background FTN poll/toss worker. FTN configuration is reloaded, and mail work
  exists in the separate [icbmailer implementation](../crates/icbmailer/src/main.rs),
  including poll/toss commands for scheduled foreground **maintenance** work.
  Absence of PCBoard F/M event modes is not absence of FTN support. This restart handshake
  does not stop or restart independently launched mailer processes.

## Validation

Executed with `CARGO_INCREMENTAL=0 cargo test-low`:

| Scope | Result |
| :--- | :--- |
| Complete icboard binary suite | 307 passed in English and 307 in German. |
| icboard `event` subset | 37 passed; included in the full runs. |
| Complete icbsetup binary suite | 89 passed in English and 89 in German. |
| Engine library `event` tests | 52 passed, including 18 model/journal tests. |
| Shared localization integration | 3 passed, checking both catalogs. |
| Workspace, all targets | `cargo check --workspace --all-targets --jobs 4` passed. |

The scheduler integration test runs the actual scheduler and a foreground shell
command against temporary board files. It verifies session-thread completion and
the service-stop acknowledgement before execution, lock release for maintenance,
config reload, node resizing, admission reopening, and no command replay. Its main
service acknowledgement is simulated: it is not a live Telnet/SSH/TLS rebind test.
Separate two-caller session tests join real session threads after shutdown at a
command prompt and in the message editor, check persisted users and the discarded
draft, and reopen mail for reading/writing. The time-cap regression also covers
elapsed session time and retained occurrences after their due time.

The nine earlier broader-suite failures have been addressed with corrected
fixtures (eight upload continuation transcripts and one capture cancellation
transcript); both full icboard runs now pass. The workspace compilation reports
an existing unused-mut warning in the message-capture test code. No whole-workspace
test run or live production-board maintenance is claimed.

Definitions and assertions are in
[scheduler tests](../crates/icboard/src/event_scheduler.rs),
[BBS tests](../crates/icy_board_engine/src/icy_board/bbs.rs),
[editor tests](../crates/icbsetup/src/editors/events_tests.rs) and
[session recovery tests](../crates/icboard/src/tests/session_recovery.rs).