# PCBoard transfer prompt audit — 2026-09-07

## Evidence and safety

The actual oracle ran successfully: **PCBoard v15.4/M 250 (04-22-97 17:16)**,
under installed DOSBox-X **2026.08.02**, using the existing ORACLE TESTER fixture.
Port 2323 initially had no listener; no DOSBox process was running. No other
DOSBox instance was stopped. The unrelated Flatpak application was left alone.

The [documented runtime recipe](../README.md) was not run verbatim: it contains
a blanket process kill and deletes a live node marker. The installed batch
helper serves **one call**, contrary to the README's discussion of a loop.
Instead, [probe.py](probe.py) copied the 17 MB DOS board and FOSSIL driver to a
fresh, disposable host directory for each attempt. Only that copy was mounted
as DOS drive C. Flatpak received `--nofilesystem=host --nofilesystem=home` and
explicit access to the copy; `-defaultconf` avoided inherited user autoexecs.
The new autoexec deletes only the **copied** node marker and exits after the
copied one-shot batch. No board configuration, account default, security level,
quota, date, or command table was edited. Normal logon/logoff and temporary
file-list/description writes occurred only in the copies.

The nullmodem listener briefly used port 2323; the client connected over
127.0.0.1. This was not a local-console session. No payload was sent, no receive
or send protocol was started, and no file overwrite was requested. Only the
runner's own process group was eligible for cleanup. Final process/socket checks
found no remaining DOSBox process or port-2323 listener.

Final validation also compared the clone's downloadable-file tree with the
original snapshot: unchanged, with no probe payload file created anywhere in
the cloned board. The only changed/new clone files were the caller log, message
base, board statistics, user-network status and user database files. All eight
attempts recorded unchanged live-file hashes. Report links and source line
ranges were checked to resolve; the runner has no reported editor diagnostics.

**Primary evidence:** [final transcript](attempt08/prompts.txt),
[raw received bytes](attempt08/received.bin), [emulator log](attempt08/dosbox.log),
and [run metadata and live-file hashes](attempt08/run.json).
Final emulator exit: **0**. Both `live_board_unchanged` and
`live_compat_files_unchanged` are **true**. The former compares every live board
file and its path; the latter checks all pre-existing oracle scratch files
excluding disposable copies in the later runs. Cloned installations remain
outside this repository, at the scratch paths in each run's metadata. No original
binaries or C source were added to this audit directory.

## Observed prompts

These observations apply to the unchanged installation/account, not every
configuration or security level.

| Probe | Actual result | Evidence |
|---|---|---|
| `D`, empty filename | `Enter the filename to Download (Enter)=none?`; blank exits through `Press (Enter) to continue?` | [D](attempt08/prompts.txt#L76-L86) |
| `BD`, empty filename | Same filename question, prefixed `(1)` | [BD](attempt08/prompts.txt#L110-L120) |
| `U`, empty filename | `Enter the Filename to Upload (Enter)=none?`; blank exits | [U](attempt08/prompts.txt#L144-L154) |
| `BU`, empty filename | Same upload question, prefixed `(1)` | [BU](attempt08/prompts.txt#L178-L188) |
| `D N ZXN907.ZIP`, `BD N ZXN907.ZIP` | Missing-file error, then filename re-prompt; batch keeps `(1)` | [D](attempt08/prompts.txt#L212-L220), [BD](attempt08/prompts.txt#L248-L256) |
| `U N ZXA907.TXT`, empty first description | Description abandoned; returns to upload filename question | [abandonment](attempt08/prompts.txt#L284-L299) |
| `U N ZXB907.TXT`, one description line, blank | Description precedes protocol selection; no second filename prompt | [single upload](attempt08/prompts.txt#L327-L359) |
| `BU N ZXC907.TXT`, one description line, blank | Requests `(2)` filename; blank advances to batch-filtered protocol menu | [batch upload](attempt08/prompts.txt#L383-L415) |
| `N` at either upload protocol menu | `(N) None`; `Protocol Type for Transfer, (Enter) or (N)=abort?`; answering `N` exits without transfer | [U](attempt08/prompts.txt#L343-L359), [BU](attempt08/prompts.txt#L403-L415) |
| `BU Y ZXA907.TXT`, description, blank next filename | `(G)oodbye after Batch, (A)bort, Change (P)rotocol, (Enter)=continue?`; `A` cancels before receiver | [ready/abort](attempt08/prompts.txt#L439-L466) |

The upload description text advertises `/` for private upload, 45 columns and
20 lines. The continuation prompt is a bare `?`, not a numbered line. This
audit observed that text; it did **not** upload a private file or exercise the
five-character rejection rule.

### Exact remaining download blocker

Selecting the existing ALLFILES.ZIP with either `D N` or `BD N` prints
**“Sorry, Oracle, download bytes left available are 0”** and returns to filename
entry: [D rejection](attempt08/prompts.txt#L489-L497),
[BD rejection](attempt08/prompts.txt#L525-L533).
Logon also prints “No Security Level Match in PWRD File!”; the transcript alone
does not establish that warning as the cause of the quota.

Consequently, the existing-file download protocol menu, BD ready/edit-list
prompt, successful transfers, credit accounting, local-console transfers,
low-security fallback, automatic promotion, and NoBatchUp variants were **not
runtime-verified**. No quota or security configuration was changed to get past
this blocker. The `existing file, explicit None cancellation` transcript heading
describes the intended probe: the quota rejection prevented that cancellation
branch from being reached.

## Original-source findings (not runtime proofs)

The locally available source is gitignored and need not match every detail of
the 15.4/M binary. References below were read directly, not inferred from older
audit summaries. Main source SHA-256:
`ed8f11b702d0ec824949f3518dffb7b08a70865d03cde65ec57a69ce85e70d03`.

### Local upload and download

- [sendfile](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L2028-L2075) and
  [receivefile](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L2898-L2947)
  dispatch **external protocols first**. For an internal protocol with
  `Asy.Online == LOCAL`, they call `sendlocal` / `receivelocal`. Local disk copy
  is not a protocol named None.
- [sendlocal](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L1631-L1662) requests
  a destination directory using `TXT_LOCALDNLDPATH`, cancels on empty input,
  handles a bare drive specifier specially, and copies each selected filename.
- [receivelocal](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L2780-L2895)
  requests a source path/filespec via `TXT_LOCALUPLDPATH`. A directory uses the
  previously entered names (or the QWK reply name); a filespec can expand
  wildcards and calls `uploadpermit` before normal uploads. Message-entry input
  uses the first matching source. Empty input aborts. No-match filespec handling
  can fall through returning success without a copied file; do not infer a
  universal “not found” UI from this function.
- [localtransfer](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L1547-L1628)
  copies in 1024-byte chunks, uses buffered file handles, preserves the source
  timestamp for downloads, calculates CPS, and calls `successful`. A
  case-insensitive identical source/destination skips copying **but still records
  success**. No explicit overwrite confirmation appears here; destination
  opening uses create/write flags. It is a copy, not a source deletion.
- The path dialog is local keyboard/screen UI:
  [getlocalinput](../../pcboard/pcb-main/SOURCE/MAIN/INKEY.C#L721-L776) saves the
  screen, draws a box, temporarily forces LOCAL, and restores state afterward.
  This remote TCP capture cannot observe that console dialog.

### Upload credits

- [successful, upload branch](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L1026-L1041):
  byte credit is `(File.Size * PcbData.ByteCredit) / 10`; subtract from signed
  `DailyDnldBytes`, add to `BytesRemaining` unless it is the unlimited sentinel
  `-1`. Time credit in seconds is
  `((File.Size / File.CPS) * PcbData.UploadCredit) / 10`, only for positive CPS.
  Preserve integer division **before** multiplication. A factor of 10 is 1×,
  not 10 percent. These allowances exclude file attachments; `Status.MsgUpload`
  exits the entire accounting routine, and the credit statements exclude
  `PCB_DEMO` builds. Monetary accounting is a separate branch.
- [bad-file reversal](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L615-L629)
  reverses byte and time credit using the same formulas, also decrementing
  successful-upload counts and byte totals.
- [addtime](../../pcboard/pcb-main/SOURCE/MAIN/MISC.C#L55-L137) accepts seconds.
  Its credit-minute counter is rounded separately; the timer receives seconds
  converted to ticks. Positive credit can be suppressed if time is already
  event-adjusted, and the later event logic can cap the timer.
- [uploadcreditminutes/givecredits](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L3847-L3893)
  displays an aggregate average-CPS estimate to one decimal minute, not the sum
  of all per-file rounded minute counters. The notice requires successful
  uploads and `UploadCredit > 10 || ByteCredit > 0`.

### BU entry versus U promotion and batch restrictions

- [BU/UB dispatch](../../pcboard/pcb-main/SOURCE/MAIN/COMMAND.C#L648-L654) sets
  `Status.Batch` from the batch security threshold and dispatches using upload
  security. Below batch security it falls back to nonbatch mode; this is not
  an unconditional “BU forbidden” check. [BD/DB](../../pcboard/pcb-main/SOURCE/MAIN/COMMAND.C#L577-L581)
  behaves analogously with download security.
- [U promotion](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L4367-L4378) is a
  **separate** decision: `PromoteBatch`, no filename/command arguments remaining,
  user default not `N`, a batch-capable user default, and sufficient batch
  security. `NoBatchUp` is **not** in this predicate, nor in BU dispatch.
- [allocatefilelist](../../pcboard/pcb-main/SOURCE/MAIN/FILELIST.C#L386-L399)
  imposes upload limit **1** for `NoBatchUp` or file attachment, otherwise
  **32000**. Download limits instead use batch security and `Status.BatchLimit`.
- [internal receiver](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L2642-L2690)
  explicitly permits batch protocols under `NoBatchUp` / message entry, ignores
  the transmitted header filename, and retains the caller's selected name.
  [After the first file](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L2761-L2765)
  it sets `Stop`, preventing the next payload. Thus “disable batch uploads” does
  not mean “reject every batch-capable protocol.”
- [external upload](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L2171-L2220)
  passes an exact selected filename rather than a batch destination directory
  under these restrictions, with post-transfer name reconciliation.
- [getnames](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L3664-L3720) loops only
  until empty input or the current file limit. Single-file mode can stop after
  one accepted filename; a blanket “D always asks until empty” claim is too broad.

### None semantics

[getxferprotocol](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L3185-L3225)
forces protocol selection when the current protocol is `N`, absent, or
incompatible with batch mode. **Empty input or `N` returns failure/cancellation**;
it is not ASCII, direct transfer, or local copy. ASCII is the separate `A`
protocol. Upload cancellation is proven above; the analogous download branch
is source-only because quota screening prevented reaching its protocol menu.
[scanfornames](../../pcboard/pcb-main/SOURCE/MAIN/TRANSFER.C#L3583-L3601)
recognizes protocol letters only when passed on the command line, which is why
these probes used `U N name` rather than changing the account default.

## Attempt history and reproduction limitations

All attempts are preserved, including guard failures, not rewritten to look
successful. Initial [capture](prompts.txt) stopped at an optional message scan;
attempt02 reached the real `Main Board Command?` rather than the README's
`command:`; attempt03 encountered the post-command continuation prompt;
attempt04 completed all four baseline commands. Attempt05 discovered bare `?`
description continuations; attempt06 established U/BU None and the D quota
blocker; attempt07 captured batch-ready wording; attempt08 completed all guarded
probes, explicitly aborted BU and logged off. Each used a fresh copy.

The runner refuses existing transcript paths and known probe-name collisions,
does not register users, stops on unexpected prompts, and never answers a
transfer-start prompt with Enter. Use a new output directory for each run.
It is a fixture-specific research helper, not a general BBS automation service.
Python was configured first and the selected workspace interpreter was obtained
before execution. No Rust source was edited and no Rust test was run. Concurrent
Rust changes already visible during this investigation were left untouched.