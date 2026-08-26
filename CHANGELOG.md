# Changelog

All notable user-visible changes to IcyBoard are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
IcyBoard is still in beta, so unstable PPL runtime 4.02 APIs may change between
releases.

## [Unreleased]

### Changed

- `Board.ConferenceCount`/`GetConference(i)` became `Board.Conferences`, the last
  of the board's count-and-accessor pairs. The board shares its conference list
  rather than copying it on every read.
- The language server no longer offers a collection's internal getter as a name,
  and completion now steps through an index, so `Board.Conferences[0].` offers
  what a conference has.

- `Board` is taken once per run rather than rebuilt on every access. Reading it
  copies every conference, so a PPE that touched `Board` inside a loop paid for
  the whole board on each step: walking 2000 areas through `Board` on a board
  with 201 conferences went from 146 ms to 7 ms, and no longer grows with the
  number of conferences.

- Turned the conference's `AreaCount`/`GetArea(i)` pairs into collections:
  `Areas`, `Directories` and `Doors` answer `Count`, are read with an index, and
  are walked with `FOREACH`. A collection shares the list it stands for, so a
  conference no longer carries a copy of every area and directory with it —
  reaching an area through `Board.GetConference(0)` on each step of a loop went
  from 802 ms to 7 ms over 2000 areas.

- Added `FOREACH ... ENDFOREACH`, which walks every element of an array whatever
  its rank. A matrix or a cube walks the same way a vector does, row-major, so a
  PPE no longer needs one nested `FOR` per dimension nor needs to know how many
  there are. The loop variable is a copy, `BREAK` and `CONTINUE` work as usual,
  and `IN` stays available as a variable name the way `TO` and `STEP` do.
  Indexing stays bound to the rank, so `a[i]` into a matrix is still the compile
  error it should be, and the flat step `FOREACH` walks with is the compiler's
  own rather than a function a PPE can call.
- Gave arrays members: `a.Len()`, `a.Len(dim)` and `a.Redim(...)` are the same
  calls as `Len(a, dim)` and `REDIM a, ...`, written the other way round. Only a
  declared array has them.

- Added `Session.User`, the caller's own record: identity, address, preferences,
  security, statistics and contacts in one object. It gathers what the `U_*`
  variables report, which stay unchanged for PCBoard compatibility.
- Made `Session.User` writable wherever `PUTUSER` used to write, so the object
  replaces the `GETUSER`/`PUTUSER` round trip instead of sitting beside it. A
  write lands at once. The caller's `Name` and the board's own accounting stay
  read-only, and writing one now names the member in the error. `SetNote()` and
  `SetPassword()` join the object, the latter hashing the way the board is
  configured to. The overlapping `FullScreenEditor`/`AskForEditor` flags became
  one `EditorMode` value of `Yes`, `No` or `Ask`, and `PasswordExpires` is
  reachable for the first time.
- Retired `U_CONTACT`. Contacts are reached through `Session.User` with
  `ContactCount`, `GetContact()`, `SetContact()` and `DeleteContact()`, and no
  longer need a `GETUSER`/`PUTUSER` round trip. Runtime 4.00 therefore adds no
  predefined user variable of its own.
- Replaced the `ERR()` function and the `ERRCLR` statement with static members
  on the `ERROR` type: `Error.Last()` and `Error.Clear()`. Every 4.00 concept is
  now reached through an object; `ON ERROR` stays a statement because it is
  control flow, and `FERR`/`DERR` are unchanged.
- Finished the PPL 4.00 API review: board objects expose `Valid`, `Board` keeps
  its conference snapshot, `Nodes` is `NodeCount`, multimedia capabilities and
  event/error kinds consistently say `Audio`, and terminal macro capability is
  exposed only by `Terminal.Info.TerminalMacros`.
- Reworked the runtime 4.00 object API after review. Board objects report the
  `Number` they were fetched under, counts are spelled `DoorCount`, `AreaCount`,
  `DirectoryCount` and `ConferenceCount`, `GetDir` is `GetDirectory`, and
  `Session` hands out the current `Area` and `Directory` as objects.
- Tightened the terminal facade: `Audio.Volume` is writable and replaces
  `SetVolume`, `Palette.SetRgb` is gone in favour of `Set(n, Rgb(...))`,
  `Margins.ResetAll()` and `Macros.DeleteAll()` say that they mean all,
  `Macros.StartRecord`/`StopRecord` replace `Record`/`End`, `Surface.DrawRect`
  replaces `Rect`, and `Event.ScanCode` splits the physical key out of `Code`.
- Removed the `Terminal.Sound` slot. `Audio.StopAll()` stops every channel and
  `Terminal.Info.Audio` reports whether the terminal can play at all.
- Collapsed the beta PPE runtimes 4.00, 4.01 and 4.02 into a single runtime
  4.00, which now carries the type table, the routine-reference marker and
  `U_CONTACT`. PPEs compiled by an earlier beta must be rebuilt from source;
  the PCBoard runtimes 1.00 through 3.40 are unchanged.
- Retired the object form of `ConfInfo(conf)`. `Board.GetConference(index)` and
  `Session.Conference` answer with the same `CONFERENCE` snapshot. The PCBoard
  `ConfInfo(conf, field)` function and statement are untouched. Its opcode slot
  was reclaimed rather than reserved, so every function opcode below it moved up
  by one and beta PPEs must be rebuilt from source.
- Replaced the experimental runtime 4.02 terminal globals with the `Terminal`
  facade: `Info`, `Gfx`, `Input`, `Margins`, `Palette`, `Font`, `Macros` and
  `Sound`, plus synchronized `BeginUpdate()`/`EndUpdate()` calls.
- Moved resource construction to static type members (`Surface.New`,
  `Surface.Load`, `Audio.Load`) and made graphics pacing a writable Boolean
  property.
- Replaced flat graphics, mouse, event and error constants with typed enums;
  split overloaded event data into `Action`, `Channel` and `Dropped`, and
  replaced raw button/modifier masks with Boolean properties.
- Retired `TERMSTATE` and the draft flat runtime 4.02 statements/functions.
  Their opcode slots were reclaimed rather than reserved, so the numbering has
  no beta holes left and PPEs built by an earlier beta must be rebuilt.

### Added

- Added the runtime 4.02 `Board` and `Session` objects. `Board` is a snapshot of
  the configured board and can walk its conferences without `HIGHCONFNUM()`;
  `Session` reads the call in progress live. Both are read-only and leave the
  classic `CURCONF()`, `PCBNODE()` and `U_*` surface untouched.
- `icbfile scan` now identifies archives that have no usable description and
  distinguishes files that are missing from disk.
- `icbfile scan --all` scans every area in a `file_areas.toml`; it can be
  combined with `--force` to re-extract descriptions in every area.
- Added end-to-end coverage for multiline `FILE_ID.DIZ` extraction and
  all-area scanning.

### Fixed

- File listings now keep size, date and description in their fixed columns when
  a filename longer than 12 characters wraps onto its own line.
- File-base lookup, upload duplicate checks, flagging and downloads now treat
  ASCII filename case like DOS while preserving the real on-disk spelling.
  This prevents case-variant duplicates and lets uppercase prompts work on
  case-sensitive filesystems.

## [0.2.1] - 2026-08-23

Second public beta, containing 69 commits since `0.2.0-beta.1`.

### Added

- Added PPL runtime 4.02 terminal multimedia APIs.
- Added object-based PPL APIs:
  - `SURFACE` for RGBA drawing, blitting, Sixel/JPEG XL presentation, caching
    and terminal-side scaling.
  - `AUDIO` for SyncTERM audio playback, looping, fading, volume and channel
    management.
  - `TERMINPUT` and immutable `EVENT` snapshots for keyboard, physical-key,
    mouse, overflow and sound events.
  - `TERMINFO` and `TERMSTATE` for terminal capabilities and text margins.
  - `ERROR` for consistent operation error reporting through `ERR()`.
  - Extensible `CONTACT` values for user contact details.
- Added terminal font loading and selection, palette control, text margins,
  synchronized output and DEC terminal macro recording/playback.
- Added OSC 8 hyperlink macros.
- Added SSH private-key and SSH agent authentication.
- Added Fractal, Paint, Palette and Tetris PPL demonstrations.
- Added PCBoard-compatible administration themes and directory color editing.
- Added file creation from ICBSetup editors.
- Added `SURFACE.GetPixel()`.
- Added completion for runtime 4.02 object members in `ppl-lsp`.

### Changed

- Replaced the experimental slot-based graphics API with `SURFACE` objects.
- Replaced the experimental global sound API with `AUDIO` objects.
- Replaced the experimental global event API with the singleton `TERMINPUT`
  object. `Free()` returns input to classic `INPUT` and `InKey` handling.
- Renamed the language-server executable to `ppl-lsp`.
- Renamed the VS Code package to `ppl-vscode` and added packages carrying the
  matching server for Linux, Windows and macOS.
- Added standalone `ppl-lsp` archives for Zed, Helix, Neovim and other LSP
  editors.
- Changed release artifact names to use one platform vocabulary:
  `linux-x64`, `windows-x64`, `macos-arm64` and `macos-x64`.
- Graphics and audio resources are now released automatically when a PPE exits.
- Media uploads use terminal acknowledgements so following output cannot
  overtake a large upload.
- Fractal frames are completed before presentation and use terminal-side
  integer scaling where available.
- Tetris uses unified event input and transmits only its changing game panel.
- Updated dependencies, including `icy_sixel` 0.6.

### Fixed

- Fixed SyncTERM JPEG XL scaling by using standard integer `ZX`/`ZY` options.
- Fixed SyncTERM WAV capability probing and audio playback compatibility.
- Fixed SyncTERM status-bar flashing during Tetris updates.
- Fixed keyboard handoff after event polling and timed waits.
- Fixed delayed Escape input.
- Fixed local-mode inline image placement and SyncTERM media handling.
- Fixed full-screen message editor rendering.
- Fixed GNU Screen TUI rendering.
- Fixed PCBoard mixed-type promotion, routine return-slot handling, constant
  folding and expression evaluation that does not need to await.
- Fixed PCBoard-compatible handling of PPE-backed conference join commands.
- Improved PCBoard import path errors.
- Regenerated the tree-sitter parser for the new runtime types, statements and
  constants.

### Compatibility

- Runtime 4.02 was unstable during this development cycle. PPEs compiled against
  intermediate graphics, sound or event APIs must be updated and recompiled.
- Existing classic PPL runtimes and source remain supported.

### Distribution

- Added board archives for Linux x64, Windows x64, macOS ARM64 and macOS x64.
- Added standalone language-server archives and platform-specific VS Code
  packages.
- Added the PDF manual to the release.

## [0.2.0-beta.1] - 2026-08-18

First public beta of IcyBoard 0.2.

### Added

- Added a cross-platform PCBoard-style BBS for local, telnet, SSH and websocket
  sessions.
- Added tools for board setup and PCBoard import, user management, file-base
  maintenance, FTN mail, menu/text editing, and PPL compilation/decompilation.
- Added support for classic PPL runtimes plus modern language features,
  diagnostics, formatting and editor integration.
- Added Linux, Windows and macOS release archives and a PDF manual.

### Known limitations

- No serial, modem or FOSSIL support.
- PCBoard import remains best effort and PPE-heavy installations require manual
  review.
- See [Known limitations](docs/known_limitations.md) for the maintained list.

## [0.2.0-lsp1] - 2026-08-15

### Added

- Added standalone native language-server archives for editor extensions.
- Let editor integrations download the platform language server automatically,
  while preferring a configured binary or one already on `PATH`.

## [0.1.7] - 2025-10-15

Last release before the 0.2 beta series. Earlier release history is available
from the repository tags and GitHub Releases.

[Unreleased]: https://github.com/mkrueger/icy_board/compare/0.2.1...HEAD
[0.2.1]: https://github.com/mkrueger/icy_board/compare/0.2.0-beta.1...0.2.1
[0.2.0-beta.1]: https://github.com/mkrueger/icy_board/compare/0.2.0-lsp1...0.2.0-beta.1
[0.2.0-lsp1]: https://github.com/mkrueger/icy_board/compare/0.1.7...0.2.0-lsp1
[0.1.7]: https://github.com/mkrueger/icy_board/releases/tag/0.1.7
