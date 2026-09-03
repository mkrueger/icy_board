The Language
============

PPL 4.0 is what the IcyBoard compiler targets by default. It is a source-level
superset of PCBoard 15.4 PPL, and every language addition sits behind a version
number so an old source keeps compiling as an old source. Calls tied directly
to DOS hardware and a few operating-system details necessarily behave
differently; :doc:`compatibility` lists them.

This page describes what the language gained after PCBoard. The reference pages
list what a program may call: :doc:`data_types`, :doc:`constants`,
:doc:`functions` and :doc:`statements`.

Two version numbers
-------------------

A PPE has a *runtime* version and a source has a *language* version. They are
set independently, because wanting new syntax and wanting a file an old board can
load are two different wishes.

.. csv-table:: Runtime and language versions
    :header: "Version", "Command line", "ppl.toml", "Environment", "What it controls"
    :widths: 12, 20, 24, 20, 24

    "Runtime", "``--runtime``", "``[package] runtime``", "", "The PPE format written to disk"
    "Language", "``--lang-version``", "``[compiler] language_version``", "``PPL_LANG_VERSION``", "Which syntax and built-ins are accepted"

The runtime defaults to 400, the newest format. The language defaults to the
runtime version up to 400, so the default pair is runtime 400 and language 400.
A format-only runtime bump therefore does not invent a new language version. The
source's ``;$LANGVERSION`` wins over the command line, the command line wins over
``ppl.toml``, and the manifest wins over ``PPL_LANG_VERSION``. The environment
variable is therefore a personal default for loose sources rather than a way to
silently change a project.

The language server reads the same sources, so the editor judges a file the way
``pplc`` will compile it. It has no command line, so for it ``;$LANGVERSION``
wins over ``ppl.toml``, which wins over ``PPL_LANG_VERSION``. A language server
inherits the environment of the editor that started it, which is not always the
shell's.

To write a PPE an original PCBoard can load, ask for its format::

    pplc --runtime 340 myscript.pps

The language then defaults to 340 as well, so the compiler refuses the syntax
that board could not run rather than letting it fail later.

Anything below is grouped by the language version that introduced it. A feature
listed under 3.50 is available at 3.50 *and* 400; a feature listed under 400
needs ``--lang-version 400``.

Language version 3.50
---------------------

3.50 is the "quality of life" version. It adds no new PPE format, so a 3.50
source can still be compiled down to an older runtime as long as it does not call
newer built-ins.

DECLARE is optional
~~~~~~~~~~~~~~~~~~~

``DECLARE FUNCTION`` / ``DECLARE PROCEDURE`` before the implementation is no
longer required; the compiler reads every signature in the file before it
compiles the code, so a routine may be called before the file gets to it.
Existing forward declarations are still accepted, and a parameter count or return
type that disagrees between a declaration and its implementation is an error.

Variable initializers
~~~~~~~~~~~~~~~~~~~~~

::

    INTEGER count = 0
    STRING  greeting = "Hello"

An array is initialized with a brace list::

    INTEGER values = { 1, 2, 3 }

which is shorthand for::

    INTEGER values(3)
    values(0) = 1
    values(1) = 2
    values(2) = 3

The brace list also decides the size, so the dimension is not written out.

Bracket indexing
~~~~~~~~~~~~~~~~

``[`` and ``]`` index an array::

    INTEGER values(10)
    values[0] = 5
    PRINTLN values[0]

Parenthesis indexing still works and still means the same thing. Brackets exist
because ``values(0)`` and a call to a function named ``values`` are written
identically, which the old language simply lived with. Brackets say which one is
meant, and they are the recommended form in new code.

Compound assignment
~~~~~~~~~~~~~~~~~~~

::

    count += 1

Available for ``+`` ``-`` ``*`` ``/`` ``%`` ``&`` ``|``. ``count += 1`` is
exactly ``count = count + 1``; there is no separate opcode.

REPEAT ... UNTIL
~~~~~~~~~~~~~~~~

A loop with the test at the bottom, so the body always runs once::

    INTEGER n = 0
    REPEAT
        n += 1
    UNTIL n >= 3

LOOP ... ENDLOOP
~~~~~~~~~~~~~~~~

A loop with no test at all, left with ``BREAK``::

    LOOP
        n *= 2
        IF n > 10 BREAK
    ENDLOOP

RETURN with a value
~~~~~~~~~~~~~~~~~~~

Inside a function, ``RETURN expr`` both sets the result and returns::

    FUNCTION Total(INTEGER v) INTEGER
        RETURN v + 1
    ENDFUNC

which is the same as the old two-step form::

    FUNCTION Total(INTEGER v) INTEGER
        Total = v + 1
        RETURN
    ENDFUNC

``RETURN expr`` in a procedure is an error.

Parentheses are optional on IF and WHILE
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

::

    IF A <> B THEN
        ...
    ENDIF

    WHILE IsValid() PRINTLN "Success."

The old ``IF (A <> B) THEN`` still parses.

QUIT and LOOP are no longer aliases
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

At language version 350 and above, ``QUIT`` is no longer a synonym for ``BREAK``
and ``LOOP`` is no longer a synonym for ``CONTINUE`` - ``LOOP`` is now a loop
keyword of its own. Sources that used the aliases need the modern spelling. They
were rare in practice.

Functions and procedures as parameters
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

A procedure or function can be declared as a parameter::

    PROCEDURE PrintHello(PROCEDURE f())
        PRINT "Hello "
        f()
    ENDPROC

The parameter is callable inside the body. Passing ``PrintHello(Hello)`` checks
the complete signature: routine kind, argument types and dimensions, ``VAR``
flags and the return type of a function. A routine parameter can be passed on to
another routine. Outside such an argument position a bare routine name is still
an error.

.. note::
   Routine references need runtime 400, because 4.00 adds the bytecode marker
   that tells a routine value from a call to that routine.

CONST
~~~~~

A name for a value the compiler works out::

    CONST INTEGER MaxTries = 3
    CONST STRING  Greeting = "Welcome"
    CONST INTEGER Warning  = MaxTries - 1

A constant is written where a variable would be, so it may stand at the top of a
program or at the top of a routine, where it belongs to that routine. Its value
may be built from literals and from constants declared before it, and it is
converted to the type it was declared with, the same way an assignment would
convert it. A constant declared with an enum type names one of that enum's
members and keeps the enum as its type.

Because the value takes the place of the name while compiling, a constant costs
nothing: the PPE is byte for byte the one the value written out by hand would
produce, whatever runtime it is built for. That also means a decompiled PPE shows
the value, never the name, and that a constant cannot be passed to a ``VAR``
parameter - there is no variable to write back to.

Writing to a constant is an error. A constant, parameter and variable may not
share a name in the same scope. A local declaration may use the name of a global
constant or variable; inside the routine the local declaration wins. ``CONST``
is a keyword from 350 on, so a 3.40 source may still have a variable called
``const``.

``;$DEFINE`` looks similar but is a different thing: it substitutes text before
the language is even read, it carries no type, and it works whatever version is
set. Reach for ``CONST`` unless the value has to steer the preprocessor.

ENUM ... ENDENUM
~~~~~~~~~~~~~~~~

An enum gives related integer values a type and a namespace::

    ENUM Color
        Red
        Green = 5
        Blue
    ENDENUM

    Color favorite = Color.Green

Members without a value start at zero and continue from the preceding member, so
``Red`` is 0 and ``Blue`` is 6 above. An explicit value must be an integer
constant expression. Members are always qualified: ``Color.Green`` is a value,
``Green`` on its own is not.

Enums are nominal. A ``Color`` may be assigned and compared only with another
``Color``; an integer or a member of a different enum is an error. Equality and
inequality are supported, arithmetic is not. Enum variables, arrays, routine
parameters and return values, and record fields all work with the same rule. A
``FOR`` may still count over an enum, because the loop writes its own comparison
and step; its start and end value have to be of the enum's type.

The type exists while compiling only. Its storage in the PPE is ``INTEGER``, and
``Color.Green`` becomes 5, so no new runtime or PPE format is needed. A
decompiler therefore recovers an ``INTEGER`` and a number, not the enum name or
member. Enums are not bitflags; use typed ``CONST`` values when named masks are
needed.

What 3.50 breaks
~~~~~~~~~~~~~~~~

* ``QUIT`` and ``LOOP`` are no longer aliases for ``BREAK`` and ``CONTINUE``.
* ``.`` is a token, so it can no longer appear in an identifier.
* ``CONST`` is a keyword, so a 3.40 source may still have a variable called
  ``const`` while a 3.50 source may not.
* ``ENUM`` and ``ENDENUM`` are keywords, so a 3.40 source may still use those
  names as identifiers.

Language version 4.00
---------------------

400 is where the language stops being bound by what PCBoard 15.4 could express.
A PPE built at runtime 400 will not load on an original PCBoard.

Parentheses, brackets and braces
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

400 gives each bracket kind one job:

=========  ====================================================
Written    Used for
=========  ====================================================
``( )``    Grouping, call arguments and array declarations
``[ ]``    Indexing
``{ }``    Array initializers and record literals
=========  ====================================================

Indexing with ``( )`` is still accepted for compatibility, but new code should
index with ``[ ]``.

Board objects
~~~~~~~~~~~~~

400 puts the ``.`` operator, which 350 already uses for enum members, to a second
use: objects that describe the board itself. The point is that a PPE no longer
has to parse the board's config files to find out what is on it.

Object lifetime and mutability
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

A snapshot does not track later changes, a live view reads current state, a
resource/controller remains active until explicit release or PPE cleanup, and a
value is copied like an ordinary PPL value.

.. csv-table::
   :header: "Type", "Lifetime", "Mutability"
   :widths: 18, 42, 40

   "``BOARD``", "First-access snapshot for the PPE run", "Read-only"
   "``CONFERENCE``, ``DIRECTORY``, ``DOOR``", "Configured-entry snapshot", "Read-only"
   "``AREA``", "Configured-entry snapshot; message methods perform live I/O", "Read-only"
   "``SESSION``", "Live active-call view", "Read-only"
   "``USER``", "Live caller view from ``Session``; snapshot from ``Board``", "Caller view selectively writable; board snapshot read-only"
   "``CONTACT``", "Value record in a snapshot array", "Local record fields writable"
   "``MSG``", "Header snapshot; ``Text()`` loads the body", "Read-only"
   "``TERMINAL``", "Live caller-terminal root", "Methods change terminal state"
   "``TERMINFO``", "Connection-time snapshot", "Read-only"
   "``TERMINPUT``", "PPE-owned input controller", "Mutable through methods; released at cleanup"
   "``EVENT``", "Polled/waited value", "Read-only"
   "``GFX``", "Caller graphics-session controller", "Mutable through methods"
   "``SURFACE``", "PPE-owned graphics resource", "Mutable until ``Free()`` or cleanup"
   "``AUDIO``", "PPE-owned channel resource", "Mutable until ``Free()`` or cleanup"
   "``MARGINS``, ``PALETTE``", "Live terminal-state controllers", "Mutable through methods"
   "``MACROS``", "PPE-owned terminal macro controller", "Mutable through methods; definitions removed at cleanup"
   "``HTTP``", "Stateless factory/root", "Static methods only"
   "``HTTPREQUEST``", "Shared request state", "Mutable through ``SetHeader()``, ``SetText()`` and ``SetForm()``"
   "``HTTPRESPONSE``", "Completed-request result snapshot", "Read-only; ``Save()`` performs output"
   "``REGEX``", "Compiled-pattern value", "Read-only"
   "``REGEXMATCH``", "Match-result value", "Read-only"
   "``ERROR``", "Published-error snapshot", "Read-only; ``Error.Clear()`` changes VM state"

EVENT field applicability
^^^^^^^^^^^^^^^^^^^^^^^^^

Check ``Event.Kind`` before reading a kind-specific field. A field not listed
for that kind returns its neutral fallback: ``0``, ``FALSE``, an empty string,
a ``None`` enum member, or ``-1`` for ``Channel``.

.. csv-table::
   :header: "Fields", "Applicable ``EventKind``", "Meaning"
   :widths: 40, 25, 35

   "``Kind``, ``Time``", "all kinds", "Discriminator and monotonic connection time"
   "``Code``, ``Text``", "``Key``", "Translated Unicode/named key code and text"
   "``ScanCode``", "``KeyEdge``", "Physical key code"
   "``Pressed``", "``Key``, ``KeyEdge``", "Press/release state; translated keys are presses"
   "``Repeated``", "``KeyEdge``", "Physical-key repeat flag"
   "``Action``, ``Button``, ``X``, ``Y``, ``Pixels``", "``Mouse``", "Typed mouse action/button and position mode"
   "``WheelX``, ``WheelY``, ``LeftDown``, ``MiddleDown``, ``RightDown``", "``Mouse``", "Wheel delta and held buttons"
   "``Shift``, ``Alt``, ``Ctrl``, ``Meta``", "``Key``, ``Mouse``", "Modifiers supplied by translated keys or mouse reports"
   "``Dropped``", "``Overflow``", "Number of queue entries lost"
   "``Channel``", "``Audio``", "Finished sound channel"

.. code-block:: PPL

    CONFERENCE conf = Session.Conference

    IF conf.HasAccess() PRINTLN conf.Name

``Board.Conferences[index]`` returns a read-only ``CONFERENCE`` snapshot, and
``Session.Conference`` the one the caller is in. An index no conference has
returns an empty conference rather than failing, so its properties can still be
read.

**CONFERENCE**

===================  ================  ==================================================
Member               Type              Description
===================  ================  ==================================================
``Name``             ``STRING``        Conference name
``Number``           ``INTEGER``       The number it was fetched under
``Valid``            ``BOOLEAN``       Whether the requested conference exists
``IsPublic``         ``BOOLEAN``       Whether the conference is configured as public
``IsReadOnly``       ``BOOLEAN``       Whether messages may only be read
``AllowAliases``     ``BOOLEAN``       Whether a caller may post under an alias
``EchoMail``         ``BOOLEAN``       Whether mail written here is echoed
``AutoRejoin``       ``BOOLEAN``       Whether a caller is rejoined here on the next call
``PrivateUploads``   ``BOOLEAN``       Whether uploads go to the private area
``Password``         ``PASSWORD``      The password needed to join
``Directories``      ``DIRECTORY[]``   The file directories of the conference
``Areas``            ``AREA[]``        The message areas of the conference
``Doors``            ``DOOR[]``        The doors of the conference
``HasAccess()``      ``BOOLEAN``       Whether the current caller can join it
``CanPost()``        ``BOOLEAN``       Whether the current caller may write a message
``CanAttach()``      ``BOOLEAN``       Whether the current caller may attach a file
===================  ================  ==================================================

**AREA**

===================  ================  ==================================================
Member               Type              Description
===================  ================  ==================================================
``Name``             ``STRING``        Area name
``Number``           ``INTEGER``       The number it was fetched under
``Valid``            ``BOOLEAN``       Whether the requested area exists
``IsReadOnly``       ``BOOLEAN``       Whether messages may only be read
``AllowAliases``     ``BOOLEAN``       Whether a caller may post under an alias
``QwkName``          ``STRING``        The name this area carries in a QWK packet
``EchoTag``          ``STRING``        The FTN tag, empty when the area is local
``HasAccess()``      ``BOOLEAN``       Whether the current caller may list it
``CanEnter()``       ``BOOLEAN``       Whether the current caller may join it
``CanAttach()``      ``BOOLEAN``       Whether the current caller may save an attachment
``LowMsg()``         ``LONG``          The lowest message number, zero when there is none
``HighMsg()``        ``LONG``          The highest message number, zero when there is none
``Read(number)``     ``MSG``           The message with that number
``Find(f, t[, s])``  ``MSG``           First message at or after ``s`` whose field ``f`` contains ``t``
===================  ================  ==================================================

**DIRECTORY**

===================  ================  ==================================================
Member               Type              Description
===================  ================  ==================================================
``Name``             ``STRING``        Directory name
``Number``           ``INTEGER``       The number it was fetched under
``Valid``            ``BOOLEAN``       Whether the requested directory exists
``Path``             ``STRING``        Where the files are kept
``IsFree``           ``BOOLEAN``       Whether downloads here cost no time or bytes
``HasNewFiles``      ``BOOLEAN``       Whether the directory is flagged as having new files
``Password``         ``PASSWORD``      The password needed to reach it
``HasAccess()``      ``BOOLEAN``       Whether the current caller may list it
``CanDownload()``    ``BOOLEAN``       Whether the current caller may download from it
===================  ================  ==================================================

**DOOR**

===================  ================  ==================================================
Member               Type              Description
===================  ================  ==================================================
``Name``             ``STRING``        Door name
``Number``           ``INTEGER``       The number it was fetched under
``Valid``            ``BOOLEAN``       Whether the requested door exists
``Description``      ``STRING``        Door description
``Path``             ``STRING``        What the door runs
``Password``         ``PASSWORD``      The password needed to open it
``HasAccess()``      ``BOOLEAN``       Whether the current caller can open it
===================  ================  ==================================================

``HighMsg()`` reads the message base to answer, so it is a call rather than a
property.

Messages
~~~~~~~~

``AREA.Read(number)`` answers with a ``MSG``, the message that area holds under
that number::

    AREA area = Session.Area
    MSG msg = area.Read(1)

    IF msg.Valid THEN
        PRINTLN msg.Number, "  ", msg.From, " -> ", msg.To
        PRINTLN msg.Subject, "  ", msg.Date, " ", msg.Time
        PRINTLN msg.Text()
    ENDIF

===================  ================  ==================================================
Member               Type              Description
===================  ================  ==================================================
``Valid``            ``BOOLEAN``       Whether the area has that message
``Number``           ``LONG``          The number it was read under
``From``             ``STRING``        Who wrote it
``To``               ``STRING``        Who it is for
``Subject``          ``STRING``        What it is about
``Date``             ``DATE``          When it was written
``Time``             ``TIME``          When it was written
``ReplyTo``          ``LONG``          The message it answers, zero when it answers none
``Status``           ``STRING``        The one character ``HDR_STATUS`` reports
``IsPrivate``        ``BOOLEAN``       Whether it is private
``IsRead``           ``BOOLEAN``       Whether it has been read
``IsDeleted``        ``BOOLEAN``       Whether it is killed
``IsEcho``           ``BOOLEAN``       Whether it is echoed
``NeedsPassword``    ``BOOLEAN``       Whether reading it takes a password
``Size``             ``LONG``          How many bytes the body holds
``Text()``           ``STRING``        The body
===================  ================  ==================================================

A message is addressed by its number, not by its position. A message base is
sparse - numbering starts at ``LowMsg()`` and a deleted message leaves its
number behind - so a walk counts over the range and asks each one whether it is
there::

    LONG n

    FOR n = area.LowMsg() TO area.HighMsg()
        MSG msg = area.Read(n)
        IF !msg.Valid CONTINUE
        PRINTLN msg.Number, " ", msg.From, " ", msg.Subject
    NEXT

That is also why messages are not a collection: ``[ ]`` indexes a position
everywhere else in the language, and a message number is not one.

Message numbers and body sizes are ``LONG``. JAM counts them in 32 unsigned
bits, which all fit in a signed 64-bit value, and ordinary integer literals can
be added or compared without narrowing. A number outside JAM's range is one no
message has, so ``Read()`` answers an invalid ``MSG`` rather than wrapping.

``LONG`` and ``ULONG`` are signed and unsigned 64-bit integers in language
4.00. Before 4.00, ``LONG`` was a synonym for the 32-bit ``INTEGER``, and
``ToLong()`` therefore performed the same conversion as ``ToInteger()``. In
4.00 ``ToLong()`` returns the new 64-bit type; ``ToULong()`` returns ``ULONG``.
The language server's **Upgrade file to language version 400** source action
rewrites old ``ToLong(value)`` calls to ``ToInteger(value)`` to preserve their
32-bit behavior.

The body stays in the base until ``Text()`` asks for it, which is why it is a
call. A listing that only prints headers never pays for a body.

A message number outside the base, a deleted message and an empty slot are
ordinary lookup misses. ``Read()`` answers an invalid ``MSG``, ``Text()``
answers an empty string and ``Error.Last().OK`` remains true. Running off the
end of ``Find()`` works the same way.

An unreadable base is an operational failure. ``Read()``, ``Find()``,
``LowMsg()``, ``HighMsg()`` and ``Text()`` keep their normal invalid, zero or
empty return value and report ``ErrKind.Msg`` as well: ``ErrCode.Io`` for a
filesystem failure and ``ErrCode.Format`` for corrupt JAM data. These failures
also enter an ``ON ERROR`` handler. An invalid ``MsgField`` reports
``ErrCode.Invalid``.

An area is read through one open message base rather than opening it again for
every message. The base is opened when a PPE first reads from that area and kept
until it reads from another one or the PPE ends. A message written after it was
opened is still found: writing through ``MESSAGE``, ``SETMSGHDR``, ``KILLMSG``
or ``MOVEMSG`` takes the base again, and a number past the end is looked up once
more before it is reported missing. ``LOMSGNUM()`` and ``HIMSGNUM()`` open the
base on every call, so they remain the way to watch a base another node is
writing to.

``Find`` is ``SCANMSGHDR`` with a type instead of a field number. It matches
without regard to case, anywhere in the field, and answers an invalid ``MSG``
when nothing matches. The ``start`` argument walks on to the next match::

    MSG hit = area.Find(MsgField.To, "STAN")

    WHILE hit.Valid DO
        PRINTLN hit.Number, " ", hit.Subject
        hit = area.Find(MsgField.To, "STAN", hit.Number + 1)
    ENDWHILE

``MsgField`` is ``To``, ``From`` or ``Subject``; the values are the matching
``HDR_*`` constants.

A ``MSG`` is a read-only snapshot. ``GETMSGHDR``, ``SETMSGHDR``, ``SCANMSGHDR``
and the ``MESSAGE`` statement are unchanged, and writing a message is still
theirs.

.. note::
   The type is called ``MSG`` rather than ``MESSAGE`` because ``MESSAGE`` has
   been a statement since PPL 1.00 and keeps that meaning.

A password is always of the runtime-only ``PASSWORD`` type. It may be compared
against a string, but printing or converting one yields ``******`` rather than
the secret, so a listing can say *that* a password is needed without saying
which one it is.

Walking a conference:

.. code-block:: PPL

    CONFERENCE conf = Session.Conference
    DOOR item

    FOREACH item IN conf.Doors
        IF item.HasAccess() PRINTLN item.Name
    ENDFOREACH

``CONFERENCE``, ``DOOR``, ``AREA`` and ``DIRECTORY`` are resolved wherever a type
name is expected, so a variable cannot be called ``door`` or ``area``. The names
are compared without regard to case, so this holds for a record type a program
declares too: ``Point point`` leaves ``point`` ambiguous.

These objects are read-only snapshots, so assigning to a member -
``conf.Name = "x"`` - is rejected. What a member answers may be asked again, so
``conf.Doors[0].Name`` reads in one go.

Overloaded built-ins
~~~~~~~~~~~~~~~~~~~~

A built-in can now have more than one signature, chosen by argument count.
``Len`` is the example: ``Len(str)`` is the length of a string, as it always was,
while ``Len(array, dim)`` is the length of one dimension of an array. ``Rgb`` is
the other one - ``Rgb(r, g, b)`` and ``Rgb(r, g, b, a)``. Old code keeps working
unchanged, because a call that named the old form still names it.

String members
~~~~~~~~~~~~~~

Language 400 exposes common operations directly on ``STRING`` values, which are
not length-limited. It has its own PPE type ID, separate from classic ``STRING``
and ``BIGSTR``. ``BIGSTR`` remains a deprecated legacy type limited to 2048
Unicode characters.
Positions in this member API are zero-based Unicode character positions and
``-1`` means no match. The classic ``INSTR`` and ``INSTRR`` functions remain
1-based and return zero when no match is found:

.. code-block:: PPL

    STRING text = "  one,two,two  "
    PRINTLN text.Find("two")
    PRINTLN text.Find("two", 7)
    PRINTLN text.FindLast("two")
    PRINTLN text.Contains("one")
    PRINTLN text.Count("two")
    PRINTLN text.Trim().ToUpper().Replace("TWO", "THREE")

Scalar strings support zero-based Unicode character indexing. ``text[0]``
returns the first character as a ``STRING``; a negative or out-of-range index
returns an empty string. String arrays keep their normal array semantics.
Indexing may be chained, so ``words[0][0]`` reads the first character of the
first string in an array.

The instance members are ``Len()``,
``Find(search [, start [, comparison]])``,
``FindLast(search [, start [, comparison]])``,
``Contains(search [, comparison])``, ``StartsWith(prefix [, comparison])``,
``EndsWith(suffix [, comparison])``, ``Count(search [, comparison])``,
``Equals(other [, comparison])``, ``Replace(search, replacement)``,
``Substring(start, length)``, ``Left(count)``, ``Right(count)``,
``Trim([characters])``, ``TrimStart([characters])``,
``TrimEnd([characters])``, ``ToUpper()`` and ``ToLower()``,
``PadLeft(width [, char])``, ``PadRight(width [, char])``,
``Remove(start, length)``, ``Insert(index, value)``, ``Reverse()``,
``ToInt([base])``, ``ToMixedCase()`` and ``StripATX()``. ``Substring`` is zero-based,
unlike the 1-based classic ``MID``; ``Left`` and ``Right`` mirror the classic
functions. ``Remove`` and ``Insert`` use the same zero-based positions.
``PadLeft``/``PadRight`` pad with a space unless a single-character ``char``
is given, and leave the string unchanged if it is already ``width`` or longer.
``ToInt`` is the member form of the classic ``S2I``, base 10 by default, base
2..=36 otherwise; an invalid base or empty string returns 0. Transformations
return ``STRING``; a language 400 ``STRING`` has no length limit, so chaining
does not truncate.

``StringComparison.Ordinal`` is the default. Pass
``StringComparison.OrdinalIgnoreCase`` as the last argument for Unicode-aware,
case-insensitive searching or equality.

The type name carries aggregation helpers:

.. code-block:: PPL

    STRING parts[] = "a,,b,".Split(",")
    PRINTLN STRING.Join(parts, "|")
    PRINTLN STRING.Repeat("-", 40)
    parts = STRING.Split("one:two:three:four", ":", 3)

``Split`` preserves empty elements and returns a dynamic ``STRING[]``. With a
positive limit, the unsplit remainder is the last element; zero means unlimited.
An empty separator or negative limit returns an empty array and reports
``ErrKind.String`` with ``ErrCode.Invalid``. The result may be assigned, indexed,
queried with ``Len()`` or consumed directly by ``FOREACH``.

Binary data
~~~~~~~~~~~

``BYTES`` stores compact binary data without the per-element overhead of a
``BYTE[]`` array. It is a scalar binary value rather than a general-purpose
array, so use ``Len()`` for its byte count and conversion members for text::

    BYTES raw = Bytes.FromBase64("AP8=")
    PRINTLN raw.Len()          ; 2
    PRINTLN raw.ToHex()        ; 00FF
    PRINTLN raw.ToBase64()     ; AP8=

``FromBase64`` accepts padded and unpadded Base64 and ignores ASCII whitespace,
which permits MIME-wrapped input. Invalid input returns empty ``BYTES`` and
reports ``ErrKind.String`` with ``ErrCode.Format``. ``ToString()`` performs
strict UTF-8 decoding and reports the same error for invalid text; it does not
guess a legacy code page.

``GetChecksum(algorithm)`` returns the digest as binary ``BYTES``. The
``Checksum`` enum offers ``CRC32`` (4 bytes in network byte order), ``MD5``
(16 bytes) and ``SHA256`` (32 bytes). Use ``ToHex()`` for the usual uppercase
hexadecimal representation::

    STRING digest = raw.GetChecksum(Checksum.SHA256).ToHex()

An invalid checksum value returns empty ``BYTES`` and reports
``ErrCode.Invalid``. Successful binary and conversion operations clear an older
``Error.Last()`` result.

Regular expressions
~~~~~~~~~~~~~~~~~~~

``REGEX`` compiles a pattern once and reuses it for matching, captures,
replacement and splitting:

.. code-block:: PPL

    REGEX parser = REGEX.Compile("(?P<name>\w+):(?P<value>\d+)")
    REGEXMATCH found = parser.Find("score:120")
    IF found.Success THEN
        PRINTLN found.NamedGroup("name"), " = ", found.NamedGroup("value")
    ENDIF

The static members are ``Compile(pattern [, options])``, ``Escape(text)`` and
``IsValid(pattern [, options])``. Instances provide ``Valid``, ``Pattern``,
``IsMatch(text [, start])``, ``Find(text [, start])``,
``FindAll(text [, start [, limit]])``, ``Replace(text, replacement [, limit])``
and ``Split(text [, limit])``, which returns a dynamic ``STRING[]``.

``RegexOptions`` contains ``None``, ``IgnoreCase``, ``MultiLine``,
``DotMatchesNewLine``, ``IgnoreWhitespace``, ``SwapGreed`` and ``Ascii``.
Flags may be combined with ``|``. Matching is Unicode-aware unless ``Ascii`` is
selected. Positions, collections and capture groups are zero-based. A missing
match or unmatched capture has start position ``-1``. Group zero is the
complete match.

``REGEXMATCH`` exposes ``Success``, ``Value``, ``Start``, ``Length``,
``GroupCount``, numbered ``Group``, ``GroupMatched``, ``GroupStart`` and
``GroupLength`` accessors, and corresponding ``NamedGroup`` methods.
``FindAll`` returns a dynamic ``REGEXMATCH[]``.

Replacement strings expand ``$1`` and ``$name``. Zero limits mean unlimited;
negative limits report ``ErrKind.Regex`` with ``ErrCode.Invalid``. ``Split``
preserves empty fields and returns a dynamic ``STRING[]``. Match collections
are limited to 100,000 items and replacement output to 16 MiB.

The engine guarantees linear-time matching and deliberately omits look-around
and backreferences. Unicode case-insensitive matching does not perform
multi-character folds such as ``ß`` to ``SS``. Invalid patterns return an
invalid ``REGEX`` and report through ``Error.Last()``.

Records
~~~~~~~

A program can declare its own record types:

.. code-block:: PPL

    TYPE Employee
        STRING  Name
        INTEGER Age, Level
    ENDTYPE

    Employee e

    e.Name = "Sysop"
    e.Age  = 42
    PRINTLN e.Name, " ", e.Age

``END TYPE`` may be written with a space, the way ``END SELECT`` may. Fields are
declared like variables, several to a line, and the type name is then usable
anywhere a built-in type name is.

A field is read and written with ``.``, and takes the type it was declared with,
so a value assigned to it is converted the same way an assignment to a variable
of that type would be. Compound assignment works too::

    e.Age += 1

A record starts out with the empty value of each of its fields, and each variable
of a record type has fields of its own. A record is a value, not a reference: two
variables of the same type do not share anything. A record travels into a routine
and back out of a function like any other value, and a ``VAR`` parameter writes
back.

A field may be a record itself, as long as its type was declared first:

.. code-block:: PPL

    TYPE Address
        STRING Town
    ENDTYPE
    TYPE Member
        Address Home
    ENDTYPE

    Member m
    m.Home.Town = "Kiel"

An array may hold records, including more than one dimension. Every element has
fields of its own::

    Member members(10)
    members[0].Home.Town = "Kiel"
    members[1].Home.Town = "Hamburg"

A record field may itself be a one-, two- or three-dimensional array. Its bounds
are part of the record type and are the same for every value, so neither
``REDIM record.Values, ...`` nor ``record.Values.Redim(...)`` is allowed.
It otherwise has the read-only array surface: ``record.Values.Len(dim)`` reports
the number of elements in a dimension and ``FOREACH value IN record.Values`` walks every element. A whole field
may be copied from another field only when element type, rank and all bounds
match; use an index whenever a scalar value is required.

Record literals
~~~~~~~~~~~~~~~

A named record literal creates a value without temporary field assignments.
Fields may appear in any order; omitted fields keep their empty value:

.. code-block:: PPL

    Point origin = Point { X = 0, Y = 0 }
    Point vertical = Point { Y = 10 }
    RETURN Point { X = source.X + 1, Y = source.Y }

Unknown and duplicate fields are errors. A field holding another record requires
the exact nominal type.

.. note::
   Records need runtime 400, which is the first format with a type table. Record
   literals need it too; the PPE stores type and field ids rather than their
   source names.

Rules the compiler enforces
~~~~~~~~~~~~~~~~~~~~~~~~~~~

* A type needs at least one field, and field names are unique within it.
* A type cannot contain a field of its own type, and can only name types that
  were declared before it, so a record cannot end up containing itself.
* Board objects such as ``CONFERENCE`` cannot be fields. They are runtime
  snapshots, not values with record copy and equality semantics.
* A type cannot reuse the name of a built-in or of a board object.
* A program may declare 156 types; ids 100-255 are reserved for them. A type may
  hold 255 fields, because the PPE stores the count in a single byte.
* A field may be a one-, two- or three-dimensional array, including an array of
    a previously declared record. Its dimensions are part of the runtime-400 type
    table and cannot be changed with ``REDIM``.
* Custom types are nominal: two separately declared records are different types
  even when their fields happen to match.
* Equality compares two records of the same type by their fields, including the
    contents of array-valued fields. Whole arrays of records cannot be compared;
    index them first.

All ``TYPE`` declarations in a package are collected before its source files are
parsed, so ``main.pps`` may use a type declared in another file. Record fields
still follow declaration order.

HTTP objects
~~~~~~~~~~~~

Runtime 400 exposes outbound HTTP through typed objects. Public HTTP and HTTPS work by
default; private and special-use addresses remain blocked. The sysop may
optionally disable access or restrict it to an exact origin allowlist::

    HttpResponse response = Http.Get("https://api.example.com/status")
    IF NOT response.Valid THEN
        PRINTLN Error.Last().Message
        RETURN
    ENDIF
    PRINTLN response.Status, " ", response.OK
    PRINTLN response.Header("Content-Type")
    PRINTLN response.Text()

``Valid`` reports a completed, bounded transport. ``OK`` reports status 200
through 299, so a 404 remains a valid response with its status and body.
``FinalUrl``, ``Size`` and ``ContentType`` are also available. ``Save(path)``
writes a retained body; ``Http.Download(url, path)`` streams a successful body
through a temporary file and commits it only after completion.

``Text()`` decodes strictly as UTF-8 and returns a ``STRING``. Binary bodies and
other character encodings report ``ErrKind.Net`` with ``ErrCode.Format``;
``Bytes()`` returns the same body as ``BYTES`` without interpreting it, and
``Download()`` or ``Save()`` write it to a file instead.

``HttpMethod`` offers ``Get``, ``Head``, ``Post``, ``Put``, ``Delete`` and
``Patch``. Only ``Get`` and ``Head`` are refused a body.

A request object supplies POST bodies and safe custom headers::

    HttpRequest request = Http.New(HttpMethod.Post, "https://api.example.com/items")
    IF !request.SetHeader("Accept", "application/json") PRINTLN Error.Last().Message
    IF !request.SetText(json, "application/json") PRINTLN Error.Last().Message
    HttpResponse response = request.Send()

The setter functions change the request and return ``TRUE`` on success. On
failure they return ``FALSE``, leave it unchanged, and publish details through
``Error.Last()``.

``SetText()`` sends its argument verbatim, which is what JSON and XML need. For
an ``application/x-www-form-urlencoded`` body every value has to be
percent-encoded instead, so that a ``&`` or ``=`` inside it cannot be mistaken
for a separator. ``SetForm(name, value)`` encodes one field, appends it to the
body and sets that content type::

    HttpRequest request = Http.New(HttpMethod.Post, "https://api.example.com/messages")
    request.SetForm("title", "SYSOP wants to chat")
    request.SetForm("message", text)
    HttpResponse response = request.Send()

``Http.UrlEncode(text)`` and ``Http.UrlDecode(text)`` expose the same encoding
for everything ``SetForm()`` does not cover, such as query strings. Encode
single values only, never a whole ``name=value&...`` string. The optional second
argument selects the dialect: ``TRUE``, the default, follows the form rules
where a space is ``+``, while ``FALSE`` follows RFC 3986 where a space is
``%20``.

The optional board policy selects ``disabled``, exact-origin ``allowlist``, or
the default ``public`` destinations and sets body, timeout, redirect and
concurrency limits. Every DNS
answer and redirect is checked, validated addresses are pinned to the connection,
and system proxies are ignored. Network failures report ``ErrKind.Net``; HTTP
status codes belong to the response rather than ``Error.Last()``.

BEGIN ... END
~~~~~~~~~~~~~

Before 400 the main program had no boundary of its own. ``BEGIN`` was a pseudo
label that told ``;$USEFUNCS`` where the body started, and the ``END`` below it
was the ordinary statement that stops a program.

400 turns the pair into a real block:

.. code-block:: PPL

    DECLARE PROCEDURE Greet()

    BEGIN
        PRINTLN "Hello"
        Greet()
    END

    PROCEDURE Greet()
        PRINTLN "from a procedure"
    ENDPROC

A ``BEGIN`` without a matching ``END`` is an error, and once a program has a
block, a statement outside it is one too - only declarations and comments may
stand next to it. Because the block says where the body is, ``;$USEFUNCS`` is no
longer needed to keep it in front of the routines; the block may just as well
follow them. ``BEGIN`` may also group statements inside a routine, where it does
nothing but read as one unit.

``END`` closes a block and nothing else from 400 on - it is no longer a
statement. Two words say what one used to: :PPL:`EXIT` ends a program normally,
:PPL:`STOP` aborts it.

.. code-block:: PPL

    BEGIN
        IF (!HasAccess()) THEN
            PRINTLN "Sorry."
            STOP
        ENDIF
        PRINTLN "Welcome."
        EXIT
    END

That removes the one place where PPL used a single word for two unrelated
things. A trailing ``EXIT`` can simply go: the compiler has always appended the
terminating instruction by itself. ``EXIT`` compiles to the instruction ``END``
always stood for, so the executable stays what it was.

The formatter indents the body of a block like any other block, and puts ``END``
back at the column its ``BEGIN`` starts on.

What 4.00 breaks
~~~~~~~~~~~~~~~~

* Runtime 400 PPEs do not load on an original PCBoard.
* ``[`` and ``]`` are index operators.
* ``BEGIN`` is a keyword, so a 3.50 source may still have a variable called
  ``begin`` while a 4.00 source may not.
* ``END`` is a block terminator rather than a statement; :PPL:`EXIT` ends a
  program and :PPL:`STOP` aborts one.
* ``EXIT`` is a statement name from 4.00 on, so a 3.50 source may still have a
  variable called ``exit``.
* A decompiled PPE names its records ``TYPE001`` and their fields ``FIELD001``,
  because the file carries no names to recover.
