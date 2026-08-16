## `AreaId()`  Function (4.00)

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

## `CONST` Declaration (4.00)

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
belongs to 4.00.

## `ENUM ... ENDENUM` Declaration (4.00)

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
