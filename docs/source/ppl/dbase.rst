DBase III+ API
==============

The DBase III+ support in PPL provides native database management capabilities, allowing direct access to .DBF files without requiring text file parsing. This implementation supports indexing for efficient record access and is compatible with the widely-used DBase III+ format.

Overview
--------

PPL's DBase support includes comprehensive functions and statements for database operations. All DBase-related commands begin with the letter 'D' for easy identification. Most statements have corresponding function versions that return error status or other useful values.

**Important Notes:**

* File extensions are optional in all file name parameters. `.DBF` and `.NDX` are assumed defaults.
* Up to 8 database channels (0-7) can be open simultaneously
* Error handling functions return inverted logic for consistency (TRUE on error)

Database Channels
-----------------

PPL supports 8 concurrent database channels numbered 0 through 7. Each channel can have one DBF file open with multiple associated index files.

Field Types
-----------

==================  ====  =========================================
Type                Code  Description
==================  ====  =========================================
Character           C     Text strings
Numeric             N     Integer numbers
Floating Point      F     Decimal numbers
Date                D     Date values (stored as DDATE)
Logical             L     Boolean values (TRUE/FALSE)
Memo                M     Large text fields (memo fields)
==================  ====  =========================================

Statements
----------

File Operations
~~~~~~~~~~~~~~~

DCREATE (3.00)
^^^^^^^^^^^^^^
  :PPL:`STATEMENT DCREATE(INTEGER channel, STRING name, BOOLEAN exclusive, STRING[] fieldInfo)`

  Create a new DBF database file.

  **Parameters**
    * :PPL:`channel` – Database channel (0-7)
    * :PPL:`name` – Database file name (extension optional)
    * :PPL:`exclusive` – TRUE for exclusive access, FALSE for shared
    * :PPL:`fieldInfo` – Array of field definitions

  **Field Definition Format**
    Each field definition string: "FieldName,Type,Length,Decimals"

  **Example**

    .. code-block:: PPL

       STRING finfo(3)
       LET finfo(0) = "First,C,20,0"
       LET finfo(1) = "Last,C,20,0"
       LET finfo(2) = "Phone,C,15,0"
       DCREATE 0, "CONTACTS", TRUE, finfo

DOPEN (3.00)
^^^^^^^^^^^^
  :PPL:`STATEMENT DOPEN(INTEGER channel, STRING name, BOOLEAN exclusive)`

  Open an existing DBF file.

  **Parameters**
    * :PPL:`channel` – Database channel (0-7)
    * :PPL:`name` – Database file name
    * :PPL:`exclusive` – TRUE for exclusive access, FALSE for shared

DCLOSE (3.00)
^^^^^^^^^^^^^
  :PPL:`STATEMENT DCLOSE(INTEGER channel)`

  Close a database file.

  **Parameters**
    * :PPL:`channel` – Database channel to close

DCLOSEALL (3.00)
^^^^^^^^^^^^^^^^
  :PPL:`STATEMENT DCLOSEALL`

  Close all open database files on all channels.

DSETALIAS (3.00)
^^^^^^^^^^^^^^^^
  :PPL:`STATEMENT DSETALIAS(INTEGER channel, STRING name)`

  Set an alias name for a database channel.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`name` – Alias name

DPACK (3.00)
^^^^^^^^^^^^
  :PPL:`STATEMENT DPACK(INTEGER channel)`

  Pack database file to permanently remove deleted records.

  **Parameters**
    * :PPL:`channel` – Database channel

Record Locking
~~~~~~~~~~~~~~

DLOCK / DLOCKF (3.00)
^^^^^^^^^^^^^^^^^^^^^
  :PPL:`STATEMENT DLOCK(INTEGER channel)`

  Lock entire database file for exclusive access.

  **Parameters**
    * :PPL:`channel` – Database channel

DLOCKR (3.00)
^^^^^^^^^^^^^
  :PPL:`STATEMENT DLOCKR(INTEGER channel, INTEGER recno)`

  Lock a specific record.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`recno` – Record number to lock

DLOCKG (3.00)
^^^^^^^^^^^^^
  :PPL:`STATEMENT DLOCKG(INTEGER channel, INTEGER recnos, INTEGER count)`

  Lock a group of consecutive records.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`recnos` – Starting record number
    * :PPL:`count` – Number of records to lock

DUNLOCK (3.00)
^^^^^^^^^^^^^^
  :PPL:`STATEMENT DUNLOCK(INTEGER channel)`

  Release all current locks on the channel.

  **Parameters**
    * :PPL:`channel` – Database channel

Index Operations
~~~~~~~~~~~~~~~~

DNCREATE (3.00)
^^^^^^^^^^^^^^^
  :PPL:`STATEMENT DNCREATE(INTEGER channel, STRING name, STRING expression)`

  Create a new index file.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`name` – Index file name
    * :PPL:`expression` – Index expression (field name)

DNOPEN (3.00)
^^^^^^^^^^^^^
  :PPL:`STATEMENT DNOPEN(INTEGER channel, STRING name)`

  Open an index file.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`name` – Index file name

DNCLOSE (3.00)
^^^^^^^^^^^^^^
  :PPL:`STATEMENT DNCLOSE(INTEGER channel, STRING name)`

  Close a specific index file.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`name` – Index file name

DNCLOSEALL (3.00)
^^^^^^^^^^^^^^^^^
  :PPL:`STATEMENT DNCLOSEALL(INTEGER channel)`

  Close all index files for a channel.

  **Parameters**
    * :PPL:`channel` – Database channel


DNEXT (3.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER DNEXT()`

  Returns the next available dBase file channel number.

  **Returns**
    Next available dBase channel number, or -1 if all channels are in use.

  **Remarks**
    Created to support code libraries with functions and procedures, allowing dBase 
    file channel numbers to be determined at runtime. DNEXT returns the lowest available 
    channel but does NOT reserve it - you must open a dBase file on that channel before 
    calling DNEXT again, otherwise it will return the same value. Never call DNEXT 
    directly in a DOPEN statement as there's no way to determine which channel was used.

  **Example**

    .. code-block:: PPL

       INTEGER chan1, chan2
       
       ; CORRECT usage - store channel, then use it
       chan1 = DNEXT()
       IF (chan1 <> -1) THEN
           DOPEN chan1, "USERS.DBF", "USERS.NDX"
       ENDIF
       
       chan2 = DNEXT()
       IF (chan2 <> -1) THEN
           DOPEN chan2, "MESSAGES.DBF", ""
       ENDIF
       
       ; WRONG - DNEXT returns same value if no file opened
       ; chan1 = DNEXT()
       ; chan2 = DNEXT()  ; ERROR: chan1 equals chan2!
       
       ; WRONG - No way to know which channel was used
       ; DOPEN DNEXT(), "FILE.DBF", ""

  **See Also**
    * :PPL:`DOPEN` – Open dBase file
    * :PPL:`DCLOSE` – Close dBase file
    * :PPL:`DCREATE` – Create dBase file
    * :PPL:`FNEXT()` – Next file channel

DTAG (3.00)
^^^^^^^^^^^
  :PPL:`STATEMENT DTAG(INTEGER channel, STRING name)`

  Select an index tag as active.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`name` – Tag name

Record Navigation
~~~~~~~~~~~~~~~~~

DTOP (3.00)
^^^^^^^^^^^
  :PPL:`STATEMENT DTOP(INTEGER channel)`

  Move to the first record.

  **Parameters**
    * :PPL:`channel` – Database channel

DBOTTOM (3.00)
^^^^^^^^^^^^^^
  :PPL:`STATEMENT DBOTTOM(INTEGER channel)`

  Move to the last record.

  **Parameters**
    * :PPL:`channel` – Database channel

DGO (3.00)
^^^^^^^^^^
  :PPL:`STATEMENT DGO(INTEGER channel, INTEGER recno)`

  Move to a specific record number.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`recno` – Target record number

DSKIP (3.00)
^^^^^^^^^^^^
  :PPL:`STATEMENT DSKIP(INTEGER channel, INTEGER number)`

  Skip forward or backward by a number of records.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`number` – Records to skip (negative for backward)

DSEEK (3.00)
^^^^^^^^^^^^
  :PPL:`STATEMENT DSEEK(INTEGER channel, STRING expression)`

  Search for a record using the active index.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`expression` – Search value

Record Manipulation
~~~~~~~~~~~~~~~~~~~

DNEW (3.00)
^^^^^^^^^^^
  :PPL:`STATEMENT DNEW(INTEGER channel)`

  Start a new record (must call DADD to save).

  **Parameters**
    * :PPL:`channel` – Database channel

DADD (3.00)
^^^^^^^^^^^
  :PPL:`STATEMENT DADD(INTEGER channel)`

  Add the new record started with DNEW.

  **Parameters**
    * :PPL:`channel` – Database channel

DAPPEND (3.00)
^^^^^^^^^^^^^^
  :PPL:`STATEMENT DAPPEND(INTEGER channel)`

  Append a blank record and position on it.

  **Parameters**
    * :PPL:`channel` – Database channel

DBLANK (3.00)
^^^^^^^^^^^^^
  :PPL:`STATEMENT DBLANK(INTEGER channel)`

  Blank all fields in the current record.

  **Parameters**
    * :PPL:`channel` – Database channel

DDELETE (3.00)
^^^^^^^^^^^^^^
  :PPL:`STATEMENT DDELETE(INTEGER channel)`

  Mark current record as deleted.

  **Parameters**
    * :PPL:`channel` – Database channel

DRECALL (3.00)
^^^^^^^^^^^^^^
  :PPL:`STATEMENT DRECALL(INTEGER channel)`

  Unmark deleted status of current record.

  **Parameters**
    * :PPL:`channel` – Database channel

Field Operations
~~~~~~~~~~~~~~~~

DFBLANK (3.00)
^^^^^^^^^^^^^^
  :PPL:`STATEMENT DFBLANK(INTEGER channel, STRING name)`

  Blank a specific field in the current record.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`name` – Field name

DGET (3.00)
^^^^^^^^^^^
  :PPL:`STATEMENT DGET(INTEGER channel, STRING name, ANY var)`

  Get value from a field into a variable.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`name` – Field name
    * :PPL:`var` – Variable to receive the value

DPUT (3.00)
^^^^^^^^^^^
  :PPL:`STATEMENT DPUT(INTEGER channel, STRING name, ANY expression)`

  Put value into a field.

  **Parameters**
    * :PPL:`channel` – Database channel
    * :PPL:`name` – Field name
    * :PPL:`expression` – Value to store

DFCOPY (3.00)
^^^^^^^^^^^^^
  :PPL:`STATEMENT DFCOPY(INTEGER channel1, STRING name1, INTEGER channel2, STRING name2)`

  Copy field value between records/databases.

  **Parameters**
    * :PPL:`channel1` – Source database channel
    * :PPL:`name1` – Source field name
    * :PPL:`channel2` – Destination database channel
    * :PPL:`name2` – Destination field name

Functions
---------

Database Information
~~~~~~~~~~~~~~~~~~~~

DRECCOUNT (3.00)
^^^^^^^^^^^^^^^^
  :PPL:`FUNCTION INTEGER DRECCOUNT(INTEGER channel)`

  Returns the total number of records in the database.

DRECNO (3.00)
^^^^^^^^^^^^^
  :PPL:`FUNCTION INTEGER DRECNO(INTEGER channel)`

  Returns the current record number.

DBOF (3.00)
^^^^^^^^^^^
  :PPL:`FUNCTION BOOLEAN DBOF(INTEGER channel)`

  Returns TRUE if positioned before the first record.

DEOF (3.00)
^^^^^^^^^^^
  :PPL:`FUNCTION BOOLEAN DEOF(INTEGER channel)`

  Returns TRUE if positioned after the last record.

DDELETED (3.00)
^^^^^^^^^^^^^^^
  :PPL:`FUNCTION BOOLEAN DDELETED(INTEGER channel)`

  Returns TRUE if current record is marked as deleted.

DCHANGED (3.00)
^^^^^^^^^^^^^^^
  :PPL:`FUNCTION BOOLEAN DCHANGED(INTEGER channel)`

  Returns TRUE if current record has been modified.

Field Information
~~~~~~~~~~~~~~~~~

DFIELDS (3.00)
^^^^^^^^^^^^^^
  :PPL:`FUNCTION INTEGER DFIELDS(INTEGER channel)`

  Returns the number of fields in the database.

DNAME (3.00)
^^^^^^^^^^^^
  :PPL:`FUNCTION STRING DNAME(INTEGER channel, INTEGER number)`

  Returns the name of field by number (1-based).

DTYPE (3.00)
^^^^^^^^^^^^
  :PPL:`FUNCTION STRING DTYPE(INTEGER channel, STRING name)`

  Returns the type of the named field (C/N/F/D/L/M).

DLENGTH (3.00)
^^^^^^^^^^^^^^
  :PPL:`FUNCTION INTEGER DLENGTH(INTEGER channel, STRING name)`

  Returns the length of the named field.

DDECIMALS (3.00)
^^^^^^^^^^^^^^^^
  :PPL:`FUNCTION INTEGER DDECIMALS(INTEGER channel, STRING name)`

  Returns the decimal places for numeric fields.

Utility Functions
~~~~~~~~~~~~~~~~~

DSELECT (3.00)
^^^^^^^^^^^^^^
  :PPL:`FUNCTION INTEGER DSELECT(STRING alias)`

  Returns the channel number associated with an alias.

DGETALIAS (3.00)
^^^^^^^^^^^^^^^^
  :PPL:`FUNCTION STRING DGETALIAS(INTEGER channel)`

  Returns the current alias for a channel.

DGET (3.00)
^^^^^^^^^^^
  :PPL:`FUNCTION STRING DGET(INTEGER channel, STRING name)`

  Returns value from a field as a string.

DSEEK (3.00)
^^^^^^^^^^^^
  :PPL:`FUNCTION INTEGER DSEEK(INTEGER channel, STRING expression)`

  Searches for a record and returns status.

  **Returns**
    * 0 = Error or not found
    * 1 = Exact match found
    * 2 = Following record (partial match)
    * 3 = End of file reached

Error Handling
~~~~~~~~~~~~~~

DERR (3.00)
^^^^^^^^^^^
  :PPL:`FUNCTION BOOLEAN DERR(INTEGER channel)`

  Returns TRUE if an error occurred on the channel.

DERRMSG (3.00)
^^^^^^^^^^^^^^
  :PPL:`FUNCTION STRING DERRMSG(INTEGER errcode)`

  Returns error message text for an error code.

Function Versions of Statements
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Most statements have function equivalents that return a BOOLEAN error status:

* :PPL:`DOPEN()`, :PPL:`DCLOSE()`, :PPL:`DCLOSEALL()`
* :PPL:`DSETALIAS()`, :PPL:`DPACK()`
* :PPL:`DLOCK()`, :PPL:`DLOCKR()`, :PPL:`DUNLOCK()`
* :PPL:`DNOPEN()`, :PPL:`DNCLOSE()`, :PPL:`DNCLOSEALL()`
* :PPL:`DNEW()`, :PPL:`DADD()`, :PPL:`DAPPEND()`
* :PPL:`DTOP()`, :PPL:`DGO()`, :PPL:`DBOTTOM()`, :PPL:`DSKIP()`
* :PPL:`DBLANK()`, :PPL:`DDELETE()`, :PPL:`DRECALL()`
* :PPL:`DTAG()`, :PPL:`DFBLANK()`
* :PPL:`DPUT()`, :PPL:`DFCOPY()`

**Note:** These functions return FALSE on success, TRUE on error (inverted logic).

Example Usage
-------------

Creating and Populating a Database
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: PPL

   ; Define database structure
   STRING fields(3)
   LET fields(0) = "Name,C,30,0"
   LET fields(1) = "Phone,C,15,0"
   LET fields(2) = "Balance,N,10,2"
   
   ; Create database
   DCREATE 0, "CUSTOMERS", TRUE, fields
   
   ; Add a record
   DNEW 0
   DPUT 0, "Name", "John Smith"
   DPUT 0, "Phone", "555-1234"
   DPUT 0, "Balance", 1500.50
   DADD 0
   
   ; Create index on Name field
   DNCREATE 0, "CUSTNAME", "Name"
   DNOPEN 0, "CUSTNAME"

Searching and Updating Records
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: PPL

   ; Open database and index
   DOPEN 0, "CUSTOMERS", FALSE
   DNOPEN 0, "CUSTNAME"
   
   ; Search for a record
   IF (DSEEK(0, "John Smith") = 1) THEN
       STRING phone
       DGET 0, "Phone", phone
       PRINTLN "Phone: ", phone
       
       ; Update balance
       DPUT 0, "Balance", 2000.00
   ENDIF
   
   DCLOSE 0

Error Handling
~~~~~~~~~~~~~~

.. code-block:: PPL

   IF (!DOPEN(0, "CUSTOMERS", FALSE)) THEN
       PRINTLN "Error opening database"
       PRINTLN DERRMSG(0)
       STOP
   ENDIF
   
   ; Alternative using DERR
   DOPEN 0, "CUSTOMERS", FALSE
   IF (DERR(0)) THEN
       PRINTLN "Database error occurred"
   ENDIF

See Also
--------

* HOWTODBF.TXT - Detailed tutorial on DBase programming in PPL
* :PPL:`FNEXT()` - Find next available file channel
* :PPL:`DNEXT()` - Find next available database channel