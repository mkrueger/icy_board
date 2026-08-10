# icbadmin

The web administration interface hosted by a running IcyBoard.

## What it is

- An **optional, additional** administration UI for a board configuration.
- A library, not a program. `icboard` starts it when `board.web_admin.enabled` is set
  and shows the URL plus access token in the node monitor (icbmoni).
- A thin HTTP layer on top of the existing `icy_board_engine` configuration types.
  Every read and write goes through the engine's own load/save path, so there is no
  second configuration model and no duplicated validation.
- Localhost-only by default.

## What it is not

- **Not** a full replacement for `icbsetup` or `icbsysmgr`. Those remain fully supported
  and are the only way to administer a board that is not running.
- Not a remote multi-user management platform. There are no user roles, no file browser,
  no shell access and no way to run doors or PPEs.
- Security levels, QWK/FTN networking, colour themes and sysop password changes stay in
  the TUI tools.

There is deliberately no standalone `icbadmin` binary. A second process writing the same
files while the board is running would leave the running board with a stale configuration,
so the only writer is `LiveAdminBackend`, which updates the running board and the files
together.

## Enabling it

In `icbsetup` open **Board Configuration** and enable the web administration, or set it
directly in `icboard.toml`:

```toml
[board.web_admin]
enabled = true
address = "127.0.0.1"
port = 8787
allow_remote = false
```

`icboard` then prints the URL and token to the log and shows both in the node monitor.

### Pages

- **Overview** – board identity, counts, statistics, path checks and warnings.
- **Board & Sysop** – identity, IEMSI, WHO options, sysop display options, external
  editor, theme, and `board.web_admin` listener settings.
- **File Locations** – system, display, survey and trashcan paths.
- **Connection Information** – Telnet / SSH / secure WebSocket listeners.
- **Event Setup** – timed event controls.
- **Subscription** – subscription mode defaults.
- **Messages** – scan and composition behaviour.
- **File Transfer** – upload credits, batch options, free-space guards.
- **System Control** – closed board, password storage, logon policy.
- **Configuration Switches** – display, registration, logging and door options.
- **Limits** – timeouts, password ageing, sysop page window.
- **Function Keys** – F1-F10 macros.
- **Accounting Configuration** – accounting mode and peak windows.
- **New User Options** – default security/groups and NewAsk questions.
- **Conferences** – create, edit and delete conferences.

The sidebar follows the order `icbsetup` uses for the same settings.

### Access token

A random token is generated on every start and written to the log. Set `ICBADMIN_TOKEN`
before starting `icboard` to use a fixed one.

The token is intentionally not a command line option, so it does not end up in the
process list or the shell history.

## JSON API

| Method | Path                                      | Description                      |
| ------ | ----------------------------------------- | -------------------------------- |
| GET    | `/api/health`                             | Liveness check, no board data    |
| GET    | `/api/overview`                           | Diagnostics                      |
| GET    | `/api/settings/{section}`                 | Current values plus fingerprint  |
| PUT    | `/api/settings/{section}`                 | Apply changes                    |
| POST   | `/api/settings/{section}/preview`         | Show what a change would do      |

`{section}` is one of:

`general`, `messages`, `file-transfer`, `system-control`, `switches`, `limits`,
`new-user`, `events`, `subscription`, `connections`, `paths`, `accounting`,
`function-keys`.

Authenticate with `Authorization: Bearer <token>`:

```sh
curl -H "Authorization: Bearer $ICBADMIN_TOKEN" \
  http://127.0.0.1:8787/api/settings/general
```

A `PUT` must echo back the `fingerprint` from the preceding `GET`. If the file
changed in the meantime the request is rejected with `409 Conflict`.

Browser form posts use the same backend with a session cookie and CSRF token.

## Save behaviour

Every write:

1. takes an exclusive lock on `.icbadmin.lock` in the board directory,
2. verifies the fingerprint of the configuration file,
3. validates the new values and rejects the whole request if anything is wrong,
4. copies the current file to `backups/icboard.toml.<timestamp>.bak`,
5. writes the new file atomically (temp file plus rename),
6. reads the result back and restores the backup if it is not loadable,
7. appends a JSON line to `icbadmin-audit.log`.

The mutation is applied to the in-memory board and to the on-disk configuration in the
same locked section. Paths are edited in the relative form the files use; the running
board keeps the resolved absolute form it needs.

## Security

- Binds to `127.0.0.1` only by default. A non-loopback address requires
  `web_admin.allow_remote` and logs a warning. **Do not expose this to the internet
  without a TLS terminating reverse proxy** – the server speaks plain HTTP.
- All API and UI routes except `/api/health` and `/style.css` require the token.
- Browser sessions use a `HttpOnly`, `SameSite=Strict` cookie; form submissions
  additionally require a per-session CSRF token. Bearer token requests do not
  need CSRF, because a browser cannot set that header cross-site.
- The sysop password is never sent to the client and never written by this tool;
  the settings page only shows whether one is set. Change it with `icbsetup`.
- The audit log records which fields changed. It does not record secrets.
- There are no endpoints for reading or writing arbitrary files or running commands.

## Concurrency

`icbsetup` and `icbsysmgr` do not take the board lock. **Do not run them against a board
whose web admin is in use.** Concurrent edits are detected on save via the fingerprint
check, but the safest workflow is to use one tool at a time.

## Scope

Editable through the multi-section UI/API today:

board & sysop identity, web admin listener, login connections, messages, file
transfer, system control, switches/options, limits, new-user defaults, events,
subscription, file locations, accounting and function keys.

Still use `icbsetup` / `icbsysmgr` for:

conference create/edit and menus, security expressions and command levels, user
maintenance, sysop password changes, QWK/FTN networking depth, and colour themes
beyond the theme name string.
