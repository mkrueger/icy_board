Data Types
==========

PPL (PCBoard Programming Language) supports several fundamental data types for programming Icy Board BBS applications.

Basic Data Types
----------------

INTEGER / SDWORD
~~~~~~~~~~~~~~~~
- **Size**: 4 bytes (32-bit signed)
- **Range**: -2,147,483,648 to 2,147,483,647
- **Default value**: 0
- **Declaration**: ``INTEGER varname`` or ``INTEGER varname = value``

Example::

    INTEGER count
    INTEGER maxUsers = 100
    INTEGER temperature = -5

LONG
~~~~
- **Size**: 8 bytes (64-bit signed)
- **Range**: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807
- **Default value**: 0
- **Declaration**: ``LONG varname`` or ``LONG varname = value``
- **Since**: PPL 4.00. In older language versions, ``LONG`` remains a synonym for ``INTEGER``.

Example::

    LONG messageNumber = 4294967295
    LONG nextMessage = messageNumber + 1

ULONG
~~~~~
- **Size**: 8 bytes (64-bit unsigned)
- **Range**: 0 to 18,446,744,073,709,551,615
- **Default value**: 0
- **Declaration**: ``ULONG varname`` or ``ULONG varname = value``
- **Since**: PPL 4.00

Example::

    ULONG byteCount = ToULong("18446744073709551615")

BOOLEAN
~~~~~~~
- **Size**: 1 byte
- **Values**: TRUE (1) or FALSE (0)
- **Default value**: FALSE
- **Declaration**: ``BOOLEAN varname`` or ``BOOLEAN varname = value``

Example::

    BOOLEAN isActive
    BOOLEAN hasAccess = TRUE
    BOOLEAN debugMode = FALSE

STRING
~~~~~~
- **Size**: Up to 256 Unicode characters before language 400; unbounded in
    language 400
- **PPE type ID**: 7 before language 400; 24 in language 400
- **Default value**: Empty string ("")
- **Declaration**: ``STRING varname`` or ``STRING varname = "value"``

Example::

    STRING userName
    STRING greeting = "Welcome to the BBS!"
    STRING menuOption = "A"

Special String Operations::

    STRING fullName = firstName + " " + lastName  ; Concatenation
    STRING upper = UPPER(userName)                 ; Convert to uppercase
    STRING lower = LOWER(userName)                 ; Convert to lowercase
    STRING part = MID(text, start, length)        ; Substring extraction

MONEY
~~~~~
- **Size**: 4 bytes
- **Range**: -21,474,836.48 to 21,474,836.47
- **Precision**: 2 decimal places (cents)
- **Default value**: 0.00
- **Declaration**: ``MONEY varname`` or ``MONEY varname = value``

Example::

    MONEY accountBalance
    MONEY price = 19.95
    MONEY debt = -150.00

DATE
~~~~
- **Size**: 2 bytes
- **Format**: Stored as days since 1/1/1900
- **Default value**: Current date
- **Declaration**: ``DATE varname`` or ``DATE varname = value``

Example::

    DATE today
    DATE birthDate = "12/25/1980"
    DATE expiration = DATE() + 30  ; 30 days from today

TIME
~~~~
- **Size**: 2 bytes  
- **Format**: Minutes since midnight
- **Range**: 0 to 1439 (23:59)
- **Default value**: Current time
- **Declaration**: ``TIME varname`` or ``TIME varname = value``

Example::

    TIME currentTime
    TIME loginTime = TIME()
    TIME meetingTime = "14:30"

BYTE / UBYTE
~~~~~~~~~~~~
- **Size**: 1 byte
- **Range**: 0 to 255 (unsigned)
- **Default value**: 0
- **Declaration**: ``BYTE varname`` or ``BYTE varname = value``

Example::

    BYTE colorCode
    BYTE menuLevel = 5
    BYTE asciiChar = 65  ; 'A'

WORD / UWORD
~~~~~~~~~~~~
- **Size**: 2 bytes
- **Range**: 0 to 65,535 (unsigned)
- **Default value**: 0
- **Declaration**: ``WORD varname`` or ``WORD varname = value``

Example::

    WORD nodeNumber
    WORD maxNodes = 250
    WORD portNumber = 8080

UNSIGNED / DWORD / UDWORD
~~~~~~~~~~~~~~~~~~~~~~~~~
- **Size**: 4 bytes
- **Range**: 0 to 4,294,967,295 (unsigned)
- **Default value**: 0
- **Declaration**: ``DWORD varname`` or ``DWORD varname = value``

Example::

    DWORD fileSize
    DWORD downloadBytes = 1048576  ; 1 MB
    DWORD totalCalls

REAL / FLOAT
~~~~~~~~~~~~
- **Size**: 4 bytes (single precision float)
- **Range**: Approximately ±3.4E38
- **Precision**: ~7 significant digits
- **Default value**: 0.0
- **Declaration**: ``REAL varname`` or ``REAL varname = value``


DREAL / DOUBLE
~~~~~~~~~~~~~~
- **Size**: 8 bytes (double precision float)
- **Range**: Approximately ±1.8E308
- **Precision**: ~15 significant digits
- **Default value**: 0.0
- **Purpose**: Higher precision floating point calculations
- **Declaration**: ``DOUBLE varname`` or ``DOUBLE varname = value``


Example::

    REAL percentage
    REAL pi = 3.14159
    REAL temperature = 98.6

SBYTE / SHORT
~~~~~~~~~~~~~
- **Size**: 1 byte
- **Range**: -128 to 127 (signed)
- **Default value**: 0
- **Declaration**: ``SBYTE varname`` or ``SBYTE varname = value``

Example::

    SBYTE temperature = -15
    SBYTE adjustment = -5
    SBYTE delta = 127

SWORD / INT
~~~~~~~~~~~~~
- **Size**: 2 bytes
- **Range**: -32,768 to 32,767 (signed)
- **Default value**: 0
- **Declaration**: ``SWORD varname`` or ``SWORD varname = value``

Special Data Types
------------------

These data types are only valid in Icy Board and not in PCBoard.
They are used for specific purposes to support new features without 
breaking compatibility with existing PCBoard PPL scripts.

MSGAREAID
~~~~~~~~~

A datatype that contains a reference to a message conference/area number.
This is used in Icy Board to support area numbers. It's used verywhere where CONFNUMBER 
was used in PCBoard for messages. 
So all PPEs are usually backwards compatible but may not be message area aware.

- **Size**: 8 bytes
- **Purpose**: Reference to message conference/area numbers

PASSWORD
~~~~~~~~
The type of ``U_PWD``, ``U_PWDHIST`` and of a door's ``Password``. It cannot be
declared. It compares against a ``STRING``, and ``U_PWD`` can be assigned one.

Converting or printing one yields the password in plain text while password
hashing is off in the system settings, and ``******`` otherwise.

- **Purpose**: Secure password storage and handling

Board objects
~~~~~~~~~~~~~

``CONFERENCE``, ``AREA``, ``DIRECTORY`` and ``DOOR`` are read-only snapshots of
what the board is configured with. They are declared like any other type and read
with the ``.`` operator::

    CONFERENCE conf = Session.Conference
    IF conf.HasAccess() PRINTLN conf.Name

See :doc:`language` for what each of them answers.

Records
~~~~~~~

A program declares record types of its own with ``TYPE ... ENDTYPE`` and then
uses the name wherever a built-in type name goes::

    TYPE Member
        STRING  Name
        INTEGER Age
    ENDTYPE

    Member m
    m.Name = "Sysop"

Records are values: two variables of the same type share nothing. See
:doc:`language` for the whole story, including record literals.

Composite Data Types
--------------------

Arrays
~~~~~~
An array is declared by writing its bounds after the name. Up to three
dimensions are supported.

- **Declaration**: ``INTEGER scores(10)``, ``INTEGER grid(10, 10)``
- **Indexing**: zero based, with ``[ ]`` or ``( )``
- **Bounds**: the declared number is the *highest* index, so ``scores(10)`` holds
  the elements 0 to 10
- **Length**: ``array.Len()`` answers the total number of elements;
    ``array.Len(dim)`` and ``Len(array, dim)`` answer the number in one dimension

Example::

    INTEGER scores(10)
    STRING  userNames(50)
    BOOLEAN weekDays(6)

    scores[0] = 100
    userNames[5] = "John Doe"
    weekDays[0] = TRUE  ; Monday

Since language version 350 an array may be written out instead::

    INTEGER values = { 1, 2, 3 }

which declares an array with exactly three elements and fills them. An array of a
record type works the same way, and every element has fields of its own::

    Member members(10)
    members[0].Name = "Sysop"

An array has to be named with one subscript per dimension wherever its value is
read, the way PCBoard required: ``scores`` on its own is an error, ``scores(0)``
is the element. The statements and functions that genuinely take a whole array -
``REDIM`` and ``SORT`` in the original language, plus the runtime-400
``array.Len()``, ``Len(array, dim)`` and ``FOREACH`` APIs - still receive all of
it. Classic one-argument ``Len(array)`` is a value read and is rejected.

Type Conversion
---------------

PPL converts between types on assignment where that makes sense. Where it should
be spelled out, the ``To...`` functions do it:

- **ToString()** / **String()**: Convert to string
- **ToInteger()**: Convert to integer
- **ToReal()**, **ToDouble()**: Convert to a floating point number
- **ToMoney()**: Convert to money
- **ToDate()**, **ToTime()**: Convert to date or time
- **ToByte()**, **ToWord()**, **ToDWord()**, **ToUnsigned()**: Convert to the
  smaller number types
- **ToBoolean()**: Convert to boolean

Example::

    STRING strNum = "123"
    INTEGER intNum = ToInteger(strNum)   ; Convert string to integer

    REAL realVal = 3.14
    STRING strVal = ToString(realVal)    ; Convert real to string

    INTEGER days = 30
    DATE future = Date() + days          ; Automatic conversion

Special Constants
-----------------

PPL defines several built-in constants:

- **TRUE**: Boolean true value (1)
- **FALSE**: Boolean false value (0)

Variable Scope
--------------

Variables in PPL have different scopes:

- **Local variables**: Declared within a procedure/function, only accessible there
- **Global variables**: Declared outside procedures, accessible throughout the program
- **System variables**: Predefined PPL variables (e.g., ``U_NAME``, ``U_PWDHIST``)

Example::

    ; Global variable
    INTEGER globalCounter
    
    PROCEDURE LocalExample()
        ; Local variable
        STRING localMessage = "This is local"
        globalCounter = globalCounter + 1
    ENDPROC

Best Practices
--------------

1. **Initialize variables**: Always initialize variables when declaring them
2. **Use appropriate types**: Choose the most appropriate data type for your needs
3. **Check ranges**: Be aware of type limits to avoid overflow
4. **Index with brackets**: ``values[0]`` says it is an array, ``values(0)`` reads
   like a call
5. **Array bounds**: The declared number is the highest index, and indices start
   at 0
6. **Type conversion**: Use explicit conversion when mixing types

Example of good practices::

    ; Good: Clear initialization and appropriate types
    STRING userName = ""
    INTEGER userAge = 0
    MONEY accountBalance = 0.00
    BOOLEAN isVerified = FALSE

    ; Check before array access
    INTEGER data(10)
    INTEGER index = 5
    IF (index >= 0 && index <= 10) THEN
        data[index] = 100
    ENDIF