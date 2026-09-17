# `TRIBBS.SYS`

TriBBS-compatible text file with 18 CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | Internal user ID | Zero-based user record number |
| 2 | User name | Caller name |
| 3 | Door password | Session door password |
| 4 | Current security level | Security level |
| 5 | `Y` or `N` | Expert mode |
| 6 | `N` for CTTY, otherwise `Y` | ANSI enabled |
| 7 | Minutes remaining | Time left |
| 8 | Home phone | Telephone number |
| 9 | User city/state | Location |
| 10 | Internal node number | Zero-based node number |
| 11 | `1` | Serial port |
| 12 | `57600` | Baud rate |
| 13 | `57600` | Locked baud rate |
| 14 | `Y` | Fixed compatibility field; purpose unknown |
| 15 | `Y` | Error-correcting connection |
| 16 | Board name | System name |
| 17 | Sysop name | Operator name |
| 18 | Alias | Caller alias |