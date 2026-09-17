# `DORINFOx.DEF`

RBBS/QuickBBS-compatible text file. The actual filename is
`DORINFO{node + 1}.DEF`, where the number is the one-based node number. It
contains 13 CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | Board name | System name |
| 2 | First name of user record 0 | Sysop first name |
| 3 | Last name of user record 0 | Sysop last name |
| 4 | `COM1` | Communications port |
| 5 | `57600 BAUD-R,N,8,1` | Port settings |
| 6 | `0` | Reserved |
| 7 | User first name | Caller first name |
| 8 | User last name | Caller last name |
| 9 | User city/state | Caller location |
| 10 | Graphics mode number | Emulation |
| 11 | Current security level | Security level |
| 12 | Minutes remaining | Time left |
| 13 | `-1` | End marker |

Graphics modes are `0` for CTTY, `2` for Avatar, and `1` for every other
mode.