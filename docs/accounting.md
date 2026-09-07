# Accounting: an operator's first setup

Accounting gives each caller a balance. Board activity spends it; accepted
uploads or an operator adjustment can replenish it. A **credit** is a unit you
define, not a downloaded byte, a banked minute, a payment or a cryptocurrency.
This subsystem does not collect payments. Start with a test caller and pretend
credits, not real money.

## What must be configured

Three independent settings are required:

1. The board's `[accounting]` section enables accounting and names a rates file.
2. That separate TOML file contains all **15 required rate/balance fields**.
3. The caller's matching PWRD security-level entry selects off, tracking or
   enforced accounting. A global enable alone does not enable every caller.

The caller also needs funds before enforced charging is useful. Existing users
do **not** receive the new-user grant when accounting is enabled.

### Modes

| Global `enabled` | Matching `[[level]]` settings | Result |
| --- | --- | --- |
| `false` | Any | Off: no automatic activity charges or enforcement. |
| `true` | `accounting_tracking = true`, nonempty tracking path | Tracking (legacy **T**): post usage without enforcing the balance. |
| `true` | `accounting_tracking = true`, empty tracking path | Off, **even if the level also has `enabled = true`**. |
| `true` | `accounting_tracking = false`, `enabled = true` | Enforced (legacy **Y**): post usage and check affordability. |
| `true` | Both level flags false, or no matching entry | Off (legacy **N**). |

**T takes precedence over Y.** The level's `enabled` field means accounting
enforcement, not whether that security level itself exists or grants access.
PWRD lookup uses the **first exact security-number match** whose password is
empty or matches the session password (ASCII case-insensitive). It does not
choose the next lower level. Tracking needs valid rates just as enforcement does.
There is no blanket sysop or local-call exemption: the selected PWRD mode decides.
Tracking is not a sandbox: it changes stored usage totals, which matter if you
later switch the account to enforcement. Use a disposable test account first.

## Configure it in ICBSetup

1. Open **General → Accounting Configuration**. Set **Enable Accounting Features**
   to Yes. Leave money display off and **Highest Debit Category Only** off for
   an ordinary additive credit system.
2. Enter a rates path, for example `main/accounting.toml`, in **Account
   Configuration File**. Press **F2** on that field to open the rates editor.
   A new file starts with zero rates. All rate fields have **F1** help; enter
   a new-user starting balance, warning threshold and at least one charge.
3. Set a writable **Account Tracking File**, for example `main/accounting.log`.
   Its parent directory must exist and be writable by the board process.
   Leave peak days unselected initially; optional display/holiday paths can
   remain empty. Relative paths are resolved from the board root, not the
   shell's current directory. **F4** opens the path browser.
4. Open the **Security Levels** file editor (PWRD) from file locations. Select
   the caller's level and press **Enter**. Set **Accounting Mode** to **Enforce**.
   For a first tracking trial, select **Tracking** instead; **Disabled** turns
   off automatic accounting for this level. Existing files with both flags set
   display **Tracking**, because T takes precedence. The stored boolean schema
   is unchanged.
5. Press **Esc** to leave each editor and choose **Yes** when asked to save;
   also save the parent board configuration. Rate/list files and board options
   have separate save boundaries. Restart the board after configuration edits
   so a new call loads the intended rates and paths; do not expect live calls
   to pick up edits immediately.

The rates editor rejects non-finite values before saving, and opening an
existing malformed rate file reports its load error instead of replacing it
with an all-zero draft. Negative global rates are supported for deliberate
adjustments, but beginners should use zero or positive values. A rate is in
credits (or money units) **per named activity**, never a percentage.

In the **Commands** file editor, select a command and press Enter to edit
**Charge per Use** and **Charge per Minute**. In a conference's **Doors** editor,
Tab to the door list and press Enter for the same fields. These rates must be
finite and non-negative; zero is free. Both editors validate before saving.
They store `charge_per_use` and `charge_per_minute` on the command/door record,
not inside its actions. Omitted rates default to zero.

## Copyable minimal configuration

These are separate files/sections, **not a complete board installation**.
Merge the first section into your existing board configuration rather than
creating a second `[accounting]` table. The example is deliberately simple:
100 starting credits for new users, 1 credit per successful logon, 10-credit
warning threshold, no time or transfer charge.

```toml
[accounting]
enabled = true
use_money = false
concurrent_tracking = false
ignore_empty_sec_level = false
cfg_file = "main/accounting.toml"
tracking_file = "main/accounting.log"
peak_usage_start = "00:00:00"
peak_usage_end = "00:00:00"
peak_days_of_week = "NNNNNNN"
peak_holiday_list_file = ""
info_file = ""
warning_file = ""
logoff_file = ""
```

Create the separate rate file at the `cfg_file` path. This is a **root record**,
without an `[accounting]` wrapper. Do not omit any of these fields: Rust's
all-zero `Default` does not supply missing TOML fields.

```toml
new_user_balance = 100.0
warn_level = 10.0
charge_per_logon = 1.0
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
```

Edit the existing matching record in the file named by `paths.pwrd_sec_level_file`.
Do not append a duplicate behind a matching record. Use the actual caller's
security number; the illustrative level 20 below is not assigned automatically.
Retain the existing unrelated security limits when editing an established board.

```toml
[[level]]
description = "Accounting test caller"
security = 20
enabled = true
accounting_tracking = false
time_per_day = 90
enforce_time_limit = true
daily_file_kb_limit = 32767
```

`32767` is the unlimited daily download allowance, unrelated to money. Zero
would block non-free downloads independently of accounting. A fresh user
registered with these rates and assigned this matching level starts with 100
credits; one charged logon leaves 99 if no other configured activity costs apply.
To trial tracking, set `accounting_tracking = true` and keep the tracking path
nonempty. Return to `false` to enforce balances.

## Fund an existing user safely

**`new_user_balance` is a one-time registration grant, not a login refill.**
An existing user lacking an account gets a zero account, not a fresh grant.
The current ICB System Manager user editor has no monetary account fields;
transfer statistics and time/byte banks are not a substitute.

A trusted operator PPE can add 100 credits to the **currently logged-in user**:

```ppl
GETUSER
ACCOUNT START_BAL, 100.0
PUTUSER
```

Use this only in a controlled operator workflow, after confirming the selected
user's identity and recording why the adjustment is being made. Do not publish
an unrestricted menu command that callers can repeatedly run. Running this as
the sysop funds the sysop, not another caller. Every execution adds another 100:
`ACCOUNT` **adds**, it does not set a target balance. `START_BAL` is field 0;
do not use `START_SESSION` to top up funds or reset debit history to hide debt.
For the current caller, `PUTUSER` updates session state; normal user persistence
and finalization save it. Log off cleanly and verify the balance on a new call.

For a different user, a purpose-built operator PPE may select a **verified,
existing, one-based** record with `GETALTUSER`, apply `ACCOUNT START_BAL, amount`,
then `PUTUSER`, which saves the selected alternate record in this runtime.
Check the record bounds and identity *before* adjusting it: an invalid
`GETALTUSER` leaves the previously selected record unchanged. Keep the target
offline and avoid concurrent user-manager edits. Back up the user base first.
Do not treat direct `ACCOUNT` adjustments as audited payment transactions;
maintain a separate operator record of funding and reconciliation.

## How the balance is calculated

Accounts retain accumulated debit and credit categories; the rate table is
separate. With `concurrent_tracking = false`:

**balance = starting balance − sum of debit categories + sum of credits**.

With `concurrent_tracking = true`, replace the sum of debit categories with
their **maximum**. Pending global online time is added to the time category
*before* computing that maximum. This legacy name does not mean concurrent
sessions, per-call maxima or a switch for real-time enforcement.

Example: starting balance 100, call debits 20, time debits 10, download-file
debits 30, upload credits 5. Additive balance is 45; maximum-category balance
is 75. Download-file and download-KiB charges are distinct categories.
Positive upload and special credits **increase** the balance in both modes.
Choose the policy before operation: changing sum/max also changes how stored
history is interpreted.

### Units and posting boundaries

| Activity | Meaning |
| --- | --- |
| Logon | `charge_per_logon`, once after authentication and accounting activation. Failed authentication is not a call charge. |
| Online time | `charge_per_time` or `charge_per_peak_time` per global calendar-minute boundary crossed. Preview waives one normal minute, or one peak minute if no normal minute exists; final settlement has **no grace minute**. |
| Conference time | `charge_time` per elapsed minute in the conference, **added to**, not replacing, global time. Settled on conference change/finalization; rounds up at 30 seconds. |
| Messages | Online read uses `charge_per_msg_read`; capture/QWK uses `charge_per_msg_read_captured`. Conference `charge_msg_read` is additive. Writes select **private, else echoed, else ordinary** rate, then add conference `charge_msg_write`; the three global write rates are not summed. |
| Downloads | Completed non-free file: `charge_per_download_file` plus `charge_per_download_bytes × floor(file bytes / 1024)`. Round down **each file**, not the batch total. |
| Uploads | Accepted file: positive `pay_back_for_upload_file` and `pay_back_for_upload_bytes × floor(file bytes / 1024)` earn credits. Successful manual-approval intake (`AwaitingApproval`) is credited **before approval or publication**; later administrative rejection does not reverse that credit automatically. Scanner-rejected uploads and failed publication earn none. |
| Group chat | Additional `charge_per_group_chat_time` per rounded elapsed minute; does not stop global/conference clocks. |
| Commands/doors | One per-use charge per invocation plus rounded elapsed minutes. A command's multiple actions do not multiply its per-use fee. Door fees begin only after successful launch/connect, intentionally unlike DOS pre-spawn charging. An invoking command and its door can both charge. Usage posts to the TPU category. |

One **KiB = 1024 bytes** despite the legacy `*_bytes` names. For example, a
1536-byte non-free file posts one KiB, whereas admission estimates 1.5 KiB.
Download preflight also estimates time at the **normal global time rate** from
connection speed, and accounts for queued costs. It is not the final invoice:
peak/conference time and actual duration can differ. Free files/directories
waive file/KiB debits, **not online time**. Capture/QWK delivery uses message
capture charges without an additional ordinary file/KiB download fee.

Command/door admission checks the per-use fee plus one minute (and pending
invoking-command costs for a door). This is not a minimum one-minute bill or
a transactional funds reservation. Elapsed activity time rounds at 30 seconds:
29 seconds costs zero minutes; 30 seconds costs one. Handler errors after an
activity started do not erase elapsed charges. Ordinary menu/logoff unwinding
settles activity before central accounting finalization.

### Peak time and holidays

Peak classification uses the server's **local date and time**, a Sunday-first
seven-character Y/N mask (`NYYYYYN` means Monday–Friday), and inclusive HH:MM
endpoints. Start 09:00/end 17:00 includes the 17:00 minute. Start 22:00/end
02:00 crosses midnight; equal endpoints mean all day on selected days.
The local date of each minute determines the day mask and holiday status.

The holiday file is plain text, **not TOML**, regardless of its extension.
Each line is an `MM-DD-YY` pattern; uppercase **X** matches one digit:

```text
12-25-XX
01-01-XX
09-07-26
```

A matching date uses the normal rate for its entire day. Invalid lines do not
match. An unreadable holiday file logs a warning and leaves no holiday
exemptions; ensure it exists before enabling peak fees. An empty path disables
holiday handling. Global billing walks actual minute boundaries and classifies
each local minute, handling midnight, multi-day calls and daylight-saving
changes deliberately more consistently than original DOS clock quirks.

## Enforcement, display and finalization

Enforced mode checks affordability in supported activity paths, including
download selection/flagging, message posting/capture, and command/door admission.
These checks are not a blanket preflight on every operation or an upload-receive
affordability gate. It can shorten the available online time based on balance
and warns at or below `warn_level`;
the warning re-arms after funds rise above that threshold. Costless activities
can still be usable at zero balance. Tracking mode posts usage but neither
denies activity for insufficient credits nor applies the empty-account drop.

At empty balance, `drop_sec_level` can impose a **lower session security ceiling**,
not permanently rewrite the user's normal security level or promote them.
The new matching PWRD entry is then resolved again. `ignore_empty_sec_level`
suppresses this drop, **not** affordability checks or time caps. Configure the
target level consciously; an accounting-off target is no longer charged.

`info_file` explains the account at login; `warning_file` is used by balance
checks; `logoff_file` is displayed on ordinary nonautomatic logoff. Empty paths
omit the display. A missing configured display is not equivalent to an empty
path. The accounting logoff display and final summaries follow settlement of
pending time and **all enclosing command/door invocations**, then successful
final account persistence. Invocation-settlement or finalization/save errors
suppress those displays rather than showing an incomplete final balance.
Disconnected callers may not receive them. Check persistence and the error log
rather than treating terminal output as a durable receipt.

Useful macros for these display files:

| Macro | Meaning in active accounting |
| --- | --- |
| `@CREDSTART@` | Account `starting_balance`, not the current call's opening balance. |
| `@CREDNOW@` | Opening balance of this call minus current balance: **this call's net usage**, not remaining credit. |
| `@CREDUSED@` | Account starting balance minus current balance: cumulative net usage. |
| `@CREDLEFT@` | Current enforced balance; shows unlimited in tracking/off modes. |

Off mode also shows unlimited for `CREDSTART` and zero for `CREDNOW`/`CREDUSED`.
`PCBACCSTAT(ACC_STAT)` reports 0/off, 1/tracking, 2/enforced; numeric balance
queries are not display text. Money display is deterministic US-dollar style
(`$1,234.50`) with exactly two decimals, **not host-locale currency selection**.
Credits display comma grouping and up to six decimal places with trailing zeros
trimmed (`1,234.5`), matching DOS fractional-credit precision. Display rounding
does not round the stored floating-point balance or the rates. Banked time/bytes
and file/byte ratio-credit macros remain separate.

Mid-call profile saves, including W and LANG, leave accounting active and its
clocks running; repeated saves post only newly elapsed whole-minute statistics.
The runtime finalizes authenticated sessions after normal return and returned
connection/handler errors: it settles pending global/conference time and saves
the user. A failed save can be retried without reposting settled time. That is
not protection against process death, power loss, task cancellation or every
possible custom PPE control flow.

## Tracking files, safety and current limits

* A case-insensitive `.DBF` extension selects a dBase III table. All other
  extensions select fixed-width **ASCII with CRLF**, not CSV, TSV or TOML.
  Fields are Date, Time, Name, NodeNumber, ConfNumber, Activity, SubAct,
  UnitCost, Quantity and Value. Numeric amounts have four decimal places in
  the audit, not the full precision of the account. Controls become spaces,
  non-ASCII characters become `?`, and names/descriptions have fixed widths.
* A persistent sibling `<tracking filename>.lock` coordinates cooperating
  writers across append/create/validation. Do not remove/replace it while
  writers run. Arbitrary DBF editors and PPE DBF operations need not honor
  this lock. Stop writers before maintenance or log rotation. Corrupt or
  incompatible DBFs and numbers too large for their fields are rejected.
* Audit errors are logged **after** the monetary posting. A failed audit write
  does not roll back a debit, and retrying the activity can charge again.
  Watch the board's error log for `Accounting charge POSTED but tracking failed`
  and for finalization/save errors. Zero-cost automatic usage is logged in
  tracking mode, but normally omitted in enforced mode. The local-call
  statistics exclusion suppresses local audit output, not local charges.
* Monetary deltas merge across sessions sharing the board's user base; this
  does not reserve funds across nodes or serialize separate board processes
  and external user editors. Do not assume concurrent calls cannot overspend.
* There is **no transactional ledger**, atomic account-plus-audit commit,
  payment processing, crash recovery journal, or financial-grade guarantee.
  Floating-point and fixed-width audit amounts need operator reconciliation.
* Legacy per-file **NoTime/FSEC monetary transfer-time refunds are unsupported**.
  A free download still accrues time; ordinary transfer time-credit settings
  are not a monetary refund implementation.

Before enabling real charges, back up rates/security/user files, test registration
and an existing funded caller, inspect one successful activity and one failed
activity, log off and reconnect, then compare the balance and tracking output.
Also test the selected empty-account level and an unwritable tracking path on
a disposable board. This is an operator checklist, not a claim that those
checks have been run for your configuration.

See the [board schema](source/configuration/board.rst),
[component schemas](source/configuration/components.rst),
[user data schema](source/configuration/data.rst) and
[known limitations](known_limitations.md) for storage details and boundaries.