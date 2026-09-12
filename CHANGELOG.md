# Changelog

All notable user-visible changes to IcyBoard are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
IcyBoard is still in beta, so unstable PPL runtime 4.00 APIs may change between
releases.

## [Unreleased]

## [0.2.2] - 2026-09-12

Third public beta. This release expands the BBS administration, upload and FTN
workflows and introduces the current PPL 4.00 language, APIs and PPE container.
It is not a stable release; see [known limitations](docs/known_limitations.md).

### Compatibility

- Recompile older beta PPEs for the current runtime 4.00, including ASCII-only
  programs. The new sectioned container replaces the old beta encoding, and
  earlier experimental APIs and array calling conventions are not preserved.
  Review source changes using the [language guide](docs/new_ppl.md) and
  [PPE format specification](docs/ppe_format.md).
- Classic PCBoard targets remain separate. Selecting language 400 is an
  upgrade, not a promise of unchanged source semantics; use the original
  language/runtime versions for compatibility-sensitive programs.
- Review upload-processing configuration before upgrading: advertisement file
  and description catalogs are separate, and ZIP comments use an explicit mode
  instead of the removed comment-rule catalog.
- Validation for this release ran on Linux. Windows/macOS execution, the full
  terminal-client matrix and some external DOS/editor workflows remain
  unverified. These are beta limitations, not successful test results.

### Added

- PPL 4.00 uses a versioned, sectioned PPE container with strict UTF-8 constants,
  record and enum metadata, host imports bound by qualified name and signature,
  optional Zstd compression and optional debug symbol names. Loader budgets
  validate decoded content; the initial whole-file read is not memory-bounded
  by those checks. Only install trusted PPEs.
- PPL records support dynamic array and host-object fields, with copy-on-write
  value semantics for nested data and shared identity for embedded resources.
  `&&` and `||` provide short-circuit evaluation through compilation,
  serialization, execution and decompilation.
- File-browser APIs: bounded indexed `Directory.Find`, read-only `FILEPAGE` and
  `FILEENTRY` snapshots, and `Directory.Flag` for exact area-scoped marking.
  The example browser separates marking from the normal BBS download command.
- Message composition through the session's post, reply and edit workflows,
  including external-editor integration and the standalone `ledit` PPE editor.
  The message-reader example uses public BBS APIs instead of parsing JAM files.
- Logical terminal resize events and an adaptive Paint PPE with ANSI/Sixel
  output, terminal-input handoff and cleanup. The JXL Paint path and a complete
  versioned client matrix remain unverified.
- Cooperative, process-local sharing for PPE file channels, including a
  documented lock-file workflow for read/modify/write operations. It does not
  provide cross-process locking or power-loss durability.
- Scheduled maintenance and online events with weekday/window rules,
  start-anchored intervals, durable execution history and manual runs.
  Maintenance drains managed writers before restart; online events must not
  modify live board files. See the [event guide](docs/events.md).
- Generated English BBS command help with 68 substantive topics, source
  export/override support and guarded offline installation. Language suffixes
  support sysop-supplied translations, not a bundled German translation.
  See the [command help guide](docs/gettingstarted.md#command-help).

- Optional, default-off email temporary-password recovery for normal BBS login.
  After three failed passwords, callers can request mail to their saved address;
  temporary verification requires a new password and normal relogin, without
  admitting the caller to surveys, accounting or menus. Includes bounded TLS
  SMTP, directly configurable SMTP credentials (legacy environment fallback), hash-only expiring challenges,
  issuance/attempt limits, atomic credential replacement, stale-session write
  protection, English/German setup help and appended ICBTEXT 781–784. An
  optional UTF-8 mail-body template can customize the letter using board,
  user, temporary-password and lifetime placeholders. Requires
  hashed credentials; sysops and inactive accounts are excluded. See the
  [configuration and security constraints](docs/configuration/board.md#optional-email-password-recovery).

- `mkicbmnu --board/-b` accepts a BBS directory or configuration file;
  `--check` validates menus without opening the editor or writing files.
  The editor adds save-without-exit, undo/redo, isolated command/action drafts,
  search and duplication, assisted action parameters, command charges,
  language prompts and a read-only menu preview with static action traces.
  New files are deferred until save and never overwrite existing destinations;
  save failures retain the open document. See the [menu editor guide](docs/mkicbmnu.md).

- Call-wait Log Viewer with bounded, read-only application/caller logs, follow
  and search. T switches filter/context; n/N navigate matches with wraparound.
  Pausing freezes content and size/UTC modification metadata, including pending
  reads; resuming requests fresh data immediately after any old read completes.
  Reads/searches stay within 256 KiB, 2000 lines and 2048 characters per line.
  Rotation notices use sampled Unix device/inode changes; truncation notices use
  size decreases, so changes between samples can be missed.
  System Status adds uptime, actual local listeners, node counts and runtime
  errors, bound-but-login-gated state and transition age/time. Disk values show
  available/total GiB and percent available, warning strictly below 1 GiB OR 10%.
  Configuration/runtime/node/disk freshness is independent, with UTC timestamps,
  sample ages, a two-second stale threshold and explicit busy/failure/pending states.
  These diagnostics are read-only and do not alter runtime policy.

- Event Monitor shows a schedule **Candidate**, not a guaranteed start, and
  weekday/daily-window restrictions (no calendar date-range settings). Cached
  history is newest first; PgUp/PgDn select older/newer runs, Left/Right scroll
  details, and L opens the selected execution's canonically guarded output under
  `event_logs`. Fixed event output is UTF-8, with Tab/E disabled. Duration/elapsed
  time and exit codes reflect journal data, not verified process runtime.
  There is no interrupt/kill button. All three screens have English and German text.

- Accounting setup and operator documentation, with per-use/per-minute command
  and door rates, legacy CMD.LST rate import, and credit for accepted uploads
  including successful intake awaiting manual approval. See the
  [accounting guide](docs/accounting.md) for modes, funding and limits.

- An optional token-protected web administration interface hosted by the
  running board, with overview diagnostics, live configuration editing and
  upload-quarantine management. It binds to localhost by default and shares
  the engine's validation and save paths rather than maintaining a second
  configuration model.

- FTN routing and AreaFix, including per-link passwords, subscription updates,
  passthru-area creation, forwarding unknown requests to an uplink and result
  netmail. ICBSetup can edit the board's AKAs, links and routes.

- `ppld --check --strict` fails on unsupported, unimplemented or partially
  implemented references; ordinary `--check` remains informational.

- Whole-file text advertisement templates in the existing member rule catalog:
  `[[text_member_rule]]` supports literal and anchored regex lines, bounded
  CP437/UTF-8 normalization and `report_only` (default), `review`, `auto_clean`.
  Reports include rule, encoding and raw SHA-256; ambiguous matches preserve the
  archive for review. Existing byte fingerprints still take precedence. Four
  reviewed auto-clean templates and a raw-corpus validation report are included.
  See the [text-rule guide](docs/upload_processing.md#whole-file-text-templates).

- Uploads can be quarantined, checked for known or patterned BBS advertisements,
  have appended `FILE_ID.DIZ` footers cleaned and ZIP comments managed, be virus
  scanned and repacked as ZIP before publication. SysOps can configure the
  pipeline in `icbsetup`, receive a private mail for each accepted upload, and
  inspect, reprocess, approve or reject quarantined files in `icbadmin`.
  One optional own advertisement file (or a trusted PPE generating that file)
  and ZIP comments are supported; archive descriptions can only be cleaned,
  never inserted or replaced with board advertising. The headless generator
  receives a private output directory and the archive name, with a 30-second
  timeout and a 16 MiB advertisement limit. See the
  [upload processing guide](docs/upload_processing.md).
  Rules for complete advertising files and description blocks
  have two separate catalogs and editable setup paths; an empty path
  disables just that category. The old `upload_processing.advertisement_rules`
  key is replaced by `advertisement_file_rules` and `advertisement_description_rules`.
  Existing combined member/description catalogs can be selected explicitly in
  both fields. Comment rules and `advertisement_comment_rules` are removed:
  `archive_comment_mode` now explicitly selects Preserve (default), Remove or
  Replace, serialized as `preserve`, `remove` or `replace`, independently of
  advertisement removal. Only Replace uses `replacement_archive_comment`;
  an empty replacement clears the comment, and the string is ignored otherwise.
  Remove/Replace enable archive processing even without advertisement removal
  or unconditional ZIP repacking. Non-ZIP source comments are not available.
  The offline corpus generator emits only member/description rule catalogs;
  the upload cleaning demo explicitly removes all ZIP comments. Prior corpus
  and audit reports retain their historical results.

- Description-advertisement rules accept `literal_lines` for complete plain-text
  blocks without regex escaping, alongside existing regex-based `lines` rules.
  Matching uses the same case, whitespace and color normalization; retained
  description bytes are unchanged. Each rule requires exactly one nonempty
  list, and invalid literal definitions require upload review. The shipped
  LiQUiD footer uses the simpler syntax.

- The AREA.LST editor imports `FIDONET.NA` and other networks' `.NA` area
  lists. It previews every tag and name, lets the sysop select what to take,
  skips tags already present, and chooses a new path rather than overwriting
  an existing message base.

- A nodelist can be read, and a link configured without a host of its own is
  looked up in it. The host and port come from its `IBN` flag, falling back to
  the address `INA` gives, so those two no longer have to be copied by hand.
  The file is read as it stands, so nothing has to be compiled first. Point
  `nodelist` in `ftn.toml` at it, or set it under Message Networking.

- A packet the tosser cannot read is moved to the directory named by
  `bad_packets` and the report says where it went. It stayed in the inbound
  before, so every later run tried it again and complained about it again,
  which buried the mail that did arrive. PCBoard kept such a directory for the
  same reason.

- An import takes the board's location from the profile PCBoard introduced
  itself with over `EMSI_DAT`, which is the only place it kept one.
  `PCBOARD.DAT` has no such field, so the location used to come out empty and
  a binkp session announced nothing.

- Files that arrive with a `.TIC` are tossed into the file directory carrying
  their area. They used to be announced as being tossed and then left in the
  inbound, because nothing here read a TIC at all. A directory says which echo
  it carries with `Fido Area Tag` in the directory editor, and one named after
  the echo is found by that name. The file is registered in the directory's file
  base with the description the TIC brought, a `Replaces` mask removes what the
  new file supersedes, and a file whose size or checksum does not match what was
  announced stays in the inbound rather than being handed to users. `Pw` is
  checked against a link's new `tic_password`.

- File requests are answered, which PCBoard configured on four screens and
  this board now offers under the same names: a FREQ path list, restrictions,
  magic names and a deny list. A requested name is matched against the listing
  of a configured path and never becomes a path of its own, so a request cannot
  reach outside what was offered. `FREQPATH.DAT`, `MAGICNAM.DAT`,
  `FREQDENY.DAT` and the restrictions in `PCBFIDO.CFG` are read on import.

- Netmail written on this board is packed and sent. It used to arrive and stay:
  a reply reached the message base and never left it. The scanner takes what
  names a destination and has not gone yet, sends it to that node when it is a
  link, through the next hop when a route names one, and otherwise through the
  only link that can be called. A board with several reachable links needs a
  route rather than having one picked for it. Sent mail is marked so it goes
  once, and mail asking to be killed once sent is removed. An area pointing at
  the netmail base is refused rather than exported as echomail to every
  downlink. What was written here is told from what came in by the local mark
  every message carries, so mail entered with an offline editor such as GoldED
  goes out as well.

- A message area can carry its own Fido origin line, set as `Fido Origin` in
  the area editor. Echomail written there leaves the board with that line
  instead of the board-wide one, which is how PCBoard's origin conference
  ranges are honoured; `ORIGINS.DAT` is read on import and handed to the areas
  of the conferences each entry names. `AREA` gained `EchoOrigin` so a PPE can
  read it.

- `HttpMethod` gained `Put`, `Delete` and `Patch`, so a PPE can update and
  remove REST resources instead of only creating and reading them.
  `HttpResponse.Bytes()` returns a retained body as `BYTES` without decoding
  it, which makes images, archives and checksums reachable without a detour
  through the file system. `HttpRequest.SetBytes()` sends a binary request body
  without converting it to text and defaults to `application/octet-stream`.

- `HttpRequest.SetForm(name, value)` appends one percent-encoded
  `application/x-www-form-urlencoded` field and sets that content type, so a
  form POST no longer has to be assembled by hand. `Http.UrlEncode(text)` and
  `Http.UrlDecode(text)` handle RFC 3986 URL components, while
  `Http.FormEncode(text)` and `Http.FormDecode(text)` expose form encoding for
  query strings and form-encoded replies. `HttpRequest.SetQuery(name, value)`
  updates one query parameter and performs the URL encoding automatically.

- Criterion benchmarks cover PPL parsing, compilation, PPE serialization,
  string and array value operations, and prepared VM execution. The dedicated
  bench profile optimizes for speed, and named baselines support before/after
  comparisons during performance work. Compilation measurements distinguish
  the complete source pipeline from work performed after parsing.

- Display files can use native IcyDraw `.icy` screens and scripted `.icyanim`
  animations directly, without invoking the external `icy_play` utility.

- PPL package manifests support transitive source-library dependencies through
  local `path` entries and Git repositories selected by `rev`, `branch` or
  `tag`. Plain dependency sources form an implicit module named after the
  dependency entry, so `IMPORT themes AS MyTheme` imports an entire library.
  Dependency modules are compiled into the consuming PPE and are available to
  language-server analysis. Because a library is a module, it may only declare
  variables, constants, types and routines; it has no program of its own to run,
  and its own `;$LANGVERSION` applies to that library alone. Two packages that
  depend on the same library, even under different aliases, share one compiled
  copy instead of colliding, and a library's top-level initializers always run
  before the importing program's own code.

- PPL 3.50 adds compile-time `MODULE ... ENDMODULE` namespaces. Declarations
  are public by default, standalone `PUBLIC` and `PRIVATE` lines switch section
  visibility, and `IMPORT module AS alias` provides qualified access without
  changing the PPE runtime format. The compiler isolates equal declaration and
  type names across modules, and the language server highlights and completes
  imported public APIs. A declaration placed before `MODULE` or after
  `ENDMODULE` in the same file is rejected as outside the module rather than
  silently joining it.

- Functions, procedures and `DECLARE` statements support Rust-style Markdown
  documentation through contiguous `;;;` comments. Such a block stays an
  ordinary comment to the compiler, so it needs no language version and reaches
  no PPE. The language server shows the documentation in hover, completion and
  signature help.

- PPL 4.00 adds compiled `REGEX`, `REGEXMATCH` and `REGEXMATCHES` objects for
  Unicode-aware matching, numbered and named captures, bounded collection,
  replacement and transactional splitting. `RegexOptions` controls matching,
  while invalid patterns and limits report through `ErrKind.Regex`.

- PPL 4.00 `STRING` and `BIGSTR` values expose discoverable members for length,
  forward/reverse search, containment, prefix/suffix tests, occurrence counting,
  replacement, trimming and case conversion. `Split` fills a dynamic string
  array while retaining empty fields, and static `STRING.Join()` and
  `STRING.Repeat()` cover aggregation. Member positions are zero-based Unicode
  character positions and missing searches return `-1`; the classic global
  functions remain 1-based. String operation failures report `ErrKind.String`.

- PPL 4.00 records can be stored through existing file channels. `FGETREC` and
  `FPUTREC` use an escaped one-scalar-per-line format that leaves following
  documentation unread; `FREADREC` and `FWRITEREC` use compact length-framed
  binary values. Both support nested records and fixed arrays, and failed reads
  leave their destination unchanged while reporting through `FERR` and
  `Error.Last()`.

- PPL 4.00 exposes the board's registered users through the read-only
  `Board.Users` collection. Entries are `USER` snapshots, including their notes
  and contacts, and `USER.Valid` distinguishes missing indexes. `Session.User`
  remains the live writable caller record.

- PPL 4.00 has basic math functions: `Sin`, `Cos`, `Tan`, `Atan`, `Log` (natural
  logarithm) and `Sqrt`, all taking and returning `DOUBLE`.

- PPL 4.00 adds the growable `BYTES` type for binary data. `TOBYTES` converts
  supported scalar values, `ToString()` decodes UTF-8, base64 functions operate
  on bytes, and `GetChecksum()` supports CRC32, MD5 and SHA-256 with hexadecimal
  output through `ToHex()`.

- The PPL language server understands `;$DEFINE`, `;$IF`/`ELSEIF`/`ELSE`/
  `ENDIF`, predefined substitutions and conditional source, with completion,
  hover and semantic highlighting for preprocessor directives.

- Telnet and SSH connections support an optional per-connection SOCKS5 proxy,
  including username/password authentication and proxy error reporting.

- Native DOS doors can run in an isolated FreeDOS environment on supported
  hosts. Door sessions enforce configurable runtime limits, terminate wedged
  processes and restore the caller cleanly when the DOS runtime exits.

- The local call-waiting screen exposes Sysop paging controls and status, so a
  waiting caller can request and cancel a page before the call is answered.

- Added a Weather PPL 4.00 example using the HTTP, graphics and terminal APIs.

- Expanded source-build documentation with bundled/system library variants and
  macOS prerequisites.

- PPL 4.00 has policy-controlled `Http`, `HttpRequest` and `HttpResponse`
  objects. Public HTTP and HTTPS destinations work by default; sysops may disable
  outbound access or restrict it to exact origins, and set request, response, timeout, redirect
  and board/per-node concurrency limits from `icbsetup`. DNS is validated and
  pinned per hop, redirects are
  rechecked, system proxies are ignored, response bodies are streamed within a
  fixed cap, and downloads are committed atomically. Network failures report
  `ErrKind.Net`; HTTP statuses remain available on the response. The earlier
  beta `WebRequest` function and statement have been removed. Runtime-400 PPEs
  built before this change must be recompiled because the unreleased opcode
  tables were compacted after that removal.

- PPL 4.00 user statistics now expose their full stored counters as `ULONG`.
  Writes to page length and security-level fields reject out-of-range values
  with `ErrKind.User` instead of silently wrapping them.

- PPL record fields support rank-1/2/3 arrays. Fixed fields retain their
  declared bounds; dynamic fields may adopt new bounds. Both survive PPE
  serialization and decompilation. Fixed-field shape mismatches and malformed
  record/member bytecode are rejected before changing the destination.

- `MSG`, the message type. `AREA.Read(number)` answers with one, and it reports
  `From`, `To`, `Subject`, `Date`, `Time`, `ReplyTo`, `Status`, `Size`,
  `IsPrivate`, `IsRead`, `IsDeleted`, `IsEcho`, `NeedsPassword` and `Text()` -
  what `GETMSGHDR(AreaId(c, a), n, HDR_SUBJ)` returned as a string picked by a
  number is now a member with a type of its own. `AREA.Find(MsgField.To, "STAN")`
  is `SCANMSGHDR` with a type instead of a field number, and takes a start number
  to walk on to the next match. `AREA.LowMsg()` joins `HighMsg()`, because a
  message base is sparse and a walk needs both ends of it.

  A message is addressed by its number rather than its position, which is why it
  is not a collection: `[ ]` indexes a position everywhere else in the language,
  and a message number is not one. The body stays in the base until `Text()`
  asks for it, so a listing that prints headers never pays for a body.

  The type is called `MSG` because `MESSAGE` has been a statement since PPL 1.00.
  A board object name is resolved wherever a type name is expected, so calling it
  `MESSAGE` would have turned `MESSAGE conf, to, ...` into a declaration at
  language 400 and quietly broken the statement.

  Legacy `GETMSGHDR`, `SETMSGHDR`, `SCANMSGHDR` and `MESSAGE` remain available.
  Use the session APIs for interactive post, reply and edit workflows.

- Added runtime 4.00 `Board` snapshots and a live `Session` view, including the
  current conference, message area, file directory and caller. The session's
  own properties are read-only; documented `Session.User` fields are writable.
- `icbfile scan` now identifies archives that have no usable description and
  distinguishes files that are missing from disk.
- `icbfile scan --all` scans every area in a `file_areas.toml`; it can be
  combined with `--force` to re-extract descriptions in every area.
- Added end-to-end coverage for multiline `FILE_ID.DIZ` extraction and
  all-area scanning.

### Changed

- Uniform hotkey bars in ICBSetup, ICBSM, ICBText, the call-wait monitors and
  shared dialogs. The existing hint bars now come from one structured catalog
  with Unicode key symbols, centered placement on the frame and theme colours
  instead of per-tool hint strings; runtime keys are unchanged.
  See the [developer guide](docs/hotkey_bars.md).

- The call-wait subscreens share one frame: yellow double borders, a centered
  bracketed title, the board's date format on the left and the clock on the
  right. The main call-wait screen keeps its own white frame and plain title.

- Call-wait now offers User / Sysop / Exit, then Log Viewer / System Status /
  Event Monitor. Local logins keep network services running; Exit stops the
  application and services without opening a shell. Event Monitor replaces
  runtime F6; setup history keeps F6, and statistics reset stays in its monitor.

- PPL 4.00 constant declarations reject numeric values outside their declared
  range (`CONST BYTE N = 257` is an error). Dependent and module constants use
  converted values consistently, preserve nominal types and DOUBLE precision,
  and keep STRING unbounded. Constant folding no longer rewrites enum namespaces
  or bypasses enum argument checks. Older-source wrapping remains unchanged.
- PPL 4.00 FOREACH validates array sources and scalar targets, including runtime
  enum/record checks. Fixed record-array fields validate actual shapes before
  replacement; indexed record targets enforce nominal types. Recursive record
  results are invocation-local, and computed rank-2/3 results support indexing
  and decompiler roundtrips, including callback signatures.
- PPL 4.00 Split evaluates text, separator and limit left-to-right. Regex.FindAll
  validates limits even outside the text. The modern StripATX member preserves
  malformed control text without changing the classic opcode. Explicit Ordinal
  comparisons preserve old errors like their default overloads. LSP completion
  and signature help retain array ranks, distinguish scalar/array members and
  offer typed StringComparison/Checksum arguments.

- Enums are open nominal signed-32-bit types from language 3.50 onward.
  Unnamed values and bit combinations are legal; different enum types do not
  mix implicitly. The first declared member is the default. `EnumName(integer)`
  and `TOINTEGER(value)` convert explicitly, while same-type `|`, `&`, `|=`,
  `&=` and `Has(mask)` retain nominal typing. Storage and explicit conversions
  require runtime 4.00. Individual host APIs still reject unsupported values
  or option bits; accepting an enum value does not imply operational support.

- PPL 4.00 arrays use square-bracket declarations and indexing. Empty brackets
  declare dynamic vectors, matrices or cubes, functions can return dynamic
  arrays, compatible whole-array assignment adopts the source bounds, and
  declarations accept initializers. Parenthesized 4.00 array syntax remains
  accepted with a migration warning.

- PPL parsing allocates routine documentation storage only when documentation is
  present. Semantic references share one source path per file, avoid reporter
  locks for path lookup, and generate variable tables without cloning all
  routine containers. Routine-heavy parsing improved by about 7% and compilation
  by about 17% in Criterion benchmarks.

- PPL statement optimization now builds a basic-block control-flow graph instead
  of repeating whole-vector scans until the statement count stabilizes. CFG
  reachability also drives semantic call edges, while jump-chain compression,
  predecessor tracking and `GOSUB`/`ON ERROR` edges preserve runtime behavior.

- PPL code generation now resolves expressions into a typed HIR before lowering
  them to PPE expressions. Stable symbol and call IDs are shared with semantic
  analysis and the call graph, and repeated argument/member resolution during
  code generation has been removed.

- Optimizer regression tests execute deterministic generated arithmetic,
  branches, labels and routine calls with optimization both enabled and
  disabled, then compare output, globals, errors, call-frame cleanup and file
  side effects.

- PPL semantic analysis now builds a call graph shared by the compiler and
  language server. The compiler removes routines, locals, globals and constants
  reachable only from dead code, and omits wholly unused record types while
  preserving every field and its order in retained record layouts. References
  in statically unreachable statements no longer keep code alive, while those
  statements are still checked for diagnostics.

- PPL execution avoids deep-cloning commands and whole collections in hot loops,
  moves routine frames instead of copying them, and shares string and array
  storage until mutation. `FOREACH` uses constant-time multidimensional indexing,
  while array and string assignments retain value semantics through copy-on-write.

- PPL 4.00 `STRING` now has its own serialized scalar type ID, 24, and remains
  unbounded Unicode text. Classic type 7 `STRING` retains its 256-character
  limit, while type 13 `BIGSTR` retains a 2048-character limit and is deprecated
  in new 4.00 source. The legacy limits count Unicode characters in IcyBoard so
  old character-oriented PPE logic behaves consistently with UTF-8 board data.
  All PPL 4.00 scalar members, object fields, parameters and return values use
  type 24 directly; types 7 and 13 remain confined to classic compatibility
  surfaces.

- The language server waits for typing to pause before reading a program again,
  and reads it on a thread of its own. A burst of keystrokes is answered once
  instead of once per key, and completion, hover and the outline no longer queue
  behind the reading of a whole package. Requests still wait for a reading that
  is under way, so an answer never comes from a program the editor has already
  left behind.

- The language server notices a file or a `ppl.toml` that changed outside the
  editor, and re-reads what is open when settings or workspace folders change. A
  manifest edited by hand or a branch switched underneath no longer needs a
  window reload to take effect.

- PPL 4.00 adds signed `LONG` and unsigned `ULONG` 64-bit integers. `ToLong()`
  now returns the new `LONG`, and `ToULong()` converts to `ULONG`; before 4.00,
  `LONG` and `ToLong()` retain their historical 32-bit `INTEGER` meaning. The
  language server's 4.00 upgrade action rewrites old `ToLong()` calls to
  `ToInteger()` so upgrading source preserves its behavior.

- `MSG.Number`, `MSG.ReplyTo`, `MSG.Size` and `AREA.LowMsg()`/`HighMsg()` are
  `LONG` rather than `INTEGER`. JAM counts messages and body bytes in 32
  unsigned bits, which fit exactly without making ordinary arithmetic unsigned.
  `Read()` answers an invalid `MSG` for a number outside JAM's range rather than
  truncating it into a message that exists.

- An area is read through one open message base instead of opening it again for
  every message. The documented walk over 2000 messages went from 16 ms to 7 ms,
  and from 26 ms to 13 ms when it reads the bodies too - `FOR n = area.LowMsg()
  TO area.HighMsg()` re-evaluates the bound on every step, so the bound alone was
  opening the base 2000 times. The modern walk is now faster than the `GETMSGHDR`
  loop it replaces rather than slower. A message written after the base was
  opened is still found, and `LOMSGNUM()`/`HIMSGNUM()` still open it on every
  call for a PPE that watches what another node writes.

- Message lookup now distinguishes absence from failure. A number outside the
  base, a deleted message or an empty slot still answers with an invalid `MSG`
  and leaves `Error.Last().OK` true. `AREA.Read`, `Find`, `LowMsg`, `HighMsg`
  and `MSG.Text()` now report `ErrKind.Msg` with `ErrCode.Io` for filesystem
  failures, `ErrCode.Format` for corrupt JAM data, and enter `ON ERROR` handlers.
  Their invalid, zero and empty fallback values are unchanged.

- The board objects report what they are configured to be, not only what they are
  called. `CONFERENCE` gained `IsReadOnly`, `AllowAliases`, `EchoMail`,
  `AutoRejoin`, `PrivateUploads`, `Password`, `CanPost()` and `CanAttach()`;
  `AREA` gained `IsReadOnly`, `AllowAliases`, `QwkName`, `EchoTag`, `CanEnter()`,
  `CanAttach()` and `HighMsg()`; `DIRECTORY` gained `Path`, `IsFree`,
  `HasNewFiles`, `Password` and `CanDownload()`; `DOOR` gained `Path`. A PPE can
  now write a listing that says which areas are read-only, echoed or hold new
  mail without falling back to `CONFINFO(conf, field)`. `HasAccess()` stays the
  question a listing asks — what a caller may then *do* is asked separately,
  because the board configures the two apart. Every password a board object hands
  out is of the masked `PASSWORD` type, so a listing can say that something is
  locked without saying what unlocks it.
- Every read-only object now refuses a write in the same words, naming the member
  that was written. `CONFERENCE`, `DIRECTORY`, `DOOR`, `ERROR`, `SURFACE` and
  `TERMINFO` used to accept one silently and drop it. Nothing a PPE can compile
  reached that path - the compiler rejects the assignment and the VM has no
  setter to call - so this is a guard for the day one of those objects gains a
  writable member, not a hole that was open.

- The PPL object API now uses typed array snapshots for `Board.Conferences`,
  `Board.Users`, conference areas/directories/doors and user notes/contacts.
  Arrays support indexing, `Len()` and `FOREACH`, not the interim collection
  wrappers or `Count`/getter pairs. Board state is captured once per PPE;
  user/conference arrays are materialized only when first requested, so reading
  metadata does not build the complete user array.
- `FOREACH` evaluates its source once and walks a value snapshot in row-major
  order at any rank. Changing or resizing the original does not alter the walk.
  Array members expose `Len()` and `Redim(...)`; computed and read-only arrays
  cannot be redimensioned, while dynamic record array fields can.
- `Session.User` exposes the live caller with immediate, checked persistence
  for writable fields. Notes and contacts are read-only array snapshots;
  mutations use `SetNote`, `AddContact` and `RemoveContact`. `SetPassword`
  follows the board's hashing configuration. Caller names, statistics and
  `Board.Users` entries remain read-only. Classic `U_*` variables remain
  available; the experimental `U_CONTACT` is removed.
- The current runtime 4.00 replaces the interim 4.01/4.02 APIs. Terminal access
  uses `Terminal.Info`, `Gfx`, `Input`, `Margins`, `Palette` and `Macros`, with
  font operations directly on `Terminal`. Resources use `Surface.New/Load` and
  `Audio.Load`; audio mutations use methods such as `SetVolume` and
  `Fade(targetVolume, durationMs)`. Error state uses `Error.Last/Clear`, while
  `ON ERROR`, `FERR` and `DERR` retain their separate roles. See the
  [API reference](docs/new_ppl.md) for complete signatures and migration rules.
- Editor grammars, completion and signatures track the current type catalog,
  including `FILEENTRY`/`FILEPAGE`, array ranks and members after indexing.
  Compiler checks, formatting and decompilation use the versioned language
  rules; `pplc --check` reports formatting differences as well as source errors.
- Command-line tools include the build's short Git commit hash in `--version`.
- Runtime persistence is serialized without holding the global board lock.
  Shutdown drains managed writes, and user updates preserve session accounting.

### Fixed

- UTF-8 terminal output uses grapheme-aware display cells across the screen
  model, local console and TUI, including combining characters and wide text.
  PPL string lengths remain Unicode-scalar counts; CP437 substitution is
  unchanged. Graphics clipping also handles off-screen coordinates safely.
- `ON ERROR` dispatches after the invoking VM instruction completes and keeps
  the first error within that instruction. PPE cleanup preserves the primary
  error; terminal reset after disconnect remains best-effort.
- Freed audio/surface handles stay invalid after resource-slot reuse or a
  graphics restart. Resource equality compares allocation identity, not reused
  channel numbers.
- `VAR` targets and indices are bound once in argument order. Copy-out retains
  the documented reverse order, while the language server warns about provably
  overlapping arguments without treating them as reference aliases.
- Classic `STRIPATX` follows PCBoard behavior; the modern member remains a
  separate API, and neither should be used as a general output sanitizer.
- File-search defaults, OSC 8 link colors and editor grammar coverage are
  corrected. `NEXT` no longer consumes the following assignment's name.
- Builds without default features skip the BBS-only stored-PPE fixture target.

- W/LANG profile saves no longer finalize accounting. Logoff waits for enclosing
  command/door usage and successful final account persistence before summaries;
  settlement or final-save errors suppress them. Credit display retains up to
  six decimals with trailing zeros trimmed; money uses fixed dollar formatting
  with two decimals. Account saves and audit writes are not an atomic ledger.

- PPL compiler and language server now share source-level semantic analysis
  before executable lowering and constant folding. Invalid expressions and
  calls in dead branches are still diagnosed at their original source spans.
  Lowering consumes checked annotations rather than rerunning source semantics;
  generated HIR is validated before executable serialization. See the
  [compiler architecture](docs/ppl_compiler_architecture.md).

- `ppld` preserves expression grouping (including raw output), fractional
  arithmetic and function-call side effects. FOR reconstruction validates the
  counter, step direction and increment target; nested loops retain cross-loop
  jumps instead of capturing them as an inner BREAK/CONTINUE. Symbolic flag
  output retains unknown mask bits. Source mode `--output` writes only source
  to stdout, with its banner and diagnostics on stderr. Regression coverage
  compares serialized PPE execution before/after decompilation under an
  instruction budget, including historical fixtures.

- Module-level variable initializers now require constant expressions, including
  recursively constant array and record literals. Calls and mutable reads are
  rejected by the compiler and language server before optimization, for explicit
  and implicit library modules. Routine-local initialization remains unrestricted.

- PPL 4.00 array parameters preserve rank, contents and bounds through direct,
  recursive and callback calls. Value parameters are independent copies;
  `VAR` array parameters copy their final value and bounds back to the caller.
  Recompile unreleased 4.00 PPEs using array parameters: variable-header flag
  `0x04` now distinguishes whole-array formals from classic element-zero
  parameters, independently of static `0x01` and dynamic-storage `0x02`.
  Unmarked legacy formals keep their rank/bounds and element-zero save/restore
  and copyback behavior, including persistent tails and runtime-400 targets;
  there is no compatibility shim for ambiguous older beta PPEs.

- `DECLARE` matching follows the source language. Below 400, implementation
  parameter types, `VAR` modes, dimensions and function result types take
  precedence; parameter counts must still match. A declared procedure may be
  implemented with `FUNCTION`, retaining the implementation's `VAR` modes but
  emitting a procedure without a result slot; the reverse is rejected.
  Multidimensional implementation formals fail at the dimension comma even
  when unused, while multidimensional declarations remain accepted. Compiler
  and LSP call checks collect normalized implementation signatures package-wide.
  Language 400 strictly checks kind, count, types, `VAR`, ranks, bounds, dynamic
  markers and function return type/rank, recursively through callbacks; names
  are irrelevant. See the [DECLARE audit](compat/DECLARE_AUDIT.md) for the 23
  authored PPLC 3.40 compiler probes and the separate source-derived runtime
  evidence, rather than a claim of universal or byte-identical compatibility.

- Record array fields accept square-bracket assignments and compound updates,
  including nested paths, without weakening read-only property checks.
- Runtime 4.00 `SORT` handles empty arrays without panicking and produces
  exactly one index per input element rather than appending a spurious zero.

- PPL 4.00 dynamic arrays now have per-call local storage and fresh function
  results, including recursion. Whole-array assignments copy all elements and
  adopt bounds; brace initializers preserve explicit dynamic declarations.
- Array-returning functions no longer require explicit `DECLARE`. Callback
  signatures check and display the complete return type, including array rank.
  Array results are rejected consistently in scalar expressions, and 4.00
  `REDIM` preserves the declared rank in both statement and member notation.
- Compound assignments evaluate target indices and object receivers once,
  including nested record fields, recursive calls and module-qualified code.
  Recompile beta 4.00 PPEs using dynamic arrays: their storage flag is now
  distinct from the classic static-variable flag. Pre-4.00 PPE behavior is
  unchanged.

- PPL 4.00 case-insensitive string comparisons report oversized search literals
  through `Error.Last()` instead of panicking. `FindLast` and `EndsWith` handle
  overlapping matches correctly, and `Regex.FindAll` starts at the requested
  character position while preserving anchor, word-boundary and empty-match
  semantics.
- PPL 4.00 user mutations roll back the caller and in-memory user record when
  saving fails, return failure and publish `ErrKind.User` / `ErrCode.Io`.
  Invalid user and text-margin mutations now publish errors consistently;
  successful mutations clear errors left by earlier statements.

- Echomail for a point is no longer discarded as already travelled merely
  because its two-dimensional `PATH` names the point's boss. This could make a
  rescan report every message as a duplicate, remove the inbound bundle and
  leave automatically added areas without message bases.

- Mail that is handed on to a downlink or routed to the next hop is written to
  the outbound before what it arrived in is removed. A bundle that could not be
  written left the tosser having already thrown the only copy away.

- A file arriving over binkp is written under a working name and only takes the
  name it was offered as once it is complete. A session that broke off used to
  leave a partial bundle that the next toss read as a whole one, and a file
  named like one still waiting in the inbound overwrote it. Working files a
  killed session left behind are cleared away by the next one.

- A command whose action is `Door` opens the door its parameter names, so a
  door can be reached by a keyword of its own instead of only through `OPEN`.
  The action was never carried out and answered `Can't run action (Door)`. A
  parameter naming no door of the conference says which one it missed, and an
  empty parameter asks the way `OPEN` does.

- Every other action a command or menu option can have is carried out as well.
  Sixteen of them were named in the setup and in the MNU format but never
  reached the board, and each answered `Can't run action`. `Menu` opens the menu
  it names, `QuitMenu` and `ExitMenus` leave one or all of them, `Conference`
  joins, `DisplayDir` lists a directory, `DisplayFile` shows a file, `Script`
  takes a survey by number, `Command` and `GlobalCommand` run a command, and
  `Disabled` is left alone rather than treated as a failure. The dispatcher no
  longer has a catch-all, so a new action cannot be forgotten again.

- Text a command stuffs is typed into the board instead of being dropped. The
  four `StuffText` actions and both `StuffFile` ones put their text in the token
  list, which is cleared as soon as the command ends, so nothing ever acted on
  it - an imported `CMD.LST`, which is a keyword standing for what the caller
  would have typed, did nothing at all. The text goes into the keyboard now, and
  the silent variants leave it off the screen.

- Replying to a message addresses the answer to whoever wrote it. The reply
  went to the original recipient instead, so answering a message addressed to
  somebody else sent it back to that same person. A reply to netmail now also
  keeps the address and message id it is answering.

- Updated Tetris to use `Audio.SetVolume()`, taught editor grammar checks about
  contextual module words, and completed Paint's pixel-mouse test handshake.

- HTTP request bodies are retained with shared request values, while `SetText()`
  rejects bodyless `GET` and `HEAD` methods with a structured error.
- PPL decompilation preserves statement structure and source compatibility more
  reliably, and PPE execution restores the caller's active colors correctly.
- Explicit display-file extensions now take precedence over automatic
  extension probing.
- The local console follows negotiated terminal geometry while reserving the
  final two rows for status, and screen snapshots preserve Unicode. The Sysop
  monitor renders CP437 correctly and handles node disconnects without leaving
  stale state.
- Delayed terminal-position replies are discarded instead of becoming caller
  input, and PCBoard import errors report the unresolved source path.
- Online file matching no longer reports missing-file noise for matches that
  intentionally refer to non-local entries.
- The ICBSetup door editor is tall enough to show its complete form without
  clipping fields.

- Classic array reads require the correct number of subscripts rather than
  silently returning an unrelated slot. Operations that explicitly accept whole
  arrays retain that contract; language 400 additionally supports typed array
  values, assignments, parameters and function results.
- File listings now keep size, date and description in their fixed columns when
  a filename longer than 12 characters wraps onto its own line.
- File-base lookup, upload duplicate checks, flagging and downloads now treat
  ASCII filename case like DOS while preserving the real on-disk spelling.
  This prevents case-variant duplicates and lets uppercase prompts work on
  case-sensitive filesystems.

## [0.2.1] - 2026-08-23

Second public beta, containing 69 commits since `0.2.0-beta.1`.

### Added

- Added PPL runtime 4.02 terminal multimedia APIs.
- Added object-based PPL APIs:
  - `SURFACE` for RGBA drawing, blitting, Sixel/JPEG XL presentation, caching
    and terminal-side scaling.
  - `AUDIO` for SyncTERM audio playback, looping, fading, volume and channel
    management.
  - `TERMINPUT` and immutable `EVENT` snapshots for keyboard, physical-key,
    mouse, overflow and sound events.
  - `TERMINFO` and `TERMSTATE` for terminal capabilities and text margins.
  - `ERROR` for consistent operation error reporting through `ERR()`.
  - Extensible `CONTACT` values for user contact details.
- Added terminal font loading and selection, palette control, text margins,
  synchronized output and DEC terminal macro recording/playback.
- Added OSC 8 hyperlink macros.
- Added SSH private-key and SSH agent authentication.
- Added Fractal, Paint, Palette and Tetris PPL demonstrations.
- Added PCBoard-compatible administration themes and directory color editing.
- Added file creation from ICBSetup editors.
- Added `SURFACE.GetPixel()`.
- Added completion for runtime 4.02 object members in `ppl-lsp`.

### Changed

- Replaced the experimental slot-based graphics API with `SURFACE` objects.
- Replaced the experimental global sound API with `AUDIO` objects.
- Replaced the experimental global event API with the singleton `TERMINPUT`
  object. `Free()` returns input to classic `INPUT` and `InKey` handling.
- Renamed the language-server executable to `ppl-lsp`.
- Renamed the VS Code package to `ppl-vscode` and added packages carrying the
  matching server for Linux, Windows and macOS.
- Added standalone `ppl-lsp` archives for Zed, Helix, Neovim and other LSP
  editors.
- Changed release artifact names to use one platform vocabulary:
  `linux-x64`, `windows-x64`, `macos-arm64` and `macos-x64`.
- Graphics and audio resources are now released automatically when a PPE exits.
- Media uploads use terminal acknowledgements so following output cannot
  overtake a large upload.
- Fractal frames are completed before presentation and use terminal-side
  integer scaling where available.
- Tetris uses unified event input and transmits only its changing game panel.
- Updated dependencies, including `icy_sixel` 0.6.

### Fixed

- Fixed SyncTERM JPEG XL scaling by using standard integer `ZX`/`ZY` options.
- Fixed SyncTERM WAV capability probing and audio playback compatibility.
- Fixed SyncTERM status-bar flashing during Tetris updates.
- Fixed keyboard handoff after event polling and timed waits.
- Fixed delayed Escape input.
- Fixed local-mode inline image placement and SyncTERM media handling.
- Fixed full-screen message editor rendering.
- Fixed GNU Screen TUI rendering.
- Fixed PCBoard mixed-type promotion, routine return-slot handling, constant
  folding and expression evaluation that does not need to await.
- Fixed PCBoard-compatible handling of PPE-backed conference join commands.
- Improved PCBoard import path errors.
- Regenerated the tree-sitter parser for the new runtime types, statements and
  constants.

### Compatibility

- Runtime 4.02 was unstable during this development cycle. PPEs compiled against
  intermediate graphics, sound or event APIs must be updated and recompiled.
- Existing classic PPL runtimes and source remain supported.

### Distribution

- Added board archives for Linux x64, Windows x64, macOS ARM64 and macOS x64.
- Added standalone language-server archives and platform-specific VS Code
  packages.
- Added the PDF manual to the release.

## [0.2.0-beta.1] - 2026-08-18

First public beta of IcyBoard 0.2.

### Added

- Added a cross-platform PCBoard-style BBS for local, telnet, SSH and websocket
  sessions.
- Added tools for board setup and PCBoard import, user management, file-base
  maintenance, FTN mail, menu/text editing, and PPL compilation/decompilation.
- Added support for classic PPL runtimes plus modern language features,
  diagnostics, formatting and editor integration.
- Added Linux, Windows and macOS release archives and a PDF manual.

### Known limitations

- No serial, modem or FOSSIL support.
- PCBoard import remains best effort and PPE-heavy installations require manual
  review.
- See [Known limitations](docs/known_limitations.md) for the maintained list.

## [0.2.0-lsp1] - 2026-08-15

### Added

- Added standalone native language-server archives for editor extensions.
- Let editor integrations download the platform language server automatically,
  while preferring a configured binary or one already on `PATH`.

## [0.1.7] - 2025-10-15

Last release before the 0.2 beta series. Earlier release history is available
from the repository tags and GitHub Releases.

[Unreleased]: https://github.com/mkrueger/icy_board/compare/0.2.2...HEAD
[0.2.2]: https://github.com/mkrueger/icy_board/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/mkrueger/icy_board/compare/0.2.0-beta.1...0.2.1
[0.2.0-beta.1]: https://github.com/mkrueger/icy_board/compare/0.2.0-lsp1...0.2.0-beta.1
[0.2.0-lsp1]: https://github.com/mkrueger/icy_board/compare/0.1.7...0.2.0-lsp1
[0.1.7]: https://github.com/mkrueger/icy_board/releases/tag/0.1.7
