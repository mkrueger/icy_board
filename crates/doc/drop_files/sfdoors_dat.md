# `SFDOORS.DAT`

SpitFire-compatible text file with 39 CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | Internal user ID | Zero-based user number |
| 2 | User name | Caller name |
| 3 | Door password | Session door password |
| 4 | User first name | First name |
| 5 | `57600` | Baud rate |
| 6 | `1` | COM port |
| 7 | Minutes remaining | Time left |
| 8 | Local seconds since midnight | Current time |
| 9 | `C:\SFBBS\` | Fixed SpitFire directory |
| 10 | `FALSE` for CTTY, otherwise `TRUE` | Graphics enabled |
| 11 | Current security level | Security level |
| 12 | Total uploads | Upload count |
| 13 | Total downloads | Download count |
| 14 | Minutes remaining | Time left, repeated |
| 15 | Login seconds since midnight | Login time |
| 16 | `0` | Extra seconds |
| 17 | `FALSȨ` | Sysop-next flag as currently emitted |
| 18 | `FALSȨ` | Front-end flag as currently emitted |
| 19 | `TRUE` for local, otherwise `FALSE` | Local flag |
| 20 | `57600` | Locked baud rate |
| 21 | `FALSE` | Error-correcting connection |
| 22 | Current conference | Conference number |
| 23 | `1` | Last file area |
| 24 | Internal node number + 1 | One-based node number |
| 25 | `32768` | Downloads allowed per day |
| 26 | Downloads today | Daily download count |
| 27 | `1000000` | Download bytes allowed per day |
| 28 | Downloaded bytes today | Daily downloaded bytes |
| 29 | Total uploaded bytes / 1024 | Uploaded KiB |
| 30 | Total downloaded bytes / 1024 | Downloaded KiB |
| 31 | Home phone | Telephone number |
| 32 | User city/state | Location |
| 33 | `3600` | Minutes allowed per day |
| 34 | `FALSE` | Fixed field; purpose unknown |
| 35 | `FALSE` | Fixed field; purpose unknown |
| 36 | `32767` | Fixed field; purpose unknown |
| 37 | `1` | COM IRQ |
| 38 | `1000` | Serial I/O port |
| 39 | `00-00-80` | Subscription date disabled |

Lines 17 and 18 deliberately document the current non-ASCII `FALSȨ` output;
this differs from the conventional `FALSE` token.