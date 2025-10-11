Data Types
==========

PPL (PCBoard Programming Language) supports several fundamental data types for programming Icy Board BBS applications.

Basic Data Types
----------------

INTEGER / SDWORD / LONG
~~~~~~~~~~~~~~~~~~~~~~~
- **Size**: 2 bytes (16-bit signed)
- **Range**: -32,768 to 32,767
- **Default value**: 0
- **Declaration**: ``INTEGER varname`` or ``INTEGER varname = value``

Example::

    INTEGER count
    INTEGER maxUsers = 100
    INTEGER temperature = -5

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
- **Size**: Variable length (up to 256 characters in PCBoard, unlimited in Icy Board)
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

MESSAGEAREAID
~~~~~~~~~~~~~

A datatype that contains a reference to a message conference/area number.
This is used in Icy Board to support area numbers. It's used verywhere where CONFNUMBER 
was used in PCBoard for messages. 
So all PPEs are usually backwards compatible but may not be message area aware.

- **Size**: 8 bytes
- **Purpose**: Reference to message conference/area numbers

PASSWORD
~~~~~~~~
Only U_PWD and U_PWDHIST are of this type. Can't be declared by the user.
Can be compared to STRING, U_PWD can be assigned from STRING.

Will be converted as string in PlainText when password hashing is disabled in the system settings.
Otherwise it will be "******"

- **Size**: Variable (typically hashed/encrypted)
- **Default value**: Empty/null
- **Purpose**: Secure password storage and handling


Composite Data Types
--------------------

Arrays
~~~~~~
PPL supports single-dimensional arrays of any basic data type.

- **Declaration**: ``TYPE ARRAY(size) varname``
- **Indexing**: 1-based (arrays start at index 1, not 0)
- **Maximum size**: Typically 1000 elements

Example::

    INTEGER ARRAY(10) scores
    STRING ARRAY(50) userNames
    BOOLEAN ARRAY(7) weekDays
    
    ; Accessing array elements
    scores(1) = 100
    userNames(5) = "John Doe"
    weekDays(1) = TRUE  ; Monday

Type Conversion
---------------

PPL provides automatic type conversion in many cases, but explicit conversion functions are available:

- **STRING()**: Convert to string
- **INTEGER()**: Convert to integer
- **REAL()**: Convert to real
- **MONEY()**: Convert to money
- **DATE()**: Convert to date
- **TIME()**: Convert to time
- **BYTE()**: Convert to byte
- **WORD()**: Convert to word
- **DWORD()**: Convert to dword

Example::

    STRING strNum = "123"
    INTEGER intNum = INTEGER(strNum)  ; Convert string to integer
    
    REAL realVal = 3.14
    STRING strVal = STRING(realVal)   ; Convert real to string
    
    INTEGER days = 30
    DATE future = DATE() + days       ; Automatic conversion

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
4. **String length**: Remember STRING has a 256-character limit
5. **Array bounds**: Always check array bounds (1-based indexing)
6. **Type conversion**: Use explicit conversion when mixing types

Example of good practices::

    ; Good: Clear initialization and appropriate types
    STRING userName = ""
    INTEGER userAge = 0
    MONEY accountBalance = 0.00
    BOOLEAN isVerified = FALSE
    
    ; Check before array access
    INTEGER ARRAY(10) data
    INTEGER index = 5
    IF (index >= 1 AND index <= 10) THEN
        data(index) = 100
    ENDIF