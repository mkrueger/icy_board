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
~~~~~~~~~~

1. High compatibility with PCBoard PPEs up to 15.4 (run, decompile, recompile)
2. Safe modernization (UTF-8 source, stricter diagnostics, secure password handling)
3. Progressive evolution (optional newer *language versions* that do not break older scripts unless you opt in)
4. Better tooling (warnings instead of silent miscompiles; IDE support; disassembly view)
5. Eliminate “anti-decompile” era tricks—make maintenance possible again

Vocabulary: Runtime vs Language Version
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

You will see two related version notions:

* **Runtime / PPE format version**: The bytecode / PPE container format (100-400).
* **Language version**: The surface syntax & feature set you target.  
  By default language version = runtime version unless overridden (``--lang-version``).

You can (for example) generate a PPE in runtime format 400 but restrict yourself
to language features of 340 to stay compatible with older boards (where applicable).

Toolchain Overview
~~~~~~~~~~~~~~~~~~

+-----------+------------------------------------------------------------+
| Tool      | Purpose                                                    |
+===========+============================================================+
| `pplc`    | Compile `.pps` (UTF-8 or CP437) into a PPE (PCBoard Exec)  |
| `ppld`    | Decompile an existing PPE back to readable PPL             |
| LSP       | Editor services: outline, hover help, go-to, completion    |
| Disasm    | Optional internal view of low-level instructions           |
+-----------+------------------------------------------------------------+


``ppld`` - The Decompiler
~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: bash

   ppld hello.ppe

Produces ``hello.ppd`` plus (optionally) a reconstructed control structure
instead of flat GOTO spaghetti. Use ``-d`` to view a disassembly and ``-r`` for
a minimal (raw) form.

Encoding & Character Set
~~~~~~~~~~~~~~~~~~~~~~~~

* **Preferred input**: UTF-8 (modern editors)
* **Legacy**: Original DOS sources were CP437. Use ``--cp437`` if auto-detection fails.
* Compiler outputs CP437 in the PPE so legacy display semantics match PCBoard expectations.
* You may convert existing PPE data files to UTF-8 with: ``icbsetup ppe-convert <PATH>`` (make backups first).

Key Differences vs Legacy PPLC (Summary)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

(See the detailed “PPL differences” section in ``ppl.md`` for the full list.)

* Reserved words: a larger set is now treated as keywords (``IF``, ``FOR``, ``CONTINUE``, etc.) to prevent ambiguous parses.
* Stricter: mismatched function return declarations are **errors** instead of silently ignored.
* Additional identifier support (e.g. UTF-8 cases, the Euro sign).
* Cleaner loop constructs / assignment operators in higher language versions.
* DECLARE blocks no longer required in newer versions (&gt;= 350).
* More (and safer) warnings for suspicious code; treat warnings seriously when porting.

Evolution by Language Version
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

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
~~~~~~~~~~~~~~~~~~~~

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
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

* Scalars: Integer, Unsigned, Byte / Word, Boolean, Float, Double, Money, Date, Time
* Strings: Normal and “BigStr” (large string buffers) 
* Arrays: 1–3 dimensional (indexed, zero-based internally)
* Password values (internally hashed if Argon2 / BCrypt storage is enabled)
* (Planned / partial in 400) Domain objects: Conference, MessageArea, FileArea, with member-like accessors or function wrappers.

Security & Safety Notes
~~~~~~~~~~~~~~~~~~~~~~~

* Passwords: Hashing (Argon2 / BCrypt) is enforced by configuration; scripts that attempt to transform (uppercase/lowercase) hashed values should expect no-ops.
* Avoid relying on internal hashes—display calls typically mask them.
* VM isolates runtime; catastrophic host crashes from buggy PPE logic are far harder now (memory safety from Rust).

Migration Workflow (Legacy PPE → Modern PPL)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

1. **Decompile** legacy ``FOO.PPE`` → ``FOO.PPS`` with ``ppld``.
2. **Review warnings** when recompiling with ``pplc``; fix shadowed variables, questionable assignments, or deprecated idioms.
3. **Decide language version**: If you need pure compatibility, stick to 340. If modern loops / returns help clarity, switch to 350.
4. **Run under Icy Board**; validate interactive paths (menus, door launching, display).
5. **Iterate**: Use LSP tooling for rename, find references, and incremental modernization.

Disassembly for Learning
~~~~~~~~~~~~~~~~~~~~~~~~

Use:

.. code-block:: bash

   pplc myscript.pps --disassemble
   # or
   ppld legacy.ppe --disassemble

This produces a low-level opcode view. Helpful for verifying optimizer or diagnosing control-flow reconstruction.

Quick Reference Cheat Card
~~~~~~~~~~~~~~~~~~~~~~~~~~

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

