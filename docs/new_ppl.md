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
| Routine documentation | 400 | any compatible runtime | Markdown `;;;` comments shown by editor hover, completion and signature help |
| Compile-time modules | 400 | any compatible runtime | `MODULE`, visibility sections and `IMPORT ... AS ...` namespaces |
| Main-program block | 400 | 400 | Real `BEGIN ... END`; `EXIT` replaces the old terminating use of `END` |
| Board objects and member calls | 400 | 400 | `CONFERENCE`, `DIRECTORY`, `AREA`, `DOOR`, `PASSWORD`, `Board`, `Session` |
| Message-area identifiers | 400 | 400 | `MSGAREAID` and `AreaId(conf, area)` |
| Overloaded built-ins | 400 | 400 | Argument-count overloads such as `Len(array, dim)` |
| Web requests | 400 | 400 | String-returning function and file-writing statement forms |
| Binary conversion and checksums | 400 | 400 | `BYTES`, `Checksum`, `TOBYTES`, base64 and checksum members |
| Extensible user contacts | 400 | 400 | `CONTACT` records on `Session.User` |
| User-defined records | 400 | 400 | `TYPE ... ENDTYPE`, nested fields, arrays of records and nominal type checking |
| Named record literals | 400 | 400 | `Point { X = 1, Y = 2 }` with checked and optional fields |
| Terminal multimedia | 400 | 400 | Sixel/JXL graphics, SyncTERM audio, mouse and physical key events |
| Regular expressions | 400 | 400 | Compiled Unicode patterns, captures, replacement and splitting |

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

### Modules and imports

A source file can place its declarations in a compile-time namespace. Declarations
are public by default; a standalone `PRIVATE` or `PUBLIC` line changes the
visibility of the declarations which follow it:

```PPL
MODULE TerminalList

PROCEDURE Draw()
ENDPROC

PRIVATE

INTEGER selected

PROCEDURE ClearRow(INTEGER row)
ENDPROC

ENDMODULE
```

Another source in the same package imports the module under a local alias and
qualifies public routines, values and types through that alias:

```PPL
IMPORT TerminalList AS List

List.Draw()
```

`PUBLIC` and `PRIVATE` are context-sensitive section words only inside a module,
so existing variables with those names remain valid elsewhere. Imports are local
aliases and are never re-exported. Private declarations remain available to code
inside their own module but cannot be reached through an import. Module names,
imports and visibility are removed while compiling; the result is still one
self-contained PPE and the syntax itself adds no runtime requirement.

A module declares; it has no program of its own. Variables, constants, types and
routines are allowed, executable statements are not, because the module would
otherwise run inside whichever program imports it.

One source file defines at most one module. Module and alias names are single PPL
identifiers. A package may contain ordinary application sources and module sources
together.

### Source libraries

Packages can include modules from other PPL packages through path or Git
dependencies in `ppl.toml`. A path is relative to the manifest containing the
dependency:

```toml
[dependencies]
terminal-ui = { path = "../terminal-ui" }
```

Git dependencies can follow the repository's default branch or select exactly
one revision, branch or tag:

```toml
[dependencies]
common = { git = "https://example.invalid/common-ppl.git", rev = "0123456789abcdef" }
widgets = { git = "https://example.invalid/widgets-ppl.git", branch = "main" }
themes = { git = "https://example.invalid/themes-ppl.git", tag = "v1.2.0" }
```

Each dependency must have a `ppl.toml` at its package root and sources below its
`src` directory. Dependencies may have dependencies of their own. Git packages
are checked out below `target/ppl-dependencies/git`; revision and tag checkouts
are reused, while moving branches are refreshed when dependencies are resolved.
The `git` executable must be available to the compiler and language server.

Plain library sources are grouped into an implicit module named after the
dependency entry. `AS` gives that module a source-local name in the consuming
package:

```PPL
IMPORT themes AS MyTheme

MyTheme.Apply()
```

This behaves as if the plain sources had been enclosed by `MODULE themes` and
`ENDMODULE`; declarations from all plain source files form one namespace.
`PUBLIC` and `PRIVATE` sections can be used without writing those delimiters.
A library source that declares an explicit `MODULE` keeps that explicit module
instead, allowing one package to provide additional named modules. Because a
library is a module, its sources declare rather than run, and a `;$LANGVERSION`
a library states applies to that library alone. Pin Git dependencies with `rev`
when reproducible builds are required.

### Routine documentation

A contiguous block of `;;;` comments documents the function, procedure or
`DECLARE` statement immediately below it. The text is Markdown and follows the
usual Rust documentation style for summaries, headings, lists and fenced PPL
examples:

```PPL
;;; Draws one item in the visible list.
;;;
;;; # Arguments
;;;
;;; - `item` - Zero-based item index.
;;; - `row` - Screen row where the item is drawn.
PROCEDURE DrawRow(INTEGER item, INTEGER row)
	PRINTLN item, row
ENDPROC
```

One optional space after `;;;` is removed; other Markdown indentation is
preserved. A blank line or ordinary comment breaks attachment. When both a
`DECLARE` statement and its implementation have documentation, the declaration
is canonical. Documentation is source metadata and does not change the PPE
runtime format.

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
are not. Fields can also be one-, two- or three-dimensional arrays, including
arrays of a previously declared record:

```PPL
TYPE BoardMap
	STRING Labels(10, 20)
	Point Positions(100)
ENDTYPE

BoardMap map
map.Labels(2, 3) = "Lobby"
map.Positions(4).X = 12
```

Array fields are part of a record value: assignment copies their contents and
record equality compares them. Their bounds are fixed by the `TYPE` declaration;
`REDIM map.Labels, ...` and `map.Labels.Redim(...)` are compile errors.
They otherwise have the read-only array surface: `map.Labels.Len(1)` reports the
number of elements in that dimension and `FOREACH label IN map.Labels` walks every element. A whole field may
be assigned from another field only when element type, rank and all bounds match;
use an index whenever a scalar value is required.

The PPE must store each record layout, so any use of `TYPE` requires runtime
4.00. Field and type names are not stored; a decompiler invents names for them.

### String members

At language 400 `STRING` is the string type and is not length-limited, so it is
used throughout. `BIGSTR` is a deprecated alias kept only for older sources; the
compiler warns when it is written at 400.

`STRING` values expose their common operations as members. This is the same
operation as the classic global function where one exists, written with the
value first:

```PPL
STRING text = "  one,two,two  "

PRINTLN text.Len()
PRINTLN text.Find("two")
PRINTLN text.Find("two", 7)
PRINTLN text.FindLast("two")
PRINTLN text.Contains("one")
PRINTLN text.Count("two")
PRINTLN text.Trim().ToUpper().Replace("TWO", "THREE")
```

| Member | Returns | Meaning |
| :--- | :--- | :--- |
| `Len()` | `INTEGER` | Number of Unicode characters |
| `Find(search [, start [, comparison]])` | `INTEGER` | First match at or after `start` |
| `FindLast(search [, start [, comparison]])` | `INTEGER` | Last match at or before `start` |
| `Contains(search [, comparison])` | `BOOLEAN` | Whether a non-empty search string occurs |
| `StartsWith(prefix [, comparison])`, `EndsWith(suffix [, comparison])` | `BOOLEAN` | Prefix or suffix test |
| `Count(search [, comparison])` | `INTEGER` | Non-overlapping occurrence count |
| `Equals(other [, comparison])` | `BOOLEAN` | String equality |
| `Replace(search, replacement)` | `STRING` | Replace every substring match |
| `Mid(start, length)` | `STRING` | Substring of `length` characters from zero-based `start` |
| `Left(count)`, `Right(count)` | `STRING` | Leftmost or rightmost `count` characters |
| `Trim([characters])` | `STRING` | Trim whitespace, or the supplied characters, at both ends |
| `TrimStart([characters])`, `TrimEnd([characters])` | `STRING` | Trim one end |
| `ToUpper()`, `ToLower()` | `STRING` | Change case |

Positions in the PPL 400 member API are zero-based Unicode character positions;
`-1` means no match. Searches are case-sensitive. An empty search string is not
considered a match and has a count of zero. `Find` and `FindLast` are the
zero-based member forms of the classic `INSTR` and `INSTRR`, which remain 1-based
and return zero when no match is found. `Mid` is likewise the zero-based member
form of the 1-based classic `MID`; `Left` and `Right` are count-based and behave
exactly like the classic functions. A single character is also reachable through
zero-based indexing (`text[0]`).

`StringComparison.Ordinal` is the default. Pass
`StringComparison.OrdinalIgnoreCase` as the last argument for Unicode-aware,
case-insensitive searching or equality.

Scalar strings support zero-based Unicode character indexing in language 400.
`text[0]` returns the first character as a `STRING`; a negative or out-of-range
index returns an empty string. String arrays keep their normal array semantics,
and indexing can be chained: `words[0][0]` reads the first character of the
first string.

Operations that transform text return `STRING`. A language 400 `STRING` has no
length limit, so member chains do not truncate.

The `STRING` type name also provides operations that do not belong to one value:

```PPL
STRING parts[] = "a,,b,".Split(",")
PRINTLN STRING.Join(parts, "|")
PRINTLN STRING.Repeat("-", 40)

parts = STRING.Split("one:two:three:four", ":", 3)
; parts contains "one", "two", "three:four"
```

`Split` accepts a multi-character separator and retains empty elements. Its
result is a dynamic `STRING[]`. The optional positive limit is the maximum
number of elements, with the unsplit remainder in the last one. A limit of zero
means unlimited. Empty separators and negative limits report `ErrKind.String` /
`ErrCode.Invalid` and return an empty array. Returned arrays may be assigned,
indexed, queried with `Len()` or consumed directly by `FOREACH`.

`STRING.Join(array, separator)` joins a one-dimensional string array and returns
`STRING`. `STRING.Repeat(value, count)` returns `STRING`; a negative count is an
error and results above 16 MiB report `ErrCode.Limit`.

### Regular expressions

`REGEX` compiles a pattern once and reuses it for matching, capture extraction,
replacement and splitting:

```PPL
REGEX parser = REGEX.Compile("(?P<name>\w+):(?P<value>\d+)")
REGEXMATCH found = parser.Find("score:120")
REGEXMATCH foundAll[]
foundAll = parser.FindAll("score:120 level:4")

IF found.Success THEN
	PRINTLN found.NamedGroup("name"), " = ", found.NamedGroup("value")
ENDIF
```

Static members are `REGEX.Compile(pattern [, options])`, `REGEX.Escape(text)`
and `REGEX.IsValid(pattern [, options])`. A compiled value exposes `Valid`,
`Pattern`, `IsMatch(text [, start])`, `Find(text [, start])`,
`FindAll(text [, start [, limit]]) -> REGEXMATCH[]`,
`Replace(text, replacement [, limit])` and `Split(text [, limit]) -> STRING[]`.

`RegexOptions` flags are `None`, `IgnoreCase`, `MultiLine`,
`DotMatchesNewLine`, `IgnoreWhitespace`, `SwapGreed` and `Ascii`; flags may be
combined with `|`. Matching is Unicode-aware unless `Ascii` is selected.
Positions, match collections and capture groups are zero-based. A missing match
or unmatched capture has start position `-1`. Group zero is the complete match.

`REGEXMATCH` exposes `Success`, `Value`, `Start`, `Length`, `GroupCount`,
`Group(index)`, `NamedGroup(name)`, and corresponding `GroupMatched`,
`GroupStart` and `GroupLength` methods. Named variants use the `Named` prefix.
`FindAll` returns a dynamic `REGEXMATCH[]` array. Access matches with
`matches[index]`; `matches.Len()` reports the number of matches.

Replacement strings expand `$1` and `$name`. A zero limit means unlimited;
negative limits report `ErrKind.Regex` / `ErrCode.Invalid`. `Split` preserves
empty fields and replaces only dynamic one-dimensional `STRING` (or legacy
`BIGSTR`) arrays, transactionally. Results are limited to 100,000 matches and replacement
output to 16 MiB.

The engine guarantees linear-time matching and deliberately does not support
look-around or backreferences. Unicode case-insensitive matching does not apply
multi-character folds such as `ß` to `SS`. Invalid patterns return an invalid
`REGEX` value and report through `Error.Last()`.

#### Record file I/O

Records can use an already open file channel in either an editable line format
or a compact binary format. Both formats walk fields in declaration order,
nested records depth-first and fixed arrays in row-major order.

`FPUTREC` writes one physical line per scalar field. `FGETREC` reads exactly the
number of lines the destination record needs, so ordinary text after the record
is left for the next `FGET`:

```PPL
FCREATE 1, "person.txt", O_WR, S_DN
FPUTREC 1, person
FPUTLN 1, "This text documents the record."
FCLOSE 1

FOPEN 1, "person.txt", O_RD, S_DN
FGETREC 1, person
FGET 1, documentation
FCLOSE 1
```

Strings keep one physical line by escaping backslash, carriage return, line
feed and NUL as `\\`, `\r`, `\n` and `\0`. Numeric values use locale-independent
decimal text, booleans use `0` or `1`, and `MSGAREAID` uses `conference,area`.

`FWRITEREC` writes a little-endian `u32` payload length followed by a positional
binary payload. Fixed-width values use their declared widths; `STRING` and
`BIGSTR` use a little-endian `u32` UTF-8 byte length followed by their bytes.
`FREADREC` reads one such frame. Frames are limited to 16 MiB and deliberately
carry no schema fingerprint, so they must be read with the matching record type.

All record reads are transactional. A malformed or truncated input leaves the
destination unchanged and reports through both `FERR(channel)` and
`Error.Last()`. Record I/O supports nested records and fixed arrays, but not
functions, procedures, tables or board resource objects.

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

`ErrKind` is `None`, `File`, `DBase`, `Stack`, `Gfx`, `Font`, `Audio`, `Term` or
`Msg`. `ErrCode` is `Ok`, `Unavailable`, `Invalid`, `Io`, `Format`, `Limit`,
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
| `IsReadOnly` | `BOOLEAN` | Whether messages may only be read |
| `AllowAliases` | `BOOLEAN` | Whether a caller may post under an alias |
| `EchoMail` | `BOOLEAN` | Whether mail written here is echoed |
| `AutoRejoin` | `BOOLEAN` | Whether a caller is rejoined here on the next call |
| `PrivateUploads` | `BOOLEAN` | Whether uploads go to the private area |
| `Password` | `PASSWORD` | The password needed to join |
| `Directories` | `DIRECTORY[]` | The file directories of the conference |
| `Areas` | `AREA[]` | The message areas of the conference |
| `Doors` | `DOOR[]` | The doors of the conference |
| `HasAccess()` | `BOOLEAN` | Whether the current caller can join the conference |
| `CanPost()` | `BOOLEAN` | Whether the current caller may write a message |
| `CanAttach()` | `BOOLEAN` | Whether the current caller may attach a file |

| Area member | Type | Description |
| :--- | :--- | :--- |
| `Name`, `Number`, `Valid` | | Name, the number it was fetched under, and whether it exists |
| `IsReadOnly` | `BOOLEAN` | Whether messages may only be read |
| `AllowAliases` | `BOOLEAN` | Whether a caller may post under an alias |
| `QwkName` | `STRING` | The name this area carries in a QWK packet |
| `EchoTag` | `STRING` | The FTN tag, empty when the area is local |
| `HasAccess()` | `BOOLEAN` | Whether the current caller may list it |
| `CanEnter()` | `BOOLEAN` | Whether the current caller may join it |
| `CanAttach()` | `BOOLEAN` | Whether the current caller may save an attachment |
| `LowMsg()`, `HighMsg()` | `LONG` | The numbers its messages run between, zero when there are none |
| `Read(number)` | `MSG` | The message with that number |
| `Find(field, text [, start])` | `MSG` | The first message at or after `start` whose field contains `text` |

| Directory member | Type | Description |
| :--- | :--- | :--- |
| `Name`, `Number`, `Valid` | | Name, the number it was fetched under, and whether it exists |
| `Path` | `STRING` | Where the files are kept |
| `IsFree` | `BOOLEAN` | Whether downloads here cost no time or bytes |
| `HasNewFiles` | `BOOLEAN` | Whether the directory is flagged as having new files |
| `Password` | `PASSWORD` | The password needed to reach it |
| `HasAccess()` | `BOOLEAN` | Whether the current caller may list it |
| `CanDownload()` | `BOOLEAN` | Whether the current caller may download from it |

| Door member | Type | Description |
| :--- | :--- | :--- |
| `Name`, `Number`, `Valid` | | Name, the number it was fetched under, and whether it exists |
| `Description` | `STRING` | Door description |
| `Path` | `STRING` | What the door runs |
| `Password` | `PASSWORD` | The password needed to open it |
| `HasAccess()` | `BOOLEAN` | Whether the current caller can open it |

Every board object reports the number it was fetched under, so a listing can
name what a caller has to type.

`HasAccess()` is always the question a *listing* asks. What a caller may then do
is asked separately - `CanPost()`, `CanEnter()`, `CanAttach()`, `CanDownload()` -
because seeing a conference and writing in it are configured apart.

`HighMsg()` and `LowMsg()` read the message base to answer, which is why they
are calls rather than properties.

A password has the runtime-only `PASSWORD` type: it can be compared with a
string, but converting or printing it produces `******` rather than the secret.
A listing can therefore say *that* a conference, directory or door is locked
without saying what unlocks it:

```PPL
CONFERENCE conf = Board.Conferences[0]

IF conf.Password <> "" PRINTLN conf.Name, " needs a password"
```

### Messages (4.00)

A message is read out of its area as a `MSG`. What `GETMSGHDR` returned as a
string picked by a number is a member with a type of its own:

```PPL
AREA area = Session.Area
MSG msg = area.Read(1)

IF msg.Valid THEN
	PRINTLN msg.Number, "  ", msg.From, " -> ", msg.To
	PRINTLN msg.Subject, "  ", msg.Date, " ", msg.Time
	PRINTLN msg.Text()
ENDIF
```

| Member | Type | Description |
| :--- | :--- | :--- |
| `Valid` | `BOOLEAN` | Whether the area has that message |
| `Number` | `LONG` | The number it was read under |
| `From`, `To`, `Subject` | `STRING` | Who wrote it, who it is for, what it is about |
| `Date`, `Time` | `DATE`, `TIME` | When it was written |
| `ReplyTo` | `LONG` | The message this one answers, zero when it answers none |
| `Status` | `STRING` | The one character `PCBoard` kept, as `HDR_STATUS` reports it |
| `IsPrivate`, `IsRead`, `IsDeleted`, `IsEcho`, `NeedsPassword` | `BOOLEAN` | What the header says about it |
| `Size` | `LONG` | How many bytes the body holds |
| `Text()` | `STRING` | The body |

A message is addressed by its **number**, not by its position. A message base is
sparse: numbering starts at `LowMsg()` and a deleted message leaves its number
behind, so a walk counts over the range and asks each one whether it is there:

```PPL
LONG n

FOR n = area.LowMsg() TO area.HighMsg()
	MSG msg = area.Read(n)
	IF !msg.Valid CONTINUE
	PRINTLN msg.Number, " ", msg.From, " ", msg.Subject
NEXT
```

Message numbers and body sizes are `LONG`. JAM counts them in 32 unsigned bits,
which all fit in a signed 64-bit value, and ordinary integer literals can be
added or compared without narrowing. A number outside JAM's range is one no
message has, so `Read()` answers an invalid `MSG` rather than wrapping.

`LONG` and `ULONG` are signed and unsigned 64-bit integers in language 4.00.
Before 4.00, `LONG` was a synonym for the 32-bit `INTEGER`, and `ToLong()`
therefore performed the same conversion as `ToInteger()`. In 4.00 `ToLong()`
returns the new 64-bit type; `ToULong()` returns `ULONG`. When upgrading old
source, replace an old `ToLong(value)` with `ToInteger(value)` to keep its
32-bit behavior. The PPL language server's **Upgrade file to language version
400** action applies that rewrite automatically.

That is also why messages are not a collection: `[ ]` indexes a position
everywhere else in the language, and a message number is not one.

The body stays in the base until `Text()` asks for it, which is why it is a call.
A listing that only prints headers never pays for a single body.

A message number that is outside the base, deleted or an empty slot is an
ordinary lookup miss: `Read()` answers an invalid `MSG`, `Text()` answers an
empty string and `Error.Last().OK` remains true. Running off the end of `Find()`
works the same way.

An operation that cannot read the base is different. `Read()`, `Find()`,
`LowMsg()`, `HighMsg()` and `Text()` keep their normal invalid/zero/empty return
value, and also report `ErrKind.Msg`: `ErrCode.Io` for a filesystem failure and
`ErrCode.Format` for corrupt JAM data. Those failures enter an `ON ERROR`
handler. An invalid `MsgField` reports `ErrCode.Invalid`.

An area is read through one open message base rather than opening it again for
every message, which is what makes the walk above worth writing. The base is
opened when a PPE first reads from that area and kept until it reads from
another one or the PPE ends. A message written after it was opened is still
found: writing through `MESSAGE`, `SETMSGHDR`, `KILLMSG` or `MOVEMSG` takes the
base again, and a number past the end is looked up once more before it is
reported missing. `LOMSGNUM()` and `HIMSGNUM()` open the base on every call, so
they remain the way to watch a base another node is writing to.

`Find` is `SCANMSGHDR` with a type instead of a field number. It matches without
regard to case, anywhere in the field, and answers an invalid `MSG` when nothing
matches. The `start` argument is what walks on to the next match:

```PPL
MSG hit = area.Find(MsgField.To, "STAN")

WHILE hit.Valid DO
	PRINTLN hit.Number, " ", hit.Subject
	hit = area.Find(MsgField.To, "STAN", hit.Number + 1)
ENDWHILE
```

`MsgField` is `To`, `From` or `Subject`. Its values are the matching `HDR_*`
constants, so naming one is a way of writing the number.

A `MSG` is a read-only snapshot of what the area holds. `GETMSGHDR`,
`SETMSGHDR`, `SCANMSGHDR` and the `MESSAGE` statement are unchanged, and writing
a message is still theirs.

> The type is called `MSG` rather than `MESSAGE` because `MESSAGE` has been a
> statement since PPL 1.00 and keeps that meaning.

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

The index is there when a single entry is wanted, and `Len()` when only the
number matters:

```PPL
PRINTLN conf.Areas.Len(), " areas, the first is ", conf.Areas[0].Name
```

An index no entry has answers with an invalid object rather than failing, so
`Valid` is what to ask. Collection properties return array snapshots; bind one
to a variable when it will be reused.

An array snapshot is an ordinary value, so it can be held in a variable. Naming
it once avoids rebuilding it and is the clearest form for repeated access:

```PPL
AREA list[] = Session.Conference.Areas
AREA item

FOREACH item IN list
	PRINTLN item.Name
ENDFOREACH
```

## Board and session (4.00)

`Board` and `Session` are the two other objects that stand for themselves, so
they need no parentheses either. They split what the board *is* from what this
one call *is doing*.

`Board` is a snapshot of the configuration, conferences and users. It is taken the
first time a PPE reads `Board` and stands for the rest of the run, so touching it
inside a loop is not paid for again:

| Member | Type | Description |
| :--- | :--- | :--- |
| `Name` | `STRING` | Board name |
| `Location` | `STRING` | Where the board says it is |
| `Operator` | `STRING` | Operator named for `EMSI` |
| `SysopName` | `STRING` | The sysop's display name |
| `NodeCount` | `INTEGER` | Number of configured nodes |
| `Conferences` | `CONFERENCE[]` | The conferences of the board |
| `Users` | `USER[]` | The registered users of the board |

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

`Users` exposes every registered user as a read-only `USER` snapshot. The
collection is fixed when `Board` is first read, and an index it does not contain
returns an empty user whose `Valid` property is false:

```PPL
USER user

FOREACH user IN Board.Users
	PRINTLN user.Name, " from ", user.City
ENDFOREACH
```

The snapshot includes the user's notes and contacts, but assignments and
`SetPassword()` are refused. Use `Session.User` when changing the caller.

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

The session is read-only. The classic `CURCONF()`, `PCBNODE()`, `MINLEFT()` and the
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
| Identity | `Valid`, `RecordNumber`, `Name`, `Alias`, `VerifyAnswer` | all but `Valid`, `RecordNumber` and `Name` |
| Address | `Street1`, `Street2`, `City`, `State`, `Zip`, `Country` | yes |
| Reaching them | `BusinessPhone`, `HomePhone`, `Email`, `Web`, `Gender`, `BirthDate` | yes |
| Sysop text | `Comment`, `SysopComment`, `Notes`, `SetNote(index, text)` | yes |
| Preferences | `ExpertMode`, `EditorMode`, `ClearScreen`, `ScrollMessageBody`, `ShortDescriptions`, `LongHeader`, `WideEditor`, `PageLength`, `Protocol` | yes |
| Preferences the session owns | `UseGraphics`, `UseAlias`, `Language`, `DateFormat` | no |
| Security | `SecurityLevel`, `ExpiredSecurityLevel`, `ExpirationDate`, `PasswordExpires`, `SetPassword(text)` | yes |
| Statistics | `TimesOn`, `FirstDateOn`, `LastDateOn`, `LastDirRead`, `MessagesRead`, `MessagesLeft`, `Uploads`, `Downloads`, `UploadBytes`, `DownloadBytes`, `DownloadBytesToday`, `MinutesToday` | no |
| Contacts | `Contacts`, `AddContact(service, account)`, `RemoveContact(index)` | yes |

Whatever `PUTUSER` could write is writable here and is saved to the user file
immediately, so the object replaces the old round trip rather than sitting beside
it. The caller's `Name` identifies them and the board's own accounting is the
board's to keep, so both stay read-only; writing one is a compile error.
`RecordNumber` is the 1-based position of the record in the user file. Nobody
logged in reads as an empty user with `Valid` false rather than failing, so a
member is always safe to read.

The cumulative statistics `TimesOn`, `MessagesRead`, `MessagesLeft`, `Uploads`
and `Downloads`, together with the byte totals `UploadBytes`, `DownloadBytes`
and `DownloadBytesToday`, are 64-bit `ULONG`, so they preserve the full counters
stored by the board. `PageLength`
accepts 0 through 65535; `SecurityLevel` and `ExpiredSecurityLevel` accept 0
through 255. An out-of-range write leaves the old value intact and reports
`ErrKind.User` with `ErrCode.Invalid`.

`EditorMode` is one `EDITORMODE` value — `Yes`, `No` or `Ask` — rather than the
two overlapping flags `PCBoard` kept. `SetNote(index, text)` writes one of the
five note slots, for an index from 0 to 4. `SetPassword()` hashes
the text the way the board is configured to, so the plain text is never stored;
an empty password is refused and it answers `FALSE` rather than failing.

### Notes and contacts

`User.Notes` returns a five-element `STRING[]` snapshot. It is read with an
index, queried with `Len()` and walked with `FOREACH`. Mutation is explicit:

```PPL
Session.User.SetNote(0, "Called about the upload")
```

An index outside 0 through 4 is refused and leaves the notes unchanged. Passing
an empty string clears a slot. An array already returned by `User.Notes` remains
unchanged after `SetNote`; read the property again to obtain the new snapshot.
`Board.Users` entries are read-only and reject `SetNote`.

A contact is a built-in `CONTACT` record with two `STRING` fields, `Service` and
`Account`. `User.Contacts` returns a `CONTACT[]` snapshot in stable list order.
Service names are open strings, so a PPE can store a new service without a
language or user-schema change.

```PPL
CONTACT entry

FOREACH entry IN Session.User.Contacts
	PRINTLN entry.Service, ": ", entry.Account
ENDFOREACH

Session.User.AddContact("matrix", "@sysop:example.org")
Session.User.RemoveContact(0)
```

`AddContact()` trims and normalizes the service name, trims the account and
appends the contact. Duplicate services are allowed. A blank service or account
is refused and answers `FALSE`. A user may hold at most 100 contacts; a further
`AddContact()` is refused with `ErrCode.Limit`. `RemoveContact(index)` removes
the entry at the zero-based index and answers whether it succeeded. An index no
contact has answers with an empty `CONTACT`.

The returned array is a snapshot. Adding or removing contacts does not mutate an
array already held by the PPE; read `User.Contacts` again to get the new list.

Mutations write straight through to the caller, so no `GETUSER`/`PUTUSER` round trip
is needed. `U_EMAIL` and `U_WEB` remain separate predefined variables for
PCBoard 3.40 compatibility and are not duplicated here.

## The `BYTES` type (4.00)

`BYTES` is a compact, growable binary blob — a contiguous run of bytes stored one
byte per byte, unlike `BYTE[]` which boxes every element. It is the type for
binary data, hashing, encoding and fast I/O. A `BYTES` value prints as
uppercase, separator-free hexadecimal, and `LEN(value)` returns its byte count.

`TOBYTES(value)` returns the binary representation of a supported scalar. Strings
use UTF-8; numeric values use their fixed-width little-endian representation.
Arrays, records, objects, tables, passwords and routine references are rejected
with `ErrCode.Invalid`. `value.ToString()` decodes UTF-8; invalid bytes report
`ErrCode.Format`.

```PPL
BYTES raw = TOBYTES("Grüße")
PRINTLN raw              ' 47 72 c3 bc c3 9f 65 -> "4772C3BCC39F65"
PRINTLN LEN(raw)         ' 7
PRINTLN raw.ToString()   ' Grüße
```

## Encoding and digest functions (4.00)

`BASE64ENC(value)` encodes a `BYTES` blob as base64 text. A string argument is
taken as its UTF-8 bytes. `BASE64DEC(value)` decodes base64 text to a `BYTES`
blob; whitespace (for line-wrapped input) is ignored, and any other malformed
input reports `ErrCode.Format` through `Error.Last()`.

`value.GetChecksum(algorithm)` returns the checksum as raw `BYTES`. Supported
algorithms are `Checksum.CRC32`, `Checksum.MD5` and `Checksum.SHA256`; more can
be added without changing the method. MD5 and SHA-256 are intended for content
integrity and identity, not password storage. `value.ToHex()` returns an
uppercase hexadecimal `STRING` with two digits per byte, preserving leading
zero bytes. `value.ToString()` remains the UTF-8 decoder.

```PPL
PRINTLN BASE64ENC("Grüße")
BYTES decoded = Bytes.FromBase64("R3LDvMOfZQ==")
PRINTLN decoded.ToString()
STRING fingerprint = TOBYTES("abc").GetChecksum(Checksum.SHA256).ToHex()
PRINTLN fingerprint
```


## Math functions (4.00)

`SIN(radians)`, `COS(radians)` and `TAN(radians)` return the sine, cosine and
tangent of an angle given in radians. `ATAN(value)` returns the arctangent of
`value`, in radians. `LOG(value)` returns the natural logarithm of `value`.
`SQRT(value)` returns the square root of `value`. All six take and return
`DOUBLE`.

```PPL
DOUBLE pi
pi = 3.14159265358979
PRINTLN Sin(pi / 2.0)
PRINTLN Sqrt(2.0)
```

## HTTP objects (4.00)

PPL programs may use public HTTP and HTTPS destinations without board-specific setup.
Private, loopback, link-local and other special-use addresses remain blocked.
The sysop can optionally disable outbound access or restrict it to an exact
origin allowlist. New code receives a typed response instead of treating an HTTP
status or a transport failure as an empty string:

```PPL
HttpResponse response = Http.Get("https://api.example.com/status")
IF NOT response.Valid THEN
	PRINTLN Error.Last().Message
	RETURN
ENDIF
PRINTLN response.Status, " ", response.OK
PRINTLN response.Header("Content-Type")
PRINTLN response.Text()
```

`Valid` means the transport completed and the body stayed within its limit.
`OK` means status 200 through 299. A 404 is therefore valid but not OK. Other
properties are `Status`, `FinalUrl`, `Size` and `ContentType`. `Save(path)`
writes a body already held by a response. `Http.Download(url, path)` streams a
successful response through a temporary file and replaces the destination only
after the complete body arrives. Its successful response reports status and
size but does not retain another copy of the body; calling `Text()` or `Save()`
on that response reports `ErrCode.Invalid`.

`Text()` decodes the body strictly as UTF-8 and returns a `STRING`. A body in any
other character encoding reports `ErrKind.Net` with `ErrCode.Format`; use
`Download()` or `Save()` when the response is binary or not UTF-8.

For a POST request or custom headers, build a request:

```PPL
HttpRequest request = Http.New(HttpMethod.Post, "https://api.example.com/items")
request = request.SetHeader("Accept", "application/json")
request = request.SetText(json, "application/json")
HttpResponse response = request.Send()
```

The builder functions return a new request and leave the receiver unchanged.
`HttpMethod` contains `Get`, `Head` and `Post`. `SetText()` needs a method that
carries a body; on a `Get` or `Head` request it reports `ErrCode.Invalid` and
leaves the request unchanged. Routing and hop-by-hop headers, including `Host`,
`Content-Length`, `Connection` and `Transfer-Encoding`, cannot be set by a PPE.

No `[ppl_http]` section is required. The default policy is equivalent to:

```toml
[ppl_http]
destination_policy = "public"
allow_http = true
```

Boards that want to restrict doors to specific services can use an exact origin
allowlist and adjust the resource limits:

```toml
[ppl_http]
destination_policy = "allowlist"
allowed_origins = ["https://api.example.com"]
max_response_bytes = 16777216
max_request_bytes = 1048576
connect_timeout_seconds = 5
request_timeout_seconds = 30
max_redirects = 3
max_concurrent_requests = 16
max_concurrent_per_node = 2
max_headers = 64
max_header_bytes = 65536
allow_http = false
```

The same optional controls are available under **Configuration Options → PPL
HTTP** in `icbsetup`; editing TOML directly is not required.

`public` permits HTTP and HTTPS destinations that resolve exclusively to public addresses.
Every redirect is checked and DNS answers are pinned to the connection. The
transport ignores system proxies. An allowlisted origin may deliberately name a
private service; scripts cannot add origins or relax any board limit.

Policy, DNS, TLS, timeout, size and file failures set `Error.Last()` with
`ErrKind.Net`. HTTP status codes remain on `HttpResponse.Status`.

## `Len()`  Function (4.00)

### Function
This overload returns the number of elements in one array dimension. PPL array
declarations still use upper bounds, so `INTEGER values(10)` contains eleven
elements and `Len(values, 0)` returns `11`. For multidimensional arrays, `dim`
is zero-based.

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
error. Runtime 4.00 therefore stores `FOREACH`, its next step and its break as
dedicated bytecodes. The VM keeps the flat row-major iterator state; there are
no hidden element-count or element-access functions a PPE can call.

## Array members (4.00)

### Function

PPL 4.00 uses square brackets for array declarations and indexing. Empty
brackets declare a dynamic vector; commas declare a dynamic matrix or cube:

```PPL
INTEGER values[]
STRING matrix[,]
REGEXMATCH cubes[,,]
INTEGER fixed[10]
```

Functions can return dynamic arrays by adding the rank after the return type:

```PPL
DECLARE FUNCTION ReadValues() INTEGER[]

INTEGER values[]
values = ReadValues()
PRINTLN values[0]
```

Whole-array assignment copies the value and adopts its bounds when element type
and rank match. Existing 4.00 source that declares arrays with parentheses is
accepted with a migration warning; newly formatted and decompiled 4.00 source
always writes square brackets. Older language versions retain the classic
parenthesis syntax.

Everything built in that takes an array first may also be written as a member of
that array, whichever reads better at the call site. The two spellings are the
same call, so neither can drift from the other.

| Member | Meaning |
| :--- | :--- |
| `a.Len()` | Total number of elements across all dimensions |
| `a.Len(dim)` | Number of elements in one zero-based dimension |
| `a.Redim(n)` | Same as `REDIM a, n` |
| `a.Redim(n1, n2)` | Same as `REDIM a, n1, n2` |
| `a.Redim(n1, n2, n3)` | Same as `REDIM a, n1, n2, n3` |

```PPL
INTEGER values[10]

PRINTLN values.Len(), " slots"
values.Redim(20)
```

Only a declared array has these members. An array's type is its element's, so it
is the declaration that says it has them; asking a plain value for `.Len()` is a
compile error. `Redim` is a statement rather than a function, so it stands on a
line of its own the way `REDIM` does. Array-valued record fields have the fixed
bounds stored in their record type and cannot use either spelling of `REDIM`.
`Len()` reports the element count; after `Redim(20)`, it returns 21 and valid
indices are 0 through 20. Empty dynamic arrays report zero.

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
