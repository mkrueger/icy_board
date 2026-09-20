# `PCBOARD.SYS`

Icy Board writes a fixed 148-byte, PCBoard 15-style binary file. Fixed-width
text is CP437 and space-padded. Numeric fields are little-endian; elapsed
time at offset 54 is a negative signed 16-bit value.

| Offset | Size | Type | Value or meaning |
| ---: | ---: | --- | --- |
| 0 | 2 | text | `-1`, display on |
| 2 | 2 | text | ` 0`, printer off |
| 4 | 2 | text | ` 0`, page bell off |
| 6 | 2 | text | ` 0`, caller alarm off |
| 8 | 1 | text | Space, sysop flag |
| 9 | 2 | text | `-1`, error correction on |
| 11 | 1 | text | `N` for CTTY, otherwise `Y` |
| 12 | 1 | text | `U`, node chat unavailable |
| 13 | 5 | text | `57600`, DTE rate |
| 18 | 5 | text | `57600`, connect rate, including local callers bridged over COM1 |
| 23 | 2 | `u16` | One-based user record number, capped at 65535 |
| 25 | 15 | CP437 | First name, space-padded |
| 40 | 12 | CP437 | Door password, space-padded |
| 52 | 2 | `u16` | Login minutes since midnight |
| 54 | 2 | `i16` | Negative minutes elapsed since login, clamped to -32767..0 |
| 56 | 5 | text | Login time as `HH:MM` |
| 61 | 2 | `u16` | `32767`, allowed minutes |
| 63 | 2 | `u16` | `32767`, allowed download KiB |
| 65 | 1 | `u8` | Conference if at most 255, otherwise `255` |
| 66 | 5 | bitmap | Joined conferences; all zero |
| 71 | 5 | bitmap | Scanned conferences; all zero |
| 76 | 2 | `u16` | Current conference additional minutes |
| 78 | 2 | `u16` | `0`, upload/chat credit |
| 80 | 4 | CP437 | Language extension, space-padded |
| 84 | 25 | CP437 | Full user name, space-padded |
| 109 | 2 | `u16` | Minutes remaining |
| 111 | 1 | `u8` | One-based node number, capped at 255 |
| 112 | 5 | text | `00:00`, event time |
| 117 | 2 | text | ` 0`, event inactive |
| 119 | 2 | bytes | Spaces, reserved |
| 121 | 4 | bytes | Zero, memorized message number |
| 125 | 1 | ASCII | `'1'` (byte 49), COM port |
| 126 | 2 | bytes | Zero, reserved and flags |
| 128 | 1 | `u8` | `0` for CTTY, otherwise `1`, ANSI flag |
| 129 | 2 | `u16` | `1`, country code |
| 131 | 2 | `u16` | `437`, code page identifier |
| 133 | 1 | `u8` | Session yes character, truncated to one byte |
| 134 | 1 | `u8` | Session no character, truncated to one byte |
| 135 | 4 | bytes | Zero, language and reserved bytes |
| 139 | 1 | `u8` | `0`, caller-exited-to-DOS flag |
| 140 | 1 | `u8` | `0`, reserved |
| 141 | 1 | `u8` | `0`, stop-uploads flag |
| 142 | 2 | `u16` | Full conference number |
| 144 | 1 | bitmap | High joined conferences; zero |
| 145 | 1 | bitmap | High scanned conferences; zero |
| 146 | 2 | `u16` | One-based node number, capped at 65535 |

The historical source text is retained in [pcboard_sys.txt](pcboard_sys.txt).