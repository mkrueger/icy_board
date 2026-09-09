# Command-level reliability tests

The initial suite lives in
[session_recovery.rs](../crates/icboard/src/tests/session_recovery.rs). It is
registered in the `icboard` binary's tests and runs in the ordinary Cargo test
suite, despite being an integration test of several components rather than a
separate integration-test binary.

Run it with `cargo test-low -p icboard --bin icboard session_recovery`.

## What is exercised

Each fixture owns a temporary board directory. Callers use the real login and
PCBoard command loop over a channel connection; the tests do not call message
save handlers directly or stuff the keyboard buffer. Each session runs on its
own OS thread, with the normal shared board and node registry.

The fixture customizes ICBTEXT prompt strings to distinctive markers, using
the same text customization available to a sysop. This makes synchronization
independent of desktop locale and avoids confusing echoed input with results.
Commands, permissions and persistence code are not replaced. Each expected
prompt and session completion has a ten-second deadline. Successful tests
drain the connection to EOF, join the session thread and reclaim the node.

| Scenario | Checks |
| --- | --- |
| `E`, `S`, `BYE`, reconnect, `R` | Saved header/body and user post count survive session exit; a fresh board object reloads the users and reads the existing JAM base; the node is reused. |
| `E`, `A`, confirmation, `BYE` | No save acknowledgement or user post credit; existing JAM header, index and text files are byte-identical. |
| Carrier loss at the editor command prompt | Session terminates, unsaved draft is discarded, original base is unchanged, next caller can connect. |
| Carrier loss while typing a line | Partial input is discarded and the session terminates without changing the base. |
| Carrier loss in full-screen editing | The editor exits on EOF instead of spinning; no partial message is saved. |
| Message-base destination under a regular file | A deterministic `ENOTDIR` failure produces an error, not a save acknowledgement or post credit; the blocker is untouched and `BYE` still works. |
| Two callers composing in the same base | Both reach the editor before either saves; exactly one distinct message per caller and one credit per user persist. |

## Boundaries and next steps

These are **session interruption and persistence integration tests**, not a
claim of crash-atomic or power-loss-safe storage. The fixture uses local-mode
terminal capabilities and known users with empty passwords; it does not test
SSH/telnet negotiation or authentication security. The fresh-board case
reconstructs test configuration and reloads persisted users/messages, not a
complete installation via its configuration loader.

Carrier-loss cases close the caller's sending half of the channel, delivering
EOF to the server while keeping output available for assertions. They do not
yet simulate a broken output connection or TCP reset.

Overlapping callers exercise normal concurrent sessions, but do not force
every internal disk-write interleaving. The invalid-path test is not a disk-full,
short-write or failed-fsync simulation. A deadline fails the test; it cannot
forcibly stop an uncooperative OS thread if a future regression spins forever.

Further release gates should build on these command dialogues:

1. Protocol-driven uploads, interrupted publication and duplicate-credit checks.
2. Subprocess crash checkpoints with a parent watchdog and fresh-process restart.
3. Injected write, flush and rename failures, distinguishing failure before and
   after data becomes visible.
4. Maintenance/concurrent-writer contention and complete backup/restore drills.

Power-loss durability requires separate filesystem/VM testing, not merely
disconnecting a caller or killing an application process.

## User update transactions

`IcyBoard::update_user(baseline, edited, mode)` is the optimistic record-update
entry point. Callers retain the original snapshot, submit their edited copy,
and refresh the baseline only after success. Unchanged fields adopt the latest
record; conflicting edits to the same field fail without publishing any part
of the update. Error messages identify fields, never their contents. Credentials
remain one conservative conflict group governed by the recovery service.

Session updates merge cumulative statistics and accounting deltas; editor
updates treat counters and balances as explicit edits. Daily statistics are
scoped to the supplied session day, so an older session cannot overwrite a newer
day's counters. At session close, `FinalSession` keeps the stored value of
conflicting fields (including the credential group), logs the conflict and
still saves nonconflicting edits and activity deltas. Ordinary profile saves
remain strict. Identity, accounting validation and I/O errors still fail the
transaction; retries acknowledge deltas only after a successful save.
Conference flags and read pointers merge per entry. Contacts,
TPA vectors, QWK settings and bank records are atomic fields: simultaneous edits
to different elements of these fields can still conflict.

`IcyBoard::edit_users` handles creation, deletion, maintenance and recovery
transactions under the existing board lock. It stages the entire base, including
security normalization, writes it through the existing atomic-file replacement,
and publishes it only after a successful save. `save_userbase` remains a
compatibility wrapper; callers must not mutate live records before calling it
when they require rollback of those mutations.

Regression coverage is in `user_store_tests`, `state::user_update_tests`, VM
`user_snapshots`, recovery tests and the ICBSM editor/list tests. It covers
independent and conflicting two-node edits, repeated saves without duplicated
charges/counters, stale credentials, map updates, failed saves and retries.

The guarantees apply to writers sharing one loaded `IcyBoard`. They do not
coordinate separate BBS/admin processes or manual file edits. Keep external
user maintenance offline. Identity currently uses the exact original primary
name plus first-logon date, not a persistent UUID: a renamed/deleted record
requires reloading, and deliberate reuse of both identity values is not detected.
This does not add cross-process locking, a transaction journal or stronger
power-loss guarantees. User/group and user/message-file writes remain separate
transactions.