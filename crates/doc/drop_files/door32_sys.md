# `door32.sys`

Mystic DOOR32 revision 1 compatible text file. The filename is intentionally
lowercase. The file contains 11 CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | `0` | Communication type: local |
| 2 | `0` | Communication or socket handle |
| 3 | `57600` | Baud rate |
| 4 | `Icy Board {version}` | BBS software identifier |
| 5 | User record ID + 1 | One-based user record position |
| 6 | User name | Real name |
| 7 | Alias | Handle or alias |
| 8 | Current security level | Security level |
| 9 | Minutes remaining | Time left |
| 10 | Graphics mode number | Emulation; see below |
| 11 | Internal node number + 1 | One-based node number |

Graphics modes are `0` for ASCII/CTTY, `1` for ANSI, `2` for Avatar, `3` for
RIP, and `4` for Icy Board's Graphics mode.