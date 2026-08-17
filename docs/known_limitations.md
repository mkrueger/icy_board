# Known limitations

What icy_board does not do, as of the beta. This is the list to read before
moving a board over, so nothing here is a surprise at two in the morning.

Three companion documents say the same thing in more detail:
[compat/OPTIONS_AUDIT.md](../compat/OPTIONS_AUDIT.md) for the switches a sysop
can set but the board ignores, [compat/COMMAND_AUDIT.md](../compat/COMMAND_AUDIT.md)
for where a command answers differently than PCBoard did, and
[differences.md](differences.md) for the deliberate departures.

## Not implemented

| Area | What is missing |
| :--- | :--- |
| Modem | Callers reach the board over telnet, SSH and websockets. There is no serial or modem support, and no FOSSIL driver. |
| Accounting | PPL `ACCOUNT`/`RECORDUSAGE` and tracking work, but normal board activity is not charged. Balance enforcement, peak rates, money display, credit macros and the warning/info/logoff files are missing. |
| Upload credits | Uploading earns configured byte credit but not time credit, and uploads are not test-extracted. The configured free-space threshold is enforced before a transfer starts. |
| FTN | icy_board is a leaf or point over BinkP: scan, poll and toss. There is no BinkP answering side, netmail arrives in a single dump base, AreaFix is missing and the AKA and link setup is hand-edited TOML. |
| Web | There is no web frontend, and the PPL web statements and functions are not implemented. |
| Sysop numeric commands | Commands `3`, `9`, `10`, `14` and `15` are missing. The level named for command 10 protects `PPE` instead; commands `1`, `2`, `4`, `5`, `6`, `7`, `8`, `11`, `12`, `13` and `16` work. |
| ICBSM | Editing users and groups, sorting and packing the user file, the bulk edits over a selection of users and the security level tables. Reports, index files and the user info file of the original have no equivalent here. |
| German help | 20 of 52 help files are translated. The English set is complete apart from the sysop help, which PCBoard never shipped either. |

## Works, but not the way PCBoard did it

| Area | What to expect |
| :--- | :--- |
| Config files | TOML, editable in any text editor. Old formats are written out again for PPEs that read them, but a PPE that writes one will not be heard. |
| Message bases | JAM. Tools that read PCBoard's old base will not work. |
| DIR files | Binary, they carry the metadata the archives do not. |
| Encoding | Everything is UTF-8 unless a file starts with a CP437 byte order mark. See [differences.md](differences.md). |
| Passwords | Hashed by default. The plain text fallback exists for PPEs that read the password and is a security risk. |
| Access | Security level, group and age instead of a single level. |
| Events | The nightly event runs, clears the board and can suspend callers. PCBoard's per node, expedited, fido and mail event modes have no equivalent, and `EVENT.DAT` is not read. |

## Import

Importing a PCBoard installation is best effort. Simple installations come over
well; the more a board relied on PPEs, absolute paths or drive letters, the more
hand work is left. `icbsetup import --dry-run` reports what it could not
resolve before anything is written, and `--map` translates a drive to a
directory. Every PPE has to be looked at one by one.

The importer is the part that most needs real installations to test against. If
one of yours does not come over, that is worth a bug report more than anything
else on this page.

## What is not planned

DOS, FOSSIL drivers, the PPE DOS and assembler functions, and printer support.
The machine underneath them is gone.
