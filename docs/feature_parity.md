# State of PCBoard features

That is the current state of PCBoard features supported.
Not the state of ICY BOARD features - that would make it too long and it's important to know what works
from PCBoard.

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
| PCBSETUP  | 90% | Most is implemented, but 38 of the switches are read by nobody - see the options audit |
| Call Waiting Screen  | 💯 | Almost the same, some improvements |
| New User Creation  | 💯 | Much better & detailed |
| Security Level check  | 💯 | |
| Local logons  | 💯  | | 
| Sysop local session view  | 💯  | Some ppl may hate it but sysops can view local sessions and chat 
| Doors  | 💯 | Much more drop files supported + BBSLINK |
| Bullettins | 💯 | | 
| Questionnaires | 💯 | Renamed them so "Surveys" | 
| Built in Message Editor | 80% | I consider line & fse done but needs 1-2 test passes to the real one to make it 100% | 
| PPE Runtime  | 90% | Every existing PPE not running is considered as a bug. Due to the Nature the PPE runtime it won't reach 100% since it's not running on DOS anymore. dBase III statements and functions are in. |
| Conferences  | 90% | Basically works, INTRO and NEWS are displayed on join |
| @ Macro support | 80% | Most should work, all @ features work  | 
| File Bases  | 90% | SQLite base with the metadata the archives do not carry, long file names, archives read through unarc-rs |
| Mail Bases | 80% | JAM base, search, QWK and an FTN leaf; netmail still lands in one dump base |
| Up/Download  | 90%  | Commands need to be checked for 100% parity, but protocols should work |
| Statistics | 80%  | Calls, messages, uploads and downloads are counted for the board and the caller, and the daily figures roll over; transfer limits still do not read them |
| Help Files | 80%  | Every command reaches a help file, the German set is 20 of 52 | 
| Modem Support | Not started yet | Telnet, SSH and websockets work |
| Limits | 85% | Time, ratios, credits and daily/total byte limits are enforced from the PWRD level, off until switched on; accounting credits are not modelled |
| Events | 80% | The nightly event runs, clears the board and can suspend callers; PCBoard's per node and expedited modes are missing | 
| Subscriptions | 20% | Expiry is warned about, a new period is never set and an expired user keeps their level | 
| Accounting | 20% | Charges and the warning file work, peak rates and the money display do not | 

## PCBoard Commands

| Command | Description | Progress | Notes | 
| :--- | :--- | :--- | :--- | 
| A  | Abandon  | 💯 | 
| B  | Bullettins | 💯 | 
| C  | Comment to Sysop  | 💯 | 
| D  | Download | 90% | 
| E  | Enter Msg  | 90% | 
| F  | Files  | 90% | 
| G  | Goodbye | 💯 | 
| H  | Help  | 💯 | 
| I  | Initial Welcome  | 💯 | 
| J  | Join Conference  | 💯 | 
| K  | Delete Message | 90% | 
| L  | Find Files | 💯 | 
| M  | Toggle Graphics  | 💯 | 
| N  | New Files | 💯 | 
| O  | Page Sysop | 90% | 
| P  | Set Page Length | 💯 | 
| Q  | Quick Message Scan | 90% | 
| R  | Read Message | 70% | 
| S  | Take Survey  | 💯 | 
| T  | Set Transfer Protocol | 💯 | 
| U  | Upload  | 90% | 
| V  | View Settings  | 90% | 
| W  | Write Settings  | 90% | 
| X  | Toggle Expert Mode  | 💯 | 
| Y  | Your Mail Scan  | 70% | 
| Z  | Zippy Directory Scan  | 💯 | 
| ALIAS  |  Alias | 💯 | 
| BYE  | Force logoff | 💯 | 
| FLAG  | Flag Files | 💯 | 
| LANG  | Set Language | 💯 | 
| NEWS  | Display News | 💯 | 
| OPEN  | Open Door | 💯 |  Alias: DOOR
| PPE  | Run PPE | 💯 | 
| !  | Recall Command | 💯 | 
| MENU  | Redisplay Menu | 💯 | 
| REPLY  | Reply Message | 💯 | 
| USER  | User List | 90% | 
| WHO  |WHO is Online | 90% | 
| QWK  | QWK command | 90% | Download, upload and the scanned bases work; upload needs more testing [^2]
| CHAT  | Group Chat| 💯 | Built in, the PPEs are no longer needed
| BD/DB  | Batch Download | 90% | Delegates to the download command with the batch flag
| BU/UB  | Batch Upload | 90% | Delegates to the upload command, which drives a batch protocol
| RM  | Read Message | 💯 | Read remembered message
| SELECT | Select Conference | 99% | Changes were needed due to message areas [^1]
| TEST | Test File | 💯 | Slight improvements - search for pattern

[^1]: PCBTEXT #586 changed to `Conference`,
  #587 changed to `#   Name                                                   Flags`

[^2]: PCBTEXT #678 `QWK Commands: (D)ownload, (U)pload, (S)canned bases`

# Unsupported Features

Some things will never work/out of scope.

| Feature | Reason | 
| :--- | :--- |
| DOS | Purely outdated - 90% of the libs I use won't run and icy board is too memory hungry for the DOS world. | 
| Fossil drivers | See above | 
| PPE DOS/Assembler functions | See above | 
| Printer support | Are you serious? | 
