<a id="adding-commands"></a>

# Adding and overriding commands

IcyBoard has three related command sources: a board-wide command list,
conference-specific command lists, and commands embedded in interactive
menus. All use the same `Command` and `CommandAction` records. Native
files are TOML; PCBoard's binary CMD.LST is an import format, not another
way to write TOML commands.

A command can display a file, launch a PPE, open a menu or door, invoke a
built-in operation, stuff keyboard input, or perform several actions in
sequence. Configure security explicitly when exposing privileged actions.

## Connecting command lists to the board

The board configuration's `paths.command_file` selects the global command
list. This is a board-configuration **fragment**, to merge into the existing
`[paths]` table, not a standalone board file:

```toml
[paths]
command_file = "main/commands.toml"
```

Each record in the separate conference file has its own `command_file`
path. Set it to a conference list, or `""` for no conference override.
This is a **fragment of an existing conference record**; it omits that
record's other required fields:

```toml
command_file = "conferences/main/commands.toml"
use_main_commands = true
```

Relative command-file paths are board-root-relative, not relative to the
conference file's location. The global file is loaded during board startup;
an unreadable or invalid global file is an error. A conference list is loaded
when its path is a file; a parse failure is logged and leaves its commands
empty. Edit the separate command file rather than adding nested commands
to a conference: the conference's in-memory `commands` field is skipped
by serde. Do not assume hand edits hot-reload into running sessions.

`use_main_commands` is serialized (missing default false), but the current
lookup code does not consult it: global commands are still searched.
Changing this flag is not a way to disable inherited commands. The setup
creator uses true for Main Board.

See [Conferences](components.md#component-conferences) for the complete conference schema and
[Menus](components.md#component-menus) for the separate menu-file schema.

## Native command-list format

A command-list file has a required root `command` array. Use
`[[command]]` for each command and `[[command.actions]]` for its actions.
The standalone empty list is `command = []`; an entirely empty file does
not deserialize as a `CommandList`. There is no `[commands]` wrapper.

Standalone global or conference command list:

```toml
[[command]]
keyword = "RULES"
security = "0"
[[command.actions]]
command_type = "DisplayFile"
parameter = "conferences/main/rules"

[[command]]
keyword = "INFO"
security = "10"
[[command.actions]]
command_type = "Menu"
parameter = "art/information.mnu"

[[command]]
keyword = "WEATHER"
security = "20"
[[command.actions]]
command_type = "RunPPE"
parameter = "ppe/weather.ppe"

[[command]]
keyword = "R"
security = "10"
[[command.actions]]
command_type = "PrintText"
parameter = "Opening the message reader...\r\n"
[[command.actions]]
command_type = "GlobalCommand"
parameter = "R"
```

This file deserializes independently; the art, menu and PPE must also exist
before their actions can succeed. The final entry safely delegates to the
built-in `R` rather than resolving its own override again.

## Complete Command schema

Every field *inside a Command record* is optional. That does not make the
command-list root array optional. Keys and enum values are case-sensitive
in TOML; keyword **matching at runtime** is ASCII-case-insensitive.

**Command fields**

| Key | Type / missing default | Use |
| --- | --- | --- |
| `display` | string / empty | Normal display string in a menu |
| `lighbar_display` | string / empty | Highlighted display string; preserve this exact misspelling |
| `position` | two-element u16 array / `[0, 0]` | Zero-based menu column and row; see below |
| `keyword` | string / empty | Typed command keyword |
| `auto_run` | enum / `"Disabled"` | `Disabled`, `FirstCmd`, `Every`, `After`, `Loop` |
| `autorun_time` | u64 / 0 | Loop autorun interval in seconds |
| `help` | string / empty | Stored command-help setting; not consumed by the menu runner |
| `security` | security-expression string / `"0"` | Access gate for activation and autorun |
| `actions` | array of CommandAction records / empty | Ordered actions; `[[command.actions]]` in a list file |

`position` uses a custom tuple serializer. Write `position = [10, 4]`,
not `"10,4"`, `"(10, 4)"`, or a table with `x` and `y` fields.
Both integers must fit 0--65535. The display code adds one to each when
calling the terminal's one-based positioning function. This storage format
is different from the comma-separated **parameter** of the `GotoXY` action.

Saving suppresses empty display/help strings, the default position and
autorun mode, zero time and empty actions. `keyword` is still written when
empty. `security` uses the special expression serialization described in
[Security expressions](components.md#component-security-expressions): an omitted value means `"0"`;
boolean `true` is suppressed on save, not the integer default.

## Complete CommandAction schema

**Action fields**

| Key | Type / missing default | Use |
| --- | --- | --- |
| `command_type` | CommandType / **required** | Exact enum string below, or the tagged value for ReadMemorizedMessage |
| `parameter` | string / empty | Input interpreted by this action, not automatically by every action |
| `trigger` | enum / `"Activation"` | `Activation` or `Selection` |

Although `CommandType::default()` is Disabled, omitting `command_type`
from an action is an error. `parameter` and `trigger` are omitted on
save when empty/default. There are no per-action `security`, `arguments`
or `enabled` fields.

Ordinary command types use exact strings such as `"RunPPE"`. The one
data-carrying variant, `ReadMemorizedMessage(u8)`, uses serde's externally
tagged form. A standalone example is:

```toml
[[command]]
keyword = "SAVED"
security = "10"
[[command.actions]]
command_type = { ReadMemorizedMessage = 0 }
```

The stored integer can fit any u8, but the built-in modes are 0 (RM), 1
(RM+) and 2 (RM-); use those modes. The current handler sends modes 0 and 1
through the same forward-reading path, and mode 2 through the backward path.
A plain `"ReadMemorizedMessage"` string
does not provide the required payload. The TOML type is
`"ChangeMessageArea"`, **not** the `"MessageArea"` text shown by its
Rust Display implementation. Nor does TOML use the separate, more permissive
`CommandType::from_str` parser used by some editor interfaces.

## Lookup, abbreviations and security

For ordinary typed board commands, lookup proceeds in this order:

1. Exact conference-list keyword.
2. Exact global-list keyword.
3. Conference-list prefix match.
4. Global-list prefix match.
5. Built-in keyword.

All list comparisons ignore ASCII case; the first matching record in a list
wins. Abbreviations need at least two typed bytes: `GR` can select
`GREED`, but `G` does not abbreviate it. Ambiguous prefixes are resolved
by file order, not rejected. An exact global match wins over a conference
prefix. If no list or built-in command is found, the ordinary single-command
runner also tries a matching door before reporting an invalid entry.

Lookup does not filter by security or fall through on denial. The selected
record's `security` is checked afterwards; an inaccessible override still
shadows a built-in or lower-priority record. A record with no actions, or
only `Disabled` actions, also consumes its keyword. This allows intentional
disabling without deleting a built-in.

> [!WARNING]
> A configured command's security expression is its activation gate.
> Direct action variants such as `RunPPE`, `ViewTextFile`, or
> `UserMaintenance` do not automatically inherit the built-in command's
> configured security level during normal dispatch. Built-ins created by
> lookup receive the board's user/sysop command security before dispatch;
> custom records do not. Set an appropriate `security` on every custom
> privileged command, or delegate through `GlobalCommand` to retain the
> built-in command gate. Resource-specific checks may still apply.

A failed check displays the unavailable-selection message, counts a security
violation, and can disconnect the caller after more than ten violations.
Hidden display text, empty keywords, and menu positions are not access
controls. Test custom commands as an ordinary caller, not only as SYSOP.

## Action sequencing and parameters

Normal dispatch checks the command once, then invokes **all actions in file
order**, and clears the session token queue afterwards. It does not filter
that loop by `trigger`. Selection actions therefore run on activation too.
An action error can end the sequence; do not treat multiple actions as a
transaction. Exit actions set menu state flags and do not inherently skip
the remainder of the action array.

`parameter` is a string with action-specific meaning:

* `Menu` resolves the filename and replaces its extension with `.mnu`.
* `DisplayFile` resolves and displays a file using normal display handling.
* `RunPPE` adds a nonempty parameter to session tokens and invokes the PPE
  runner (PPE path followed by any arguments).
* `Script` parses a one-based survey number in the current conference.
* `Conference` and `DisplayDir` add the parameter to the token queue and
  invoke the conference-join or file-directory command respectively.
* `Door` selects a current-conference door by name/number; an empty
  parameter falls back to the ordinary door prompt/list behavior.
* `GotoXY` parses a zero-based comma-separated `"x,y"` string as u16
  coordinates, defaulting an unparseable coordinate to zero.
* `PrintText` prints the parameter. `RefreshDisplayString` redraws the
  enclosing command's normal display string.
* `Command` adds the parameter to session tokens and invokes normal
  list-aware lookup. Avoid self-referential aliases and recursive menus.
* `GlobalCommand` invokes lookup **without either TOML command list**.
  Despite its name it does not mean "the global TOML list only"; it selects
  built-ins (and the single-command runner's door fallback). It is useful
  inside overrides of built-in keywords.

Most direct built-in variants, such as `ReadMessages`, do **not** inject
their `parameter` into input; they use tokens already left by the caller.
For a preselected built-in operation, prefer `GlobalCommand` with the full
command line, or the dedicated `Conference`, `DisplayDir`, `Script` or
`Door` action. Appending parameters to a shared token queue is not isolated
argument passing: caller tokens can precede the configured parameter, and
nested commands may consume or clear them. Keep compound commands simple.
Tokenization splits on spaces and semicolons; it is not shell parsing and
does not protect spaces inside quoted filenames. Adjacent semicolons can
produce empty tokens for prompt answers.

`StuffText` and `StuffTextSilent` queue the parameter as visible or hidden
keyboard input. `StuffFile` and `StuffFileSilent` read and queue a file's
contents. The two `StuffTextAndExitMenu` variants queue **literal parameter
text**, not a filename, then request exit from all menus. Trailing CR/LF is
trimmed, and a carriage return is appended when the text does not already
end in `^M`. Caret control notation is recognized by keyboard stuffing.

Stuffed input normally bypasses CMD.LST-style TOML lookup so an overridden
command can stuff its built-in keyword without retriggering itself.
Interactive menu-local exact matches are still checked first; stuffing a
menu's own keyword can therefore still recurse. The `Command` and
`GlobalCommand` actions are immediate token-based dispatch, not keyboard
stuffing.

## Complete CommandType vocabulary

All variants declared by the serializer are listed here. Names in parentheses
are common built-in keywords, **not** alternative TOML enum spellings.
An action's existence means it is dispatched, not that every historical
PCBoard sub-option is implemented.

### Menu, display and dispatch actions

* `Disabled`, `DisableMenuOption`: no operation. DisableMenuOption does
  not dynamically remove another menu entry.
* `Menu`: open a menu; `QuitMenu`: leave the current menu;
  `ExitMenus`: leave all active menus.
* `Script`: select a survey; `Conference`: parameterized conference join;
  `DisplayDir`: parameterized directory display; `Door`: select a door.
* `DisplayFile`: display a file; `GotoXY`: move the cursor;
  `PrintText`: print text; `RefreshDisplayString`: redraw this command.
* `StuffText`, `StuffTextSilent`, `StuffFile`, `StuffFileSilent`,
  `StuffTextAndExitMenu`, `StuffTextAndExitMenuSilent`: keyboard input
  variants described above.
* `Command`, `GlobalCommand`: list-aware or built-in command dispatch.
* `RunPPE`: execute a PPE.

### Caller operations

**Direct caller action variants**

| CommandType | Common keyword | Operation |
| --- | --- | --- |
| `AbandonConference` | A | Leave the current conference |
| `BulletinList` | B | Bulletin selection/display |
| `CommentToSysop` | C | Write a comment to the sysop |
| `Download`, `BatchDownload` | D, BD/DB | Download and batch-download workflows |
| `EnterMessage` | E | Compose a message |
| `FileDirectory` | F | File-directory command |
| `FlagFiles` | FLAG | Flag files for transfer |
| `Goodbye`, `Bye` | G, BYE | Normal goodbye or immediate BYE path (skips flag scan) |
| `Help` | H/? | Help command |
| `InitialWelcome` | I | Initial welcome display |
| `JoinConference` | J | Conference selection |
| `DeleteMessage` | K | Delete a message |
| `LocateFile` | L | Locate files |
| `ToggleGraphics` | M | Graphics-mode selection |
| `NewFileScan` | N | Scan for new files |
| `PageSysop` | O | Page the sysop |
| `SetPageLength` | P | Set screen page length |
| `QuickMessageScan` | Q | Quick message scan |
| `ReadMessages` | R | Message reader |
| `Survey` | S | Survey selection |
| `SetTransferProtocol` | T | Select transfer protocol |
| `UploadFile`, `BatchUpload` | U, BU/UB | Upload and batch-upload workflows |
| `ViewSettings`, `WriteSettings` | V, W | View/edit caller settings |
| `ExpertMode` | X | Expert-mode selection |
| `YourMailScan` | Y | Scan for personal mail |
| `ZippyDirectoryScan` | Z | Search file descriptions |
| `GroupChat` | CHAT | Group chat |
| `OpenDoor` | DOOR/OPEN | Door command, rather than the parameterized Door action |
| `TestFile` | TEST | Test a file |
| `UserList`, `WhoIsOnline` | USER, WHO | Caller list and online callers |
| `ShowMenu` | MENU | Display the current user/sysop menu |
| `DisplayNews`, `SetLanguage` | NEWS, LANG | Conference news and language selection |
| `ReplyMessage` | REPLY | Compose a reply |
| `EnableAlias` | ALIAS | Toggle alias use |
| `ReadEmail`, `WriteEmail` | @, @W | Read/write email |
| `TextSearch` | TS | Message text search |
| `QWK` | QWK | QWK packet workflow |
| `SelectConferences` | SELECT | Conference scan selection |
| `ReadMemorizedMessage` with u8 payload | RM, RM+, RM- | Memorized-message modes 0, 1, 2 |
| `ChangeMessageArea` | AREA | Message-area selection |

### Sysop operations

**Direct sysop action variants**

| CommandType | Common keyword | Operation |
| --- | --- | --- |
| `Broadcast` | BR | Broadcast to callers |
| `ViewCallerLog` | 1 | Caller log |
| `ViewUserFile` | 2 | User-file listing |
| `PackMessageBase` | 3 | Pack message base |
| `RestoreMessage` | 4 | Recover deleted messages |
| `HeaderScan` | 5 | Message-header scan |
| `ViewTextFile` | 6 | View an arbitrary text file |
| `UserMaintenance` | 7 | User maintenance |
| `PackUserFile` | 8 | Pack user file |
| `NodeList` | 11 | Other-node listing |
| `LogoffNode` | 12 | Log off another node |
| `NodeCallerLog` | 13 | Another node's caller log |
| `DirCommand` | 16 | Sysop directory command |

## Menu context, selection and autorun

The same Command schema appears in menu files under **plural**
`[[commands]]`, with `[[commands.actions]]`. A menu is a root record
with required `title`, not a CommandList. Its complete fields and a
standalone example are in [Menus](components.md#component-menus).

The current conference's `users_menu` or `sysop_menu` supplies the menu
for ordinary callers or sysops. The current-menu display path tries a PPE
variant first, then a `.mnu` TOML menu, then ordinary display art. A
`Menu` action opens a TOML `.mnu` directly. The displayed art is not a
command schema and does not by itself define actions.

Inside a menu, exact menu-local keywords are checked before ordinary board
lookup. Menu-local lookup has no prefix search. Arrow keys select using
`position`; Enter with no typed command activates the current item when
its `lighbar_display` is nonempty. Hotkey menus return after the first
accepted character, while Command and Lightbar modes wait for Enter.
The input buffer is limited to 13 characters by the current runner, even
though the serialized keyword is an unrestricted string.

`trigger = "Selection"` runs an action when the lightbar moves onto an
entry, including the runner's initial selection. That path calls individual
actions with built-in-security checking enabled, but does **not** check the
enclosing command's `security`. File/PPE and other actions differ in their
internal checks. Keep selection actions cosmetic; do not use them to perform
privileged or destructive operations. Activation, including autorun, runs
every action regardless of trigger, after checking the enclosing command.

Autorun is a menu-runner feature; adding it to a global or conference list
does not create a scheduler. Modes are:

* `Disabled`: no automatic execution (the command remains manually callable).
* `FirstCmd`: once when this menu is entered, before its main loop.
* `Every`: before each display cycle.
* `After`: after menu art and command display strings are drawn.
* `Loop`: during input waiting, using `autorun_time` seconds between
  executions. It is checked on input-idle/timer passes (the timer is about
  half a second), not a precise scheduled job.

The first Loop check runs without waiting for a prior interval. Zero removes
the interval gate; it does not disable Loop. Times are recorded by command
index in shared menu-runner state and cleared on menu entry, so nested menus
can affect timing. Autorun commands also remain available for ordinary
keyword activation. Do not use this mechanism for reliable maintenance jobs.

The menu schema retains `help_file`, `force_display`, `pass_through`
and alternate `prompts` values, but the current runner does not use those
fields to control help, display, fallback or prompt selection. A command's
`help` string is likewise not a custom-help hook in this path. Do not
promise those behaviors based solely on the serialized fields.

## Legacy CMD.LST and MNU import

PCBoard CMD.LST consists of **64-byte (0x40) binary records**. It is not a
comma-separated text list and not a TOML file with a different extension.
The command importer reads:

**CMD.LST record layout (zero-based byte offsets)**

| Bytes | Imported content |
| --- | --- |
| 0--14 | 15-byte CP437 keyword |
| 15 | u8 minimum security |
| 16--55 | 40-byte CP437 parameter |
| 56--63 | Not used by this importer |

Each record becomes one Command with one Activation action. A parameter
whose path extension is MNU becomes `Menu`; otherwise a parameter
containing `.PPE` (case-insensitive) becomes `RunPPE`; otherwise it becomes
`StuffText`. The security byte becomes a quoted numeric expression. Display,
lightbar and help strings are empty, position is zero, and autorun is Disabled.

Use setup's PCBoard import to convert such files, then edit the resulting
TOML. Ordinary `CommandList::load` is the TOML loader and does not inspect
the file extension to import binary records automatically. Renaming CMD.LST
to a TOML filename does not convert it.

Legacy PCBoard MNU files are a separate line-oriented import format with
numeric option types. Native Menu files use the root schema in
[Menus](components.md#component-menus) and string action variants above. Imported numeric
MNU type codes are not accepted as native `command_type` values.

## Validation checklist

1. Use `[[command]]` in standalone lists and `[[commands]]` in menus.
2. Supply each action's `command_type` with exact case and spelling.
3. Store security as strings and positions as two integers; consult
   [Security expressions](components.md#component-security-expressions) for parser limitations.
4. Avoid self-recursive `Command` aliases; use `GlobalCommand` when
   forwarding an overridden built-in.
5. Check list order, abbreviations, resource paths and ordinary-caller access.
6. Test lightbar selection, activation and autorun separately: their security
   and trigger paths are not identical.
7. Treat successful deserialization as a format check, not proof that the
   referenced resources exist or the command is safe to expose.