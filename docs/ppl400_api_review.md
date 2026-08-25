# PPL 4.00 API review

A review of the object API that PPL 4.00 adds on top of PCBoard 15.4, written
against the implementation rather than the documentation. It records what the
surface looks like now, which decisions are deliberate, and what is still worth
changing before the format is frozen.

Reviewed at commit `ab104390`. The classic PCBoard surface is out of scope: it
is frozen and its behaviour is a compatibility requirement, not a design choice.

## Summary

The 4.00 surface is a small language addition and one coherent object model.
Nearly everything a PPE reaches for goes through an object, and only four
statements were added to the whole language.

| Area | Assessment |
| :--- | :--- |
| Object model | Consistent. One root per concept, no duplicated paths |
| Naming | Consistent after the review round; counts, resets and record verbs agree |
| Error handling | One model (`Error.Last()`), applied everywhere, with three signalling styles |
| Capabilities | Single source of truth in `Terminal.Info` |
| Runtime versioning | One runtime, 4.00, with no beta holes |
| Legacy overlap | Deliberate and documented for session data |

The remaining items are policy decisions, not defects.

## The surface

### Roots

Three objects stand for themselves and need no parentheses:

| Root | Stands for | Lifetime |
| :--- | :--- | :--- |
| `Terminal` | The caller's terminal | Live |
| `Board` | The configured board | Snapshot taken when read |
| `Session` | The call in progress | Live |

`Terminal` groups the terminal by responsibility rather than exposing 30 globals:

| Member | Purpose |
| :--- | :--- |
| `Info` | Cached identity, dimensions and capabilities |
| `Gfx` | Graphics session and backend selection |
| `Input` | Keyboard, physical keys and mouse |
| `Margins` | Vertical and horizontal scrolling regions |
| `Palette` | The 16 DOS colours |
| `Font` | Font selection and uploads |
| `Macros` | Terminal-resident DEC macro slots |
| `BeginUpdate()`, `EndUpdate()` | Nestable synchronized output |

### Resources

`SURFACE` and `AUDIO` are constructed by static members on their own type, so
construction reads like construction:

```PPL
SURFACE screen = Surface.New(640, 400)
SURFACE image  = Surface.Load("logo.png")
AUDIO   music  = Audio.Load("music.opus")
```

Both answer an invalid object rather than failing the program, so `Valid` is
always safe to read. `Audio.StopAll()` is a static because it is not about one
sound.

### Board objects

`Board` and `Session` are the two ways in. Every board object reports the
`Number` it was fetched under and whether it exists:

```PPL
INTEGER i
FOR i = 0 TO Board.ConferenceCount - 1
	CONFERENCE conf = Board.GetConference(i)
	IF conf.HasAccess() PRINTLN conf.Number, " ", conf.Name
NEXT
```

`CONFERENCE`, `AREA`, `DIRECTORY` and `DOOR` all carry `Name`, `Number`, `Valid`
and `HasAccess()`.

### Typed families

Eight builtin enums replace what used to be bare integers: `EventKind`,
`MouseAction`, `MouseButton`, `MouseMode`, `MouseTracking`, `GfxBackend`,
`ErrKind` and `ErrCode`. They are nominal, so a number that means something else
cannot be compared against one.

## What is deliberate

These look like inconsistencies and are not. They are recorded here so they are
not "fixed" later by someone reading only the surface.

### Negative `None` values

`MouseButton.None` and `GfxBackend.None` are `-1`, while `EventKind.None`,
`MouseAction.None` and `ErrKind.None` are `0`.

The enum values mirror the numbers the runtime and the wire protocol already
use. Mouse button `0` is **Left**, and graphics backend `0` is **Auto**, so `0`
was taken in both cases. `GfxBackend` also has no `1`: it is reserved for a
future character-based backend.

### `KEY_*` and `GFX_FLIP_*` stay flat

Both would be enums if they could be.

- `Event.Code` also carries arbitrary Unicode, so key codes are not a closed set.
- `GFX_FLIP_X` and `GFX_FLIP_Y` are combinable flags, and PPL enums have no
  bitflag semantics by design.

These are the only remaining flat constants in the object APIs.

### Three ways an operation reports failure

| Style | Used by |
| :--- | :--- |
| `BOOLEAN` return | Operations |
| `Valid` property | Constructors |
| `Error.Last()` | Everything, with kind, code, message and channel |

Three is a lot, but each answers a different question: did this call work, is
this object usable, and what exactly went wrong. `Error.Last()` is the one that
always applies.

### Property assignment reports only through `Error.Last()`

`Terminal.Gfx.Pacing` and `Audio.Volume` are writable properties. An assignment
is not an expression, so a failed assignment cannot answer `FALSE`:

```PPL
Audio.Volume = 70
IF !Error.Last().OK PRINTLN Error.Last().Message
```

This is consistent with the rest of the API, where `Error.Last()` is
authoritative either way.

### Unknown capabilities are attempted

A capability boolean in `Terminal.Info` means confirmed support. An unknown
optional DEC mode still receives a standards-compliant request when an operation
is tried, and reports through `Error.Last()` if the terminal rejects it.

This is why capability state lives in exactly one place. A second property that
answered "available" for an unknown terminal would contradict `Terminal.Info`
for the same terminal.

### `Session` overlaps the classic functions

Seven `Session` members restate a frozen classic API:

| `Session` | Classic |
| :--- | :--- |
| `UserName` | `U_NAME()` |
| `SecurityLevel` | `CURSEC()` |
| `MinutesLeft` | `MINLEFT()` |
| `Node` | `PCBNODE()` |
| `IsLocal` | `ONLOCAL()` |
| `Language` | `LANGEXT()` |
| `PageLength` | `U_PAGELEN` |

This is a deliberate trade. The classic functions can never be removed, so the
overlap is permanent; in exchange a 4.00 PPE can read the whole session from one
discoverable root instead of remembering seven unrelated names.

## Decisions taken and still open

Nothing here blocks a freeze. Each is a judgement call that is cheaper to make
now than after the format is published.

### `TERMINFO` and `TERMINPUT` are abbreviated, their members are not

`Gfx`, `Margins`, `Palette`, `Font` and `Macros` are named exactly like the
member that hands them out. The other two are not:

```PPL
TERMINFO info = Terminal.Info
```

The obvious fix is worse than the problem: `Info` and `Input` are far too
generic as global type names. Recommended action is to accept this and say so in
the reference.

### `ERR()` was a function while the other singletons were not — resolved

`Terminal`, `Board` and `Session` need no parentheses, but `ERR()` did and
`ERRCLR` was a statement. The error API was the last part of 4.00 that spread one
concept over a function, a statement and a type.

Both are gone. `ERROR` now carries the two static members itself:

```PPL
ERROR failed = Error.Last()
Error.Clear()
```

`ON ERROR` stays a statement, because it is control flow rather than a value.
The `Err` function opcode and the `ErrClr` statement opcode were reclaimed rather
than reserved, so a beta PPE using them has to be rebuilt.

`FERR` and `DERR` are untouched: they are PCBoard's and keep their own behaviour.

### Counts and accessors instead of collections

`ConferenceCount` with `GetConference(index)` is honest, but it is still a count
plus an accessor:

```PPL
FOR i = 0 TO conf.DoorCount - 1
	DOOR item = conf.GetDoor(i)
NEXT
```

A collection or iteration concept would read better, but it is a language
feature rather than a facade change. The current naming is the right shape to
live with until that decision is made on its own merits.

### `Event` fields depend on `Kind`

`Code`, `ScanCode`, `Action`, `Channel` and `Dropped` each answer for one event
kind and report zero, or `-1` for `Channel`, otherwise. This is the intended
design and much better than the single overloaded field it replaced, but it is
the one place where a property's meaning depends on another property.

## Compatibility

Nothing in 4.00 changes the classic surface.

- PCBoard runtimes 1.00 through 3.40 are untouched, including their opcode
  numbers, argument shapes and the predefined user-variable prefix.
- `ConfInfo(conf, field)` and the `CONFINFO` statement remain PCBoard's.
- The classic session functions keep working unchanged.

Runtime 4.00 is the only runtime Icy Board writes. It carries the type table,
the routine-reference marker and `U_CONTACT`. The 4.01 and 4.02 beta formats no
longer load; those PPEs have to be rebuilt from source.

Type ids 38 and 48 are unused. They belonged to `TERMSTATE` and `Sound`, both
retired during the redesign.

## Verdict

The object model is sound and the naming is now self-consistent. The parts that
still look uneven are either forced by the wire format, forced by PCBoard
compatibility, or a documented trade.

With the error API folded into the `ERROR` type, every 4.00 concept is reached
through an object. What remains open is a matter of taste rather than shape, so
the surface can be frozen as it stands.
