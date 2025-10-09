.. role:: PPL(code)
   :language: PPL


Statements
----------

CLS (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT CLS`

  **Parameters**
    * None

  **Returns**
    None

  **Description**
    Clears the caller’s (and local) display screen and resets cursor to home position.

  **Example**
    .. code-block:: PPL

       CLS
       PRINTLN "Welcome."

CLREOL (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT CLREOL`

  **Description**
    Clears from the current cursor position to the end of the line.

COLOR (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT COLOR(INTEGER attr)`

  **Parameters**
    * :PPL:`attr` – Packed color (foreground/background + attributes)

  **Description**
    Sets current output color. Use :PPL:`DEFCOLOR` / :PPL:`CURCOLOR()` to query defaults.

  **Example**
    .. code-block:: PPL

       COLOR 14
       PRINTLN "Yellow text"

PRINT (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT PRINT <expr_list>`

  **Description**
    Writes expressions to the console without appending a newline. Adjacent arguments separated by commas.

  **Example**
    .. code-block:: PPL

       PRINT "User: ", U_NAME()

PRINTLN (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT PRINTLN <expr_list>`

  **Description**
    Same as :PPL:`PRINT` but appends a newline at end.

  **Example**
    .. code-block:: PPL

       PRINTLN "Bytes left:", MINLEFT()

SPRINT / SPRINTLN (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SPRINT <expr_list>`
  :PPL:`STATEMENT SPRINTLN <expr_list>`

  **Description**
    “Secure” print variants that typically filter control/high ASCII or respect user flags (implementation dependent).

MPRINT / MPRINTLN (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT MPRINT <expr_list>`
  :PPL:`STATEMENT MPRINTLN <expr_list>`

  **Description**
    Message-area context print (legacy differentiation; acts like PRINT/PRINTLN under modern engine unless specialized).

NEWLINE (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT NEWLINE`

  **Description**
    Emits a single CR/LF pair (same as empty PRINTLN).

NEWLINES (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT NEWLINES(INTEGER count)`

  **Parameters**
    * :PPL:`count` – Number of blank lines to emit (<=0 no-op)

INPUT (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT INPUT(<VAR> target)`

  **Parameters**
    * :PPL:`target` – Variable to receive a raw line (basic editing)

  **Description**
    Reads a full line of user input (no masking/validation) into the variable.

INPUTSTR / INPUTINT / INPUTDATE / INPUTTIME / INPUTMONEY / INPUTCC (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTSTR(<VAR> target, INTEGER flags)`
  :PPL:`STATEMENT INPUTINT(<VAR> target, INTEGER flags)`
  :PPL:`STATEMENT INPUTDATE(<VAR> target, INTEGER flags)`
  :PPL:`STATEMENT INPUTTIME(<VAR> target, INTEGER flags)`
  :PPL:`STATEMENT INPUTMONEY(<VAR> target, INTEGER flags)`
  :PPL:`STATEMENT INPUTCC(<VAR> target, INTEGER flags)`

  **Parameters**
    * :PPL:`target` – Variable to fill
    * :PPL:`flags` – Bitwise OR of input behavior flags (e.g. FIELDLEN, UPCASE, ECHODOTS)

  **Description**
    Validating input routines specialized for type. For credit cards, format and Luhn validation can occur.

  **Example**
    .. code-block:: PPL

       INTEGER Age
       INPUTINT Age, FIELDLEN + UPCASE

INPUTYN (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTYN(<VAR> target, INTEGER flags)`

  **Description**
    Prompts for a Yes/No style single-key answer; stores 'Y' or 'N' (or configured YESCHAR/NOCHAR) into :PPL:`target`.

KILLMSG (3.20)
~~~~~~~~~~~~~~

  :PPL:`STATEMENT KILLMSG(INTEGER confnum, INTEGER msgnum)`

  **Parameters**
    * :PPL:`confnum` – Conference number containing the target message
    * :PPL:`msgnum`  – Message number to delete

  **Returns**
    None

  **Description**
    Deletes the specified message from the given conference (if it exists and permissions allow).

  **Example**

    .. code-block:: PPL

       KILLMSG 10,10234

  **Notes**
    Fails silently in legacy semantics if the message cannot be removed. Modern engines may log a warning.

  **See Also**
    (future) message management functions / queries


SOUNDDELAY (3.20)
~~~~~~~~~~~~~~~~~

  :PPL:`STATEMENT SOUNDDELAY(INTEGER frequency, INTEGER duration)`

  **Parameters**
    * :PPL:`frequency` – PC speaker tone frequency (legacy; ignored on some modern hosts)
    * :PPL:`duration`  – Clock ticks to sound (~18 ticks = 1 second)

  **Returns**
    None

  **Description**
    Produces a tone for the specified duration. Introduced to replace the DOS two-step
    SOUND on / SOUND off sequence (not portable to OS/2 or modern systems) with a single call.

  **Example**

    .. code-block:: PPL

       IF (inputVal <> validVal) SOUNDDELAY 500,18

  **Notes**
    May be a no-op on non-emulated systems. Consider providing a visual fallback.

  **See Also**
    (None)


USELMRS (3.20)
~~~~~~~~~~~~~~

  :PPL:`STATEMENT USELMRS(BOOLEAN useLmrs)`

  **Parameters**
    * :PPL:`useLmrs` – TRUE to load alternate user’s Last Message Read pointers on GETALTUSER; FALSE to suppress

  **Returns**
    None

  **Description**
    Controls whether subsequent :PPL:`GETALTUSER` calls will also load the target user's LMRS (Last Message Read pointers).
    Disabling can save memory when many conferences exist and LMRS data is not needed.

  **Example**

    .. code-block:: PPL

       USELMRS FALSE
       GETALTUSER 300
       PRINTLN "Skipped loading user 300's LMRS to save memory."
       USELMRS TRUE
       GETALTUSER 300
       PRINTLN "Now LMRS for user 300 are loaded."

  **Notes**
    Use the FUNCTION form :PPL:`USELMRS()` (if provided) to query current state.

  **See Also**
    * :PPL:`GETALTUSER`


ADDUSER (3.20)
~~~~~~~~~~~~~~

  :PPL:`STATEMENT ADDUSER(STRING username, BOOLEAN keepAltVars)`

  **Parameters**
    * :PPL:`username`     – Name of the new user
    * :PPL:`keepAltVars`  – TRUE leaves new user vars active (as if GETALTUSER on the new record); FALSE restores current user

  **Returns**
    None

  **Description**
    Creates a new user record with system defaults for all fields except the supplied name.

  **Example**

    .. code-block:: PPL

       ADDUSER "New Caller", TRUE
       PRINTLN "Created & switched context to: New Caller"

  **Notes**
    Validate for duplicates before creation if possible.

  **See Also**
    * :PPL:`GETALTUSER`
    * :PPL:`PUTALTUSER`


MKDIR (3.20)
~~~~~~~~~~~~

  :PPL:`STATEMENT MKDIR(STRING path)`

  **Parameters**
    * :PPL:`path` – Directory path to create

  **Returns**
    None

  **Description**
    Creates a directory (legacy DOS semantics). Intermediate path components are not automatically created.

  **Example**

    .. code-block:: PPL

       MKDIR "\PPE\TEST"

  **Notes**
    May fail silently if already exists or permissions deny.

  **See Also**
    * :PPL:`RMDIR()`
    * :PPL:`CWD()`


RMDIR (3.20)
~~~~~~~~~~~~

  :PPL:`STATEMENT RMDIR(STRING path)`

  **Parameters**
    * :PPL:`path` – Directory path to remove (must be empty)

  **Returns**
    None

  **Description**
    Removes an empty directory.

  **Example**

    .. code-block:: PPL

       RMDIR "\PPE\TEST"

  **Notes**
    Will not remove non-empty directories.

  **See Also**
    * :PPL:`MKDIR()`
    * :PPL:`CWD()`



ADJTUBYTES (3.20)
~~~~~~~~~~~~~~~~~

  :PPL:`STATEMENT ADJTUBYTES(INTEGER deltaBytes)`

  **Parameters**
    * :PPL:`deltaBytes` – Positive or negative number of bytes to adjust the user's upload total

  **Returns**
    None

  **Description**
    Adjusts the tracked total upload bytes for the (current or alternate) user.

  **Example**

    .. code-block:: PPL

       GETALTUSER 10
       ADJTUBYTES -2000
       PUTALTUSER

  **Notes**
    Pair with :PPL:`GETALTUSER` / :PPL:`PUTALTUSER` to persist for alternate users.

  **See Also**
    (future accounting helpers)


GRAFMODE (3.20)
~~~~~~~~~~~~~~~

  :PPL:`STATEMENT GRAFMODE(INTEGER mode)`

  **Parameters**
    * :PPL:`mode` – Display mode selector:
      * 1 = Color ANSI (if user supports)
      * 2 = Force color ANSI
      * 3 = ANSI black & white
      * 4 = Non-ANSI (plain)
      * 5 = RIP (if supported)

  **Returns**
    None

  **Description**
    Switches the caller’s graphics/terminal capability mode.

  **Example**

    .. code-block:: PPL

       PRINTLN "Switching to color ANSI…"
       GRAFMODE 1

  **Notes**
    Forcing modes unsupported by user terminal may cause display corruption.

  **See Also**
    Terminal / capability query functions (future)


FDOQADD (3.20)
~~~~~~~~~~~~~~

  :PPL:`STATEMENT FDOQADD(STRING addr, STRING file, INTEGER flags)`

  **Parameters**
    * :PPL:`addr`  – FidoNet destination address
    * :PPL:`file`  – Packet / file to queue
    * :PPL:`flags` – Delivery mode: 1=NORMAL, 2=CRASH, 3=HOLD

  **Returns**
    None

  **Description**
    Adds a record to the Fido queue for later processing.

  **Example**

    .. code-block:: PPL

       FDOQADD "1/311/40","C:\PKTS\094FC869.PKT",2

  **Notes**
    Paths should be validated; behavior undefined if file not present.

  **See Also**
    * :PPL:`FDOQMOD()`
    * :PPL:`FDOQDEL()`


FDOQMOD (3.20)
~~~~~~~~~~~~~~

  :PPL:`STATEMENT FDOQMOD(INTEGER recnum, STRING addr, STRING file, INTEGER flags)`

  **Parameters**
    * :PPL:`recnum` – Existing queue record number to modify
    * :PPL:`addr`   – Updated FidoNet address
    * :PPL:`file`   – Updated file path
    * :PPL:`flags`  – 1=NORMAL, 2=CRASH, 3=HOLD

  **Returns**
    None

  **Description**
    Modifies an existing Fido queue entry.

  **Example**

    .. code-block:: PPL

       FDOQMOD 6,"1/311/40","C:\PKTS\UPDATED.PKT",1

  **Notes**
    Duplicate legacy doc blocks collapsed into one canonical entry.

  **See Also**
    * :PPL:`FDOQADD()`
    * :PPL:`FDOQDEL()`


FDOQDEL (3.20)
~~~~~~~~~~~~~~

  :PPL:`STATEMENT FDOQDEL(INTEGER recnum)`

  **Parameters**
    * :PPL:`recnum` – Queue record to delete

  **Returns**
    None

  **Description**
    Deletes a Fido queue record.

  **Example**

    .. code-block:: PPL

       FDOQDEL 6

  **Notes**
    Deleting a non-existent record has no effect (legacy behavior).

  **See Also**
    * :PPL:`FDOQADD()`
    * :PPL:`FDOQMOD()`


CONFINFO (Modify) (3.20)
~~~~~~~~~~~~~~~~~~~~~~~~

  :PPL:`STATEMENT CONFINFO(INTEGER confnum, INTEGER field, VAR newValue)`

  **Parameters**
    * :PPL:`confnum`  – Conference number
    * :PPL:`field`    – Field selector (1–54)
    * :PPL:`newValue` – Value to assign (type must match field definition)

  **Returns**
    None

  **Description**
    Writes a single conference configuration field. Field meanings mirror the FUNCTION
    form (see earlier table for 1–54). Only appropriate types are accepted.

    Security / Privacy:
      Field 40 (Join Password) SHOULD be handled carefully. Avoid logging or echoing this value.

  **Example**

    .. code-block:: PPL

       CONFINFO 200,1,"Stan's New Conference Name"

  **Notes**
    Writing invalid types may produce runtime errors or be ignored depending on implementation.

  **See Also**
    * :PPL:`CONFINFO()` (read / variant form)


PROMPTSTR (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT PROMPTSTR(<VAR> target, INTEGER flags)`

  **Description**
    Like INPUTSTR but prints a system prompt first (legacy UI consistency).

TOKENIZE (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT TOKENIZE(STRING line)`

  **Parameters**
    * :PPL:`line` – Source to break into tokens for later :PPL:`GETTOKEN()` / :PPL:`TOKCOUNT()`

  **Description**
    Loads the internal token buffer with split tokens (whitespace / delimiter rules legacy-defined).

GETTOKEN (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT GETTOKEN(<VAR> target)`

  **Description**
    Pops next token (or empty if none) into :PPL:`target`.

SHELL (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT SHELL(STRING command)`

  **Description**
    Executes a system shell / external program (availability/security can be restricted).

BYE / GOODBYE (1.00)
~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT BYE`
  :PPL:`STATEMENT GOODBYE`

  **Description**
    Terminates user session gracefully (GOODBYE synonym). May trigger logoff scripts, accounting flush.

HANGUP (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT HANGUP`

  **Description**
    Immediate disconnect / carrier drop (hard termination). Prefer BYE for clean logout.

LOG (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT LOG(STRING line)`

  **Description**
    Appends :PPL:`line` to the system activity / event log.

DELAY (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT DELAY(INTEGER ticks)`

  **Parameters**
    * :PPL:`ticks` – ~18 per second

  **Description**
    Sleeps (non-busy) for specified ticks unless carrier loss or abort condition.

WAIT (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT WAIT(INTEGER ticks)`

  **Description**
    Similar to DELAY but may flush output first or enforce a minimum pacing (legacy pacing semantics).

BEEP (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT BEEP`

  **Description**
    Emits an audible terminal bell (Ctrl-G) if user’s terminal supports it.

KBDSTUFF (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDSTUFF(STRING text)`

  **Description**
    Queues keystrokes into the input buffer as if typed by the caller.

KBDFLUSH / KBDCHKON / KBDCHKOFF (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDFLUSH`
  :PPL:`STATEMENT KBDCHKON`
  :PPL:`STATEMENT KBDCHKOFF`

  **Description**
    Manage keyboard buffering and carrier/abort key checks.

SENDMODEM (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SENDMODEM(STRING raw)`

  **Description**
    Sends raw bytes (unfiltered) to remote terminal/modem (legacy; may be sanitized in modern environments).

PAGEON / PAGEOFF (1.00)
~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT PAGEON`
  :PPL:`STATEMENT PAGEOFF`

  **Description**
    Enable/disable user “page” requests (sysop chat paging).

CHAT (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT CHAT`

  **Description**
    Enters sysop chat mode if available (toggles live keyboard sharing).

FLAG (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT FLAG(INTEGER flagId)`

  **Description**
    Sets a transient per-session flag bit (implementation-defined). Often used with prompt display logic.

ALIAS (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT ALIAS(STRING newName)`

  **Description**
    Temporarily changes display name (legacy; may not persist).

GETUSER / PUTUSER (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT GETUSER(INTEGER record)`
  :PPL:`STATEMENT PUTUSER`

  **Parameters (GETUSER)**
    * :PPL:`record` – User record number

  **Description**
    Loads user record into current context / writes modified current user back to storage.

GETALTUSER / FREALTUSER / PUTALTUSER (1.00 / 3.20+ semantics)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT GETALTUSER(INTEGER record)`
  :PPL:`STATEMENT FREALTUSER`
  (Persist changes with :PPL:`PUTALTUSER` (if provided) or :PPL:`PUTUSER` after adjusting context.)

  **Description**
    Loads an alternate user profile (for inspection/modification) while preserving original active user data.

ADJTIME (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT ADJTIME(INTEGER deltaMinutes)`

  **Description**
    Adjusts remaining time this call by :PPL:`deltaMinutes` (negative to subtract).

ADJBYTES / ADJTBYTES / ADJDBYTES / ADJTFILES (1.00+)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT ADJBYTES(INTEGER delta)`
  :PPL:`STATEMENT ADJTBYTES(INTEGER delta)` (uploads)
  :PPL:`STATEMENT ADJDBYTES(INTEGER delta)` (downloads)
  :PPL:`STATEMENT ADJTFILES(INTEGER delta)` (upload file count)

  **Description**
    Adjust quota/accounting counters. Prefer the more explicit *T*/*D* forms when available.  
    (You already documented :PPL:`ADJTUBYTES`—the upload bytes variant in expanded semantics.)

DELETE / RENAME (1.00)
~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT DELETE(STRING file)`
  :PPL:`STATEMENT RENAME(STRING old, STRING new)`

  **Description**
    Remove or rename a filesystem entry (basic DOS semantics; silent failure if missing or permission denied).

FCREATE / FOPEN / FAPPEND (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FCREATE(INTEGER ch, STRING file, INTEGER access, INTEGER share)`
  :PPL:`STATEMENT FOPEN(INTEGER ch, STRING file, INTEGER access, INTEGER share)`
  :PPL:`STATEMENT FAPPEND(INTEGER ch, STRING file, INTEGER access, INTEGER share)`

  **Parameters**
    * :PPL:`ch` – Channel number (1–8)
    * :PPL:`file` – Path
    * :PPL:`access` – One of :PPL:`O_RD`, :PPL:`O_WR`, :PPL:`O_RW`
    * :PPL:`share` – One of :PPL:`S_DN`, :PPL:`S_DR`, :PPL:`S_DW`, :PPL:`S_DB`

  **Description**
    Opens a file for subsequent buffered I/O. Create always truncates/creates; Append opens write and seeks end.

  **Example**
    .. code-block:: PPL

       FCREATE 1,"log.txt",O_WR,S_DN
       FPUTLN 1,"Session start"
       FCLOSE 1

FPUT / FPUTLN / FPUTPAD (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FPUT(INTEGER ch, STRING data)`
  :PPL:`STATEMENT FPUTLN(INTEGER ch, STRING data)`
  :PPL:`STATEMENT FPUTPAD(INTEGER ch, STRING data, INTEGER width)`

  **Description**
    Write text (optionally newline or right-pad to width).

FGET (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT FGET(INTEGER ch, <VAR> target, INTEGER length)`

  **Description**
    Reads up to :PPL:`length` bytes (or line depending on legacy mode) into :PPL:`target`.

FSEEK (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT FSEEK(INTEGER ch, INTEGER offset, INTEGER whence)`

  **Parameters**
    * :PPL:`whence` – :PPL:`SEEK_SET`, :PPL:`SEEK_CUR`, :PPL:`SEEK_END`

FFLUSH (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT FFLUSH(INTEGER ch)`

  **Description**
    Forces buffered channel output to disk.

FCLOSE / FCLOSEALL (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FCLOSE(INTEGER ch)`
  :PPL:`STATEMENT FCLOSEALL`

  **Description**
    Close one or all open channels (releases locks).

FREAD / FWRITE (1.00)
~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FREAD(INTEGER ch, <VAR> bigstrTarget, INTEGER bytes)`
  :PPL:`STATEMENT FWRITE(INTEGER ch, BIGSTR buffer, INTEGER bytes)`

  **Description**
    Raw byte read/write (binary).

FREWIND (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FREWIND(INTEGER ch)`

  **Description**
    Equivalent to :PPL:`FSEEK ch,0,SEEK_SET`.

DISPFILE / DISPTEXT / DISPSTR (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT DISPFILE(STRING file, INTEGER flags)`
  :PPL:`STATEMENT DISPTEXT(STRING text, INTEGER flags)`
  :PPL:`STATEMENT DISPSTR(STRING text)`

  **Description**
    Display PCBoard @-code aware content (file or inline). Flags may control paging, security, or language substitution.

RESETDISP / STARTDISP (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT RESETDISP`
  :PPL:`STATEMENT STARTDISP(INTEGER flags)`

  **Description**
    Manage internal buffered display/paging state.

JOIN (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT JOIN(INTEGER confnum)`

  **Description**
    Switches current conference (permission verified).

CONFFLAG / CONFUNFLAG (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT CONFFLAG(INTEGER confnum, INTEGER flagMask)`
  :PPL:`STATEMENT CONFUNFLAG(INTEGER confnum, INTEGER flagMask)`

  **Description**
    Set / clear specific conference attribute bits (F_MW, F_SYS, etc.).

BITSET / BITCLEAR (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT BITSET(<VAR> var, INTEGER bit)`
  :PPL:`STATEMENT BITCLEAR(<VAR> var, INTEGER bit)`

  **Description**
    Sets or clears (0-based) bit in integer variable.

INC / DEC (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT INC(<VAR> var)`
  :PPL:`STATEMENT DEC(<VAR> var)`

  **Description**
    var = var ± 1 (legacy bytecode convenience).

ALIAS (already documented above, retained for clarity)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

SAVESCRN / RESTSCRN (1.00)
~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SAVESCRN`
  :PPL:`STATEMENT RESTSCRN`

  **Description**
    Save/restore current screen buffer (local + remote if supported).

ANSIPOS (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT ANSIPOS(INTEGER col, INTEGER row)`

  **Description**
    Directly positions cursor (1-based coordinates).

KBDSTRING (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDSTRING(STRING str)`

  **Description**
    Inject entire string into keyboard buffer (contrast :PPL:`KBDSTUFF` which may differ historically).

SETENV (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT SETENV(STRING name, STRING value)`

  **Description**
    Sets (or overrides) an environment variable for subsequent processes / shell calls.

CHDIR (3.20)
~~~~~~~~~~~~
  :PPL:`STATEMENT CHDIR(STRING path)`

  **Description**
    Changes the current working directory.

RENAME (already included above)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

SHORTDESC (3.20)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SHORTDESC(STRING text)`

  **Description**
    Sets a short descriptive string for the PPE (shown in sysop listings / logs).

MOVEmsg (3.20)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT MOVEMSG(INTEGER fromConf, INTEGER msgNum, INTEGER toConf)`

  **Description**
    Moves a message between conferences (permissions & existence required).

SETBANKBAL (3.20)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SETBANKBAL(INTEGER userRec, MONEY amount)`

  **Description**
    Adjusts stored “bank” balance (economy/game feature – semantics engine-defined).

WEBREQUEST (400 tentative)
~~~~~~~~~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT WEBREQUEST(STRING url, <VAR> responseBigStr)`

  **Description**
    Experimental HTTP GET/HEAD style fetch populating response data (subject to change; may require runtime 400).

D* Database / Table Primitives (Overview)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
  (Full per-statement docs can be added—summary here)

  * :PPL:`DCREATE name, layout…` – Create structured data file
  * :PPL:`DOPEN name` / :PPL:`DCLOSE` / :PPL:`DCLOSEALL`
  * Record navigation: :PPL:`DTOP`, :PPL:`DBOTTOM`, :PPL:`DGO n`, :PPL:`DSKIP delta`
  * CRUD: :PPL:`DADD`, :PPL:`DAPPEND`, :PPL:`DBLANK` (new empty), :PPL:`DDELETE`, :PPL:`DRECALL`
  * Locking: :PPL:`DLOCK`, :PPL:`DLOCKR`, :PPL:`DLOCKG`, :PPL:`DUNLOCK`
  * Field IO: :PPL:`DGET`, :PPL:`DPUT`
  * Index / seek: :PPL:`DSEEK`, :PPL:`DFCOPY`
  * Alias / pack: :PPL:`DSETALIAS`, :PPL:`DPACK`
  * NewName variants (DN*) manage named index or alt dataset.

  Add a request if you want these expanded in the same detailed template.
