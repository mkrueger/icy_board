# Sysop administration

This chapter is the operating checklist for a board you own. It connects the
detailed configuration and utility references; it is not a replacement for
backups or for testing with a normal caller account.

## Before the first call

1. Install one consistent set of binaries. Record `icboard --version`.
2. Create a **new** board in a nonexistent directory, or follow
   [migration](../migration.md) for an existing PCBoard installation.
3. Change the generated sysop **user account** password in `icbsm`. The local
   setup password is a separate setting.
4. Inspect every listener before starting the board. A generated board enables
   Telnet on port 1337; an empty Telnet address binds to all IPv4 interfaces.
5. For a local-only trial, set `address = "127.0.0.1"` in the existing
   `[login_server.telnet]` table and leave other listeners disabled.
6. Run `icbsetup check icboard.toml`, then make a real local call.

Do not publish the initial password in screenshots, logs or issue reports.
Telnet is unencrypted. `--localon` starts a local session but does not disable
network listeners. See [getting started](../gettingstarted.md) and
[listener configuration](../configuration/board.md#login-servers).

## Routine operation

Start `icboard` from the board directory, or pass the board configuration
explicitly. The call-waiting screen provides local login, session inspection,
logs, status and event monitoring; see its
[complete reference](../source/icy_board.rst).

Before allowing remote callers:

- Confirm the bind addresses and ports that actually opened.
- Test a normal account as well as the sysop account.
- Check registration, menus, message entry/readback, file listings and logoff.
- Test the configured transfer protocol with a compatible terminal client.
- Review logs for missing artwork, failed PPEs and unavailable data files.

Only expose listeners and open firewall rules intentionally. SSH and secure
WebSocket have different setup requirements from Telnet; a successful local
Telnet call does not validate those transports.

## Users, groups and access

Use `icbsm` for users, groups and user-maintenance security tables. Make user
data changes while other writers are stopped; do not bypass board locks.

Configure policies at their owning level:

| Policy | Reference |
| --- | --- |
| Registration questions and initial account values | [New users](../configuration/board.md#new-users) |
| Password encoding and operator identity | [Board and operator](../configuration/board.md#board-and-operator) |
| Command permission thresholds | [Security configuration](../configuration/board.md#security-configuration) |
| Security-level time and transfer allowances | [Security levels](../configuration/components.md#security-levels) |
| Conference access and registration | [Conferences](../configuration/components.md#conferences) |
| Area-specific read/write permissions | [Message areas](../configuration/components.md#message-areas) |
| Account expiry | [Subscriptions](../configuration/board.md#events-accounting-and-subscriptions) |
| Balance tracking and enforcement | [Accounting](../accounting.md) |

When access is denied, identify the exact action and account before raising
security levels. Menu visibility, command permission, conference admission and
message visibility are different checks. A sysop test alone can hide mistakes.

### User-file maintenance

Packing removes selected users; it is not simply file compression. Preview it
before writing:

```sh
icbsm --pack --dry-run icboard.toml
```

The batch tool supports age and never-logged-on selection, protected security
levels, locked-out handling and telephone-number standardization. Its built-in
backup and `--undo` are useful recovery aids, not a substitute for a board backup.
See [the exact options](tools.md#icbsm-users-and-groups).

## Conferences and messages

Use `icbsetup` to create or edit conferences and their area lists.
Treat a conference as a group of resources and a message area as one JAM base.
Choose whether it is public, restricted or networked before admitting callers.

For each new area:

1. Set its name, base path and access policy.
2. Add network mapping only if it is intended to exchange mail.
3. Validate configured paths.
4. Join with `J`, switch with `AREA`, write with `E` and read with `R`.
5. Repeat as a caller who should be denied access.
6. Test private and personal mail separately from public messages.

Do not manually renumber or copy individual live JAM components. Back up a
consistent stopped board and its complete message-base files.
The [component reference](../configuration/components.md#message-areas)
defines the fields; the [command reference](commands.md) explains the caller
experience.

## File areas and uploads

The [file-area guide](../icbfile.md) includes a complete new-area workflow,
listing import, metadata precedence, scanning and backup. Use it rather than
inventing a `DIR` filename layout.

A typical maintenance cycle is:

1. Stop conflicting writers and take a backup.
2. Review configured directories with `icbfile areas`.
3. Import listings or scan physical files as appropriate.
4. Check for missing physical files before using a pruning option.
5. Read the result with `icbfile list`.
6. Test listing and downloading as a normal caller.

Uploads add another pipeline. Configure destination directories, size/space
limits, descriptions, scanner policy and quarantine through
[upload processing](../upload_processing.md). A received file is not necessarily
a published file. Inspect quarantine reports rather than manually copying
rejected content into public areas.

## Events and mail exchange

Use the built-in scheduler for tasks that need coordination with a running
board. **Maintenance** events drain managed writers and reload the board;
**Online** events do not and must not modify live board data.

Read [events](../events.md) for schedules, admission gates, history,
manual runs and failure recovery. Commands run in the foreground; a warning
threshold is not a forced execution timeout.

Mailer jobs belong in the coordinated maintenance workflow when they modify
live bases. Raw `icbmailer` does not acquire the board lock itself.
FTN, QWKnet and ZCONNECT have different queues, checkpoints and peer
requirements:

- [FTN/BinkP](../source/mailer.rst)
- [QWKnet](../qwknet.md)
- [ZCONNECT](../zconnect.md)

Do not delete checkpoints or repeat acknowledgements as a generic way to
clear an error: that can lose or replay network traffic.

## Backups and restoring

A useful backup includes:

- Main and referenced configuration files.
- Users, groups, statistics and accounting data.
- Complete message bases and personal mail.
- File-area metadata **and** downloadable files.
- Artwork, ICBTEXT, help, menus, PPEs and their private data.
- Event journals, mail queues and network checkpoints.
- Any configured files outside the board root.

Stop the board and independent writers before a simple filesystem backup.
Protect copies containing password hashes, SMTP credentials, private mail,
door credentials or tokens.

To test a restore, copy it to a separate location, audit absolute paths and
disable external networking/polling before starting it. Otherwise a trial
restore can write into the production board or exchange old network queues.
Run `icbsetup check`, then verify a local call and representative data. Keep
the original backup unchanged until the restored installation is accepted.

## Upgrades

1. Read release notes and [known limitations](../known_limitations.md).
2. Record the old version, binary locations and enabled external integrations.
3. Stop writers and back up the board.
4. Replace the binaries as a matching set.
5. Validate using the new `icbsetup`.
6. Test locally before opening remote admission or restarting mail jobs.

Do not assume that older binaries can read data rewritten by newer ones.
Rollback may require restoring both binaries and the matching backup.
Generated help is independently managed; review
[help ownership and regeneration](../gettingstarted.md#command-help) before
replacing customized topics.
