# The PPE file format

A PPE is a compiled PPL program. There are two containers:

* The **legacy container**, written for runtimes up to 3.40. It starts with the
  text preamble `PCBoard Programming Language Executable` and is described from
  [Legacy container](#legacy-container) on.
* The **400 container**, written for runtime 400. It starts with the magic bytes
  `ICYPPE\0\0` and is described in [Runtime 400 container](#runtime-400-container).

The loader picks the container from these first bytes, not from a version field.
Both are read into the same in-memory program, so the VM and the decompiler do
not care which one a file came from.

All multi byte numbers are little endian. String literals use CP437 in the legacy
container and strict UTF-8 in the 400 container. The target PPE version selects
the encoding, independently of the source language version.

The authority for everything here is the code, not this page:
[container.rs](../crates/icy_board_ppl/src/executable/container.rs) for the 400
header and sections, [format400.rs](../crates/icy_board_ppl/src/executable/format400.rs)
for its payloads, [code400.rs](../crates/icy_board_ppl/src/executable/code400.rs)
for the 400 instruction encoding,
[imports400.rs](../crates/icy_board_ppl/src/executable/imports400.rs) for the
host ABI, [exec.rs](../crates/icy_board_ppl/src/executable/exec.rs) for the
legacy container, [variable_table.rs](../crates/icy_board_ppl/src/executable/variable_table.rs)
for the variable table and [crypt.rs](../crates/icy_board_ppl/src/crypt.rs)
for legacy encryption and packing.

## Runtime 400 container

The 400 container is a header, a section directory and the section payloads.
Every section has an explicit offset and length. The decoder checks section
bounds before interpreting payloads. The current `Executable::read_file` entry
point nevertheless reads the entire file into memory before decoding it;
the container layout is not a streaming-loader guarantee.

```text
+--------------------------------------+
| header                     64 bytes  |
+--------------------------------------+
| directory      48 bytes per section  |
+--------------------------------------+
| section payloads                     |
+--------------------------------------+
```

### Header — 64 bytes

| Offset | Size | Contents |
| ---: | ---: | :--- |
| 0 | 8 | `ICYPPE\0\0` |
| 8 | 2 | container major version, currently 1 |
| 10 | 2 | container minor version, currently 0 |
| 12 | 4 | header size, must be 64 |
| 16 | 4 | runtime, must be 400 |
| 20 | 4 | bytecode version, currently 1 |
| 24 | 8 | directory offset, must be 64 |
| 32 | 4 | section count |
| 36 | 4 | directory entry size, must be 48 |
| 40 | 4 | entry routine id; 0 means the implicit main program |
| 44 | 4 | reserved, must be zero |
| 48 | 8 | total file size, must match the file exactly |
| 56 | 8 | reserved, must be zero |

The four version numbers are deliberately separate. The container version covers
the header and directory, the bytecode version covers the instruction encoding,
every section carries its own schema number, and the host ABI has its own version
inside `IMPT`. A change to one of them does not force the others to move.

### Directory entry — 48 bytes

| Offset | Size | Contents |
| ---: | ---: | :--- |
| 0 | 4 | section kind, four ASCII bytes |
| 4 | 2 | section schema, currently 1 |
| 6 | 2 | flags; bit 0 marks the section as required |
| 8 | 4 | compression: 0 none, 1 Zstd |
| 12 | 4 | reserved, must be zero |
| 16 | 8 | payload offset |
| 24 | 8 | stored size |
| 32 | 8 | decoded size |
| 40 | 4 | entry count |
| 44 | 4 | reserved, must be zero |

A kind may appear only once. Payloads must lie behind the directory, inside the
file, and must not overlap — including the payloads of sections the loader does
not understand.

### Sections

| Kind | Required | Contents |
| :--- | :--- | :--- |
| `TYPE` | yes | record layouts and enum value lists |
| `CONS` | yes | typed constant pool |
| `VARS` | yes | variable table |
| `ROUT` | yes | function and procedure descriptors |
| `IMPT` | yes | host types and members the program uses |
| `CODE` | yes | instructions |
| `IDEN` | no | SHA-256 content identity |
| `DBUG` | no | variable names |
| `META` | no | reserved; recognised but carries no meaning yet |

**Unknown data is not silently dropped.** An unknown kind or an unknown schema is
rejected when the section is required, and kept verbatim when it is optional, so
writing the file back does not lose a future writer's data. An unknown
compression value is not a future section but a malformed one: compression is a
container-version-1 mechanic, so the whole file is rejected. `META` is reserved
for future metadata and is treated like any other optional section today.

### Compression

Compression is explicit per section, never inferred from a length mismatch. The
compiler writes uncompressed sections unless `--compression zstd` is given; the
header and directory are always plain so a file can be inspected without a
decompressor.

Zstd sections use one standard frame at level 3 with the content size and a
checksum in the frame, no dictionary, no concatenated or trailing frames, and a
window no larger than the loader's limit. A packed payload is only stored when it
is actually smaller than the raw bytes. The decoded size in the directory must
match what the frame produces.

### Content identity

`IDEN` holds a 32 byte SHA-256 digest over the runtime, the entry routine and the
six program sections, each with its kind, schema, flags, entry count and payload.

The digest deliberately covers the program and nothing else. Repacking a file,
stripping its debug names or adding an optional section therefore leaves the
identity valid, which is what makes those operations safe for a reader that only
knows this version of the format.

This is an integrity and identity check, not authentication. The format carries
no signature and no encryption, and a digest proves nothing about who wrote a file.

### Debug data

`DBUG` carries the names the runtime does not need: one per variable, then the
record and field names, then the enum and member names. It holds no source paths,
no source text and no line numbers.

Its structure always matches the program even when a name is unknown, in which
case the name is empty and the reader falls back to a generated one. `pplc` omits
the whole section unless `--debug` is given; a stripped and an unstripped build of
the same program have the same content identity and behave identically. With the
names present the decompiler reproduces the declared record, field and enum names
instead of inventing `TYPE001`.

### Section payloads

All counts and references are 32 bit. Text is length-framed UTF-8 without a NUL
terminator, so a literal may contain embedded NUL bytes.

`TYPE` — one record after another, then the enums:

```text
record: u32 kind = 1, u32 type id, u32 field count,
        per field: u32 type, u32 rank, u32 dynamic, u32 vector, u32 matrix, u32 cube
enum:   u32 kind = 2, u32 type id, u32 value count, i32 values...
```

Record ids are sequential from 100. A field may only name a primitive, an already
declared record or an imported host type; forward and recursive references are
rejected. The first enum value is its default; unknown values stay as they are.

`CONS` — `u32 type`, `u32 tag`, `u32 length`, payload. Tag 1 is an eight byte
scalar, tag 2 is UTF-8 text and tag 3 is a byte string.

`VARS` — nine `u32` per entry: type, rank, flags, the three bounds, storage kind,
function id and the one-based constant reference (0 for a default value).
Variable ids are implied by position, starting at 1.

`ROUT` — six `u32` per routine: variable id, parameter count, local count, start
instruction, first frame variable and result variable; then one `u32` per
parameter, 1 for `VAR` and 0 for by value. A start of `0xFFFFFFFF` marks a
callback parameter, which has no body of its own and is bound at run time.

The `VAR` modes are a per-parameter list, so runtime 400 is no longer limited to
the legacy 16 bit pass mask.

`IMPT` — `u32` ABI version, then per host type its id, kind, qualified name and
the members the program actually uses, each with id, name, kind, static flag,
result type, rank, required argument count and parameter types.

**Host binding is by name and signature, not by stored number.** On load the file's
host ids are matched against the current catalog by qualified name, and every used
member must still exist with the same kind, static flag, rank and result type.
The stored parameter types must match the current signature's prefix. A newer
host may append optional parameters or require fewer existing parameters, but
may not require more than the stored signature did. The reverse direction is
not guaranteed: a file built against an extended signature cannot bind to a
shorter old one. Ids may be renumbered and the catalog may grow; incompatible
changes are rejected before the program runs. Enum value lists are not part of
this contract, so new enum values do not invalidate old files.

`CODE` — one length-framed record per instruction, each starting with a `u32`
opcode. Jumps address instruction indices rather than byte offsets. Expressions
are a `u32` tag and a `u32` operand followed by the operands the tag needs.
Builtin calls are checked against their declared signature while decoding, so a
malformed call is refused before the VM sees it.

### Limits

The wire format uses 32 bit counts and 64 bit offsets. The limits below are
operating budgets that keep a corrupt or hostile file from allocating without
bound; they are not the widths the format can express.

These checks apply during container decoding. The initial whole-file read
described above happens first, so the file-size budget does not cap that
initial allocation. PPE files should be installed from trusted sources.

| Limit | Value |
| :--- | ---: |
| File size | 64 MiB |
| One decoded section | 32 MiB |
| All decoded sections | 64 MiB |
| Sections per file | 64 |
| Zstd window | 2^25 |
| Items in a section | 1,000,000 |
| Expression nesting | 96 |
| Records per program | 65,536 |
| Fields per record | 4,096 |
| Parameters per routine | 4,096 |
| Locals per routine | 65,536 |

Record field bounds are stored and held in memory as 32 bit values; only the
abandoned legacy type table was limited to 16 bit. The compiler measures a
program against the `CODE` section it actually produces, not against the byte
count of the old word encoding.

### Validation before execution

The loader checks the file before the VM runs a single instruction: header and
reserved fields, section bounds and overlap, sizes against the limits, then the
type graph, the constant representations, the variable and routine tables, and
finally the code — every variable, routine and record reference, argument counts,
array ranks, assignment targets and builtin signatures. A file that fails any of
these is refused; it does not run half way and stop.

### Compatibility

Pre-400 PPEs keep their container, their encryption and their execution semantics
unchanged. Unreleased beta 400 PPEs written in the old container are refused with
a message asking for a recompile; that break was decided explicitly rather than
guessed at from the file.

## Legacy container

Everything from here on describes the container used up to runtime 3.40.

## Layout

```text
+--------------------------------------+
| header                     48 bytes  |
+--------------------------------------+
| variable table             variable  |
+--------------------------------------+
| type table   (unreachable today)     |
+--------------------------------------+
| code size                   2 bytes  |
+--------------------------------------+
| code                       to EOF    |
+--------------------------------------+
```

There is no length field for the variable table and none for the code. The
variable table is read entry by entry until its declared count is reached, and
the code runs to the end of the file. A PPE therefore cannot be parsed backwards,
and a truncated file is only noticed when a read runs past the end.

## Header

48 bytes, plain text, never encrypted.

| Offset | Size | Contents |
| ---: | ---: | :--- |
| 0 | 39 | `PCBoard Programming Language Executable` |
| 39 | 2 | two spaces |
| 41 | 1 | major version digit |
| 42 | 1 | `.` |
| 43 | 2 | minor version digits |
| 45 | 3 | `0D 0A 1A` — CR, LF, DOS end of file |
| 48 | | end of header |

A file that does not start with the preamble is rejected.

The version is read back out of the digits rather than from a number field:

```text
version = ((buf[40] & 15) * 10 + (buf[41] & 15)) * 100
        + (buf[43] & 15) * 10 + (buf[44] & 15)
```

so `4.00` becomes `400` and `3.40` becomes `340`. This one number is what the
rest of the file is interpreted against — it decides whether entries carry two
filler bytes, whether the file is encrypted, and whether a type table follows.
The trailing `1A` means `TYPE FILE.PPE` under DOS stops at the header instead of
spraying binary over the screen.

## Variable table

```text
u16                 number of entries
entry               repeated, highest id first
```

Entries are written **in reverse**, from the highest id down to id 1. Ids are one
based; id 0 is not a variable.

Each entry is a fixed 11 byte header followed by a payload whose shape depends on
the type in that header.

### Entry header — 11 bytes

| Offset | Size | Field |
| ---: | ---: | :--- |
| 0 | 2 | id |
| 2 | 1 | dimensions, 0 to 3 |
| 3 | 2 | vector size |
| 5 | 2 | matrix size |
| 7 | 2 | cube size |
| 9 | 1 | variable type |
| 10 | 1 | flags |

The header is encrypted on its own, separately from the payload that follows it.
A dimension count above 3 is treated as corrupt and clamped to 3 rather than
trusted.

| Flag | Symbol | Meaning |
| :--- | :--- | :--- |
| `0x01` | `VARIABLE_FLAG_STATIC` | Static call-frame behavior for locals. |
| `0x02` | `VARIABLE_FLAG_DYNAMIC_ARRAY` | Runtime-400 dynamic storage: retain declared rank, initially allocate zero elements rather than the stored upper bounds. |
| `0x04` | `VARIABLE_FLAG_ARRAY_PARAMETER` | Runtime-400 whole-array formal: transfer the array value and its current bounds at calls. |

These flags are independent. A bounded language-400 array formal normally has
`0x04`; a dynamic array formal has `0x06`. Dynamic storage does not make a local
static, and it does not by itself select whole-array parameter passing.
Pre-4.00 runtimes do not interpret `0x02` as dynamic storage or `0x04` as the
modern array calling convention. The latter applies to ordinary array formals,
not function/procedure reference headers, whose dimension field encodes arity.

For marked array formals on runtime 400 or newer, value parameters receive an
independent array including current bounds; procedure `VAR` parameters also
copy back the final array and bounds. The procedure descriptor's `pass_flags`
still selects which arguments are `VAR`; header flag `0x04` does not replace
that bitmask.

**Unmarked formal arrays use the classic element-zero convention, including
on runtime 400.** Their implementation rank and bounds remain in the header;
they are not flattened to `dim = 0`. A scalar actual is assigned to element
zero, only that element is saved/restored and copied back for `VAR`, and the
tail persists between calls and through recursion. Source languages below 400
emit these unmarked formals even when targeting runtime 400. Rank or runtime
alone therefore cannot distinguish classic and whole-array parameters.
The [DECLARE audit](../compat/DECLARE_AUDIT.md) separates original-compiler
header evidence from these source-derived runtime semantics and IcyBoard tests.

**Recompile unreleased 4.00 PPEs using array parameters.** Earlier unmarked
whole-array formals cannot be distinguished from classic parameters, and no
compatibility shim guesses their intended calling convention. The unpublished
4.00 implementation also previously reused `0x01` for dynamic arrays; recompile
those affected beta PPEs as well. Neither correction changes classic PCBoard
PPE formats.

### Type byte

| Byte | Type | Byte | Type |
| ---: | :--- | ---: | :--- |
| 0 | `BOOLEAN` | 11 | `SBYTE` |
| 1 | `UNSIGNED` | 12 | `SWORD` |
| 2 | `DATE` | 13 | `BIGSTR` |
| 3 | `EDATE` | 14 | `DOUBLE` |
| 4 | `INTEGER` | 15 | `FUNCTION` |
| 5 | `MONEY` | 16 | `PROCEDURE` |
| 6 | `FLOAT` | 17 | `DDATE` |
| 7 | `STRING` | 18 | `TABLE` |
| 8 | `TIME` | 19 | `MSGAREAID` |
| 9 | `BYTE` | 20 | `PASSWORD` |
| 10 | `WORD` | 21 | `LONG` |
| 22 | `ULONG` | 23 | `BYTES` |
| 24 | `STRING` (4.00) | 255 | none |

IDs 21 through 24 are PPL 4.00 scalar types. Type 24 is unbounded Unicode text;
the source keyword remains `STRING`. IcyBoard's compact object range starts at
30 and includes board, session, user, messaging, terminal, media, HTTP and regex
types. IDs from 100 upward are records a program declares with `TYPE`. Anything
a reader does not know should be treated as an opaque user type rather than as a
broken file.

### Entry payload

**Function and procedure**

```text
u8 u8               two filler bytes, runtime < 340 only
u8                  type byte again
u8                  zero
u8                  parameter count
u8                  local variable count
u16                 start offset in the code
i16                 id of the first variable that belongs to it
i16                 function: id of the return variable
                    procedure: one bit per by reference parameter
```

The last field is the only place the two differ. A start offset of 0 was a known
trick to stop decompilers finding the body; IcyBoard reports it and leaves the
body inline instead of following it.

**String, no dimensions**

```text
u16                 byte length of the text including its terminator
bytes               text payload followed by NUL; encryption follows runtime
```

Below runtime 400 the payload remains CP437. From runtime 400 it is UTF-8
without a BOM. The maximum text payload is 65,534 bytes, regardless of code
point count. The final NUL is excluded from the decoded text; embedded NULs
are preserved. The loader also accepts a zero-length payload as an empty value.
For runtime 400, invalid UTF-8 and a missing terminator in a nonempty payload
are errors, as are truncated declared payloads.

This UTF-8 change was explicitly pulled forward from the container work on
2026-09-09. Old 400-beta PPEs with non-ASCII literals must be recompiled, and
new PPEs need the updated loader. Both use version 400, so the loader does not
attempt to identify old CP437 payloads heuristically. ASCII payloads are
unchanged. No legacy PPE encoding, length-field width or opcode changes here.

**String, with dimensions**

```text
u16                 zero
```

An array of strings stores no text. Its elements are built from the dimensions in
the header.

**Everything else**

```text
u8 u8               two filler bytes, runtime < 340 only
u8                  type byte again
u8                  zero
u32                 value, runtime 100
u64                 value, runtime above 100
```

The two filler bytes below runtime 340 are what PCBoard wrote and ignored. They
carry nothing and are only kept so old boards still read the file.

## Type table

This table only ever existed in the unreleased 400 beta, which used the legacy
container. Nothing is written for the PCBoard runtimes — they shipped before
records existed — and runtime 400 now uses `TYPE` in the 400 container instead.
The reader below is kept as documentation of that beta layout; a 400 file in the
legacy container is refused before it is reached.

```text
u8                  type-table format: 1 (records), 2 (records and enums)
u8                  number of record types
  u8                number of fields
    u8              field type
    u8              dimensions, 0 to 3
    u16             vector upper bound
    u16             matrix upper bound
    u16             cube upper bound
```

Format 2 appends the closed enum domains after the record layouts:

```text
u8                  number of enum domains
  u8                enum type id
  u16               number of domain entries (nonzero)
    i32             valid numeric value, default first
```

The first domain entry is the default for scalars, array elements, record fields and
routine-local/result resets. Duplicate numeric values (aliases) are allowed;
duplicate enum ids, empty domains and record/enum id collisions are rejected.
User declarations retain their member order and aliases. Built-in domains need
not have names for every value: `RegexOptions` stores 0–63 but exposes only
`None` and six individual options. There are no generated combination names.
Enum ids are retained in variable and field headers instead of being erased to
`INTEGER`. Assignments and checked casts validate membership before publishing
a value. Source language 350 and 400 share this representation; both require
runtime 400 for enum storage and checked conversion. Old beta PPEs with erased
enum storage need recompilation; their domains cannot be inferred from integers.

`EnumName(integer)` compiles to the internal `EnumCast(type_id, integer)`
function. The decompiler restores source casts and synthesizes user enum and
member names; built-in names are reused when the complete ordered domain matches.

Enum `a | b` and `a & b` compile to `EnumCast(type_id, BOR(a, b))` and
`EnumCast(type_id, BAND(a, b))`. Each nested operation has its own check, so an
invalid intermediate result cannot escape through a comparison or a later mask.
The ordinary logical bytecodes are unchanged. The decompiler restores bitwise
source operators when their operands belong to the expected enum domain.

`value.Has(mask)` uses the internal runtime-400 `EnumHas(type_id, value, mask)`
function (opcode -356). It evaluates the receiver and mask once, in that order,
checks their domain membership and returns the Boolean result of the numeric
test `(value & mask) == mask`. The numeric intersection need not be an enum
member. A zero mask always succeeds. The decompiler restores the instance method;
no additional enum-domain format is needed.

Both counts fit in a byte: ids run 100 to 255, so there can be no more than 156
types, and a record is capped at 255 fields for exactly this reason. The field
list is counted rather than terminated because there is no spare byte to end it
with - 0 is `BOOLEAN`.

Type *n* in this list is type id `100 + n`, which is how a `UserData(id)` in a
variable header finds its layout. A field that is itself a record simply carries
that record's type byte, so nesting needs no extra encoding.

Each field descriptor is eight bytes. Its three bounds have the same meaning as
the bounds in a variable header: ``Values(10)`` has dimension 1 and vector bound
10, so indices 0 through 10 exist. Bounds not named by the dimension count are
zero. The loader rejects dimensions above 3, nonzero inactive bounds and shapes
whose element count exceeds the runtime array limit.

The table stores field **layouts** and ordered enum domains, not source names:

* No type name and no field name. The format keeps no variable, routine or label
  names either — the decompiler makes those up. Custom types are treated the same
  way, so no source identifier reaches a shipped PPE.
* No initializer. Every element begins with its type's empty value; for enums
  this is the first declared member, not necessarily zero.

The table is written plain. It is not encrypted and not packed.

## Legacy limits

Unlike the 400 limits, these are not budgets a loader picked. They follow from
the field widths above and cannot be raised without changing the format.

| Limit | Value | Where it comes from |
| :--- | ---: | :--- |
| Code per program | 32,767 bytes | 16 bit code size field |
| Variable table entries | 32,767 | 16 bit entry count |
| Parameters per routine | 255 | one byte parameter count |
| `VAR` parameters per procedure | 16 | 16 bit pass mask |
| Locals per routine | 254 | one byte local count |
| Records and enums per program | 156 shared ids, 100 to 255 | one byte type id |
| Fields per record | 255 | one byte field count |
| String literal | 65,534 bytes plus terminator | 16 bit payload length |

## Code

```text
u16                 code size in bytes, before packing
bytes               the code, to the end of the file
```

The code is an array of `i16`. The size field holds the size **before** packing,
so comparing it against what is actually left in the file is how a reader learns
whether the code was packed:

Runtime 400's indexed-member expression stores a member id followed by a rank
and that many index expressions. The rank must be 1 to 3. A missing operand or a
rank outside that range is rejected as malformed bytecode before the VM runs it.

Computed matrix and cube indexing uses the internal runtime-400 functions
`ArrayValueAt2` (-357, array plus two indices) and `ArrayValueAt3` (-358,
array plus three indices). The rank-one `ArrayValueAt` keeps its original
opcode and argument count. These functions evaluate the array before each index,
once each, and require the matching runtime rank. No type-table format change is
needed; the decompiler restores bracket indexing and callback parameter signatures
when recoverable from concrete bytecode call sites.

Runtime 400 stores `FOREACH` structurally instead of lowering it to hidden
function calls and temporary variables:

```text
236 FOREACH       variable-id, collection-expression, end-byte-offset
237 NEXTFOREACH   body-byte-offset
```

`FOREACH` evaluates the collection once and creates a VM iterator frame.
`NEXTFOREACH` advances its flat row-major index and jumps to the body while an
element remains. The stored targets are byte offsets, like `GOTO` targets.
The collection must be an array and the target a writable scalar. Iteration uses
the normal checked assignment path, retaining enum domains and nominal record
types instead of replacing raw variable storage.

`BREAK` needs no opcode of its own: it compiles to a `GOTO` onto the loop end,
and any jump leaving the body discards the iterator frame.

```text
packed = (bytes remaining) != (code size)
```

There is no flag for it.

### Packing

A simple run length encoding, and only for zero bytes:

```text
00 nn               nn zero bytes, nn is 1 to 255
xx                  any other byte, as is
```

Zero runs are frequent because opcodes and operands are 16 bit and most values
are small, so every other byte tends to be zero.

Packing is only used when it actually helps and only from runtime 300 on. If the
packed form would be larger than the original, the original is written instead —
which is exactly why the reader has to compare sizes rather than trust a flag.

### Encryption

Runtime 300 up to but not including 400. Below 300 nothing is encrypted, and 400
is written plain again.

Encryption runs over chunks of 2047 bytes, each chunk on its own:

1. A rolling XOR against a 17 byte table, added to the remaining length — runtime 330 and up.
2. A 16 bit pass that rotates each word by a count derived from the previous word and XORs it against a seed starting at `0xDB24`. A trailing odd byte is handled separately.
3. The first byte XOR `'T'` — runtime 340 and up.

Decryption undoes these in reverse. Step 2 chains through the block, so a single
wrong byte ruins everything after it in that chunk.

The 2047 byte chunking has one wrinkle worth knowing: when the code is packed and
a chunk ends on a zero byte, the next chunk starts one byte later. Both sides
must agree on this or every chunk after the first goes wrong.

Note that the variable table is encrypted the same way, but always with the
chunking rule for unpacked data, whatever the code section does.

## Reading a legacy PPE

1. Check the preamble; refuse the file if it is missing.
2. Read the version out of the header digits.
3. Refuse version 400 — that is an obsolete beta and must be recompiled.
4. Skip to offset 48.
5. Read the variable table entry count, then that many entries, highest id first.
6. Read the code size.
7. Whatever is left is the code. If its length differs from the code size, it is packed.
8. Decrypt, then unpack, then read the result as `i16`.

## What the legacy container does not carry

Worth stating plainly, because all of it has to be reconstructed or invented when
decompiling. The 400 container addresses the first three points with optional
`DBUG` names, explicit section lengths and the `IDEN` digest.

* No names — not for variables, routines, labels, types or fields.
* No line numbers, no source file name, no comments.
* No checksum. A corrupt PPE is found by a read running out of bounds, not by a
  mismatch.
* No source spelling for field dimensions; only their rank and numeric upper
  bounds survive.