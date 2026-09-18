# Display Macros

IcyBoard display macros insert caller and board information into display text,
or perform actions such as changing colors, clearing the screen, and pausing
output. This reference covers the current implementation, including recognized
codes that do not yet produce a value.

## Contents

- [Display Macros](#display-macros)
  - [Contents](#contents)
  - [Syntax](#syntax)
  - [Field Width, Alignment, and Trimming](#field-width-alignment-and-trimming)
    - [The Optional T Flag](#the-optional-t-flag)
  - [Caller Information](#caller-information)
  - [Board, Conference, and Area](#board-conference-and-area)
  - [Dates and Session Time](#dates-and-session-time)
  - [Messages](#messages)
  - [Files, Limits, and Ratios](#files-limits-and-ratios)
  - [Transfer Statistics](#transfer-statistics)
  - [Accounting Credits](#accounting-credits)
  - [Screen and Output Control](#screen-and-output-control)
  - [Colors](#colors)
  - [Hyperlinks](#hyperlinks)
  - [Context and Environment](#context-and-environment)
  - [Recognized but Not Implemented](#recognized-but-not-implemented)
  - [Example Display](#example-display)
  - [Implementation Sources](#implementation-sources)

## Syntax

Most macros start and end with `@`:

```text
Welcome to @BOARDNAME@, @FIRST@!
Conference: @CONFNAME@    Time left: @TIMELEFT@
```

Names are case-insensitive: `@FIRST@` and `@first@` mean the same thing.
Use no spaces inside ordinary macro names or formatting suffixes. The hyperlink
macro is an exception: its label may contain spaces.

There are three useful distinctions:

| Kind | Example | Meaning |
| :--- | :--- | :--- |
| Value | `@CITY@` | Insert text from the current session. |
| Formatted value | `@CITY:20C@` | Insert text in a centered, 20-character field. |
| Action | `@CLS@`, `@POS:40@` | Change the screen or output state instead of inserting a value. |

Color codes use the special form `@Xhh`, with two hexadecimal digits and **no
closing `@`**. See [Colors](#colors).

Values depend on the active caller and context. For example, a transfer counter
is useful in transfer status text, and `@OPTEXT@` only has meaning when the
calling operation has supplied a value. User-dependent values may be empty
before login. Numbers generally have no thousands separators; date formats and
some labels depend on the board configuration and language.

## Field Width, Alignment, and Trimming

Append a colon and a decimal width to a value macro:

```text
@NAME:width@
@NAME:widthR@
@NAME:widthC@
```

Here `NAME` stands for a real macro such as `CITY` or `BOARDNAME`. The width is a
field length, not an absolute screen column. Use `@POS:n@` to move to a column.

| Suffix | Alignment | Whitespace removed before formatting | Padding |
| :--- | :--- | :--- | :--- |
| None | Left | Leading whitespace | None; no length limit. |
| `:20` | Left | Leading whitespace | Spaces on the right. |
| `:20R` | Right | Trailing whitespace | Spaces on the left. |
| `:20C` | Center | Leading and trailing whitespace | Spaces on both sides. |

Trimming and truncation are different operations:

1. **Trim** whitespace from the edge or edges listed above. Interior whitespace
   is left alone. Left alignment keeps existing trailing whitespace; right
   alignment keeps existing leading whitespace.
2. **Truncate** the result to the first `width` characters if a width was given.
   Even right-aligned fields keep the beginning of an overlong value, not its end.
3. **Pad** with spaces to fill the field. For centered text with an odd number
   of padding spaces, the extra space goes on the right.

For example, with `@CITY@` equal to `Hi`, the following fields produce these
results. The vertical bars are literal delimiters to make padding visible:

```text
|@CITY:5@|    -> |Hi   |
|@CITY:5R@|   -> |   Hi|
|@CITY:5C@|   -> | Hi  |
```

If the value is `Hello world`, all three five-character fields contain `Hello`.
A width of zero produces an empty value.

### The Optional T Flag

`T` is accepted between the width and alignment letter:

```text
@CITY:20T@
@CITY:20TR@
@CITY:20TC@
```

In the current implementation, **every explicit width already enables
truncation**. Thus `:20` and `:20T` behave identically, as do `:20R` and `:20TR`.
There is no suffix for a minimum width that allows longer values to overflow.
Omit the suffix to avoid truncation. Lowercase `t`, `r`, and `c` also work.

Widths count Unicode characters, not bytes or terminal cells. Combining marks,
wide characters, and embedded escape sequences can therefore make a field's
visible width differ from its specified length. Use plain text values for
predictable fixed-column layouts. The suffix does not format action macros or
hyperlinks, and `@POS:40@` means column 40, not a 40-character field.

## Caller Information

| Macro | Value |
| :--- | :--- |
| `@ALIAS@` | Caller's alias, falling back to the session's first name when no alias is set. |
| `@FIRST@` | Session first name with normalized capitalization. |
| `@FIRSTU@` | Session first name in uppercase. |
| `@USER@` | Uppercase display name; uses the alias when the session is using aliases, otherwise the real name. |
| `@REAL@` | Caller's real name, without forcing uppercase. |
| `@CITY@` | Caller's city/state field. |
| `@DATAPHONE@` | Business/data phone number as stored in the user record. |
| `@HOMEPHONE@` | Home/voice phone number as stored in the user record. |
| `@SECURITY@` | Security level stored in the current user record. |
| `@NUMTIMESON@` | Call count stored in the user record. |
| `@PROLTR@` | Caller's selected default transfer protocol code. |
| `@PRODESC@` | Description of that protocol, if it exists in the board's protocol list. |
| `@YESCHAR@` | Session's localized yes-response character. |
| `@NOCHAR@` | Session's localized no-response character. |

## Board, Conference, and Area

| Macro | Value |
| :--- | :--- |
| `@BOARDNAME@` | Configured BBS name. |
| `@SYSOPNAME@` | Configured sysop name, or the first user record's name when the real-name option is enabled. |
| `@VERSION@` | IcyBoard package version. |
| `@GFXMODE@` | Localized name of the current graphics mode: Off, ANSI, Graphics, Avatar, or RIP. |
| `@BPS@` | Session connection speed. |
| `@CARRIER@` | Currently the same value as `@BPS@`. |
| `@NODE@` | Internal node index, currently zero-based. |
| `@NUMCALLS@` | Board's total call count. |
| `@NUMCONF@` | Number of configured conferences. |
| `@CONFNAME@` | Current conference name. |
| `@CONFNUM@` | Current conference number; the main board is 0. |
| `@INCONF@` | Localized Main Board label, or the conference name and number followed by the localized Conference label. Includes a trailing space. |
| `@NUMBLT@` | Number of bulletins configured in the current conference. |
| `@NUMDIR@` | Number of file directories configured in the current conference. |
| `@NUMAREA@` | Number of message areas configured in the current conference. |
| `@DIRNAME@` | Current file directory name. |
| `@DIRNUM@` | Current file directory index, currently zero-based. |
| `@AREANAME@` | Current message area name. |
| `@AREANUM@` | Current message area number, starting at 1. |

The numbering conventions above are not uniform: do not assume `@NODE@` and
`@DIRNUM@` use the same base as `@AREANUM@`. The configured counts are not filtered
to only the entries the caller can access.

## Dates and Session Time

Date and clock-time macros use the server's local timezone. Dates use the
session's selected date format; clock times use 24-hour `HH:MM` format.

| Macro | Value |
| :--- | :--- |
| `@SYSDATE@` | Current date in the session's date format. |
| `@SYSTIME@` | Current time. |
| `@LOGDATE@` | Login date of this session. |
| `@LOGTIME@` | Login time of this session. |
| `@LASTDATEON@` | Last-on date stored in the user record. |
| `@LASTTIMEON@` | Last-on time stored in the user record. |
| `@EXPDATE@` | Subscription expiration date, or `00-00-00` when subscriptions are disabled or no date is set. |
| `@EXPDAYS@` | Subscription days remaining, evaluated against the session's login date; localized unlimited text when no expiration applies. |
| `@SYSOPIN@` | Configured start of sysop paging hours. |
| `@SYSOPOUT@` | Configured end of sysop paging hours. |
| `@TIMELIMIT@` | Session time limit in minutes; 0 means unlimited. |
| `@TIMELEFT@` | Session limit minus elapsed whole minutes; literal `UNLIMITED` when the limit is 0. Does not reserve flagged-file transfer time. |
| `@MINLEFT@` | Session limit minus elapsed whole minutes, clamped to at least 0, or localized unlimited text. |
| `@TIMEUSED@` | Whole minutes elapsed since login. |
| `@TOTALTIME@` | Current session minutes plus the user's stored minutes used today. |

`@TIMELEFT@` can become negative after the session limit is exceeded; it does not
use the zero clamp applied by `@MINLEFT@`. Unlike the historical PCBoard
description, the current `@MINLEFT@` calculation does not subtract estimated
flagged-file transfer time either.

## Messages

| Macro | Value |
| :--- | :--- |
| `@CURMSGNUM@` | Session's current message number. |
| `@HIGHMSGNUM@` | Session's high message number for the current area. |
| `@LOWMSGNUM@` | Session's low message number for the current area. |
| `@LMR@` | Session's last-message-read pointer. |
| `@MSGLEFT@` | Number of messages the caller has posted, from the user record. |
| `@MSGREAD@` | Number of messages the caller has read, from the user record. |

## Files, Limits, and Ratios

Byte quantities below are bytes unless explicitly described as kilobytes.
Kilobytes use 1024 bytes and discard any fractional remainder. Unlimited labels
are localized board text.

| Macro | Value |
| :--- | :--- |
| `@DLBYTES@` | Caller's total downloaded bytes. |
| `@DLFILES@` | Caller's total downloaded file count. |
| `@UPBYTES@` | Caller's total uploaded bytes. |
| `@UPFILES@` | Caller's total uploaded file count. |
| `@DAYBYTES@` | Downloaded bytes recorded for today. |
| `@BYTECREDIT@` | Session's byte credit. This is a quantity, not a ratio. |
| `@FILECREDIT@` | Session's file credit. |
| `@BYTELIMIT@` | Daily byte allowance, or unlimited. |
| `@BYTESLEFT@` | Bytes currently available under the transfer limits, accounting for the flagged batch, or unlimited. |
| `@KBLEFT@` | Available bytes expressed in kilobytes, or unlimited. |
| `@KBLIMIT@` | Smaller of the daily byte allowance and total byte limit, expressed in kilobytes; unlimited if neither limit applies. |
| `@MAXBYTES@` | Total download byte limit, or unlimited when no limit is set. |
| `@MAXFILES@` | Total download file limit, or unlimited when no limit is set. |
| `@BYTERATIO@` | Caller's actual download-to-upload byte ratio. |
| `@FILERATIO@` | Caller's actual download-to-upload file ratio. |
| `@RATIOBYTES@` | Configured byte-ratio limit, in the form `5.0:1`, or unlimited when disabled. |
| `@RATIOFILES@` | Configured file-ratio limit, in the form `5.0:1`, or unlimited when disabled. |
| `@FBYTES@` | Sum of the sizes of flagged files whose filesystem metadata can be read. |
| `@FFILES@` | Number of flagged files. |
| `@FNUM@` | Next flagged-file ordinal: the flagged count plus 1. |

Do not confuse the caller's actual ratios (`@BYTERATIO@`, `@FILERATIO@`) with
the configured limits (`@RATIOBYTES@`, `@RATIOFILES@`). Likewise, available bytes
and byte credit are separate values.

## Transfer Statistics

These values come from the session's transfer statistics, not the caller's
lifetime totals. Receive and send are from the **BBS's** perspective: received
files are uploads, and sent files are downloads. CPS means characters/bytes per
second.

| Macro | Value |
| :--- | :--- |
| `@RBYTES@` | Uploaded bytes in the transfer statistics. |
| `@RFILES@` | Uploaded file count in the transfer statistics. |
| `@RCPS@` | Upload transfer rate. |
| `@SBYTES@` | Downloaded bytes in the transfer statistics. |
| `@SFILES@` | Downloaded file count in the transfer statistics. |
| `@SCPS@` | Download transfer rate. |
| `@BICPS@` | Integer average of the upload and download rates, not their sum. |

## Accounting Credits

| Macro | Value |
| :--- | :--- |
| `@CREDSTART@` | Account's starting balance, not the balance at the start of this call. |
| `@CREDNOW@` | Opening balance of this call minus current balance: this call's net usage, not remaining credit. |
| `@CREDUSED@` | Account starting balance minus current balance: cumulative net usage. |
| `@CREDLEFT@` | Current enforced balance; unlimited in tracking/off modes. |

With accounting off, `@CREDSTART@` also shows unlimited, while `@CREDNOW@` and
`@CREDUSED@` show zero. Unlike most numeric macros, accounting values use comma
grouping. Credit display keeps up to six decimal places without trailing zeros;
money display uses US-dollar style with two decimal places, such as `$1,234.50`.

See [Accounting](accounting.md#enforcement-display-and-finalization) for setup
and the meaning of balances, charges, and credits.

## Screen and Output Control

These macros perform actions. Field-width suffixes do not pad their output.

| Macro | Action |
| :--- | :--- |
| `@CLS@` | Clear the screen. |
| `@CLREOL@` | Clear from the cursor to the end of the current line. |
| `@POS:n@` | Add spaces until column `n` on the current line, counting the first column as 1. Does nothing if already at or beyond that column. |
| `@BEEP@` | Request an audible bell; whether it is heard depends on terminal and sound settings. |
| `@DELAY:n@` | Wait for `n` units of 10 milliseconds. For example, `@DELAY:50@` waits 0.5 seconds. |
| `@MORE@` | Invoke the normal More prompt. |
| `@WAIT@` | Invoke the Press Enter prompt. |
| `@PAUSE@` | Invoke the Press Enter prompt while temporarily setting the automatic-continuation flag. See the timeout limitation below. |
| `@AUTOMORE@` | Set the automatic-continuation flag. See the timeout limitation below. |
| `@POFF@` | Disable automatic page pauses (nonstop output). |
| `@PON@` | Enable line counting and automatic page pauses again. |
| `@QOFF@` | Disable display-abort checking; More prompts become Press Enter prompts. |
| `@QON@` | Enable display-abort checking again. |
| `@WHO@` | Display the active-node listing. |
| `@HANGUP@` | Run the logoff/disconnection action. Use only where terminating the session is intended. |
| `@XOFF@` | Switch the session graphics mode to ANSI. See the limitation below. |
| `@XON@` | Switch to Graphics mode unless the board's non-graphics setting forbids it. See the limitation below. |

Pair temporary `@POFF@` and `@QOFF@` changes with `@PON@` and `@QON@` so later
output does not unexpectedly remain nonstop or non-interruptible.

**Automatic-continuation limitation:** the input code does not currently read
the flag set by `@AUTOMORE@` and `@PAUSE@`. They do not provide the historical
ten-second automatic continuation. In ordinary use, `@PAUSE@` waits for Enter
just like `@WAIT@`.

`@DELAY:n@` and `@POS:n@` accept unsigned 16-bit decimal arguments (0 through
65535). The delay unit above describes the current IcyBoard runtime; do not use
the historical PCBoard tenths-of-a-second rule to calculate these delays.

**XON/XOFF limitation:** the macro parser recognizes both names, but the normal
text scanner treats `@X` as the start of a color code before trying named macros.
Consequently, do not rely on `@XON@` or `@XOFF@` in ordinary display text. They
are listed here to distinguish their recognized runtime actions from usable
color syntax.

## Colors

Write `@Xbf`, where `b` is the background attribute digit and `f` is the
foreground digit. Both are hexadecimal; letter digits may be uppercase or
lowercase. There is no closing `@`:

```text
@X0FWhite on black@X07 light gray on black
@X1EYellow on blue@X07
```

| Digit | Color | Digit | Bright Color |
| :--- | :--- | :--- | :--- |
| `0` | Black | `8` | Dark gray |
| `1` | Blue | `9` | Light blue |
| `2` | Green | `A` | Light green |
| `3` | Cyan | `B` | Light cyan |
| `4` | Red | `C` | Light red |
| `5` | Magenta | `D` | Light magenta |
| `6` | Brown | `E` | Yellow |
| `7` | Light gray | `F` | White |

The foreground uses the full `0`-`F` range. In the traditional DOS attribute,
background digits `8`-`F` set the blink bit in addition to the base background
color; rendering depends on the terminal and graphics mode.

Two values receive special handling in ordinary display text:

- `@X00` saves the current color without switching to black-on-black.
- `@XFF` reapplies the current color. Although historically described as a
  restore code, the current text-output handler does not read the saved color
  here. Use an explicit code such as `@X07` when the following color must be
  predictable.

Colors affect following output until changed again. They are control codes, not
text fields, and do not take alignment or truncation suffixes.

## Hyperlinks

```text
@URL:IcyBoard documentation(https://example.org/docs)@
@URL:(https://example.org/docs)@
```

`@URL:label(uri)@` emits an OSC 8 terminal hyperlink displaying `label`. With an
empty label, the URI itself becomes the label. Clickability requires a terminal
that supports OSC 8 links.

- Spaces are allowed in the label, not in the URI.
- The URI must be nonempty and contain no whitespace or control characters.
- Balanced parentheses may appear in the label or URI.
- A literal `@` ends the macro, so avoid it inside the label or URI; use an
  appropriate encoded URI where needed.
- Field-width suffixes are not supported for hyperlinks.

The macro does not select a text color. Put color codes before and after it if
the link should stand out visually.

## Context and Environment

| Macro | Value |
| :--- | :--- |
| `@OPTEXT@` | Text supplied by the current operation, commonly a filename, name, or other prompt argument. Use it in text records whose operation supplies this value. |
| `@ENV=NAME@` | Value in the session's environment map, or empty text when it is not found. This is not a lookup of the server process's operating-system environment. |

Environment names use word characters and underscores, without spaces. The
entry name is case-sensitive even though the macro name `ENV` is not. The
ordinary field suffix also works, for example `@ENV=NAME:20R@`.

## Recognized but Not Implemented

The following names are recognized, but their runtime handlers currently return
empty text. An explicit width still turns that empty value into a field of
spaces. The second column identifies their intended legacy purpose, not an
implemented feature.

| Macro | Intended Purpose |
| :--- | :--- |
| `@EVENT@` | Time of the next scheduled event. |
| `@FREESPACE@` | Free space on the current conference's upload drive. |
| `@INAME@` | Reserved legacy name; no value is currently supplied. |
| `@LASTCALLERNODE@` | Name and city of the last caller to this node. |
| `@LASTCALLERSYSTEM@` | Name and city of the last caller across all nodes. |
| `@OFFHOURS@` | Hours during which lower-speed callers are allowed. |
| `@PWXDATE@` | Password expiration date. |
| `@PWXDAYS@` | Days until password expiration. |

## Example Display

```text
@CLS@@X0F@BOARDNAME:60C@@X07
Welcome, @FIRST@!

Caller: @USER:30@ City: @CITY:25@
Conference: @CONFNAME:24@ Area: @AREANAME:24@
Time left: @TIMELEFT:10R@  Download KB left: @KBLEFT:12R@

@X1EDownloads@X07@POS:30@Files: @DLFILES:8R@
@X1EUploads  @X07@POS:30@Files: @UPFILES:8R@
@WAIT@
```

The title is centered within 60 characters, not automatically within the
caller's terminal width. Choose field widths and cursor columns that fit your
intended display width.

## Implementation Sources

The [macro parser and formatter](../crates/icy_board_engine/src/icy_board/macro_parser.rs)
define the accepted names and field syntax. The
[text scanner and runtime handlers](../crates/icy_board_engine/src/icy_board/state/mod.rs)
determine what is displayed and which actions run. Descriptions above follow
those handlers where older PCBoard-oriented source comments differ.