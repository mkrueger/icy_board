# State of PCBoard features

That is the current state of PCBoard features supported.
Not the state of ICY BOARD features - that would make it too long and it's important to know what works
from PCBoard.

If something is missing just let me know.

# Supported Features

| Feature | Progress | Notes | 
| :--- | :--- | :--- | 
| Importing PCBoard installations | 60%  | I need test cases - simple ones work | 
| Creating new installations | 💯 |  PCBoard did it during install - icy board with icbsetup | 
| PPLC  | 💯 | Better than the original |
| PPLD  | 💯 | Better than the original. Was 3rd party software but it's important for icy board. |
| MKPCBTXT  | 💯 | Much better |
| ICBSysMgr  | 40% | Edit user files work but nothing else |
| MKPCBMNU  | 💯 | MKICBMNU can do much more |
| PCBSETUP  | 90% | Most is implemented  |
| Call Waiting Screen  | 💯 | Almost the same, some improvements |
| New User Creation  | 💯 | Much better & detailed |
| Security Level check  | 💯 | |
| Local logons  | 💯  | | 
| Sysop local session view  | 💯  | Some ppl may hate it but sysops can view local sessions and chat 
| Doors  | 💯 | Much more drop files supported + BBSLINK |
| Bullettins | 💯 | | 
| Questionnaires | 💯 | Renamed them so "Surveys" | 
| Built in Message Editor | 80% | I consider line & fse done but needs 1-2 test passes to the real one to make it 100% | 
| PPE Runtime  | 90% | Every existing PPE not running is considered as a bug. Due to the Nature the PPE runtime it won't reach 100% since it's not running on DOS anymore. DBASE3 support missing.| 
| Conferences  | 90% | Basically works, No INTRO SCAN/NEWS yet |
| @ Macro support | 80% | Most should work, all @ features work  | 
| File Bases  | 80% | No testing/checking, metadata missing and a solution for long file names needed |
| Mail Bases | 50% | Qwk, Search, FTNs Missing  |
| Up/Download  | 90%  | Commands need to be checked for 100% parity, but protocols should work |
| Statistics | 30%  | Works a bit but not checked - modelling is done | 
| Help Files | 80%  | Mostly done | 
| Modem Support | Not started yet | Telnet works |
| Limits | 10% | In Setup but not checked | 
| Events | 10% | Data Strucutures are there - rest not implemented | 
| Subscriptions | 10% | Data Strucutures are there - rest not implemented | 
| Accounting | 10% | Data Strucutures are there - rest not implemented | 

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
| QWK  | QWK command | 50% | Needed some changes. Upload missing atm [^2]
| CHAT  | Group Chat| 0% | Note: There are working PPEs for that
| BD/DB  | Batch Download | 0% | 
| BU/UB  | Batch Upload | 0% | 
| RM  | Read Message | 0% | Read remembered message
| SELECT | Select Conference | 99% | Changes were needed due to message areas [^1]
| TEST | Test File | 0% | 

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
