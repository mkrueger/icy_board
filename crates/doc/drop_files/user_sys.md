# `USERS.SYS`

PCBoard 15.3 binary user exchange file, written beside `PCBOARD.SYS`. Strings
are fixed-width CP437, NUL-padded; numeric fields are little-endian. The header
is 40 bytes and the fixed user record is 1007 bytes, including unsupported
PSA slots. With no current user, the fixed record is zero-filled.

## Header

| Offset | Size | Type | Value or meaning |
| ---: | ---: | --- | --- |
| 0 | 2 | `u16` | `1530`, PCBoard version |
| 2 | 4 | `u32` | One-based user record number |
| 6 | 2 | `u16` | `1007`, fixed record size |
| 8 | 2 | `u16` | Conference count, including Main; at least 1 |
| 10 | 2 | `u16` | `7`, conference bitmap count |
| 12 | 2 | `u16` | Bytes per bitmap: max(5, ceil(conference count / 8)) |
| 14 | 15 | CP437 | TPA name; empty for normal door exports |
| 29 | 2 | `u16` | `0`, TPA version |
| 31 | 2 | `u16` | `0`, TPA fixed record size |
| 33 | 2 | `u16` | `0`, TPA conference record size |
| 35 | 4 | `u32` | `0`, TPA record offset |
| 39 | 1 | boolean | `0`, updated flag |

## Fixed Record

Offsets below are relative to the record, which begins at file offset 40.

| Offset | Size | Value or meaning |
| ---: | ---: | --- |
| 0 | 26 | User name |
| 26 | 25 | City/state |
| 51 | 13 | Door password |
| 64 | 14 | Business/data phone |
| 78 | 14 | Home/voice phone |
| 92 | 2 | Last-on PCBoard date |
| 94 | 6 | Last-on time, `HH:MM` plus NUL |
| 100 | 1 | Expert mode |
| 101 | 1 | Default protocol |
| 102 | 1 | Packed user flags |
| 103 | 2 | Zero, last directory scan date |
| 105 | 2 | Security level |
| 107 | 2 | Number of calls |
| 109 | 1 | Page length |
| 110 | 2 | Upload count |
| 112 | 2 | Download count |
| 114 | 4 | Downloaded bytes today |
| 118 | 31 | User comment |
| 149 | 31 | Sysop comment |
| 180 | 2 | Signed elapsed minutes, clamped to 0..32767 |
| 182 | 2 | Zero, registration expiration date |
| 184 | 2 | Zero, expired security level |
| 186 | 2 | Current conference |
| 188 | 4 | Total downloaded bytes, low 32 bits |
| 192 | 4 | Total uploaded bytes, low 32 bits |
| 196 | 1 | Zero, delete flag |
| 197 | 4 | One-based USERS.INF record number |
| 201 | 9 | Zero, flags/reserved |
| 210 | 4 | Messages read |
| 214 | 4 | Messages left |
| 218 | 27 | Alias support byte (1), then 26-byte alias |
| 245 | 167 | Address support byte (1), then streets (51 each), city (26), state (11), postal code (11), country (16) |
| 412 | 46 | Password-history support byte (0), then 45 zero bytes |
| 458 | 27 | Verification support byte (1), then 26-byte answer |
| 485 | 31 | Statistics support byte (0), then 30 zero bytes |
| 516 | 306 | Notes support byte (0), then 305 zero bytes |
| 822 | 138 | Accounting support byte (0), then 137 zero bytes |
| 960 | 31 | QWK support byte (0), then 30 zero bytes |
| 991 | 8 | Total downloaded bytes as IEEE 754 `f64` |
| 999 | 8 | Total uploaded bytes as IEEE 754 `f64` |

Packed flags use bit 1 for message clear, bit 3 to suppress the FSE prompt,
bit 4 for FSE enabled, bit 5 for scrolling, bit 6 for short file descriptions,
and bit 7 for the wide editor. Bits 0 and 2 are unset.

## Conference Data

The record is followed by one 32-bit last-read pointer per conference (from
message area 0), then seven zero-filled conference bitmaps. TPA payloads are
not exported. Unsupported PSA data must still occupy its fixed slot even
when the support byte is zero.

The importer validates the header, fixed record, and conference-tail length
before changing user fields. It accepts fixed records of at least 991 bytes
(15.2 layout) and leaves passwords and unsupported PSA data unchanged.
For 15.3-sized records, the floating-point totals preserve counters above
4 GiB; negative or non-finite totals are rejected before modifying user data.
The historical specification is retained in [user_sys.txt](user_sys.txt).