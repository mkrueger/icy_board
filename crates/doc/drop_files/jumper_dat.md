# `JUMPER.DAT`

2AM BBS-compatible text file with 17 CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | Board name | System name |
| 2 | Sysop name | Operator name |
| 3 | User name | Full caller name |
| 4 | Internal user ID | Zero-based user record number |
| 5 | User first name | First name |
| 6 | User last name | Last name |
| 7 | User city/state | Location |
| 8 | Minutes remaining | Time left |
| 9 | `1` | COM port |
| 10 | `57600` | Baud rate |
| 11 | `0` | Required null count |
| 12 | `FALSE` | Linefeeds disabled |
| 13 | `FALSE` | Uppercase-only disabled |
| 14 | `TRUE` | 80 columns enabled |
| 15 | `TRUE` | IBM graphics enabled |
| 16 | `FALSE` for CTTY, otherwise `TRUE` | ANSI enabled |
| 17 | `FALSE` | System bell disabled |