# New in PPL 3.50 and 4.x

Icy Board evolves PPL through an explicit *language version*. A source can put
`;$LANGVERSION 350` or `;$LANGVERSION 400` in its header; the same choice is
available as `pplc --lang-version`, `[compiler] language_version` in `ppl.toml`
and the `PPL_LANG_VERSION` environment default.

The *runtime version* is separate. It controls the PPE format written to disk.
There is no language version 401: runtime 4.01 adds storage needed by some 4.00
language features.

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
| Routine parameters | 350 | 401 | Pass a matching function or procedure as a checked callable value |
| Main-program block | 400 | 400 | Real `BEGIN ... END`; `EXIT` replaces the old terminating use of `END` |
| Board objects and member calls | 400 | 400 | `CONFERENCE`, `DIRECTORY`, `AREA`, `DOOR`, `PASSWORD`, `ConfInfo()` |
| Message-area identifiers | 400 | 400 | `MSGAREAID` and `AreaId(conf, area)` |
| Overloaded built-ins | 400 | 400 | Argument-count overloads such as `ConfInfo(conf)` and `Len(array, dim)` |
| Web requests | 400 | 400 | String-returning function and file-writing statement forms |
| UTF-8 encoding and digest functions | 400 | 400 | `BASE64ENC`, `BASE64DEC` and `SHA256` |
| Extensible user contacts | 400 | 402 | Mutable `CONTACT` records in `U_CONTACT` |
| User-defined records | 400 | 401 | `TYPE ... ENDTYPE`, nested fields, arrays of records and nominal type checking |
| Named record literals | 400 | 401 | `Point { X = 1, Y = 2 }` with checked and optional fields |
| Terminal multimedia | 400 | 400 | Sixel/JXL graphics, SyncTERM audio, mouse and physical key events |

Several compiler improvements are deliberately **not** tied to 3.50. The
compiler collects routine signatures before generating code, so `DECLARE` is
optional at every language version. `RETURN expression` is likewise accepted
when compiling classic source. In both cases the generated PPE uses ordinary
old instructions; declarations that disagree with implementations are errors.

## Language version 3.50

3.50 is mostly syntax that lowers to classic PPE instructions, so constants,
enums, loops, initializers, brackets and compound assignments can target an old
runtime. Passing a routine is the exception because only runtime 4.01 can mark
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
to another routine. The callable reference needs runtime 4.01.

## Language version 4.00

4.00 adds syntax and board APIs that do not exist on PCBoard. A runtime 4.00 or
4.01 PPE therefore targets Icy Board rather than the original board.

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
4.01. Field and type names are not stored; a decompiler invents names for them.

### Board objects

Board objects are read-only snapshots rather than custom records. They expose
the configured conferences, message areas, file directories and doors without
making a PPE parse Icy Board's TOML files. The detailed member table follows
below.

### Terminal multimedia

Graphics use an in-memory RGBA surface model and select Sixel or SyncTERM JPEG
XL at presentation time. `GFX_AUTO` probes the terminal and returns `GFX_NONE`
when neither backend is advertised; `GFX_SIXEL` explicitly selects Sixel for
terminals that cannot report capabilities. These APIs use PPL language 4.00
syntax but require PPE runtime 4.02.

```PPL
GfxInit GFX_AUTO
IF GfxBackend() = GFX_NONE EXIT

SURFACE screen = NewSurface(640, 400)
screen.Clear(Rgb(20, 24, 32))
screen.FillRect(20, 20, 100, 40, Rgb(255, 80, 40, 192))
screen.Present()
GfxShutdown
```

`Rgb(red, green, blue[, alpha])` returns packed `0xRRGGBBAA`; components clamp
to 0 through 255 and alpha defaults to 255. It is a constant expression, so it
can be used in `CONST` declarations. Large hexadecimal literals such as
`0FF0000FFh` are also `UNSIGNED` and retain hexadecimal formatting.

A surface is a `SURFACE` object created by one of two functions:

| Function | Purpose |
| :--- | :--- |
| `NewSurface(width, height)` | Create a transparent surface |
| `LoadSurface(file)` | Load PNG, JPEG XL or another supported image |

Both return an invalid surface on failure, so check `GfxError()` or the `Valid`
property. Everything else is a member of the surface itself:

| Member | Purpose |
| :--- | :--- |
| `Width`, `Height`, `Valid` | Read-only status properties |
| `Clear(color)` | Fill the whole surface |
| `SetPixel(x, y, color)` | Set a single pixel |
| `GetPixel(x, y)` | Read a single pixel back; out of bounds answers `0` |
| `FillRect(x, y, w, h, color)`, `Rect(x, y, w, h, color)` | Draw packed RGBA colors |
| `Blit(source, x, y)`, `BlitRect(source, sx, sy, w, h, x, y)` | Alpha-compose surfaces in memory |
| `Present()` | Present the complete surface |
| `PresentRect(sx, sy, w, h[, dx, dy[, dw, dh[, flip]]])` | Present a source rectangle at a pixel destination |
| `PresentAt(column, row)` | Present at a text-cell position |
| `Pin()`, `Unpin()` | Load/unload an immutable JXL surface in a SyncTERM client buffer |
| `Free()` | Release the surface |

The drawing and presentation members return a `BOOLEAN` that is `FALSE` when the
operation failed, and they can also be used as bare statements. The remaining
global statements control the session rather than a surface:

| API | Purpose |
| :--- | :--- |
| `GfxSetPacing frames` | Enable DSR-acknowledged presentation when positive |
| `GfxShutdown` | Release resources and restore terminal modes |

`GfxError` reports the status of the last operation. `GfxCaps` is a bitmask of
`GFX_CAP_*` constants. `GfxCellWidth`, `GfxCellHeight`, `GfxScreenWidth` and
`GfxScreenHeight` report probed pixel geometry. `Pin` uses at most the two
client pixel buffers; drawing onto a pinned surface automatically invalidates
its client copy.

Surfaces are limited to 2048 by 2048 pixels, 256 simultaneous surfaces and 64 MiB
of resident RGBA pixels. Source image files are limited to 32 MiB and raster
decoders receive the same dimension and allocation limits before decoding.
Graphics and sound together may add at most 256 MiB of uniquely named persistent
media during one connection. Changing frames that overwrite a surface cache
entry do not consume that budget. `GfxError` returns the named `GFX_ERR_*`
constants; every graphics operation updates it.

`PresentRect` can hand the scaling and mirroring to the terminal: `dw` and
`dh` give the destination size and `flip` combines `GFX_FLIP_X` and
`GFX_FLIP_Y`. The frame still travels at its source size, so drawing a small
surface enlarged costs the bandwidth of the small one. Both are options of the
image APC, so they need the JPEG XL backend and report
`GFX_ERR_UNSUPPORTED` on Sixel.

```PPL
; A 160x100 surface fills a 640x400 area without sending 640x400 pixels.
screen.PresentRect(0, 0, 160, 100, 0, 0, 640, 400)
```

Sound is an `AUDIO` object. `LoadAudio(file)` probes the container/codec, uploads
the file to the caller's per-board SyncTERM cache and takes an available channel
for it. A file the caller already holds is not sent again, and the object gives
its channel back when it is freed.

```PPL
AUDIO music = LoadAudio("music.opus")
IF music.Valid THEN
	music.SetVolume(50)
	music.Play(TRUE)
ENDIF
```

| Member | Purpose |
| :--- | :--- |
| `Valid`, `Playing`, `Volume`, `Channel`, `Error` | Read-only status properties |
| `Play([loop])` | Start the sound, looping when told to |
| `Stop()` | Stop it and free the channel's mixer slot |
| `SetVolume(percent)` | Set the volume, 0 through 100 |
| `Fade(percent, milliseconds)` | Ride the volume to `percent` |
| `Free()` | Give the channel back |

`Error` is zero for a loaded object and describes why an invalid object could not
be loaded. A sound that ends reports itself as an `EVENT_SOUND` event naming its
channel, so a program waits for it rather than polling.

`EventPoll` and `EventWait(milliseconds)` provide one ordered input stream.
Both return an immutable `EVENT` object. Its `Kind` is one of `EVENT_NONE`,
`EVENT_KEY`, `EVENT_KEY_EDGE`, `EVENT_MOUSE` or `EVENT_OVERFLOW`; `EventWait`
blocks only until an event arrives or its timeout expires, while `EventPoll` never blocks.
`EventWait(0)` is equivalent to a poll; a negative timeout waits indefinitely
while still servicing the session.

```PPL
EVENT event

MouseOn MOUSE_PIXELS, MOUSE_TRACK_DRAG
KeyEvents KEY_EVENTS_ON

event = EventWait(16)
IF event.Kind = EVENT_KEY THEN
	IF event.Text = "q" EXIT
ELSEIF event.Kind = EVENT_KEY_EDGE THEN
	PRINTLN event.Code, " ", event.Pressed
ELSEIF event.Kind = EVENT_MOUSE THEN
	PRINTLN event.Code, " ", event.X, ",", event.Y
ENDIF
```

The read-only fields describe that value:

| Field | Meaning |
| :--- | :--- |
| `Kind` | One of the `EVENT_*` constants |
| `Code` | Unicode value for `EVENT_KEY`, physical code for `EVENT_KEY_EDGE`, or `MOUSE_*` event kind for `EVENT_MOUSE` |
| `Text` | The translated character for `EVENT_KEY`; empty for other event kinds |
| `Pressed` | `TRUE` for a translated character, or the press/release state of a physical key edge |
| `Repeated` | For physical edges, whether another press arrived before release; translated keys report `FALSE` |
| `X`, `Y` | Mouse coordinates, in cells or pixels according to `MouseOn` |
| `Button` | Mouse button or wheel direction |
| `Buttons` | Held-button mask using `MOUSE_BUTTON_LEFT`, `MOUSE_BUTTON_MIDDLE` and `MOUSE_BUTTON_RIGHT` |
| `WheelX`, `WheelY` | Wheel movement; positive `WheelY` is up and negative is down |
| `Modifiers` | Combined `EVENT_SHIFT`, `EVENT_ALT`, `EVENT_CTRL` and `EVENT_META` bits; the first three alias the mouse modifier values |
| `Pixels` | Whether DEC 1016 pixel-coordinate mode was accepted; available even on `EVENT_NONE` |
| `Time` | Monotonic milliseconds since the connection's event input state was created, sampled when the event is selected; an `EVENT_NONE` carries it too, so a poll that came back empty still reads the clock |

Ordinary translated characters are always available. `EVENT_KEY_EDGE` requires
`KeyEvents KEY_EVENTS_ON`, or `KEY_EVENTS_SUPPRESS` to suppress translated
characters. `KEY_EVENTS_OFF` restores ordinary input. `EVENT_MOUSE` requires
`MouseOn`; its optional second argument selects button-only, drag or all-motion
tracking. `EVENT_SOUND` needs nothing turned on: a sound that ends reports its
channel in `Code`.

ANSI cursor and navigation sequences are returned as one `EVENT_KEY`, with
`Text` such as `"UP"`, `"HOME"` or `"PGDN"` and a named `KEY_*` value in
`Code`. Printable input uses its Unicode value. `KEY_ESCAPE`, `KEY_ENTER`,
`KEY_TAB`, `KEY_BACKSPACE` and `KEY_DELETE` name the common control values.
Parameterized xterm sequences such as Ctrl+Up also populate `Modifiers`.
Physical CTerm edges remain `EVENT_KEY_EDGE`, because their scan codes and
press/release semantics are distinct from translated keys.

Consecutive unconsumed mouse-motion reports are coalesced to the newest
position. Press, release, wheel and key events remain ordered. If a bounded
physical-key or mouse queue still overflows, the next event is
`EVENT_OVERFLOW` and its `Code` is the number of discarded events.
Horizontal wheel reports use `MOUSE_WHEEL_LEFT` and `MOUSE_WHEEL_RIGHT` in
`Button` and a signed delta in `WheelX`.

A frame-paced program needs no separate pacing statement: poll first so queued
input is handled at once, then let `EventWait` be the one place the program
blocks, with the time left in the frame as its timeout.

```PPL
UNSIGNED nextFrame
EVENT e = EventPoll()
nextFrame = e.Time + FRAME_MS

WHILE running DO
	e = EventPoll()
	IF e.Kind = EVENT_NONE & e.Time < nextFrame THEN
		e = EventWait(nextFrame - e.Time)
	ENDIF
	IF e.Kind <> EVENT_NONE Handle(e)
	IF e.Time >= nextFrame THEN
		nextFrame = e.Time + FRAME_MS
		Advance()
	ENDIF
ENDWHILE
```

Input is answered as soon as it arrives rather than once per frame, and a fixed
pause that should not end early is still `DELAY`.

The outermost PPE always disables mouse/key modes, stops sound it started,
releases graphics and restores cursor, wrapping, palette and Sixel scrolling
state, including after `STOP` or an execution error.

### Text margins

Text margins expose ANSI's independent vertical and horizontal scrolling
regions. Coordinates are 1-based and inclusive, like `ANSIPOS`.

```PPL
SetVMargins 5, 18
SetHMargins 18, 63

TERMSTATE state = TermState()
IF state.VerticalMargins PRINTLN state.MarginTop, "-", state.MarginBottom
IF state.HorizontalMargins PRINTLN state.MarginLeft, "-", state.MarginRight

ResetVMargins
ResetHMargins
; ResetMargins resets both axes at once.
```

`TermState()` returns an immutable snapshot with `MarginTop`, `MarginBottom`,
`MarginLeft`, `MarginRight`, `VerticalMargins` and `HorizontalMargins`. An
inactive axis reports zero bounds and a `FALSE` flag. The flags describe the
margins this session asked for, not a capability the terminal confirmed, so a
client that ignores `DECLRMM` still reports `HorizontalMargins` as `TRUE`.

Bounds are 1-based and the low bound must be smaller than the high one; an
empty or out-of-range region is ignored. Sessions without ANSI, including
AVATAR and plain ASCII, ignore all five statements rather than printing the
escape sequence as text. The
[`margins` PPE](../ppe/margins/src/margins.pps) demonstrates a bordered list
that uses the region for efficient keyboard, click and mouse-wheel scrolling.

### Fonts

`SetFont` picks one of the terminal's fonts, `LoadFont` uploads one of your own.

```PPL
SetFont 0, 5                    ; font 5 wherever the normal attribute applies
LoadFont 43, "topaz.psf"
IF (FontError() = FONT_OK) SetFont 0, 43
```

Font numbers 0 to 42 are the terminal's built-in fonts and are read-only:
`LoadFont` accepts 43 to 255 and answers `FONT_ERR_INVALID_SLOT` below that,
rather than sending an upload the terminal would silently discard. `SetFont`
selects any font, built-in or uploaded. `LoadFont` reads PSF1, PSF2, YAFF and
raw files, and a raw file is recognised by its size alone: 256 glyphs of equal
height.

The slot is one of the four attribute classes a terminal keeps a font for,
numbered 0 to 3. Four fonts can therefore be on screen at once, picked per
character by its attribute.

`FontError()` returns `FONT_OK`, `FONT_ERR_INVALID_SLOT`, `FONT_ERR_IO`,
`FONT_ERR_FORMAT` or `FONT_ERR_LIMIT` for the last font operation. Sessions
without ANSI ignore both statements. A font a PPE selects stays selected after
the PPE ends, since changing the board font may be the point of running it.

> **Note:** icy_term does not read the slot argument yet, so a font it accepts
> applies regardless of which slot was named. SyncTERM uses the slot as written.

## Runtime 4.01

Runtime 4.01 is a PPE-format extension, not another source language. It adds:

- a type table for `TYPE ... ENDTYPE` layouts
- a routine-reference marker for functions and procedures passed as values
- a record-literal opcode carrying type and field identifiers

Use language 400 with runtime 401 for the complete feature set. A language 350
source also needs runtime 401 when it passes routines; all its other additions
can lower to an older compatible runtime.

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

`ConfInfo(conf)` returns a read-only `CONFERENCE` snapshot. An invalid conference
number returns an empty conference object, so its properties can still be read.

| Conference member | Type | Description |
| :--- | :--- | :--- |
| `Name` | `STRING` | Conference name |
| `IsPublic` | `BOOLEAN` | Whether the conference is configured as public |
| `Directories` | `INTEGER` | Number of file directories |
| `Areas` | `INTEGER` | Number of message areas |
| `Doors` | `INTEGER` | Number of doors |
| `HasAccess()` | `BOOLEAN` | Whether the current caller can access the conference |
| `GetDir(index)` | `DIRECTORY` | File directory at the zero-based index |
| `GetArea(index)` | `AREA` | Message area at the zero-based index |
| `GetDoor(index)` | `DOOR` | Door at the zero-based index |

`DIRECTORY` and `AREA` provide `Name` and `HasAccess()`. `DOOR` provides
`Name`, `Description`, `Password` and `HasAccess()`. A door password has the
runtime-only `PASSWORD` type: it can be compared with a string, but converting
or printing it produces `******` rather than the secret.

```PPL
CONFERENCE conf = CONFINFO(CURCONF())
INTEGER i

FOR i = 0 TO conf.Doors - 1
	DOOR item = conf.GetDoor(i)
	IF item.HasAccess() PRINTLN item.Name
NEXT
```

## User contacts (4.00)

`U_CONTACT` is a mutable array of built-in `CONTACT` records and requires
runtime 4.02 because it extends the predefined user-variable prefix. Each record has
two `STRING` fields: `Service` and `Account`. Service names are open strings,
so a PPE can store new services without a language or user-schema change.

`GETUSER` fills the array and `PUTUSER` writes it back. On write, service names
are trimmed and converted to lowercase; entries with a blank service or account
are discarded. Duplicate services are allowed. Because PPL arrays use inclusive
upper bounds, an empty contact list is represented by one blank element at
index zero.

`U_EMAIL` and `U_WEB` remain separate predefined variables for PCBoard 3.40
compatibility and are not duplicated in `U_CONTACT`.

```PPL
GETUSER

INTEGER i
FOR i = 0 TO LEN(U_CONTACT, 1)
	IF U_CONTACT[i].Service <> "" THEN
		PRINTLN U_CONTACT[i].Service, ": ", U_CONTACT[i].Account
	ENDIF
NEXT

REDIM U_CONTACT(LEN(U_CONTACT, 1) + 1)
U_CONTACT[LEN(U_CONTACT, 1)].Service = "matrix"
U_CONTACT[LEN(U_CONTACT, 1)].Account = "@sysop:example.org"
PUTUSER
```

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
