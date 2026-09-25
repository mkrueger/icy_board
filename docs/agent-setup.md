# Setting up a board with an AI agent

This guide is for creating **a new local Icy Board**, not migrating an existing
PCBoard installation or deploying a public service. For binaries and build
prerequisites, see [installation](../INSTALL.md); for the operator's first-call
walkthrough, see [getting started](gettingstarted.md). An agent should ask the
operator for a destination, a board name and the desired access policy rather
than inventing public-facing settings. Keep generated boards outside the source
checkout unless the operator asks otherwise.

## 1. Get the tools

Use a [prebuilt release](../INSTALL.md#getting-the-programs), or build from a
source checkout:

```sh
cargo build --release
```

The release archive supplies `bin/`; a source build puts the programs in
`target/release/`. Put the chosen directory on `PATH` (or invoke its binaries
by absolute path). The board and setup tools must both be available. A build
failure is not a reason to edit the generated board or assume setup succeeded.

## 2. Create a board

Pick a **nonexistent** destination; `icbsetup create` refuses an existing one.
In the following examples, `mybbs` stands for the operator-approved destination.
**The operator should run this step in a private terminal** if agent command
output is recorded in a chat or CI log:

```sh
icbsetup create mybbs
cd mybbs
```

Creation writes a complete board and prints a random initial sysop password.
The agent can resume from the created board without seeing the password. Do not
capture or paste it into a chat log, issue, commit, screenshot or shared build
log, and do not replace it with a documented example password. The operator
should change the sysop **user account** password with the `icbsm` user editor
before allowing callers in.

## 3. Configure before running

Run `icbsetup` from the board directory in a terminal of at least 80x25. Set
the board and sysop names, confirm the node count and inspect **all** network
listeners. Its local sysop password field is **not** the initial sysop user
account password; change the latter in `icbsm`. The editors are interactive; if
the agent cannot operate them, ask the operator to make those changes or edit only
well-understood fields in the generated files using the
[configuration reference](configuration/README.md). Do not replace
`icboard.toml` with a partial TOML example: most tables are required.

In particular, a new board enables Telnet on port **1337**. An empty
`login_server.telnet.address` binds to `0.0.0.0`, **not** just localhost. For a
local test, set `address = "127.0.0.1"` in the existing
`[login_server.telnet]` table and verify the saved value before starting
`icboard`. Leave SSH and secure WebSocket disabled unless the operator requests
and configures them. See the [listener reference](configuration/board.md#login-servers)
for the exact fields and limitations. Do not open firewall ports, change a
listener to a wildcard or public address, or publish credentials without
explicit operator approval. Telnet does not encrypt passwords; use local
testing only until the operator has chosen an appropriate remote access setup.

Use the editor's normal save-and-validate path (or save the generated TOML
carefully); hand-edited unknown TOML keys may be silently ignored. Board-relative
paths start at the directory containing `icboard.toml`, not necessarily the
agent's working directory. Read the
[format reference](configuration/README.md#toml-rules-that-matter-here)
before modifying any referenced file.

## 4. Validate, then test locally

From inside the board directory:

```sh
icbsetup check icboard.toml
```

This checks the board configuration and configured paths. Investigate any
nonzero exit status and path reports rather than treating a readable TOML file
as success. Avoid `--create-dirs` in an unattended run: it offers to create
directories, including paths outside the board. A successful check does **not**
verify listener exposure, password policy or an actual login.

Only after confirming the loopback binding, start a local sysop session in a
real terminal:

```sh
icboard --localon
```

Make a test call as in [getting started](gettingstarted.md#walk-the-board-once):
join a conference, enter and read a message, inspect file areas and log off.
Optionally connect from a second local terminal with `telnet localhost 1337`
if a Telnet client is installed. Inspect `icboard.log` for errors; don't assume
the service is healthy just because the process started. Shut down the test
process when finished unless the operator asked to keep the board running.
Do not start a network listener merely to claim validation when you cannot
verify its bind address or supervise it.

## 5. Hand off to the operator

Report the board location, which installation method was used, changes made to
the generated defaults, the listener address/port, the exact checks and local
call actually performed, any failures and what remains for the operator. Do
not include passwords or private board data in the report. A path check alone
is not a completed smoke test.

For further work, use [file areas](icbfile.md), [menus](mkicbmnu.md),
[configuration formats](configuration/README.md), [feature status](feature_parity.md)
and [known limitations](known_limitations.md). For an old PCBoard installation,
follow [migration](migration.md) instead of this fresh-board procedure.
