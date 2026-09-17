# `CHAIN.TXT`

WWIV-compatible text file with 32 CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | Internal user ID | Zero-based user record number |
| 2 | Alias | User handle |
| 3 | User name | Real name |
| 4 | Empty | Amateur-radio callsign |
| 5 | Age | Whole UTC calendar years since birth date; `0` if unavailable |
| 6 | Gender | User gender value |
| 7 | `0` | Gold balance |
| 8 | `MM/DD/YY` | Last call date |
| 9 | Display buffer width | Screen columns |
| 10 | Display buffer height | Screen rows |
| 11 | Current security level | Security level |
| 12 | `0` | Reserved |
| 13 | `1` for sysop, otherwise `0` | Sysop flag |
| 14 | `0` for CTTY, otherwise `1` | ANSI flag |
| 15 | `0` for local, otherwise `1` | Remote flag |
| 16 | Seconds remaining | Time until logoff |
| 17 | `C:\WWIV\GFILES\` | Fixed GFILES directory |
| 18 | `C:\WWIV\DATA\` | Fixed DATA directory |
| 19 | `890519.LOG ` | Fixed log filename, including trailing space |
| 20 | `57600` | Baud rate |
| 21 | `1` | COM port |
| 22 | Board name | System name |
| 23 | Sysop name | Operator name |
| 24 | Login seconds since midnight | Login time |
| 25 | Current UTC time minus login time, in seconds | Elapsed time |
| 26 | Total uploaded bytes / 1024 | Uploaded KiB |
| 27 | Total upload count | Uploaded files |
| 28 | Today's downloaded bytes / 1024 | Downloaded KiB today |
| 29 | Total download count | Downloaded files |
| 30 | `8N1` | Serial framing |
| 31 | `57600` | Locked baud rate |
| 32 | Internal node number | Zero-based node number |