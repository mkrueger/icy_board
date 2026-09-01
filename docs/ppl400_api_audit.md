# PPL 4.00 API and language audit

Reviewed on 2026-09-01 against the compiler registry, statement and function
definition tables, runtime registrations, tests, and the language reference.
This is a design review, not merely a feature list. Its standard is deliberately
conservative: modernize PPL where the old language is ambiguous or too limited,
but preserve PPL's procedural, statement-oriented character and PCBoard source
compatibility.

The classic PCBoard API is not rejudged here. Its behavior is a compatibility
contract. Language 3.50 additions are included because they are inherited by a
4.00 source and establish much of 4.00's syntax.

The object inventory was reproduced with:

```sh
cargo test -p icy_board_engine --lib dump_api -- --ignored --nocapture
```

## Verdict scale

- **GOOD** — fits PPL, composes with the rest of 4.00, and should be kept.
- **BAD** — should be changed before the 4.00 format/API is frozen.
- **GOOD, document** — design is sound, but its contract is too easy to misuse.
- **GOOD, compatibility risk** — worth keeping, but migration behavior must stay
  explicit.

“BAD” does not mean the feature should be removed. It means the present spelling,
contract, or duplication points the language in an avoidably different direction.

## Executive verdict

PPL 4.00 is recognizably PPL. It remains case-insensitive, procedural,
statement-oriented, permissive about omitted call parentheses, and centered on
board concepts. The strongest additions are typed board snapshots, `Session`,
records, enums, `FOREACH`, and a single terminal root. They remove magic numbers
and configuration-file parsing without replacing PPL with another language.

The design should **not be frozen unchanged**, however. Three items deserve a
pre-freeze decision:

1. Fixed arrays should keep classic `name(upperBound)` declarations as the
   canonical spelling. Square brackets are excellent for indexing and empty
   brackets are useful for dynamic arrays, but rewriting every fixed declaration
   to `name[upperBound]` needlessly makes PPL look like a different language.
2. The immutable `HTTPREQUEST` builder is inconsistent with every other mutable
   resource API. Either document it very prominently as a value builder or make
   its mutators return `BOOLEAN` and modify the request.
3. Reference documentation has drifted from the registry in several places.
   API generation or registry-backed checks should cover member signatures,
   return rank, enum values, and counts before freeze.

Current public inventory: **25 object/record types** (IDs 30–54), **14 built-in
enums**, **18 public global 4.00 function signatures** when overloads and root
accessors are counted, and **seven public 4.00 statement forms** (`ON ERROR`,
four record-I/O statements, `FOREACH`, and its source terminator). Internal
member, literal, static-receiver, indexed-member, and iterator opcodes are not
source API.

## Design rules that should govern 4.00

| Rule | Verdict | Reason / required action |
| :--- | :---: | :--- |
| Old runtime opcodes and old API behavior stay frozen | **GOOD** | This is the project's compatibility promise. |
| New syntax is selected by `;$LANGVERSION` | **GOOD** | Source breaks are contained rather than silently reinterpreted. |
| One root per responsibility: `Board`, `Session`, `Terminal`, `Http`, `Error` | **GOOD** | Prevents a second flat namespace of globals. |
| Board misses return an object with `Valid = FALSE` | **GOOD** | Natural for listings and sparse message bases. Operational errors remain distinguishable. |
| Fallible operations return `BOOLEAN` or an object with `Valid`, and publish `Error.Last()` | **GOOD** | The three signals answer different questions. Apply it consistently. |
| I/O and work are methods; cheap state is a property | **GOOD** | Explains `Area.HighMsg()` and `Msg.Text()` without surprises. |
| Collections are ordinary typed arrays | **GOOD** | Better than bespoke `Count`/`Get` wrapper types and works with `FOREACH`. |
| New indexes and positions are zero-based | **GOOD, document** | Matches PPL arrays, but differs from classic `INSTR`, `INSTRR`, and `MID`. Every API must say which convention it uses. |
| Live, snapshot, and resource lifetimes are explicit in documentation | **GOOD, document** | The type system cannot express this distinction. |
| New APIs prefer typed enums over selector integers | **GOOD** | A major usability gain that still lowers to PPL integers. |
| Internal angle-bracketed opcodes remain hidden | **GOOD** | Lowering machinery must not become accidental API. |

# Part I — Language constructs

## A. Language 3.50 constructs inherited by 4.00

| Construct | Verdict | Fit with PPL and improvement |
| :--- | :---: | :--- |
| Scalar initializer: `INTEGER n = 1` | **GOOD** | Removes a redundant assignment and still reads like a PPL declaration. Keep. |
| Array initializer: `INTEGER values = { 1, 2, 3 }` | **GOOD, document** | Concise and unsurprising. Fix documentation that expands three values to `values(3)`: classic upper-bound semantics would allocate four slots. |
| Bracket indexing: `values[0]` | **GOOD** | Resolves the old ambiguity between an array reference and a call while retaining parenthesis indexing for old source. |
| Compound assignment: `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=` | **GOOD** | Pure syntax lowering; compact without changing PPL's model. |
| `REPEAT ... UNTIL` | **GOOD** | Fills the missing post-test loop with conventional block syntax. |
| `LOOP ... ENDLOOP` | **GOOD, compatibility risk** | Useful infinite loop. It takes `LOOP` away from its obscure old `CONTINUE` alias; the version gate and migration warning must remain. |
| Optional `IF`/`WHILE` parentheses | **GOOD** | More PPL-like, not less: conditions read as statements rather than C expressions. |
| `CONST type name = value` | **GOOD** | Typed and scoped, yet erased during compilation. This is a conservative improvement over textual `;$DEFINE`. |
| `ENUM ... ENDENUM` | **GOOD** | Scoped nominal values eliminate selector-number mistakes. Keep equality-only semantics for ordinary user enums. |
| Routine parameters | **GOOD** | Checked callbacks extend existing `FUNCTION`/`PROCEDURE` concepts instead of inventing lambdas or closures. Runtime 4.00 dependency is justified. |
| Optional `DECLARE` | **GOOD** | This is compiler intelligence, not a new runtime direction. Explicit declarations remain valid and checked. |
| `RETURN expression` | **GOOD** | A direct spelling for the existing result-assignment-plus-return idiom. |

### 3.50 conclusion

The 3.50 layer is the best model for modernization: almost all of it lowers to
classic PPE instructions, and every construct repairs a concrete ambiguity or
omission. Do not add expression-bodied routines, closures, exceptions, classes,
or generalized generics merely because callbacks and records now exist.

## B. Language 4.00 declarations and source organization

| Construct | Verdict | Fit with PPL and improvement |
| :--- | :---: | :--- |
| Markdown `;;;` routine documentation | **GOOD** | Tooling metadata with no runtime effect. The semicolon spelling belongs in PPL. |
| `MODULE ... ENDMODULE` | **GOOD** | A declaration namespace is enough; correctly forbids executable module bodies. |
| `IMPORT module AS alias` | **GOOD** | Explicit qualification avoids global-name collisions. No wildcard imports or re-exports should be added. |
| `PUBLIC` / `PRIVATE` sections | **GOOD** | Section-oriented visibility fits old PPL better than modifiers on every declaration. Keep them context-sensitive. |
| Path and Git source dependencies | **GOOD, document** | Packaging is outside the language/runtime and enables reusable PPE source. Reproducible builds should recommend pinned `rev`; do not add runtime package loading. |
| `BEGIN ... END` main block | **GOOD, compatibility risk** | Gives routines and the main body a real boundary, but changes `END` from termination to delimiter. Keep the version gate and never reinterpret pre-400 source. |
| `EXIT` | **GOOD** | Necessary once `END` closes a block; maps to the classic terminating instruction. `STOP` remains the abort form. |
| Nested `BEGIN ... END` blocks | **GOOD** | A grouping construct only; it does not introduce a new scope model. |

## C. Records and nominal data

| Construct | Verdict | Fit with PPL and improvement |
| :--- | :---: | :--- |
| `TYPE ... ENDTYPE` records | **GOOD** | A natural extension of typed PPL variables. Value semantics are simpler and safer than references or classes. |
| Nested record fields | **GOOD** | Composition without object-oriented inheritance. |
| Fixed array fields | **GOOD** | Layout is explicit and serializable. Keeping their bounds immutable is correct. |
| Arrays of records | **GOOD** | Composes existing arrays with records; no second collection model. |
| Named record literals | **GOOD** | Checked field names and omitted-field defaults are clearer than positional constructors. |
| Nominal assignment/equality | **GOOD** | Prevents accidental structural matches. Do not add arithmetic or ordering to records. |
| Record values copied on assignment and parameter passing | **GOOD, document** | Fits existing scalar/array value behavior, but large values may surprise users. State copying costs in reference docs. |
| No self-recursive record layouts | **GOOD** | Required by the fixed PPE layout and keeps records simple. |
| Board objects forbidden as record fields | **GOOD** | Prevents live/resource handles from pretending to have record copy/equality semantics. |
| Record layouts stored without source names | **GOOD** | Consistent with historical PPE stripping identifiers; decompiler-generated names are acceptable. |

## D. Arrays and iteration

| Construct | Verdict | Fit with PPL and improvement |
| :--- | :---: | :--- |
| Dynamic vector declaration: `TYPE name[]` | **GOOD** | Empty brackets clearly mean “rank one, no current bound”; classic syntax had no clean spelling. |
| Dynamic matrix/cube: `name[,]`, `name[,,]` | **GOOD** | Compact extension of the same idea. |
| Fixed declaration rewritten as `name[10]` | **BAD** | PPL has always declared arrays with parentheses and uses an upper bound. Keep `name(10)` canonical for fixed arrays; reserve brackets for indexing and dynamic-rank declarations. Accepting brackets may remain a convenience, but the formatter/decompiler should not rewrite classic declarations. |
| Whole-array assignment for matching type/rank | **GOOD** | Required for functions returning arrays and snapshot properties. Value-copy semantics fit PPL. |
| Functions returning `TYPE[]` | **GOOD** | Directly supports `Split`, `FindAll`, and user routines without wrapper objects. |
| `a.Len()` total element count | **GOOD** | Clear for a flattened `FOREACH`; zero for an empty dynamic array is correct. |
| `a.Len(dim)` and `Len(a, dim)` | **GOOD, document** | Useful and PPL-style global/member duality. Emphasize that the result is a count although declarations use upper bounds. |
| `a.Redim(...)` and `REDIM a, ...` | **GOOD** | The member spelling composes with the array API while preserving the classic statement. |
| No redundant `Count()` member | **GOOD** | Avoids two names for the same array property. |
| `FOREACH item IN array` | **GOOD** | The loop variable is an explicitly declared copy, matching PPL's typed variables. |
| Multidimensional `FOREACH` flattened row-major | **GOOD, document** | Practical and deterministic. The order and copy semantics must remain prominent. |
| `BREAK`/`CONTINUE` in `FOREACH` | **GOOD** | Essential consistency with every other loop. |
| `IN` context-sensitive rather than globally reserved | **GOOD** | Preserves more old identifiers. |

## E. Scalar and text types

| Construct | Verdict | Fit with PPL and improvement |
| :--- | :---: | :--- |
| 64-bit signed `LONG` | **GOOD, compatibility risk** | Needed for JAM message numbers and counters. It changes the pre-400 alias meaning, so the LSP rewrite from old `ToLong()` to `ToInteger()` is part of the compatibility contract. |
| 64-bit unsigned `ULONG` | **GOOD** | Appropriate for cumulative byte/message counters. Avoid spreading unsigned arithmetic into APIs that do not need it. |
| Unbounded language-400 `STRING`; deprecated `BIGSTR` alias | **GOOD, compatibility risk** | Removes arbitrary truncation. Keep the alias for old source and keep the warning version-specific. |
| `BYTES` scalar blob | **GOOD** | A compact binary value is better than pretending `BYTE[]` is efficient binary storage. Its value semantics fit PPL. |
| Runtime-only masked `PASSWORD` | **GOOD** | Strong, focused safety improvement. It compares but cannot be declared, printed, or converted to reveal a secret. |
| `MSGAREAID` plus `AreaId(conf, area)` | **GOOD** | Solves multi-conference addressing without changing old message-call signatures. |
| String character indexing `text[index]` | **GOOD, document** | Arrays are already zero-based, and returning `""` for an invalid position is safe. State clearly that it is read-only and Unicode-character based. |
| Zero-based string member positions with `-1` for absence | **GOOD, document** | Internally coherent with arrays and regex, but unlike classic `INSTR`/`MID`. Do not change the legacy functions. |
| `StringComparison` enum | **GOOD** | Typed, last-argument comparison mode scales better than separate insensitive function names. |

## F. Control flow and failure handling

| Construct | Verdict | Fit with PPL and improvement |
| :--- | :---: | :--- |
| `ON ERROR GOTO label` | **GOOD** | Familiar BASIC/PPL-style recovery; disarming before the non-returning jump is correct. |
| `ON ERROR GOSUB label` | **GOOD** | Fits existing `GOSUB`/`RETURN` control flow. |
| `ON ERROR Procedure` | **GOOD** | Typed modern form without adding exception objects or `try/catch` syntax. |
| `ON ERROR OFF` | **GOOD** | Explicitly restores manual checking. |
| Handler receives `ERROR` or no argument | **GOOD** | Small and type-safe. Rejecting `VAR ERROR` is correct because the error is a value copy. |
| Failure inside a handler does not recursively invoke it | **GOOD** | Prevents runaway error recursion. |
| VM corruption/disconnection remains fatal | **GOOD** | `ON ERROR` should handle API failures, not make an invalid executable recoverable. |

# Part II — Global functions and statement APIs

## A. Public global function signatures added at 4.00

| Function | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `AreaId(conf, area) -> MSGAREAID` | **GOOD** | Narrow compatibility bridge for message APIs. |
| `Len(array, dimension) -> INTEGER` | **GOOD** | Extends a classic PPL function by arity; member spelling is equivalent. |
| `Base64Enc(value) -> STRING` | **GOOD** | Global functions are classic PPL style; retaining the member alias eases newer code. |
| `Base64Dec(value) -> BYTES` | **GOOD** | Correctly returns binary rather than assuming decoded UTF-8. |
| `Rgb(r, g, b) -> UNSIGNED` | **GOOD** | Compact color constructor; clamping makes it safe. |
| `Rgb(r, g, b, a) -> UNSIGNED` | **GOOD** | Arity overload is already supported and clearer than a second name. |
| `Board -> BOARD` | **GOOD** | Parameterless root reads naturally without parentheses, like a PPL predefined value. |
| `Session -> SESSION` | **GOOD** | Same root convention as `Board`. |
| `Terminal -> TERMINAL` | **GOOD** | Same root convention and one owner for terminal extensions. |
| `ToLong(value) -> LONG` | **GOOD, compatibility risk** | Correct 4.00 meaning; migration tooling for old 32-bit calls must remain. |
| `ToULong(value) -> ULONG` | **GOOD** | Conventional PPL conversion name. |
| `Sin(radians) -> DOUBLE` | **GOOD** | Global math functions match classic PPL's function style. |
| `Cos(radians) -> DOUBLE` | **GOOD** | Keep. |
| `Tan(radians) -> DOUBLE` | **GOOD** | Keep. |
| `Atan(value) -> DOUBLE` | **GOOD** | Keep. |
| `Log(value) -> DOUBLE` | **GOOD, document** | Specify natural logarithm and domain-error behavior. |
| `Sqrt(value) -> DOUBLE` | **GOOD, document** | Specify negative-input behavior through `Error.Last()`. |
| `ToBytes(value) -> BYTES` | **GOOD** | PPL-style global conversion and necessary for numeric binary representations. |

The compiler also has internal opcodes for member access/calls, static receivers,
record literals, indexed members, string operations, and array element access.
They are implementation details and must stay out of completion and reference
material.

## B. Public statement forms added at 4.00

| Statement | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `ON ERROR ...` / `ONERROR ...` | **GOOD** | The spaced form reads naturally; preserving the one-word spelling fits old PPL conventions. |
| `FGETREC channel, record` | **GOOD** | Extends the established channel-based file statements. Transactional reads are important. |
| `FPUTREC channel, record` | **GOOD** | Human-readable line format is useful for editable data. |
| `FREADREC channel, record` | **GOOD** | Framed binary format composes with repeated records on one channel. |
| `FWRITEREC channel, record` | **GOOD** | Fits classic binary file statement naming. |
| `FOREACH item IN array` | **GOOD** | High-level source construct with dedicated runtime iteration state. |
| `ENDFOREACH` / `NEXT` | **GOOD** | `ENDFOREACH` is unambiguous; accepting familiar `NEXT` is appropriately PPL-like. |

# Part III — Board, session, and message API

Every entry below is listed from the current registry. Equivalent, closely
related properties are kept in one row only where they have the same contract.

## `BOARD`

Lifetime: board snapshot, initialized on first use and stable for the PPE run.

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Name` | **GOOD** | Core board identity. |
| `Location` | **GOOD** | Core board metadata. |
| `Operator` | **GOOD** | Keep the distinction from the sysop display name documented. |
| `SysopName` | **GOOD** | Core board metadata. |
| `NodeCount` | **GOOD** | Typed replacement for configuration-file parsing. |
| `Conferences -> CONFERENCE[]` | **GOOD** | Natural root collection; array snapshot composes with `FOREACH`. |
| `Users -> USER[]` | **GOOD, document** | Useful but potentially expensive/sensitive. Document snapshot cost, access policy, and that returned users are read-only. |

## `CONFERENCE`

Lifetime: read-only snapshot. A failed lookup yields `Valid = FALSE`.

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Name`, `Number`, `Valid` | **GOOD** | This identity trio should remain common to configured board entries. |
| `IsPublic`, `IsReadOnly` | **GOOD** | Clear configuration facts. |
| `AllowAliases`, `EchoMail`, `AutoRejoin`, `PrivateUploads` | **GOOD** | Board-domain names; no artificial abstraction. |
| `Password -> PASSWORD` | **GOOD** | Masked type prevents accidental disclosure. |
| `Areas -> AREA[]` | **GOOD** | Correct ownership under the conference. |
| `Directories -> DIRECTORY[]` | **GOOD** | Correct ownership under the conference. |
| `Doors -> DOOR[]` | **GOOD** | Correct ownership under the conference. |
| `HasAccess()` | **GOOD, document** | Consistently means list/join at the first gate, not every possible action. |
| `CanPost()` | **GOOD** | Action-specific permission is clearer than overloading `HasAccess()`. |
| `CanAttach()` | **GOOD** | Same permission pattern. |

## `AREA`

Lifetime: read-only configured snapshot; message methods perform I/O.

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Name`, `Number`, `Valid` | **GOOD** | Consistent configured-object identity. |
| `IsReadOnly`, `AllowAliases` | **GOOD** | Relevant area policy. |
| `QwkName`, `EchoTag` | **GOOD** | PPL is a BBS language; exposing these domain concepts is appropriate. |
| `HasAccess()`, `CanEnter()`, `CanAttach()` | **GOOD** | Uses the shared list/action permission split. |
| `LowMsg()`, `HighMsg()` -> `LONG` | **GOOD** | Calls rather than properties correctly signal message-base I/O. |
| `Read(number) -> MSG` | **GOOD** | A message number is an identity, not an array index. |
| `Find(field, text [, start]) -> MSG` | **GOOD** | Typed `MsgField` modernizes `SCANMSGHDR` without replacing it. |

## `MSG`

Lifetime: read-only header snapshot; body is loaded by `Text()`.

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Valid`, `Number` | **GOOD** | Supports sparse/deleted message ranges without exceptions. |
| `From`, `To`, `Subject` | **GOOD** | Natural typed form of header selectors. |
| `Date`, `Time`, `ReplyTo`, `Status` | **GOOD** | Preserves board semantics without stringly typed access. |
| `IsPrivate`, `IsRead`, `IsDeleted`, `IsEcho`, `NeedsPassword` | **GOOD** | Named booleans are much clearer than status-bit inspection. |
| `Size` | **GOOD** | Lets a PPE decide whether to load the body. |
| `Text()` | **GOOD** | Function form makes lazy I/O visible. |

A missing/deleted message is normal absence and leaves `Error.Last().OK` true;
I/O or format failure returns the same fallback and also publishes an error.
That distinction is excellent and should be copied by future sparse-resource APIs.

## `DIRECTORY`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Name`, `Number`, `Valid` | **GOOD** | Consistent configured-object identity. |
| `Path` | **GOOD, document** | Useful for compatibility, but document whether it is host-native and whether scripts may expose it to callers. |
| `IsFree`, `HasNewFiles` | **GOOD** | Direct BBS concepts. |
| `Password -> PASSWORD` | **GOOD** | Safe masked access. |
| `HasAccess()`, `CanDownload()` | **GOOD** | Correct shared permission pattern. |

## `DOOR`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Name`, `Number`, `Valid` | **GOOD** | Consistent configured-object identity. |
| `Description` | **GOOD** | Appropriate listing metadata. |
| `Path` | **GOOD, document** | Same host-path disclosure warning as `DIRECTORY.Path`. |
| `Password -> PASSWORD` | **GOOD** | Safe masked access. |
| `HasAccess()` | **GOOD** | A door has one main action, so another `CanEnter()` would be redundant. |

## `SESSION`

Lifetime: live view of the active call; read-only as an object.

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Conference`, `Area`, `Directory` | **GOOD** | One obvious path to current board context. |
| `User` | **GOOD, document** | Stored caller data, live and writable, unlike `Board.Users` snapshots. |
| `UserName`, `AliasName` | **GOOD** | Active-call names can differ from stored user fields. |
| `SecurityLevel` | **GOOD** | Correctly reports effective call security, which may differ from stored security. |
| `Node`, `MinutesLeft` | **GOOD** | Natural replacements for remembered global function names. |
| `PageLength`, `Language` | **GOOD** | Correctly report active-call preferences. |
| `IsLocal`, `IsSysop` | **GOOD** | Clear call context. |

## `USER` and built-in `CONTACT`

`Session.User` is live and writes through. Entries returned by `Board.Users` are
read-only snapshots. `Notes` and `Contacts` return array snapshots.

| Member group | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| Identity: `Valid`, `RecordNumber`, `Name`, `Alias`, `VerifyAnswer` | **GOOD** | Clear grouping; read-only identity fields protect indexing/account ownership. |
| Address: `Street1`, `Street2`, `City`, `State`, `Zip`, `Country` | **GOOD** | Faithful modernization of stored caller data. |
| Contact channels: `BusinessPhone`, `HomePhone`, `Email`, `Web` | **GOOD** | Common fields remain convenient even with extensible contacts. |
| Personal data: `Gender`, `BirthDate` | **GOOD** | Existing user-record data, not a new language direction. |
| Sysop text: `Comment`, `SysopComment` | **GOOD** | Direct board terminology. |
| `Notes -> STRING[]`, `SetNote(index, text)` | **GOOD, document** | Explicit mutation preserves snapshot semantics. The fixed five-slot limit and zero-based index need to be prominent. |
| Preferences: `ExpertMode`, `EditorMode`, `ClearScreen`, `ScrollMessageBody`, `ShortDescriptions`, `LongHeader`, `WideEditor`, `PageLength`, `Protocol` | **GOOD** | Typed facade over legacy flags. |
| Session-owned preferences: `UseGraphics`, `UseAlias`, `Language`, `DateFormat` | **GOOD, document** | Their read-only status is non-obvious beside writable preferences; explain ownership. |
| Security: `SecurityLevel`, `ExpiredSecurityLevel`, `ExpirationDate`, `PasswordExpires` | **GOOD** | Range-checked write-through is preferable to raw record mutation. |
| `SetPassword(text)` | **GOOD** | Hashes according to board policy and refuses empty input; never exposes storage details. |
| Statistics: `TimesOn`, dates, message/upload/download counters, bytes, minutes | **GOOD** | `ULONG` preserves counters; keeping board-maintained statistics read-only is correct. |
| `Contacts -> CONTACT[]` | **GOOD** | Open service strings avoid freezing a social-network enum into the language. |
| `AddContact(service, account)` | **GOOD** | Direct, PPL-style mutator with `BOOLEAN` result. |
| `RemoveContact(index)` | **GOOD** | Stable zero-based list operation. |
| `CONTACT.Service`, `CONTACT.Account` | **GOOD** | Minimal built-in record; no wrapper object is needed. |

# Part IV — Terminal and media API

## `TERMINAL`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Info -> TERMINFO` | **GOOD** | Immutable capabilities have one home. |
| `Input -> TERMINPUT` | **GOOD** | Event-mode input is separated from classic line input. |
| `Gfx -> GFX` | **GOOD** | Graphics session state belongs under the terminal. |
| `Margins -> MARGINS` | **GOOD** | Focused DEC feature group. |
| `Palette -> PALETTE` | **GOOD** | Focused color group. |
| `Macros -> MACROS` | **GOOD** | Focused terminal-resident macro group. |
| `SetFont(font [, slot])` | **GOOD, document** | Function rather than `Font` property is right because terminals cannot report the selected font. Document client slot limitations. |
| `LoadFont(font, file)` | **GOOD** | Upload is visibly an operation. |
| `BeginUpdate()`, `EndUpdate()` | **GOOD, document** | Nestable synchronized output is useful; inactive `EndUpdate()` and cleanup behavior must remain specified. A scoped language construct would be excessive. |

## `TERMINFO`

Lifetime: immutable connection-time snapshot; reading it sends no terminal query.

| Member group | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| Identity: `Program`, `DeviceAttrs`, `RipVersion`, `Utf8`, `CTermLevel` | **GOOD** | Centralized protocol identity. |
| Text dimensions: `Columns`, `Rows` | **GOOD** | Basic terminal facts. |
| Pixel dimensions: `CellWidth`, `CellHeight`, `ScreenWidth`, `ScreenHeight` | **GOOD** | Necessary for pixel input/graphics; zero for unknown is pragmatic. |
| Graphics: `Sixel`, `Jxl`, `InlineGraphics`, `ClientBlit` | **GOOD** | Confirmed capabilities, not policy switches. |
| Input: `PixelMouse`, `PhysicalKeys` | **GOOD** | Correct capability owner. |
| Output: `Audio`, `SynchronizedOutput`, `TerminalMacros` | **GOOD** | Correct capability owner. |

The abbreviated type name `TERMINFO` with property `Terminal.Info` is acceptable;
calling the global type `INFO` would be too generic.

## `TERMINPUT` and `EVENT`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `MouseOn(mode, tracking)`, `MouseOff()` | **GOOD** | Explicit lifecycle and typed modes. |
| `KeyboardOn([keyUp])`, `KeyboardOff()` | **GOOD, document** | The optional boolean's meaning must be named in docs/completion; a typed mode would be excessive for one flag. |
| `Release()` | **GOOD** | One cleanup call for all event modes. |
| `Poll()` | **GOOD** | Conventional non-blocking event retrieval. |
| `Wait(milliseconds)` | **GOOD, document** | Zero=poll and negative=infinite are useful but should be stated at every signature location. |
| `Event.Kind` | **GOOD** | Required discriminator for the compact tagged event. |
| Key fields: `Code`, `ScanCode`, `Text`, `Pressed`, `Repeated` | **GOOD, document** | Compact and practical; document applicable `EventKind` for every field. |
| Mouse fields: `Action`, `Button`, `X`, `Y`, `Pixels`, `WheelX`, `WheelY`, held buttons | **GOOD, document** | Same tagged-event caveat. Do not add more generic fields with kind-dependent meanings. |
| Modifiers: `Shift`, `Alt`, `Ctrl`, `Meta` | **GOOD** | Shared across key and mouse events. |
| `Channel` | **GOOD, document** | Meaningful for `EventKind.Audio`, otherwise `-1`. |
| `Dropped` | **GOOD, document** | Meaningful for overflow only. |
| `Time -> UNSIGNED` | **GOOD** | Monotonic connection time avoids wall-clock problems. |

A tagged `EVENT` is more PPL-like than a hierarchy of event classes. It should
stay, but the kind-to-field applicability table is release-blocking documentation.

## `GFX` and `SURFACE`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Gfx.Init(backend [, fullscreen])` | **GOOD** | Typed backend, simple lifecycle, `BOOLEAN` result. |
| `Gfx.Backend` | **GOOD** | Reports the selected backend after `Auto`. |
| `Gfx.SetPacing(enabled)` | **GOOD** | Returns `BOOLEAN` and publishes failure details through `Error.Last()`, matching the shared fallible-operation convention. |
| `Gfx.Shutdown()` | **GOOD** | Explicit lifecycle operation. |
| `Surface.New(width, height)` | **GOOD** | Static constructor is clear. |
| `Surface.Load(file)` | **GOOD** | Static resource constructor with `Valid` fallback. |
| `Width`, `Height`, `Valid` | **GOOD** | Minimal resource state. |
| `Clear`, `SetPixel`, `GetPixel` | **GOOD** | Expected primitive surface operations. |
| `FillRect`, `DrawRect` | **GOOD** | Useful without turning PPL into a full graphics framework. |
| `Blit`, `BlitRect` | **GOOD** | Essential composition operations. |
| `Present`, `PresentAt`, `PresentRect` | **GOOD, document** | Flexible but `PresentRect` has nine integers and is error-prone. Document argument groups and consider a record/options form only in a later version, not another overload now. |
| `Pin`, `Unpin` | **GOOD, document** | Backend optimization belongs on the resource, but client-cache lifetime and memory cost need an example. |
| `Free` | **GOOD** | Explicit release matches bounded BBS-session resources. Cleanup must remain a safety net. |

## `AUDIO`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Audio.Load(file)` | **GOOD** | Static constructor with invalid-object fallback. |
| `Audio.StopAll()` | **GOOD** | Correctly static because it affects all PPE-owned channels. |
| `Valid`, `Playing`, `Channel` | **GOOD** | Minimal observable state. |
| `SetVolume(percent)` | **GOOD** | Returns `BOOLEAN` and publishes `ErrKind.Audio`, code, message and channel through `Error.Last()` on failure. |
| `Play([loop])`, `Stop()` | **GOOD** | Straightforward playback lifecycle. |
| `Fade(percent, milliseconds)` | **GOOD** | Useful operation with `BOOLEAN` result. |
| `Free()` | **GOOD** | Explicitly returns the bounded channel resource. |

## `MARGINS`, `PALETTE`, and `MACROS`

| API | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Margins.Top`, `Bottom`, `Left`, `Right`, `HasVertical`, `HasHorizontal` | **GOOD, document** | One-based inclusive terminal coordinates intentionally differ from zero-based arrays. |
| `SetVertical`, `SetHorizontal` | **GOOD** | Axis-specific names prevent coordinate confusion. |
| `ResetVertical`, `ResetHorizontal`, `ResetAll` | **GOOD** | Symmetric cleanup surface. |
| `Palette.Set(index, color)` | **GOOD** | Direct 16-color operation using packed `Rgb`. |
| `Palette.Reset(index)`, `ResetAll()` | **GOOD** | Symmetric restoration. |
| `Macros.Recording` | **GOOD** | Observable state is sufficient. |
| `BeginRecord(slot)`, `EndRecord()` | **GOOD** | Explicit statement-like lifecycle. |
| `Play(slot)`, `Delete(slot)`, `DeleteAll()` | **GOOD** | Small complete slot API. |

# Part V — HTTP API

HTTP is appropriately policy-controlled by the board. Scripts must not gain
proxy, DNS, redirect, TLS, private-network, or size-limit switches that weaken
sysop policy.

## `HTTP`, `HTTPREQUEST`, and `HTTPRESPONSE`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Http.Get(url)` | **GOOD** | Simple common case with typed response. |
| `Http.Download(url, path)` | **GOOD** | Atomic streaming avoids a second body copy and belongs as a convenience operation. |
| `Http.New(method, url)` | **GOOD** | Typed request construction; the method set is intentionally small. |
| `Request.Method`, `Request.Url` | **GOOD** | Useful immutable builder state. |
| `Request.SetHeader(name, value) -> HTTPREQUEST` | **BAD** | Returning a modified copy (`request = request.SetHeader(...)`) differs from mutable `Surface`, `Audio`, `User`, and other resource APIs. Before freeze, choose either explicit value-builder terminology (`WithHeader`) or in-place `SetHeader() -> BOOLEAN`. |
| `Request.SetText(text, contentType) -> HTTPREQUEST` | **BAD** | Same naming/value-semantics mismatch. `WithText` would make immutable behavior honest if value building is retained. |
| `Request.Send() -> HTTPRESPONSE` | **GOOD** | Natural terminal operation for the builder. |
| `Response.Valid` | **GOOD** | Transport/body success, distinct from status. |
| `Response.OK` | **GOOD** | Correctly means HTTP 2xx, so a valid 404 is representable. |
| `Status`, `FinalUrl`, `Size`, `ContentType` | **GOOD** | Minimal useful response metadata. |
| `Header(name)` | **GOOD** | Avoids exposing an awkward table/record collection solely for headers. Document duplicate-header joining behavior. |
| `Text()` | **GOOD, document** | Strict UTF-8 is safe, but callers need a binary path. The current answer is `Save`/`Download`; state this prominently. |
| `Save(path)` | **GOOD** | Useful for a retained body; correctly fails on streamed download responses that retain no body. |

The `HTTPREQUEST` issue is not an argument for classes or promises. It is a
small consistency choice: PPL methods named `Set...` look mutating everywhere
else.

# Part VI — Text, regex, bytes, and utility member APIs

## `STRING`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Len()` | **GOOD** | Discoverable member form of classic `LEN`. |
| `Find`, `FindLast` | **GOOD, document** | Zero-based/`-1` is coherent in the new API; contrast with classic functions. |
| `Contains`, `StartsWith`, `EndsWith` | **GOOD** | Clear predicates with comparison overloads. |
| `Count` | **GOOD** | Non-overlapping count is useful; empty-search behavior must stay specified. |
| `Equals` | **GOOD** | Mainly justified by `StringComparison`; ordinary `=` remains the simple case. |
| `Replace` | **GOOD** | Expected value transformation. |
| `Mid`, `Left`, `Right` | **GOOD, document** | Member `Mid` is zero-based while classic `MID` is one-based; this is the sharpest string compatibility edge. |
| `Trim`, `TrimStart`, `TrimEnd` | **GOOD** | Optional character sets avoid a proliferation of functions. |
| `ToUpper`, `ToLower` | **GOOD** | Conventional transformations. |
| `STRING.Join(array, separator)` | **GOOD** | Correctly static because no one string owns the array. |
| `STRING.Repeat(value, count)` | **GOOD** | Small useful utility. |
| `STRING.Split(text, separator [, limit])` and value member form | **GOOD** | Dynamic array result validates the array design. Retaining empty elements is deterministic. |

This is a substantial member-oriented layer, but it does not butcher PPL because
classic global functions remain and the members are discoverable equivalents.
Do not continue by moving every classic function behind a pseudo-object.

## `REGEX` and `REGEXMATCH`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Regex.Compile(pattern [, options])` | **GOOD** | Explicit compiled value is efficient and its `Valid` fallback matches resources. |
| `Regex.Escape(text)` | **GOOD** | Correctly static. |
| `Regex.IsValid(pattern [, options])` | **GOOD** | Useful when invalid input is expected and no object is needed. |
| `Valid`, `Pattern` | **GOOD** | Minimal compiled-value state. |
| `IsMatch(text [, start])` | **GOOD** | Simple predicate. |
| `Find(text [, start])` | **GOOD** | Typed snapshot result. |
| `FindAll(text [, start [, limit]]) -> REGEXMATCH[]` | **GOOD** | Ordinary array result; no collection wrapper. |
| `Replace(text, replacement [, limit])` | **GOOD, document** | Specify `$1`/`$name`, limits, and unsupported backreferences in the pattern separately. |
| `Split(text [, limit]) -> STRING[]` | **GOOD** | Composes with dynamic arrays and is transactionally bounded. |
| Match `Success`, `Value`, `Start`, `Length`, `GroupCount` | **GOOD** | Compact immutable result. |
| `Group`, `GroupMatched`, `GroupStart`, `GroupLength` | **GOOD** | `Matched` distinguishes an unmatched group from an empty capture. |
| Named group variants | **GOOD** | Symmetric with indexed groups. |
| Linear-time engine; no look-around/backreferences | **GOOD** | Predictable runtime is more important in a BBS script than maximal regex syntax. |

## `BYTES`

| Member/function | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `ToBytes(value)` | **GOOD** | Explicit scalar-to-binary conversion. Numeric little-endian representation must stay documented. |
| `value.ToString()` strict UTF-8 | **GOOD** | Clear member conversion when documented as decoding, not formatting. |
| `value.ToBase64()` / `Base64Enc(value)` | **GOOD** | Member and global bridge are acceptable; freeze one as canonical in docs. |
| `Bytes.FromBase64(text)` / `Base64Dec(text)` | **GOOD** | Same bridge. Avoid adding more duplicate spellings. |
| `value.ToHex()` | **GOOD** | Preserves leading bytes and is clearer than implicit display for program logic. |
| `value.GetChecksum(algorithm) -> BYTES` | **GOOD** | Raw bytes compose with hex/base64; `Checksum` makes algorithms discoverable. |

## Built-in enums

| Enum | Current values | Verdict | Improvement |
| :--- | :--- | :---: | :--- |
| `EventKind` | `None`, `Key`, `KeyEdge`, `Mouse`, `Overflow`, `Audio` | **GOOD** | Keep synchronized with the event applicability table. |
| `MouseAction` | `None`, `Press`, `Release`, `Motion`, `Wheel` | **GOOD** | Clear protocol vocabulary. |
| `MouseButton` | `None`, `Left`, `Middle`, `Right`, four wheel directions | **GOOD, document** | `None = -1` because `Left = 0`; do not normalize protocol values cosmetically. |
| `MouseMode` | `Text`, `Pixels` | **GOOD** | Clear. |
| `MouseTracking` | `Buttons`, `Drag`, `All` | **GOOD** | Clear. |
| `GfxBackend` | `None`, `Auto`, `Sixel`, `Jxl` | **GOOD, document** | `None = -1`; backend numeric values are runtime/protocol values. |
| `ErrKind` | `None`, `File`, `DBase`, `Stack`, `Gfx`, `Font`, `Audio`, `Term`, `Msg`, `Net`, `User`, `String`, `Regex` | **GOOD** | Sector categories scale adequately. Add one only when callers need to distinguish a sector. |
| `ErrCode` | `Ok`, `Unavailable`, `Invalid`, `Io`, `Format`, `Limit`, `Unsupported`, `Stack`, `Denied`, `Timeout` | **GOOD** | Portable categories are preferable to exposing OS codes. |
| `EditorMode` | `Yes`, `No`, `Ask` | **GOOD** | Preserves PCBoard's tri-state meaning better than names such as `Line`/`FSE`. |
| `MsgField` | `To`, `From`, `Subject` | **GOOD** | Values intentionally match `HDR_*` selectors. |
| `HttpMethod` | `Get`, `Head`, `Post` | **GOOD** | Small set matches the implemented body model. Do not add verbs without request-body and policy semantics. |
| `RegexOptions` | bit flags through `Ascii` | **GOOD, document** | Built-in combinable flags are an exception to ordinary nominal enum arithmetic. Explain why `|` is valid here but user enums are not general bitfields. |
| `StringComparison` | `Ordinal`, `OrdinalIgnoreCase` | **GOOD** | Small and explicit. |
| `Checksum` | `CRC32`, `MD5`, `SHA256` | **GOOD, document** | State that these are integrity/identity digests, not password hashing or authentication. |

# Part VII — Error API

## `ERROR`

| Member | Verdict | Fit and improvement |
| :--- | :---: | :--- |
| `Error.Last()` | **GOOD** | One cross-sector query is better than many unrelated globals. |
| `Error.Clear()` | **GOOD** | Explicit reset complements “successful operation clears” behavior. |
| `OK` | **GOOD** | Readable success test. |
| `Kind` | **GOOD** | Identifies owning sector without parsing text. |
| `Code` | **GOOD** | Portable control-flow category. |
| `Message` | **GOOD, document** | For logs only; wording and paths are not stable program logic. |
| `Channel` | **GOOD, document** | Relevant only to file/dBase/audio-style errors; otherwise `-1`. |

The overall three-layer convention should be frozen:

1. `BOOLEAN` answers whether a mutating operation succeeded.
2. `Valid` answers whether a returned object/resource can be used.
3. `Error.Last()` explains the last fallible operation.

Expected absence is not failure. This works especially well for invalid board
indexes and sparse messages. The one unresolved inconsistency is property
assignment: it cannot provide layer 1. Resolve or explicitly bless that exception
before freezing writable object members.

# Part VIII — Consistency and PPL-style assessment

## What fits together well

- `Board` is configuration, `Session` is the active call, and `Session.User` is
  stored caller data. The apparent overlap reflects real PCBoard semantics.
- `Terminal` owns terminal extensions; capability facts stay in `Terminal.Info`.
- Arrays unify board collections, contacts, notes, split results, and regex
  matches. `FOREACH` therefore works everywhere without protocols or iterators in
  source.
- Typed enums replace magic selector values while still compiling to integer-like
  PPE data.
- `Valid` plus `Error.Last()` cleanly separates normal lookup misses from
  operational failure.
- Explicit `Free`, `Shutdown`, `Release`, and reset methods suit bounded sessions;
  automatic cleanup is a safety net rather than invisible lifetime magic.
- New control flow uses familiar PPL/BASIC words instead of importing braces,
  lambdas, exceptions, or class syntax.

## Where the direction starts to drift

- Rewriting fixed array declarations from parentheses to brackets changes an
  iconic PPL spelling without solving a remaining ambiguity. Brackets solve
  indexing ambiguity; they are not needed for fixed declarations.
- Immutable request methods named `Set...` borrow a fluent value-builder style
  that no other PPL resource uses.
- A growing member API is useful for objects, strings, arrays, bytes, and regex,
  but should not become a campaign to replace the classic global/statement API.
- `PresentRect` shows the limit of long positional calls. Future complex options
  should use a small record rather than ever-longer overloads, but only when a
  real second use case appears.
- Fallible mutation uses methods returning `BOOLEAN`; property assignment is
  reserved for values whose assignment cannot fail operationally.

# Part IX — Documentation and implementation findings

These are concrete inconsistencies found during the review:

1. The older review counted 30 object/record types and 11 enums. The current
   registry defines IDs 30–54 (25 types including `CONTACT`) and 14 built-in
   enums.
2. The reference still contains old wrapper collection names in places, while
   the runtime returns arrays (`CONFERENCE[]`, `USER[]`, `AREA[]`, and so on).
3. One language page says fixed arrays use parentheses at 4.00; another says the
   formatter/decompiler emits brackets and warns on parentheses. This is not just
   stale prose—it reflects the unresolved style decision called out above.
4. Generated registry dumps currently omit array return rank in their printed
   type, making `FindAll`, `Split`, `Notes`, `Contacts`, and board collections
   look scalar. The dump should print rank before it becomes a review oracle.
5. API prose has previously drifted on `ErrKind`, `ErrCode`, `HttpMethod`,
   `EditorMode`, input signatures, and checksum names. Enum values and signatures
   should be generated or tested against documentation.
6. The global function count changed as bytes conversions were added. Counts in
   narrative reviews should either be generated or omitted.

## Recommended pre-freeze actions

| Priority | Action | Why |
| :---: | :--- | :--- |
| 1 | Decide fixed-array canonical syntax; recommendation: preserve `name(10)` and use brackets for indexing/dynamic rank. | Highest PPL-style impact. |
| 2 | Rename immutable request transforms to `WithHeader`/`WithText`, or make `Set...` mutate and return `BOOLEAN`. | Aligns naming with semantics and the rest of the API. |
| 3 | Extend the registry dump to show array rank, record fields, enum values, writable flags, optional arguments, and static/member status. | Prevents another stale API review. |
| 4 | Add documentation checks/generated tables for signatures and enum values. | Current prose already contains contradictions. |
| 5 | Add an `EventKind` applicability table and lifetime/mutability label to every object reference. | Prevents the most likely API misuse. |
| 6 | Freeze IDs/opcodes only after the above decisions; keep pre-400 numbers untouched. | 4.00 can still be compacted before release, but not afterward. |

## Post-freeze restraint

Do not add these merely to look modern:

- classes, inheritance, interfaces, exceptions, lambdas, async/await, or generic
  collection protocols;
- LINQ-style array pipelines or automatic iterators;
- implicit network access outside board policy;
- replacement APIs for classic PPL calls that already work;
- multiple synonymous spellings for each bytes/string operation;
- writable board configuration objects.

Reasonable future additions are small board-domain APIs, missing array operations
that have demonstrated real PPE use cases, and typed records for calls that have
outgrown positional parameters.

# Final verdict

**Overall: GOOD, with three pre-freeze corrections.** PPL 4.00 modernizes the
language without giving it a new identity. Its board model, typed values,
collections, control flow, and terminal grouping are coherent. The main risks
are not the large features; they are small consistency choices that accumulate:
fixed-array spelling, immutable HTTP `Set...` methods, and stale handwritten API
inventories.

Preserve the procedural core, preserve old APIs, keep new board concepts typed,
and require every future addition to answer four questions: who owns it, whether
it is live or a snapshot, how failure is reported, and whether its spelling still
looks natural beside classic PPL.
