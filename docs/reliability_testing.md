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

Runtime callers submit `IcyBoard::write_users` requests. A bounded queue and one
dedicated thread serialize checks, merges, security normalization and atomic
file replacement. The writer takes cheap copy-on-write user/configuration
snapshots under a short board lock, then releases it before cloning user records,
serializing or performing file I/O. It briefly reacquires the lock to publish
the saved base and revision; only then does the caller receive success.
Readers retain the previous committed snapshot while a write is in progress.
The callback's `UserWriteContext` exposes read-only snapshots and staged
`edit_users`/`update_user` methods, not a mutable live board.

Statistics use separate `write_statistics` payloads in the same ordered queue;
their existing log-only disk-error policy is unchanged. Live-admin configuration
transactions and backups deliberately share this FIFO with user/statistics writes.
Policy and path changes cannot interleave with an active user write. A slow backup
or save delays every later writer; this is ordering, not a write-throughput
improvement. Snapshot readers retain short board-lock access during persistence
I/O. Captured configuration snapshots remain stable; capturing again observes
the latest published configuration.

An accepted request completes even if its waiting caller is cancelled. Sessions
retain the pending request and its acknowledgement, reconcile later local edits,
and advance saved baselines before retrying or switching users. This prevents
reposting already committed accounting deltas after cancellation; it does not
add process-crash recovery.

`flush_persistence` waits for prior requests; it is not an aggregate success report
for their individual writes. Synchronous `IcyBoard::edit_users`
and `update_user` remain available to offline tools and share the merge code;
they return `WriterBusy` while the runtime writer holds its transaction gate.
Both synchronous and queued operations fail closed with `WriterPoisoned` if the
gate is poisoned: an interrupted transaction may have left disk and memory
inconsistent. Poison is not cleared or treated as a stopped worker; committed
state must be verified before restarting. `WriterStopped` denotes failure to
obtain a worker acknowledgement, while a caught callback panic is `WriterPanicked`.
`save_userbase` remains an offline compatibility wrapper.

The public `users`, `statistics`, `config` and `persistence_writer` fields remain
available for offline struct literals and initialization. This boundary is a
caller contract, not type-enforced: direct live mutation bypasses the gate and
can be silently overwritten by an in-flight worker. Holding the board lock alone
does not prevent this, since the worker releases it during I/O. Replacing the
writer also breaks shared ordering. Stop producers and drain accepted work before
direct offline mutation; guarded offline user edits can coexist with an idle
worker but reject an active or poisoned gate. Runtime mutations must use
`write_users`, `write_statistics` or `ordered_persistence`. A snapshot-identity
check before disk I/O cannot close this race, and detecting a conflict after
commit would not roll back the durable write.

Regression coverage is in `user_store_tests`, `state::user_update_tests`, VM
`user_snapshots`, recovery tests and the ICBSM editor/list tests. It covers
independent and conflicting two-node edits, repeated saves without duplicated
charges/counters, stale credentials, map updates, failed saves and retries.
`persistence_tests` additionally pauses the actual file-write path immediately
before `sync_all`, checks that reads and the async executor remain available,
and verifies FIFO merges, delayed acknowledgements, cancellation, failed writes,
panic isolation, poison rejection, idle offline edits, busy-gate rejection and
shutdown barriers. `UserWriteContext` doctests pair forbidden direct mutation
examples with a compiling read/staged-write example using the same API. Snapshot
tests check copy-on-write isolation and unchanged TOML formats; live-admin tests check ordered config
publication and retention of live settings after failed persistence.

The guarantees apply to writers sharing one loaded `IcyBoard`. They do not
coordinate separate BBS/admin processes or manual file edits. Keep external
user maintenance offline. Identity currently uses the exact original primary
name plus first-logon date, not a persistent UUID: a renamed/deleted record
requires reloading, and deliberate reuse of both identity values is not detected.
This does not add cross-process locking, a transaction journal or stronger
power-loss guarantees. User/group and user/message-file writes remain separate
transactions.