# Message entry and reading compatibility audit — 2026-09-06

## Implementation follow-up (same day)

The audit below is the **pre-fix snapshot**, not the current support matrix.
The follow-up implements substantial native message functionality, but does not
claim exhaustive or byte-identical PCBoard compatibility.

### Implemented and regression-tested

- Editor: Q/Ctrl-O quoting, bounded native U text upload and SA attachments,
  SN/SK results, stacked line commands, original substitution dialog, Unicode-safe
  editing/wrapping, tab/join/exit controls, word movement, width toggle and pages.
  Fragmented terminal key decoding now handles Delete, F1 and correct PageUp/Down.
- Reader: body EDIT, FORWARD, attachment view/flag, native X export, C/D/Z capture,
  QWK export, authorized all-conference scans, selection/WAIT, header lengths,
  aliases/help, in-loop search/thread changes and multiple ranges.
- Successful-read pointers are monotonic and honor O/configuration; unread filtering
  uses message status. Recipient status/date and idempotent return receipts are
  persisted. Read effects are bound to the displayed snapshot across terminal waits.
- Sender/group password semantics are distinguished in native reading and protection
  changes. Entry and reply enforce destination/read/write security. Replies carry
  numeric and network references, original quote context, own-message/RO addressing,
  and native reply dates. Email replies use the actual email base. Both standalone
  and reader SK save first and perform authorized, snapshot-checked deletion.
- MOVE/COPY preserve metadata and enclosure files, clear cross-base numeric thread
  links, fail closed on destination errors, synchronize the destination before MOVE
  deletion, and reject concurrent source replacement. Header/body edits detect
  conflicting writes. Deletion validates sender/recipient/admin permissions.
- Explicit local-only entry choices are honored by FTN echomail and QWKnet export.

### Carbon copy: yes

`SC` saves the original then asks for successive carbon-copy recipients, subject
to the original global enablement and comment/long-header restrictions. Copies have
independent JAM storage and message IDs, retain security/thread/expiry metadata,
and do not inherit a previous recipient's network address or read state.
Deleting one copy does not delete the others. Persistence is tested through both
the engine and actual BBS command flow.

`@LIST@` entry collects bounded recipients under conference CarbonLevel/CarbonLimit
permissions, **independently of the global SC switch**, as in original PCBoard.
It currently expands into separate private JAM messages, not a shared PCBoard
list message with per-recipient extended-header status records. That representation
is an explicit remaining list-compatibility distinction.

### Corrections to the initial audit

- Original `entermessage()` skips security selection in NoPrivateMsgs conferences;
  the initial E6 allegation about skipping that dialog was incorrect.
- The original initial subject prompt also limits entry to 54 characters. E5's
  limit alone was not evidence of a defect; extended reply/header flows differ.
- CarbonLevel/CarbonLimit belong to `@LIST@`, not sequential SC copying. Adding those
  restrictions to SC would introduce a compatibility difference.

### Remaining compatibility boundaries

- Reader NET capture is explicitly rejected: per-user network routing, loop/tag
  handling and NETFLAGS export are not implemented by this reader adapter.
- Native X downloads a raw JAM header/body file; it is **not** the original
  PCBoard export format or EXPORT.BAT hook.
- Capture omits attachments with an explicit warning; optional QWK screens and
  native ASCII/external transfer-command support are not implemented here.
- QWK export is tested; the separate existing REP importer is not certified for
  round-trip conference mapping, private attributes, or destination authorization.
  Do not interpret reader QWK export support as full offline-mail round-trip parity.
- PPL compatibility accessors still need reconciliation with native sender-password
  and reply-date markers; maintaining native data alone does not complete those APIs.
- Header recipient editing and some ALL continuation contexts remain less complete
  than original PCBoard. Display-cell/grapheme-width parity is not guaranteed.
- No new live PCBoard oracle transcripts were captured. Behavior is source-grounded;
  actual native transfers were tested with paired ChannelConnection/Zmodem peers.

### Final validation

- Engine library: **1,815 passed, 5 ignored** with one independently failing
  ZConnect configuration fixture excluded. An unfiltered run identifies that fixture
  as `zconnect_settings_save_reload_and_path_resolution` (missing language-file path).
- Full icboard binary suite: **265 passed** in explicit English.
- Final German message validation: **236 engine user-command tests**, **27 entry
  integration tests**, and **51 reader integration tests** passed. A prior full
  German icboard run also passed before the final targeted race/SK refinements.
- Three real paired-Zmodem regressions cover upload text, save attachment and reader
  capture, including bytes, cancellation, unsafe names and temporary-file cleanup.
- Whitespace validation passed. Unrelated workspace changes were preserved;
  no commits or live board-data changes were made by this implementation task.

## Verdict and evidence

**No: basic message entry/reading works, but full PCBoard parity is not implemented.**
Recognizing the original command grammar is not equivalent to executing it.

Compared current Rust implementation with the locally available original
PCBoard `MSGENTER.C`, `MSGREAD.C`, and `MESSAGES.C`. Original function names
below identify the reference behavior without reproducing proprietary code.
The existing [command audit](../compat/COMMAND_AUDIT.md) records earlier live
PCBoard 15.4 prompt checks; those are not new verification of advanced features.

No new live oracle results: DOSBox-X was running, but its configured runtime
oracle TCP port 2323 had no listener. The existing DOSBox process was not stopped
or reconfigured. Findings below are source comparisons unless explicitly labeled
as existing test coverage. No runtime code or live board data was changed by this audit.
Other work was modifying this workspace concurrently, including editor disconnect
handling; this report is a dated snapshot, not a claim about later changes.

## Implemented baseline

- E: recipient prefill and prompt, exact recipient lookup, subject, read-only
  conference/write-level checks, security prompts, receipt-request flag,
  routing/newsgroup fields, saving, and optional SC carbon copies.
- Both line and full-screen editors exist. Basic insertion/deletion, navigation,
  full-screen reflow, save/abort, mode selection, and the help footer exist.
  This is **not** a claim of editor/key-by-key parity.
- R: basic ranges, backward/forward reading, initial text/name/date/personal
  filters, pagination, sender/recipient privacy checks, password challenge,
  SET/SKIP, kill, basic protect/unprotect, and header editing via E.
- MOVE/COPY and basic REPLY execute; they are not stubs, but have significant
  correctness gaps below. Header E is distinct from unimplemented body EDIT.

## Entry/editor gaps

| ID | Finding | Evidence / original comparison |
|---|---|---|
| E1 | Missing U body upload, SA attachment, SN save/next, SK save/kill, and Q quoting. Full-screen Ctrl-O is an empty branch; no original-body context reaches the editor. | [Editor command dispatch](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/editor/mod.rs), `EditResult`, `full_screen_edit`; original `msgeditor`, `receivemsgupload`, `receiveattachment`, `linequote`. |
| E2 | Line E is not PCBoard's old-text/new-text substitution dialog. Stacked editor arguments such as `D 2` are not tokenized by the exact-string command dispatch. L has no start-line argument. | [Editor](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/editor/mod.rs), `edit_message`, `edline`; original `editline`, `listmsgbuffer`, and `msgeditor` token handling. |
| E3 | Line input does not enforce/wrap at `max_line_length`, trims leading indentation on Enter, and both input loops accept only ASCII space through tilde, rejecting CP437 high characters. | [Editor](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/editor/mod.rs), `get_line`, `full_screen_edit`; original `inputline` and its translation tables. |
| E4 | Full-screen key meanings differ: Tab/Ctrl-I centers instead of advancing tab stops; Ctrl-J left-justifies instead of joining lines; Ctrl-U deletes instead of exiting. Ctrl-left iterates an empty reversed range. No original narrow/wide switch. Page movement changes viewport without the original cursor relocation. | [Editor key dispatch](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/editor/mod.rs), `full_screen_edit`; original `inputline` cases and Wide/Narrow settings. |
| E5 | Recipient validation has no original S/Soundex/U candidate selection. Long names/subjects are capped at 54 rather than the original extended-header flows. | [Entry](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/e_enter_message.rs), `get_message_recipient`; original `gettoname`, `entermessage`. JAM need not use PCBoard's header encoding, but the entry behavior is still incomplete. |
| E6 | D pack-out selection is not gated by the original KEEPMSG security level. `disallow_private_msgs` skips the entire security dialog, unlike the original rejection of R while still offering other security choices. | [Entry options](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/e_enter_message.rs), `get_message_options`, `get_message_security`; original `getsecurity`. |
| E7 | Echo Y/N affects whether routing questions are asked, but is not saved as an echo choice in the message attributes or subfields. | [Entry options](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/e_enter_message.rs), `get_message_options`; [message construction](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/c_comment_to_sysop.rs), `make_message`. This finding concerns the caller's choice, not a claim that all FTN/QWK transport is absent. |

## Reader, reply, and persistence gaps

| ID | Finding | Evidence / original comparison |
|---|---|---|
| R1 | X export, body EDIT, FORWARD, and V attachment viewing parse but have no action implementation. | [Action dispatch](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/read_actions.rs), `run_read_action`; original `readmessage` action switch. The X test explicitly expects Invalid Entry. |
| R2 | C/D/Z capture and R QWK flags are not consumed. Inside-loop capture can merely redisplay, rather than report an error. Standalone QWK functionality elsewhere does not implement the reader's capture workflow. | [Parser](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/read_command.rs), `open_capture`, `capture_single`, `open_qwk`; [reader](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/mod.rs); original `interpretcommand`, `readmessage`. |
| R3 | A/ALL/WAIT do not walk conferences. LONG/SHORT, alias toggles, and parsed help flags are not applied by the reader. | [Reader entry](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/r_read_messages.rs) opens one base; [reader loop](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/mod.rs) remains in it. Compare original `interpretcommand` and all-conference read traversal. |
| R4 | End-of-message searches/thread commands do not rebuild the filter or initialize the thread from the displayed subject. Only the first newly requested numeric range is adopted. Several outer-prompt actions also never reach action dispatch. | [Reader loop](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/mod.rs), `read_msgs_from_base`, `read_message_number`; original `interpretcommand` and thread setup. Parser unit tests do not prove runtime filtering. |
| R5 | Last-read is written once at range entry, before matching/display/password success, not after each message. Reading 1+ does not advance it to the last displayed message; reading backward can lower it. Parsed O/update-pointer suppression is ignored. | [Reader](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/mod.rs), `read_message_number`; original `readmessage` advances its pointer after reading, only forward, subject to `Read.UpdatePtrs`. |
| R6 | Reading does not mark recipient mail read, record the read/reply dates, or generate requested return receipts. U outside the loop filters by number versus last-read rather than original message unread status. | [Reader](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/mod.rs) and [filter](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/message_filter.rs); original `readmessage`, `createreceipt`, and `Read.UnReadOnly` filtering. Setting `MSG_RECEIPTREQ` at entry is not receipt delivery. |
| R7 | S sender-password and G group-password messages collapse to the same JAM password behavior. Original reading challenges group passwords, with read-all-mail exemption; current reading challenges every password-bearing message without that exemption. Header P/S/G additionally sets private for password messages, unlike new-message entry. P/U only toggles private and leaves the password requirement. | [Entry security](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/e_enter_message.rs), [reader](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/mod.rs), [header/protection actions](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/read_actions.rs); original `getsecurity`, `readmessage`, `protect`, `unprotect`. |
| R8 | Replies always target the original sender, including replying to one's own message. RO does not ask for another recipient. The original message number is not persisted as the new message's `reply_to`; only available network MsgID is copied as ReplyID. Original replied status is not updated. | [Reply](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/reply_message.rs), `reply_details`, `reply_message_command`; [construction](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/c_comment_to_sysop.rs), `make_message`; original `enterreply` and reader reply handling. |
| R9 | Direct REPLY reads a header by number without applying reader private/password checks and omits the per-conference write-level check that E performs. It also asks for return receipt unconditionally, unlike the original private-message/security-level gating. | [Reply](../crates/icy_board_engine/src/icy_board/state/user_commands/pcb/reply_message.rs); original read-access path and `enterreply`. This is an access-control discrepancy, not a claim that this path displays the original body. |
| R10 | MOVE/COPY reconstructs only from/to/subject/text/attributes/reply_to, replaces the date, and drops password CRC, expiry, network IDs/addresses, and attachment/other subfields. The copied numeric reply reference can point to an unrelated destination message. | [Copy helper](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/read_actions.rs), `copy_message_to_conference`; original `movemessage`. Preserving the attribute bits alone does not preserve message security/metadata. |
| R11 | MOVE has a data-loss failure path: destination base-open failure is displayed but returned as success by `send_message`; copy then reports success and MOVE deletes the source. Source deletion errors are also ignored. | [Save helper](../crates/icy_board_engine/src/icy_board/state/functions.rs), `send_message`; [move action](../crates/icy_board_engine/src/icy_board/state/user_commands/mods/messagereader/read_actions.rs). Source-confirmed control flow; not fault-injected during this audit. |

## Validation and next steps

Existing focused tests passed during this audit:

| Suite/filter | Passed |
|---|---:|
| Engine message-reader module | 43 |
| Engine editor module | 24 |
| icboard `tests::cmd_e` | 8 |
| icboard `tests::cmd_r` | 28 |
| **Total** | **103** |

These suites test useful baseline behavior, parser results, and prompt presence.
They do not establish end-to-end parity for the findings above. No new regression
tests or fault-injection probes were added. An unrelated compiler warning in
ZConnect polling appeared during the engine runs.

Recommended order: (1) MOVE failure/metadata preservation and reply access checks;
(2) password semantics, read-pointer/read-status/receipt lifecycle and reply links;
(3) wire the accepted reader options into execution; (4) implement missing
attachment/capture/editor workflows and exact editor input semantics.
Add persistence assertions and source-derived regression tests, then live oracle
transcripts for key sequences, routing/header edge cases, and transfer workflows.
Do not report a percentage of parity from the count of recognized commands.