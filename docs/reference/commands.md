# Caller and sysop commands

These are the English command-help sources shipped with this version of Icy
Board. The website reads the original files directly; changes to command help
therefore also update this reference. A board can replace commands, menus and
help files, so its actual command prompt may differ.

Use `H command` for help while connected. Entering a command without its
arguments normally prompts for the missing values. Prompts inside a command
have their own vocabulary: `N` in the message reader does not have the same
meaning as `N` at the main menu.

## Access and customization

Command availability depends on the caller, conference and configured command
list. The main and conference command lists can override built-ins; consult
[command configuration](../configuration/commands.md) before changing dispatch.
Security expressions are described in the
[board configuration](../configuration/board.md#security-configuration).

Numeric commands are administrative commands, not ordinary caller shortcuts.
Help for `9`, `10`, `14` and `15` explains unavailable functionality; their
presence in this manual does **not** mean they are implemented.

## Main-menu commands

```{toctree}
:maxdepth: 1

A - Abandon conference </crates/icy_board_help/data/en/hlpa>
B - Bulletins </crates/icy_board_help/data/en/hlpb>
C - Comment to sysop </crates/icy_board_help/data/en/hlpc>
D - Download </crates/icy_board_help/data/en/hlpd>
E - Enter a message </crates/icy_board_help/data/en/hlpe>
F - File directories </crates/icy_board_help/data/en/hlpf>
G - Goodbye </crates/icy_board_help/data/en/hlpg>
H - Help </crates/icy_board_help/data/en/hlph>
I - Initial welcome </crates/icy_board_help/data/en/hlpi>
J - Join a conference </crates/icy_board_help/data/en/hlpj>
K - Kill a message </crates/icy_board_help/data/en/hlpk>
L - Locate files </crates/icy_board_help/data/en/hlpl>
M - Graphics mode </crates/icy_board_help/data/en/hlpm>
N - New files </crates/icy_board_help/data/en/hlpn>
O - Page the sysop </crates/icy_board_help/data/en/hlpo>
P - Page length </crates/icy_board_help/data/en/hlpp>
Q - Quick message scan </crates/icy_board_help/data/en/hlpq>
R - Read messages </crates/icy_board_help/data/en/hlpr>
S - Surveys </crates/icy_board_help/data/en/hlps>
T - Transfer protocol </crates/icy_board_help/data/en/hlpt>
U - Upload </crates/icy_board_help/data/en/hlpu>
V - View settings </crates/icy_board_help/data/en/hlpv>
W - Write settings </crates/icy_board_help/data/en/hlpw>
X - Expert mode </crates/icy_board_help/data/en/hlpx>
Y - Your mail </crates/icy_board_help/data/en/hlpy>
Z - Zippy file search </crates/icy_board_help/data/en/hlpz>
```

## Word and symbol commands

```{toctree}
:maxdepth: 1

! - Recall a command </crates/icy_board_help/data/en/hlp!>
@ - Personal inbox </crates/icy_board_help/data/en/hlp@>
@W - Write personal mail </crates/icy_board_help/data/en/hlp@w>
ALIAS - Caller alias </crates/icy_board_help/data/en/hlpalias>
AREA - Message area </crates/icy_board_help/data/en/hlparea>
BROADCAST - Message to nodes </crates/icy_board_help/data/en/hlpbrd>
CHAT - Group chat </crates/icy_board_help/data/en/hlpchat>
FLAG - Queue files </crates/icy_board_help/data/en/hlpflag>
LANG - Language </crates/icy_board_help/data/en/hlplang>
NEWS - News display </crates/icy_board_help/data/en/hlpnews>
OPEN - Doors </crates/icy_board_help/data/en/hlpopen>
PPE - Run a PPE </crates/icy_board_help/data/en/hlpppe>
QWK - Offline mail </crates/icy_board_help/data/en/hlpqwk>
REPLY - Reply to a message </crates/icy_board_help/data/en/hlprep>
RM - Remembered message </crates/icy_board_help/data/en/hlprm>
SELECT - Select conferences </crates/icy_board_help/data/en/hlpsel>
TEST - Test files </crates/icy_board_help/data/en/hlptest>
TS - Search message text </crates/icy_board_help/data/en/hlpts>
USERS - Caller directory </crates/icy_board_help/data/en/hlpusers>
WHO - Online callers </crates/icy_board_help/data/en/hlpwho>
```

### Aliases and related actions

| Input | Reference / behavior |
| --- | --- |
| `DOWNLOAD`, `DB` | [Download](../../crates/icy_board_help/data/en/hlpd.md). |
| `BD` | Batch download through the download command. |
| `UPLOAD`, `UB` | [Upload](../../crates/icy_board_help/data/en/hlpu.md). |
| `BU` | Batch upload through the upload command. |
| `HELP` | [Help](../../crates/icy_board_help/data/en/hlph.md). |
| `JOIN` | [Join conference](../../crates/icy_board_help/data/en/hlpj.md). |
| `DOOR` | [Open door](../../crates/icy_board_help/data/en/hlpopen.md). |
| `NODE` | [Group chat](../../crates/icy_board_help/data/en/hlpchat.md). |
| `MENU` | Redisplay the current menu. |
| `BYE` | Immediate logoff rather than the normal goodbye interaction. |

## Message composition and reading

These topics describe commands at secondary prompts, not new main-menu
commands. Read the relevant topic before assuming a letter has its main-menu
meaning.

```{toctree}
:maxdepth: 1

Message composition menu </crates/icy_board_help/data/en/hlpcmenu>
Full-screen message editor </crates/icy_board_help/data/en/hlpfscrn>
End-of-message actions </crates/icy_board_help/data/en/hlpendr>
Message security </crates/icy_board_help/data/en/hlpsec>
Search prompts </crates/icy_board_help/data/en/hlpsrch>
Conference registration flags </crates/icy_board_help/data/en/hlpreg>
```

## Numeric sysop commands

```{toctree}
:maxdepth: 1

/crates/icy_board_help/data/en/hlp1
/crates/icy_board_help/data/en/hlp2
/crates/icy_board_help/data/en/hlp3
/crates/icy_board_help/data/en/hlp4
/crates/icy_board_help/data/en/hlp5
/crates/icy_board_help/data/en/hlp6
/crates/icy_board_help/data/en/hlp7
/crates/icy_board_help/data/en/hlp8
/crates/icy_board_help/data/en/hlp9
/crates/icy_board_help/data/en/hlp10
/crates/icy_board_help/data/en/hlp11
/crates/icy_board_help/data/en/hlp12
/crates/icy_board_help/data/en/hlp13
/crates/icy_board_help/data/en/hlp14
/crates/icy_board_help/data/en/hlp15
/crates/icy_board_help/data/en/hlp16
```

## Installing or translating command help

The website displays the built-in English topics, not an installed board's
custom help. Use the [help-generation workflow](../gettingstarted.md#command-help)
to export sources, edit or translate them, validate their terminal rendering
and install them. A language suffix alone does not translate a topic.

For implementation differences rather than usage, consult the
[command audit](../../compat/COMMAND_AUDIT.md) and
[known limitations](../known_limitations.md).
