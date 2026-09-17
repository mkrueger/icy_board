# `EXITINFO.BBS`

RemoteAccess 2.62-compatible binary drop file. It is always 2267 bytes. All
integers are unsigned little-endian; fixed strings are CP437 and NUL-padded.
Date and time fields are fixed-width ASCII.

| Offset | Size | Type | Value or meaning |
| ---: | ---: | --- | --- |
| 0 | 2 | `u16` | Baud rate truncated from `57600` |
| 2 | 4 | `u32` | Total board calls |
| 6 | 35 | CP437 | Last caller name, or empty |
| 41 | 35 | CP437 | Last caller name repeated, or empty |
| 76 | 92 | bytes | Zero, SYSINFO free space |
| 168 | 8 | ASCII | Login date, `MM-DD-YY` |
| 176 | 48 | `u16[24]` | Zero, busy-per-hour counters |
| 224 | 14 | `u16[7]` | Zero, busy-per-day counters |
| 238 | 35 | CP437 | User name |
| 273 | 25 | CP437 | City/state |
| 298 | 50 | CP437 | Empty organisation |
| 348 | 50 | CP437 | Street line 1 |
| 398 | 50 | CP437 | Street line 2 |
| 448 | 50 | CP437 | City |
| 498 | 35 | CP437 | Alias |
| 533 | 80 | CP437 | User comment |
| 613 | 4 | `u32` | CRC32 of the stored user password bytes |
| 617 | 15 | CP437 | Business/data phone |
| 632 | 15 | CP437 | Home/voice phone |
| 647 | 8 | ASCII | Last-on date, `MM-DD-YY` |
| 655 | 1 | bitmap | Attribute: bit 3 for ANSI, Avatar, or RIP |
| 656 | 1 | bitmap | Attribute2: bit 1 for Avatar |
| 657 | 8 | `u32[2]` | Zero, credit and pending |
| 665 | 2 | `u16` | Messages left |
| 667 | 2 | `u16` | Current security level |
| 669 | 4 | `u32` | Zero, last-read pointer |
| 673 | 4 | `u32` | Number of calls |
| 677 | 4 | `u32` | Upload count |
| 681 | 4 | `u32` | Download count |
| 685 | 4 | `u32` | Total uploaded bytes / 1024 |
| 689 | 4 | `u32` | Total downloaded bytes / 1024 |
| 693 | 4 | `u32` | Today's downloaded bytes / 1024 |
| 697 | 4 | `u32` | Zero, elapsed time |
| 701 | 2 | `u16` | Page length |
| 703 | 1 | `u8` | Zero, last password change |
| 704 | 2 | `u16` | Zero, group |
| 706 | 400 | `u16[200]` | Zero, combined information |
| 1106 | 8 | ASCII | First-on date, `MM-DD-YY` |
| 1114 | 8 | ASCII | Birth date, `DD-MM-YY` |
| 1122 | 8 | ASCII | First-on date repeated as subscription date |
| 1130 | 1 | `u8` | Display width, truncated to one byte |
| 1131 | 1 | `u8` | Zero, language |
| 1132 | 1 | `u8` | `2`, MM-DD-YY date format |
| 1133 | 35 | CP437 | Empty forwarding address |
| 1168 | 4 | `u16[2]` | Zero, message and file areas |
| 1172 | 1 | byte | First byte of default protocol, or space |
| 1173 | 2 | `u16` | Current conference as file group |
| 1175 | 1 | `u8` | Zero, last birth-date check |
| 1176 | 2 | `u16` | Current conference as message group |
| 1178 | 1 | `u8` | Zero, Attribute3 |
| 1179 | 15 | CP437 | Door password |
| 1194 | 8 | ASCII | Century prefixes: last-on, first-on, birth, first-on |
| 1202 | 19 | bytes | Zero, user-record free space |
| 1221 | 1 | `u8` | `2`, event disabled |
| 1222 | 5 | ASCII | `00:00`, event start |
| 1227 | 3 | bytes | Zero, error level/days/forced |
| 1230 | 8 | ASCII | `00-00-00`, event date |
| 1238 | 2 | bytes | Zero, netmail/echomail entered |
| 1240 | 5 | ASCII | Login time, `HH:MM` |
| 1245 | 8 | ASCII | Login date, `MM-DD-YY` |
| 1253 | 2 | `u16` | Minutes remaining |
| 1255 | 4 | `u32` | User's stored security level |
| 1259 | 4 | `u32` | Internal user ID |
| 1263 | 6 | `u16[3]` | Zero, read-through/pages/download limit |
| 1269 | 5 | ASCII | Current local time, `HH:MM` |
| 1274 | 4 | `u32` | Password CRC32 repeated |
| 1278 | 1 | `u8` | Zero, wants-chat flag |
| 1279 | 4 | `u32` | Zero, deducted time |
| 1283 | 400 | CP437 | 50 empty eight-byte menu-stack entries |
| 1683 | 1 | `u8` | Zero, menu-stack pointer |
| 1684 | 200 | bytes | Zero, UserXI free space |
| 1884 | 1 | `u8` | `1`, error-free connection |
| 1885 | 1 | `u8` | Zero, sysop-next flag |
| 1886 | 201 | mixed | EMSI flag and five 40-byte CP437 fields |
| 2087 | 3 | bytes | Zero, hold attributes and length |
| 2090 | 80 | CP437 | Empty page reason |
| 2170 | 1 | `u8` | Zero, status line |
| 2171 | 8 | CP437 | Empty last-cost menu |
| 2179 | 2 | `u16` | Zero, menu cost per minute |
| 2181 | 1 | `u8` | `1` for Avatar, otherwise `0` |
| 2182 | 1 | `u8` | `1` for RIP, otherwise `0` |
| 2183 | 1 | `u8` | Zero, RIP version |
| 2184 | 4 | ASCII | Last-on and login century prefixes |
| 2188 | 79 | bytes | Zero, trailing free space |

The resulting size is 2267 bytes, despite older RemoteAccess layouts commonly
being shorter. When EMSI data is absent, its five strings are empty but retain
their fixed widths.