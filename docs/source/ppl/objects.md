# PPL 4.00 object and library API

PPL 4.00 provides typed access to the board, caller, messages, files and terminal,
plus HTTP, regular expressions and archive libraries. These APIs are **not**
just additional global functions: they have properties, instance methods,
static factories, value records, enums and resource lifetimes.

Use language **400** and runtime **400** when compiling these programs. They
are Icy Board extensions, not APIs available to a classic PCBoard runtime.
See [compiler target selection](../../pplc.md).

## Find an API

| Task | Types and member references | Behavior and examples |
| --- | --- | --- |
| Board identity and current call | [Board](api/board.md), [Session](api/session.md) | [Board and session](../../new_ppl.md#board-and-session-400) |
| User profiles, passwords, notes and contacts | [User](api/user.md), [CONTACT](api/contact.md) | [Caller records](../../new_ppl.md#the-caller-400), [notes and contacts](../../new_ppl.md#notes-and-contacts) |
| Conferences and configured content | [Conference](api/conference.md), [Area](api/area.md), [Directory](api/directory.md), [Door](api/door.md), [Bulletin](api/bulletin.md), [Survey](api/survey.md) | [Configured objects](../../new_ppl.md#board-objects-400) |
| Read, compose, reply to or edit messages | [Msg](api/msg.md), [MSGHEADER](api/msgheader.md), [MsgField](api/msgfield.md) | [Reading and visibility](../../new_ppl.md#messages-400), [message composition](../../new_ppl.md#composing-and-editing-messages) |
| Search and mark files | [FileEntry](api/fileentry.md), [FilePage](api/filepage.md), [Directory](api/directory.md) | [Paged searches](../../new_ppl.md#filebase-search-400), [download flags](../../new_ppl.md#marking-files-400) |
| Read or extract archives | [Archive](api/archive.md), [ArchiveOptions](api/archiveoptions.md), [ArchiveReader](api/archivereader.md), [ArchiveEntry](api/archiveentry.md) | [Limits, cursor and extraction](../../new_ppl.md#reading-archives-400) |
| Write ZIP archives | [Zip](api/zip.md), [ZipWriter](api/zipwriter.md) | [Defaults, entry options and publication](zip.md) |
| Terminal capabilities and output | [Terminal](api/terminal.md), [TermInfo](api/terminfo.md), [Margins](api/margins.md), [Palette](api/palette.md), [Macros](api/macros.md) | [Terminal information](../../new_ppl.md#terminal-information), [synchronized output](../../new_ppl.md#synchronized-output), [fonts](../../new_ppl.md#fonts) |
| Graphics and audio | [Gfx](api/gfx.md), [Surface](api/surface.md), [Audio](api/audio.md) | [Graphics](../../new_ppl.md#graphics), [audio](../../new_ppl.md#audio) |
| Keyboard, mouse and events | [TermInput](api/terminput.md), [Event](api/event.md) | [Input and events](../../new_ppl.md#input-and-events) |
| HTTP requests and downloads | [Http](api/http.md), [HttpRequest](api/httprequest.md), [HttpResponse](api/httpresponse.md) | [HTTP usage](../../new_ppl.md#http-objects-400), [operator policy](../../configuration/board.md#ppe-http-policy) |
| Regular expressions | [Regex](api/regex.md), [RegexMatch](api/regexmatch.md), [RegexOptions](api/regexoptions.md) | [Patterns, indexing and limits](../../new_ppl.md#regular-expressions) |
| Errors and cleanup | [Error](api/error.md), [ErrKind](api/errkind.md), [ErrCode](api/errcode.md) | [Error contract](../../new_ppl.md#errors), [ON ERROR](../../new_ppl.md#on-error), [PPE cleanup](../../new_ppl.md#ppe-cleanup) |

The [complete API index](api/index.md) includes **all properties, methods,
record fields and enum values**, not just the commonly used members.

## Obtaining objects

`Board`, `Session` and `Terminal` are root objects. Parentheses are optional:
`Board.Name` and `Board().Name` refer to the same root. A declaration alone,
such as `USER profile`, does not obtain the current caller. Use an explicit
source:

```PPL
;$LANGVERSION 400
USER profile
CONFERENCE conf

profile = Session.User
IF profile.Valid PRINTLN "Hello ", profile.Name

FOREACH conf IN Board.Conferences
    IF conf.HasAccess() PRINTLN conf.Number, ": ", conf.Name
ENDFOREACH
```

Compile this example with `pplc --lang-version 400 --runtime 400 example.pps`.
It requires a board session to execute; it is not a standalone userbase reader.

Other values come from properties, collections or factories:

- `Session.Area`, `Session.Directory` and `Session.Conference`: current context.
- `Board.Conferences`, then a conference's `Areas`, `Directories`, `Doors`,
  `Bulletins` or `Surveys`: configured collections.
- `Board.FindUser(name)` / `Board.AddUser(name)`: writable stored user handles.
- `Area.Read(number)` / `Area.Find(...)`: message snapshots.
- `Directory.Find(...)`: a page of file results.
- `Terminal.Info`, `.Input`, `.Gfx`, `.Margins`, `.Palette`, `.Macros`: terminal
  capabilities and controllers.
- `Surface.New(...)`, `Surface.Load(...)`, `Audio.Load(...)`: resources.
- `Http.New(...)`, `Regex.Compile(...)`, `Archive.Open(...)`, `Zip.Create(...)`:
  library factories.

An API signature headed `User.SetNote(...)` identifies the **owning type**;
call an instance method on a value, for example `profile.SetNote(...)`.
Only members marked **static** are called on the type itself. Properties are
read without parentheses. Functions return values; procedures do not.

In signatures, `[TYPE name]` means an optional trailing argument, while
`TYPE[]` means a one-dimensional array result. `TYPE[,]` and `TYPE[,,]` mean
two and three dimensions. Brackets describing optional arguments are not
literal call syntax.

## Snapshots, live handles and local values

The [lifetime and mutability contract](../../new_ppl.md#object-lifetime-and-mutability)
is as important as a member's type:

- **Snapshots** retain the state captured when returned. A saved `BOARD` value
  does not refresh its user list after a later user edit. Arrays are snapshots
  too: read the property again to see changes.
- **Live handles** follow the current session. `Session.User` writes through
  immediately where supported; `Board.FindUser()` and `Board.AddUser()` return
  writable stored records. Entries from `Board.Users` remain read-only even
  when their type has writable properties.
- **Value records** such as `CONTACT` and `MSGHEADER` are local copies.
  Editing `Msg.Header`'s returned copy does not edit the stored message. Pass
  a header to the appropriate session composition method.
- **Resources** have explicit cleanup operations. Release input, graphics and
  audio when finished; terminal resources also have PPE cleanup. Archive and
  ZIP handles have their own `Close`, `Finish` and `Abort` contracts.

A read-only property can return a mutable object: `Session.User` cannot be
replaced by assigning another user to that property, but supported fields on
the returned user can be changed.

## Permissions and failures

```{warning}
Obtaining a configured object does not establish caller access. Check the
relevant `HasAccess`, `CanEnter`, `CanDownload` or posting permissions.
In particular, `Area.Read`, `Area.Find` and `Msg.Text` do **not** reproduce
the interactive reader's privacy filtering or update its read pointers.
Read the [message visibility contract](../../new_ppl.md#who-may-see-a-message-is-the-programs-decision)
before exposing any message data.
```

`Valid`, `Ok` and `Success` are not interchangeable. A failed lookup can return
an invalid object without an operational error. A regex can be valid but find
no match. An HTTP response can be valid while its status is unsuccessful.
Check the result and the relevant [Error](api/error.md) contract.

`Error.Last()` is a snapshot of the published error. A later successful
fallible operation can clear an older error, so inspect it immediately.
`ON ERROR` dispatches at VM-instruction boundaries, not at every nested call
in an expression. Keep fallible operations separate when their failures need
different handling.

Archive access does not implicitly enforce file-area permissions, and archive
byte limits are not a general memory/CPU sandbox. HTTP is subject to the
operator's configured network policy. A documented API does not grant a PPE
permission to bypass either caller access or board policy.

## Value and collection libraries

PPL 4.00 also has member APIs on ordinary values, beyond the host-object
registry:

- [DATE, TIME and TIMESTAMP](../../new_ppl.md#calendar-dates-times-and-timestamps):
  constructors, parsing, components, formatting and arithmetic.
- [STRING](../../new_ppl.md#string-members): search, comparison, slicing,
  transformations, splitting, joining and conversions.
- [BYTES](../../new_ppl.md#the-bytes-type-400): byte buffers, text conversion,
  Base64, hexadecimal and checksums.
- [Arrays](../../new_ppl.md#array-members-400): dimensions, bounds, counts and
  resizing; [object collections](../../new_ppl.md#collections) are array values.
- [Enums](../../new_ppl.md#enum--endenum-declaration-350): nominal types,
  qualified members, bit operations and `Has(mask)`.
- [MSGAREAID / AreaId](../../new_ppl.md#areaid-function-400): conference/area
  identity, distinct from a message number or an array position.

## Complete API reference

```{toctree}
:maxdepth: 2

api/index
zip
```
