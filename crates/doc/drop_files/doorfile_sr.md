# `DOORFILE.SR`

Solar Realms-compatible text file with nine CRLF-terminated lines.

| Line | Value | Meaning |
| ---: | --- | --- |
| 1 | User name or alias | Display identity selected by the session |
| 2 | `0` for CTTY, otherwise `1` | ANSI status |
| 3 | `1` | IBM graphics enabled |
| 4 | User city/state | Caller location |
| 5 | Page length | Screen height in lines |
| 6 | `57600` | Baud rate |
| 7 | `1` | COM port |
| 8 | Minutes remaining | Time limit |
| 9 | User name | Real name |