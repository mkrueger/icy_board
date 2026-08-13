
# PPL

IcyBoard features a rewriten engine for ppl execution, a compiler and a decompiler.

Features:

* A compiler (pplc) that compiles UTF-8/CP437 files to output CP437 PPEs
* A decompiler (ppld) that decompiles all old .PPE files (up to PCBoard 15.4 PPEs) 
  - Reconstructs PPL all old statements 
* A language server that provides developer functionality in any editor that supports lsp
  - Included VS Code Extension (.vsix) for easy installation - just dnd it into the vs code extensions panel.

## What works

* Both compiler & decompiler is DONE. I would say it's better than everything we had back in the 90'.
Everything that doesn't work is a bug - please report issues.
* The decompiler should be able to decompile existing PPE files, including 4.x files with custom types, and handle the anti-decompilation tricks that were common in the 90s.
* Compiler should be able to parse a PPS and generate running PPE files
  - There are slight differences to PPLC - the new one is more strict. Issues should be easy fixable
  - Be prepared for tons of warnings of non trivial .PPS files. The old PPLC hasn't had much error checks. In doubt I added a warning instead.
* IcyBoard should be able to run most PPE files
  - PPE data files can be converted to UTF8 with (icbsetup ppe-convert <PATH>) but backup all files first
  - ppe-convert can take a <FILENAME> to convert the single file to UTF8
  - WARNING: Handle ppe-convert with care - can potentially destroy things. Convert one PPE after another.
  - No need to convert PPE - CP437 works, just consider that - I do it because no modern editor supports CP437 anymore.
* LSP should provide highlighting, help, find all refs/goto definition and a basic code completion 

### Decompiler

First Decompiler was based upon ppld. Find the original code here:
https://github.com/astuder/ppld

Much effort was done for implementing the decompiler. Existing PPEs may need to be altered for IcyBoard or at least analyzed so being able to decompile
the old PPEs is important for the project.

The current Decompiler is completely rewritten and uses a ppl machine language - which it can disassemble - to reconstruct a PPL AST.

* PPE 3.40 Support
* Full reconstruction of IF/THEN, SWITH, WHEN etc.
* It tries to do some name guessing based on variable usage.

```text
Usage: ppld [-r] [-d] [-o] [--check] [--cp437] [--style <style>] [--] <file>

PCBoard Programming Language Decompiler

Positional Arguments:
  file              file[.ppe] to decompile

Options:
  -r, --raw         raw ppe without reconstruction control structures
  -d, --disassemble output the disassembly instead of ppl
  -o, --output      output to console instead of writing to file
  --check           checks a .ppe file for compatibility with the current
                    runtime
  --cp437           write the source as cp437 instead of utf8, for use with the
                    original tooling
  --style           keyword casing style, valid values are u=upper (default),
                    l=lower, c=camel
  --help, help      display usage information
```

The dissamble output can be used to see what the compilers are generating and for debugging purposes.

### Compiler

Supports up to 15.4 PPL (1.0 -> 3.40 PPE format)

Should be compatible to the old PCB compiler with some slight differences (see PPL differences)

The compiler decides itself if uservars are generated or not (so --novars is no longer needed)

pplc has following options:

```text
Usage: pplc [-d] [--nowarnings] [--runtime <runtime>] [--lang-version <lang-version>] [--cp437] [--init] [--defines <defines>] [--format] [--check] [--] [<file>]

PCBoard Programming Language Compiler

Positional Arguments:
  file              file[.pps] to compile (extension defaults to .pps if not
                    specified)

Options:
  -d, --disassemble output the disassembly instead of compiling
  --nowarnings      don't report any warnings
  --runtime         version number for the compiled PPE, valid: 100, 200, 300,
                    310, 320, 330, 340, 400, 401 (default)
  --lang-version    language version, valid: 100, 200, 300, 310, 320, 330, 340,
                    350, 400 (default)
  --cp437           specify the encoding of the file (cp437 = true, utf8 =
                    false), defaults to autodetection
  --init            create & init new ppl package in target directory
  --defines         semicolon separated list of pre processor variables
  --format          formats source file instead of compile
  --check           checks source/package for errors without compiling
  --help, help      display usage information
```

As default the compiler takes UTF8 input - DOS special chars are translated to CP437 in the output.

Called without a file `pplc` builds the package described by `ppl.toml` in the
current directory. `pplc --init <dir>` creates one:

```text
mypkg/ppl.toml
mypkg/src/main.pps
```

```toml
[package]
name = "mypkg"
version = "0.1.0"

[compiler]
language_version = 400
```

Note:  All old DOS files are usually CP437 - so it's recommended to use --cp437 for compiling these.

#### PPL differences

The aim is to be as compatible as possible.

* Added keywords that are invalid as identifiers (but are ok for labels):
  ```LET```, ```IF```, ```ELSE```, ```ELSEIF```, ```ENDIF```, ```WHILE```, ```ENDWHILE```, ```FOR```, ```NEXT```, ```BREAK```, ```CONTINUE```, ```RETURN```, ```GOSUB```, ```GOTO```, ```SELECT```, ```CASE```, ```DEFAULT```, ```ENDSELECT```

I think it improves the language and it's open for discussion. Note that some aliases like "quit" for the break keyword is not a keyword but is recognized as 'break' statement. I can change the status of a keyword so it's not a hard limit - as said "open for discussion".

* Added ```€``` as valid identifier character. (for UTF8 files)
* Return type differences in function declaration/implementation is an error, original compiler didn't care.


## The PPL 4.0 language

PPL 4.0 is what IcyBoard's compiler targets by default. It is a superset of
PCBoard 15.4 PPL: everything the original compiler accepted still means the same
thing, and every addition sits behind a version number, so an old source keeps
compiling as an old source.

### Two version numbers

A PPE has a *runtime* version and a source has a *language* version. They are set
independently, because wanting new syntax and wanting a file an old board can load
are two different wishes.

| | Command line | `ppl.toml` | What it controls |
| :--- | :--- | :--- | :--- |
| Runtime | `--runtime` | `[package] runtime` | The PPE format written to disk. Valid: 100, 200, 300, 310, 320, 330, 340, 400, 401. |
| Language | `--lang-version` | `[compiler] language_version` | Which syntax and which built-ins the compiler accepts. Valid: 100, 200, 300, 310, 320, 330, 340, 350, 400. |

The runtime defaults to 401. The language defaults to the runtime version up to
400, so the default pair is runtime 401 and language 400. A format-only runtime
bump therefore does not invent a new language version. The command line wins
over `ppl.toml`.

Anything below is grouped by the language version that introduced it. A feature
listed under 350 is available at 350 *and* 400; a feature listed under 400 needs
`--lang-version 400`.

### Language version 350

3.50 is the "quality of life" version. It adds no new PPE format, so a 3.50
source can still be compiled down to an older runtime as long as it does not
call newer built-ins.

#### DECLARE is optional

`DECLARE FUNCTION` / `DECLARE PROCEDURE` before the implementation is no longer
required; the compiler reads every signature in the file before it compiles the
code, so a routine may be called before the file gets to it. Existing forward
declarations are still accepted, and a parameter count or return type that
disagrees between a declaration and its implementation is an error.

#### Variable initializers

```PPL
INTEGER count = 0
STRING  greeting = "Hello"
```

An array is initialized with a brace list:

```PPL
INTEGER values = { 1, 2, 3 }
```

which is shorthand for

```PPL
INTEGER values(3)
values(0) = 1
values(1) = 2
values(2) = 3
```

The brace list also decides the size, so the dimension is not written out.

#### Bracket indexing

`[` and `]` index an array:

```PPL
INTEGER values(10)
values[0] = 5
PRINTLN values[0]
```

Parenthesis indexing still works and still means the same thing. Brackets exist
because `values(0)` and a call to a function named `values` are written
identically, which the old language simply lived with. Brackets say which one is
meant, and they are the recommended form in new code.

#### Compound assignment

```PPL
count += 1
```

Available for `+` `-` `*` `/` `%` `&` `|`. `count += 1` is exactly
`count = count + 1`; there is no separate opcode.

#### REPEAT ... UNTIL

A loop with the test at the bottom, so the body always runs once:

```PPL
INTEGER n = 0
REPEAT
    n += 1
UNTIL n >= 3
```

#### LOOP ... ENDLOOP

A loop with no test at all, left with `BREAK`:

```PPL
LOOP
    n *= 2
    IF n > 10 BREAK
ENDLOOP
```

#### RETURN with a value

Inside a function, `RETURN expr` both sets the result and returns:

```PPL
FUNCTION Total(INTEGER v) INTEGER
    RETURN v + 1
ENDFUNC
```

which is the same as the old two-step form:

```PPL
FUNCTION Total(INTEGER v) INTEGER
    Total = v + 1
    RETURN
ENDFUNC
```

`RETURN expr` in a procedure is an error.

#### Parentheses are optional on IF and WHILE

```PPL
IF A <> B THEN
    ...
ENDIF

WHILE IsValid() PRINTLN "Success."
```

The old `IF (A <> B) THEN` still parses.

#### QUIT and LOOP are no longer aliases

At language version 350 and above, `QUIT` is no longer a synonym for `BREAK` and
`LOOP` is no longer a synonym for `CONTINUE` — `LOOP` is now a loop keyword of its
own. Sources that used the aliases need the modern spelling. They were rare in
practice.

#### Functions and procedures as parameters

A procedure or function can be declared as a parameter:

```PPL
PROCEDURE PrintHello(PROCEDURE f())
    PRINT "Hello "
    f()
ENDPROC
```

The parameter is callable inside the body. Passing `PrintHello(Hello)` checks the
complete signature: routine kind, argument types and dimensions, `VAR` flags and
the return type of a function. A routine parameter can be passed on to another
routine. Outside such an argument position a bare routine name is still an error.

Routine references need runtime 401 because 4.01 adds the bytecode marker that
distinguishes a routine value from a call to that routine.

### Language version 400

400 is where the language stops being bound by what PCBoard 15.4 could express.
A PPE built at runtime 400 or 401 will not load on an original PCBoard.

Runtime 400 introduced the IcyBoard-only format. Runtime 401 adds the type table
needed by custom types while keeping 4.00 files readable.

#### Parentheses, brackets and braces

400 gives each bracket kind one job:

| | Used for |
| :--- | :--- |
| `( )` | Grouping, call arguments, and array declarations |
| `[ ]` | Indexing |
| `{ }` | Array initializers |

Indexing with `( )` is still accepted for compatibility, but new code should
index with `[ ]`.

#### The dot operator and board objects

400 introduces the `.` operator and, with it, objects that describe the board
itself. The point is that a PPE no longer has to parse the board's config files
to find out what is on it.

```PPL
CONFERENCE conf = CONFINFO(CURCONF())

IF conf.HasAccess() PRINTLN conf.Name
```

`ConfInfo(conf)` returns a read-only `CONFERENCE` snapshot. An invalid conference
number returns an empty conference rather than failing, so its properties can
still be read.

**`CONFERENCE`**

| Member | Type | Description |
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

**`DIRECTORY`** and **`AREA`**

| Member | Type | Description |
| :--- | :--- | :--- |
| `Name` | `STRING` | Directory / area name |
| `HasAccess()` | `BOOLEAN` | Whether the current caller can access it |

**`DOOR`**

| Member | Type | Description |
| :--- | :--- | :--- |
| `Name` | `STRING` | Door name |
| `Description` | `STRING` | Door description |
| `Password` | `PASSWORD` | The door's password |
| `HasAccess()` | `BOOLEAN` | Whether the current caller can access the door |

Walking a conference:

```PPL
CONFERENCE conf = CONFINFO(CURCONF())
INTEGER i

FOR i = 0 TO conf.Doors - 1
    DOOR item = conf.GetDoor(i)
    IF item.HasAccess() PRINTLN item.Name
NEXT
```

Note that `CONFERENCE`, `DOOR`, `AREA` and `DIRECTORY` are resolved wherever a
type name is expected, so a variable cannot be called `door` or `area`. The names
are compared without regard to case, so this holds for a record type a program
declares too: `Point point` leaves `point` ambiguous.

These objects are read-only snapshots, so assigning to a member — `conf.Name = "x"`
— is rejected. What a member answers may be asked again, so
`conf.GetDoor(0).Name` reads in one go.

#### Overloaded built-ins

A built-in can now have more than one signature, chosen by argument count.
`CONFINFO` is the example: the original two-argument form returns a single field
whose type depends on which field was asked for, while the new one-argument form
returns a `CONFERENCE`. Old code keeps working unchanged.

`LEN` is overloaded the same way — `Len(array, dim)` returns the length of one
dimension.

#### New types

| Type | Declarable | Description |
| :--- | :--- | :--- |
| `MSGAREAID` | yes | A combined conference/message-area identifier, produced by `AreaId()` |
| `PASSWORD` | no | A password. Comparable against a string, but printing or converting one yields `******` instead of the secret. |

`PASSWORD` exists only at runtime; it is the type of `DOOR.Password` and cannot
be written in a declaration.

#### New library surface

| | Kind | Signature | Description |
| :--- | :--- | :--- | :--- |
| `ConfInfo` | Function | `ConfInfo(conf) : CONFERENCE` | Snapshot of a conference |
| `AreaId` | Function | `AreaId(conf, area) : MSGAREAID` | Addresses a message area in any conference |
| `Len` | Function | `Len(array, dim) : INTEGER` | Length of one array dimension |
| `WebRequest` | Function | `WebRequest(url) : STRING` | Fetches a URL and returns the body |
| `WEBREQUEST` | Statement | `WEBREQUEST url, file` | Fetches a URL and saves it to a file |

`AreaId()` is how message functions reach a message area outside the current
conference without breaking any of the old calls. `WebRequest` in both forms logs
and gives up after 30 seconds rather than holding the caller's node, and a failed
request answers an empty string / writes no file instead of stopping the PPE.

See [new_ppl.md](new_ppl.md) for the per-function reference pages.

#### TYPE ... ENDTYPE

A program can declare its own record types:

```PPL
TYPE Employee
    STRING  Name
    INTEGER Age, Level
ENDTYPE

Employee e

e.Name = "Sysop"
e.Age  = 42
PRINTLN e.Name, " ", e.Age
```

`END TYPE` may be written with a space, the way `END SELECT` may. Fields are
declared like variables, several to a line, and the type name is then usable
anywhere a built-in type name is.

A field is read and written with `.`, and takes the type it was declared with, so
a value assigned to it is converted the same way an assignment to a variable of
that type would be. Compound assignment works too:

```PPL
e.Age += 1
```

A record starts out with the empty value of each of its fields, and each variable
of a record type has fields of its own. A record is a value, not a reference:
two variables of the same type do not share anything. A record travels into a
routine and back out of a function like any other value, and a `VAR` parameter
writes back.

A field may be a record itself, as long as its type was declared first, and the
fields of that field are reached by carrying on with `.`:

```PPL
TYPE Address
    STRING Town
ENDTYPE
TYPE Member
    Address Home
ENDTYPE

Member m
m.Home.Town = "Kiel"
PRINTLN m.Home.Town
```

An array may hold records, including more than one dimension. Every element has
fields of its own:

```PPL
Member members(10)
members[0].Home.Town = "Kiel"
members[1].Home.Town = "Hamburg"
```

A named record literal creates a value without temporary field assignments.
Fields may appear in any order; omitted fields keep their empty value:

```PPL
Point origin = Point { X = 0, Y = 0 }
Point vertical = Point { Y = 10 }
RETURN Point { X = source.X + 1, Y = source.Y }
```

Unknown and duplicate fields are errors. A field holding another record requires
the exact nominal type. Record literals need runtime 401; the PPE stores type and
field ids rather than their source names.

The reverse shape - an array as one field of a record - is not supported yet.
`INTEGER Values(10)` inside a `TYPE` block is rejected explicitly because the
current PPE type table stores each field's type but not its dimensions.

Rules the compiler enforces:

* A type needs at least one field.
* Field names must be unique within the type.
* A type cannot contain a field of its own type, and can only name types that
  were declared before it, so a record cannot end up containing itself.
* Board objects such as `CONFERENCE` cannot be fields. They are runtime snapshots,
  not values with record copy and equality semantics.
* A type cannot reuse the name of a built-in or of a board object.
* A program may declare 156 types; ids 100–255 are reserved for them, leaving
  30–99 for board objects.
* A type may hold 255 fields; the PPE stores the count in a single byte.
* Naming a field the record does not have is an error, on both sides of an
  assignment.
* Custom types are nominal: two separately declared records are different types
  even when their fields happen to match. Assignments and routine arguments
  require the exact custom type.
* Equality compares two individual records of the same type by their fields.
  Whole arrays of records cannot be compared; index them first.

All `TYPE` declarations in a package are collected before its source files are
parsed, so `main.pps` may use a type declared in another file. Record fields still
follow declaration order: a record may only contain another record declared
earlier in the package.

Records are the one thing a PPE may assign a member of. The board objects are
read-only snapshots, so `conf.Name = "x"` is rejected.

`TYPE` and `ENDTYPE` are keywords only at language version 400, so a 3.50 source
may still have a variable called `type`.

The record layout is written into the PPE, which is why a program using `TYPE`
needs runtime 401. 4.00 has no type table - it was fixed before records existed.

Only the field types are written, not their names — the same as for variables,
routines and labels, none of which keep a name either. A shipped PPE therefore
carries no identifier from the source, and a decompiler has to invent them. See
[the PPE format](ppe_format.md) for the layout.

#### What 400 breaks

* Runtime 400 and 401 PPEs do not load on an original PCBoard.
* `.` is a token, so it can no longer appear in an identifier.
* `[` and `]` are index operators.
* A decompiled PPE names its records `TYPE001` and their fields `FIELD001`,
  because the file carries no names to recover.

### The preprocessor

The preprocessor is not tied to a language version — it works whatever `--lang-version`
is set to. Its directives are written as `;`-comments so that a source using them
still reads as a comment to any older tool.

#### Conditional compilation

| Directive | Meaning |
| :--- | :--- |
| `;$DEFINE name[=value]` | Defines a preprocessor variable |
| `;$IF expr` | Opens a block that is compiled only if `expr` is true |
| `;$ELSEIF expr` | Closes the preceding block and opens a new one on `expr` |
| `;$ELIF expr` | Same as `;$ELSEIF` |
| `;$ELSE` | Closes the preceding block and opens one for the case where no branch was taken |
| `;$ENDIF` | Closes the preceding conditional block |

Directive names are case insensitive. Blocks nest, and text in a branch that is
not taken is never lexed, so it does not have to be valid PPL. Only the first
branch whose condition is true is compiled.

An `;$IF` left open, or an `;$ELSE`, `;$ELSEIF` or `;$ENDIF` without a matching
`;$IF`, is an error. A `;$` word that is not a directive is treated as an
ordinary comment.

#### Predefined variables

| Name | Type | Value |
| :--- | :--- | :--- |
| `VERSION` | `STRING` | The `version` field from `ppl.toml` |
| `RUNTIME` | `INTEGER` | The PPE runtime version being written |
| `LANGVERSION` | `INTEGER` | The language version being compiled against |

More can be added with `;$DEFINE` or with `pplc --defines "A=1;B=2"`.

Because `VERSION` is a string, version comparisons want `RUNTIME` or
`LANGVERSION`:

```PPL
;$IF RUNTIME <= 340
    PrintLn "World"
;$ELSEIF RUNTIME < 200
    PrintLn "Old World"
;$ELSE
    PrintLn "New World"
;$ENDIF
```

#### Substitution tokens

`;#NAME` is replaced by the value of the preprocessor variable `NAME`, and a name
that was never defined is an error:

```
PrintLn "Version:", ;#Version
PrintLn "Runtime:", ;#Runtime
PrintLn "Language:", ;#LangVersion
```

Would print:

```text
Version:0.1.0
Runtime:401
Language:400
```

## Building & Running

* Get rust on your system <https://www.rust-lang.org/tools/install>

```bash
cd PPLEngine
cargo build -r
```

```bash
cd target/release
./ppld [PPEFILE]
./pplc [PPLFILE]
```
