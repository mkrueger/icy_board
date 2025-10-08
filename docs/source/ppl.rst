.. role:: PPL(code)
   :language: PPL

Introduction to PPL
===================

PPL (PCBoard Programming Language) is the classic scripting language
used to extend and customize the PCBoard Bulletin Board System.  
Icy Board ships with a modern, memory-safe, fully reimplemented PPL toolchain:

* A virtual machine (runtime) executing PPE (compiled PPL) modules
* A modern, stricter compiler: ``pplc``
* A robust decompiler: ``ppld`` (recovers structure from legacy PPEs)
* A Language Server (LSP) + VS Code extension for syntax help, hovers, navigation
* Extended language versions introducing optional new syntax and data types

Core Goals
----------

1. High compatibility with PCBoard PPEs up to 15.4 (run, decompile, recompile)
2. Safe modernization (UTF-8 source, stricter diagnostics, secure password handling)
3. Progressive evolution (optional newer *language versions* that do not break older scripts unless you opt in)
4. Better tooling (warnings instead of silent miscompiles; IDE support; disassembly view)
5. Eliminate “anti-decompile” era tricks—make maintenance possible again

Vocabulary: Runtime vs Language Version
---------------------------------------

You will see two related version notions:

* **Runtime / PPE format version**: The bytecode / PPE container format (100-400).
* **Language version**: The surface syntax & feature set you target.  
  By default language version = runtime version unless overridden (``--lang-version``).

You can (for example) generate a PPE in runtime format 400 but restrict yourself
to language features of 340 to stay compatible with older boards (where applicable).

Toolchain Overview
------------------

+-----------+------------------------------------------------------------+
| Tool      | Purpose                                                    |
+===========+============================================================+
| `pplc`    | Compile `.pps` (UTF-8 or CP437) into a PPE (PCBoard Exec)  |
| `ppld`    | Decompile an existing PPE back to readable PPL             |
| LSP       | Editor services: outline, hover help, go-to, completion    |
| Disasm    | Optional internal view of low-level instructions           |
+-----------+------------------------------------------------------------+


``ppld`` - The Decompiler
-------------------------

.. code-block:: bash

   ppld hello.ppe

Produces ``hello.ppd`` plus (optionally) a reconstructed control structure
instead of flat GOTO spaghetti. Use ``-d`` to view a disassembly and ``-r`` for
a minimal (raw) form.

Encoding & Character Set
------------------------

* **Preferred input**: UTF-8 (modern editors)
* **Legacy**: Original DOS sources were CP437. Use ``--cp437`` if auto-detection fails.
* Compiler outputs CP437 in the PPE so legacy display semantics match PCBoard expectations.
* You may convert existing PPE data files to UTF-8 with: ``icbsetup ppe-convert <PATH>`` (make backups first).

Key Differences vs Legacy PPLC (Summary)
----------------------------------------

(See the detailed “PPL differences” section in ``ppl.md`` for the full list.)

* Reserved words: a larger set is now treated as keywords (``IF``, ``FOR``, ``CONTINUE``, etc.) to prevent ambiguous parses.
* Stricter: mismatched function return declarations are **errors** instead of silently ignored.
* Additional identifier support (e.g. UTF-8 cases, the Euro sign).
* Cleaner loop constructs / assignment operators in higher language versions.
* DECLARE blocks no longer required in newer versions (&gt;= 350).
* More (and safer) warnings for suspicious code; treat warnings seriously when porting.

Evolution by Language Version
-----------------------------

* **&lt;= 340**: Classic era; close to PCBoard 15.4 semantics.
* **350** (PPL 4.0 modernization stage 1):
  
  - New loop forms: :PPL:`REPEAT ... UNTIL` and :PPL:`LOOP ... ENDLOOP`
  - Assignment operators (:PPL:`+= -= *= /= %= &=`, etc.)
  - Inline :PPL:`RETURN expr` (instead of assigning to function name)
  - Optional braces disambiguation improvements
  - Variable initializers: :PPL:`TYPE VAR = expr` or array initializer :PPL:`TYPE VAR = { a, b, c }`

* **400** (In progress – **experimental / subject to change**):

  - Distinct usage of :PPL:`[]` for indexing, :PPL:`{}` exclusively for array literals
  - Emerging *object-style* access to BBS domain entities (e.g. :PPL:`CONFERENCE` objects, with member properties & helper functions)
  - Overloadable predefined functions (e.g. dual :PPL:`CONFINFO` forms)
  - Goal: reduce need for manual file / config parsing in scripts

Use language gating to write compatible code:

.. code-block:: PPL

   ;$IF VERSION < 350
       PRINTLN "Legacy path"
   ;$ELSE
       PRINTLN "Newer language features enabled"
   ;$ENDIF

Preprocessor Summary
--------------------

Directives (start with ``;$`` on their own line):

* :PPL:`;$DEFINE NAME[=VALUE]` – define a symbol (value optional)
* :PPL:`;$IF expr` / :PPL:`;$ELIF expr` / :PPL:`;$ELSE` / :PPL:`;$ENDIF` – conditional compilation
* Token substitutions: :PPL:`;#Version` :PPL:`;#Runtime` :PPL:`;#LangVersion` expand to numeric values

Simple example:

.. code-block:: PPL

   PRINTLN "Compiler Version:", ;#Version
   ;$IF LANGVERSION >= 350
       PRINTLN "Modern language features active."
   ;$ENDIF

Types & Data Model (High Level)
-------------------------------

* Scalars: Integer, Unsigned, Byte / Word, Boolean, Float, Double, Money, Date, Time
* Strings: Normal and “BigStr” (large string buffers) 
* Arrays: 1–3 dimensional (indexed, zero-based internally)
* Password values (internally hashed if Argon2 / BCrypt storage is enabled)
* (Planned / partial in 400) Domain objects: Conference, MessageArea, FileArea, with member-like accessors or function wrappers.

Security & Safety Notes
-----------------------

* Passwords: Hashing (Argon2 / BCrypt) is enforced by configuration; scripts that attempt to transform (uppercase/lowercase) hashed values should expect no-ops.
* Avoid relying on internal hashes—display calls typically mask them.
* VM isolates runtime; catastrophic host crashes from buggy PPE logic are far harder now (memory safety from Rust).

Migration Workflow (Legacy PPE → Modern PPL)
--------------------------------------------

1. **Decompile** legacy ``FOO.PPE`` → ``FOO.PPS`` with ``ppld``.
2. **Review warnings** when recompiling with ``pplc``; fix shadowed variables, questionable assignments, or deprecated idioms.
3. **Decide language version**: If you need pure compatibility, stick to 340. If modern loops / returns help clarity, switch to 350.
4. **Run under Icy Board**; validate interactive paths (menus, door launching, display).
5. **Iterate**: Use LSP tooling for rename, find references, and incremental modernization.

Disassembly for Learning
------------------------

Use:

.. code-block:: bash

   pplc myscript.pps --disassemble
   # or
   ppld legacy.ppe --disassemble

This produces a low-level opcode view. Helpful for verifying optimizer or diagnosing control-flow reconstruction.

Quick Reference Cheat Card
--------------------------

.. code-block:: text

   Compile:    pplc script.pps
   Decompile:  ppld module.ppe
   Disasm:     pplc script.pps -d
   Encoding:   pplc --cp437 legacy.pps
   Lang ver:   pplc script.pps --lang-version 350
   PPE ver:    pplc script.pps --version 400


Developing PPL Applications
===========================

Create ``hello.pps``:

.. code-block:: text

   PRINTLN "Hello from Icy Board PPL!"

Compile:

.. code-block:: bash

   pplc hello.pps

Result: ``hello.ppe``


PCBoard Programming Language (PPL)
==================================

CONSTANTS & VARIABLES
---------------------


PPL Statements
--------------

This section enumerates all executable statement forms recognized by the modern Icy Board PPL compiler/VM.  
Internal AST variants like ``Block`` or ``Empty`` are not user-written and are omitted.

Version legend:
  (100+)  Available since earliest supported baseline (classic PCB era).
  (200+)  Introduced when SELECT/CASE became available.
  (300+)  Introduced with DECLARE / FUNCTION / PROCEDURE formalization.
  (350+)  Modernization wave (repeat/until, loop/endloop, inline return expr, compound assignments).

Control Flow
~~~~~~~~~~~~

IF single-line (100+)
  Syntax: :PPL:`IF ( <expr> ) <statement>`
  Executes exactly one following statement if expression is TRUE (non-zero / non-empty). No ELSE on same line.

IF / THEN multi-line (100+)
  Syntax skeleton::
  
     IF ( <expr> ) THEN
         <statements>
     [ELSEIF ( <expr> ) THEN
         <statements>]...
     [ELSE
         <statements>]
     ENDIF
  
  Notes:
    * Parentheses are required around the condition in modern forms.
    * :PPL:`ELSEIF` chains are evaluated in order; first TRUE branch wins.
    * :PPL:`ENDIF` terminator required.

SELECT / CASE (200+)
  Multi-way conditional.
  Syntax::
  
     SELECT ( <expr> )
         CASE <const_expr>[, <const_expr>...]:
             <statements>
         [CASE <const_expr_range_or_value_list>:
             <statements>]...
         [DEFAULT:
             <statements>]
     ENDSELECT
  
  Notes:
    * Comparison is by value (string/integer/date/etc.) with standard PPL coercions.
    * Multiple values per CASE separated by commas.
    * Range syntax (e.g. 1..5) is supported where decompiler emits it.
    * ``DEFAULT`` optional.

WHILE single-line (100+)
  Syntax: ``WHILE ( <expr> ) <statement>``
  Evaluates condition before each iteration; terminates when FALSE.

WHILE / ENDWHILE block (100+)
  Syntax::
  
     WHILE ( <expr> )
         <statements>
     ENDWHILE

DO WHILE style (WhileDo AST) (100+ legacy form)
  Some legacy PPEs decompile to a block starting with ``WHILE`` ending with ``ENDWHILE`` (same as above).  
  The engine distinguishes single-line vs block internally; syntax to author is identical to "block" form.

REPEAT / UNTIL (350+)
  Post-condition loop (always executes body at least once).
  Syntax::
  
     REPEAT
         <statements>
     UNTIL ( <expr> )
  
  Loop ends when expression becomes TRUE (reverse of ``WHILE`` semantics).

LOOP / ENDLOOP (350+)
  General loop for complex flows with manual BREAK/CONTINUE.
  Syntax::
  
     LOOP
         <statements>
     ENDLOOP
  
  Equivalent to ``while TRUE`` with explicit termination via BREAK.

FOR / NEXT (100+)
  Counter iteration.
  
  Syntax::
  
     FOR <identifier> = <start_expr> TO <end_expr> [STEP <step_expr>]
         <statements>
     NEXT
  
  or (legacy synonym) ``ENDFOR`` in place of ``NEXT`` (mapped internally).
  
  Notes:
  * Counter variable is (re)assigned the start value first.
  * Step defaults to 1 or -1? (In classic PCBoard PPL: default is +1; negative requires explicit STEP -1.)
  * Inclusive end bound (executes while counter <= end when step > 0, or >= end when step < 0).
  * Modifying the loop variable inside the body is allowed but discouraged (can cause skipped termination).


GOTO (100+)
  Syntax: ``GOTO <label>``
  Transfers control unconditionally to a declared label (``:<label>`` somewhere earlier or later).  
  Use sparingly; prefer structured constructs.

GOSUB (100+)
  Syntax: ``GOSUB <label>``
  Pushes return point and jumps to label. Return occurs when an implicit ``RETURN`` or fall-through to end?  
  In modern Icy Board PPL you normally use PROCEDURE/FUNCTION; GOSUB is legacy support.

Labels (100+)
  Syntax: ``:<label_name>``
  Declares a target for GOTO/GOSUB/BREAK label forms. Must be at statement start.  
  Case-insensitive. Decompiler emits uppercase or original style.

BREAK (100+; extended label form 350+)
  Syntax:
    * Unlabeled: ``BREAK`` — exits innermost loop (WHILE / FOR / REPEAT / LOOP / SELECT inside loop).
    * Labeled (350+): ``BREAK :MyLabel`` — jumps out to label (decompiler may emit for structured transforms).

CONTINUE (100+; labeled 350+)
  Syntax:
    * ``CONTINUE`` — skips to next iteration of current loop.
    * ``CONTINUE :Label`` (350+) — advanced flow (rare; produced by transformations).

RETURN (100+; inline expression 350+)
  Syntax (classic): ``RETURN``  
  Syntax (modern 350+): ``RETURN <expr>`` inside a FUNCTION (or to exit PROCEDURE early ignoring value).
  If legacy code assigns to the FUNCTION name and uses plain ``RETURN``, both styles coexist.

Procedural & Calls
~~~~~~~~~~~~~~~~~~

Procedure call (user-defined) (300+ for explicit PROCEDURE syntax; existed implicitly earlier)
  Syntax:
    * With arguments: ``ProcName(arg1, arg2, ...)``
    * No arguments: ``ProcName``

  Parentheses optional if no arguments (but recommended for clarity in modern code).

Predefined call (Built-in procedure statement) (100+)
  Examples: ``PRINT <expr_list>``, ``PRINTLN ...``, ``BYE``, file / user / message operations.
  Grammar:
  * Some accept argument lists separated by commas.
  * See "Predefined Procedures" section (not duplicated here).

GOSUB / RETURN pair (legacy)
  See above under control flow. Consider rewriting heavy gosub usage into PROCEDURE/FUNCTION for clarity.

Assignment & Variables
~~~~~~~~~~~~~~~~~~~~~~

LET / implicit LET (100+)
  Classic form allows optional ``LET`` keyword:
  
  ``LET Var = <expr>`` or ``Var = <expr>``
  
  Modern assignments (350+) support compound operators:
  
  * ``=`` (assign)
  * ``+=`` ``-=`` ``*=`` ``/=`` ``%=`` (arithmetic)
  * ``&=`` ``|=`` (bitwise / logical according to operand types)
  
  Array / function-like indexing assignment:
  
  ``ArrayVar(i, j) = <expr>``  
  (Parens style maintained for compatibility; some newer docs may show ``ArrayVar[i, j]`` when 400 object/index syntax fully stabilizes.)

Variable Declaration (typed) (300+)
  Syntax prototype::
  
     <TYPE> Var1[, Var2, Var3]
     <TYPE> ArrayName(dim1[, dim2[, dim3]])
  
  Types (summary): BOOLEAN, INTEGER, UNSIGNED, BYTE, WORD, SBYTE, SWORD, MONEY, FLOAT, DOUBLE, STRING, BIGSTR, DATE, EDATE, TIME, DDATE, TABLE, MESSAGEAREAID, PASSWORD (plus future USERDATA objects 400+).
  
  Declarations can appear at top level (global) or at start of procedure/function bodies.  
  (Versions <300 historically inferred variables on first assignment; modern compiler encourages explicit declarations for clarity and diagnostics.)

Comments
~~~~~~~~

Single-line (100+)
  * Semicolon: ``; This is a comment`` (preferred)
  * Apostrophe: ``' Also valid`` (legacy)
  * Leading ``*``: ``* Legacy style`` (still recognized for imported sources)

Block comments (350+ experimental)
  Parser supports multi-line block markers internally (emitted rarely by tools). Prefer single-line forms for portability.

Miscellaneous
~~~~~~~~~~~~~

SELECT fall-through
  There is no implicit fall-through between CASE blocks; each CASE’s block executes fully then control jumps to ENDSELECT (unless ``BREAK`` inside a nested loop is interpreted). Use multiple CASE value lists instead of stacked empty CASEs.

END (synthetic)
  The decompiler may show an internal ``End`` comment or label transformation; you do not author a standalone ``END`` statement in modern PPL (``ENDIF``, ``ENDWHILE``, ``ENDSELECT``, ``ENDLOOP``, ``NEXT`` serve as terminators).

Deprecated / Discouraged Patterns
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* Heavy :PPL:`GOTO` / :PPL:`GOSUB` chains → replace with PROCEDURE / FUNCTION.
* Relying on implicit variable creation (pre-300 era) → add explicit declarations.
* Using uppercase/lowercase inconsistently for labels → labels are case-insensitive but pick one style (snake_case or ALLCAPS).
* Modifying loop variables inside FOR other than via STEP semantics → can create subtle off-by-one termination behavior.

Version Feature Matrix
~~~~~~~~~~~~~~~~~~~~~~

+------------------+---------+---------+---------+---------+
| Statement / Form | 100–199 | 200–299 | 300–349 | 350+    |
+==================+=========+=========+=========+=========+
| IF/THEN          | Yes     | Yes     | Yes     | Yes     |
+------------------+---------+---------+---------+---------+
| SELECT/CASE      | -       | Yes     | Yes     | Yes     |
+------------------+---------+---------+---------+---------+
| WHILE            | Yes     | Yes     | Yes     | Yes     |
+------------------+---------+---------+---------+---------+
| FOR/NEXT         | Yes     | Yes     | Yes     | Yes     |
+------------------+---------+---------+---------+---------+
| GOTO / GOSUB     | Yes     | Yes     | Yes     | Yes     |
+------------------+---------+---------+---------+---------+
| BREAK/CONTINUE   | Yes     | Yes     | Yes     | Yes (*) |
+------------------+---------+---------+---------+---------+
| REPEAT/UNTIL     | -       | -       | -       | Yes     |
+------------------+---------+---------+---------+---------+
| LOOP/ENDLOOP     | -       | -       | -       | Yes     |
+------------------+---------+---------+---------+---------+
| Compound assign  | -       | -       | -       | Yes     |
+------------------+---------+---------+---------+---------+
| RETURN <expr>    | -       | -       | -       | Yes     |
+------------------+---------+---------+---------+---------+
| DECLARE / PROC / FUNC | -  | -       | Yes     | Yes     |
+------------------+---------+---------+---------+---------+

(*) Labeled BREAK / CONTINUE variants appear with modernization transforms (350+).

Examples
~~~~~~~~

Single-line IF:

.. code-block:: PPL

   IF (X > 10) PRINTLN "High"

Block IF:

.. code-block:: PPL

   IF (User.TimeLeft < 5) THEN
       PRINTLN "Time is low!"
       GOTO WarnLoop
   ELSEIF (User.TimeLeft < 1) THEN
       PRINTLN "Disconnecting soon..."
   ELSE
       PRINTLN "Plenty of time."
   ENDIF

FOR loop:

.. code-block:: PPL

   FOR I = 1 TO 10
       Total += I
   NEXT

REPEAT / UNTIL:

.. code-block:: PPL

   REPEAT
       Line = INPUT()
   UNTIL (Len(Line) = 0)

Compound assignment:

.. code-block:: PPL

   BytesLeft -= ChunkSize
   Flags &= ~FLAG_NEW

Procedure call:

.. code-block:: PPL

   UpdateUserStats(UserId, TRUE)

Label / GOTO (legacy):

.. code-block:: PPL

   :RetryLogin
   IF (Attempts > 3) GOTO Lockout
   PRINT "Password: "
   ...
   GOTO RetryLogin
   :Lockout
   PRINTLN "Too many attempts."

Return with value (350+):

.. code-block:: PPL

   FUNCTION Add(a, b) INTEGER
       RETURN a + b
   ENDFUNC

PPL Functions
-----------------

FTELL (3.20)
~~~~~~~~~~~~

  :PPL:`FUNCTION INTEGER FTELL(INTEGER channel)`

  **Parameters**
    * :PPL:`channel` (INTEGER) - The file channel number (1-8)
  
  **Returns**
    Current file pointer position in bytes (0 if channel not open)
  
  **Description**
    :PPL:`FTELL` returns the current file pointer offset for the specified 
    file channel. If the channel is not open, it will return 0.
    Otherwise it will return the current position in the open file.

  **Example**

    .. code-block:: PPL

        FOPEN 1,"C:\MYFILE.TXT",O_RD,S_DN
        FSEEK 1,10,SEEK_SET
        PRINTLN "Current file offset for MYFILE.TXT is ",FTELL(1)
        FCLOSE 1

OS (3.20)
~~~~~~~~~

  :PPL:`FUNCTION INTEGER OS()`

  **Parameters**
    None
  
  **Returns**
    An integer indicating which operating system/PCBoard version the PPE is running under:
    
    * 0 = Unknown
    * 1 = DOS/Windows
    * 2 = OS/2 (legacy - unused)
    * 3 = Linux
    * 4 = MacOS
  
  **Description**
    :PPL:`OS` returns a value indicating the operating system environment.
    In Icy Board, this currently returns 0 (unknown) as a placeholder for
    compatibility. Legacy PPEs may use this to detect DOS vs OS/2 environments.

  **Example**
    .. code-block:: PPL

        SELECT CASE (OS())
            CASE 0
                PRINTLN "Running on Icy Board or unknown system"
            CASE 1
                PRINTLN "Running DOS version of Icy Board"
            CASE 2
                PRINTLN "Running OS/2 version of Icy Board"
        END SELECT

I2BD (3.20)
~~~~~~~~~~~

  :PPL:`FUNCTION BIGSTR I2BD(INTEGER value)`

  **Parameters**
    * :PPL:`value` – integer to serialize

  **Returns**
    * :PPL:`BIGSTR` – 8 raw bytes representing a “bdreal” (double) form

  **Description**
    Converts a PPL INTEGER into an 8‑byte BASIC double binary image.

  **Example**

    .. code-block:: PPL

       BIGSTR  raw
       INTEGER v

       v   = 12345
       raw = I2BD(v)
       FOPEN 1,"double.bin",O_WR,S_DN
       FWRITE 1,raw,8
       FCLOSE 1

TINKEY (3.20)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING TINKEY(INTEGER ticks)`

  **Parameters**
    * :PPL:`ticks` – Maximum clock ticks to wait (~18 ticks per second).  
      Use 0 to wait indefinitely (implementation–limited upper bound ~4 hours or until carrier loss).

  **Returns**
    * :PPL:`STRING` – Key pressed (special names like UP / DOWN / PGUP) or empty string if timed out

  **Description**
    Waits for user input for up to the specified number of clock ticks.

  **Example**

    .. code-block:: PPL

       STRING resp
       PRINTLN "Press a key (10 second timeout)…"
       resp = TINKEY(180)
       IF (resp = "") THEN
           PRINTLN "Timeout."
       ELSE
           PRINTLN "You pressed: ", resp
       ENDIF

GETDRIVE (3.20)
~~~~~~~~~~~~~~~

  :PPL:`FUNCTION INTEGER GETDRIVE()`

  **Parameters**
    None

  **Returns**
    * :PPL:`INTEGER` – Current “drive number”  
      (A:=0, B:=1, C:=2, …). On non‑DOS systems mapping is virtual.

  **Description**
    Returns the logical drive index. Primarily legacy; on modern platforms the value may be synthesized.

  **Example**

    .. code-block:: PPL

       INTEGER d
       d = GETDRIVE()
       IF (d = 2) PRINTLN "Drive C: is current"

CONFINFO (3.20)
~~~~~~~~~~~~~~~

  :PPL:`FUNCTION <VARIANT> CONFINFO(INTEGER confnum, INTEGER field)`

  **Parameters**
    * :PPL:`confnum` – Conference number
    * :PPL:`field`   – Field selector (see list)

  **Returns**
    Variant type depending on the field (STRING, BOOLEAN, INTEGER, BYTE, DREAL)

  **Description**
    Reads a conference configuration attribute. Field meanings:

  **Valid fields**

+----+-----------+-----------------------------------------------+
| 1  | STRING    | Conference Name                               |
+----+-----------+-----------------------------------------------+
| 2  | BOOLEAN   | Public Conference                             |
+----+-----------+-----------------------------------------------+
| 3  | BOOLEAN   | Auto Rejoin                                   |
+----+-----------+-----------------------------------------------+
| 4  | BOOLEAN   | View Other Users                              |
+----+-----------+-----------------------------------------------+
| 5  | BOOLEAN   | Make Uploads Private                          |
+----+-----------+-----------------------------------------------+
| 6  | BOOLEAN   | Make All Messages Private                     |
+----+-----------+-----------------------------------------------+
| 7  | BOOLEAN   | Echo Mail in Conf                             |
+----+-----------+-----------------------------------------------+
| 8  | INTEGER   | Required Security public                      |
+----+-----------+-----------------------------------------------+
| 9  | INTEGER   | Additional Conference Security                |
+----+-----------+-----------------------------------------------+
| 10 | INTEGER   | Additional Conference Time                    |
+----+-----------+-----------------------------------------------+
| 11 | INTEGER   | Number of Message Blocks                      |
+----+-----------+-----------------------------------------------+
| 12 | STRING    | Name/Loc MSGS File                            |
+----+-----------+-----------------------------------------------+
| 13 | STRING    | User Menu                                     |
+----+-----------+-----------------------------------------------+
| 14 | STRING    | Sysop Menu                                    |
+----+-----------+-----------------------------------------------+
| 15 | STRING    | News File                                     |
+----+-----------+-----------------------------------------------+
| 16 | INTEGER   | Public Upload Sort                            |
+----+-----------+-----------------------------------------------+
| 17 | STRING    | Public Upload DIR file                        |
+----+-----------+-----------------------------------------------+
| 18 | STRING    | Public Upload Location                        |
+----+-----------+-----------------------------------------------+
| 19 | INTEGER   | Private Upload Sort                           |
+----+-----------+-----------------------------------------------+
| 20 | STRING    | Private Upload DIR file                       |
+----+-----------+-----------------------------------------------+
| 21 | STRING    | Private Upload Location                       |
+----+-----------+-----------------------------------------------+
| 22 | STRING    | Doors Menu                                    |
+----+-----------+-----------------------------------------------+
| 23 | STRING    | Doors File                                    |
+----+-----------+-----------------------------------------------+
| 24 | STRING    | Bulletin Menu                                 |
+----+-----------+-----------------------------------------------+
| 25 | STRING    | Bulletin File                                 |
+----+-----------+-----------------------------------------------+
| 26 | STRING    | Script Menu                                   |
+----+-----------+-----------------------------------------------+
| 27 | STRING    | Script File                                   |
+----+-----------+-----------------------------------------------+
| 28 | STRING    | Directories Menu                              |
+----+-----------+-----------------------------------------------+
| 29 | STRING    | Directories File                              |
+----+-----------+-----------------------------------------------+
| 30 | STRING    | Download Paths File                           |
+----+-----------+-----------------------------------------------+
| 31 | BOOLEAN   | Force Echo on All Messages                    |
+----+-----------+-----------------------------------------------+
| 32 | BOOLEAN   | Read Only                                     |
+----+-----------+-----------------------------------------------+
| 33 | BOOLEAN   | Disallow Private Messages                     |
+----+-----------+-----------------------------------------------+
| 34 | INTEGER   | Return Receipt Level                          |
+----+-----------+-----------------------------------------------+
| 35 | BOOLEAN   | Record Origin                                 |
+----+-----------+-----------------------------------------------+
| 36 | BOOLEAN   | Prompt For Routing                            |
+----+-----------+-----------------------------------------------+
| 37 | BOOLEAN   | Allow Aliases                                 |
+----+-----------+-----------------------------------------------+
| 38 | BOOLEAN   | Show INTRO in 'R A' scan                      |
+----+-----------+-----------------------------------------------+
| 39 | INTEGER   | Level to Enter a Message                      |
+----+-----------+-----------------------------------------------+
| 40 | STRING    | Join Password (private)                       |
+----+-----------+-----------------------------------------------+
| 41 | STRING    | INTRO File                                    |
+----+-----------+-----------------------------------------------+
| 42 | STRING    | Attachment Location                           |
+----+-----------+-----------------------------------------------+
| 43 | STRING    | Auto-Register Flags                           |
+----+-----------+-----------------------------------------------+
| 44 | BYTE      | Attachment Save Level                         |
+----+-----------+-----------------------------------------------+
| 45 | BYTE      | Carbon Copy List Limit                        |
+----+-----------+-----------------------------------------------+
| 46 | STRING    | Conf-specific CMD.LST                         |
+----+-----------+-----------------------------------------------+
| 47 | BOOLEAN   | Maintain old MSGS.NDX                         |
+----+-----------+-----------------------------------------------+
| 48 | BOOLEAN   | Allow long (Internet) TO: names               |
+----+-----------+-----------------------------------------------+
| 49 | BYTE      | Carbon List Level                             |
+----+-----------+-----------------------------------------------+
| 50 | BYTE      | NetMail Conference Type                       |
+----+-----------+-----------------------------------------------+
| 51 | INTEGER   | Last Message Exported                         |
+----+-----------+-----------------------------------------------+
| 52 | DREAL     | Charge Per Minute                             |
+----+-----------+-----------------------------------------------+
| 53 | DREAL     | Charge Per Message Read                       |
+----+-----------+-----------------------------------------------+
| 54 | DREAL     | Charge Per Message Written                    |
+----+-----------+-----------------------------------------------+

  **Example**

    .. code-block:: PPL

       IF (CONFINFO(100,50) = 5) PRINTLN "Conference 100 is FIDO type"

  **See Also**
    * CONFINFO (object form – future user data variant)

CONFINFO (Delete Queue Record) (3.20)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

  :PPL:`FUNCTION CONFINFO(INTEGER recnum)`

  **Parameters**
    * :PPL:`recnum` – Queue record number to delete (legacy Fido queue semantics)

  **Returns**
    * None

  **Description**
    Legacy form used to delete Fido queue records. (Retained for script compatibility.)

  **Example**

    .. code-block:: PPL

       CONFINFO(6)  ; delete queue record #6

BS2I / BD2I / I2BS / I2BD See Also
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

  * :PPL:`FILEINF()` for file size/date/time
  * :PPL:`EXIST()` for existence checks

FINDFIRST (3.20)
~~~~~~~~~~~~~~~~

  :PPL:`FUNCTION STRING FINDFIRST(STRING file)`

  **Parameters**
    * :PPL:`file` – Path or pattern (may include wildcards like `*.BAK`)

  **Returns**
    * First matching filename (no path normalization) or empty string if none

  **Description**
    Begins a wildcard (pattern) scan. Use :PPL:`FINDNEXT()` repeatedly to enumerate
    additional matches. Only names are returned; use :PPL:`FILEINF()` for metadata.

  **Example**

    .. code-block:: PPL

       STRING toDelete
       toDelete = FINDFIRST("*.BAK")
       WHILE (toDelete <> "")
           DELETE toDelete
           PRINTLN toDelete, " deleted."
           toDelete = FINDNEXT()
       ENDWHILE

  **See Also**
    * :PPL:`FINDNEXT()`, :PPL:`EXIST()`, :PPL:`FILEINF()`

FINDNEXT (3.20)
~~~~~~~~~~~~~~~

  :PPL:`FUNCTION STRING FINDNEXT()`

  **Parameters**
    * None

  **Returns**
    * Next filename in the active scan or empty string when exhausted

  **Description**
    Continues the enumeration started by :PPL:`FINDFIRST()`. Stops when an empty
    string is returned.

  **Example**

    .. code-block:: PPL

       STRING n
       n = FINDFIRST("*.BAK")
       WHILE (n <> "")
           PRINTLN "Processing ", n
           n = FINDNEXT()
       ENDWHILE

  **See Also**
    * :PPL:`FINDFIRST()`, :PPL:`FILEINF()`, :PPL:`EXIST()`


PPL Statements
--------------

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
    Produces a tone for the specified duration. Introduced to replace the DOS two‑step
    SOUND on / SOUND off sequence (not portable to OS/2 or modern systems) with a single call.

  **Example**

    .. code-block:: PPL

       IF (inputVal <> validVal) SOUNDDELAY 500,18

  **Notes**
    May be a no‑op on non‑emulated systems. Consider providing a visual fallback.

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
    Will not remove non‑empty directories.

  **See Also**
    * :PPL:`MKDIR()`
    * :PPL:`CWD()`


CWD (3.20)
~~~~~~~~~~

  :PPL:`FUNCTION STRING CWD()`

  **Parameters**
    None

  **Returns**
    * :PPL:`STRING` – Current working directory path

  **Description**
    Retrieves the process (or session) current directory.

  **Example**

    .. code-block:: PPL

       PRINTLN "Current working directory = ", CWD()

  **Notes**
    Function (not a statement) but historically documented among statements.

  **See Also**
    * :PPL:`MKDIR()`
    * :PPL:`RMDIR()`


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
      * 4 = Non‑ANSI (plain)
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
    Deleting a non‑existent record has no effect (legacy behavior).

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