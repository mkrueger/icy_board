hint-preprocessor-langversion=Declares the PPL language version used by this source file. It must appear before code and takes precedence over workspace, command-line, and environment settings.
hint-preprocessor-define=Defines a case-insensitive preprocessor variable. The optional value may be used in conditional expressions or inserted into source with `;#name`; a definition without a value is true.
hint-preprocessor-if=Starts a conditional-compilation branch. The expression may use `VERSION`, `RUNTIME`, `LANGVERSION`, and variables introduced with `;$DEFINE`. Source in an inactive branch is not compiled.
hint-preprocessor-elseif=Starts another conditional-compilation branch when no preceding branch in the same block was selected.
hint-preprocessor-elif=Short spelling of `;$ELSEIF`; starts another conditional-compilation branch when no preceding branch was selected.
hint-preprocessor-else=Starts the fallback conditional-compilation branch when no preceding branch in the same block was selected.
hint-preprocessor-endif=Ends the conditional-compilation block opened by `;$IF`.
hint-preprocessor-usefuncs=Legacy compatibility marker indicating that a source uses user-defined functions. Current compilers accept it as a no-op.
hint-preprocessor-substitution=Inserts the value of a predefined, workspace, or `;$DEFINE` preprocessor variable into the token stream. An undefined name is an error.
hint-keyword-if=Starts a conditional block. Its body runs when the condition is true; optional `ELSEIF` and `ELSE` branches may follow.
hint-keyword-let=Introduces an assignment. `LET` is optional in ordinary assignments and is retained for compatibility with classic PPL source.
hint-keyword-while=Starts a pre-test loop that repeats while its condition remains true.
hint-keyword-endwhile=Ends a `WHILE` loop.
hint-keyword-else=Starts the fallback branch of an `IF` block when no preceding condition matched.
hint-keyword-elseif=Adds another condition to an `IF` block. It is tested only when all preceding conditions were false.
hint-keyword-endif=Ends an `IF` block.
hint-keyword-for=Starts a counted loop with an initial value, limit and optional step.
hint-keyword-next=Ends a `FOR` loop and advances its control variable. `ENDFOR` is an equivalent spelling.
hint-keyword-endfor=Ends a `FOR` loop and advances its control variable. `NEXT` is an equivalent spelling.
hint-keyword-break=Immediately leaves the innermost active loop or selection construct.
hint-keyword-continue=Skips the remainder of the current iteration and begins the next iteration of the innermost loop.
hint-keyword-return=Returns from the current function or procedure. A function return supplies its result value.
hint-keyword-gosub=Calls a label as a subroutine. Execution resumes after the `GOSUB` when the subroutine returns.
hint-keyword-goto=Continues execution at the named label.
hint-keyword-select=Starts a multi-branch selection block whose expression is compared with its `CASE` branches.
hint-keyword-case=Introduces one value branch inside a `SELECT` block.
hint-keyword-default=Introduces the fallback branch of a `SELECT` block when no `CASE` matched.
hint-keyword-endselect=Ends a `SELECT` block.
hint-keyword-declare=Declares a function or procedure signature before its implementation, allowing earlier calls and type checking.
hint-keyword-function=Starts a named routine that computes and returns a value.
hint-keyword-procedure=Starts a named routine that performs actions without returning a value.
hint-keyword-endproc=Ends a `PROCEDURE` implementation.
hint-keyword-endfunc=Ends a `FUNCTION` implementation.
hint-keyword-repeat=Starts a post-test loop. Its body runs at least once and is followed by `UNTIL`.
hint-keyword-until=Ends a `REPEAT` loop and stops repetition when its condition becomes true.
hint-keyword-loop=Starts an unconditional loop that continues until control leaves it, normally with `BREAK` or `RETURN`.
hint-keyword-endloop=Ends a `LOOP` block and starts its next iteration.
hint-keyword-const=Declares a compile-time named constant.
hint-keyword-enum=Starts a declaration of named integer constants belonging to one enum type.
hint-keyword-endenum=Ends an `ENUM` declaration.
hint-keyword-type=Starts a user-defined record type declaration containing named fields.
hint-keyword-endtype=Ends a `TYPE` declaration.
hint-keyword-begin=Starts the executable body following declarations in a structured PPL 400 program.
hint-keyword-onerror=Declares the routine or label that handles runtime errors for the current program.
hint-keyword-foreach=Starts a PPL 400 loop over a real array. Each iteration assigns the next element to the loop variable; empty arrays execute no iterations. The element variable must be type-compatible with the array.
hint-keyword-endforeach=Ends a `FOREACH` loop and advances to the next collection element.
hint-keyword-exit=Terminates the current PPE. In PPL 400 this contextual statement replaces the classic `END` statement name.
hint-keyword-usage=Usage
hint-const-builtin=A predefined PPL constant.

hint-type-boolean=unsigned character (1 byte) 0 = FALSE, non-0 = TRUE
hint-type-date=unsigned integer (2 bytes) PCBoard julian date (count of days since 1/1/1900) 
hint-type-ddate=
    Signed long integer for julian date. DDATE is for use with DBase date fields.
    It holds a long integer for julian dates. When coerced to string type it is in the format CCYYMMDD or 19940527
hint-type-integer=signed long integer (4 bytes) Range: -2,147,483,648 → +2,147,483,647
hint-type-money=signed long integer (4 bytes) Range: -$21,474,836.48 → +$21,474,836.47
hint-type-string=far character pointer (4 bytes) NULL is an empty string non-NULL points to a string of some length less than or equal to 256
hint-type-string-unbounded=Unbounded string. Starting with PPL 400, `STRING` is no longer limited to 256 characters.
hint-type-time=signed long integer (4 bytes) Count of seconds since midnight
hint-type-bigstr=Allows up to 2048 characters per big string (up from 256 for STRING variables) May include CHR(0) characters in the middle of the big string (unlike STRING variables which may not)
hint-type-edate=Julian date in earth date format Deals with dates formatted YYMM.DD Range: Same as DATE
hint-type-float=4-byte floating point number Range: +/-3.4E-38 - +/-3.4E+38 (7-digit precision)
hint-type-double=8-byte floating point number Range: +/-1.7E-308 - +/-1.7E+308 (15-digit precision)
hint-type-unsigned=4-byte unsigned integer Range: 0 - 4,294,967,295
hint-type-long=8-byte signed integer Range: -9,223,372,036,854,775,808 - 9,223,372,036,854,775,807
hint-type-ulong=8-byte unsigned integer Range: 0 - 18,446,744,073,709,551,615
hint-type-bytes=Compact, contiguous binary data for encoding, checksums and binary I/O without the per-element cost of a `BYTE[]` array.
hint-bytes-len=Returns the number of bytes in this value.
hint-bytes-to-string=Decodes these bytes as UTF-8 text. Invalid UTF-8 reports `ErrCode.Format`.
hint-bytes-to-base64=Encodes these bytes as base64 text.
hint-bytes-to-hex=Returns uppercase hexadecimal text with two digits per byte, preserving leading zero bytes.
hint-bytes-get-checksum=Calculates the selected `Checksum` algorithm and returns raw bytes: CRC32 produces 4 bytes in network order, MD5 16 bytes and SHA256 32 bytes. An invalid algorithm returns empty `BYTES` and reports `ErrCode.Invalid`.
hint-bytes-from-base64=Decodes padded or unpadded base64 text into raw bytes. ASCII whitespace is ignored so MIME-wrapped input is accepted. Malformed input returns empty `BYTES` and reports `ErrCode.Format`.
hint-type-regex=A compiled regular expression. Matching uses Unicode by default and guarantees linear-time search without look-around or backreferences.
hint-type-regex-match=An immutable snapshot of one regular-expression match and its capture groups.
hint-type-board=Read-only snapshot of board configuration and indexed conference and user collections.
hint-type-session=Live state for the current caller, including the selected conference, area, directory and user.
hint-type-user=A user record. Writable profile fields update the current user immediately; snapshots remain read-only.
hint-type-http=Static entry point for policy-controlled HTTP requests and downloads.
hint-type-http-request=Mutable HTTP request. `SetHeader`, `SetText` and `SetForm` change this request and return whether they succeeded.
hint-type-http-response=HTTP result with status, headers and a bounded retained body, or an invalid value with details in `Error.Last()`.
hint-type-checksum=Algorithm used by `BYTES.GetChecksum`: `CRC32`, `MD5` or `SHA256`.
hint-type-gfx=The caller's graphics session. Use `Terminal.Gfx` to select a backend and control frame pacing.
hint-type-gfx-backend=Graphics transport selected for a session: `None`, `Auto`, `Sixel` or `Jxl`.
hint-type-surface=An off-screen RGBA drawing target. Create one with `Surface.New()` or decode an image with `Surface.Load()`.
hint-member-board-users=The users registered when `Board` was first read, exposed as a read-only `USER[]` snapshot.
hint-member-user-valid=Whether this `USER` represents an existing record. An out-of-range `Board.Users` index returns an empty user with `Valid` false.
hint-member-terminal-gfx=The caller's graphics session. Initialize it before creating or presenting surfaces and shut it down when drawing is complete.
hint-member-terminal-input=Keyboard and mouse input for the caller. Enable event reporting before polling or waiting, and call `Release` to return input to the board.
hint-member-terminal-margins=The terminal's active scrolling and text-output margins.
hint-member-margins-set-vertical=
    Sets 1-based top and bottom rows; `top` must be at least 1 and less than `bottom`.
    <br><br>**Terminal protocol:** sends DECSTBM `CSI top ; bottom r`, bytes `ESC [ top ; bottom r`. Requires a VT/ANSI terminal that implements DECSTBM; unsupported terminals may ignore it.
hint-member-margins-set-horizontal=
    Sets 1-based left and right columns; `left` must be at least 1 and less than `right`.
    <br><br>**Terminal protocol:** first sends DECLRMM `CSI ? 69 h` (DECSET private mode 69), then DECSLRM `CSI left ; right s`. Requires a DEC-compatible terminal with left/right margin support; many basic ANSI terminals do not implement it.
hint-member-margins-reset-vertical=
    Restores the full terminal height. <br><br>**Terminal protocol:** sends DECSTBM reset `CSI r`, bytes `ESC [ r`.
hint-member-margins-reset-horizontal=
    Restores the full terminal width. <br><br>**Terminal protocol:** sends `CSI ? 69 l` (DECRST private mode 69), bytes `ESC [ ? 6 9 l`.
hint-member-margins-reset-all=
    Restores both margin axes. <br><br>**Terminal protocol:** sends DECSTBM reset `CSI r`, followed by DECRST 69 `CSI ? 69 l`.
hint-member-margins-edge=The current 1-based margin edge, or zero when that margin is not active.
hint-member-margins-active=Whether the corresponding terminal margin is active.
hint-member-terminput-poll=Returns the next pending input event without waiting.
hint-member-terminput-wait=Waits up to the requested number of milliseconds for an input event.
hint-member-terminput-mouse-on=Enables mouse events in text-cell or pixel coordinates and returns whether the mode was accepted. Tracking may be `Buttons`, `Drag` or `All`; when omitted it defaults to `MouseTracking.All`. <br><br>**Terminal protocol:** disables DECSET modes 1000, 1002 and 1003, enables respectively 1000 (button), 1002 (drag) or 1003 (all motion), then enables SGR mouse mode 1006. Pixel mode also enables mode 1016 and queries it with `CSI ? 1016 $ p`; text mode disables 1016. Requires xterm/DEC mouse reporting, and pixel mode requires mode 1016.
hint-member-terminput-mouse-off=Disables mouse event reporting. <br><br>**Terminal protocol:** sends DECRST for private modes 1000, 1002, 1003, 1006 and 1016: `CSI ? 1000 l` … `CSI ? 1016 l`.
hint-member-terminput-keyboard-on=Enables physical keyboard events. The optional `echo` flag controls whether translated key input is suppressed. <br><br>**Terminal protocol:** always enables CTerm physical-key mode with `CSI = 1 h`. With `echo = FALSE` it first sends `CSI = 2 l`; with `echo = TRUE` it sends `CSI = 2 h`. These are CTerm/SyncTERM extensions, not standard ANSI.
hint-member-terminput-keyboard-off=Disables physical keyboard events. <br><br>**Terminal protocol:** sends `CSI = 2 l` followed by `CSI = 1 l`. These are CTerm/SyncTERM extensions, not standard ANSI.
hint-member-terminput-release=Disables input reporting and returns keyboard and mouse handling to the board.
hint-type-terminal=The caller's live terminal and the entry point for capabilities, input, graphics, margins, palette, macros, synchronized output and downloadable fonts.
hint-type-terminfo=Read-only snapshot of terminal capabilities negotiated when the session started.
hint-type-terminput=Keyboard and mouse event control for the current caller.
hint-type-margins=The active vertical scrolling region and horizontal text-output margins.
hint-type-palette=Controls the 16 DOS colors used by `COLOR` through terminal OSC palette commands.
hint-type-macros=Records raw terminal output in 64 session-local slots and replays it without interpreting its text or escape sequences.
hint-type-audio=A terminal audio channel. Failed loads return an invalid object; inspect `Error.Last()` for details.
hint-type-error=A snapshot of the last PPL 400 operation result, including subsystem, error code, message and optional channel.
hint-type-event=One keyboard, mouse, queue-overflow or audio-channel event returned by `Terminal.Input`.
hint-type-msg=A read-only message header with lazy access to its message body.
hint-type-conference=A board conference with its message areas, file directories, doors and access checks.
hint-type-area=A conference message area with access checks and read/search operations.
hint-type-directory=A conference file directory with download access information.
hint-type-door=A configured external program or game and its access requirement.
hint-type-contact=One service/account pair from a user's read-only contact list.
hint-type-enum-400=A strongly named PPL 400 enum value. Enum values are stored as their documented integer representation.
hint-member-terminal-info=Read-only capabilities and dimensions negotiated with the caller's terminal.
hint-member-terminal-palette=Controls the 16 DOS palette entries through xterm-compatible OSC 4 and OSC 104 commands.
hint-member-terminal-macros=Records and replays the raw byte stream sent to this caller. Slots are local to the session.
hint-member-terminal-begin-update=
    Starts or nests a synchronized terminal update. Drawing is buffered by supporting terminals until the matching outermost `EndUpdate`, reducing visible flicker. Returns `FALSE` with `ErrKind.Term`/`ErrCode.Unavailable` when unsupported.
    <br><br>**Terminal protocol:** the outermost call sends `CSI ? 2026 h`, bytes `ESC [ ? 2 0 2 6 h`. This is DEC private mode 2026, **Synchronized Output**; support is checked through `Terminal.Info.SynchronizedOutput`.
hint-member-terminal-end-update=
    Ends one synchronized-update nesting level. The terminal presents accumulated output when the outermost level ends. Returns `FALSE` with `ErrCode.Invalid` if no update is active.
    <br><br>**Terminal protocol:** the outermost end sends `CSI ? 2026 l`, bytes `ESC [ ? 2 0 2 6 l` (DECRST private mode 2026).
hint-member-terminal-set-font=
    Selects font number 0–255 for attribute slot 0–3. If `slot` is omitted, the font is assigned to all four slots; `LoadFont` can upload writable font numbers 43–255.
    <br><br>**Terminal protocol:** sends the SyncTERM/CTerm extension `CSI slot ; font SP D`, bytes `ESC [ slot ; font SPACE D`, once per selected attribute slot. This is not standard ANSI.
hint-member-terminal-load-font=
    Loads a board-relative font file, decodes it with `BitFont`, and uploads it into writable terminal font number 43–255. File, format and per-session upload limits are reported through `Error.Last()`.
    <br><br>**Terminal protocol:** sends the CTerm DCS `DCS CTerm:Font:font:base64 ST`, bytes `ESC P CTerm:Font:… ESC \`. The payload is a base64-encoded 256-glyph bitmap. This is a CTerm extension, not standard ANSI.
hint-member-terminfo-program=Detected terminal program: `IcyTerm`, `SyncTerm`, or `Unknown`.
hint-member-terminfo-device-attrs=Raw primary/secondary device-attribute response retained during terminal negotiation.
hint-member-terminfo-cells=Current terminal size in text columns or rows.
hint-member-terminfo-utf8=Whether the connection negotiated UTF-8 text instead of a legacy code page.
hint-member-terminfo-rip=Negotiated RIP graphics version, or an empty string when RIP is unavailable.
hint-member-terminfo-cterm=Detected CTerm protocol revision, or zero when CTerm extensions are unavailable.
hint-member-terminfo-graphics=Whether this terminal advertises the selected inline graphics transport (`Sixel`, JPEG XL, or any inline blob transport).
hint-member-terminfo-audio=Whether terminal-side audio playback is available.
hint-member-terminfo-physical-keys=Whether the terminal can report physical key transitions independently of translated text input.
hint-member-terminfo-pixel-mouse=Whether mouse coordinates can be reported in pixels (CTerm revision 1330 or newer).
hint-member-terminfo-client-blit=Whether client-side image blitting is available (CTerm revision 1318 or newer).
hint-member-terminfo-synchronized-output=Whether DEC private mode 2026 synchronized output is advertised for `BeginUpdate` and `EndUpdate`.
hint-member-terminfo-terminal-macros=Whether terminal macro facilities were negotiated.
hint-member-terminfo-cell-pixels=Detected width or height of one text cell in pixels; zero when unknown.
hint-member-terminfo-screen-pixels=Detected physical screen width or height in pixels; zero when unknown.
hint-member-palette-set=
    Replaces DOS color 0–15 with packed `0xRRGGBBAA`; alpha is ignored. Invalid colors set `ErrCode.Invalid`.
    <br><br>**Terminal protocol:** sends xterm `OSC 4 ; index ; rgb:RR/GG/BB ST`, bytes `ESC ] 4 ; index ; rgb:RR/GG/BB ESC \`. Requires an xterm-compatible writable palette. DOS indices map to ANSI as `0,4,2,6,1,5,3,7,8,12,10,14,9,13,11,15`.
hint-member-palette-reset=
    Restores one DOS color 0–15 to the terminal default.
    <br><br>**Terminal protocol:** sends xterm `OSC 104 ; index ST`, bytes `ESC ] 104 ; index ESC \`, after DOS-to-ANSI index translation.
hint-member-palette-reset-all=
    Restores all 16 colors to the DOS default palette.
    <br><br>**Terminal protocol:** sends one combined OSC 4 command containing all 16 `index ; rgb:RR/GG/BB` pairs, terminated by ST (`ESC \`). Requires xterm-compatible OSC 4 support.
hint-member-macros-recording=Whether a terminal macro is currently recording.
hint-member-macros-begin-record=Starts recording raw output for slot 0–63. Text, control characters and ANSI/VT sequences are preserved verbatim; each stream is limited to 512 KiB. No terminal command is emitted until recorded bytes are replayed.
hint-member-macros-end-record=Stops the active recording and stores its caller/sysop byte streams. No terminal sequence is sent.
hint-member-macros-play=Writes the recorded bytes directly to the caller/sysop terminal stream. The bytes may contain any ANSI/VT or proprietary sequences originally recorded; no transformation or capability check is performed.
hint-member-macros-delete=Deletes one session-local recording. No terminal sequence is sent.
hint-member-macros-delete-all=Deletes all session-local recordings. No terminal sequence is sent.
hint-member-audio-valid=Whether this value identifies a successfully loaded terminal audio channel.
hint-member-audio-playing=Whether this loaded channel is currently tracked as active.
hint-member-audio-set-volume=Sets the logical volume from 0 through 100 and returns whether it succeeded. On failure, `Error.Last()` provides `ErrKind.Audio`, an error code, message and channel.
hint-member-audio-channel=Logical PPL channel 0–13; it maps to SyncTERM/CTerm channel 2–15.
hint-member-audio-play=Starts playback; optional `looping` defaults to `FALSE`. <br><br>**Terminal protocol:** sends `APC SyncTERM:A;Load;S=slot;cache ST`, `…;Volume;C=channel;V=dB ST`, then `…;Queue;C=channel;S=slot[;L] ST`. Non-looping playback also sends `…;Update;C=channel ST`. Requires SyncTERM/CTerm audio APC support.
hint-member-audio-stop=Stops this channel without freeing its data. <br><br>**Terminal protocol:** sends `APC SyncTERM:A;Flush;C=channel;O=0 ST` (`APC` = `ESC _`, `ST` = `ESC \`).
hint-member-audio-fade=Fades this channel to `targetVolume` over `durationMs`. <br><br>**Terminal protocol:** sends `APC SyncTERM:A;Volume;C=channel;V=dB;T=durationMs ST`; volume 0–100 is converted to decibels.
hint-member-audio-free=Stops and releases this channel. <br><br>**Terminal protocol:** sends `APC SyncTERM:A;Flush;C=channel;O=0 ST`; the client cache may remain for reuse.
hint-member-audio-load=Loads a board-relative WAV, AIFF, FLAC, Ogg/Vorbis or Opus file up to 16 MiB. <br><br>**Terminal protocol:** probes using `APC SyncTERM:Q;libsndfileFormat;major;subtype ST`, expects `CSI = 7 ; 101 ; major ; subtype ; supported n`, and uploads with `APC SyncTERM:C;S;cacheName;base64 ST`. Requires SyncTERM/CTerm media and audio extensions.
hint-member-audio-stop-all=Stops every active PPL audio channel by sending `APC SyncTERM:A;Flush;C=channel;O=0 ST` for each one.
hint-member-error-ok=Whether `Code` equals `ErrCode.Ok`.
hint-member-error-kind=Subsystem that produced the result, such as file, graphics, font, audio, terminal, message, network, user, string or regex.
hint-member-error-code=Machine-readable result category such as invalid input, unavailable capability, I/O failure, limit or timeout.
hint-member-error-message=Human-readable diagnostic detail. Programs should branch on `Kind` and `Code`, not translated prose.
hint-member-error-channel=Related audio/media channel, or `-1` when the result is not channel-specific.
hint-member-error-last=Returns a stable copy of the most recently published PPL 400 operation result.
hint-member-error-clear=Clears the stored error and returns `TRUE`.
hint-member-event-kind=Discriminator for this event. Read only the properties meaningful for the selected `EventKind`.
hint-member-event-key=Translated key code or text for `EventKind.Key`; other kinds return zero or an empty string.
hint-member-event-scan-code=Physical key code for `EventKind.KeyEdge`; other kinds return zero.
hint-member-event-pressed=Press state for `EventKind.Key` or `EventKind.KeyEdge`; other kinds return `FALSE`.
hint-member-event-repeated=Repeat flag for `EventKind.KeyEdge`; other kinds return `FALSE`.
hint-member-event-position=Mouse position and coordinate mode for `EventKind.Mouse`; other kinds return neutral values.
hint-member-event-mouse=Mouse action, changed button, or wheel delta for `EventKind.Mouse`; other kinds return neutral values.
hint-member-event-time=Terminal event timestamp in milliseconds.
hint-member-event-channel=Audio channel reported as drained by an `EventKind.Audio` event.
hint-member-event-dropped=Number of queue entries lost before an `EventKind.Overflow` event.
hint-member-event-buttons=Whether the corresponding mouse button was held for `EventKind.Mouse`; other kinds return `FALSE`.
hint-member-event-modifiers=Whether the corresponding modifier was active for `EventKind.Key` or `EventKind.Mouse`; other kinds return `FALSE`.
hint-member-msg-number=Message number, reply target, or stored body size from the read-only JAM header.
hint-member-msg-valid=Whether this value names an existing message. Missing or out-of-range reads return an invalid `MSG` instead of failing the member chain.
hint-member-msg-header=Read-only sender, recipient, subject, or status text from the message header.
hint-member-msg-written=Message creation date or time; an invalid message returns zero.
hint-member-msg-flags=Read-only message attribute derived from its JAM header.
hint-member-msg-text=Loads and returns the message body on demand. I/O failures return an empty string and update `Error.Last()`.
hint-member-contact-service=Name of the contact service, such as email, web, IRC or another configured service.
hint-member-contact-account=Account name or address on this contact's service.
hint-member-conference-identity=Read-only conference name, configured number, or validity flag.
hint-member-conference-options=Read-only conference behavior and messaging policy.
hint-member-conference-password=Protected conference password value; it can be compared by access checks but is not exposed as plain text.
hint-member-conference-collections=Read-only one-dimensional snapshot of this conference's file directories, message areas, or doors.
hint-member-conference-access=Checks the current caller's security and conference configuration for general access, posting, or attachments.
hint-member-area-identity=Read-only message-area name, configured number, or validity flag.
hint-member-area-options=Read-only area policy and QWK/echo-network metadata.
hint-member-area-access=Checks whether the current caller may access, enter, or attach files in this area.
hint-member-area-range=Lowest or highest currently available message number in this area's message base.
hint-member-area-read=Reads an exact message number. A missing message returns an invalid `MSG`; inspect its `Valid` property.
hint-member-area-find=Finds the next message whose `To`, `From`, or `Subject` field contains the text. Optional `startAfter` continues after a message number.
hint-member-directory-identity=Read-only file-directory name, configured number, or validity flag.
hint-member-directory-options=Read-only storage path, free-download/new-file state, or protected password.
hint-member-directory-access=Checks the current caller's security for directory access or downloading.
hint-member-door-identity=Read-only door name, configured number, or validity flag.
hint-member-door-options=Read-only description, executable path, or protected password for this external program.
hint-member-door-access=Checks whether the current caller meets this door's security requirement.
hint-member-board-property=Read-only board identity, location, operator/sysop name, or configured node count.
hint-member-board-conferences=Read-only `CONFERENCE[]` snapshot of configured conferences.
hint-member-user-editor-mode=Preferred full-screen editor policy: always use it, never use it, or ask each time.
hint-member-user-profile=User profile or usage statistic. `Session.User` is live and writable where supported; entries from `Board.Users` are read-only snapshots.
hint-enum-event-kind-none=No event was available, for example after a non-blocking `Poll` or a timed-out `Wait`.
hint-enum-event-kind-value=Selects which `EVENT` property group is meaningful: translated key, physical key edge, mouse, queue overflow, or audio completion.
hint-enum-mouse-action=Mouse transition reported by `EVENT.Action`: none, press, release, motion, or wheel.
hint-enum-mouse-button=Mouse button or wheel direction that generated an event; held buttons are exposed separately as booleans.
hint-enum-mouse-mode-text=Reports mouse coordinates in 1-based terminal text cells.
hint-enum-mouse-mode-pixels=Reports mouse coordinates in pixels; requires `Terminal.Info.PixelMouse`.
hint-enum-mouse-tracking=Selects button-only, drag-motion, or all-motion mouse reporting.
hint-enum-error-kind=Subsystem category used by `Error.Kind`; `None` means no subsystem error.
hint-enum-error-code=Portable operation result used by `Error.Code`: success, unavailable, invalid, I/O, format, limit, unsupported, stack, denied, or timeout.
hint-enum-editor-mode=User preference for the full-screen editor: `Yes`, `No`, or `Ask`.
hint-enum-msg-field=Message-header field searched by `AREA.Find`: recipient, sender, or subject.
hint-enum-http-method=HTTP request method accepted by the policy-controlled request builder: GET, HEAD, or POST.
hint-enum-regex-options=Bit flags for regex compilation: `None`, `IgnoreCase`, `MultiLine`, `DotMatchesNewLine`, `IgnoreWhitespace`, `SwapGreed` and `Ascii`. Combine flags with `|`.
hint-enum-string-comparison=Ordinal Unicode comparison, either case-sensitive or case-insensitive.
hint-enum-checksum=Algorithm used by `Bytes.GetChecksum`: `CRC32` returns 4 raw bytes in network order, `MD5` 16 bytes and `SHA256` 32 bytes. Call `ToHex()` when text is required.
hint-member-gfx-init=
    Starts a graphics session with the requested `GfxBackend`. `Auto` chooses the best terminal capability; fullscreen defaults to `TRUE`.
    <br><br>**Terminal protocol:** `Auto` uses negotiated Sixel/JPEG-XL capabilities; an explicit Sixel request performs no additional probe. Fullscreen sends `CSI 2 J`, `CSI H`, then `CSI ? 25 l`, `CSI ? 7 l`, `CSI ? 80 l`, and `CSI ? 1070 l`. Sixel requires Sixel DCS support; JPEG XL requires SyncTERM/CTerm media APC support.
hint-member-gfx-shutdown=
    Ends the graphics session and restores normal text output.
    <br><br>**Terminal protocol:** fullscreen sends `CSI ? 80 h`, `CSI ? 7 h`, and `CSI ? 25 h`. After Sixel output it restores all 16 DOS colors with OSC 4, then sends SGR reset `CSI 0 m` and restores the board color.
hint-member-gfx-backend=The selected read-only `GfxBackend`, or `None` when no graphics session is active.
hint-member-gfx-set-pacing=Enables or disables frame pacing and returns whether it succeeded. When enabled, presentation waits for terminal acknowledgement before sending another frame. On failure, `Error.Last()` provides details.
hint-param-backend=Graphics backend to request; `Auto` chooses the best available backend.
hint-param-enabled=Whether frame pacing should be enabled.
hint-parameters-title=Parameters
hint-param-optional=optional
hint-param-fullscreen=Whether to clear the screen and switch the terminal into fullscreen graphics mode. Defaults to `TRUE`.
hint-param-top=1-based first row of the vertical scrolling region.
hint-param-bottom=1-based last row of the vertical scrolling region.
hint-param-left=1-based first column of the horizontal margin region.
hint-param-right=1-based last column of the horizontal margin region.
hint-param-timeout-ms=Maximum time to wait, in milliseconds.
hint-param-mode=Input, rendering, or operation mode to use.
hint-param-tracking=Mouse tracking policy; defaults to `MouseTracking.All` when omitted.
hint-param-echo=Whether accepted keyboard input is also echoed. Defaults to `FALSE`.
hint-param-color=DOS palette color number from 0 through 15.
hint-param-rgba=Packed color in `0xRRGGBBAA` format.
hint-param-slot=Target slot number.
hint-param-looping=Whether playback restarts after reaching the end. Defaults to `FALSE`.
hint-param-duration-ms=Fade duration in milliseconds.
hint-param-target-volume=Final volume from 0 through 100.
hint-param-volume=Volume from 0 through 100.
hint-param-font=Terminal font number.
hint-param-file=Board-relative source or destination file name.
hint-param-password=New plain-text password to validate and store securely.
hint-param-service=Contact service name, such as an email or chat provider.
hint-param-account=Account name or address for the selected service.
hint-param-index=Zero-based item or capture-group index.
hint-param-text=Input or replacement text used by the operation.
hint-param-url=Absolute HTTP URL permitted by the board network policy.
hint-param-method=HTTP method to use: `Get`, `Head`, or `Post`.
hint-param-name=Header, group, or field name to select.
hint-param-value=Value assigned to the selected name.
hint-param-content-type=Optional MIME content type of the text body.
hint-param-form=Optional dialect switch; `TRUE` (the default) uses `application/x-www-form-urlencoded` rules where a space is `+`, `FALSE` uses RFC 3986 rules where a space is `%20`.
hint-param-pattern=Regular-expression pattern to compile or validate.
hint-param-options=Optional `RegexOptions` flags; defaults to `RegexOptions.None`.
hint-param-start=Optional zero-based Unicode character position at which searching begins.
hint-param-limit=Optional maximum result or replacement count; zero uses the documented unlimited bound.
hint-param-replacement=Replacement template; `$1` and `$name` expand capture groups.
hint-param-message-number=Message number to read.
hint-param-field=Field selected for the operation.
hint-param-start-message=Optional message number at which searching begins.
hint-param-x=Zero-based horizontal pixel coordinate.
hint-param-y=Zero-based vertical pixel coordinate.
hint-param-width=Width in pixels.
hint-param-height=Height in pixels.
hint-param-source=Source surface to copy from.
hint-param-source-x=Zero-based left edge of the source rectangle.
hint-param-source-y=Zero-based top edge of the source rectangle.
hint-param-source-width=Width of the source rectangle in pixels.
hint-param-source-height=Height of the source rectangle in pixels.
hint-param-destination-x=Zero-based horizontal destination pixel coordinate.
hint-param-destination-y=Zero-based vertical destination pixel coordinate.
hint-param-column=1-based terminal text column.
hint-param-row=1-based terminal text row.
hint-param-destination-width=Optional displayed width in pixels; omission keeps the source width.
hint-param-destination-height=Optional displayed height in pixels; omission keeps the source height.
hint-param-flip=Optional flip flags used by the graphics backend.
hint-member-gfx-backend-none=No graphics backend is active.
hint-member-gfx-backend-auto=Selects the best graphics backend advertised by `Terminal.Info`.
hint-member-gfx-backend-sixel=Uses Sixel graphics.
hint-member-gfx-backend-jxl=Uses the JPEG XL graphics protocol.
hint-member-surface-dimension=The read-only surface dimension in pixels.
hint-member-surface-valid=Whether this surface refers to a live image. Resource failures return an invalid surface and set `Error.Last()`.
hint-member-surface-clear=Fills the entire surface with a packed `0xRRGGBBAA` color.
hint-member-surface-set-pixel=Writes a packed `0xRRGGBBAA` color at the zero-based pixel coordinates.
hint-member-surface-get-pixel=Returns the packed `0xRRGGBBAA` color at the zero-based pixel coordinates.
hint-member-surface-fill-rect=Fills the pixel rectangle with a packed `0xRRGGBBAA` color.
hint-member-surface-draw-rect=Outlines the pixel rectangle with a packed `0xRRGGBBAA` color.
hint-member-surface-blit=Alpha-composites the source surface at the destination pixel coordinates.
hint-member-surface-blit-rect=Alpha-composites a source pixel rectangle at the destination pixel coordinates.
hint-member-surface-present=Presents the entire surface at the current terminal position. <br><br>**Terminal protocol:** Sixel sends a Sixel DCS image. JPEG XL uses `APC SyncTERM:C;DrawJXLBlob;DX=x;DY=y;base64 ST`, or uploads with `…;S;cacheName;base64 ST` and draws with `…;DrawJXL;DX=x;DY=y;cacheName ST`.
hint-member-surface-present-at=Presents the entire surface at the given 1-based text column and row. <br><br>**Terminal protocol:** Sixel sends `CSI ? 1070 h`, saves with `ESC 7`, moves with `CSI row ; column H`, sends Sixel DCS, restores with `ESC 8`, then sends `CSI ? 1070 l`. JPEG XL converts cells to pixels and uses SyncTERM `DrawJXLBlob`/`DrawJXL` APC.
hint-member-surface-present-rect=Presents a source pixel rectangle with optional destination, scaling and flip. <br><br>**Terminal protocol:** identity output may use Sixel DCS. Scaling or flipping requires JPEG XL and SyncTERM/CTerm `DrawJXLBlob` or `DrawJXL` APC with `DX`, `DY`, and transform options.
hint-member-surface-pin=Uploads an immutable JPEG-XL client buffer. <br><br>**Terminal protocol:** sends `APC SyncTERM:C;LoadJXLBlob;B=buffer;base64 ST`. Requires the JPEG-XL backend and SyncTERM/CTerm client-buffer support.
hint-member-surface-unpin=Releases the server-side association with the pinned JPEG-XL client buffer. No terminal sequence is sent; the client may retain its cached buffer.
hint-member-surface-free=Releases the surface and its resident pixel memory.
hint-member-surface-new=Creates a transparent surface with the requested pixel dimensions. Surfaces are limited to 2048 by 2048 pixels.
hint-member-surface-load=Decodes an image file into a surface. The source file is limited to 32 MiB.
hint-member-session-context=The object currently selected by this live session. Read it again after changing conference, area, directory or user state.
hint-member-session-value=A live, read-only value from the current caller's session.
hint-member-user-record-number=The zero-based persistent user record number, or `-1` when this is not a stored user.
hint-member-user-contacts=Read-only `CONTACT[]` snapshot, limited to 100 entries. Use AddContact and RemoveContact to change the current user.
hint-member-user-notes=Read-only `STRING[]` snapshot of the five note slots. Use SetNote to update the current user.
hint-member-user-set-password=Validates and hashes a new password for the current user. Snapshots cannot change passwords.
hint-member-user-contact-method=Adds or removes a contact on the current user and reports failures through `Error.Last()`.
hint-member-user-set-note=Updates one of the five note slots on the current user.
hint-http-get=Performs a policy-controlled GET request and returns an `HTTPRESPONSE`. Transport or policy failures return an invalid response and report details through `Error.Last()`; HTTP error statuses remain valid responses.
hint-http-new=Creates a mutable `HTTPREQUEST` for `HttpMethod.Get`, `Head` or `Post` and the supplied URL. It performs no network I/O until `Send()` is called.
hint-http-download=Streams a successful GET response to a temporary board-relative file and commits it atomically only after completion. The configured response-size limit applies; failures leave the destination unchanged and return an invalid response.
hint-http-url-encode=Percent-encodes one form field or URL component. Encode single values only, never a whole `name=value&...` string, because the separators must stay unencoded.
hint-http-url-decode=Reverses `UrlEncode()`. Byte sequences that are not valid UTF-8 are replaced rather than reported.
hint-http-request-property=Read-only request metadata.
hint-http-request-set-header=Sets this request's header and returns whether it succeeded. Restricted or malformed headers return `FALSE`; `Error.Last()` reports `ErrKind.Net` and `ErrCode.Invalid`.
hint-http-request-set-text=Sets this request's UTF-8 body and optional content type, returning whether it succeeded. GET and HEAD requests return `FALSE`; `Error.Last()` reports the failure. The body is sent verbatim, so form fields must be encoded with `Http.UrlEncode()` first.
hint-http-request-set-form=Appends one percent-encoded `application/x-www-form-urlencoded` field to this request's body and sets that content type. Repeated calls accumulate and keep duplicate names. GET and HEAD requests, or a body that is not already form data, return `FALSE`.
hint-http-request-send=Sends this request under the board's HTTP policy and returns its response.
hint-http-response-property=Read-only response metadata. Check Valid before relying on network results and OK for a 2xx status.
hint-http-response-text=Decodes the retained response body strictly as UTF-8. Invalid text reports `ErrCode.Format`.
hint-http-response-header=Returns the selected response header, or an empty string when it is absent.
hint-http-response-save=Writes the retained response body atomically to a board-relative file.
hint-regex-valid=Whether the regular expression compiled successfully.
hint-regex-pattern=The source pattern used to compile this regular expression.
hint-regex-compile=Compiles a pattern with optional `RegexOptions`. An invalid pattern returns an invalid `REGEX` and sets `Error.Last()`.
hint-regex-escape=Escapes all regular-expression metacharacters in literal text.
hint-regex-is-valid=Reports whether a pattern and optional options can be compiled without reporting an error.
hint-regex-is-match=Reports whether the text has a match at or after the optional zero-based Unicode character position.
hint-regex-find=Returns the first match at or after the optional zero-based Unicode character position.
hint-regex-find-all=Returns matches at or after an optional zero-based start position. A positive limit restricts the result count; zero is unlimited up to 100,000.
hint-regex-replace=Replaces all matches, or at most a positive limit, using `$1` and `$name` capture expansion.
hint-regex-split=Returns a dynamic BIGSTR array by splitting text. Empty fields are retained and a positive limit leaves the remainder in the final element.
hint-regex-match-success=Whether a match was found.
hint-regex-match-value=The complete matched text.
hint-regex-match-start=The zero-based Unicode character position of the match, or -1 when no match was found.
hint-regex-match-length=The match length in Unicode characters.
hint-regex-match-group-count=The number of capture groups, excluding group zero for the complete match.
hint-regex-match-group=Returns a numbered capture. Group zero is the complete match.
hint-regex-match-named-group=Returns a named capture.
hint-regex-match-group-matched=Reports whether the selected optional capture participated in the match.
hint-regex-match-group-start=Returns the selected capture's zero-based Unicode character position, or -1 when it did not participate.
hint-regex-match-group-length=Returns the selected capture's length in Unicode characters.
hint-string-len=Returns the number of Unicode characters in the string.
hint-string-find=Returns the zero-based position of the first match, optionally at or after a start position and with a StringComparison; -1 means not found.
hint-string-find-last=Returns the zero-based position of the last match, optionally at or before a start position and with a StringComparison; -1 means not found.
hint-string-contains=Reports whether the string contains a non-empty search string, optionally using a StringComparison.
hint-string-starts-with=Reports whether the string starts with the prefix, optionally using a StringComparison.
hint-string-ends-with=Reports whether the string ends with the suffix, optionally using a StringComparison.
hint-string-count=Counts non-overlapping occurrences of a non-empty search string, optionally using a StringComparison.
hint-string-equals=Reports string equality, optionally using a StringComparison.
hint-string-replace=Replaces all occurrences of a substring and returns a `BIGSTR`.
hint-string-trim=Removes whitespace, or supplied characters, from both ends and returns a `BIGSTR`.
hint-string-trim-start=Removes whitespace, or supplied characters, from the start and returns a `BIGSTR`.
hint-string-trim-end=Removes whitespace, or supplied characters, from the end and returns a `BIGSTR`.
hint-string-to-upper=Converts the string to uppercase and returns a `BIGSTR`.
hint-string-to-lower=Converts the string to lowercase and returns a `BIGSTR`.
hint-string-split=Returns a dynamic BIGSTR array by splitting the string. Empty elements are retained; an optional limit keeps the unsplit remainder in the last element.
hint-string-join=Joins a one-dimensional string array with a separator and returns a `BIGSTR`.
hint-string-repeat=Repeats a string the requested number of times and returns a `BIGSTR`.
hint-type-byte=1-byte unsigned integer Range: 0 - 255
hint-type-word=2-byte unsigned integer Range: 0 - 65,535
hint-type-sbyte=1-byte signed Integer Range: -128 - 127
hint-type-sword=2-byte signed integer Range: -32,768 - 32,767

hint-statement-end=Ends the program execution
hint-statement-cls=Clears the screen
hint-statement-clreol=Clears to the end of the line
hint-statement-more=Pauses and waits for a keypress (Displays a MORE? prompt)
hint-statement-wait=Pauses and waits for a keypress
hint-statement-color=Sets the text color to @1
hint-statement-goto=Jumps to the label specified
hint-statement-let=Assigns the value of `exp` to `var1`
hint-statement-print=
    Print a line to the screen

    ### Remarks
    This statement will process all @ codes and display them as expected.
hint-statement-println=
    Print a line to the screen and append a newline to the end of the expression(s).

    ### Remarks
    This statement will process all @ codes and display them as expected.
hint-statement-confflag=Turn on the conference @1 flags specified by @2
hint-statement-confunflag=Turn off the conference @1 flags specified by @2
hint-statement-dispfile=
    Display file @1 with @2 alternate file flags
    ### Valid Flags
    - `GRAPH`
    - `SEC`
    - `LANG`
hint-statement-input=Display @1 and get input from user, assigning it to @2 (60 characters maximum)
hint-statement-fcreate=
    Use channel @1 to create and open file @2 in access mode @3 and share mode @4
    | Valid | Values |
    | :--- | :--- |
    | Channels     | `0` - `7` (`0` is used for surveys) |
    | Access Modes | `O_RD`, `O_WR`, `O_RW` (should use `O_WR`) |
    | Share Modes  | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fopen=
    Use channel @1 to open file @2 in access mode @3 and share mode @4
    | Valid | Values |
    | :--- | :--- |
    | Channels     | `0` - `7` (`0` is used for surveys) |
    | Access Modes | `O_RD`, `O_WR`, `O_RW` (should use `O_WR`) |
    | Share Modes  | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fappend=
    Use channel @1 to append to file @2 in access mode @3 and share mode @4
    | Valid | Values |
    | :--- | :--- |
    | Channels     | `0` - `7` (`0` is used for surveys) |
    | Access Modes | `O_RD`, `O_WR`, `O_RW` (should use `O_WR`) |
    | Share Modes  | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fclose=
    Close channel @1

    Accept channel -1 as the `ReadLine()` function 'channel' and close it
hint-statement-fget=Read a line from channel @1 and assign it to @2
hint-statement-fput=Write one or more @2 out to channel @1
hint-statement-fputln=Write one or more @2 out to channel @1 and terminate with a carriage return/line feed pair
hint-statement-resetdisp=Reset the display after an user abort
hint-statement-startdisp=
    Start display monitoring in mode @1
    ### Valid Modes
    - `NC`
    - `FNS`
    - `FCL`
hint-statement-fputpad=Write out @2, padding or truncating to length @3 as needed, to channel @1
hint-statement-hangup=Hangup on the user without any notification
hint-statement-getuser=Fill the predefined variables (U_…) with current information from the user record
hint-statement-putuser=
    Write the information from the predefined variables (U_…) to the user record
    This statement is only intended to update user information if a successful GetUser or GetAltUser was issued previously.
    This was done to ensure that information for the current user wasn't written to another user or vice versa.
hint-statement-defcolor=Resets the current color to the system default
hint-statement-delete=Deletes the filename specified by @1 (`ERASE` is a synonym)
hint-statement-deluser=Flags the current user record for deletion
hint-statement-adjtime=Add or subtract @1 minutes to the users time available this session
hint-statement-log=Write string @1 to the callers log, left justified if @2 is `TRUE`
hint-statement-inputstr=
    Display @1 in color @3 and get a string (maximum length @4, valid characters @5, flags @6) from the user, assigning it to @2

    ### Valid Flags
    `ECHODOTS`, `FIELDLEN`, `GUIDE`, `UPCASE`, `STACKED`, `ERASELINE`, `NEWLINE`, `LFBEFORE`, `LFAFTER`, `WORDWRAP`, `NOCLEAR`, `HIGHASCII`, `AUTO`, `YESNO`  
hint-statement-inputyn=
    Display @1 in color @3 and get a yes/no response from the user, assigning it to @1 (1 characters maximum, valid characters determined by language)
hint-statement-inputmoney=
    Display @1 in color @3 and get a money formatted string from the user, assigning it to @1 (13 characters maximum, valid characters `0-9 $ .`)
hint-statement-inputint=
    Display @1 in color @3 and get an integer formatted string from the user, assigning it to @1 (11 characters maximum, valid characters `0-9`)
hint-statement-inputcc=
    Display @1 in color @3 and get a credit card formatted string from the user, assigning it to @1 (16 characters maximum, valid characters `0-9`)
hint-statement-inputdate=
    Display @1 in color @3 and get a date formatted string from the user, assigning it to @1 (8 characters maximum, valid characters `0-9 - /`)
hint-statement-inputtime=
    Display @1 in color @3 and get a time formatted string from the user, assigning it to @1 (8 characters maximum, valid characters `0-9 :`)
hint-statement-gosub=Transfer control to `LABEL`, marking the current PPE location for a future Return statement (`GO SUB` is a synonym)
hint-statement-return=Return to the statement after the last `GoSub` or, if no `GoSub` is waiting for a `RETURN`, end the PPE
hint-statement-promptstr=
    Display PCBTEXT entry @1 and get a string (maximum length @3, valid characters @4, flags @5) from the user, assigning it to @1
    ### Valid Flags
    `ECHODOTS`, `FIELDLEN`, `GUIDE`, `UPCASE`, `STACKED`, `ERASELINE`, `NEWLINE`, `LFBEFORE`, `LFAFTER`, `WORDWRAP`, `NOCLEAR`, `HIGHASCII`, `AUTO`, `YESNO`  
hint-statement-dtron=Turn on the DTR signal
hint-statement-dtroff=
    Turn off the DTR signal,

    Note: on most modems, lowering DTR will cause modem to hangup… this is a good way if you want to simulate a bad connection,
    and then hangup without goodbye screens… This is the best way for you, the nice sysop, to free your line quickly… :)
hint-statement-cdchkon=Turn on carrier detect checking
hint-statement-cdchkoff=Turn off carrier detect checking
hint-statement-delay=Pause for @1 clock ticks (1 clock tick = 1/18.2 second)
hint-statement-sendmodem=Send the text in @1 out to the modem
hint-statement-inc=Increment the value of @1
hint-statement-dec=Decrement the value of @1
hint-statement-newline=Write a newline to the display
hint-statement-newlines=Write @1 newlines to the display
hint-statement-tokenize=Tokenize string @1 into individual items separated by semi-colons or spaces
hint-statement-gettoken=
    ### Returns
    The next string token from a prior call to `Tokenize` (Same as the `GETTOKEN` statement but can be used in an expression without prior assignement to a variable)
    
    ### Example
    `GETTOKEN VAR`
    
    Get a token from a previous call to Tokenize and assign it to `VAR`
hint-statement-shell=
    Shell (via COMMAND.COM if @1 is `TRUE`) to program/command @2 with arguments @3, saving the return value in @1
    NOTE: If @1 is `TRUE`, the value assigned to @1 will be the return code of COMMAND.COM, not @3)
hint-statement-disptext=
    Display PCBTEXT prompt @1 using flags @2

    ### Valid Flags
    `NEWLINE`, `LFBEFORE`, `LFAFTER`, `BELL`, `LOGIT`, `LOGITLEFT`
hint-statement-stop=Abort PPE execution without appending answers (channel 0) to the answer file
hint-statement-inputtext=Display @1 in color @3 and get a string (maximum length @4) from the user, assigning it to @1
hint-statement-beep=Beeps the speaker
hint-statement-push=Push a list of evaluated expressions onto the stack
hint-statement-pop=Pop values (previously pushed onto the stack) into a list of variables
hint-statement-kbdstuff=Stuff the keyboard buffer with the contents of @1
hint-statement-call=Load and execute PPE filename specified by @1
hint-statement-join=Performs a join conference command, passing it @1 as arguments
hint-statement-quest=Do script questionnaire @1
hint-statement-blt=Display bulletin number @1
hint-statement-dir=Performs a file directory command, passing it @1 as arguments
hint-statement-kbdfile=Stuff the keyboard buffer with the contents of file @1
hint-statement-bye=Same as having the user type BYE from the command prompt
hint-statement-goodbye=Same as having the user type G from the command prompt
hint-statement-broadcast=Broadcast message @3 to nodes from @1 to @2 inclusive
hint-statement-waitfor=
    Wait up to @3 seconds for the string @1, assigned `TRUE` to @1 if the string is found in the time specified or `FALSE` if the string is not found (`WAIT FOR` is a synonym)
hint-statement-kbdchkon=Turn on keyboard time out checking
hint-statement-kbdchkoff=Turn off keyboard time out checking
hint-statement-optext=Writes string @1 into the `@OPTEXT@` macro
hint-statement-dispstr=Display file if @1 is `“%filename”`, execute PPE if @1 is `“!filename”`, or display string @1
hint-statement-rdunet=Read information from USERNET.XXX for node @1
hint-statement-wrunet=
    Write information to USERNET.XXX for node @1, where @2 is the new node status,
     @3 is the new node user name, 
     @4 is the new node city, 
     @5 is the new node operation text, 
     and @6 is broadcast text
hint-statement-dointr=Generate interrupt number “intr” (0-255) with the register values passed as parameters
hint-statement-varseg=Assign the segment address of @1 to @2
hint-statement-varoff=Assign the offset address of @1 to @2
hint-statement-pokeb=Assign the value @2 (0-255) to memory address @1 (POKE is a synonym)
hint-statement-pokew=Assign the value @2 (0-65535) to memory address @1
hint-statement-varaddr=Assign the address (segment and offset) of @1 to @2
hint-statement-ansipos=
    Move the cursor to column @1 and row @2

    ```
    1 <= @1 <= 80  
    1 <= @2 <= 23 (Because of the status lines)  
    ```
    (1,1) is the top left corner of the screen
hint-statement-backup=Backup (move the cursor to the left) @1 columns without going past column 1
hint-statement-forward=Move the cursor forward @1 columns without going past column 80
hint-statement-freshline=If the cursor is not in column 1, do a newline
hint-statement-wrusys=Writes (creates) a USERS.SYS file which can be used by a SHELLed application
hint-statement-rdusys=Reads a USERS.SYS file, if present, and updates the users record
hint-statement-newpwd=
    Changes the current user's password with PSA validation.

    `@1` is the new password. `@2` receives `TRUE` when it was accepted or `FALSE` when validation failed. Password history, expiration and the change counter are updated on success.
hint-statement-opencap=
    Open @1 and capture all screen output to it.
    If an error occurs creating or opening @1, @2 is set to `TRUE`, otherwise @2 is set to `FALSE`.
hint-statement-closecap=Close the capture file previously opened with OpenCap
hint-statement-message=
    Write a message in conference @1, to user @2 (empty string defaults to current caller), 
    from user @3 (empty string defaults to current caller), subject @4, 
    security in @5 ("N" or "R"; "N" is the default),
    pack out date in @6 (0 for no pack out date), 
    @7 True if return receipt desired, 
    @8 TRUE if message should be echoed, and
    @9 is the filename to use for the message text
hint-statement-savescrn=Save the current screen in a buffer for later restoration with the RestScrn
hint-statement-restscrn=Restore the screen from the buffer previously saved with SaveScrn
hint-statement-sound=
    Turn on the BBS PC speaker at the frequency (1-65535) specified by @1 (or turn it off if the frequency is 0)
hint-statement-chat=Initiate SysOp chat mode
hint-statement-sprint=
    Display one or more string expressions on the BBS screen only (this statement does not send anything to the modem)
hint-statement-sprintln=
    Display zero or more string expressions on the BBS screen only and follow with a newline (this statement does not send anything to the modem)
hint-statement-mprint=
    Display one or more string expressions on the callers screen only (this statement does not send anything to the BBS screen)
hint-statement-mprintln=
    Display zero or more string expressions on the callers screen only and follow with a newline (this statement does not send anything to the BBS screen)
hint-statement-rename=Rename file @1 to @2
hint-statement-frewind=Rewind channel @1 after flushing buffers and committing the file to disk.
hint-statement-pokedw=Assign the value @2 (-2147483648 - +2147483647) to memory address @1
hint-statement-dbglevel=Assign the debug level to @1
hint-statement-showon=Turns on display of information to the screen
hint-statement-showoff=Turns off display of information to the screen
hint-statement-pageon=Turn on the SysOp paged indicator (flashing p on status line)
hint-statement-pageoff=Turn off the SysOp paged indicator (flashing p on status line)
hint-statement-fseek=
    Position to any random location within a file
    @2 is the number of bytes to move (+/-) relative to position
    @3 is the base location to start the seek from:

    `SEEK_SET (0)` for the beginning of the file

    `SEEK_CUR (1)` for the current file pointer location  

    `SEEK_END (2)` for the end of the file  
hint-statement-fflush=flush a specified channel changes to disk
hint-statement-fread=
    Read binary data from a file.

    @1 is the channel number

    @2 is the variable to store the data

    @3 is the number of bytes to read
hint-statement-fwrite=
    Write binary data to a file

    @1 is the channel number

    @2 is the expression whose result should be written

    @3 is the size of data to write to var
hint-statement-fdefin=Specify a default input file channel (used to speed up file input)
hint-statement-fdefout=Specify a default output file channel (used to speed up file output)
hint-statement-fdget=Default channel input statement: use the exact same arguments as FGet except a channel parameter (the channel specified by FDefIn is assumed)
hint-statement-fdput=Default channel output statement: use the exact same arguments as FPut except a channel parameter (the channel specified by FDefOut is assumed)
hint-statement-fdputln=Default channel output statement: use the exact same arguments as FPutLn except a channel parameter (the channel specified by FDefOut is assumed)
hint-statement-fdputpad=Default channel output statement: use the exact same arguments as FPutPad except a channel parameter (the channel specified by FDefOut is assumed)
hint-statement-fdread=Default channel input statement: use the exact same arguments as FRead except a channel parameter (the channel specified by FDefIn is assumed)
hint-statement-fdwrite=Default channel output statement: use the exact same arguments as FWrite except a channel parameter (the channel specified by FDefOut is assumed)
hint-statement-adjbytes=
    Adjust the users total and daily download.

    To subtract bytes use a negative number for bytes.

    To add bytes use a positive number.
hint-statement-kbdstring=Stuff strings to the keyboard (just like KbdStuff except 'keystrokes' are echoed to the display)
hint-statement-alias=Enables (`TRUE`) or disables (`FALSE`) use of the current user's alias. It has no effect when aliases are not permitted for the user or conference. Use `ALIAS()` to query the current state.
hint-statement-redim=
    Resizes a previously declared array at runtime: `REDIM array, dim1 [, dim2 [, dim3]]`.

    The number of dimensions must match the declaration; only their bounds may change. Existing values outside the new bounds are lost. Record-field arrays have fixed bounds and cannot be resized.
hint-statement-append=Append the contents of one file to another file.
hint-statement-copy=Copy the contents of one file to another file.
hint-statement-kbdflush=Flush the local keyboard buffer and any stuffed keystroke buffers. It takes no arguments.
hint-statement-mdmflush=Flush the incoming modem buffer. It takes no arguments.
hint-statement-keyflush=Flush both the local buffers and the incoming modem buffer. It takes no arguments.
hint-statement-lastin=Set the users last conference in value. It can be used during the logon process to force the user into a particular conference at start up (for example, from a logon script).
hint-statement-flag=Allow flagging files for download directly from a PPE.
hint-statement-download=
    Downloading files from PPL.
    
    The string passed to DOWNLOAD is a list of commands in the same format as what a user would type after a D or DB command.

    If a file name for download is specified here it must be downloadable according to the criteria established in the FSEC and DLPATH.LST files.

    If it is necessary to download a file not normally available via the FSEC and/or DLPATH.LST files the FLAG statement may be used to force it into the list of files to download.
hint-statement-wrusysdoor=Write a USERS.SYS file with a TPA record for a DOOR application.
hint-statement-getaltuser=
    Get the information for an alternate user.

    It will fill the user variables with information from the specified user record as well as redirect user statements and functions.

    If an attempt is made to get a record number that doesn't exist, 
    the user functions will revert to the current user and the user variables will be invalidated as though no GetUser/GetAltUser 
    statement had been issued (though they will continue to maintain any value held). 

    `PutUser`/`PutAltUser` should be issued to commit any variable changes to the user record.
    Additionally, there is at least one statement that will not affect alternate users: `AdjTime`. 
    
    It is restricted to the current user online.
    
    Also, if the alternate user is online, changes to the record won't take hold until after the user has logged off. 
    Also, if there is not enough memory available (primarily for the last message read pointers) this statement will fail.
hint-statement-adjdbytes=
    Adjust the users daily download bytes.

    To subtract bytes use a negative number for bytes.

    To add bytes use a positive number.
hint-statement-adjtbytes=
    Adjust the users total download bytes.
    
    To subtract bytes use a negative number for bytes.

    To add bytes use a positive number.
hint-statement-adjtfiles=
    Adjust the users total download files.

    To subtract files use a negative number for files.

    To add files use a positive number.
hint-statement-lang=Change the language in use by the current user.
hint-statement-sort=
    Sort the contents of an array into a pointer array.

    Note that sortArray and pointerArray are restricted to one (1) dimensional arrays
hint-statement-mousereg=
    Set up a RIP mouse region on the remote terminal.
    
    | | |
    | --- | --- |
    | @1 | Is the RIP region number| 
    | @2, @3 | The (X,Y) coordinates of the upper-left of the region |
    | @4, @5 | The (X,Y) coordinates of the lower-right of the region |
    | @6 | The width of each character in pixels |
    | @7 | The height of each character in pixels |
    | @8 | A boolean flag (TRUE to invert the region when clicked) |
    | @9 | A boolean flag (TRUE to clear and full screen the text window) | 
    | @10 | Text that the remote terminal should transmit when the region is clicked |
hint-statement-scrfile=Find a file name and line number that is currently on the screen.
hint-statement-searchinit=Initialize search parameters for a faster BOYER-MOORE search algorithm.
hint-statement-searchfind=Execute a BOYER-MOORE search on a text buffer using criteria previously defined with a SearchInit statement.
hint-statement-searchstop=Clears out previously entered search criteria. It takes no parameters.
hint-statement-prfound=These work just like Print and PrintLn but, if the last SearchFind statement resulted in a match, it will automatically highlight found words.
hint-statement-prfoundln=These work just like Print and PrintLn but, if the last SearchFind statement resulted in a match, it will automatically highlight found words.
hint-statement-tpaget=Get static information from a named TPA in string format.
hint-statement-tpaput=Put static information to a named TPA in string format.
hint-statement-tpacget=
    Get information from a named TPA for a specified conference in string format.

    @1 The keyword of the TPA to use  

    @2 The variable into which to store the information  

    @3 The conference number for which to retrieve information  
hint-statement-tpacput=
    Put information to a named TPA for a specified conference in string format.
    
    @1 The keyword of the TPA to use  

    @2 The expression to write to store the TPA

    @3 The conference number for which to retrieve information  
hint-statement-tparead=
    Get static information from a named TPA.

    @1 The keyword of the TPA to use  

    @2 The variable into which to store the information  
hint-statement-tpawrite=
    Put static information to a named TPA.

    @1 The keyword of the TPA to use  

    @2 The expression to write to store the TPA
hint-statement-tpacread=
    Get information from a named TPA for a specified conference.

    @1 The keyword of the TPA to use  

    @2 The variable into which to store the information

    @3 The conference number for which to retrieve information  
hint-statement-tpacwrite=
    Put information to a named TPA for a specified conference.
    
    @1 The keyword of the TPA to use  

    @2 The expression to write to store the TPA

    @3 The conference number for which to retrieve information  
hint-statement-bitset=
    Set a specified bit from a variable.

    This statement is primarily intended to be used with BIGSTR variables which can be up to 2048 bytes long. 
    However, it will work with other data types as well if desired.
    
    Just be aware of the potential problems in 'bit twidling' non-string buffers and then trying to access them later as their 'intended' 
    type without re-initializing the variable. 
    
    If the bit parameter (an integer from 0 to the number of bits in the object) is invalid no processing takes place.
hint-statement-bitclear=
    Clears a specified bit from a variable.

    This statement is primarily intended to be used with BIGSTR variables which can be up to 2048 bytes long.
    
    However, it will work with other data types as well if desired. Just be aware of the potential problems in 'bit twidling' 
    non-string buffers and then trying to access them later as their 'intended' type without re-initializing the variable.
    
    If the bit parameter (an integer from 0 to the number of bits in the object) is invalid no processing takes place.
hint-statement-brag=Obsolete PCBoard command for the former BRAG display. PCBoard 15.3 and IcyBoard accept it for compatibility but perform no action.
hint-statement-frealtuser=
    Since only one `GETALTUSER` can be active at one time, `FREALTUSER` can allow other processes which need to use `GETALTUSER` (such as the `MESSAGE` commend) to do so.
hint-statement-setlmr=
    Set the last read pointers for the specified conference.

    If @1 is greater than the number of actual confrences @1 will default to the highest conference number.
    
    If @2 is greater than the highest message number in that conference, it will default to the highest message number in that conference. 
    This could be used to set a new users mesg pointers to recent messages so they aren't replying to 3 years old messages. 
    A useful feature would be to get the high conference number.
hint-statement-setenv=
    Set an environment variable.

    String format is:`"VAR=VALUE"`
hint-statement-fcloseall=Closes all file channels
hint-statement-stackabort=
    This allows the programmer to tell the runtime module to try its best to continue executing after a stack error has occurred.
    
    If it is passed `FALSE`, it will abort execution after a stack error. If it is passed `TRUE` the PPE will continue to run.

    > [!CAUTION]
    > If you continue to execute after a stack error, program execution will be unpredictable.
    > PPL will not allow system memory to be corrupted because of a stack error.
hint-statement-dcreate=create DBF file
hint-statement-dopen=open DBF file
hint-statement-dclose=close DBF file
hint-statement-dsetalias=set DBF alias
hint-statement-dpack=pack DBF file
hint-statement-dcloseall=close all NDX files
hint-statement-dlock=lock DBF file
hint-statement-dlockr=lock a record
hint-statement-dlockg=lock a group of records
hint-statement-dunlock=unlock any current locks
hint-statement-dncreate=create NDX file
hint-statement-dnopen=open NDX file
hint-statement-dnclose=close NDX file
hint-statement-dncloseall=close all NDX files
hint-statement-dnew=start a new record
hint-statement-dadd=add the new record
hint-statement-dappend=append a blank record
hint-statement-dtop=go to top record
hint-statement-dgo=go to specific record
hint-statement-dbottom=go to bottom record
hint-statement-dskip=skip +/- a number of records
hint-statement-dblank=blank the record
hint-statement-ddelete=delete the record
hint-statement-drecall=recall the record
hint-statement-dtag=select a tag
hint-statement-dseek=
    returns error status ( 0|1 )
    ; or seek success (0 = Error
    ; 1 = success, 2 = following record
    ; 3 = end of file )
hint-statement-dfblank=blank a named field
hint-statement-dget=get a value from a named field
hint-statement-dput=put a value to a named field
hint-statement-dfcopy=copy a field to a field
hint-statement-account=
    @1 is a value between 0-14. Using system constants is recommended.
    
    @2 is the amount of credits to add or subtract to field the field
hint-statement-recordusage=
    @1 is the field number to access (using DEB… consts) descr1 is the descripttion of the charge descr2 is a subdescription of the charge unitcost 
    is the cost per unit value is the number of units Recordusage will update debit values in PCBoard as well as record descriptions and other 
    information in an accounting file.
    
    Valid values for the field parameter are 2-16. The constants corresponding with these values (DEB???) could and should be used here. 
    
    (see the Accounting section for a list of consts)

hint-statement-msgtofile=
    Writes a message into a file.

    This statement will take the given message and write it to a text file. 
    The file's first 15 lines will contain standard header information. (One field per line) The headers are formatted to make parsing easier.
    The 16th line will state how many extended headers are present. The following line(s) will contain extended headers.
    (one per line) Finally, after the extended headers, will be a line containing “Message body:”. Everything after that is the body of the message.

hint-statement-qwklimits=
    This statement allows the PPL programmer to modify a users QWK limits. Four fields can be modified with their statement.
hint-statement-command=
    Process a command as if it were typed on the prompt.

    @1: A boolean value indcating whether or not to try to find the command in CMD.LST.
    If TRUE and the command is not in CMD.LST, it will try the standard commands automatically, failing if the command does not exist.

    @2: A string value with the command and parameters to execute. (like “R A Y O S”)

    NOTES!!! Not all portions of PCBoard are re-entrant. For example, you should not try to have two message editor processes active at the same time (in other words, you shouldn't launch the message editor from within a MNU and then launch a PPE from a shifted function key that tries to enter another message). So you'll need to be carefull about nested COMMAND (or equivalent) calls. But sequential processing should not be a problem at all.
    If it is determined at some point in the future that allowing this flexibility causes more problems than it solves, the COMMAND statement will be scaled back to ensure that attempts are not made to re-enter code. So use it well and wisely!
hint-statement-uselmrs=Controls whether subsequent `GETALTUSER` calls load the alternate user's Last Message Read pointers. Pass `FALSE` to save memory when LMR data is not needed and `TRUE` to restore loading. `USELMRS()` returns the current setting.
hint-statement-confinfo=
    This statement can be used to modify a field in the conference
    configuration.

    @1 = The conference number to get information about
    
    @2 = Conference field to modify. (See note)
    
    @3 = New value to store in field

    { conference_access_constants }

hint-statement-adjtubytes=
    This statement can be used to adjust a users total uploads bytes
    up or down.
    
    @1 = Number of bytes to adjust current users upload bytes with.
    This can be a positive or negative value
hint-statement-grafmode=
    This statement can be used to change a users graphics mode while online.

    @1 = The graphics mode to change to.
    1 = If user has ANSI capabilities it will change graphics to color ANSI
    2 = Will attempt to put user in color ansi regardless of ansi ability
    3 = Puts user in ansi-black and white mode
    4 = Puts user in non-ansi black and white mode
    5 = If user has RIP ability, will put user in RIP mode.
    (IcyBoard: 6 = Avatar mode
    )

hint-statement-adduser=
    @1 = name of the new user to add
    @2 = TRUE instructs PCBoard to leave the new user's variables active, as if a GETALTUSER were executed
    using the new user record number. FALSE will restore the current users variables.

    ### Remarks
    This statement allows PPL to create a new user record, filling
    in all fields but the name with pcboard default values.
hint-statement-killmsg=
    @1 = conference number in which the doomed message resides.
    @2 = message number to kill
hint-statement-chdir=Changes to directory
hint-statement-mkdir=Creates a new directory
    
    @1 = directory to create 
hint-statement-rmdir=
    Removes a directory

    @1 = directory to remove 

    ### Note
    The directory must be empty before removing it.
hint-statement-fdowraka=Compatibility stub for writing a PCBoard FidoNet AKA entry. The original PCBoard implementation was never completed; IcyBoard logs a warning and makes no change.
hint-statement-fdoaddaka=Compatibility stub for adding a PCBoard FidoNet AKA entry. The original PCBoard implementation was never completed; IcyBoard logs a warning and makes no change.
hint-statement-fdowrorg=Compatibility stub for writing a PCBoard FidoNet origin line. The original PCBoard implementation was never completed; IcyBoard logs a warning and makes no change.
hint-statement-fdoaddorg=Compatibility stub for adding a PCBoard FidoNet origin line. The original PCBoard implementation was never completed; IcyBoard logs a warning and makes no change.
hint-statement-fdoqmod=
    Replaces an entry of the outbound queue

    @1 = record number, counted from one
    @2 = address of the link the file is for
    @3 = file to send
    @4 = NORMAL or CRASH, read and ignored
hint-statement-fdoqadd=
    Puts a file into the outbound queue of a link

    @1 = address of the link the file is for
    @2 = file to send
    @3 = NORMAL or CRASH, read and ignored
hint-statement-fdoqdel=
    Takes an entry out of the outbound queue

    @1 = record number, counted from one
hint-statement-sounddelay=
    @1 = frequency at which to sound the PC speaker
    @2 = length, in clock ticks (18 = 1 second), to leave the speaker on

    ### Remarks
    This function was added to replace the
    ```
    SOUND 500
    SOUND 0
    ```
    combination required for DOS, since this functionality is not available under OS/2.
hint-statement-shortdesc=
    Sets the current user's status for viewing short (one line) or full file descriptions.

    @1 = A boolean expression stating if the short description set on.
hint-statement-movemsg=
    Moves the message from its current location to the end of the message base.

    @1 = conference number in which the message resides
    @2 = message number to move
    @3 = A Boolean expression stating where the message should be
    move or not.  TRUE if it will be moved, FALSE if the message is to be copied.
hint-statement-setbankbal=
    Sets the value of a specified field.

    @1 An interger expression stating the field to get.
    @2 An interger expression stating the value that the specified field is to set to.

    ### Fields
    Time Fields (in minutes)
    ------------------------
        0 = Last Deposit Date
        1 = Last Withdrawal Date
        2 = Last Transaction Amount (in minutes)
        3 = Amount Saved (their time balance in their account)
        4 = Max Withdrawal (the max a user can withdraw in a day)
        5 = Max Stored Amount (Maximum time allowed to be stored)

    Byte Fields (in K bytes)
    ------------------------
        6 = Last Deposit Date
        7 = Last Withdrawal Date
        8 = Last Transaction Amount (in K bytes)
        9 = Amount Saved (their K byte balance in their account)
        10 = Max Withdrawal (the max a user can withdraw in a day)
        11 = Max Stored Amount (Maximum K bytes allowed to be stored)
hint-function-len=
    ### Returns
    Returns the length of the string @1
hint-function-lower=
    ### Returns
    Returns the string @1 converted to lower case
hint-function-upper=
    ### Returns
    Returns the string @1 converted to upper case
hint-function-mid=
    ### Returns
    Returns a substring of @1 starting at position @2 and @3 characters long
hint-function-left=
    ### Returns
    Returns the leftmost @2 characters of @1
hint-function-right=
    ### Returns
    Returns the rightmost @2 characters of @1
hint-function-space=
    ### Returns
    Returns a string of @1 spaces
hint-function-ferr=Returns whether an error has occurred on file channel `@1` since it was last checked. Reading `FERR()` clears that channel's error flag. End-of-file after `FGET` or `FREAD` also sets the flag.
hint-function-chr=
    ### Returns
    Returns a single character long string of the character represented by ASCII code var (0-255)
hint-function-asc=
    ### Returns
    Returns the ASCII value of the first character in @1
hint-function-instr=Returns the position of @2 in @1 `(1-LEN(@1))` or `0` if @2 not in @1
hint-function-abort=Returns a flag indicating whether or not the user aborted the display of data via ^K / ^X or answering no to a MORE? prompt
hint-function-ltrim=Returns a string of @1 with the first character of @2 trimmed from the left
hint-function-rtrim=Returns a string of @1 with the first character of @2 trimmed from the right
hint-function-trim=Returns a string of @1 with the first character of @2 trimmed from both ends
hint-function-random=Returns a random number between 0 and @2 inclusive
hint-function-date=Returns todays date
hint-function-time=Returns the current time
hint-function-u_name=Returns the current users name
hint-function-u_ldate=Returns the current users last date on the system
hint-function-u_ltime=Returns the current users last time on the system
hint-function-u_ldir=Returns the current users last directory scan date
hint-function-u_logons=Returns the current users number of times logged on
hint-function-u_ful=Returns the current users number of files uploaded
hint-function-u_fdl=Returns the current users number of files downloaded
hint-function-u_bdlday=Returns the current users number of bytes downloaded today
hint-function-u_timeon=Returns the current users time online today in minutes
hint-function-u_bdl=Returns the current users number of bytes downloaded
hint-function-u_bul=Returns the current users number of bytes downloaded
hint-function-year=Returns the year (1900-2079) of @1
hint-function-month=Returns the month of the year (1-12) of @1
hint-function-day=Returns the day of the month (1-31) of @1
hint-function-dow=Returns the day of the week (0 = Sunday, 6 = Saturday) that @1 fell on
hint-function-hour=Returns the hour of the day (0-23) of @1
hint-function-min=Returns the minute of the hour (0-59) of @1
hint-function-sec=Returns the second of the minute (0-59) of @1
hint-function-timeap=Returns a string representing the time @1 in civilian format (XX:XX:XX AM)
hint-function-ver=Returns the version number of PCBoard that is running
hint-function-nochar=Returns the current language no character
hint-function-yeschar=Returns the current language yes character
hint-function-stripatx=Returns a string of @1 with all @X codes removed
hint-function-replace=Returns a string of @1 with all occurences of the first character of @2 replaced by the first character of @3
hint-function-strip=Returns a string of @1 with all occurrences of the first character of @2 removed
hint-function-inkey=Returns the next keypress as a single character long string, or a string with the name of the function or cursor control key
hint-function-tostring=Converts an expression to a `STRING` type
hint-function-mask_pwd=Returns a valid character mask for input statements of passwords
hint-function-mask_alpha=Returns a valid character mask for input statements of A through Z and a through z
hint-function-mask_num=Returns a valid character mask for input statements of 0 through 9
hint-function-mask_alnum=Returns a valid character mask for input statements of A through Z, a through z, and 0 through 9
hint-function-mask_file=Returns a valid character mask for input statements of file names
hint-function-mask_path=Returns a valid character mask for input statements of path names
hint-function-mask_ascii=Returns a valid character mask for input statements of space (“ ”) through tilde (“~”)
hint-function-curconf=Returns the current conference number
hint-function-pcbdat=Returns a string with the path and file name of PCBOARD.DAT
hint-function-ppepath=Returns a string with the path (no file name) of the currently executing PPE file
hint-function-valdate=Returns `TRUE` if @1 is in a valid date format
hint-function-valtime=Returns `TRUE` if @1 is in a valid time format
hint-function-u_msgrd=Returns the number of messages the user has read
hint-function-u_msgwr=Returns the number of messages the user has written
hint-function-pcbnode=Returns the node number
hint-function-readline=Read and return line number @2 from file @1
hint-function-sysopsec=Returns the SysOp security defined in PCBOARD.DAT
hint-function-onlocal=Returns `TRUE` if the user is on locally
hint-function-un_stat=Returns a nodes status from USERNET.XXX after a RdUnet statement
hint-function-un_name=Returns a nodes user name from USERNET.XXX after a RdUnet statement
hint-function-un_city=Returns a nodes city from USERNET.XXX after a RdUnet statement
hint-function-un_oper=Returns a nodes operation text from USERNET.XXX after a RdUnet statement
hint-function-cursec=Returns the users current security level
hint-function-gettoken=
    Returns the next string token from a prior call to `Tokenize` (Same as the `GETTOKEN` statement but can be used in an expression without prior assignement to a variable)
hint-function-minleft=Returns the current callers minutes left to use online
hint-function-minon=Returns the current callers minutes online so far this session
hint-function-getenv=Returns the value of the environment variable named by @1
hint-function-callid=Returns the caller ID string
hint-function-regal=Returns the value of the AL register after a DoIntr statement
hint-function-regah=Returns the value of the AH register after a DoIntr statement
hint-function-regbl=Returns the value of the BL register after a DoIntr statement
hint-function-regbh=Returns the value of the BH register after a DoIntr statement
hint-function-regcl=Returns the value of the CL register after a DoIntr statement
hint-function-regch=Returns the value of the CH register after a DoIntr statement
hint-function-regdl=Returns the value of the DL register after a DoIntr statement
hint-function-regdh=Returns the value of the DH register after a DoIntr statement
hint-function-regax=Returns the value of the AX register after a DoIntr statement
hint-function-regbx=Returns the value of the BX register after a DoIntr statement
hint-function-regcx=Returns the value of the CX register after a DoIntr statement
hint-function-regdx=Returns the value of the DX register after a DoIntr statement
hint-function-regsi=Returns the value of the SI register after a DoIntr statement
hint-function-regdi=Returns the value of the DI register after a DoIntr statement
hint-function-regf=Returns the value of the flags register after a DoIntr statement
hint-function-regcf=Returns the value of the carry flag register after a DoIntr statement
hint-function-regds=Returns the value of the DS register after a DoIntr statement
hint-function-reges=Returns the value of the ES register after a DoIntr statement
hint-function-b2w=
    Returns a word built from two byte sized values by the formula:
    `(@1*0100h+@2)`
hint-function-peekb=Returns a byte value (0-255) located at memory address @1 (PEEK is a synonym)
hint-function-peekw=Returns a word value (0-65535) located at memory address @1
hint-function-mkaddr=
    Returns a segment:offset address as a long integer built from two word sized values by the formula:
    `@1*00010000h+@2`
hint-function-exist=Returns a boolean `TRUE` value if the file @1 exists
hint-function-i2s=Returns a string representing the integer value @1 converted to base @2
hint-function-s2i=Returns an integer representing the string @1 converted from base @2
hint-function-carrier=Returns the carrier speed as reported by the modem to PCBoard
hint-function-tokenstr=Returns a previously tokenized string reconstructed with semi-colons separating the component tokens
hint-function-cdon=Returns `TRUE` if the carrier detect signal is on, `FALSE`
hint-function-langext=Returns the file extension for the users language selection
hint-function-ansion=Returns `TRUE` if the user is on locally
hint-function-valcc=Returns `TRUE` if @1 is a valid credit card number
hint-function-fmtcc=Returns a formatted credit card number based on @1
hint-function-cctype=Returns the issuer of credit card number @1
hint-function-getx=Returns the current column (X position) of the cursor on the display
hint-function-gety=Returns the current row (Y position) of the cursor on the display
hint-function-band=Returns the bitwise and of two integer expressions
hint-function-bor=Returns the bitwise or of two integer expressions
hint-function-bxor=Returns the bitwise exclusive-or of two integer expressions
hint-function-bnot=Returns the bitwise complement (all bits inverted) of an integer expression
hint-function-u_pwdhist=Returns the specified password from the password history Valid values for @1 are 1 through 3
hint-function-u_pwdlc=Returns the date of the last password change
hint-function-u_pwdtc=Returns the number of times the password has been changed
hint-function-u_stat=
    Returns a statistic about the user that is tracked by PCBoard
    Valid values for @1 are 1 through 15
    |||
    | --- | --- |
    | 1 | first date the user called the system |
    | 2 | number of SysOp pages the user has requested |
    | 3 | number of group chats the user has participated in |
    | 4 | number of comments the user has left |
    | 5 | number of 300 bps connects |
    | 6 | number of 1200 bps connects |
    | 7 | bumber of 2400 bps connects |
    | 8 | number of 9600 bps connects |
    | 9 | number of 14400 bps connects |
    | 10 | number of security violations |
    | 11 | number of “not registered in conference” warnings |
    | 12 | number of times the users download limit has been reached |
    | 13 | number of “file not found” warnings |
    | 14 | number of password errors the user has had |
    | 15 | number of verify errors the user has had |

hint-function-defcolor=Returns system default color.
hint-function-abs=Returns the absolute value of @1
hint-function-sin=Returns the sine of @1 (given in radians).
hint-function-cos=Returns the cosine of @1 (given in radians).
hint-function-tan=Returns the tangent of @1 (given in radians).
hint-function-atan=Returns the arctangent of @1, in radians.
hint-function-log=Returns the natural logarithm of @1.
hint-function-sqrt=Returns the square root of @1.
hint-function-grafmode=
    Returns a character indicating the users graphics status

    | Value | Meaning |
    | :--- | :--- |
    | R | RIPscrip supported |
    | G | ANSI graphics (color and positioning) supported |
    | A | ANSI positioning (no color) supported |
    | N | No graphics (RIP or ANSI) supported |

hint-function-psa=
    Returns the value of the specified PSA variable

    @1 = The PSA variable to retrieve

    ### PSA
    | | |
    | :--- | :--- |
    | 1 | Alias Support Enabled |
    | 2 | Verify Support Enabled |
    | 3 | Address Support Enabled |
    | 4 | Password Support Enabled |
    | 5 | Statistics Support Enabled |
    | 6 | Notes Support Enabled |
hint-function-fileinf=
    Returns information about the file specified by @1
    
    @1 = The file to get information about

    @2 = The information to return

    ### Valid Options
    | | |
    | :--- | :--- |
    | 1 | Return TRUE if file exists |
    | 2 | Return file date stamp |
    | 3 | Return file time stamp |
    | 4 | Return file size |
    | 5 | Return file attributes 1) |
    | 6 | Return file drive |
    | 7 | Return file path |
    | 8 | Return file base name |
    | 9 | Return file extension |

    | 1) File Attribute | |
    | :--- | :--- |
    | 01h | Read Only |
    | 02h | Hidden |
    | 04h | System |
    | 20h | Archive |
hint-function-ppename=Returns the name of the currently executing PPE file minus the path and extension
hint-function-mkdate=Returns a date with the year specified by year (1900-2079), month specified by month (1-12), and day specified by day (1-31).
hint-function-curcolor=Returns the current color (0-255) in use by the ANSI driver
hint-function-kinkey=Returns the next keypress from the BBS keyboard as a single character long string, or a string with the name of the function or cursor control key
hint-function-minkey=Returns the next keypress from the remote caller as a single character long string, or a string with the name of the function or cursor control key
hint-function-maxnode=Returns the maximum node possible with the current software (ie, /2 would return 2, /10 would return 10, etc)
hint-function-slpath=Returns the path, as specified in PCBSetup, to the login security files
hint-function-helppath=Returns the path, as specified in PCBSetup, to the help files
hint-function-temppath=Returns the path, as specified in PCBSetup, to the temporary work directory
hint-function-modem=Returns the modem connect string as reported by the modem to PCBoard
hint-function-loggedon=Returns `TRUE` if the user has already logged on to the BBS, `FALSE` otherwise
hint-function-callnum=Returns the caller number of the current user.
hint-function-mgetbyte=Returns the value of the next byte from the modem (0-255) or -1 if there are no bytes waiting for input
hint-function-tokcount=Returns the number of tokens available via the GetToken statement and/or function
hint-function-u_recnum=Returns the user record number (0-65535) for user name user or -1 if user is not registered on this system.
hint-function-u_inconf=Returns `TRUE` if user record number @1 is registered in conference @2
hint-function-peekdw=Returns a signed integer value (-2147483648 - +2147483647) located at memory address “var”
hint-function-dbglevel=Returns the debug level in effect
hint-function-scrtext=
    ### Returns
    Returns a string of @3 characters from the screen at @1, @2.
    If @3 is `TRUE` then the string will be returned with all @ codes intact.
hint-function-showstat=Returns `TRUE` if writing to the display is active, `FALSE` if writing to the display is disabled
hint-function-pagestat=Returns `TRUE` if the user has paged the SysOp (or PageOn has been issued), `FALSE` otherwise (or PageOff has been issued)
hint-function-replacestr=
    It functions just like the Replace function except that a complete sub-string may be specified for both search and replace
hint-function-stripstr=
    Functions just like the Strip function except that a complete sub-string may be specified for search
hint-function-tobigstr=Converts an expression to a `BIGSTR` type
hint-function-toboolean=Converts an expression to a `BOOLEAN` type
hint-function-tobyte=Converts an expression to a `BYTE` type
hint-function-todate=Converts an expression to a `DATE` type
hint-function-todreal=Converts an expression to a `DREAL` type
hint-function-toedate=Converts an expression to a `EDATE` type
hint-function-tointeger=Converts an expression to an `INTEGER` type
hint-function-tolong=Converts an expression to a signed 64-bit `LONG` type
hint-function-toulong=Converts an expression to an unsigned 64-bit `ULONG` type
hint-function-tomoney=Converts an expression to a `MONEY` type
hint-function-toreal=Converts an expression to a `REAL` type
hint-function-tosbyte=Converts an expression to a `SBYTE` type
hint-function-tosword=Converts an expression to a `SWORD` type
hint-function-totime=Converts an expression to a `TIME` type
hint-function-tounsigned=Converts an expression to a `UNSIGNED` type
hint-function-toword=Converts an expression to a `WORD` type
hint-function-mixed=Converts a string to mixed (or proper name) case
hint-function-alias=Return the users current ALIAS setting (TRUE = alias use on, FALSE = alias use off)
hint-function-confreg=Returns TRUE if users registered flag is set, FALSE otherwise
hint-function-confexp=Returns TRUE if users expired flag is set, FALSE otherwise
hint-function-confsel=Returns TRUE if user has selected the conference, FALSE otherwise
hint-function-confsys=Returns TRUE if user has conference SysOp access, FALSE otherwise
hint-function-confmw=Returns TRUE if user has mail waiting in conference confnum, FALSE otherwise
hint-function-lprinted=Return the number of lines printed on the display
hint-function-isnonstop=Return whether or not the display is currently in non-stop mode (ie, did the user type NS as part of their command line)
hint-function-errcorrect=Returns TRUE if a session is determined to be error corrected (or FALSE for non-error corrected sessions).
hint-function-confalias=Return TRUE if the current conference is configured to allow aliases
hint-function-useralias=Return TRUE if the current user is allowed to use an alias
hint-function-curuser=
    Determine what users information, if any, is available via the user variables. It takes no arguments and returns one of the following values:
    NO_USER (-1) - User variables are currently undefined  
    CUR_USER (0) - User variables are for the current user  
    Other        - The record number of an alternate user for whom user  variables are defined 
hint-function-u_lmr=function to return the number of the last message read for the specified conference.
hint-function-chatstat=Return the current users chat availability status (TRUE means available, FALSE means unavailable).
hint-function-defans=Returns the last default answer passed to an Input statement. For example, this allows a PPE to determine what the default answer would have been had a PCBTEXT prompt not been replaced with a PPE.
hint-function-lastans=function to return the last answer accepted by an Input statement.
hint-function-meganum=Converts a decimal number (from 0 to 1295) to a hexa-tri-decimal number, or meganum.
hint-function-evttimeadj=Detects if the users time has been adjusted for an upcoming event. This is useful to detect if a users time left can be increased with the AdjTime statement.
hint-function-isbitset=
    Check the status of a specified bit in a variable.
    This function is primarily intended to be used with BIGSTR variables which can be up to 2048 bytes long.
    However, it will work with other data types (and expressions) as well if desired.
hint-function-fmtreal=
    Formats REAL/DREAL values for display purposes.
    ### Parameters
    realExp	A REAL/DREAL floating point expression
    fieldWidth	The minimum number of characters to display
    decimalPlaces	The number of characters to display to the right of the decimal point

hint-function-flagcnt=Return the number of files flagged for download.
hint-function-kbdbufsize=Return the number of key presses pending in the KbdString buffer
hint-function-pplbufsize=Returns the number of key presses pending in the KbdStuff buffer.
hint-function-kbdfilused=Returns `TRUE` while keyboard input is being supplied by a `KBDFILE` script, otherwise `FALSE`. This distinguishes file-driven input from `KBDSTUFF` and `KBDSTRING`.
hint-function-lomsgnum=Returns the low message number for the current conference.
hint-function-himsgnum=Returns the high message number for the current conference.
hint-function-drivespace=Return Val: Amount of divespace left of drive drivespec. 
hint-function-outbytes=Returns the number of bytes waiting in the modems output buffer Not available in local mode.
hint-function-hiconfnum=Returns the highest conference number available on the board
hint-function-inbytes=Returns number of bytes waiting in the modem input buffer Not available in local mode.
hint-function-crc32=Returns an UNSIGNED value of the CRC of a file or string.
hint-function-pcbmac=
    Returns a BIGSTR containing the expanded text of a PCB MACRO

    ### PCB MACROS not supported
    @automore@ @beep@ @clreol@ @cls@ @delay@ @more@ @pause@ @poff@ @pon@ @pos@ @qoff@ @qon@ @wait@ @who@ @x@
hint-function-actmsgnum=
    ### Returns
    Returns number of active messages in current conference

    ### Example
    ```
    integer i
    println "There are ",ACTMSGNUM()," messages in conference ",CURCONF()
    ```
hint-function-stackleft=Returns the number of bytes left on the system stack.
hint-function-stackerr=Returns a boolean value which indicates a stack error has occured if TRUE.
hint-function-dgetalias=return the current alias
hint-function-dbof=Returns `TRUE` when the selected DBase channel's record pointer is before the first record, otherwise `FALSE`.
hint-function-dchanged=return the changed flag
hint-function-ddecimals=return decimals of named field
hint-function-ddeleted=return the deleted flag
hint-function-deof=return the end of file status
hint-function-derr=return error flag for channel
hint-function-dfields=return count of fields
hint-function-dlength=return length of named field
hint-function-dname=return name of numbered field
hint-function-dreccount=Returns the total number of records in the active DBase file.
hint-function-drecno=return the current record number
hint-function-dtype=return type of named field
hint-function-fnext=Returns an available file channel. -1 when none are available.
hint-function-dnext=
    Returns the next unused DBase channel number, or `-1` when none is available.

    The channel is not reserved until a file is opened. Repeated `DNEXT()` calls therefore return the same number; store it and open the file before asking again.
hint-function-toddate=Converts a date to a string in the format MM/DD/YYYY
hint-function-dcloseall=close all DBF files
hint-function-dopen=open DBF file
hint-function-dclose=close DBF file
hint-function-dsetalias=set DBF alias
hint-function-dpack=pack DBF file
hint-function-dlockf=lock DBF file
hint-function-dlock=lock DBF file
hint-function-dlockr=Attempts to lock one record on the selected DBase channel and returns whether the lock succeeded. Use the matching unlock operation when the update is complete.
hint-function-dunlock=unlock any current locks
hint-function-dnopen=open NDX file
hint-function-dnclose=close NDX file
hint-function-dncloseall=close all NDX files
hint-function-dnew=start a new record
hint-function-dadd=add the new record
hint-function-dappend=append a blank record
hint-function-dtop=go to top record
hint-function-dgo=go to specific record
hint-function-dbottom=go to bottom record
hint-function-dskip=skip +/- a number of records
hint-function-dblank=blank the record
hint-function-ddelete=delete the record
hint-function-drecall=recall the record
hint-function-dtag=select a tag
hint-function-dseek=
    returns error status ( 0|1 )
    or seek success (0 = Error
    1 = success, 2 = following record
    3 = end of file )
hint-function-dfblank=blank a named field
hint-function-dget=get a value from a named field
hint-function-dput=put a value to a named field
hint-function-dfcopy=copy a field to a field
hint-function-dselect=returns channel associated with alias
hint-function-dchkstat=Returns `0` when DBase channel `@1` is open and `1` when it is closed or unavailable.
hint-function-pcbaccount=
    Returns what PCBoard will charge a user for a certain activity. These are values the SysOp assigns in PCBsetup when accounting is configures and enabled.
    Valid values for the field parameter are 0-14. Use of the corresponding constants is encouraged. (see the Accounting section)

    { accounting_constants }

hint-function-pcbaccstat=
    Returns value in status field
    This function can and should be used in conjunction with the ACC_??? constants as the field parameter. Valid values for field are 0-3. 

 | Field | dec | Field Description |
 | :--- |  :--- | :--- |
 | `ACC_STAT`   | `0`  | Returns status of the “Enable Accounting” switch in the PWRD file.  |
 | `ACC_TIME`   | `1`  | The amount of ADDITIONAL units to charge |
 | `ACC_MSGR`   | `2`  | The amount to charge in ADDITION for each message read in the current conference. |
 | `ACC_MSGW`   | `3`  | The amount to charge in ADDITION for each message entered in the current conference. |

hint-function-derrmsg=returns last DBase error text
hint-function-account=Returns amount of credits charged for services corresponding to the field parameter.

hint-function-scanmsghdr=
    Returns the first message number in the message base which matches the search criteria.

    { message_header_constants }
hint-function-checkrip=Returns `TRUE` if the terminal has RIP.
hint-function-ripver=
    Returns a string containing the RIP version. If no RIP is available "0" is returned.
hint-function-qwklimits=
    Returns one QWK limit for the current user. `@1` is `MAXMSGS`, `CMAXMSGS`, `ATTACH_LIM_U` or `ATTACH_LIM_P`.

    Call `GETUSER` first. System-wide PCBSetup limits still cap values configured for an individual user.
hint-function-findfirst=
    Find the first occurence of filespec in a directory. Used in conjunction with FindNext to get a directory listing.

    ### Parameters
    @1 = A string expression with the path and file name to access information about.
    Quite often this expression involves a DOS wildcard (e.g., *.*, *.BAT, etc.)

    ### Returns
    The first filename matching the filename criteria.

    ### Remarks
    This function is designed to help locate files matching a specific
    criteria.  For example, you may want to delete all files matching *.BAK
    in the current directory.  This can be done easily because
    FINDFIRST() locates the first match, while FINDNEXT() locates
    additional matches.

    It should be noted that only the filenames are returned.  If you need
    additional information such as date, time, or size of the file, use
    the FILEINF() function.

hint-function-findnext=
    This function determines if there are any more files matching a specified pattern.

    ### Returns
    The next filename matching the filename criteria or an
    empty string if there are no more matching files.

    ### Remarks
    This function is designed to continue where the FINDFIRST() function
    leaves off because it locates any additional files matching the pattern
    last searched for.  There are no more matching files when the return
    value is null or an empty string. Because you do not know how many
    matching files there are, a WHILE loop is usually involved in gathering
    all of the filenames.

    It should be noted that only the filenames are returned.  If you need
    additional information such as date, time, or size of the file, use the
    FILEINF() function.
hint-function-uselmrs=
    ### Parameters
    @1 = Instructs PCBoard NOT to load an alternate users LMRS
    when a GETALTUSER is executed.

    ### Remarks
    This statment can save a significant amount of memory when a GETALTUSER
    is executed at a later time. When GETALTUSER is executed, it loads
    the users LMR's by default. If you have a significant number of conferences
    on your system, this can require a great deal of memory. Since PCBoard
    is so rich with features, it can take most if not all available
    conventional memory, leaving PPEs out to dry. If an alternate users
    LMRs are not needed by the PPE application, then you can use this
    statement to tell PCBoard not to load the LMR data.

    See also the FUNCTION USELMRS, this will return the current status
    of USELMRS. Eg, if the funtion USELMRS returns TRUE, then a GETALTUSEr
    will load LMRS, if it returns FALSE, LMRS will not be loaded.
hint-function-confinfo=
    This statement can be used to access a field in the conference
    configuration.

    ### Parameters
    @1 = The conference number to get information about
    @2 = Conference field to modify. (See note)

    { conference_access_constants }
    
hint-function-tinkey=
    ### Parameters
    @1 = Number of clock ticks to wait for input.

    ### Returns
    Input entered by user

    ### Remarks
    Y1 is the number of clock ticks you wish `TINEKY` to wait for input
    before timing out. 1 second = 18 ticks (approx)
    
    A tick value of 0 will cause `TINKEY` to wait indefinatly for input with
    a maximum timout time of about 4 hours. Carrier loss will also terminate
    `TINKEY`.
hint-function-cwd=
    ### Returns
    The current working directory
hint-function-instrr=
    Returns the right most position of @2 in @1 `(1-LEN(@1))` or `0` if @2 not in @1
hint-function-base64enc=Encodes the bytes of @1 as base64 text. A string argument contributes its UTF-8 bytes.
hint-function-base64dec=Decodes base64 text in @1 to a byte blob. Malformed input reports `ErrCode.Format`.
hint-function-tobytes=The binary representation of @1 as a byte blob. Strings use UTF-8; numeric scalars use fixed-width little-endian storage.
hint-function-rgb=Packs red, green, blue and optional alpha components into an RGBA color.
hint-function-terminal=The caller's terminal and the root of graphics, input, margins, palette, fonts, macros, audio and cached capability information.
hint-function-board=A snapshot of the configured board: name, location, operator, sysop name, node count, conferences and registered users.
hint-function-session=The call in progress, read live: conference, areas, caller, security level, node, minutes left and language.
hint-statement-on-error=ON ERROR GOTO label | GOSUB label | Procedure | OFF - where a failed operation sends the program.
hint-statement-fgetrec=Reads one escaped text line per scalar field from channel @1 into record @2. The destination changes only after the complete record is valid; following lines remain unread.
hint-statement-fputrec=Writes record @2 to channel @1 as one escaped text line per scalar field. Additional documentation may be written after it with `FPUTLN`.
hint-statement-freadrec=Reads one length-framed binary record from channel @1 into @2. The destination changes only after the complete frame matches its record layout.
hint-statement-fwriterec=Writes record @2 to channel @1 as a compact length-framed binary value.
hint-function-fdordaka=
    Returns the address this board answers to, as zone:net/node with the point
    appended when there is one, or an empty string when there is no such record

    @1 = record number, counted from one
hint-function-fdordorg=
    Returns the origin line appended to echomail written here

    @1 = record number, counted from one. Only one origin line is configured,
    so every other number answers empty
hint-function-fdordarea=
    Returns the tag of a message area that takes part in the network, or an
    empty string when there is no such record

    @1 = record number, counted from one
hint-function-fdoqrd=
    Returns the file waiting under that number in the outbound queue, or an
    empty string when nothing waits there

    @1 = record number, counted from one
hint-function-getdrive=
    ### Returns
    The current drive letter

    ### Remarks
    Drive numbers correspond to drive letters in the following way
    A: = 0
    B: = 1
    C: = 2
    …
hint-function-setdrive=Selects DOS drive number `@1` and returns the selected drive number. IcyBoard has no DOS current-drive state, so this compatibility function returns its argument without changing path resolution.
hint-function-bs2i=
    Converts a 4 byte bsreal to a PPL integer.

    ### Parameters
    @1 is a BIGSTR type since BIGSTR types can contain
    binary data. For this function, PPL will convert the first
    4 bytes of the BIGSTR into an INTEGER variable and retun
    it.

    ### Returns
    Returns a converted 4 byte bsreal in the form of a 4 byte integer.
hint-function-bd2i=Converts an 8 byte bdreal to a PPL integer.
hint-function-i2bs=Converts a 4 byte PPL INTEGER into a 4 byte bsreal and stores it in a BIGSTR.
hint-function-i2bd=Converts a 4 byte PPL INTEGER into an 8 byte bdreal and stores it.
hint-function-ftell=
        `FTELL` returns the current file pointer offset for the specified
        file channel. If the channel is not open, it will return 0.
        Otherwise it will return the current position in the open file.

        ### Parameters
        @1 - The file channel to process
        
        ### Returns
        4 byte signed integer containing the file pointer offset
        of the file attached to channel.
hint-function-os=
        ### Returns
        An Integer indicating which operating system/pcboard version
        the PPE is currently running under.
        1=DOS, 2 = OS2, 0 = unknown.
hint-function-short_desc=
    ### Returns
    TRUE if the user has short file descriptions set to on else it returns FALSE.
hint-function-getbankbal=
    ### Parameters
    @1 The field to get.

    ### Returns
    Returns the value of a specified field.

    ### Fields

    Time Fields (in minutes)
    ------------------------
        0 = Last Deposit Date
        1 = Last Withdrawal Date
        2 = Last Transaction Amount (in minutes)
        3 = Amount Saved (their time balance in their account)
        4 = Max Withdrawal (the max a user can withdraw in a day)
        5 = Max Stored Amount (Maximum time allowed to be stored)

    Byte Fields (in K bytes)
    ------------------------
        6 = Last Deposit Date
        7 = Last Withdrawal Date
        8 = Last Transaction Amount (in K bytes)
        9 = Amount Saved (their K byte balance in their account)
        10 = Max Withdrawal (the max a user can withdraw in a day)
        11 = Max Stored Amount (Maximum K bytes allowed to be stored)

hint-function-getmsghdr=
    ### Parameters
    @1 = conference number of the message base
    @2 = A double expression stating the message number of the message to get the message header value.
    @3 = The field to get.

    ### Returns
    Returns the value of the specified field.

    { message_header_constants }
hint-function-setmsghdr=
    ### Parameters
    @1 = An integer expression stating the conference number of the message base.
    @2 = A double expression stating the message number of the message to set the message header value.
    @3 = An integer expression between 1 and 5 representing the field to get.
    @4 = A string expression containing the data to insert into the specified field.

    ### Fields
    1 = 'To' field
    2 = 'From' field
    3 = 'Subject' field
    4 = 'Password' field
    5 = 'Echo' Flag

    ### Returns
    Returns the value of the message number.  If the message will
    fit in the same place as the original then it will be the same.
    If modefied header change will not fit in the original message
    header then it will insert the message to the end of the message
    base.
hint-function-areaid=Generates a tuple conference/area to identify a message base.
hint-function-len_dim=
    @1 = The array whose element count is requested
    @2 = Zero-based dimension number (`0`, `1` or `2`)
    ### Returns
        Returns the element count in dimension @2, not its highest index. For example, an array declared with bound `[10]` has length 11. An invalid dimension returns 0.

hint-const-true=BOOLEAN `TRUE` value
hint-const-false=BOOLEAN `FALSE` value
hint-const-stk_limit=This constant was added so the PPL programmer could determine how close they are getting to the stack limit when using recursion.
hint-const-attach_lim_p=Public attach bytes limit
hint-const-attach_lim_u=Personal attach bytes limit
hint-const-f_net=Conference network-mail flag used with `CONFFLAG` and `CONFUNFLAG`.
hint-const-cmaxmsgs=Max Messages per conference
hint-const-maxmsgs=Max messages per qwk packet
hint-const-cur_user=Parameter passed to `CURUSER()`/Return by `GetUser` - User variables are for the current user
hint-const-no_user=Return by `GetUser` - variables are currently undefined
hint-const-acc_cur_bal=Selects the user's current, up-to-date account balance.
hint-const-acc_stat=
    Returns status of the "Enable Accounting" switch in the PWRD file.  

    0=Accounting disabled (N)
    1=Tracking (T), and 2=Enabled (Y).
hint-const-acc_time=The amount of ADDITIONAL units to charge per minute while in the current conference.
hint-const-acc_msgread=The amount to charge in ADDITION for each message read in the current conference.
hint-const-acc_msgwrite=The amount to charge in ADDITION for each message entered in the current conference.
hint-const-defs=Parameter passed to various statements for default values
hint-const-bell=Parameter passed to ``DISPTEXT`` statement (sound a bell when prompt displayed)
hint-const-logit=Parameter passed to `DISPTEXT` statement (log text to callers log)
hint-const-logitleft=Parameter passed to `DISPTEXT` statement (log text to callers log, forcing left justification)
hint-const-auto=Parameter passed to ``INPUTSTR`` and ``PROMPTSTR`` statements (automatically press enter after 10 seconds of no user input)
hint-const-echodots=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (echo dots instead of user input)
hint-const-eraseline=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (erase the current line when user presses enter)
hint-const-fieldlen=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (displays parenthesis to show input field width if ANSI enabled)
hint-const-guide=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (displays parenthesis above current line if FIELDLEN used and ANSI not enabled
hint-const-highascii=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (allow high ascii characters, regardless of current valid character set, if disable high ascii filter set to yes)
hint-const-lfafter=Parameter passed to `INPUTSTR`, `PROMPTSTR` and `DISPTEXT` statements (send an extra line feed after user presses enter)
hint-const-lfbefore=Parameter passed to `INPUTSTR`, `PROMPTSTR` and `DISPTEXT` statements (send an extra line feed before prompt display)
hint-const-newline=Parameter passed to `INPUTSTR`, `PROMPTSTR` and `DISPTEXT` statements (send a line feed after user presses enter)
hint-const-noclear=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (don't clear field at first keypress regardless of ANSI)
hint-const-stacked=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (allow semi-colons and spaces in addition to valid character set passed)
hint-const-upcase=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (force user input to upper case)
hint-const-wordwrap=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (if user hits end of line, save the text at the end of the line for future use)
hint-const-yesno=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (Only allow international yes/no responses)
hint-const-newbalance=Credits Given to a new user account
hint-const-chrg_call=Credits charged for a call
hint-const-chrg_time=Credits charged for time used (in minutes)
hint-const-chrg_peaktime=Credits charged for peak time used
hint-const-chrg_chat=Credits charged for chat session
hint-const-chrg_msgread=Credits charged for reading a message
hint-const-chrg_msgcap=Credits charged for capturing a message
hint-const-chrg_msgwrite=Credits charged for writing a message
hint-const-chrg_msgechoed=Credits charged for writing an echoed message
hint-const-chrg_msgprivate=Credits charged for writing a private message
hint-const-chrg_downfile=Credits charged for downloading a file
hint-const-chrg_downbytes=Credits charged for downloading bytes
hint-const-pay_upfile=Credits given for uploading a file
hint-const-pay_upbytes=Credits given for uploading bytes
hint-const-warnlevel=Credit threshold for low credit warning
hint-const-crc_file=
    These constants were added to avoid confusion when telling the function `CRC32` what it is taking the CRC of.
    CRCFILE tells `CRC32` to calculate the CRC of the file contained within the string argument.
    CRCFILE has a value of 1 (`TRUE`)
hint-const-crc_str=
    These constants were added to avoid confusion when telling the function `CRC32` what it is taking the CRC of.
    CRCSTR tells `CRC32` to calculate the CRC of the string argument itself.
    CRCSTR has a value of 0 (`FALSE`)
hint-const-start_bal=Users starting balance.
hint-const-start_session=Users starting balance for this session.
hint-const-deb_call=Debit for this call
hint-const-deb_time=Debit for time on
hint-const-deb_msgread=Debit for reading message
hint-const-deb_msgcap=Debit for capturing a message.
hint-const-deb_msgwrite=Debit for writing a message
hint-const-deb_msgechoed=Debit for echoed message
hint-const-deb_msgprivate=Debit for writing private message
hint-const-deb_downfile=Debit for downloading a file.
hint-const-deb_downbytes=Debit for downloading bytes
hint-const-deb_chat=Debit for chat
hint-const-deb_tpu=Debit for TPU
hint-const-deb_special=Debit special
hint-const-cred_upfile=Credit for uploading a file
hint-const-cred_upbytes=Credit for uploading bytes
hint-const-cred_special=Credit special
hint-const-sec_drop=Security level to drop to at 0 credits
hint-const-f_exp=Expired subscription access allowed flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-f_mw=Mail waiting flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-f_reg=Registered access allowed flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-f_sel=Conference selected flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-f_sys=Conference SysOp access flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-fcl=Value passed to `STARTDISP` to force line counting display
hint-const-fns=Value passed to `STARTDISP` to force non-stop display
hint-const-nc=Value passed to `STARTDISP` to not change display mode
hint-const-graph=Parameter passed to `DISPFILE` statement to search for graphics specific files
hint-const-sec=Parameter passed to `DISPFILE` statement to search for security specific files
hint-const-lang=Parameter passed to `DISPFILE` statement to search for language specific files
hint-const-hdr_active=Message active flag field
hint-const-hdr_blocks=Number of 128 byte blocks in message
hint-const-hdr_date=Date message was written
hint-const-hdr_echo=Echoed message flag
hint-const-hdr_from=Who the message is from
hint-const-hdr_msgnum=Message number
hint-const-hdr_msgref=Selects the message reference field in the current message header.
hint-const-hdr_pwd=Message password
hint-const-hdr_reply=Message reply flag
hint-const-hdr_rplydate=Reply message date
hint-const-hdr_rplytime=Reply message time
hint-const-hdr_status=Message status
hint-const-hdr_subj=Message subject
hint-const-hdr_time=Message time
hint-const-hdr_to=Message to field
hint-const-o_rd=Parameter passed to `FCREATE/FOPEN/FAPPEND` to open a file in read only mode
hint-const-o_rw=Parameter passed to `FCREATE/FOPEN/FAPPEND` to open a file in read and write mode
hint-const-o_wr=Parameter passed to `FCREATE/FOPEN/FAPPEND` to open a file in write only mode
hint-const-seek_cur=for the current file pointer location
hint-const-seek_end=for the end of the file
hint-const-seek_set=for the beginning of the file
hint-const-s_db=Parameter passed to `FCREATE/FOPEN/FAPPEND` to deny read and write (both) access from other processes
hint-const-s_dn=Parameter passed to `FCREATE/FOPEN/FAPPEND` to allow read and write (deny none) access from other processes
hint-const-s_dr=Parameter passed to `FCREATE/FOPEN/FAPPEND` to deny read access from other processes
hint-const-s_dw=Parameter passed to `FCREATE/FOPEN/FAPPEND` to deny write access from other processes


# Tables 

message_header_constants= 
 ### Message Header Field Access Constants
 
 | Field | hex | dec | Field Description |
 | :--- | :--- | :--- | :--- |
 | `HDR_ACTIVE`   | `0x0E` | `14`  | Message active flag field |
 | `HDR_BLOCKS`   | `0x04` | `4`   | Number of 128 byte blocks in message |
 | `HDR_DATE`     | `0x05` | `5`   | Date message was written |
 | `HDR_ECHO`     | `0x0F` | `15`  | Echoed message flag |
 | `HDR_FROM`     | `0x0B` | `11`  | Who the message is from |
 | `HDR_MSGNUM`   | `0x02` | `2`   | Message number | 
 | `HDR_MSGREF`   | `0x03` | `3`   | Reference message |
 | `HDR_PWD`      | `0x0D` | `13`  | Message password |
 | `HDR_REPLY`    | `0x0A` | `10`  | Message reply flag |
 | `HDR_RPLYDATE` | `0x08` | `8`   | Reply message date |
 | `HDR_RPLYTIME` | `0x09` | `9`   | Reply message time |
 | `HDR_STATUS`   | `0x01` | `1`   | Message status |
 | `HDR_SUBJ`     | `0x0C` | `12`  | Message subject |
 | `HDR_TIME`     | `0x06` | `6`   | Message time |
 | `HDR_TO`       | `0x07` | `7`   | Receiver of the message |
 
conference_access_constants=
 ### Fields
 | Value | Purpose | Type |
 | ---: | :--- | :--- |
 |1| Conference Name | STRING |
 |2| Public Conference|BOOLEAN |
 |3| Auto Rejoin|BOOLEAN |
 |4| View Other Users|BOOLEAN |
 |5| Make Uploads Private|BOOLEAN |
 |6| Make All Messages Private|BOOLEAN |
 |7| Echo Mail in Conf|BOOLEAN |
 |8| Required Security if public|INTEGER |
 |9| Additional Conference Security|INTEGER |
 |10| Additional Conference Time| INTEGER |
 |11| Number of Message Blocks| INTEGER |
 |12| Name/Loc of MSGS File| STRING |
 |13| Name/Loc of Users's Menu| STRING |
 |14| Name/Loc of Sysops Menu| STRING |
 |15| Name/Loc of NEWS file.| STRING |
 |16| Public Upload Sort| INTEGER |
 |17| Name/Loc upload DIR file| STRING |
 |18| Location of Public Uploads| STRING |
 |19| Private Upload Sort| INTEGER |
 |20| Name/Loc Private Upload DIR file| STRING |
 |21| Location of private uploads| STRING |
 |22| Doors Menu| STRING |
 |23| Doors File| STRING |
 |24| Bulletin Menu| STRING |
 |25| Bulletin File| STRING |
 |26| Script Menu| STRING |
 |27| Script File| STRING |
 |28| Directories Menu| STRING |
 |29| Directories File| STRING |
 |30| Download Paths File| STRING |
 |31| Force Echo on All Messages| BOOLEAN |
 |32| Make Conference Read Only| BOOLEAN |
 |33| Disallow Private Messages| BOOLEAN |
 |34| Level to Request Return Receipt| INTEGER |
 |35| Place Origin Info In Messages| BOOLEAN |
 |36| Prompt For Route info| BOOLEAN |
 |37| Allow Aliases to be used| BOOLEAN |
 |38| Show INTRO in 'R A' Scan| BOOLEAN |
 |39| Level to Enter a Message| INTEGER |
 |40| Password to Join if Private;| STRING |
 |41| Name/Loc of Conf INTRO File| STRING |
 |42| Location for Attachments| STRING |
 |43| Auto-Register Flags| STRING |
 |44| Level to Save File Attachment| BYTE |
 |45| Carbon Copy List Limit| BYTE |
 |46| Conf-Specific CMD.LST File| STRING |
 |47| Maintain Old MSGS.NDX File| BOOLEAN |
 |48| Allow Internet (long) TO: Names| BOOLEAN |
 |49| Level to Enter Carbon List Msgs| BYTE |
 |50| Type of NetMail Conference| BYTE |
 |51| Last Message Exported| INTEGER |
 |52| Charge Per Minute| DREAL |
 |53| Charge per Message Read| DREAL |
 |54| Charge per Message Written| DREAL |

accounting_constants= 
 ### Accounting Information
 
 | Field | dec | Field Description |
 | :--- |  :--- | :--- |
 | `NEWBALANCE`      | `0`  | Credits Given to a new user account |
 | `CHRG_CALL`       | `1`  | Credits charged for a call |
 | `CHRG_TIME`       | `2`  | Credits charged for time used (in minutes) |
 | `CHRG_PEAKTIME`   | `3`  | Credits charged for peak time used |
 | `CHRG_CHAT`       | `4`  | Credits charged for chat session |
 | `CHRG_MSGREAD`    | `5`  | Credits charged for reading a message | 
 | `CHRG_MSGCAP`     | `6`  | Credits charged for capturing a message |
 | `CHRG_MSGWRITE`   | `7`  | Credits charged for writing a message |
 | `CHRG_MSGECHOED`  | `8`  | Credits charged for writing an echoed message |
 | `CHRG_MSGPRIVATE` | `9`  | Credits charged for writing a private message |
 | `CHRG_DOWNFILE`   | `10` | Credits charged for downloading a file |
 | `CHRG_DOWNBYTES`  | `11` | Credits charged for downloading bytes |
 | `PAY_UPFILE`      | `12` | Credits given for uploading a file |
 | `PAY_UPBYTES`     | `13` | Credits given for uploading bytes |
 | `WARNLEVEL`       | `14` | Credit threshold for low credit warning |
