# `USER.SYS`

PCBoard-compatible binary user exchange file. Fixed-width strings are CP437,
NUL-padded. Integers are unsigned little-endian. The current file is 36 bytes
when no current user exists and 484 bytes when a current user exists.

## Header

| Offset | Size | Type | Value or meaning |
| ---: | ---: | --- | --- |
| 0 | 2 | `u16` | `1530`, PCBoard version |
| 2 | 4 | `u32` | Internal user ID |
| 6 | 2 | `u16` | `400`, advertised fixed record size |
| 8 | 2 | `u16` | `5`, bit-field size |
| 10 | 15 | CP437 | Third-party application name; empty in normal drop files |
| 25 | 2 | `u16` | `0`, application version |
| 27 | 2 | `u16` | `0`, application record size |
| 29 | 2 | `u16` | `0`, application conference record size |
| 31 | 4 | `u32` | `0`, application record offset |
| 35 | 1 | `u8` | `0`, updated flag |

Unlike the historical `syshdrtype`, the emitted header does not contain a
separate `NumOfAreas` or `NumOfBitFields` field. Offset 8 is written directly
as the bit-field size.

## User record

| Offset | Size | Type | Value or meaning |
| ---: | ---: | --- | --- |
| 36 | 26 | CP437 | User name |
| 62 | 25 | CP437 | City/state |
| 87 | 13 | CP437 | Door password |
| 100 | 14 | CP437 | Business/data phone |
| 114 | 14 | CP437 | Home/voice phone |
| 128 | 2 | `u16` | Last-on date in PCBoard date form |
| 130 | 1 | `u8` | Expert mode boolean |
| 131 | 1 | byte | First byte of default protocol, or space |
| 132 | 1 | bitmap | User flags; see below |
| 133 | 2 | `u16` | `0`, last directory scan date |
| 135 | 4 | `u32` | Current security level |
| 139 | 2 | `u16` | Number of calls |
| 141 | 1 | `u8` | Page length |
| 142 | 2 | `u16` | Upload count |
| 144 | 2 | `u16` | Download count |
| 146 | 4 | `u32` | Downloaded bytes today |
| 150 | 31 | CP437 | User comment |
| 181 | 31 | CP437 | Sysop comment |
| 212 | 4 | `u32` | Downloaded bytes today, repeated |
| 216 | 4 | `u32` | Minutes elapsed since login |
| 220 | 2 | `u16` | `0`, registration expiration date |
| 222 | 4 | `u32` | `0`, expired security level |
| 226 | 2 | `u16` | `0`, last conference |
| 228 | 4 | `u32` | Total downloaded bytes |
| 232 | 4 | `u32` | Total uploaded bytes |
| 236 | 1 | `u8` | `0`, delete flag |
| 237 | 4 | `u32` | Internal user ID as USERS.INF record number |
| 241 | 9 | bytes | Zero, flags/reserved |
| 250 | 4 | `u32` | Messages read |
| 254 | 4 | `u32` | Messages left |
| 258 | 1 | `u8` | `1`, alias support |
| 259 | 26 | CP437 | Alias |
| 285 | 1 | `u8` | `1`, address support |
| 286 | 51 | CP437 | Street line 1 |
| 337 | 51 | CP437 | Street line 2 |
| 388 | 26 | CP437 | City |
| 414 | 11 | CP437 | State |
| 425 | 11 | CP437 | Postal code |
| 436 | 16 | CP437 | Country |
| 452 | 1 | `u8` | `0`, password-history support |
| 453 | 1 | `u8` | `1`, verification support |
| 454 | 26 | CP437 | Verification answer |
| 480 | 1 | `u8` | `0`, statistics support |
| 481 | 1 | `u8` | `0`, notes support |
| 482 | 1 | `u8` | `0`, accounting support |
| 483 | 1 | `u8` | `0`, QWK support |

Flag byte 132 uses bit 1 for message clear, bit 3 to suppress the FSE prompt,
bit 4 for FSE enabled, bit 5 for scrolling message bodies, bit 6 for short file
descriptions, and bit 7 for the wide editor. Bits 0 and 2 are not set.

The header advertises the legacy 400-byte `PcbUserRecord::RECORD_SIZE`, but the
writer currently emits a 448-byte record. No conference pointers, conference
bitmaps, or third-party application records follow it. The historical source
text is retained in [user_sys.txt](user_sys.txt).