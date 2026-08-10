# icbadmin

A local web interface for administering an IcyBoard installation.

## What it is

- An **optional, additional** administration UI for a board configuration.
- A thin HTTP layer on top of the existing `icy_board_engine` configuration types.
  Every read and write goes through the engine's own `IcbConfig` load/save path,
  so there is no second configuration model and no duplicated validation.
- Localhost-only by default.
- Available as a standalone binary and as a library (`AdminService` offline, `LiveAdminBackend` for an in-process board).
- When `board.web_admin.enabled` is set, `icboard` starts the live admin server itself and
  shows the URL plus access token in the node monitor (icbmoni).

## What it is not

- **Not** a full replacement for `icbsetup` or `icbsysmgr`. Those remain fully supported.
- Not a remote multi-user management platform. There are no user roles, no file browser,
  no shell access and no way to run doors or PPEs.
- Conference CRUD, security expressions, QWK/FTN depth, colour pickers and sysop
  password changes stay in the TUI tools for now.

## Usage

```sh
# defaults to 127.0.0.1:8787
icbadmin /path/to/icboard.toml

# different port
icbadmin --bind 127.0.0.1:9000 /path/to/icboard.toml

# allow a non-loopback bind (requires a reverse proxy in front)
icbadmin --bind 0.0.0.0:8787 --allow-remote /path/to/icboard.toml
```

On start the tool prints the URL and an access token. Open the URL, sign in with
the token, and use the sidebar to open configuration sections.

### Pages

- **Overview** – board identity, counts, statistics, path checks and warnings.
  Still works when the board fails to load and then shows the load error.
- **Board & Sysop** – identity, IEMSI, WHO options, sysop display options, external
  editor, theme, and `board.web_admin` listener settings.
- **Connections** – Telnet / SSH / secure WebSocket listeners.
- **Messages** – scan and composition behaviour.
- **File Transfer** – upload credits, batch options, free-space guards.
- **System Control** – closed board, password storage, logon policy.
- **Switches & Options** – display, registration, logging and door options.
- **Limits** – timeouts, password ageing, sysop page window.
- **New Users** – default security/groups and NewAsk questions.
- **Events** – timed event controls.
- **Subscription** – subscription mode defaults.
- **File Locations** – system, display, survey and trashcan paths.
- **Accounting** – accounting mode and peak windows.
- **Function Keys** – F1–F10 macros.

### Access token

By default a random token is generated on every start and printed to the console.
Set `ICBADMIN_TOKEN` to use a fixed one:

```sh
ICBADMIN_TOKEN=... icbadmin /path/to/icboard.toml
```

The token is intentionally **not** a command line option, so it does not end up in
the process list or the shell history.

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

`LiveAdminBackend` applies the same mutation to the in-memory board and the on-disk
configuration, then reloads from disk for the fingerprint/backup path.

## Security

- Binds to `127.0.0.1` only by default. A non-loopback address requires
  `--allow-remote` (or `web_admin.allow_remote` when embedded) and prints a
  warning. **Do not expose this to the internet without a TLS terminating reverse
  proxy** – the tool speaks plain HTTP.
- All API and UI routes except `/api/health` and `/style.css` require the token.
- Browser sessions use a `HttpOnly`, `SameSite=Strict` cookie; form submissions
  additionally require a per-session CSRF token. Bearer token requests do not
  need CSRF, because a browser cannot set that header cross-site.
- The sysop password is never sent to the client and never written by this tool;
  the settings page only shows whether one is set. Change it with `icbsetup`.
- The audit log records which fields changed. It does not record secrets.
- There are no endpoints for reading or writing arbitrary files or running commands.

## Concurrency

The lock only protects against other `icbadmin` instances. `icbsetup` and
`icbsysmgr` do not take it. **Do not run them against the same board while
icbadmin is being used to make changes.** Concurrent edits are detected on save
via the fingerprint check, but the safest workflow is to use one tool at a time.

## Scope

Editable through the multi-section UI/API today:

board & sysop identity, web admin listener, login connections, messages, file
transfer, system control, switches/options, limits, new-user defaults, events,
subscription, file locations, accounting and function keys.

Still use `icbsetup` / `icbsysmgr` for:

conference create/edit and menus, security expressions and command levels, user
maintenance, sysop password changes, QWK/FTN networking depth, and colour themes
beyond the theme name string.
