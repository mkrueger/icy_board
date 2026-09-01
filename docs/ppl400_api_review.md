# PPL 4.00 API review

> **Note:** This is the earlier sector-level review. The current, registry-backed
> itemized findings and pre-freeze recommendations are in the
> [PPL 4.00 API and language audit](ppl400_api_audit.md).

This reviews the API added by PPL language and runtime 4.00 on top of the
PCBoard-compatible surface. It is split by responsibility so each sector can
evolve without reopening unrelated parts of the API.

The classic PCBoard API is out of scope. Its names and behaviour are a
compatibility contract rather than a new design surface. There is currently no
runtime later than 4.00, so "4.00+" means the extensions introduced at 4.00.

Reviewed on 2026-08-27 against the compiler registry and opcode tables. The
object inventory can be reproduced with:

```sh
cargo test -p icy_board_engine --lib dump_api -- --ignored --nocapture
```

## Executive summary

The API has a coherent object model and is ready to keep stable. Its strongest
decision is ownership: board configuration, the current session, terminal
facilities, HTTP and errors each have one obvious entry point. Collections and
typed enums have removed most count/accessor pairs and untyped selector values.

| Sector | Public surface | Assessment |
| :--- | :--- | :--- |
| Language and runtime | Records, blocks, iteration, arrays, 64-bit values, routine references | Sound; the runtime boundary is explicit |
| Board and messaging | 10 types, 1 enum | Consistent snapshots and collections |
| Session and user | 6 types, 1 enum | Useful facade; legacy overlap is deliberate |
| Terminal and media | 10 types, 6 enums | Well grouped; lifecycle and event rules need care |
| HTTP | 3 types, 1 enum | Small, policy-aware and composable |
| Errors | 1 type, 2 enums, `ON ERROR` | One cross-sector model with documented signalling styles |

The registry contains 30 built-in object or record types and 11 built-in enums.
The function table adds 18 public 4.00 signatures, including the two `Rgb`
overloads; compiler-internal member, record and iteration opcodes are not API.

## Findings

### Documentation must remain registry-backed

The API changes often enough that a hand-maintained inventory becomes stale.
The previous review predated collections, messages, the user facade and HTTP,
and still described the retired `Terminal.Font` object. The registry dump is
the authority for names and signatures; this review explains design and should
not duplicate every member table from [New in PPL 3.50 and 4.x](new_ppl.md).

**Recommendation:** keep `api_dump::dump_api` as the review input and retain the
existing grammar/LSP registry checks. Add a generated machine-readable dump only
if another tool needs to consume the surface.

### Error signalling has three layers

Operations return `BOOLEAN`, resource constructors return an object with
`Valid`, and failures publish details through `Error.Last()`. This is more than
one convention, but each layer answers a different question:

| Signal | Question |
| :--- | :--- |
| `BOOLEAN` | Did this operation succeed? |
| `Valid` | Is this returned object usable? |
| `Error.Last()` | What failed, in which sector and channel? |

Fallible mutations use methods such as `Terminal.Gfx.SetPacing()` and
`Audio.SetVolume()` rather than property assignment, so they can return
`BOOLEAN` while publishing failure details through `Error.Last()`.

**Recommendation:** freeze this model. New fallible APIs should always publish
`Error.Last()` and also use the sector's local `BOOLEAN` or `Valid` convention.

### Some objects expose tagged-union state

An `EVENT` has a stable shape, but members such as `Code`, `ScanCode`, `Action`,
`Channel` and `Dropped` only carry meaning for particular `Event.Kind` values.
This is compact and avoids a type per event, at the cost of values whose meaning
depends on another property.

**Recommendation:** keep the shape stable and document the applicable `Kind`
beside every conditional member. Do not add more overloaded catch-all fields.

### Live values and snapshots must stay distinguishable

`Board` and its children are snapshots, `Session` and `Session.User` are live,
and `Terminal.Info` is an immutable connection-time snapshot. These lifetimes
are sensible but cannot be inferred from type syntax.

**Recommendation:** every new root or resource must state whether it is live,
snapshotted, cached or copied. Preserve the current lifetime when extending an
existing type.

## Sector 1: language and runtime

This sector owns the syntax and value model on which all later sectors depend:

- `TYPE ... ENDTYPE`, record literals and nominal record equality
- `BEGIN ... END`, `EXIT` and `ON ERROR`
- `FOREACH ... ENDFOREACH`
- array members `Len()` and `Redim()`, plus the flat `Len(array, dimension)`
- function and procedure references
- 64-bit `LONG` and `ULONG`, with `ToLong()` and `ToULong()`
- `AreaId()` for lossless conference/message-area addressing
- `Base64Enc()`, `Base64Dec()`, `BYTES.GetChecksum()` and the six `DOUBLE` math functions
- line-oriented `FGETREC`/`FPUTREC` and framed binary `FREADREC`/`FWRITEREC`
- discoverable `STRING` members plus static `Join`, `Repeat` and `Split`
- compiled `REGEX` matching with capture snapshots, match collections, replacement and transactional split

The public functions are intentionally a small utility layer. Member access,
static receivers and record literals have opcode entries but angle-bracketed
internal names and are excluded from completion and source. `FOREACH` uses
dedicated statement bytecodes rather than internal helper functions.

`LONG` is the one intentional source-compatibility edge: before language 4.00 it
was a 32-bit `INTEGER` alias, while at 4.00 it is a signed 64-bit type. The
language server migration rewrites old `ToLong(value)` calls to
`ToInteger(value)` when their old behaviour must be retained.

**Assessment:** freeze. Future language additions should not be placed in an
object sector merely to avoid adding syntax, and internal lowering opcodes must
remain absent from completion, hover coverage and grammar keyword lists.

## Sector 2: board and messaging

The sector contains `BOARD`, `CONFERENCE`, `AREA`, `DIRECTORY`, `DOOR` and
`MSG`, with `MsgField` as its typed selector enum. Collections are arrays of
these element types rather than separate wrapper types.

`Board` is the configured-board root. Conferences own their area, directory and
door collections; `Session` provides the current conference, area and directory.
This gives navigation one direction and avoids duplicated global accessors.

The four configured object types consistently expose `Name`, `Number`, `Valid`
and `HasAccess()`. A missing index returns an invalid object rather than an
exception. Permission questions are intentionally split:

- `HasAccess()` asks whether an item may be listed or entered at the first gate.
- `CanPost()`, `CanEnter()`, `CanAttach()` and `CanDownload()` ask about a
  specific action.
- Password properties use the masked `PASSWORD` type and never reveal the
  configured secret as text.

Message numbers are identities rather than collection indexes, so messages are
read through `Area.Read(number)` and searched with `Area.Find(...)`. Sparse and
deleted slots return an invalid `MSG`; storage or format failures additionally
set `Error.Last()` and enter `ON ERROR`. `LowMsg()`, `HighMsg()` and `Msg.Text()`
are functions because they perform I/O.

**Assessment:** freeze. Keep object misses non-exceptional, keep I/O as calls,
and do not turn messages into a collection whose index would imply position.

## Sector 3: session and user

The sector contains `SESSION`, `USER` and the built-in `CONTACT` record, with
`EditorMode` as its enum. Notes and contacts are exposed as
`STRING[]` and `CONTACT[]`; there are no separate `NOTES` or `CONTACTS` types.

`Session` describes the active call. `Session.User` describes the stored caller
record and writes through immediately. The apparent duplicates are deliberate:

| Active call | Stored user |
| :--- | :--- |
| `Session.SecurityLevel` | `Session.User.SecurityLevel` |
| `Session.PageLength` | `Session.User.PageLength` |
| `Session.Language` | `Session.User.Language` |
| `Session.UserName`, `AliasName` | `Session.User.Name`, `Alias` |

A conference may temporarily raise security, and a call may temporarily change
language or page length. Code should read `Session` for what is in force and
`Session.User` for what is stored.

User-owned mutable lists follow snapshot semantics. `User.Notes` returns a
five-element `STRING[]`, with mutation through `SetNote(index, text)`.
`User.Contacts` returns a stable `CONTACT[]` snapshot; `AddContact(service,
account)` appends and `RemoveContact(index)` removes by zero-based position.
Duplicate services are allowed. Mutation does not alter arrays already returned.

`Board.Users` is the board-wide `USER[]` snapshot. Each indexed item is a
read-only `USER` whose `Valid` property
distinguishes a real record from an out-of-range lookup. Its nested notes and
contacts belong to that snapshot. Only `Session.User` is writable.

**Assessment:** freeze. New call-only state belongs on `Session`; persistent
caller data belongs on `User`. Avoid adding a second path to the same stored
field unless PCBoard compatibility requires it.

## Sector 4: terminal and media

The sector contains `TERMINAL`, `TERMINFO`, `TERMINPUT`, `EVENT`, `GFX`,
`SURFACE`, `AUDIO`, `MARGINS`, `PALETTE` and `MACROS`. Its enums are
`EventKind`, `MouseAction`, `MouseButton`, `MouseMode`, `MouseTracking` and
`GfxBackend`.

`Terminal` is the single root and groups responsibilities instead of exposing
flat globals:

| Member | Responsibility |
| :--- | :--- |
| `Info` | Identity, dimensions and confirmed capabilities |
| `Input` | Keyboard, physical-key and mouse events |
| `Gfx` | Graphics backend and `SetPacing()` control |
| `Margins` | Horizontal and vertical scrolling regions |
| `Palette` | DOS palette entries |
| `Macros` | Terminal-resident macro slots |
| `SetFont()`, `LoadFont()` | Font selection and upload |
| `BeginUpdate()`, `EndUpdate()` | Nestable synchronized output |

`SURFACE` and `AUDIO` use static `New` or `Load` constructors and explicit
`Free` operations. `Audio.StopAll()` is correctly static because it targets all
channels. Capability booleans live only on `Terminal.Info`; an unknown optional
feature may still be attempted and report an operational error.

The abbreviated type names `TERMINFO` and `TERMINPUT` differ from the handing
members `Info` and `Input`. Global types named `INFO` and `INPUT` would be too
generic, so this asymmetry is preferable to renaming them.

`MouseButton.None` and `GfxBackend.None` use `-1`, while several other `None`
enum variants use `0`. These values mirror protocol/runtime values: mouse button
zero is `Left`, and graphics backend zero is `Auto`.

**Assessment:** freeze with targeted documentation. Keep capabilities on
`Terminal.Info`, state the event-kind applicability of conditional fields, and
preserve explicit resource release.

## Sector 5: HTTP

The sector contains `HTTP`, `HTTPREQUEST` and `HTTPRESPONSE`, with `HttpMethod`
as its enum.

`Http.Get()` and `Http.Download()` cover simple calls. `Http.New()` creates a
mutable request; `SetHeader()` and `SetText()` change it in place and return
`BOOLEAN`, and `Send()` produces a typed response. Failed changes leave the
request unchanged and publish details through `Error.Last()`. `Valid` reports
transport and body success, while `OK` means an HTTP status in the 200-299
range. This avoids turning a valid 404 response into a transport failure.

Security policy is board-owned rather than script-owned. Destination filtering,
DNS pinning, redirect checks, limits and the optional origin allowlist are not
mutable from PPL. Network, policy, TLS, size and file failures report
`ErrKind.Net`; HTTP statuses remain on the response.

**Assessment:** freeze the current small surface. Add methods only for common
HTTP semantics that preserve policy enforcement; do not expose transport knobs
that let a script weaken board configuration.

## Sector 6: errors

The sector contains `ERROR`, `ErrKind`, `ErrCode` and the `ON ERROR` control-flow
statement. `Error.Last()` reads the last operation result and `Error.Clear()`
resets it. Keeping both as static members avoids spreading one concept across a
global function, statement and type.

`Kind` identifies the owning sector, `Code` gives a portable category,
`Message` supplies detail, and `Channel` identifies a media channel when one is
relevant. `OK` makes the success path readable without comparing enum values.

Expected absence is not an error: a missing board index, message number or
search result returns an invalid object while `Error.Last().OK` remains true.
Operational failure returns the same fallback object and also records an error.
That distinction is essential for straightforward scans through sparse data.

**Assessment:** freeze. Each new sector should receive an `ErrKind` only when
callers need to distinguish it; prefer existing `ErrCode` categories over
sector-specific numeric details.

## Compatibility and freeze rules

- PCBoard runtimes 1.00 through 3.40 keep their opcode numbers, argument shapes
  and predefined variables.
- Language 3.50 features may target older runtimes unless they need routine
  references. Object members and records require runtime 4.00.
- Built-in IDs added for language 4.00 remain compact and may be renumbered
  until the format is released. PCBoard-compatible IDs stay frozen.
- Internal angle-bracketed members and opcodes are lowering details, not source
  API, and must remain hidden from editor completion.
- New members should follow the owning sector's naming, lifetime, failure and
  mutability conventions.

## Verdict

The 4.00 API is broad but not shapeless. Its sectors have clear owners, and the
few cross-sector conventions - collections, `Valid`, `Error.Last()` and typed
enums - are applied consistently. The remaining asymmetries are explained by
PCBoard compatibility, protocol values or PPL syntax rather than accidental
design.

The surface is coherent but remains unfrozen until the 4.00 format is released.
Future reviews should be sector-specific and use
the registry dump as their inventory, with a full review reserved for changes to
the shared error, lifetime, serialization or collection contracts.