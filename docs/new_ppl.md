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
| Open nominal enums | 350 | 400 for storage and explicit conversion | `ENUM ... ENDENUM`, scoped members such as `Color.Red`; member constants alone can target classic runtimes |
| Compile-time modules | 350 | any compatible runtime | `MODULE`, visibility sections and `IMPORT ... AS ...` namespaces |
| Routine parameters | 400 | 400 | Pass a matching function or procedure as a checked callable value |
| Main-program block | 400 | 400 | Real `BEGIN ... END`; `EXIT` replaces the old terminating use of `END` |
| Short-circuit logic | 400 | 400, in-memory only until the new encoding is decided | `&&` and `\|\|` skip the right operand when the left determines the result |
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
old instructions. Whether an authored declaration must match its implementation
depends on the source language, as described below.
Routine documentation is not in the table either, for the same reason.

## Logical evaluation and operator precedence

From **language 400**, `&&` and `||` short-circuit: `left && right` evaluates
`right` only when `left` is true; `left || right` evaluates it only when `left`
is false. Evaluation proceeds left to right, once per required operand. These
operators work in every expression context, including arguments and loop
conditions. Both operands are still checked for valid names and types, even
when execution skips the right operand.

`&` and `|` continue to evaluate both operands. Logical operations produce
`BOOLEAN` and retain the existing scalar truth-value conversions; numeric
operands do not turn these into bitwise operations. Enum bit operations remain
available through `&` and `|` on matching enum types, not through `&&` or `||`.
Integer bit operations remain the separate `AND()` and `OR()` functions.

In **languages below 400**, `&&` is an alias of `&`, and `||` is an alias of `|`:
all four evaluate both operands. This also holds when compiling legacy source
for runtime 400. Existing PPE files retain their original evaluation behavior.
Recompiling source as language 400 can therefore remove right-hand side effects;
use `&` or `|` where complete evaluation is intentional. No new keyword is added.

All language versions use original PPLC precedence, strongest first:

| Level | Operators |
| :--- | :--- |
| Unary sign | unary `+`, unary `-` |
| Power | `^` |
| Multiplication | `*`, `/`, `%` |
| Addition | `+`, `-` |
| Comparison | `=`, `<>`, `<`, `<=`, `>`, `>=` and their aliases |
| Logical negation | `!` |
| Logical AND | `&`, `&&` |
| Logical OR | `\|`, `\|\|` |

Binary operators at the same level associate left to right, including power.
Parentheses override these rules. Thus `TRUE | FALSE & FALSE` is true,
`!1 = 2` means `!(1 = 2)`, `-2^2` is 4, and `2^3^2` is 64. Correcting the
previous equal precedence of AND/OR and the overly strong NOT is a compiler
compatibility fix, not a language-400-only rule. Existing compiled PPE trees
are not regrouped; decompilation inserts parentheses where required.

**Temporary release boundary:** new short-circuit expressions currently run
only from the compiler's in-memory script. Writing such an executable is
explicitly rejected with `UnsupportedShortCircuitEncoding`. No provisional
opcode or file representation is emitted; file encoding and its roundtrip
acceptance belong to C1/C2 of the PPL 400 release plan. Programs without these
expressions continue to use the existing PPE path.

## DECLARE contracts and language versions

In **language 400**, an explicit `DECLARE` must match its implementation:
routine kind, parameter count, parameter types (including nominal enum/record
identity), `VAR` modes, array ranks, exact bounds and dynamic markers are
checked. Function return type and rank must match too. These checks recurse
through callback signatures; parameter names are irrelevant. In particular,
`INTEGER values[]` and `INTEGER values[0]` are different declarations. This
signature check does not prevent a whole-array argument from supplying its
current bounds at call time. `DECLARE` remains optional.

In **languages below 400**, the PPLC-compatible contract is deliberately
permissive: parameter count must match, but implementation parameter types,
`VAR` modes, dimensions and function result type win over the declaration.
An explicit `DECLARE PROCEDURE` may have a `FUNCTION` implementation, which
is normalized to a procedure with no function result slot; the reverse is
rejected. Even in this special case, `VAR` comes from the **implementation**,
not the declaration. This does not allow `VAR` on a genuine function.

The compiler and language server collect legacy implementation signatures
package-wide after module qualification and routine-kind normalization, before
checking calls, including calls whose implementation is in another file.
Strictness follows source language, not the output format: language 340 or 350
targeting runtime 400 still uses the legacy contract. Modern features retain
their separate runtime requirements.

Enums are an IcyBoard extension, not part of the DOS contract. From language
350 onward, enum parameters must match their declaration in nominal type,
`VAR`, rank and bounds; enum result types must match too. Classic parameter
types retain the legacy rules above.

The [DECLARE audit](../compat/DECLARE_AUDIT.md) gives the exact evidence scope:
23 authored probes against original PPLC 3.40, with IcyBoard legacy compiler
coverage for language/runtime 340/340, 340/400 and 350/400. It does not claim
identical diagnostics, byte-identical PPEs or compatibility with every original
compiler version.

## Routine documentation

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
is canonical.

A `;;;` block is an ordinary comment to the compiler, so it needs no language
version and changes no PPE: the text never reaches the file. It is read by the
language server, which shows it in hover, completion and signature help, and a
source written for any language version can carry it.

## Language version 3.50

Most 3.50 syntax lowers to classic PPE instructions: constants, loops,
initializers, brackets, compound assignments and modules can target an old
runtime. Enum member constants can too; nominal enum storage and explicit
conversions require runtime 400, equally for language 350 and 400.

### Initializers and indexing

```PPL
INTEGER count = 1
INTEGER values = { 10, 20, 30 }

values[0] += count
```

Compound assignments evaluate their target indices from left to right exactly
once, then read the target, evaluate the right-hand side, and write back to the
same target. An object-property target also retains the original receiver even
if the right-hand side reassigns the variable that held it.

The brace initializer declares the array and determines its size. Parenthesis
indexing remains valid for old source; brackets are recommended because they
cannot be mistaken for a function call. Below 350 all three bracket kinds were
merely other ways of writing `( )`, so this is where they gain a meaning of
their own rather than where they first parse.

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

Constants are typed compile-time expressions. Enums are open nominal integer types:
members are scoped below the enum name, and two different enum types cannot be
mixed merely because their stored numbers match. Their first declared member
is the default, even when its value is not zero. Every signed 32-bit integer is
a valid enum value, including unnamed flag combinations. The current PPE stores
ordered enum metadata, not an allowed-value restriction or source names; the
decompiler synthesizes names for user enums.

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

Module-level variable initializers must be constant expressions: literals,
`CONST` references, enum members, parentheses and unary/binary operations on
these values. Array and record literals are allowed when every supplied element
or field is constant. Existing constant declaration order and type rules still
apply. A constant initializer does not make a variable immutable; use `CONST`
for that. Declarations without an initializer retain normal default values.

Function calls are never allowed in module-level initializers, including pure
user functions and built-in or member functions. Reading a mutable variable,
an array element, or a runtime property such as `Session.User.Name` is also
forbidden. This is checked before optimization, so multiplying a forbidden
expression by zero does not make it valid.

The rule applies to explicit modules and implicit source-library modules. It
does not restrict initializers inside functions or procedures, or ordinary
application globals. Runtime setup belongs in a routine the application calls
explicitly, such as `MyModule.Initialize(...)`; no routine is called
automatically because of its name or an import.

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

## Language version 4.00

4.00 adds syntax and board APIs that do not exist on PCBoard. A runtime 4.00 PPE
therefore targets Icy Board rather than the original board.

### Functions and procedures as parameters

```PPL
PROCEDURE Apply(PROCEDURE action(), FUNCTION check(INTEGER n) BOOLEAN)
	IF check(1) action()
ENDPROC
```

The compiler checks routine kind, parameter types and dimensions, `VAR` flags
and function return type including array rank, also for nested callback
signatures. For example, `FUNCTION read() INTEGER[]` declares a vector-returning
callback; `INTEGER[,]` and `INTEGER[,,]` declare matrix and cube results.
A routine parameter is callable and can be passed on
to another routine. The callable reference is stored in the PPE, so it needs
runtime 4.00.

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
defined between individual records of the same type when every field supports
equality; arithmetic and ordering are not. Fields can also be one-, two- or three-dimensional arrays, including
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
record equality compares them. Explicit bounds are fixed by the `TYPE` declaration;
`REDIM map.Labels, ...` and `map.Labels.Redim(...)` are compile errors.
They otherwise have the read-only array surface: `map.Labels.Len(1)` reports the
number of elements in that dimension and `FOREACH label IN map.Labels` walks every element. A whole field may
be assigned from another field only when element type, rank and all bounds match;
use an index whenever a scalar value is required.

The PPE must store each record layout, so any use of `TYPE` requires runtime
4.00. Field and type names are not stored; a decompiler invents names for them.

#### S1: dynamic and host-object fields (in-memory implementation)

**File-format gate:** The compiler and VM implement the following additions,
but the PPE writer deliberately rejects these new record layouts until the
separate PPE-400 format decision is implemented. They cannot yet be deployed
as PPE files. Existing fixed value-only layouts retain their encoding.

- `INTEGER Values[]`, `STRING Grid[,]` and `Point Cells[,,]` declare empty
	dynamic fields with a fixed element type and rank. Assignments may change
	their bounds; `REDIM` and `.Redim(...)` allocate fresh default elements, as
	for ordinary PPL arrays. They do not preserve the previous contents.
- Existing host types can be fields and array elements. Record copies retain
	the host object's existing snapshot, live-view or shared-resource behavior;
	embedding an object grants no additional write access.
- Ordinary record and array data has copy-on-write value semantics: changing
	a nested field in a copy leaves the original data unchanged. Host state is
	not deep-copied. Use `VAR` for checked copy-back through record field paths.
- Fresh records, partial record literals and routine-local defaults contain
	empty dynamic fields and typed empty host values. Empty resources have
	`Valid = FALSE`; a blank controller does not silently attach to the live
	session. Obtain live controllers through the documented providers.
- `AUDIO` and `SURFACE` compare by allocation identity, including when held
	inside a record. Two empty resources of the same type compare equal. Other
	host types do not gain implicit equality: comparisons involving them or
	containing records are compiler errors. `CONTACT` retains value equality.
- Direct and indirect recursive record types remain forbidden, including
	recursion through dynamic arrays. No pointers or recursive types are added.

### Text, bytes and literal encoding

Ordinary `"..."` literals are Unicode text, not binary data. `STRING` and
`BIGSTR` hold Unicode scalar values (code points); no automatic Unicode
normalization is performed. Text operations count these values, not UTF-8
bytes, grapheme clusters (user-perceived characters), or terminal cells.

| Text | Code points | UTF-8 bytes |
| :--- | ---: | ---: |
| `€` | 1 | 3 |
| `界` | 1 | 3 |
| `e` followed by U+0301 (combining acute accent) | 2 | 3 |
| U+1F600 (a non-BMP character) | 1 | 4 |

The decomposed `e` plus U+0301 and the single code point `é` are different
strings under ordinal equality. `Reverse()` reverses code points, so it can
separate a combining mark from its base. `PadLeft()` and `PadRight()` pad to a
code point count, not a terminal width. Display width depends on terminal
handling of wide, combining and other Unicode sequences; `.Len()` is not a
layout measurement.

`BYTES` holds raw bytes. `TOBYTES(text)` encodes UTF-8; `bytes.ToString()` decodes
strict UTF-8, returning an empty string and `ErrKind.String` / `ErrCode.Format`
on invalid input. It does not guess CP437 or substitute replacement characters.
Numeric `TOBYTES` conversions retain their fixed-width little-endian format.

**Stored literal encoding follows the PPE target runtime, not the source
language version.** Below runtime 400 the existing CP437 format is unchanged.
From runtime 400 literals are stored as UTF-8 without a BOM and loaded strictly
as UTF-8. The existing `u16` byte length includes the final NUL: a literal can
contain at most 65,534 encoded bytes. Embedded NULs are preserved by the
length-delimited payload. This file limit does not limit language-400 `STRING`
values constructed at runtime. Larger literal lengths and the remaining
container changes are separate work.

**Recompile old 400-beta PPEs containing non-ASCII literals.** Their CP437 bytes
cannot reliably be distinguished from UTF-8 under the same version number.
New files require the updated loader; there is no heuristic CP437 fallback.
Malformed UTF-8, truncated literal payloads and missing final NULs are rejected.

Stored text encoding is independent of terminal encoding. UTF-8 caller
connections receive UTF-8. At an actual CP437 output boundary, including the
sysop monitor, each unrepresentable code point becomes `.`; the stored string
is unchanged. Virtual CP437 screens track the same substituted output.

### String members

At language 400 `STRING` is the string type and is not length-limited, so it is
used throughout. It has its own PPE type ID, separate from classic `STRING` and
`BIGSTR`. `BIGSTR` remains a deprecated legacy type limited to 2048 Unicode
code points; the compiler warns when it is written at 400.

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
| `Len()` | `INTEGER` | Number of Unicode code points (scalar values) |
| `Find(search [, start [, comparison]])` | `INTEGER` | First match at or after `start` |
| `FindLast(search [, start [, comparison]])` | `INTEGER` | Last match at or before `start` |
| `Contains(search [, comparison])` | `BOOLEAN` | Whether a non-empty search string occurs |
| `StartsWith(prefix [, comparison])`, `EndsWith(suffix [, comparison])` | `BOOLEAN` | Prefix or suffix test |
| `Count(search [, comparison])` | `INTEGER` | Non-overlapping occurrence count |
| `Equals(other [, comparison])` | `BOOLEAN` | String equality |
| `Replace(search, replacement)` | `STRING` | Replace every substring match |
| `Substring(start, length)` | `STRING` | Substring of `length` characters from zero-based `start` |
| `Left(count)`, `Right(count)` | `STRING` | Leftmost or rightmost `count` characters |
| `Trim([characters])` | `STRING` | Trim whitespace, or the supplied characters, at both ends |
| `TrimStart([characters])`, `TrimEnd([characters])` | `STRING` | Trim one end |
| `ToUpper()`, `ToLower()` | `STRING` | Change case |
| `PadLeft(width [, char])`, `PadRight(width [, char])` | `STRING` | Pad with a space, or `char`, up to `width`; unchanged if already that long |
| `Remove(start, length)` | `STRING` | Delete `length` characters from zero-based `start` |
| `Insert(index, value)` | `STRING` | Insert `value` at zero-based `index` |
| `Reverse()` | `STRING` | Reverse the characters |
| `ToInt([base])` | `INTEGER` | Parse as an integer, base 10 by default (2-36); `0` if invalid |
| `ToMixedCase()` | `STRING` | Title-case each word |
| `StripATX()` | `STRING` | Remove `@X` color codes |

Positions in the PPL 400 member API are zero-based Unicode code point positions;
`-1` means no match. Searches are case-sensitive. An empty search string is not
considered a match and has a count of zero. `Find` and `FindLast` are the
zero-based member forms of the classic `INSTR` and `INSTRR`, which remain 1-based
and return zero when no match is found. `Substring` is the zero-based member
alternative to the 1-based classic `MID`; `Left` and `Right` are count-based and behave
exactly like the classic functions. `Remove` and `Insert` use the same
zero-based positions. `ToInt` is the member form of the classic `S2I`. A single
character is also reachable through zero-based indexing (`text[0]`).

`Substring` retains `MID`'s padding rule: it returns the requested positive
number of code points, using spaces for positions outside the text. A
non-positive length returns an empty string. Indexing outside the text returns
an empty string; `Insert` clamps its index to the text bounds. `Remove` leaves
the text unchanged for a negative or out-of-range start or non-positive length,
and otherwise removes up to the requested number of code points.

`StringComparison.Ordinal` is the default. Pass
`StringComparison.OrdinalIgnoreCase` as the last argument for Unicode-aware,
case-insensitive searching or equality.

Pure ordinal comparisons preserve an earlier `Error.Last()` value, both with
the default overload and with explicit `StringComparison.Ordinal`. The fallible
`OrdinalIgnoreCase` operations clear an older error on success, but never an
error raised earlier in the same statement.

The comparison mode does not change overlap semantics: `FindLast` includes
overlapping occurrences and `EndsWith` tests the actual suffix; `Count` counts
non-overlapping occurrences. An insensitive comparison whose compiled literal
exceeds the regex engine's size limit reports `ErrKind.String` /
`ErrCode.Limit` instead of aborting execution. Its fallback is `-1` for
`Find`/`FindLast`, `0` for `Count`, and `FALSE` for the boolean comparisons.

Scalar strings support zero-based Unicode code point indexing in language 400.
`text[0]` returns the first character as a `STRING`; a negative or out-of-range
index returns an empty string. String arrays keep their normal array semantics,
and indexing can be chained: `words[0][0]` reads the first character of the
first string.

Operations that transform text return `STRING`. A language 400 `STRING` has no
length limit, so member chains do not truncate.

`StripATX()` removes only complete uppercase `@X` followed by two ASCII hex
digits. Incomplete or malformed tokens and other `@` text are preserved. This
is the modern member's rule; the classic `STRIPATX` opcode remains unchanged.

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
Both `text.Split(separator, limit)` and `STRING.Split(text, separator, limit)`
evaluate text, separator and limit exactly once, in that order.

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

`RegexOptions` members are `None`, `IgnoreCase`, `MultiLine`,
`DotMatchesNewLine`, `IgnoreWhitespace`, `SwapGreed` and `Ascii`. Combine them
with `|` and test them with `&` and `==` (or `!=`). There are only these seven
names; all combinations of their known bits are supported. Like every enum,
`RegexOptions` can also store unknown bits, but the regex API currently rejects
bits outside 0–5 with `ErrKind.Regex` / `ErrCode.Invalid`. No separate flags type
is needed. Matching is Unicode-aware unless `Ascii` is selected.

```PPL
RegexOptions options = RegexOptions.IgnoreCase | RegexOptions.MultiLine
IF (options.Has(RegexOptions.IgnoreCase)) THEN
	PRINTLN "Case-insensitive matching enabled"
ENDIF
```

`options.Has(mask)` tests whether all options in the mask are set, as does
`(options & mask) == mask`;
`(options & mask) != RegexOptions.None` tests whether any are set. Parenthesize
the bitwise expression because comparisons bind more tightly than `&` and `|`.
`options.Has(RegexOptions.None)` is always `TRUE`: an empty mask requires no
bits. Use `options == RegexOptions.None` to test that no options are set.
Positions, match collections and capture groups are zero-based. A missing match
or unmatched capture has start position `-1`. Group zero is the complete match.

`REGEXMATCH` exposes `Success`, `Value`, `Start`, `Length`, `GroupCount`,
`Group(index)`, `NamedGroup(name)`, and corresponding `GroupMatched`,
`GroupStart` and `GroupLength` methods. Named variants use the `Named` prefix.
`FindAll` returns a dynamic `REGEXMATCH[]` array. Access matches with
`matches[index]`; `matches.Len()` reports the number of matches.

`Find`, `IsMatch` and `FindAll` search at or after the zero-based Unicode
code point position `start`, even if it lies inside a match that would have
started earlier. The whole text still supplies the context for anchors and
word boundaries: `start` is not a new beginning for `^`. `FindAll` returns
non-overlapping matches and suppresses an empty match immediately following
the preceding match at the same position. Empty matches advance on Unicode
character boundaries and cannot make iteration stall.

Replacement strings expand `$1` and `$name`. A zero limit means unlimited;
negative limits report `ErrKind.Regex` / `ErrCode.Invalid`. `Split` preserves
empty fields and returns a dynamic `STRING[]`; on failure it returns an empty
array. `FindAll` results are limited to 100,000 matches and replacement output
to 16 MiB.

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
functions, procedures, tables, host objects or dynamic record fields. Unsupported
layouts are rejected even if an offending array is empty, before any record
bytes are read or written. Runtime usability does not imply serializability.

### Board objects

Board objects are read-only snapshots rather than custom records. They expose
the configured conferences, message areas, file directories and doors without
making a PPE parse Icy Board's TOML files. `Board` and `Session` are the way in:
one for what the board is configured to be, one for the call in progress. The
detailed member table follows below.

### Object lifetime and mutability

The label for each built-in object is part of its contract. A **snapshot** does
not change after it is returned, a **live view** reads current board/session
state, a **resource** remains usable until `Free()`/`Release()`/`Shutdown()` or
PPE cleanup, and a **value** is copied like an ordinary PPL value.

| Type | Lifetime | Mutability |
| :--- | :--- | :--- |
| `BOARD` | Snapshot created on first access; stable for the PPE run | Read-only |
| `CONFERENCE` | Configured-entry snapshot | Read-only |
| `AREA` | Configured-entry snapshot; message methods perform live I/O | Read-only |
| `DIRECTORY` | Configured-entry snapshot | Read-only |
| `DOOR` | Configured-entry snapshot | Read-only |
| `SESSION` | Live view of the active call | Read-only; mutate caller data through `Session.User` |
| `USER` | Live write-through view from `Session.User`; snapshot from `Board.Users` | Session user is writable where documented; board snapshots are read-only |
| `CONTACT` | Value record copied in contact-array snapshots | Record fields are writable on the local copy |
| `MSG` | Header snapshot; `Text()` loads the current stored body on demand | Read-only |
| `TERMINAL` | Live root for the caller's terminal | Read-only properties; methods change terminal state |
| `TERMINFO` | Connection-time snapshot | Read-only |
| `TERMINPUT` | PPE-owned input controller, released at cleanup | Mutable through methods |
| `EVENT` | Value returned by `Poll()`/`Wait()` | Read-only |
| `GFX` | Caller graphics-session controller | Mutable through `Init()`, `SetPacing()` and `Shutdown()` |
| `SURFACE` | PPE-owned graphics resource until `Free()` or cleanup | Mutable through drawing methods |
| `AUDIO` | PPE-owned channel resource until `Free()` or cleanup | Mutable through playback methods |
| `MARGINS` | Live terminal scrolling-region controller | Mutable through methods |
| `PALETTE` | Live terminal palette controller | Mutable through methods |
| `MACROS` | PPE-owned terminal macro controller | Mutable through methods |
| `HTTP` | Stateless factory/root | Static methods only |
| `HTTPREQUEST` | Shared request state until no PPL value names it | Mutable through `SetHeader()`, `SetText()`, `SetBytes()` and `SetForm()` |
| `HTTPRESPONSE` | Result snapshot from one completed request | Read-only; `Save()` performs output without changing the response |
| `REGEX` | Compiled-pattern value | Read-only |
| `REGEXMATCH` | Match-result value | Read-only |
| `ERROR` | Snapshot returned by `Error.Last()` | Read-only; `Error.Clear()` changes the VM's published error, not an existing snapshot |

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

`Terminal.Gfx.Backend` reports the selected `GfxBackend`.
`SetPacing(enabled)` controls whether presentation waits for a terminal
acknowledgement before sending another frame. It returns `TRUE` on success and
`FALSE` when no graphics session is active; `Error.Last()` provides the failure
details.

```PPL
IF !Terminal.Gfx.SetPacing(TRUE) PRINTLN Error.Last().Message
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

Copies of a surface share its allocation. After `Free()`, graphics reinitialization
or cleanup, old copies stay invalid: a new surface cannot revive them, even if
its numeric handle is reused. Stale copies cannot draw, present, serve as blit
sources or free a replacement. Comparing two stale copies still compares their
original identity; it does not make either copy valid.

`PresentRect` scaling and `GFX_FLIP_X`/`GFX_FLIP_Y` are JPEG XL features. Sixel
reports `ErrCode.Unsupported` for them.

Surfaces are limited to 2048 by 2048 pixels, 256 simultaneous surfaces and 64
MiB of resident RGBA pixels. Source image files are limited to 32 MiB. Graphics
and sound together may add at most 256 MiB of persistent media per connection.

#### Audio

`Audio.Load(file)` probes the format, uploads it to the caller's SyncTERM cache
and takes an available channel. A cached file is not sent again.

Audio copies share one allocation, not ownership of the channel number forever.
After `Free()` or cleanup, they remain invalid when another sound reuses that
channel and cannot play, stop, change volume or free the replacement.

```PPL
AUDIO music = Audio.Load("music.opus")
IF music.Valid THEN
	IF !music.SetVolume(50) PRINTLN Error.Last().Message
    music.Play(TRUE)
ENDIF
```

| Member | Purpose |
| :--- | :--- |
| `Valid`, `Playing`, `Channel` | Read-only state |
| `SetVolume(percent)` | Set playback volume and return whether it succeeded; failures update `Error.Last()` |
| `Play([loop])`, `Stop()` | Start or stop playback |
| `Fade(targetVolume, durationMs)` | Reach `targetVolume` over `durationMs` |
| `Free()` | Give the channel back |

Volume is a percentage and is clamped to 0 through 100; `Fade` takes the volume
first and the duration second. A duration of zero or less changes the volume at
once instead of over time.

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
`Audio`. `Kind` must be checked before reading kind-specific fields. A dash means
the field has no meaning for that kind and returns its neutral fallback (`0`,
`FALSE`, `""`, `MouseAction.None`, `MouseButton.None`, or `-1` for `Channel`).

| Field | `None` | `Key` | `KeyEdge` | `Mouse` | `Overflow` | `Audio` | Meaning |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| `Kind` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Event discriminator |
| `Time` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Monotonic connection time in milliseconds |
| `Code`, `Text` | — | ✓ | — | — | — | — | Translated Unicode/named key code and text |
| `ScanCode` | — | — | ✓ | — | — | — | Physical key code |
| `Pressed` | — | ✓ | ✓ | — | — | — | `Key` is a press; `KeyEdge` distinguishes press/release |
| `Repeated` | — | — | ✓ | — | — | — | Physical-key repeat flag |
| `Action`, `Button` | — | — | — | ✓ | — | — | Typed mouse action and changed button |
| `X`, `Y`, `Pixels` | — | — | — | ✓ | — | — | Mouse position and cell/pixel coordinate mode |
| `WheelX`, `WheelY` | — | — | — | ✓ | — | — | Mouse wheel delta |
| `LeftDown`, `MiddleDown`, `RightDown` | — | — | — | ✓ | — | — | Mouse buttons held at the event |
| `Shift`, `Alt`, `Ctrl`, `Meta` | — | ✓ | — | ✓ | — | — | Active modifiers supplied by translated keys or mouse reports |
| `Dropped` | — | — | — | — | ✓ | — | Number of queue entries lost before this event |
| `Channel` | — | — | — | — | — | ✓ | Finished sound channel |

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

A non-positive start coordinate or an end coordinate that is not greater than
the start is rejected with `FALSE` and `ErrKind.Term` / `ErrCode.Invalid`.
Successful set/reset operations clear an older error. These failures also
enter an installed `ON ERROR` handler.

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

`ErrKind` includes `None`, `File`, `DBase`, `Stack`, `Gfx`, `Font`, `Audio`,
`Term`, `Msg`, `Net`, `User`, `String` and `Regex`. `ErrCode` includes `Ok`,
`Unavailable`, `Invalid`, `Io`, `Format`, `Limit`, `Unsupported`, `Stack`,
`Denied` and `Timeout`. These are open nominal enums; retain a fallback for
unknown values.

Use `Kind` and `Code` when a PPE has to make a decision. `Message` may include
paths and operating-system text, and its wording may change between releases.

A successful fallible operation clears an older error, but cannot clear an
error still pending for the current VM statement. The first pending failure
wins over later failures, including those in nested function evaluation.
Later operands and their side effects still run; there is no rollback.
`Error.Clear()` explicitly clears both the published error and pending handler
dispatch. The value is a copy, so a PPE can keep one while it carries on:

```PPL
ERROR failed = Error.Last()
```

`FERR` and `DERR` are unchanged, including that `FERR` clears itself when read
and that `FGET` or `FREAD` reaching the end of a file raises it. Reaching the end
is not an error, so it leaves `Error.Last().OK` true and never reaches an
`ON ERROR` handler.

A search with no match or an empty handle's `Valid = FALSE` is likewise a
normal result. Trying to operate on an invalid resource is a failure. With no
handler installed, operational failures remain available through `Error.Last()`
and execution continues; they do not automatically become fatal errors.

Which member reports that depends on what is being asked, and the three are not
interchangeable:

| Member | Question | Where |
| :--- | :--- | :--- |
| `Valid` | Does this handle refer to something that exists? | Objects reached by number, index or lookup |
| `OK` | Was the answer itself good? | `ERROR`, and `HTTPRESPONSE` alongside `Valid` |
| `Success` | Did the search hit? | `REGEXMATCH` |

Objects a PPE simply has, such as `Board`, `Session` and `Terminal`, carry none
of them: no lookup happened that could have failed. An operation that either
works or does not returns `BOOLEAN` and leaves the detail in `Error.Last()`.

### ON ERROR

`ON ERROR` says where a failed operation sends the program. It may be written as
one word, `ONERROR`. GOSUB and procedure handlers stay armed; GOTO is disarmed
before the jump because its cleanup path has no natural return boundary.
The setting is VM-wide, not routine-local: returning from a routine that changed
the handler does not restore the previous setting. A nested PPE has its own VM.

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

Dispatch occurs after the invoking VM instruction completes, not at an inner
function's statement boundary. Thus an assignment receives its result before
its handler runs. A procedure-call instruction evaluates arguments and installs
the call frame before dispatch; a returning handler resumes at the procedure
body, and normal `VAR` copy-back occurs when that procedure returns. Structured
source statements may lower to several VM instructions. Fatal failures abort
execution without waiting for this operational-error boundary.

`ON ERROR` also catches the checked call-stack limit, which lets runaway
recursion report an operational `Stack` error instead of corrupting the VM.

### PPE cleanup

The BBS releases session-owned PPE resources when the outermost PPE finishes,
including ordinary completion, `EXIT`, `STOP`, fatal execution failure and a
detected disconnect. Nested PPE return and ordinary routine return do not release
the parent's resources. Cleanup releases input modes and events, graphics,
audio, terminal macros, synchronized updates and changed margins. PPE parameters
are cleared even when loading, execution or diagnostic output fails.

Terminal reset output is best-effort. Local cleanup continues after send
failures, but a disconnected terminal cannot be guaranteed to receive reset
sequences. Both caller-color restorations are attempted. An existing execution
or loading error remains the primary diagnostic if restoration or error display
also fails; successfully reported PPE failures retain the existing `FALSE`
completion result.

Detected input EOF ends timed and indefinite event waits and bypasses
`ON ERROR`; no later VM instruction runs. These guarantees apply to normal
asynchronous completion and detected session failures, not process crashes,
forced future cancellation or an undetected dead connection. Cleanup does not
execute user-defined handlers after a fatal VM failure.

`TRY ... CATCH ... FINALLY ... ENDTRY` is desired future work, deliberately
postponed while the PPL 400 release plan is completed. `DEFER` is also postponed.
Neither construct is available in this release step; `ON ERROR` remains supported.

> **Note:** icy_term does not read the slot argument yet, so a font it accepts
> applies regardless of which slot was named. SyncTERM uses the slot as written.

## Runtime 4.00

Runtime 4.00 is the PPE format Icy Board writes. Next to the PCBoard formats it
adds:

- a type table for `TYPE ... ENDTYPE` layouts
- a routine-reference marker for functions and procedures passed as values
- a record-literal opcode carrying type and field identifiers

A language 350 source lowers to an older compatible runtime; runtime 400 is only
needed once a source uses something the list above adds.

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
| `EchoOrigin` | `STRING` | The origin line, empty when the board wide one applies |
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
| `RequestPasswordRecovery(userName)` | `BOOLEAN` | Request recovery mail for a login name or alias |

```PPL
PRINTLN "Node ", Session.Node, ", ", Session.MinutesLeft, " minutes left"
PRINTLN "In ", Session.Conference.Name, " on ", Board.Name
```

Where a conference, area or directory sits is asked of the thing itself:
`Session.Conference.Number`, `Session.Area.Number`, `Session.Directory.Number`.

### Requesting password recovery

`Session.RequestPasswordRecovery(userName)` (language/runtime 400) requests a
temporary password using the existing recovery service and only the target
account's saved email address. Names and aliases match case-insensitively, with
surrounding whitespace ignored. It can be called from login, direct-PPE or
ordinary sessions, including sysop tools acting for another user. The PPE
controls access and dialogue; the function does not check caller permissions,
print anything, disconnect, switch users or authenticate anyone.

`FALSE` means recovery is globally disabled. Otherwise `TRUE` means the request
was accepted, **not** that mail was sent. Unknown/empty names, excluded accounts,
rate limits, persistence and SMTP failures have the same public result and do
not publish details through `Error.Last()` or enter `ON ERROR`; a completed call
clears an earlier error. Details are logged for the sysop. Existing target-account
exclusions and account/board limits still apply, with no extra per-connection cap.
Temporary-password verification and the mandatory password change remain in the
native login. A successful normal-password login cancels that account's pending
recovery, so a login PPE should end the connection after its confirmation when
requesting recovery for the caller.

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

Mutations are committed only when the user file is saved successfully. A save
failure restores both the live caller and the board's in-memory user record,
reports `ErrKind.User` / `ErrCode.Io`, and returns `FALSE` from mutating
methods. Property assignments publish the same error without changing their
old value. Invalid arguments report `ErrCode.Invalid`; a missing current user
reports `ErrCode.Unavailable`. Successful mutations clear an older error, but
never hide an error already raised within the same statement. Failures enter
an installed `ON ERROR` handler.

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
the text the way the board is configured to; an empty password is refused with
`FALSE` and `ErrCode.Invalid`.

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
is refused with `FALSE` and `ErrCode.Invalid`. A user may hold at most 100 contacts; a further
`AddContact()` is refused with `ErrCode.Limit`. `RemoveContact(index)` removes
the entry at the zero-based index and answers whether it succeeded; an invalid
index returns `FALSE` and `ErrCode.Invalid`. Reading an out-of-range index from
the `Contacts` snapshot instead answers with an empty `CONTACT`.

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

Binary file channels read and write `BYTES` without text conversion. `FREAD`
requires exactly the requested number of bytes; a short read stores an empty
blob and sets `FERR(channel)`. `FWRITE` writes the complete blob and pads it
with zero bytes when `size` is larger. `FDREAD` and `FDWRITE` provide the same
operations through the channels selected by `FDEFIN` and `FDEFOUT`.

```PPL
BYTES source = Bytes.FromBase64("AEEAf/8=")
FCREATE 1, "binary.dat", O_WR, S_DN
FWRITE 1, source, source.Len()
FCLOSE 1

BYTES target
FOPEN 1, "binary.dat", O_RD, S_DN
FREAD 1, target, 5
FCLOSE 1
PRINTLN target.ToHex()   ' 0041007FFF
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
size but does not retain another copy of the body; calling `Text()`, `Bytes()`
or `Save()` on that response reports `ErrCode.Invalid`.

`Text()` decodes the body strictly as UTF-8 and returns a `STRING`. A body in any
other character encoding reports `ErrKind.Net` with `ErrCode.Format`. `Bytes()`
returns the same body as `BYTES` without interpreting it, which is what an
image, an archive or any other binary answer needs:

```PPL
HttpResponse response = Http.Get("https://example.com/logo.png")
BYTES image = response.Bytes()
PRINTLN image.Len(), " bytes, SHA-256 ", image.GetChecksum(Checksum.SHA256).ToHex()
```

Use `Download()` or `Save()` instead when the body should reach a file without
passing through memory. Both `Text()` and `Bytes()` report `ErrCode.Invalid`
when the response never retained a body.

For a POST request or custom headers, build a request:

```PPL
HttpRequest request = Http.New(HttpMethod.Post, "https://api.example.com/items")
IF !request.SetHeader("Accept", "application/json") PRINTLN Error.Last().Message
IF !request.SetText(json, "application/json") PRINTLN Error.Last().Message
HttpResponse response = request.Send()
```

`SetQuery()`, `SetHeader()`, `SetText()`, `SetBytes()` and `SetForm()` change
the request and return `TRUE` on success.
On failure they return `FALSE`, leave the request unchanged, and publish details
through `Error.Last()`. `HttpMethod` contains `Get`, `Head`, `Post`, `Put`,
`Delete` and `Patch`.
`SetText()` needs a method that carries a body; on a `Get` or `Head` request it
reports `ErrKind.Net` with `ErrCode.Invalid`. Routing and hop-by-hop headers,
including `Host`, `Content-Length`, `Connection` and `Transfer-Encoding`, cannot
be set by a PPE.

`SetQuery(name, value)` replaces every query parameter with that decoded name,
preserves unrelated parameters and the URL fragment, and encodes the new name
and value automatically with RFC 3986 rules. `SetText()` sends its argument
verbatim, which is what JSON, XML and plain text need; do not URL- or
form-encode those bodies. `SetBytes(data [, contentType])` sends a `BYTES` value
without text conversion and defaults its content type to
`application/octet-stream`. Both body setters replace any existing body and
content type, and both reject `Get` and `Head` requests.

An `application/x-www-form-urlencoded` body is different: every value has
to be percent-encoded so that a `&` or `=` inside it cannot be mistaken for a
separator. `SetForm()` does that for one field and appends it to the body:

```PPL
HttpRequest request = Http.New(HttpMethod.Post, "https://api.pushover.net/1/messages.json")
request.SetForm("token", token)
request.SetForm("user", user)
request.SetForm("message", "SYSOP wants to chat")
HttpResponse response = request.Send()
```

Each call appends one `name=value` pair, encodes both sides, and sets
`Content-Type: application/x-www-form-urlencoded`. Repeated names are kept, as
an HTML form would send them. `SetForm()` refuses a `Get` or `Head` request, and
refuses to append to a body that `SetText()` wrote with a different content
type, so a form body and a JSON body can never be mixed by accident.

`Http.UrlEncode(text)` and `Http.UrlDecode(text)` handle one URL component with
RFC 3986 rules, where a space is `%20` and a `+` is literal.
`Http.FormEncode(text)` and `Http.FormDecode(text)` handle one
`application/x-www-form-urlencoded` field, where a space is `+`. `SetForm()`
already applies the form encoding automatically. Encode single values only,
never a whole `name=value&...` string, because the separators must stay
unencoded. Text is encoded as UTF-8 one byte at a time, so `ä` becomes
`%C3%A4`. Both decode functions replace byte sequences that are not valid UTF-8
rather than reporting them.

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
The source must be an array, not a scalar treated as a singleton. The target
must be a writable scalar, not an array, constant or routine. Ordinary scalar
assignment conversions apply; enum and record element types must match nominally.
The VM also checks these invariants before writing each element.

`BREAK` and `CONTINUE` work the way they do in every other loop. PPL arrays are
declared with upper bounds and index from zero, so `STRING names(10)` walks
eleven elements.

The collection expression is evaluated once and its array value is snapshotted
when the loop starts. Resizing or modifying the original array inside the loop
changes neither the visited values nor the number of steps.

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
and rank match, including ordinary variables declared with an initial upper
bound. Record array fields declared with bounds remain fixed in shape; dynamic
record fields (`[]`, `[,]`, `[,,]`) may adopt new bounds. Explicit dynamic declarations
such as `INTEGER values[] = { 1, 2 }` stay dynamic; `{}` initializes an empty
array. Local dynamic arrays start empty on each routine call and retain separate
storage across recursive calls. An array function that exits without assigning
its result returns an empty array, not the result of a previous call.

Array-returning functions and callbacks can be indexed directly at every rank:
`MakeVector()[i]`, `MakeMatrix()[i, j]`, `MakeCube()[i, j, k]`. The result and
indices are evaluated once, left to right. Modern record and array function
results belong to each invocation: a recursive call does not overwrite the
outer call's assigned result. Historical scalar function-result behavior is
unchanged.

Record array fields check the source's actual current shape, not just its
declaration. Assigning a resized incompatible array, including inside a record
literal or nested field, raises a runtime error before replacing the destination.

Routine parameters can also declare an array rank. In language 4.00 this
requires runtime 4.00; the argument must be an array with a compatible element
type and the same rank, not a scalar or one array element.

```PPL
PROCEDURE Inspect(INTEGER values[])
	PRINTLN values.Len()
ENDPROC

PROCEDURE ReplaceValues(VAR INTEGER values[])
	INTEGER replacement[] = { 7, 8 }
	values = replacement
ENDPROC
```

Value parameters receive independent array values (copy-on-write), including
the argument's current bounds. Array-returning calls, record array fields and
read-only array properties can be passed by value. Changing or redimensioning
the parameter does not change the caller's array. Parameters declared with
initial bounds, such as `INTEGER values[10]`, also adopt the argument's bounds;
they are not fixed-shape record fields.

`VAR` array parameters take writable array variables and copy their final value
and bounds back on return. As with existing `VAR` parameters, this is
copy-in/copy-out, not shared-reference aliasing: arguments are evaluated
left-to-right, and aliased parameters are written back in reverse parameter
order. The first parameter therefore wins when the same array variable is
passed more than once. Array-valued calls, read-only properties and record
temporaries are not writable variable arguments for `VAR`. Writable record
field paths are supported: dynamic fields can receive new bounds, whereas
fixed fields must still satisfy their declared shape at copy-out.

`VAR` targets are bound at their argument's position in the left-to-right
evaluation order. Indices are evaluated once, not again on return. Changing an
index variable in a later argument or inside the procedure does not redirect
copy-out. Record paths keep their selected indices and write into the caller's
current value, so unrelated fields are not replaced by a stale record snapshot.
Copy-out still validates the destination; binding a path does not permit
changing a fixed field's shape or writing through a read-only property.

Language 400 warns when two `VAR` arguments provably overlap, including the
same variable, equal constant indices, or a whole record/array and one of its
parts. This is a warning, not a ban. Dynamic indices are not assumed equal;
absence of a warning is not proof that targets are disjoint. Ordinary value
parameters do not participate. The editor diagnostic is `ppl.var-alias`.

Classic PPE runtimes below 400 copy out before restoring the routine's saved
frame, as original PCBoard does. In recursive calls, restoration can therefore
overwrite an inner `VAR` result targeting that routine's own parameter storage.
Runtime 400 retains its existing frame-safe rule: restore the caller's frame
first, then copy out the saved results. This runtime distinction also applies
to legacy-language source targeting runtime 400. Both use reverse parameter
order and once-bound indices; neither uses shared-reference parameters.

Whole-array formals carry PPE variable-header flag `0x04`
(`VARIABLE_FLAG_ARRAY_PARAMETER`), independent of static `0x01` and dynamic
storage `0x02`. **Recompile unreleased 4.00 programs using array parameters**:
unmarked formals now explicitly retain the classic calling convention. There
is no compatibility shim to infer which convention an older beta PPE intended;
see the [PPE format](ppe_format.md#entry-header--11-bytes).

Legacy formal arrays are different, even on runtime 400. An implementation
formal such as `INTEGER values(3)` retains rank 1 and upper bound 3 in its
header, without `0x04`; a scalar actual goes into element zero. Only that
element is saved/restored and copied back for `VAR`; the tail persists between
calls and across recursion. These execution rules are derived from PCBoard
source and covered by IcyBoard VM tests. Separate S3 original-runtime probes
now verify indexed target binding, reverse copy-out and recursive parameter
storage; they are not a runtime execution of all DECLARE probes.
Earlier N1–N4 notes must not be read as requiring legacy
headers to be flattened to `dim = 0`.

Below language 400, rank-2/3 implementation formals are rejected at the raw
dimension comma, even if the body does not use them. Multidimensional
`DECLARE` formals remain accepted and do not determine implementation shape;
ordinary multidimensional arrays are unaffected. The
[audit](../compat/DECLARE_AUDIT.md) distinguishes the original rank-2 compiler
probes from generated rank-3 parser coverage and source-derived runtime rules.

Existing 4.00 source that declares arrays with parentheses is
accepted with a migration warning. Square brackets are the canonical 4.00
notation. Formatting preserves authored delimiters, including legacy source;
it does not guess whether an unresolved `name(...)` is an array or a function.
Use the editor's array-bracket migration action for diagnosed legacy syntax.
Decompiler array output uses square brackets for language 400. Older language
versions retain classic parenthesis syntax.

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
line of its own the way `REDIM` does. In language 400 both forms require a
declared array variable and exactly one bound per declared dimension: they
change bounds, not rank. Computed arrays and read-only properties cannot be
redimensioned. Fixed record array fields cannot use either spelling of `REDIM`;
dynamic record array fields support both. `REDIM` allocates fresh default
elements and does not preserve old contents.
`Len()` reports the element count; after `Redim(20)`, it returns 21 and valid
indices are 0 through 20. `Redim(0)` creates one element, not an empty array.
Empty dynamic arrays report zero; assign an empty array of matching type and
rank to clear resizable storage. Bounds, rank and element count are distinct:
`INTEGER matrix[1, 2]` has rank two and six elements, with dimension counts two
and three. An empty return preserves its declared rank.

Record array elements can be assigned with square brackets, including nested
record paths and compound assignments such as `item.rows[0].value += 1`.
This does not make read-only object properties or string-character indices
writable.

In runtime 4.00, `SORT values, indices` produces exactly one index for every
element of the input vector. An empty vector produces an empty index vector;
there is no trailing default index. Existing pre-4.00 PPE behavior is unchanged.

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
name while compiling and retains its declared conversion and type. A decompiled
PPE therefore shows the value or its explicit conversion, never the constant name.

From language 400, a numeric constant initializer must fit its declared type.
For example, `CONST BYTE N = 257` is a compilation error because `BYTE` ranges
from 0 to 255; `CONST BYTE N = 255` is valid. Negative unsigned initializers,
integer-width overflow, nonfinite floating-point values and overflow of `REAL`
are rejected even for unused constants. Accepted fractional-to-integer conversions
retain the existing truncation behavior: `CONST INTEGER N = 1.5` becomes 1.
Dependent constants and module initializers use that converted value, not the raw
initializer. `CONST STRING` remains unbounded in language 400. Earlier language
versions and ordinary variable assignments retain their existing conversions.

Writing to a constant is an error. A constant, parameter and variable may not
share a name in the same scope, but a local declaration may shadow a global
constant or variable. A constant cannot be passed to a `VAR` parameter - there
is no variable to write back to.

`;$DEFINE` is the other way to name a value: it substitutes text before the
language is read, carries no type and works at any version. `CONST` is typed and
belongs to 3.50.

## `ENUM ... ENDENUM` Declaration (3.50)

### Function
Defines an open nominal integer type and its named values.

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
compared with each other. Equality and inequality are supported. `|` and `&`
perform bitwise integer operations when both operands have the same enum type;
the result retains that type, whether or not it has a declared name.
This is a general enum rule, not an exception for `RegexOptions`. Ordinary
integer/boolean `|` and `&` keep their existing logical behavior.

All enums accept every signed 32-bit integer value. If only `One = 1` and
`Two = 2` are declared, both `One | Two` (3) and `One & Two` (0) are valid.
There is no separate flags declaration: any enum can be used as a bitmask.
An explicit `Bits(64)` is valid even if bit 6 has no declared name. Unknown bits
are preserved, not stripped or mapped to an `Unknown` member.

Operands are evaluated once, left to right. Compound `|=` and `&=` retain the
same nominal type and permit unnamed results, just like ordinary `|` and `&`.
Arithmetic, unary negation and numeric `FOR` counters remain forbidden. These
rules apply uniformly from language 350 onward, including built-in enums.

All enum values also provide `Has(mask) -> BOOLEAN`, where `mask` must have the
same nominal enum type. It tests whether all bits in the mask are present in
the receiver. Receiver and mask are evaluated once, in that order; the method
does not modify either value. A zero mask always returns `TRUE`.

`Has` is equivalent to testing `(value & mask) == mask`, including unnamed
values and unknown bits. With only members `One = 1` and `Two = 2`,
`Bits.One.Has(Bits.Two)` returns `FALSE`; `(Bits.One | Bits.Two).Has(Bits.One)`
returns `TRUE`. Different enum types are not interchangeable. `Has` is available
from language 350 and requires runtime 400; constant-only calls may also be used
in `CONST` declarations.

An enum must have at least one member. Its **first declared member** is the
default, not necessarily zero or the smallest value. This includes scalar
variables, array elements, omitted record fields, fresh routine locals and
function result slots. Array allocation and resizing use the same default.
Aliases with the same numeric value are permitted.

Use `Color(number)` to explicitly convert an `INTEGER` to `Color`, preserving
the number even when no member names it. Non-integer arguments and other enum
types are rejected; integer assignment to an enum still needs the cast. Use
`TOINTEGER(color)` in the other direction. Arithmetic must operate on the
explicit integer representation. Untyped output statements such as `POP` and
`FREAD` cannot write directly into enums: read into an `INTEGER` temporary and
convert explicitly. Printing enum values continues to show their numeric value.

Typed record I/O supports enum fields (including nested records and arrays):
text records use decimal integers and binary records use signed little-endian
32-bit values. `FGetRec` and `FReadRec` preserve unnamed values and restore
their nominal type and default. Malformed integers, overflow and truncated
records still report a file-format error and leave the destination unchanged.

Host enums use exactly the same value rules as user enums. An older program can
store an unknown event or error value, pass it through arrays, records and
routines, and handle it in `CASE ELSE`. Accepting a value does not imply API
support: individual host operations reject unsupported modes, fields, methods
or option bits with their subsystem error before changing state. For example,
`RegexOptions(64)` is a valid value, but `Regex.Compile("x", RegexOptions(64))`
reports `ErrKind.Regex` / `ErrCode.Invalid`. Unknown `MouseMode`, `MouseTracking`
and `EditorMode` inputs report `Invalid`; unknown `GfxBackend` and `HttpMethod`
inputs report `Unsupported`. An unsupported graphics init leaves live surfaces
intact, and an unsupported editor mode leaves the user setting unchanged.

Enum variables, arrays, parameters, function results and record fields retain
their nominal type in runtime 400 PPEs, along with ordered member metadata.
Storage, explicit conversions and bitwise operations require runtime 400 even in language 350;
enum declarations and member constants alone can still target classic runtimes.
The decompiler reconstructs user enum declarations with synthetic type/member
names and preserves defaults, unnamed values and explicit conversions. Original
names are not stored. This replaces the earlier beta's closed-value checks;
programs must not rely on unknown numeric values causing a VM error. Deploy
with the updated runtime; earlier beta runtimes can still reject these values.
The existing metadata layout is unchanged. Stable host identity independent of
compact file-local type IDs and evolving member lists remains a C1/C2 task.

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
