# State of PCBoard features

That is the current state of PCBoard features supported.
Not the state of ICY BOARD features - that would make it too long and it's important to know what works
from PCBoard.

For the deliberate improvements that are not part of PCBoard parity, see
[differences and improvements](differences.md). For a production-readiness
checklist, see [known limitations](known_limitations.md); for an old board, use
the [migration guide](migration.md).

A percentage here is what the code does, not what it is meant to do. What a
sysop can set but the board ignores is listed in
[compat/OPTIONS_AUDIT.md](../compat/OPTIONS_AUDIT.md), where a command answers
differently than the original in
[compat/COMMAND_AUDIT.md](../compat/COMMAND_AUDIT.md), and what is missing
altogether in [known limitations](known_limitations.md).

If something is missing just let me know.

# Supported Features

| Feature | Progress | Notes | 
| :--- | :--- | :--- | 
| Importing PCBoard installations | 60%  | I need test cases - simple ones work | 
| Creating new installations | 💯 |  PCBoard did it during install - icy board with icbsetup | 
| PPLC  | 💯 | Better than the original |
| PPLD  | 💯 | Better than the original. Was 3rd party software but it's important for icy board. |
| MKPCBTXT  | 💯 | Much better |
| ICBSM      | 75% | User and group editor plus the bulk maintenance; no reports or index files |
| MKPCBMNU  | 💯 | MKICBMNU can do much more |
| PCBSETUP  | 80% | Most runtime options work; 19 of 126 options and 11 of 29 sysop security levels are still inactive - see the options audit |
| Call Waiting Screen  | 💯 | Almost the same, some improvements |
| New User Creation  | 💯 | Much better & detailed |
| Security and Access Checks | 85% | User command, conference, group and age expressions work; 11 of 29 configurable sysop security levels are still inactive |
| Languages | 80% | Language files and `LANG` work; the global multilingual enable switch is currently ignored |
| Local logons  | 💯  | | 
| Sysop local session view  | 💯  | Some ppl may hate it but sysops can view local sessions and chat 
| Doors  | 💯 | Much more drop files supported + BBSLINK |
| Bulletins | 💯 | |
| Surveys | 💯 | PCBoard called them questionnaires |
| Built in Message Editor | 80% | I consider line & fse done but needs 1-2 test passes to the real one to make it 100% | 
| PPE Runtime  | 90% | Every existing PPE not running is considered as a bug. Due to the Nature the PPE runtime it won't reach 100% since it's not running on DOS anymore. dBase III statements and functions are in. |
| Conferences  | 90% | Basically works, INTRO and NEWS are displayed on join |
| @ Macro support | 80% | Most work; accounting credits, event/off-hours, free-space and a few caller/password macros remain stubs |
| File Bases  | 90% | SQLite base with the metadata the archives do not carry, long file names, archives read through unarc-rs |
| Mail Bases | 80% | JAM base, search, QWK and an FTN leaf; netmail still lands in one dump base |
| FTN Mailer | 70% | Leaf/point scan, poll and toss over BinkP work; no answering side, AreaFix, per-user netmail or setup UI for AKA/links |
| Up/Download  | 90%  | Commands need to be checked for 100% parity, but protocols should work |
| Statistics | 80%  | Board and caller activity, daily rollover and per-file download counts work; PCBoard's per-node statistics are not modelled |
| Help Files | 80%  | Every command reaches a help file, the German set is 20 of 52 | 
| Serial/Modem Support | Not started | Telnet, SSH and websockets work; serial ports, FOSSIL and modem control are out of scope |
| Limits | 85% | PWRD time, ratios, credits and daily/total byte/file limits work; FSEC `NOTIME` and per-file `FREE` are not imported |
| Events | 80% | The nightly event runs, clears the board and can suspend callers; PCBoard's per node and expedited modes are missing | 
| Subscriptions | 90% | New-user periods, warning/expired files, temporary expired security, R/X conference access, macros and sysop renewal work; no payment-driven renewal exists |
| Accounting | 20% | Config plus PPL `ACCOUNT`/`RECORDUSAGE` and tracking work; built-in actions do not charge, balance enforcement, peak rates, credit macros and display files are missing |

## PCBoard Commands

| Command | Description | Progress | Notes | 
| :--- | :--- | :--- | :--- | 
| A  | Abandon  | 💯 | 
| B  | Bulletins | 💯 |
| C  | Comment to Sysop  | 💯 | 
| D  | Download | 90% | Filename/prompt flow and limits work; message capture and last-viewed filename default are missing. Aliases: `DB`, `DOWNLOAD` |
| E  | Enter Msg  | 90% | 
| F  | Files  | 90% | 
| G  | Goodbye | 💯 | 
| H  | Help  | 💯 | Alias: `HELP` |
| I  | Initial Welcome  | 💯 | 
| J  | Join Conference  | 💯 | Alias: `JOIN` |
| K  | Delete Message | 90% | 
| L  | Find Files | 💯 | 
| M  | Toggle Graphics  | 💯 | 
| N  | New Files | 💯 | 
| O  | Page Sysop | 90% | Issue is that Sysop doesn't get informed. Need a new way - maybe an App. But it works if sysop is around and watching the session.
| P  | Set Page Length | 💯 | 
| Q  | Quick Message Scan | 💯 | Scans every area of the conference [^3] |
| R  | Read Message | 85% | Prompt and read loops match; REPLY, WHO, CHAT, JOIN, E, SKIP, JUMP, SEL/DESEL, Q, FLAG and F/TO run in the loop. Export, EDIT, FORWARD, VIEW and the capture actions (`C/D/Z`) are parsed and answered but not carried out |
| S  | Take Survey  | 💯 | 
| T  | Set Transfer Protocol | 💯 | 
| U  | Upload  | 90% | Description, private/public placement, batch protocol and byte credits work; verification/test-extraction is missing. Aliases: `UB`, `UPLOAD` |
| V  | View Settings  | 💯 | Every line of PCBoard's block; falls back to a built-in display when the `STAT` file is absent |
| W  | Write Settings  | 💯 | 
| X  | Toggle Expert Mode  | 💯 | 
| Y  | Your Mail Scan  | 💯 | Quick and long form, scan direction and conference selection as in the original, plus the private mail base [^3] |
| Z  | Zippy Directory Scan  | 💯 | 
| ALIAS  |  Alias | 💯 | 
| BROADCAST | Broadcast to nodes | 💯 | Sysop word command |
| BYE  | Force logoff | 💯 | 
| FLAG  | Flag Files | 💯 | 
| LANG  | Set Language | 💯 | 
| NEWS  | Display News | 💯 | 
| OPEN  | Open Door | 💯 |  Alias: DOOR
| PPE  | Run PPE | 💯 | 
| !  | Recall Command | 💯 | 
| MENU  | Redisplay Menu | 💯 | 
| REPLY  | Reply Message | 💯 | 
| USERS  | User List | 💯 | Only the callers registered in the conference, searched by name and location |
| WHO  |WHO is Online | 💯 | Node, status and caller as in the original; `X` adds the operation line for sysops |
| QWK  | QWK command | 90% | Download, upload and the scanned bases work; upload needs more testing [^2]
| CHAT  | Group Chat| 💯 | Built in, the PPEs are no longer needed
| NODE | Group Chat alias | 💯 | PCBoard alias for `CHAT` |
| TS | Text search | 💯 | Searches message text across selected areas |
| BD | Batch Download | 90% | Delegates to the download command with the batch flag |
| BU | Batch Upload | 90% | Delegates to the upload command, which drives a batch protocol |
| RM  | Read Message | 💯 | Read remembered message
| SELECT | Select Conference | 99% | Changes were needed due to message areas [^1]
| TEST | Test File | 💯 | Slight improvements - search for pattern

## Sysop Numeric Commands

| Command | Description | Progress | Notes |
| :--- | :--- | :--- | :--- |
| 1 | View caller log | 💯 | Shared node-stamped log instead of one DOS file per node |
| 2 | View/print users | 90% | Listing works; printer output is intentionally absent |
| 3 | Pack message base | 90% | PCBoard's prompts and criteria, run in-process over every area of the conference instead of shelling out to PCBPack |
| 4 | Recover message | 💯 | |
| 5 | Quick/header scan | 💯 | |
| 6 | View text file | 💯 | Confined to the board directory |
| 7 | User maintenance | 80% | Browse, find, delete/undelete and change expiration; full record editing remains in ICBSM |
| 8 | Pack users file | 90% | Record 1 only; protects online users and writes a backup |
| 9 | Remote DOS | Out of scope | DOS shelling is intentionally unsupported |
| 10 | DOS command | Out of scope | The configured level currently protects `PPE`; DOS commands are unsupported |
| 11 | Node list | 💯 | |
| 12 | Log off node | 💯 | Uses the board's node shutdown channel |
| 13 | View node caller log | 💯 | Filters the shared caller log by node |
| 14 | Drop node to DOS | Out of scope | DOS shelling is intentionally unsupported |
| 15 | Recycle node | Missing | A non-DOS recycle equivalent has not been implemented |
| 16 | Directory listing | 90% | Safe name/size/date listing rather than shelling out to DOS `DIR` |

[^1]: PCBTEXT #586 changed to `Conference`,
  #587 changed to `#   Name                                                   Flags`

[^2]: PCBTEXT #678 `QWK Commands: (D)ownload, (U)pload, (S)canned bases`

[^3]: A conference holds several message areas here, so a scan covers all of
  them. `Y` also reports the private mail base, which PCBoard had no equivalent
  for, on a line of its own (PCBTEXT #779 `E-Mail`).

# Unsupported Features

Some things will never work/out of scope.

| Feature | Reason | 
| :--- | :--- |
| DOS | Purely outdated - 90% of the libs I use won't run and icy board is too memory hungry for the DOS world. | 
| Fossil drivers | See above | 
| PPE DOS/Assembler functions | See above | 
| Printer support | Are you serious? | 
