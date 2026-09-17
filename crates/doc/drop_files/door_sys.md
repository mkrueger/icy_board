# `DOOR.SYS`

GAP-compatible 52-line text file. Every line is CRLF-terminated.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | `COM1:` | COM port |
| 2 | `57600` | Baud rate |
| 3 | `8` | Data bits |
| 4 | Internal node number + 1 | One-based node number |
| 5 | `57600` | Locked baud rate |
| 6 | `Y` | Screen display enabled |
| 7 | `N` | Printer disabled |
| 8 | `N` | Page bell disabled |
| 9 | `N` | Caller alarm disabled |
| 10 | User name | Full caller name |
| 11 | User city/state | Location |
| 12 | Home phone | Voice phone |
| 13 | Business phone | Data phone |
| 14 | Door password | Session door password |
| 15 | Current security level | Security level |
| 16 | Total calls | Times online |
| 17 | `MM/DD/YY` | Last call date |
| 18 | Seconds remaining | Time left in seconds |
| 19 | Minutes remaining | Time left in minutes |
| 20 | `NG` for CTTY, otherwise `GR` | Graphics mode |
| 21 | Page length | Screen lines |
| 22 | `Y` or `N` | Expert mode |
| 23 | Empty | Reserved |
| 24 | Empty | Reserved |
| 25 | `01/01/99` | Expiration date |
| 26 | Internal user ID + 1 | One-based user record number |
| 27 | Default protocol | Protocol identifier |
| 28 | Total uploads | Uploaded files |
| 29 | Total downloads | Downloaded files |
| 30 | Today's downloaded bytes / 1024 | Daily downloaded KiB |
| 31 | `999999` | Daily download KiB limit |
| 32 | Country-formatted birth date | Birth date |
| 33 | `C:\HOME` | User database path |
| 34 | `C:\MSGS` | Message database path |
| 35 | Sysop name | Operator name |
| 36 | Alias | User handle |
| 37 | `00:00` | Next event time |
| 38 | `Y` | Error-free connection |
| 39 | `N` | Fixed compatibility field |
| 40 | `Y` | Fixed compatibility field |
| 41 | Default DOS foreground color, or `7` | BBS color |
| 42 | `0` | Fixed compatibility field |
| 43 | `01/01/70` | Last new-files scan date |
| 44 | `HH:MM` | Login time |
| 45 | `HH:MM` | Previous login time |
| 46 | `32768` | Fixed compatibility field |
| 47 | Downloads today | Daily file count |
| 48 | Total uploaded bytes / 1024 | Uploaded KiB |
| 49 | Total downloaded bytes / 1024 | Downloaded KiB |
| 50 | User comment | Comment |
| 51 | Doors executed | Door count |
| 52 | Messages left | Posted message count |

The foreground color is the configured DOS color modulo 15. Non-DOS or unset
colors produce `7`.