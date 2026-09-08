# mkicbmnu

`mkicbmnu` edits Icy Board's native interactive menus in a terminal. It provides
general settings, command and action drafts, language-prompt maintenance,
undo/redo, static validation, and a read-only **Preview / Test** tab. A native
menu uses the `.mnu` extension but contains TOML, not PCBoard's legacy MNU text
format.

Start with [Getting started](gettingstarted.md) for a working board and
[Installation](../INSTALL.md) for the tools. See the
[menu schema](configuration/components.md#component-menus) and
[command reference](configuration/commands.md) for the underlying records.
This guide describes the current editor and its limits; “Test” does not mean
running a caller session.

## Open, create, or check a menu

```text
mkicbmnu [--board DIRECTORY_OR_CONFIG] [--full-screen] FILE
mkicbmnu --create [--board DIRECTORY_OR_CONFIG] [--full-screen] FILE
mkicbmnu --check [--board DIRECTORY_OR_CONFIG] FILE
mkicbmnu --help
mkicbmnu --version
```

| Option | Meaning |
| --- | --- |
| `--board`, `-b` | Use a board directory or a board configuration file. A directory selects its `icboard.toml`. |
| `--create`, `-c` | Open a new, unsaved menu. Refuse an existing destination; there is no overwrite switch. |
| `--check` | Load and check an existing menu without opening the TUI or writing files. Cannot be combined with `--create`. |
| `--full-screen`, `-f` | Use the terminal's full area rather than the default 80×25 editor. |
| `--help`, `-h` | Print command-line help. |
| `--version` | Print the version and exit. |

The filename's extension is **always replaced with `.mnu`**. Supplying a name
without an extension works; supplying another extension does not select a
different format. Without a filename, the program prints help to stderr and
exits with status 1, unless help or version was explicitly requested.

### Board context and paths

Board context is required even when creating or checking a menu. If `--board`
is omitted, the editor looks for `icboard.toml` in the menu's directory, then
each parent directory, using the first file found. This is not an independent
search from the working directory or through `ICB_PATH`. An explicitly
selected configuration that cannot be loaded is an error, not a request to
fall back to another board.

The configuration is canonicalized and loaded with `IcyBoard::load()`, including
its associated data. A board configuration fragment alone is not enough.
The configuration's containing directory supplies the board root, file-browser
base, reference choices, and administrator color theme.

Keep these two path rules separate:

* The **menu filename on the command line** remains relative to the shell's
  current working directory. `--board` does not change that directory or
  relocate the menu being edited.
* **Paths inside the menu**, such as display/help files and file-action
  parameters, are resolved against the board root. Absolute paths remain
  absolute. They are not relative to the menu's own directory.

Examples, assuming the board and destination directories already exist:

```sh
# Find the board by walking upward from the menu directory.
mkicbmnu /srv/mybbs/menus/main.mnu

# Edit ./scratch.mnu against a board elsewhere; do not edit /srv/mybbs/scratch.mnu.
mkicbmnu -b /srv/mybbs ./scratch.mnu

# Start a new menu, using an explicitly named configuration.
mkicbmnu -c -b /srv/mybbs/icboard.toml /srv/mybbs/menus/information

# Read-only checks suitable for a terminal without an interactive screen.
mkicbmnu --check -b /srv/mybbs /srv/mybbs/menus/information.mnu
```

`--create` does not write an empty placeholder. The destination appears only
after an explicit save, and its parent directory must already exist. Discarding
the new document leaves no menu file. Interactive startup does open a log beside
the board configuration, so deferred menu creation is not a promise that the
entire interactive process performs no writes. `--check` returns before logging
and terminal initialization.

## Navigation, drafts, and saving

The tabs are **General**, **Commands**, **Language prompts**, **Preview / Test**,
and **About**. Use an 80×25 or larger terminal for normal editing. Smaller
screens clip or simplify content; the command dialog asks for more room when
its available area is too small. Full-screen mode gives the forms and preview
more space; it does not change stored menu coordinates.

These application keys apply when no draft, picker, filter, or other modal
control owns input:

| Key | Action |
| --- | --- |
| Tab / Shift+Tab | Next / previous tab. |
| Ctrl+S | Validate and save without closing. |
| Ctrl+Z / Ctrl+Y | Undo / redo changes already applied to the document. |
| F9 | Open Preview / Test. |
| F1 | Application help; in Preview / Test, preview-specific help. Help uses the shared Markdown viewer: Up/Down, Page Up/Down, Home/End scroll, Esc or q closes. |
| Esc | Quit immediately if clean; otherwise ask whether to save. |

The window title names the application only, so the tabs stay readable; the
edited file appears in the frame title of the **General** page. The title marks
unsaved changes. A newly created menu is dirty even before its
first edit. General-field edits update the in-memory document immediately;
command, action, and language-prompt dialogs use separate drafts:

* **F10** applies an action to its command draft, or a command draft to the
  document. Applying the action is not enough: apply its enclosing command too.
* **F2 or F10**, not Ctrl+Enter, applies a language-prompt draft. Many
  terminals consume Ctrl+Enter before the editor sees it.
* **Esc** discards the current draft. In an open choice list, file browser, or
  position picker, Esc first cancels that nested control, not its parent draft.
* Tab is local inside drafts. A command dialog switches between fields and
  actions; it does not change the application tab.

**There is no global file save while a modal control owns input.** Apply or
cancel it first. In particular, Ctrl+S in the file browser selects the current
directory; it does not save the menu. A Commands search filter also owns input:
clear it with Esc before saving, undoing, or switching tabs.

Undo/redo uses up to 100 whole-menu history entries, not a persistent journal.
A general-field keystroke can be an undo step; an accepted command draft is a
single document change. A new edit after undo clears redo. Undo/redo rebuilds
the tabs from the restored document; do not rely on local search or preview
state surviving it. Saving establishes the new clean baseline without clearing
the in-session history.

### Save review and failures

Every save runs the same checks used by `--check`. If there are issues, the
review screen offers **F8 to save anyway** or **Esc to return to editing**.
Up/Down and Page Up/Down scroll the review. This confirmation applies to both
warnings and errors: interactive validation is advisory, not a hard save gate.
An empty menu, for example, can be saved deliberately but must not be used as
a working runtime menu.

On exit with changes, the editor shows the same answer bar as the other tools:
Left/Right move the lightbar between **Yes** and **No**, Enter confirms, and
**Esc** returns to editing. Yes preselected saves and closes and may still lead
to the issue review; No discards changes since the last save and closes.
Previously saved data is not undone by No.

Before saving, the editor compares the destination bytes with those loaded or
last saved. If another process changed, removed, or created the destination,
save is refused and edits remain open. There is no automatic merge or force
overwrite. Existing menus use the shared atomic writer; new menus use a
same-directory temporary file and a no-clobber persist. This is conflict
detection, not a file lock or a guarantee against every concurrent-writer race.
Other write failures also return to editing without clearing the dirty state.

Saving serializes the typed `Menu` model as TOML. Known stored fields are kept,
including unused settings, but comments, original formatting, and unknown
TOML keys are not a round-trip-preservation contract.

## General settings

Use Up/Down to select a field, type to edit text, Enter to open the menu-type
list, and Space to toggle a boolean.

| Field | Effect |
| --- | --- |
| Title | Stored menu title. |
| Display file | Background artwork path. |
| Help file | Stored menu-help path; checked for existence, but not consumed by the current menu runner. |
| Menu type | `Hotkey` submits one accepted character immediately. `Command` and `Lightbar` wait for Enter. |
| Main prompt | Prompt used by the runtime before menu input. |
| Force display | Preserves/edits `force_display`; currently ignored by the menu runner. |
| Pass through | Preserves/edits `pass_through`; currently ignored by the menu runner. |

The current runner displays the background each cycle and can fall back to
board command lookup regardless of `pass_through`. Setting it to false is not
an access-control mechanism. Arrow navigation uses command positions; the
menu-type label alone does not supply highlighted strings or usable commands.

### File browser

**F4** opens the browser on a path field. Enter opens a directory or selects a
file; Right opens a directory; Left/Backspace goes to the parent. Up/Down,
Home/End, and Page Up/Down move through entries, **.** toggles hidden files,
Ctrl+S selects the current directory, and Esc cancels.

Browsing starts at the field's path or its nearest existing directory. For an
empty or relative field, an in-board selection remains board-root-relative;
an outside selection becomes absolute. An already absolute field stays
absolute. Browsing neither creates files nor changes the working directory.
Type a missing filename directly when needed. This editor does not provide
the setup tool's external file-edit/create shortcuts.

## Commands and actions

The Commands table shows keyword, display text, action summary, security, and
autorun mode. The selected row's summary identifies its underlying command
number. Search includes normal/highlight text, action types and parameters,
security, and autorun as well as the keyword.

| Key in the command list | Action |
| --- | --- |
| Up/Down, Home/End | Select a command. Unfiltered lists also accept k/j for up/down. |
| Insert | Open a new command draft; append it only when applied. |
| Enter | Edit the selected command. |
| Delete | Remove the selected command immediately from the document; Ctrl+Z can undo. |
| Ctrl+D | Open an independent duplicate draft; applying appends it. |
| / | Start a case-insensitive substring filter; type or Backspace to change it. |
| Esc while filtering | Clear the filter without quitting. |
| Page Up / Page Down | Move the command one place up/down; disabled while filtering. |

Filtered editing and deletion affect the actual matching record, not its visible
row number. Duplicating keeps the old keyword until you change it; duplicate
keywords are reported because runtime local lookup uses the **first
ASCII-case-insensitive exact match**, not a prefix abbreviation.

### Command fields and placement

A command draft exposes normal text, highlighted text, position, keyword,
autorun mode, interval, help path, security expression, charge per use, and
charge per minute, plus its ordered action list. Normal/highlight samples
render PCBoard `@X` colors. The model's highlighted-text field is deliberately
spelled `lighbar_display` in TOML.

* Position is edited as zero-based `x,y`; both numbers must fit 0–65535.
  Saving uses a TOML array, for example `position = [2, 4]`. Validation warns
  about positions/text outside the 80×25 preview and overlapping labels.
* On Position, **Enter or F4** opens the positioning view with the background,
  other commands, and the current draft. Arrows (or h/j/k/l) move it, **F6**
  switches normal/highlight, Enter/F10 accepts the position, and Esc cancels
  the move. Accepting a position still leaves the command draft open.
* The positioning view shares the bounded text loader with Preview / Test
  (1 MiB limit) and resolves background variants for Graphics, security 0,
  no language. It uses an 80×25 canvas, panning on smaller screens without
  changing stored coordinates. An unreadable or rejected background produces
  a warning and a blank fallback.
* Security uses the engine expression parser. F10 rejects a reported parse
  failure, but parsing success is not proof of a correct access policy; see
  [Security below](#security-is-only-partially-evaluated).
* Charges must be finite, nonnegative numbers with a decimal point, such as
  `2.75`; zero disables that command surcharge. Negative values, NaN,
  infinity, overflow, and decimal commas are rejected. These are command
  surcharges, independent of charges for a door/activity they invoke. See
  [Accounting](accounting.md) for runtime billing; preview never charges.
* The timer is a whole number of seconds stored as `u64`; large existing
  values are not truncated to a small editor range.

Autorun modes are `Disabled`, `FirstCmd` (on menu entry before the loop),
`Every` (before each background display), `After` (after display, before
input), and `Loop` (interval checks while waiting for input). Only Loop uses
the interval. A zero Loop interval warns because it can run on every timer
check. Preview lists these settings but never schedules autoruns.

Command help paths, like menu help paths, are editable and checked but not
used by the current menu runner. A nonempty highlighted string matters for
activation: Enter with no typed command activates the selected record only
when that string is nonempty.

### Build an action sequence

In the command dialog, Tab/Shift+Tab switches between fields and the action
table. Use Up/Down or Home/End to select an action. Insert adds a draft after
the selected action; Enter edits; Delete removes from the command draft;
Ctrl+D duplicates into a new action draft. **1/2** or Page Up/Page Down moves
an action one place up/down. Shortcuts **m**, **p**, **d**, and **q** open drafts
for `Menu`, `RunPPE`, `Door`, and `QuitMenu` respectively. Nothing is executed.

The action dialog edits type, parameter, and **Run on selection**. Enter on
Type opens the categorized list; typing searches names/categories, Backspace
shortens the search, arrows select, Enter confirms, and Esc closes the list.
Categories are Files, Navigation, Board, Text, and Commands. **F10** applies
the action; **Esc** discards it. Close a nested list/browser first.

Parameters are stored as strings even when assisted controls offer choices:

| Action type | Parameter assistance and meaning |
| --- | --- |
| `Menu`, `DisplayFile`, `StuffFile`, `StuffFileSilent` | Path field with F4 browsing, relative to the board root. `Menu` replaces the target extension with `.mnu`. |
| `RunPPE` | Separate file and arguments fields. F4 replaces only the file and keeps the argument suffix, including semicolon-separated empty arguments. F2 offers the complete raw parameter. |
| `Conference` | Known conference choices, numbered from **0**; raw input also permits a name. |
| `Door`, `DisplayDir` | Choices from loaded door/directory data only when the board has **exactly one conference**; local IDs start at **1**. Otherwise enter a raw runtime-conference parameter. |
| `Script` | Raw positive, **one-based survey number in the runtime conference**, not a script/PPE filename. There is no survey picker. |
| `GotoXY` | Raw zero-based `x,y`, for example `10,5`. |
| `PrintText`, `StuffText` and its variants | Literal text. The two `StuffTextAndExitMenu` variants take text, not a filename. |
| `Disabled`, `DisableMenuOption`, `QuitMenu`, `ExitMenus`, `RefreshDisplayString` | No parameter needed; an existing parameter is preserved rather than silently cleared. |
| Other command types | Raw, action-specific parameter; do not assume every built-in consumes it. |

**F2 toggles raw/assisted parameter input** without deliberately replacing the
value. Existing values absent from a choice list are retained. A menu does
not identify its runtime conference, so even available choices are assistance,
not proof that a reference is correct wherever the menu will be opened.
Selecting another action type retains the parameter and returns to assisted
mode; check that the old text still makes sense.

`RunPPE` accepts a PPE path followed by arguments. Runtime tokenization splits
on spaces and semicolons; it is not shell quoting. Assisted editing rejects
file paths containing those separators; raw editing remains possible but does
not make such paths work. Arguments may include their leading separators;
otherwise a space is inserted. Unedited parameters survive raw/assisted
switching unchanged. `Command` performs list-aware
command lookup; `GlobalCommand` bypasses the configured command lists to use
built-in lookup. The [action reference](configuration/commands.md#action-sequencing-and-parameters)
provides further parameter details.

With **Run on selection** off, an action has trigger `Activation`; with it on,
the trigger is `Selection`. In the current runtime, ordinary activation and
autorun dispatch filter to enabled Activation actions, in stored order.
Selection actions are invoked separately on lightbar selection, including the
initial selection. `Disabled` and `DisableMenuOption` are no-ops. An action
sequence is not a transaction, and the preview does not predict where errors,
nested menus, or logoff will interrupt it.

## Language prompts

A menu has one **main prompt** without a language, edited on the General page.
This tab adds prompts that replace it for one language each, so **every entry
needs its language**; that is the entry's only purpose. The tab shows the main
prompt for reference. Language overrides come from legacy menus, where the
first prompt is the suffix-less one and the rest are `suffix,prompt` pairs;
they are **not follow-up questions or command parameters**. The current runtime
uses only the main prompt and ignores these entries.

The language column names the board's configured language when the stored
suffix matches one, comparing without a leading dot and ignoring letter case;
otherwise the raw suffix is shown. Up/Down, Home/End, and Page Up/Page Down
select a row. Insert opens a new draft, Enter edits, and Delete asks for
confirmation (Enter deletes, Esc cancels).

A draft has a language list and the prompt text. Up/Down switches fields,
**F3** replaces the list with free text for a suffix the board does not know,
**F2 or F10 applies**, and Esc discards. In an open list, Esc closes the list
first.

A suffix must not contain whitespace, control characters, commas, or either
slash; a comma separates fields in the legacy format. A leading dot is
optional. Two entries for the same language are rejected, ignoring a leading
dot and letter case. Choosing a language from the list keeps an imported
suffix's exact spelling, and editing only the prompt text preserves that key
byte-for-byte. Text and row order are retained. If the underlying prompt list
changes while a draft or delete confirmation is open, the operation is refused
instead of overwriting another edit; cancel and reopen it.

## Preview / Test: inspect, do not execute

F9 opens a static view of the **applied in-memory menu**, including unsaved
changes. It does not include an unaccepted draft. The background, positioned
labels, main prompt, inspector, issues, and action trace help find layout and
configuration problems before saving.

**No menu action executes here.** The preview does not run PPEs, launch doors
or subprocesses, enter nested menus, invoke BBS commands, stuff keyboard input,
charge accounts, or schedule autoruns. A trace is a description of configured
actions, not an execution result. The application-level Ctrl+S and quit keys
still work on this tab; “read only” describes preview activity, not a lock on
the entire editor.

| Preview key | Action |
| --- | --- |
| Arrows | Positional navigation using the menu model. |
| Ctrl+arrows | Cycle through all commands, including entries difficult to reach by position. |
| Home / End | First / last command. |
| Type a keyword | Hotkey submits immediately; Command/Lightbar waits for Enter. Backspace removes a character. |
| Enter | Show a static action trace. With no typed input, report activation only if the selected highlighted string is nonempty. |
| F2 | Toggle Issues / Inspector. |
| F3 | Show Inspector. |
| F4 | Switch selected highlight display / normal-only display. |
| F5 | Reload the background and rerun checks. |
| F6 / F7 | Decrease / increase preview security (0–255); Ctrl changes it by 10. |
| Page Up / Page Down | Scroll inspector, trace, or issue text, including long individual messages. |
| Alt+arrows | Pan the cropped canvas; use Inspector/Trace rather than Issues for panning. |
| F1 | Preview help. Esc closes help; otherwise Esc remains application quit. |

In Issues, Up/Down and Home/End select an issue instead of a command. Enter
on a command-specific issue selects that command in the **preview inspector**;
it does not open the command editor. Return to Commands to fix it.

Typed input uses the runtime's accepted-character mask and 13-character
limit. Local matching is first exact match, ignoring ASCII case. Global and
conference command fallback, door fallback, remembered-command expansion,
and caller token/control-flow effects are not simulated.

### Rendering and file lookup limits

The preview uses a fixed **80×25 canvas**, cropped and pannable rather than
scaled. It follows the selected command until manually panned. Labels whose
security is denied, unknown, or invalid are dimmed, not hidden; the selected
label is underlined. These are inspection aids, not exact runtime styling.

Backgrounds are read as regular files, at most **1 MiB**, and imported in memory
as ANSI, PCBoard, or Avatar text with CP437 conversion. Known executable,
binary, animation, and graphical extensions are refused; unknown extensions
use text import rather than program execution. Menu strings render `@X`
colors; inline controls in labels/traces are made inert. Runtime macros,
embedded commands, mixed ANSI/PCBoard semantics, fonts, and graphics are not
fully emulated. The main prompt has its own row, not the runtime caret
position; additional prompts appear only in the inspector.

The display lookup profile is **Graphics, selected security, no language**.
An explicitly extended existing file wins. Otherwise the resolver tries the
security-suffixed base before the unsuffixed base, graphics `g` before plain,
and `.ans`, `.avt`, `.pcb`, `.asc`, then no extension. This follows the relevant
runtime lookup ordering, not every possible caller profile. Backgrounds are
cached instead of reread on every unchanged frame; use F5 after external file
changes. Load errors are visible and do not prevent inspecting the commands.

### Security is only partially evaluated

The preview evaluates context-free expressions and `U_SEC()` with the engine's
actual admission method at the selected level. For example, `20` and
`U_SEC() >= 20` can be checked. Integer results use the engine's u8 threshold
conversion; keep literal security levels within 0–255.

Expressions involving `U_AGE()`, `U_GROUP(...)`, `TIME()`, `TIME_LEFT()`, or
`DOW()` are marked **Unknown; caller/session context required**, rather than
evaluated with invented user, group, or time data. Recognized invalid calls,
unsafe argument forms, and invalid context-free results are errors. This is a
conservative partial classifier, not a full expression type/arity checker:
Unknown does not certify that the expression will evaluate successfully.
Parser limitations also remain; follow the
[security-expression reference](configuration/components.md#component-security-expressions)
rather than treating successful parsing as policy verification.

“Command security allows” says only that this command-level expression allows
that level. It says nothing conclusive about resource permissions, conference
state, balances, doors, PPE behavior, or the rest of an action sequence.

**Selection actions need special care:** the current runner invokes them
without the command-level admission check, including initial selection.
Individual action checks may still apply. Do not put a privileged side effect
on Selection and assume the command's security or dimmed preview protects it.
For custom Activation actions, set command security explicitly; a direct
built-in action does not automatically inherit every built-in command gate.
Use controlled live caller testing for the actual access policy.

## What validation checks

`--check`, Preview Issues, and save review share `validate_menu()`. CLI issues
are printed to stdout with localized severity, `Menu` or a one-based command
number, and explanatory text. There is no JSON report or automatic repair.

* **Exit 1**: at least one validation error. Startup/load failures also fail.
* **Exit 0**: no validation errors, including a warnings-only result.
* No issues produces an explicit “not a runtime guarantee” message.
* The headless check does not save the menu, create missing targets, initialize
  the interactive log, or execute actions.

Errors cover an empty command list; nonfinite/negative charges; security
expressions the partial evaluator identifies as invalid; nonpositive or
nonnumeric `Script` survey parameters; and empty `Menu`, `DisplayFile`,
`StuffFile`, or `StuffFileSilent` parameters.

Warnings cover duplicate/untypable keywords, multi-character Hotkey keywords,
an empty keyword without autorun or highlighted activation, missing actions
or no enabled Activation action, zero Loop intervals, positions/label widths
outside 80×25, overlapping labels, malformed/out-of-range `GotoXY`, selected
empty text/command parameters, context-dependent security, unused runtime
options, and missing referenced display/help/menu/stuff/PPE files.

Display/help existence warnings always use **Graphics, security 0, no
language**, even when the preview security is changed. They do not establish
absence for every caller profile. A `RunPPE` file check considers the first
token and an optional `.ppe` extension; it does not validate PPE arguments or
prove that a PPE is loadable. An empty first filename token is an error.

Checks do not recursively validate submenus, detect every recursion or control
flow problem, verify all conference-relative IDs, validate executable contents,
or exercise billing and action-specific permissions. A positive survey number
can still refer to no survey in the eventual conference. Background format or
size failures can appear in Preview even when headless existence checks pass.

## Short workflow: a three-command submenu

This example creates a submenu with visible text and no external artwork.
Use a working board with an existing menu directory:

```sh
mkicbmnu -c -b /srv/mybbs /srv/mybbs/menus/information.mnu
```

1. In General, set Title to `Information`, Menu type to `Command`, and Main
   prompt to `H = hello, I = info, Q = return: `. Leave display/help paths empty.
2. In Commands, press Insert for each of these records. Use Up/Down for fields
   and Tab for the action table. Insert an action, select its type, enter its
   parameter, and leave Run on selection off. Press F10 for the action and
   F10 again for the command.

   | Keyword | Normal / highlighted text | Position | Action | Parameter |
   | --- | --- | --- | --- | --- |
   | `H` | `Hello` / `> Hello` | `2,4` | `PrintText` | `Hello from this board!` |
   | `I` | `Information` / `> Information` | `2,6` | `PrintText` | `Ask the sysop for more information.` |
   | `Q` | `Return` / `> Return` | `2,8` | `QuitMenu` | Empty |

   Keep security at `0`, autorun Disabled, and both charges zero for this example.
3. Press F9. Type H, I, or Q followed by Enter to inspect the traces. Move
   between labels, compare normal/highlight with F4, and inspect Issues with
   F2. The PrintText messages are listed, not executed, and Q does not close
   the preview.
4. Press Ctrl+S. Resolve any unexpected issues, or consciously use F8 in the
   review to proceed. Esc closes the editor after a successful save.
5. In the intended parent menu, add a command with an Activation `Menu` action
   whose parameter is `menus/information.mnu`, then apply and save that parent.
   Creating the child alone does not connect it to the board. At runtime,
   `QuitMenu` returns from the child; `ExitMenus` requests leaving all menus.

Check the saved child independently:

```sh
mkicbmnu --check -b /srv/mybbs /srv/mybbs/menus/information.mnu
```

Finally, use a controlled local call as described in
[Getting started](gettingstarted.md#make-the-first-call) to verify the actual
parent/child transition, output, and return. That is a separate runtime test,
not something F9 or `--check` performs. Test real privileged or priced commands
with representative non-sysop callers as well.

## Developer notes and test boundaries

The implementation is split along these boundaries:

| Source | Responsibility |
| --- | --- |
| [CLI entry point](../crates/mkicbmnu/src/main.rs) | Argument handling, parent-board discovery, headless check, theme/log/terminal startup. |
| [Application](../crates/mkicbmnu/src/app.rs) | Tab construction, modal key ownership, document history dispatch, help and save/quit review. |
| [Document](../crates/mkicbmnu/src/document.rs) | Dirty baseline, bounded snapshots, disk conflict check, atomic/no-clobber persistence. |
| [General tab](../crates/mkicbmnu/src/tabs/general.rs) | Immediate general-field publication without render-dependent dirty tracking. |
| [Commands tab](../crates/mkicbmnu/src/tabs/commands.rs) and [command dialog](../crates/mkicbmnu/src/edit_command_dialog.rs) | Filter-to-model index mapping, isolated command/action drafts, parameter assistance and placement. |
| [Language prompts](../crates/mkicbmnu/src/tabs/prompts.rs) | Ordered language pairs, key validation, draft/delete conflict handling. |
| [Validation](../crates/mkicbmnu/src/validation.rs) | Shared read-only diagnostics, display candidate ordering, partial access evaluation. |
| [Preview](../crates/mkicbmnu/src/tabs/preview.rs) | Immutable menu snapshots, bounded background import/cache, canvas, inspector, issues and static traces. |
| [Menu model](../crates/icy_board_engine/src/icy_board/menu.rs), [command model](../crates/icy_board_engine/src/icy_board/commands.rs), [runtime runner](../crates/icy_board_engine/src/icy_board/state/menu_runner.rs) | Serialized data, navigation, and actual dispatch semantics; runtime is not invoked by preview. |

When extending the editor, preserve modal key ownership and publish accepted
changes before returning from input handling so undo/dirty tracking sees them.
Keep enum storage values separate from translated labels, and keep assisted
parameters lossless when no matching choice exists. Do not add an execution
capability to a trace builder as a shortcut to better preview fidelity.

Existing source-level tests cover deferred/no-clobber creation, save/load and
external-change failures, undo/redo, modal cancellation and acceptance,
filtered edits, duplication/reordering, parameter choices/raw switching,
numeric fields, prompt validation/conflicts, and rendering with Ratatui's
in-memory `TestBackend`, including small screens. Validation/preview tests
cover selected keyword, position, file-precedence, security, text-decoding,
cache, issue-scrolling, and inert-trace cases. See the tests embedded in those
modules, [command-dialog tests](../crates/mkicbmnu/src/edit_command_dialog_tests.rs),
and [command-list tests](../crates/mkicbmnu/src/tabs/commands_tests.rs).

The current [CLI integration tests](../crates/mkicbmnu/tests/cli.rs) exercise
English/German help and errors, version output, explicit board directories
and named configurations, parent discovery, CWD-relative menu paths, invalid
explicit boards without fallback, create refusal, and headless check exit
codes. Board/menu/log snapshots verify that checks leave files unchanged.
They do **not** provide end-to-end coverage of a real terminal editing session,
the complete create–preview–save workflow, or live BBS behavior. In-memory render
tests are not a visual/keyboard compatibility guarantee for every terminal;
static traces are not PPE/door/security/accounting integration tests.

The engine has `Menu::import_pcboard()` and board-migration support, but there
is **no PCBoard menu-import UI or import option in `mkicbmnu`**. Use the
[migration guide](migration.md) for board import; do not open legacy MNU text
as if it were native TOML. See [Known limitations](known_limitations.md) for
broader project boundaries.