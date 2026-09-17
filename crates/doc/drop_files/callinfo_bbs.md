# `CALLINFO.BBS`

RBBS-style `CALLINFO.BBS` text file with 36 CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | User name | Caller name |
| 2 | `5` | Local baud code |
| 3 | User city/state | Caller location |
| 4 | Current security level | Security level |
| 5 | Minutes remaining | Time left |
| 6 | `MONO` for CTTY, otherwise `COLOR` | Display mode |
| 7 | Door password | Session door password |
| 8 | Internal user ID + 1 | One-based user reference |
| 9 | `0` | Time online |
| 10 | `HH:MM` | Login time |
| 11 | `HH:MM MM/DD/YY` | Login time and date |
| 12 | Current conference | Conference number |
| 13 | Downloads today | Daily download count |
| 14 | `999` | Maximum downloads |
| 15 | Today's downloaded bytes / 1024 | Daily downloaded KiB |
| 16 | `1022976` | Maximum download KiB (`999 * 1024`) |
| 17 | Home phone | Telephone number |
| 18 | `MM/DD/YY HH:MM` | Current local date and time |
| 19 | `EXPERT` or `NOVICE` | User mode |
| 20 | `All` | Transfer method |
| 21 | `MM/DD/YY` | Last call date |
| 22 | Total calls | Times online |
| 23 | Page length | Lines per page |
| 24 | `42` | Highest message read |
| 25 | Total uploads | Upload count |
| 26 | Total downloads | Download count |
| 27 | `8` | Data bits |
| 28 | `LOCAL` or `REMOTE` | Connection location |
| 29 | `COM1` | COM port |
| 30 | Country-formatted birth date | Birth date |
| 31 | `57600` | Baud rate |
| 32 | `TRUE` | Already connected |
| 33 | `Normal Connection ` | Connection type, including trailing space |
| 34 | `MM/DD/YY HH:MM` | Current UTC date and time |
| 35 | Internal node number + 1 | One-based node ID |
| 36 | Door list index | Door number passed to the writer |

Line 11 currently uses the formatter `%H:%M %m/%d%/%y`; the extra `%` before
the second slash is emitted literally. Line 18 uses local time while line 34
uses UTC.