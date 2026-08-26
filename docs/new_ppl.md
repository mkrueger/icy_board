# New in PPL 3.50 and 4.x

Icy Board evolves PPL through an explicit *language version*. A source can put
`;$LANGVERSION 350` or `;$LANGVERSION 400` in its header; the same choice is
available as `pplc --lang-version`, `[compiler] language_version` in `ppl.toml`
and the `PPL_LANG_VERSION` environment default.

The *runtime version* is separate. It controls the PPE format written to disk.
Icy Board writes one runtime of its own, 4.00. Every lower number is a PCBoard
format, so 4.00 is what a PPE targets whenever it uses anything below.

| Feature | Language | Minimum runtime | What it adds |
| :--- | :---: | :---: | :--- |
| Scalar variable initializer | 350 | any compatible runtime | `INTEGER n = 1` |
| Array initializer | 350 | any compatible runtime | `INTEGER values = { 1, 2, 3 }` |
| Bracket indexing | 350 | any compatible runtime | `values[0]` without confusing indexing with a call |
| Compound assignment | 350 | any compatible runtime | `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=` |
| Post-test and infinite loops | 350 | any compatible runtime | `REPEAT ... UNTIL`, `LOOP ... ENDLOOP` |
| Optional parentheses | 350 | any compatible runtime | `IF condition THEN`, `WHILE condition ...` |
| Typed constants | 350 | any compatible runtime | `CONST`, erased to its value during compilation |
| Nominal integer enums | 350 | any compatible runtime | `ENUM ... ENDENUM`, scoped members such as `Color.Red` |
| Routine parameters | 350 | 400 | Pass a matching function or procedure as a checked callable value |
| Main-program block | 400 | 400 | Real `BEGIN ... END`; `EXIT` replaces the old terminating use of `END` |
| Board objects and member calls | 400 | 400 | `CONFERENCE`, `DIRECTORY`, `AREA`, `DOOR`, `PASSWORD`, `Board`, `Session` |
| Message-area identifiers | 400 | 400 | `MSGAREAID` and `AreaId(conf, area)` |
| Overloaded built-ins | 400 | 400 | Argument-count overloads such as `Len(array, dim)` |
| Web requests | 400 | 400 | String-returning function and file-writing statement forms |
| UTF-8 encoding and digest functions | 400 | 400 | `BASE64ENC`, `BASE64DEC` and `SHA256` |
| Extensible user contacts | 400 | 400 | `CONTACT` records on `Session.User` |
| User-defined records | 400 | 400 | `TYPE ... ENDTYPE`, nested fields, arrays of records and nominal type checking |
| Named record literals | 400 | 400 | `Point { X = 1, Y = 2 }` with checked and optional fields |
| Terminal multimedia | 400 | 400 | Sixel/JXL graphics, SyncTERM audio, mouse and physical key events |

Several compiler improvements are deliberately **not** tied to 3.50. The
compiler collects routine signatures before generating code, so `DECLARE` is
optional at every language version. `RETURN expression` is likewise accepted
when compiling classic source. In both cases the generated PPE uses ordinary
old instructions; declarations that disagree with implementations are errors.

## Language version 3.50

3.50 is mostly syntax that lowers to classic PPE instructions, so constants,
enums, loops, initializers, brackets and compound assignments can target an old
runtime. Passing a routine is the exception because only runtime 4.00 can mark
a routine reference as a value.

### Initializers and indexing

```PPL
INTEGER count = 1
INTEGER values = { 10, 20, 30 }

values[0] += count
```

The brace initializer declares the array and determines its size. Parenthesis
indexing remains valid for old source; brackets are recommended because they
cannot be mistaken for a function call.

### Loops

```PPL
REPEAT
	count += 1
UNTIL count >= 10

LOOP
	IF Finished() BREAK
ENDLOOP
```

`CONTINUE` and `BREAK` work in both forms. At 3.50, `LOOP` becomes a real
keyword and is no longer the old alias for `CONTINUE`; `QUIT` is no longer an
alias for `BREAK`.

### Constants and enums

```PPL
CONST INTEGER MaxAttempts = 3

ENUM Color
	Red
	Green = 5
	Blue
ENDENUM

Color selected = Color.Green
```

Constants are typed compile-time expressions. Enums are nominal integer types:
members are scoped below the enum name, and two different enum types cannot be
mixed merely because their stored numbers match. Both are erased before the PPE
is written, so a decompiler can recover the value but not the source name.

### Functions and procedures as parameters

```PPL
PROCEDURE Apply(PROCEDURE action(), FUNCTION check(INTEGER n) BOOLEAN)
	IF check(1) action()
ENDPROC
```

The compiler checks routine kind, parameter types and dimensions, `VAR` flags
and function return type. A routine parameter is callable and can be passed on
to another routine. The callable reference needs runtime 4.00.

## Language version 4.00

4.00 adds syntax and board APIs that do not exist on PCBoard. A runtime 4.00 PPE
therefore targets Icy Board rather than the original board.

### Blocks and program exit

```PPL
BEGIN
	IF !HasAccess() STOP
	PRINTLN "Welcome"
	EXIT
END
```

`BEGIN ... END` is a real block and marks the main body without `;$USEFUNCS`.
`END` only closes a block; `EXIT` performs the normal program termination that
`END` represented in old source. `STOP` remains the aborting form.

### User-defined records

```PPL
TYPE Point
	INTEGER X, Y
ENDTYPE

TYPE Line
	Point Start
	Point Finish
ENDTYPE

Point origin = Point { X = 0, Y = 0 }
Line axis
axis.Start = origin
axis.Finish.Y = 10
```

Records are nominal values. Fields can contain a previously declared record,
record variables can be arrays, member chains can be read or assigned, and
routine parameters and return values retain the exact record type. Equality is
defined between individual records of the same type; arithmetic and ordering
are not. Record fields cannot currently be arrays.

The PPE must store each record layout, so any use of `TYPE` requires runtime
4.00. Field and type names are not stored; a decompiler invents names for them.

### Board objects

Board objects are read-only snapshots rather than custom records. They expose
the configured conferences, message areas, file directories and doors without
making a PPE parse Icy Board's TOML files. `Board` and `Session` are the way in:
one for what the board is configured to be, one for the call in progress. The
detailed member table follows below.

### Terminal multimedia

Runtime 4.00 exposes terminal features through the `Terminal` object. The name
stands for the caller's one terminal, so parentheses and a temporary variable
are optional:

```PPL
PRINTLN Terminal.Info.Program
Terminal.BeginUpdate()
DrawScreen()
Terminal.EndUpdate()
```

The root groups the session by responsibility:

| Member | Purpose |
| :--- | :--- |
| `Info` | Cached identity, dimensions and capabilities |
| `Gfx` | Graphics-session state and backend selection |
| `Input` | Keyboard, physical-key and mouse events |
| `Margins` | Vertical and horizontal scrolling regions |
| `Palette` | The 16 DOS colours selected by `COLOR` |
| `Macros` | Terminal-resident DEC macro slots |
| `SetFont(font [, slot])`, `LoadFont(font, file)` | Terminal font selection and uploads |
| `BeginUpdate()`, `EndUpdate()` | Nestable synchronized output |

All operations that can fail update [`Error.Last()`](#errors). A function returning a
resource returns an invalid object on failure, so it is safe to inspect its
`Valid` property before continuing.

#### Graphics

`Terminal.Gfx.Init(backend[, fullscreen])` starts a graphics session. `backend`
is `GfxBackend.Auto`, `Sixel` or `Jxl`; `Auto` chooses the best capability in
`Terminal.Info`. Fullscreen defaults to `TRUE`.

```PPL
IF !Terminal.Gfx.Init(GfxBackend.Auto) EXIT
IF Terminal.Gfx.Backend = GfxBackend.None EXIT

SURFACE screen = Surface.New(640, 400)
screen.Clear(Rgb(20, 24, 32))
screen.FillRect(20, 20, 100, 40, Rgb(255, 80, 40, 192))
screen.Present()
Terminal.Gfx.Shutdown()
```

`Terminal.Gfx.Backend` reports the selected `GfxBackend`. `Pacing` is a writable
`BOOLEAN`; when true, presentation waits for a terminal acknowledgement before
sending another frame.

```PPL
Terminal.Gfx.Pacing = TRUE
```

`Rgb(red, green, blue[, alpha])` returns packed `0xRRGGBBAA`; components clamp
to 0 through 255 and alpha defaults to 255. It is a constant expression.

A surface is created by a static function on its type:

| Static function | Purpose |
| :--- | :--- |
| `Surface.New(width, height)` | Create a transparent surface |
| `Surface.Load(file)` | Decode PNG, JPEG XL or another supported image |

Surface members are:

| Member | Purpose |
| :--- | :--- |
| `Width`, `Height`, `Valid` | Read-only status properties |
| `Clear(color)` | Fill the whole surface |
| `SetPixel(x, y, color)`, `GetPixel(x, y)` | Write or read one pixel |
| `FillRect(x, y, w, h, color)`, `DrawRect(x, y, w, h, color)` | Fill or outline a rectangle in packed RGBA |
| `Blit(source, x, y)`, `BlitRect(source, sx, sy, w, h, x, y)` | Alpha-compose surfaces |
| `Present()`, `PresentAt(column, row)` | Present the surface |
| `PresentRect(sx, sy, w, h[, dx, dy[, dw, dh[, flip]]])` | Present a source rectangle |
| `Pin()`, `Unpin()` | Load or release an immutable JXL client buffer |
| `Free()` | Release the surface |

`PresentRect` scaling and `GFX_FLIP_X`/`GFX_FLIP_Y` are JPEG XL features. Sixel
reports `ErrCode.Unsupported` for them.

Surfaces are limited to 2048 by 2048 pixels, 256 simultaneous surfaces and 64
MiB of resident RGBA pixels. Source image files are limited to 32 MiB. Graphics
and sound together may add at most 256 MiB of persistent media per connection.

#### Audio

`Audio.Load(file)` probes the format, uploads it to the caller's SyncTERM cache
and takes an available channel. A cached file is not sent again.

```PPL
AUDIO music = Audio.Load("music.opus")
IF music.Valid THEN
    music.Volume = 50
    music.Play(TRUE)
ENDIF
```

| Member | Purpose |
| :--- | :--- |
| `Valid`, `Playing`, `Channel` | Read-only state |
| `Volume` | Playback volume in percent, writable |
| `Play([loop])`, `Stop()` | Start or stop playback |
| `Fade(percent, milliseconds)` | Change volume over time |
| `Free()` | Give the channel back |

Audio that ends produces `EventKind.Audio` with its channel in `Event.Channel`.
`Audio.StopAll()` flushes every channel the PPE started, and
`Terminal.Info.Audio` says whether the terminal can play anything at all.

#### Input and events

`Terminal.Input` is the caller's keyboard and mouse. Turning mouse or physical
key reporting on takes that input over from classic `INPUT`/`InKey`; `Release()`
stops those modes and gives it back. `Poll()` never blocks. `Wait(milliseconds)`
waits for an event, with zero meaning poll and a negative value meaning no
timeout.

```PPL
EVENT event
Terminal.Input.MouseOn(MouseMode.Pixels, MouseTracking.Drag)
Terminal.Input.KeyboardOn()

event = Terminal.Input.Wait(16)
IF event.Kind = EventKind.Key THEN
    IF event.Text = "q" EXIT
ELSEIF event.Kind = EventKind.KeyEdge THEN
    PRINTLN event.ScanCode, " ", event.Pressed
ELSEIF event.Kind = EventKind.Mouse THEN
    PRINTLN event.Action, " ", event.X, ",", event.Y
ENDIF

Terminal.Input.Release()
```

`MouseMode` is `Text` or `Pixels`. `MouseTracking` is `Buttons`, `Drag` or `All`.
`MouseButton` is `None`, `Left`, `Middle`, `Right`, `WheelUp`, `WheelDown`,
`WheelLeft` or `WheelRight`. `MouseAction` is `None`, `Press`, `Release`,
`Motion` or `Wheel`.

`Event.Kind` is an `EventKind`: `None`, `Key`, `KeyEdge`, `Mouse`, `Overflow` or
`Audio`. Its read-only fields are:

| Field | Meaning |
| :--- | :--- |
| `Kind` | Event category |
| `Code` | Unicode or named key code, zero for other kinds |
| `ScanCode` | Physical key code of a `KeyEdge`, zero for other kinds |
| `Text` | Translated key text; empty for other kinds |
| `Pressed`, `Repeated` | Key press/release state |
| `Action`, `Button` | Typed mouse action and button |
| `X`, `Y`, `Pixels`, `WheelX`, `WheelY` | Mouse position and wheel movement |
| `LeftDown`, `MiddleDown`, `RightDown` | Held mouse buttons |
| `Shift`, `Alt`, `Ctrl`, `Meta` | Active modifiers |
| `Channel` | Finished sound channel, otherwise `-1` |
| `Dropped` | Overflow count, otherwise zero |
| `Time` | Monotonic connection time in milliseconds |

ANSI navigation keys use `KEY_UP`, `KEY_HOME`, `KEY_PAGE_DOWN` and the other
`KEY_*` constants in `Code`. Printable input uses its Unicode value.
Consecutive unconsumed mouse motion reports are coalesced; press, release, wheel
and key events remain ordered.

### Terminal information

`Terminal.Info` is an immutable snapshot populated during connection setup. It
never sends a new query when read.

```PPL
PRINTLN Terminal.Info.Program, " ", Terminal.Info.Columns, "x", Terminal.Info.Rows
IF Terminal.Info.InlineGraphics PRINTLN "Inline JPEG XL available"
```

| Property | Meaning |
| :--- | :--- |
| `Program`, `DeviceAttrs`, `RipVersion`, `Utf8` | Terminal identity and encoding |
| `Columns`, `Rows` | Text dimensions |
| `CellWidth`, `CellHeight` | Cell dimensions in pixels |
| `ScreenWidth`, `ScreenHeight` | Screen dimensions in pixels, or zero |
| `CTermLevel` | Highest known CTerm-compatible protocol level |
| `Sixel`, `Jxl`, `InlineGraphics` | Graphics capabilities |
| `PixelMouse`, `PhysicalKeys`, `ClientBlit` | Input and client-side drawing capabilities |
| `Audio`, `SynchronizedOutput`, `TerminalMacros` | Output capabilities |

Capability booleans mean confirmed support. Unknown optional DEC modes are still
allowed to receive a standards-compliant request when an operation is tried.

### Synchronized output

`Terminal.BeginUpdate()` and `Terminal.EndUpdate()` wrap a redraw in DEC mode
2026. Calls may nest; only the outer pair emits terminal sequences. Ending an
inactive update reports `ErrCode.Invalid`. Cleanup ends an update left active by
`STOP`, `EXIT` or an execution error.

### Terminal output macros

`Terminal.Macros` manages 64 DEC macro slots numbered 0 through 63:

```PPL
Terminal.Macros.BeginRecord(0)
COLOR @X1F
PRINT "Reusable heading"
Terminal.Macros.EndRecord()
Terminal.Macros.Play(0)
Terminal.Macros.Delete(0)
```

`Recording` is read-only. `DeleteAll()` deletes every slot this PPE defined.
Definitions use hex encoding and may contain arbitrary
ANSI, OSC, DCS, UTF-8 and control bytes. A completed macro may be played while
another is recorded. Cleanup finishes an open recording, plays it so output is
not lost, and removes the PPE's definitions.

### Text margins

`Terminal.Margins` exposes DEC's independent vertical and horizontal regions.
Coordinates are 1-based and inclusive.

```PPL
Terminal.Margins.SetVertical(5, 18)
Terminal.Margins.SetHorizontal(18, 63)
PRINTLN Terminal.Margins.Top, "-", Terminal.Margins.Bottom
Terminal.Margins.ResetAll()
```

`Top`, `Bottom`, `Left`, `Right`, `HasVertical` and `HasHorizontal` report the
current virtual-screen state. `ResetVertical()` and `ResetHorizontal()` reset one
axis; `ResetAll()` restores both. PPE cleanup independently remembers whether the
PPE changed margins, so a caller is restored even if the virtual screen and
physical terminal disagree.

### Fonts

`Terminal.SetFont(font)` selects a font for every attribute class, which is what
changing *the* font means. `Terminal.SetFont(font, slot)` selects it for one
class, 0 through 3. `Terminal.LoadFont(font, file)` uploads PSF1, PSF2, YAFF or
size-recognised raw data into writable font numbers 43 through 255.

A terminal does not report which font a class is using, so there is nothing to
read back and these are calls rather than an object.

```PPL
Terminal.LoadFont(43, "topaz.psf")
IF Error.Last().OK Terminal.SetFont(43)
```

### Palette colors

`Terminal.Palette` changes the 16 DOS colours used by `COLOR`:

```PPL
Terminal.Palette.Set(1, Rgb(0, 64, 255))
Terminal.Palette.Reset(1)
Terminal.Palette.ResetAll()
```

Packed alpha is ignored and `Rgb()` clamps its components, so only an invalid
colour number reports `ErrCode.Invalid`; sessions without ANSI report
`ErrCode.Unavailable`.

### Errors

`Error.Last()` answers with an `ERROR` describing the last operation that could fail.
It reads the same whichever part of the board failed, so one piece of code can
handle a file, a font, a sound or a picture going wrong.

```PPL
Terminal.LoadFont(43, "topaz.psf")
IF (!Error.Last().OK) THEN
	PrintLn "Sorry: ", Error.Last().Message
ENDIF
```

| Member | Purpose |
| :--- | :--- |
| `OK` | `TRUE` while nothing has gone wrong |
| `Kind` | Which part failed, as an `ErrKind` |
| `Code` | What went wrong, as an `ErrCode` |
| `Message` | Informational English text, meant for a log rather than control flow |
| `Channel` | The file, dBase or sound channel, `-1` when the error has none |

`ErrKind` is `None`, `File`, `DBase`, `Stack`, `Gfx`, `Font`, `Audio` or
`Term`. `ErrCode` is `Ok`, `Unavailable`, `Invalid`, `Io`, `Format`, `Limit`,
`Unsupported` or `Stack`.

Use `Kind` and `Code` when a PPE has to make a decision. `Message` may include
paths and operating-system text, and its wording may change between releases.

An operation that works clears the error, so `Error.Last()` always answers for
the last thing that was tried rather than for the last thing that failed.
`Error.Clear()` forgets it as well. The value is a copy, so a PPE can keep one
while it carries on:

```PPL
ERROR failed = Error.Last()
```

`FERR` and `DERR` are unchanged, including that `FERR` clears itself when read
and that `FGET` or `FREAD` reaching the end of a file raises it. Reaching the end
is not an error, so it leaves `Error.Last().OK` true and never reaches an
`ON ERROR` handler.

### ON ERROR

`ON ERROR` says where a failed operation sends the program. It may be written as
one word, `ONERROR`. GOSUB and procedure handlers stay armed; GOTO is disarmed
before the jump because its cleanup path has no natural return boundary.

| Form | What it does |
| :--- | :--- |
| `ON ERROR GOTO label` | Jumps, and stays there - for cleaning up and ending |
| `ON ERROR GOSUB label` | Calls, and `RETURN` carries on after the failed statement |
| `ON ERROR Handler` | Calls a `PROCEDURE`, then carries on the same way |
| `ON ERROR OFF` | Back to checking `Error.Last()` by hand |

```PPL
DECLARE PROCEDURE Complain(ERROR e)

ON ERROR Complain
Terminal.LoadFont(43, "topaz.psf")
PrintLn "still running"

PROCEDURE Complain(ERROR e)
	PrintLn "Sorry: ", e.Message
ENDPROC
```

A handler procedure takes the error or takes nothing at all; a `VAR` parameter is
refused, because there is no variable behind an error to write back to. A
failure inside the handler is recorded but does not call the handler again.

`ON ERROR` handles operational failures reported by PPL APIs. A malformed PPE,
an invalid VM instruction, or a disconnected session is fatal to execution and
does not enter a handler.

The handler runs once the failing statement is over, so the statement always
finishes first. `ON ERROR` also catches running out of call stack, which lets a
runaway recursion apologise instead of disappearing.

> **Note:** icy_term does not read the slot argument yet, so a font it accepts
> applies regardless of which slot was named. SyncTERM uses the slot as written.

## Runtime 4.00

Runtime 4.00 is the PPE format Icy Board writes. Next to the PCBoard formats it
adds:

- a type table for `TYPE ... ENDTYPE` layouts
- a routine-reference marker for functions and procedures passed as values
- a record-literal opcode carrying type and field identifiers

A language 350 source needs runtime 400 when it passes routines; all its other
additions can lower to an older compatible runtime.

For the full rules, limits, diagnostics and compatibility breaks, see
[PPL](ppl.md#the-ppl-40-language). The sections below are the library and
declaration reference.

## `AreaId()` Function (4.00)

### Function
Returns the value for conference/message area. This is used for all message releated functions
to make them compatible with icy board message areas without breaking old code. Code that isn't
message area just works in icy board. But with icy board it's possible to specify a 
(non current) message area in all message related calls.

### Syntax
`AreaId(conf, area)`

`conf`      An integer expression stating the conference number of the message base.

`area`      An integer expression stating the message area of the message base.

### Returns
`MessageAreaID`   Combined Value of conference/message area

## Board objects (4.00)

`Board.Conferences[index]` returns a read-only `CONFERENCE` snapshot, and
`Session.Conference` the one the caller is in. An index no conference has returns
an empty conference object, so its properties can still be read.

| Conference member | Type | Description |
| :--- | :--- | :--- |
| `Name` | `STRING` | Conference name |
| `Number` | `INTEGER` | The number the conference was fetched under |
| `Valid` | `BOOLEAN` | Whether the requested conference exists |
| `IsPublic` | `BOOLEAN` | Whether the conference is configured as public |
| `Directories` | `DIRECTORIES` | The file directories of the conference |
| `Areas` | `AREAS` | The message areas of the conference |
| `Doors` | `DOORS` | The doors of the conference |
| `HasAccess()` | `BOOLEAN` | Whether the current caller can access the conference |

`DIRECTORY` and `AREA` provide `Name`, `Number`, `Valid` and `HasAccess()`.
`DOOR` provides `Name`, `Number`, `Valid`, `Description`, `Password` and
`HasAccess()`. Every board object
reports the number it was fetched under, so a listing can name what a caller has
to type. A door password has the
runtime-only `PASSWORD` type: it can be compared with a string, but converting
or printing it produces `******` rather than the secret.

### Collections

A collection answers `Count` and is read with an index. It is walked with
`FOREACH`, which is what it is usually for:

```PPL
CONFERENCE conf = Session.Conference
DOOR item

FOREACH item IN conf.Doors
	IF item.HasAccess() PRINTLN item.Name
ENDFOREACH
```

The index is there when a single entry is wanted, and `Count` when only the
number matters:

```PPL
PRINTLN conf.Areas.Count, " areas, the first is ", conf.Areas[0].Name
```

An index no entry has answers with an invalid object rather than failing, so
`Valid` is what to ask. A collection shares the list it stands for rather than
copying it, so reading `conf.Areas` once per loop step costs nothing.

A collection is an ordinary value, so it can be held in a variable. A walk reads
its source once per step, which makes the length of that source what a long loop
pays for. Naming it once is the cheapest form and the clearest to read:

```PPL
AREAS list = Session.Conference.Areas
AREA item

FOREACH item IN list
	PRINTLN item.Name
ENDFOREACH
```

## Board and session (4.00)

`Board` and `Session` are the two other objects that stand for themselves, so
they need no parentheses either. They split what the board *is* from what this
one call *is doing*.

`Board` is a snapshot of the configuration and its conferences. It is taken the
first time a PPE reads `Board` and stands for the rest of the run, so touching it
inside a loop is not paid for again:

| Member | Type | Description |
| :--- | :--- | :--- |
| `Name` | `STRING` | Board name |
| `Location` | `STRING` | Where the board says it is |
| `Operator` | `STRING` | Operator named for `EMSI` |
| `SysopName` | `STRING` | The sysop's display name |
| `NodeCount` | `INTEGER` | Number of configured nodes |
| `Conferences` | `CONFERENCES` | The conferences of the board |

`Conferences` is what lets a PPE walk the board without `HIGHCONFNUM()`. An index
no conference has answers with an object whose `Valid` property is false.
Listing conferences says nothing about who may enter
one, so check `HasAccess()` before showing a name:

```PPL
CONFERENCE conf

FOREACH conf IN Board.Conferences
	IF conf.HasAccess() PRINTLN conf.Number, " ", conf.Name
ENDFOREACH
```

`Session` is the call in progress. Unlike `Board` it is read live, so a value
kept in a variable still answers with what the session became:

| Member | Type | Description |
| :--- | :--- | :--- |
| `Conference` | `CONFERENCE` | The conference the caller is in |
| `User` | `USER` | The caller's own record |
| `Area`, `Directory` | `AREA`, `DIRECTORY` | The message area and file directory in use |
| `UserName`, `AliasName` | `STRING` | Who is calling |
| `SecurityLevel` | `INTEGER` | The caller's current security level |
| `Node` | `INTEGER` | Node number, as `PCBNODE()` reports it |
| `MinutesLeft` | `INTEGER` | Minutes left in this call |
| `PageLength` | `INTEGER` | Lines before a `MORE` prompt |
| `Language` | `STRING` | Selected language |
| `IsLocal`, `IsSysop` | `BOOLEAN` | How the caller got on |

```PPL
PRINTLN "Node ", Session.Node, ", ", Session.MinutesLeft, " minutes left"
PRINTLN "In ", Session.Conference.Name, " on ", Board.Name
```

Where a conference, area or directory sits is asked of the thing itself:
`Session.Conference.Number`, `Session.Area.Number`, `Session.Directory.Number`.

### The session and the user are not the same thing

`Session.SecurityLevel`, `Session.PageLength`, `Session.Language`,
`Session.UserName` and `Session.AliasName` look like they repeat what
`Session.User` holds, and mostly they agree — but they are the call's values,
not the record's. `PCBoard` splits them that way and the split is kept:

- `Session.SecurityLevel` is what the caller may do **right now**, which a
  conference can raise for the duration. `Session.User.SecurityLevel` is what
  the user record says.
- `Session.PageLength` and `Session.Language` are what this call is using, and
  may have been changed for it alone. The `Session.User` ones are what the caller
  will get on the next call.

Read the session when asking what is in force, and the user when asking what is
stored. Writing goes to `Session.User`; the session's own values are read-only.

Both are read-only. The classic `CURCONF()`, `PCBNODE()`, `MINLEFT()` and the
`U_*` variables keep working unchanged.

## The caller (4.00)

`Session.User` is the caller's own record, read live and written through. It
gathers what the `U_*` variables report, so a 4.00 PPE does not have to remember
which predefined name holds which detail, nor bracket its work in
`GETUSER`/`PUTUSER`:

```PPL
PRINTLN Session.User.Name, " from ", Session.User.City
PRINTLN Session.User.SecurityLevel, " until ", Session.User.ExpirationDate

Session.User.City = "Berlin"
```

| Group | Members | Writable |
| :--- | :--- | :--- |
| Identity | `Name`, `Alias`, `VerifyAnswer` | all but `Name` |
| Address | `Street1`, `Street2`, `City`, `State`, `Zip`, `Country` | yes |
| Reaching them | `BusinessPhone`, `HomePhone`, `Email`, `Web`, `Gender`, `BirthDate` | yes |
| Sysop text | `Comment`, `SysopComment`, `Notes` | yes |
| Preferences | `ExpertMode`, `EditorMode`, `ClearScreen`, `ScrollMessageBody`, `ShortDescriptions`, `LongHeader`, `WideEditor`, `PageLength`, `Protocol` | yes |
| Preferences the session owns | `UseGraphics`, `UseAlias`, `Language`, `DateFormat` | no |
| Security | `SecurityLevel`, `ExpiredSecurityLevel`, `ExpirationDate`, `PasswordExpires`, `SetPassword(text)` | yes |
| Statistics | `TimesOn`, `FirstDateOn`, `LastDateOn`, `LastDirRead`, `MessagesRead`, `MessagesLeft`, `Uploads`, `Downloads`, `UploadBytes`, `DownloadBytes`, `DownloadBytesToday`, `MinutesToday` | no |
| Contacts | `Contacts` | yes |

Whatever `PUTUSER` could write is writable here and lands at once, so the object
replaces the old round trip rather than sitting beside it. The caller's `Name`
identifies them and the board's own accounting is the board's to keep, so both
stay read-only; writing one is a compile error. Nobody logged in reads as an
empty user rather than failing, so a member is always safe to read.

`EditorMode` is one `EDITORMODE` value — `Yes`, `No` or `Ask` — rather than the
two overlapping flags `PCBoard` kept. `Notes.Set()` takes an index from 0 to 4 and
answers whether the note existed. `SetPassword()` hashes the text the way the
board is configured to, so the plain text is never stored; an empty password is
refused. Both answer `FALSE` rather than failing.

### Notes and contacts

Both are collections: they answer `Count`, are read with an index and are walked
with `FOREACH`. `Notes` holds the five sysop notes as `STRING`s and is written
through the index:

```PPL
Session.User.Notes[0] = "Called about the upload"
```

An index no note has is refused and leaves the rest alone. A compound assignment
such as `Notes[0] = Notes[0] + "..."` has to be written out; `+=` on an index is
not read as an assignment.

A contact is a built-in `CONTACT` record with two `STRING` fields, `Service` and
`Account`. Service names are open strings, so a PPE can store a new service
without a language or user-schema change.

```PPL
CONTACT entry

FOREACH entry IN Session.User.Contacts
	PRINTLN entry.Service, ": ", entry.Account
ENDFOREACH

Session.User.Contacts.Put("matrix", "@sysop:example.org")
```

`Contacts.Put()` replaces the account when the service is already there and adds
it otherwise, so there can never be two entries meaning the same service. It is
`Put` rather than `Set` because it is keyed by service, not by position. Service
names are trimmed and compared without regard to case; a blank service or account
is refused and answers `FALSE`. `Contacts.Delete()` answers whether it removed
anything. An index no contact has answers with an empty `CONTACT`.

Unlike the board's collections these can change while a PPE runs, but how many
steps a walk takes is settled when it starts, so adding or removing inside one
does not change its length.

Both write straight through to the caller, so no `GETUSER`/`PUTUSER` round trip
is needed. `U_EMAIL` and `U_WEB` remain separate predefined variables for
PCBoard 3.40 compatibility and are not duplicated here.

## Encoding and digest functions (4.00)

`BASE64ENC(value)` encodes the UTF-8 bytes of a string and returns base64 text.
`BASE64DEC(value)` ignores characters outside the base64 alphabet, decodes the
remaining text, and interprets the result as UTF-8. Invalid UTF-8 bytes become
the Unicode replacement character.

`SHA256(value)` returns the 64-character lowercase hexadecimal SHA-256 digest
of the value's UTF-8 bytes. It is intended for content integrity and identity,
not password storage.

```PPL
PRINTLN BASE64ENC("Grüße")
PRINTLN BASE64DEC("R3LDvMOfZQ==")
PRINTLN SHA256("abc")
```

## `WebRequest()`  Function (4.00)

### Function
Gets data from a web server and returns it as a string.

### Syntax
`WebRequest(url)`

`url` An string expression stating the url to get data from.
        
### Returns
`STRING`   Returns the web request value as STRING.

### Remarks
A request that fails - a bad url, a host that is not there, an error from the
server - is logged and answers an empty string rather than stopping the PPE.
A request gives up after 30 seconds, so a host that never answers cannot hold
the caller's node.

## `WEBREQUEST()` Statement (4.00)

### Function
Gets data from a web server and stores it as a file.

### Syntax
`WEBREQUEST url, file`

`url`  An string expression stating the url to get data from.

`file` An string expression stating the file to save the data to.

### Remarks
The file is resolved against the board like every other file a PPE writes, so a
DOS style path works the way it does everywhere else. A request that fails is
logged, writes no file and lets the PPE carry on; it gives up after 30 seconds.

## `Len()`  Function (4.00)

### Function
With this overload of the len function it's possible to get the length of an array dimension.
Note: With 400 `Len(arr, 0)` behaves like `Len(arr)`. PPL array declarations
use upper bounds, so `INTEGER values(10)` makes both calls return `10`.
For multidimensional arrays, `dim` is zero-based.

### Syntax
`Len(array, dim)`

`array`    An array expresison to get the length of

`dim`      The dimension to get the length of

## `FOREACH ... ENDFOREACH` Statement (4.00)

### Function

Walks every element of an array, whatever its rank. `FOR` needs one loop per
dimension and needs to know how many there are; `FOREACH` needs neither.

```PPL
STRING names(10)
STRING name

FOREACH name IN names
    PRINTLN name
ENDFOREACH
```

A two- or three-dimensional array walks exactly the same way, row-major, with the
last index moving fastest:

```PPL
INTEGER grid(9, 9)
INTEGER cell

FOREACH cell IN grid
    total = total + cell
ENDFOREACH
```

### Syntax

`FOREACH variable IN array`, the body, then `ENDFOREACH` or `NEXT`.

The loop variable is declared like any other and has to be able to hold an
element. It is a **copy**: assigning to it inside the loop changes the copy, not
the array. Write through the array itself when a walk should change it.

`BREAK` and `CONTINUE` work the way they do in every other loop. PPL arrays are
declared with upper bounds and index from zero, so `STRING names(10)` walks
eleven elements.

How many elements there are is settled when the loop starts. Resizing an array
inside its own walk therefore changes neither how many steps it takes nor where
it stops, and the source is read once per step rather than twice. The board's
collections cannot change while a PPE runs at all.

`IN` is not a reserved word. Like `TO` and `STEP` it is only read as part of the
statement, so it stays available as a variable name.

`FOREACH` is the only flat walk there is. Indexing is bound to the rank: `a[i]`
reads a vector, a matrix wants `a[i, j]` and one index into it is a compile
error. That check is worth keeping, so the flat step `FOREACH` is built from
stays the compiler's own and is not a function a PPE can call.

## Array members (4.00)

### Function

Everything built in that takes an array first may also be written as a member of
that array, whichever reads better at the call site. The two spellings are the
same call, so neither can drift from the other.

| Member | The same as |
| :--- | :--- |
| `a.Len()` | `Len(a, 0)` |
| `a.Len(dim)` | `Len(a, dim)` |
| `a.Redim(n)` | `REDIM a, n` |
| `a.Redim(n1, n2)` | `REDIM a, n1, n2` |
| `a.Redim(n1, n2, n3)` | `REDIM a, n1, n2, n3` |

```PPL
INTEGER values(10)

PRINTLN values.Len(), " slots"
values.Redim(20)
```

Only a declared array has these members. An array's type is its element's, so it
is the declaration that says it has them; asking a plain value for `.Len()` is a
compile error. `Redim` is a statement rather than a function, so it stands on a
line of its own the way `REDIM` does.

## `CONST` Declaration (3.50)

### Function
Gives a name to a value the compiler works out.

### Syntax
`CONST <type> <name> = <value>`

`type`   The type the value is converted to, written like any other declaration

`name`   The name the value is used under

`value`  An expression of literals and constants declared before it, or an enum
member when the declared type is that enum

### Remarks
A constant stands where a variable would, so it may open a program or a routine,
and one declared in a routine belongs to it. The value takes the place of the
name while compiling, so a constant costs nothing at runtime: the PPE is the one
the value written out by hand would produce, whatever runtime it targets. A
decompiled PPE therefore shows the value, never the name.

Writing to a constant is an error. A constant, parameter and variable may not
share a name in the same scope, but a local declaration may shadow a global
constant or variable. A constant cannot be passed to a `VAR` parameter - there
is no variable to write back to.

`;$DEFINE` is the other way to name a value: it substitutes text before the
language is read, carries no type and works at any version. `CONST` is typed and
belongs to 3.50.

## `ENUM ... ENDENUM` Declaration (3.50)

### Function
Defines a compile-time integer type and its named values.

### Syntax
```PPL
ENUM Color
	Red
	Green = 5
	Blue
ENDENUM

Color favorite = Color.Green
```

### Remarks
The first implicit value is zero; every later implicit value follows the member
before it. An explicit value must be an integer constant expression. Members
live under the enum name, so `Color.Green` is valid and `Green` alone is not.

Enums are nominal: different enums and plain integers cannot be assigned to or
compared with each other. Equality and inequality are supported; arithmetic and
bitflag behavior are not. A `FOR` may count over an enum, since the loop writes
its own comparison and step, and its start and end value must be of the enum's
type. Enum variables, arrays, routine parameters and return
values, and record fields are stored as `INTEGER` in the PPE. The type and names
therefore cost nothing at runtime and cannot be recovered by the decompiler.

## `BEGIN ... END` Block (4.00)

### Function
Groups statements into a block. At top level the block is the main program.

### Syntax
```PPL
BEGIN
    <statements>
END
```

### Remarks
Before 400 `BEGIN` was a pseudo label for `;$USEFUNCS` and `END` was the
statement that stops a program. From 400 on the pair is a real block: a `BEGIN`
without a matching `END` is an error, and a program that has a block may not
have statements outside it - only declarations and comments. The block says
where the body is, so it may stand after the routines and `;$USEFUNCS` is no
longer needed. Inside a routine a block only groups statements.

`END` closes a block and nothing else. Use `EXIT` to end a program and `STOP` to
abort one.

## `EXIT` Statement (4.00)

### Function
Ends the program normally.

### Syntax
`EXIT`

### Remarks
`EXIT` is what `END` meant up to 3.50 and compiles to the same instruction, so
the executable is unchanged. The compiler appends the terminating instruction by
itself, which makes a trailing `EXIT` optional.

`STOP` ends the program too, but as an abort: the channel 0 output a script
questionnaire collects is dropped instead of being appended to the answer file.
The decompiler prints the terminating instruction as `EXIT`.
