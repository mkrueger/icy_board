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
   :header: "Version", "Command line", "ppl.toml", "What it controls"
   :widths: 15, 25, 30, 30

   "Runtime", "``--runtime``", "``[package] runtime``", "The PPE format written to disk"
   "Language", "``--lang-version``", "``[compiler] language_version``", "Which syntax and built-ins are accepted"

The runtime defaults to 401, the newest format. The language defaults to the
runtime version up to 400, so the default pair is runtime 401 and language 400.
A format-only runtime bump therefore does not invent a new language version. The
command line wins over ``ppl.toml``.

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
   Routine references need runtime 401, because 4.01 adds the bytecode marker
   that tells a routine value from a call to that routine.

Language version 4.00
---------------------

400 is where the language stops being bound by what PCBoard 15.4 could express.
A PPE built at runtime 400 or 401 will not load on an original PCBoard.

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

Writing to a constant is an error, and a constant and a variable may not share a
name. ``CONST`` is a keyword from 400 on, so a 3.50 source may still have a
variable called ``const``.

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

The dot operator and board objects
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

400 introduces the ``.`` operator and, with it, objects that describe the board
itself. The point is that a PPE no longer has to parse the board's config files
to find out what is on it.

.. code-block:: PPL

    CONFERENCE conf = ConfInfo(CurConf())

    IF conf.HasAccess() PRINTLN conf.Name

``ConfInfo(conf)`` returns a read-only ``CONFERENCE`` snapshot. An invalid
conference number returns an empty conference rather than failing, so its
properties can still be read.

**CONFERENCE**

================  ===============  ==================================================
Member            Type             Description
================  ===============  ==================================================
``Name``          ``STRING``       Conference name
``IsPublic``      ``BOOLEAN``      Whether the conference is configured as public
``Directories``   ``INTEGER``      Number of file directories
``Areas``         ``INTEGER``      Number of message areas
``Doors``         ``INTEGER``      Number of doors
``HasAccess()``   ``BOOLEAN``      Whether the current caller can access it
``GetDir(i)``     ``DIRECTORY``    File directory at the zero based index
``GetArea(i)``    ``AREA``         Message area at the zero based index
``GetDoor(i)``    ``DOOR``         Door at the zero based index
================  ===============  ==================================================

**DIRECTORY** and **AREA** provide ``Name`` and ``HasAccess()``. **DOOR**
provides ``Name``, ``Description``, ``Password`` and ``HasAccess()``.

Walking a conference:

.. code-block:: PPL

    CONFERENCE conf = ConfInfo(CurConf())
    INTEGER i

    FOR i = 0 TO conf.Doors - 1
        DOOR item = conf.GetDoor(i)
        IF item.HasAccess() PRINTLN item.Name
    NEXT

``CONFERENCE``, ``DOOR``, ``AREA`` and ``DIRECTORY`` are resolved wherever a type
name is expected, so a variable cannot be called ``door`` or ``area``. The names
are compared without regard to case, so this holds for a record type a program
declares too: ``Point point`` leaves ``point`` ambiguous.

These objects are read-only snapshots, so assigning to a member -
``conf.Name = "x"`` - is rejected. What a member answers may be asked again, so
``conf.GetDoor(0).Name`` reads in one go.

Overloaded built-ins
~~~~~~~~~~~~~~~~~~~~

A built-in can now have more than one signature, chosen by argument count.
``ConfInfo`` is the example: the original two-argument form returns a single
field whose type depends on which field was asked for, while the new one-argument
form returns a ``CONFERENCE``. Old code keeps working unchanged. ``Len`` is
overloaded the same way - ``Len(array, dim)`` returns the length of one
dimension.

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
   Records need runtime 401, which is the first format with a type table. Record
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
* An array as a field - ``INTEGER Values(10)`` inside a ``TYPE`` block - is
  rejected: the type table stores each field's type but not its dimensions.
* Custom types are nominal: two separately declared records are different types
  even when their fields happen to match.
* Equality compares two records of the same type by their fields. Whole arrays of
  records cannot be compared; index them first.

All ``TYPE`` declarations in a package are collected before its source files are
parsed, so ``main.pps`` may use a type declared in another file. Record fields
still follow declaration order.

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

* Runtime 400 and 401 PPEs do not load on an original PCBoard.
* ``.`` is a token, so it can no longer appear in an identifier.
* ``[`` and ``]`` are index operators.
* ``BEGIN`` is a keyword, so a 3.50 source may still have a variable called
  ``begin`` while a 4.00 source may not.
* ``END`` is a block terminator rather than a statement; :PPL:`EXIT` ends a
  program and :PPL:`STOP` aborts one.
* ``EXIT`` is a statement name from 4.00 on, so a 3.50 source may still have a
  variable called ``exit``.
* ``CONST`` is a keyword, so a 3.50 source may still have a variable called
  ``const``.
* ``ENUM`` and ``ENDENUM`` are keywords, so a 3.50 source may still use those
    names as identifiers.
* A decompiled PPE names its records ``TYPE001`` and their fields ``FIELD001``,
  because the file carries no names to recover.
