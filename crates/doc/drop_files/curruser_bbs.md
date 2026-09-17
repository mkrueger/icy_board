# `CURRUSER.BBS`

RyBBS-compatible text file with 14 CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | User name | Caller name |
| 2 | Current security level | Security level |
| 3 | Internal user ID | Zero-based user record number |
| 4 | Home phone | Telephone number |
| 5 | User city/state | Location |
| 6 | `1` | COM port |
| 7 | `57600` | Baud rate |
| 8 | `N` | Parity |
| 9 | `8` | Data bits |
| 10 | `1` | Stop bits |
| 11 | Empty | Reserved |
| 12 | `DOORM.MNU` | Menu filename |
| 13 | Minutes remaining | Time left |
| 14 | Terminal mode | Emulation |

Terminal mode is `NONE` for CTTY, `IBM` for ANSI, and `ANSI` for Graphics,
Avatar, or RIP.