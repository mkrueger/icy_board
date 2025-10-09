.. role:: PPL(code)
   :language: PPL


Functions
---------

ABORT (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN ABORT()`

  Returns a flag indicating whether or not the user has aborted the display of information. 

  **Returns**
    TRUE if the user has aborted the current display (via ^K, ^X, or answering "N" to MORE? prompt), FALSE otherwise.

  **Remarks**
    Checks if the user has requested to stop the current output. After an abort, BBS won't display 
    further information until RESETDISP is called. Check this function during long outputs to gracefully 
    stop and continue with the next program section.

  **Example**

    .. code-block:: PPL

       INTEGER I
       STARTDISP FCL
       ; While the user has not aborted, continue
       WHILE (!ABORT()) DO
          PRINTLN “I is equal to ",I
          INC I
       ENDWHILE
       RESETDISP 

ABS (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER ABS(INTEGER value)`
  
  Returns the absolute value of an integer expression. 

  **Parameters**
    * :PPL:`value` – Integer input (may be negative)

  **Returns**
    Absolute value of :PPL:`value`.

  **Remarks**
    Returns the non-negative magnitude of a number. Useful for calculating the distance 
    between two values regardless of order. For example, ABS(8-13) returns 5.
    It is easier to code and understand than this:
    
    .. code-block:: PPL

      INTEGER D
      D = 8 - 13
      IF (D < 0) D = -D 

  **Example**

    .. code-block:: PPL

      DIFF = ABS(A - B)

AND (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER AND(INTEGER value1, INTEGER value2)`

  Performs a bitwise AND operation between two integers.

  **Parameters**
    * :PPL:`value1` – First integer operand
    * :PPL:`value2` – Second integer operand

  **Returns**
    Bitwise AND of the two values.

  **Remarks**
    Each bit in the result is 1 only if both corresponding bits in the operands are 1.
    Common uses include clearing specific bits with a mask or calculating remainders 
    for power-of-two divisions.

  **Example**

    .. code-block:: PPL

       INTEGER flags, result
       flags = 0x0F
       result = AND(flags, 0x03)  ; Mask to keep only lowest 2 bits
       PRINTLN "Result: ", result  ; Prints 3

  **See Also**
    * :PPL:`OR()` – Bitwise OR operation
    * :PPL:`XOR()` – Bitwise XOR operation
    * :PPL:`NOT()` – Bitwise NOT operation

ANSION (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN ANSION()`
  
  Report the status of ANSI availability with the current caller. 

  **Returns**
    TRUE if the caller can support ANSI, FALSE otherwise.

  **Remarks**
    Determines ANSI capability from the user's graphics prompt response at login or 
    automatic terminal detection. Use this to conditionally display ANSI escape codes
    for colors and cursor positioning.

  **Example**

    .. code-block:: PPL

       IF (ANSION()) PRINTLN "You have ANSI support available!" 

  **See Also**
    * :PPL:`ANSIPOS` – Position cursor using ANSI codes
    * :PPL:`GRAFMODE` – Get current graphics mode

ASC (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER ASC(STRING ch)`

  Converts a character to it's ASCII code. 

  **Parameters**
    * :PPL:`ch` – String (first character used)

  **Returns**
    Returns the ASCII code of the first character of `ch` (1-255) or 0 if `ch` is an empty string.

  **Example**

    .. code-block:: PPL

       CODE = ASC("#")

B2W (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER B2W(INTEGER low, INTEGER high)`
  
  Convert two byte-sized arguments into a single word-sized argument. 
  
  **Parameters**
    * :PPL:`low` – Low byte value (0x00-0xFF)
    * :PPL:`high` – High byte value (0x00-0xFF)

  **Returns**
    Word value (0x0000-0xFFFF) computed as: low + (high * 0x100)

  **Example**

    .. code-block:: PPL

       ; Display 25 asterisks using BIOS interrupt
       ; B2W combines service 09h with ASCII value of "*"
       DOINTR 0x10, B2W(0x09, ASC("*")), 0x0007, 25, 0, 0, 0, 0

CALLID (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING CALLID()`

  Access caller ID information returned from caller ID compatible modems. 

  **Returns**
    Caller ID information captured from a compatible modem, or empty string if unavailable.

  **Remarks**
    Returns the phone number and/or name of the caller if your modem supports Caller ID 
    service and it's enabled in your area. Information is typically captured between the 
    first and second rings.

  **Example**

    .. code-block:: PPL

       FAPPEND 1,"CID.LOG",O WR,S DW
       FPUTLN 1,LEFT(U NAME(),30)*CALLID()
       FCLOSE 1 

CALLNUM (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER CALLNUM()`
  
  Returns the current caller number. 
  
  **Returns**
    Current system caller number.

  **Remarks**
    Returns the incrementing caller number assigned when users log on. The counter is 
    stored in the main conference MSGS file and only increments after successful login, 
    so check LOGGEDON() before using.

  **Example**

    .. code-block:: PPL

       IF (LOGGEDON() & (CALLNUM() = 1000000)) THEN
           PRINTLN "@BEEP@CONGRATULATIONS! YOU ARE THE 1,000,000th CALLER!"
           GETUSER
           LET U_SEC = 99
           PUTUSER
       ENDIF

  **See Also**
    * :PPL:`LOGGEDON()` – Check if user is logged in
    * :PPL:`ONLOCAL()` – Check if local session

CARRIER (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER CARRIER()`

  Determine what speed the current caller is connected at. 
  
  **Returns**
    Current connection speed in bps, or 0 if no carrier detected.

  **Remarks**
    Returns the caller's connection speed as reported by the modem. In locked port 
    configurations, this may return the DTE rate rather than actual connect speed. 
    Modern implementations may return 0 (local) or a fixed value for telnet/SSH.

  **Example**

    .. code-block:: PPL

       IF (CARRIER() < 9600) THEN
           PRINTLN "Sorry, downloads require 9600 bps or higher"
           END
       ENDIF

CCTYPE (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING CCTYPE(STRING ccnum)`

  Determine the type of a credit card based on the credit card number.

  **Parameters**
    * :PPL:`ccnum` – Credit card number string to check

  **Returns**
    Card type string: "VISA", "MASTERCARD", "AMERICAN EXPRESS", "DISCOVER", 
    "CARTE BLANCHE", "DINERS CLUB", "OPTIMA", or "UNKNOWN" if invalid/unrecognized.

  **Remarks**
    Identifies card issuer by analyzing the card number prefix. Returns "UNKNOWN" 
    for invalid numbers (VALCC() = FALSE) or unrecognized patterns.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "Credit card number",s
       IF (VALCC(s)) PRINTLN LEFT(CCTYPE(s),20)," - ",FMTCC(s)

  **See Also**
    * :PPL:`FMTCC()` – Format credit card for display
    * :PPL:`VALCC()` – Validate credit card number

CDON (1.00)
~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN CDON()`

  Determine if carrier detect is on or not.

  **Returns**
    TRUE if carrier detect is present, FALSE if carrier lost.

  **Remarks**
    If you've used CDCHKOFF to disable automatic carrier checking, use this function 
    to manually detect carrier loss and respond appropriately.

  **Example**

    .. code-block:: PPL

       IF (!CDON()) THEN
           LOG "Carrier lost in PPE "+PPENAME(),FALSE
           HANGUP
       ENDIF

  **See Also**
    * :PPL:`CDCHKOFF` – Disable automatic carrier checking
    * :PPL:`CDCHKON` – Enable automatic carrier checking

CHR (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION STRING CHR(INTEGER code)`

  Converts an ASCII code to a character.

  **Parameters**
    * :PPL:`code` – ASCII value (0–255)

  **Returns**
    Single-character string for codes 1–255, or empty string for code 0.

  **Remarks**
    PPL uses null-terminated strings, so CHR(0) returns empty rather than a null character. 
    All other values return a valid single-character string.

  **Example**

    .. code-block:: PPL

       PRINTLN "The ASCII code for S is ",ASC("S")
       ; Convert lowercase s to uppercase
       STRING s
       LET s = CHR(ASC("s")-ASC("a")+ASC("A"))

  **See Also**
    * :PPL:`ASC()` – Get ASCII code of character

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

CURCOLOR (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER CURCOLOR()`

  Returns the color in use by the ANSI driver.

  **Returns**
    Color code most recently issued to the ANSI driver.

  **Remarks**
    BBS's @X processor saves/restores colors with @X00/@XFF but only remembers 
    one at a time. Use this function to save multiple color states in your application.

  **Example**

    .. code-block:: PPL

       INTEGER savedColor
       savedColor = CURCOLOR()
       COLOR @X0F
       PRINTLN "White text"
       COLOR savedColor  ; Restore previous color

  **See Also**
    * :PPL:`COLOR` – Set current color
    * :PPL:`DEFCOLOR()` – Get default color

CURCONF (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER CURCONF()`

  Get the current conference number.

  **Returns**
    Current conference number.

  **Remarks**
    Useful for making PPL programs behave differently in different conferences. 
    For example, prompting for extra information in specific conferences.

  **Example**

    .. code-block:: PPL

       IF (CURCONF() = 6) THEN
           PRINTLN "You are in the beta conference."
           PRINTLN "Please include file date/time and problem description."
       ENDIF

  **See Also**
    * :PPL:`JOIN` – Switch conferences
    * :PPL:`CONFINFO()` – Get conference configuration

CURSEC (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER CURSEC()`

  Get the user's current security level.

  **Returns**
    Current effective security level.

  **Remarks**
    Returns the 'logical' security level accounting for base level, expiration adjustments, 
    conference-specific additions, and keyboard overrides. Use this instead of U_SEC 
    when you need the live value without calling GETUSER.

  **Example**

    .. code-block:: PPL

       IF (CURSEC() < 100) PRINTLN "Insufficient security!"

  **See Also**
    * :PPL:`U_EXPSEC` – Expiration security level
    * :PPL:`U_SEC` – Base security level

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

DATE (1.00)
~~~~~~~~~~~
  :PPL:`FUNCTION DATE DATE()`

  Get today's date.

  **Returns**
    Current system date.

  **Remarks**
    Returns date in internal julian format (days since January 1, 1900). Can be used 
    directly for display/storage or assigned to an integer for arithmetic.

  **Example**

    .. code-block:: PPL

       PRINTLN "Today is ",DATE()

  **See Also**
    * :PPL:`DAY()` – Extract day component
    * :PPL:`DOW()` – Day of week
    * :PPL:`MKDATE()` – Construct date
    * :PPL:`MONTH()` – Extract month
    * :PPL:`YEAR()` – Extract year

DAY (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER DAY(DATE d)`

  Extracts the day of the month from a date.

  **Parameters**
    * :PPL:`d` – Date value

  **Returns**
    Day of month (1-31).

  **Remarks**
    Extracts the day component from any date value for use in calculations or display.

  **Example**

    .. code-block:: PPL

       PRINTLN "Today is day: ", DAY(DATE())

  **See Also**
    * :PPL:`DATE()` – Get current date
    * :PPL:`DOW()` – Day of week
    * :PPL:`MONTH()` – Extract month
    * :PPL:`YEAR()` – Extract year

DBGLEVEL (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER DBGLEVEL()`

  Returns the debug level in effect.

  **Returns**
    Current debug level (0-3).

  **Remarks**
    Returns the system debug level where 0 is no debug output and 1-3 are increasing 
    verbosity levels. Use this to conditionally log debug information in your PPL programs.

  **Example**

    .. code-block:: PPL

       IF (DBGLEVEL() >= 1) LOG "Writing DEBUG info for "+PPENAME(),0

  **See Also**
    * :PPL:`DBGLEVEL` – Set debug level statement
    * :PPL:`LOG` – Write to caller log

DEFCOLOR (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER DEFCOLOR()`

  Return the system default color.

  **Returns**
    System default color as configured by the SysOp.

  **Remarks**
    Returns the default color value for passing to statements that require a color parameter. 
    Unlike the DEFCOLOR statement which sets output to default, this function returns the 
    actual color value.

  **Example**

    .. code-block:: PPL

       STRING yn
       LET yn = YESCHAR()
       INPUTYN "Continue",yn,DEFCOLOR()
       IF (yn = NOCHAR()) END

  **See Also**
    * :PPL:`COLOR` – Set color statement
    * :PPL:`CURCOLOR()` – Get current color
    * :PPL:`DEFCOLOR` – Reset to default color statement

DOW (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER DOW(DATE d)`

  Determine the day of the week of a particular date.

  **Parameters**
    * :PPL:`d` – Date value

  **Returns**
    Day of week (0=Sunday through 6=Saturday).

  **Remarks**
    Extracts the day of week from any date value for use in day-specific logic or display.

  **Example**

    .. code-block:: PPL

       PRINTLN "Today is day: ", DOW(DATE())

  **See Also**
    * :PPL:`DATE()` – Get current date
    * :PPL:`DAY()` – Extract day of month
    * :PPL:`MONTH()` – Extract month
    * :PPL:`YEAR()` – Extract year

EXIST (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN EXIST(STRING file)`

  Determine whether or not a file exists.

  **Parameters**
    * :PPL:`file` – Path to check (drive and directory optional)

  **Returns**
    TRUE if file exists, FALSE otherwise.

  **Remarks**
    Checks for file existence before processing. Drive defaults to current drive, 
    path defaults to current directory if not specified.

  **Example**

    .. code-block:: PPL

       STRING file
       LET file = "NEWS."+STRING(CURNODE())
       IF (EXIST(file)) DISPFILE file,0

  **See Also**
    * :PPL:`DELETE` – Remove file
    * :PPL:`FILEINF()` – Get file information
    * :PPL:`READLINE()` – Read file content

FERR (1.00)
~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN FERR(INTEGER channel)`

  Determine whether or not an error has occurred on a channel since last checked.

  **Parameters**
    * :PPL:`channel` – File channel number (0-7)

  **Returns**
    TRUE if an error occurred since last check, FALSE otherwise.

  **Remarks**
    Checks for file I/O errors (missing file, EOF, disk full, hardware issues). 
    Use after every file operation for reliability. The error flag is cleared when read.

  **Example**

    .. code-block:: PPL

       STRING s
       FOPEN 1,"FILE.DAT",O_RD,S_DW
       IF (FERR(1)) THEN
           PRINTLN "Error opening file"
           END
       ENDIF
       FGET 1,s
       WHILE (!FERR(1)) DO
           PRINTLN s
           FGET 1,s
       ENDWHILE
       FCLOSE 1

  **See Also**
    * :PPL:`FOPEN` – Open file
    * :PPL:`FCLOSE` – Close file
    * :PPL:`FGET` – Read from file

FILEINF (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION <VARIANT> FILEINF(STRING file, INTEGER item)`

  Access a piece of information about a file.

  **Parameters**
    * :PPL:`file` – Path and filename to query
    * :PPL:`item` – Information selector (1-9)

  **Returns**
    Varies by item:
    * 1: BOOLEAN (TRUE if exists)
    * 2: DATE (file date stamp)
    * 3: TIME (file time stamp)
    * 4: INTEGER (size in bytes)
    * 5: INTEGER (DOS attribute bits)
    * 6-9: STRING (drive/path/name/extension)

  **Remarks**
    Multi-purpose file information function. Items 6-9 parse the file specification into
    components. Item 1 duplicates EXIST() functionality.

  **Example**

    .. code-block:: PPL

       STRING file
       INPUT "File",file
       IF (FILEINF(file,1)) THEN
           PRINTLN "Size: ",FILEINF(file,4)," bytes"
           PRINTLN "Date: ",FILEINF(file,2)
       ENDIF

  **See Also**
    * :PPL:`EXIST()` – Check file existence
    * :PPL:`DELETE` – Remove file

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


FMTCC (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION STRING FMTCC(STRING ccnum)`

  Formats a credit card number for display purposes.

  **Parameters**
    * :PPL:`ccnum` – Credit card number string

  **Returns**
    Formatted string with spaces: 13 digits as "XXXX XXX XXX XXX", 
    15 as "XXXX XXXXXX XXXXX", 16 as "XXXX XXXX XXXX XXXX", or unchanged if other length.

  **Remarks**
    Adds spacing for standard credit card display formats based on length.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "CC #",s
       IF (VALCC(s)) PRINTLN CCTYPE(s)," - ",FMTCC(s)

  **See Also**
    * :PPL:`CCTYPE()` – Identify card type
    * :PPL:`VALCC()` – Validate credit card

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

GETDRIVE (3.20)
~~~~~~~~~~~~~~~

  :PPL:`FUNCTION INTEGER GETDRIVE()`

  **Parameters**
    None

  **Returns**
    * :PPL:`INTEGER` – Current “drive number”  
      (A:=0, B:=1, C:=2, …). On non-DOS systems mapping is virtual.

  **Description**
    Returns the logical drive index. Primarily legacy; on modern platforms the value may be synthesized.

  **Example**

    .. code-block:: PPL

       INTEGER d
       d = GETDRIVE()
       IF (d = 2) PRINTLN "Drive C: is current"

GETENV (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING GETENV(STRING name)`

  Access the value of an environment variable.

  **Parameters**
    * :PPL:`name` – Environment variable name

  **Returns**
    Value of the environment variable, or empty string if not set.

  **Remarks**
    Returns the value of any environment variable that was set when the BBS was started. 
    Useful for accessing system paths and configuration values.

  **Example**

    .. code-block:: PPL

       STRING path
       LET path = GETENV("PATH")
       PRINTLN "System PATH: ", path

  **See Also**
    * :PPL:`PCBDAT()` – Get PCBoard data directory

GETX (1.00)
~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER GETX()`

  Report the X coordinate (column) of the cursor on screen.

  **Returns**
    Current cursor column (1-80).

  **Remarks**
    Queries the ANSI emulator for the cursor's horizontal position. Useful for saving 
    cursor position or maintaining column while changing rows.

  **Example**

    .. code-block:: PPL

       INTEGER x, y
       x = GETX()
       y = GETY()
       ANSIPOS 1, 23
       PRINTLN "Status line"
       ANSIPOS x, y  ; Restore position

  **See Also**
    * :PPL:`GETY()` – Get cursor row
    * :PPL:`ANSIPOS` – Set cursor position

GETY (1.00)
~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER GETY()`

  Report the Y coordinate (row) of the cursor on screen.

  **Returns**
    Current cursor row (1-23).

  **Remarks**
    Queries the ANSI emulator for the cursor's vertical position. Useful for saving 
    cursor position or maintaining row while changing columns.

  **Example**

    .. code-block:: PPL

       IF (GETY() >= 23) THEN
           CLS  ; Screen full, clear it
       ENDIF

  **See Also**
    * :PPL:`GETX()` – Get cursor column
    * :PPL:`ANSIPOS` – Set cursor 

GRAFMODE (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING GRAFMODE()`

  Report the graphics mode in use.

  **Returns**
    Single character: "N" (none), "A" (ANSI positioning only), "G" (full ANSI graphics), 
    or "R" (RIPscrip).

  **Remarks**
    Returns the current user's graphics capability level for conditional display logic.

  **Example**

    .. code-block:: PPL

       IF (GRAFMODE() = "R") THEN
           PRINTLN "RIPscrip Graphics Supported"
       ELSE IF (GRAFMODE() = "G") THEN
           PRINTLN "Full ANSI Graphics"
       ELSE IF (GRAFMODE() = "A") THEN
           PRINTLN "ANSI positioning only"
       ELSE
           PRINTLN "No graphics"
       ENDIF

  **See Also**
    * :PPL:`ANSION()` – Check ANSI availability
    * :PPL:`ANSIPOS` – Position cursor

HELPPATH (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING HELPPATH()`

  Return the path of help files as defined in PCBSetup.

  **Returns**
    Path to system help files directory.

  **Remarks**
    Returns the help files location for adding system help capabilities to your PPE applications.

  **Example**

    .. code-block:: PPL

       DISPFILE HELPPATH()+"HLPR", GRAPH+LANG+SEC

  **See Also**
    * :PPL:`PPEPATH()` – Get PPE files path
    * :PPL:`SLPATH()` – Get security levels path  
    * :PPL:`TEMPPATH()` – Get temporary files path

HOUR (1.00)
~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER HOUR(TIME t)`

  Extract the hour from a specified time of day.

  **Parameters**
    * :PPL:`t` – Time value

  **Returns**
    Hour component (0-23).

  **Remarks**
    Extracts the hour component from any time value for use in time-based logic.

  **Example**

    .. code-block:: PPL

       PRINTLN "The hour is ",HOUR(TIME())

  **See Also**
    * :PPL:`MIN()` – Extract minutes
    * :PPL:`SEC()` – Extract seconds
    * :PPL:`TIME()` – Get current time

I2S (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION STRING I2S(INTEGER value, INTEGER base)`

  Convert an integer to a string in a specified number base.

  **Parameters**
    * :PPL:`value` – Integer to convert
    * :PPL:`base` – Target base (2-36)

  **Returns**
    String representation of value in specified base.

  **Remarks**
    Converts numbers to any base from binary (2) to base-36. Useful for displaying 
    hex, octal, or binary values. I2S(10,2) returns "1010"; I2S(35,36) returns "Z".

  **Example**

    .. code-block:: PPL

       INTEGER num
       INPUTINT "Enter a number",num,@X0E
       PRINTLN "Binary: ",I2S(num,2)
       PRINTLN "Hex: ",I2S(num,16)

  **See Also**
    * :PPL:`S2I()` – Parse string to integer

I2BD (3.20)
~~~~~~~~~~~
  :PPL:`FUNCTION BIGSTR I2BD(INTEGER value)`

  **Parameters**
    * :PPL:`value` – integer to serialize

  **Returns**
    * :PPL:`BIGSTR` – 8 raw bytes representing a “bdreal” (double) form

  **Description**
    Converts a PPL INTEGER into an 8-byte BASIC double binary image.

  **Example**

    .. code-block:: PPL

       BIGSTR  raw
       INTEGER v

       v   = 12345
       raw = I2BD(v)
       FOPEN 1,"double.bin",O_WR,S_DN
       FWRITE 1,raw,8
       FCLOSE 1

INKEY (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION STRING INKEY()`

  Get the next key input.

  **Returns**
    Single character for displayable keys or named string for special keys 
    (e.g., "UP", "DOWN", "F1", "SHIFT-F1"). Empty if no key available.

  **Remarks**
    Non-blocking key read. Returns special key names for function keys and cursor 
    movement when ANSI or DOORWAY sequences detected. Reads from both remote and 
    local input. Many function keys may be reserved by the BBS.

  **Example**

    .. code-block:: PPL

       STRING key
       WHILE (key <> CHR(27)) DO
           LET key = INKEY()
           IF (key <> "") THEN
               IF (LEFT(key,5) = "SHIFT") THEN
                   PRINTLN "Shifted key: ",key
               ELSE
                   PRINTLN "Key pressed: ",key
               ENDIF
           ENDIF
       ENDWHILE

  **See Also**
    * :PPL:`KINKEY()` – Blocking key read
    * :PPL:`TINKEY()` – Timed key input

INSTR (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER INSTR(BIGSTR str, STRING search)`

  Find the position of one string within another string.

  **Parameters**
    * :PPL:`str` – Source text to search in
    * :PPL:`search` – Substring to find

  **Returns**
    1-based position of first occurrence, or 0 if not found.

  **Remarks**
    Searches for substring within a string. Position 1 is the first character. 
    Case-sensitive search; use UPPER() or LOWER() for case-insensitive matching.

  **Example**

    .. code-block:: PPL

       STRING s
       WHILE (INSTR(UPPER(s),"QUIT") = 0) DO
           INPUTTEXT "Enter string",s,@X0E,40
           PRINTLN s
       ENDWHILE

  **See Also**
    * :PPL:`LEN()` – Get string length
    * :PPL:`MID()` – Extract substring
    * :PPL:`REPLACE()` – Replace substring

KINKEY (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING KINKEY()`

  Get the next key input from the local keyboard only.

  **Returns**
    Single character for displayable keys or named string for special keys 
    (e.g., "UP", "F1", "SHIFT-F1", "CTRL-A", "ALT-X"). Empty if no key available.

  **Remarks**
    Non-blocking local keyboard read. Returns special key names for function keys and 
    cursor movement. Only reads from local console, not remote users. Many function keys 
    may be reserved by the BBS.

  **Example**

    .. code-block:: PPL

       STRING key
       WHILE (key <> CHR(27)) DO
           LET key = KINKEY()
           IF (key <> "") THEN
               IF (LEFT(key,5) = "SHIFT") THEN
                   PRINTLN "Shifted key: ",key
               ELSEIF (LEFT(key,4) = "CTRL") THEN
                   PRINTLN "Control key: ",key
               ELSEIF (LEFT(key,3) = "ALT") THEN
                   PRINTLN "Alt key: ",key
               ELSE
                   PRINTLN "Key: ",key
               ENDIF
           ENDIF
       ENDWHILE

  **See Also**
    * :PPL:`INKEY()` – Read from both local and remote
    * :PPL:`TINKEY()` – Timed key input

LANGEXT (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING LANGEXT()`

  Get the file extension for the current language.

  **Returns**
    ".XXX" formatted extension where XXX is 1-3 characters based on current language.

  **Remarks**
    Returns the file extension used for language-specific files, allowing you to create 
    your own multi-language filename schemes.

  **Example**

    .. code-block:: PPL

       PRINTLN "Language extension: ",LANGEXT()
       DISPFILE "WELCOME"+LANGEXT(), GRAPH+LANG

  **See Also**
    * :PPL:`LANG` – Language display flag constant

LEFT (1.00)
~~~~~~~~~~~
  :PPL:`FUNCTION BIGSTR LEFT(BIGSTR str, INTEGER count)`

  Access the leftmost characters from a string.

  **Parameters**
    * :PPL:`str` – Source string
    * :PPL:`count` – Number of characters to extract

  **Returns**
    Leftmost characters. If count > length, result is padded with spaces. 
    If count ≤ 0, returns empty string.

  **Remarks**
    Extracts substring from the beginning. Useful for fixed-width field processing 
    and text formatting.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "Enter text",s
       PRINTLN "First 10 chars: '",LEFT(s,10),"'"

  **See Also**
    * :PPL:`MID()` – Extract from middle
    * :PPL:`RIGHT()` – Extract from end

LEN (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER LEN(BIGSTR str)`

  Access the length of a string.

  **Parameters**
    * :PPL:`str` – String to measure

  **Returns**
    Number of characters (0-256 for STRING, larger for BIGSTR).

  **Remarks**
    Returns the character count of any string expression.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "Enter text",s
       PRINTLN "Length: ",LEN(s)," characters"

  **See Also**
    * :PPL:`INSTR()` – Find substring position
    * :PPL:`SPACE()` – Create string of spaces

LOGGEDON (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN LOGGEDON()`

  Determine if a user has completely logged on to the BBS.

  **Returns**
    TRUE if the user has completed logging in, FALSE otherwise.

  **Remarks**
    Some PPL features (user variables, CALLNUM) are unavailable until login completes. 
    Use this to check if these features are accessible.

  **Example**

    .. code-block:: PPL

       IF (!LOGGEDON()) LOG "User not logged on",0

  **See Also**
    * :PPL:`CALLNUM()` – Get caller number
    * :PPL:`ONLOCAL()` – Check if local session
    * :PPL:`U_LOGONS()` – Get logon count
LOWER (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION BIGSTR LOWER(BIGSTR str)`

  Converts uppercase characters in a string to lowercase.

  **Parameters**
    * :PPL:`str` – String to convert

  **Returns**
    String with all uppercase characters converted to lowercase.

  **Remarks**
    Useful for case-insensitive string comparisons and formatting. LOWER("STRING") 
    returns "string".

  **Example**

    .. code-block:: PPL

       STRING s
       WHILE (UPPER(s) <> "QUIT") DO
           INPUT "Text",s
           PRINTLN LOWER(s)
       ENDWHILE

  **See Also**
    * :PPL:`UPPER()` – Convert to uppercase

LTRIM (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION BIGSTR LTRIM(BIGSTR str, STRING charSet)`

  Trim specified characters from the left end of a string.

  **Parameters**
    * :PPL:`str` – String to trim
    * :PPL:`charSet` – Character(s) to remove from left

  **Returns**
    String with leading characters from charSet removed.

  **Remarks**
    Strips any characters found in charSet from the beginning of str. Commonly used 
    to remove leading spaces or other formatting characters.

  **Example**

    .. code-block:: PPL

       STRING s
       LET s = "   TEST   "
       PRINTLN LTRIM(s," ")  ; Prints "TEST   "

  **See Also**
    * :PPL:`RTRIM()` – Trim from right
    * :PPL:`TRIM()` – Trim from both ends

MASK_... (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING MASK_...()`

  Return a string for use as a valid character mask.

  **Returns**
    String containing valid characters for input validation.

  **Available Functions**
    * :PPL:`MASK_ALNUM()` – Returns A-Z, a-z, 0-9
    * :PPL:`MASK_ALPHA()` – Returns A-Z, a-z
    * :PPL:`MASK_ASCII()` – Returns all printable ASCII (32-126)
    * :PPL:`MASK_FILE()` – Returns valid filename characters
    * :PPL:`MASK_NUM()` – Returns 0-9
    * :PPL:`MASK_PATH()` – Returns valid pathname characters
    * :PPL:`MASK_PWD()` – Returns valid password characters

  **Remarks**
    Provides standard character sets for INPUTSTR and PROMPTSTR validation. 
    Use these instead of manually defining character sets for consistency.

  **Example**

    .. code-block:: PPL

       INTEGER i
       STRING s
       INPUTSTR "Enter a number from 0 to 1000",i,@X0E,4,MASK_NUM(),DEFS
       PROMPTSTR 148,s,12,MASK_PWD(),ECHODOTS
       INPUTSTR "Enter your comment",s,@X0E,60,MASK_ASCII(),DEFS

  **See Also**
    * :PPL:`INPUTSTR` – Get validated string input
    * :PPL:`PROMPTSTR` – Prompt at screen position

MAXNODE (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER MAXNODE()`

  Determine how many nodes a system may have.

  **Returns**
    Maximum number of nodes licensed for the system.

  **Remarks**
    Returns the node limit configured for the BBS. Used for multi-node operations 
    like broadcasting messages or checking node status.

  **Example**

    .. code-block:: PPL

       INTEGER i
       FOR i = 1 TO MAXNODE()
           RDUNET i
           IF (UN_STAT() = "A") THEN
               PRINTLN "Node ",i," is available"
           ENDIF
       NEXT

  **See Also**
    * :PPL:`PCBNODE()` – Get current node number

MGETBYTE (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER MGETBYTE()`

  Get the next byte input from the modem.

  **Returns**
    Byte value (0-255) from modem buffer, or -1 if empty.

  **Remarks**
    Bypasses PCBoard's normal string filtering to access raw incoming bytes. 
    Use CHR() to convert values to characters if needed.

  **Example**

    .. code-block:: PPL

       INTEGER byte
       WHILE (byte <> 27) DO
           LET byte = MGETBYTE()
           IF (byte >= 0) PRINTLN "Byte value: ",byte
       ENDWHILE

  **See Also**
    * :PPL:`INKEY()` – Filtered key input
    * :PPL:`MINKEY()` – Modem-only key input

MID (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION BIGSTR MID(BIGSTR str, INTEGER pos, INTEGER count)`

  Access any substring of a string.

  **Parameters**
    * :PPL:`str` – Source string
    * :PPL:`pos` – Starting position (1-based)
    * :PPL:`count` – Number of characters to extract

  **Returns**
    Substring from position. Pads with spaces if pos/count exceed bounds.
    Empty if count ≤ 0.

  **Remarks**
    Extracts characters from any position. Position < 1 or > length adds padding spaces.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "Enter text",s
       PRINTLN "Middle 5 chars: '",MID(s,3,5),"'"

  **See Also**
    * :PPL:`LEFT()` – Extract from start
    * :PPL:`RIGHT()` – Extract from end

MIN (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER MIN(TIME t)`

  Extract the minute of the hour from a specified time.

  **Parameters**
    * :PPL:`t` – Time value

  **Returns**
    Minute component (0-59).

  **Remarks**
    Extracts the minute component from any time value for time-based logic.

  **Example**

    .. code-block:: PPL

       PRINTLN "The minute is ",MIN(TIME())

  **See Also**
    * :PPL:`HOUR()` – Extract hour
    * :PPL:`SEC()` – Extract seconds
    * :PPL:`TIME()` – Get current time

MINKEY (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING MINKEY()`

  Get the next key input from the modem only.

  **Returns**
    Single character or special key name (e.g., "F1", "SHIFT-F1"). Empty if no key.

  **Remarks**
    Non-blocking modem-only input. Returns special names for function keys detected 
    via ESC sequences or DOORWAY codes. Ignores local keyboard.

  **Example**

    .. code-block:: PPL

       STRING key
       WHILE (key <> CHR(27)) DO
           LET key = MINKEY()
           IF (key <> "") PRINTLN "Remote user pressed: ",key
       ENDWHILE

  **See Also**
    * :PPL:`INKEY()` – Both local and remote
    * :PPL:`KINKEY()` – Local keyboard only
    * :PPL:`MGETBYTE()` – Raw byte input

MINLEFT (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER MINLEFT()`

  Return the user's minutes left.

  **Returns**
    Minutes remaining in session or today (depends on system configuration).

  **Remarks**
    Check time remaining before allowing time-consuming operations. Value depends 
    on whether SysOp enforces daily or per-session limits.

  **Example**

    .. code-block:: PPL

       IF (MINLEFT() > 10) THEN
           KBDSTUFF "D"+CHR(13)
       ELSE
           PRINTLN "Sorry, not enough time left to download"
       ENDIF

  **See Also**
    * :PPL:`MINON()` – Minutes used
    * :PPL:`ADJTIME()` – Adjust time remaining

MINON (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER MINON()`

  Return the user's minutes online.

  **Returns**
    Minutes used this session.

  **Remarks**
    Always returns session time regardless of daily limit configuration. 
    Use to restrict features until minimum session time reached.

  **Example**

    .. code-block:: PPL

       IF (MINON() < 10) THEN
           PRINTLN "Please stay online 10 minutes before downloading"
       ENDIF

  **See Also**
    * :PPL:`MINLEFT()` – Minutes remaining
    * :PPL:`U_TIMEON()` – User time variable

MODEM (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION STRING MODEM()`

  Access the connect string as reported by the modem.

  **Returns**
    Modem connect string (e.g., "CONNECT 9600/ARQ/V32").

  **Remarks**
    Returns the full connect string including speed, error correction, and 
    compression info if reported by modem.

  **Example**

    .. code-block:: PPL

       FAPPEND 1,"MODEM.LOG",O_WR,S_DW
       FPUTLN 1,LEFT(U_NAME(),30)+" "+MODEM()
       FCLOSE 1

  **See Also**
    * :PPL:`CALLID()` – Caller ID info
    * :PPL:`CARRIER()` – Connection speed

MONTH (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER MONTH(DATE d)`

  Extracts the month of the year from a specified date.

  **Parameters**
    * :PPL:`d` – Date value

  **Returns**
    Month of year (1-12).

  **Remarks**
    Extracts the month component from any date value for use in calculations or display.

  **Example**

    .. code-block:: PPL

       PRINTLN "This month is: ",MONTH(DATE())

  **See Also**
    * :PPL:`DATE()` – Get current date
    * :PPL:`DAY()` – Extract day of month
    * :PPL:`DOW()` – Day of week
    * :PPL:`YEAR()` – Extract year

MKDATE (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION DATE MKDATE(INTEGER year, INTEGER month, INTEGER day)`

  **Returns**
    Constructed date (invalid inputs may produce undefined / sentinel).

MONTH (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER MONTH(DATE d)`

  **Returns**
    Month (1–12).

NOCHAR (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING NOCHAR()`

  Get the no response character for the current language.

  **Returns**
    "No" character for current language (e.g., "N" for English).

  **Remarks**
    Returns language-specific negative response character for internationalization. 
    Use instead of hardcoding "N" for multi-language support.

  **Example**

    .. code-block:: PPL

       STRING ans
       LET ans = YESCHAR()
       INPUTSTR "Run program now",ans,@X0E,1,"",AUTO+YESNO
       IF (ans = NOCHAR()) END

  **See Also**
    * :PPL:`YESCHAR()` – Get "yes" character
    * :PPL:`YESNO` – Yes/no input flag constant

NOT (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER NOT(INTEGER value)`

  Calculate the bitwise NOT of an integer argument.

  **Parameters**
    * :PPL:`value` – Integer to invert

  **Returns**
    Bitwise NOT of value (all bits toggled).

  **Remarks**
    Inverts all bits: set bits become clear, clear bits become set. 
    Useful for toggling flags or inverting bitmasks.

  **Example**

    .. code-block:: PPL

       PRINTLN NOT(0x1248)  ; Toggle the bits
       ; Toggle all flags
       INTEGER flag
       LET flag = NOT(flag)

  **See Also**
    * :PPL:`AND()` – Bitwise AND
    * :PPL:`OR()` – Bitwise OR
    * :PPL:`XOR()` – Bitwise XOR

ONLOCAL (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN ONLOCAL()`

  Determine whether or not a caller is on locally.

  **Returns**
    TRUE if the caller is logged on locally, FALSE for remote connection.

  **Remarks**
    Check if user is at the local console vs. remote modem/network connection. 
    Use to handle features that differ for local vs. remote users (file transfers, 
    modem operations).

  **Example**

    .. code-block:: PPL

       IF (ONLOCAL()) THEN
           PRINTLN "Call back verification cannot be performed for"
           PRINTLN "users logged in locally!"
           END
       ENDIF
       CALL "CALLBACK.PPE"

  **See Also**
    * :PPL:`CALLNUM()` – Get caller number
    * :PPL:`LOGGEDON()` – Check if logged in
    * :PPL:`CARRIER()` – Connection speed

OR (1.00)
~~~~~~~~~
  :PPL:`FUNCTION INTEGER OR(INTEGER value1, INTEGER value2)`

  Calculate the bitwise OR of two integer arguments.

  **Parameters**
    * :PPL:`value1` – First integer operand
    * :PPL:`value2` – Second integer operand

  **Returns**
    Bitwise OR of the two values.

  **Remarks**
    Result bit is 1 if either corresponding bit in the operands is 1. 
    Use to set specific bits by ORing with a mask (1s for bits to set, 0s to preserve).

  **Example**

    .. code-block:: PPL

       ; Set bits in the low byte
       PRINTLN OR(0x1248, 0x00FF)
       ; Randomly set a flag
       INTEGER flag
       LET flag = OR(RANDOM(1), RANDOM(1))

  **See Also**
    * :PPL:`AND()` – Bitwise AND
    * :PPL:`NOT()` – Bitwise NOT
    * :PPL:`XOR()` – Bitwise XOR

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

PAGESTAT (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN PAGESTAT()`

  Determine if the current user has paged the SysOp.

  **Returns**
    TRUE if the user has paged the SysOp, FALSE otherwise.

  **Remarks**
    Check if user has already attempted to page. Use with PAGEON, PAGEOFF, and CHAT 
    to implement custom operator page functionality.

  **Example**

    .. code-block:: PPL

       IF (PAGESTAT()) THEN
           PRINTLN "You have already paged the SysOp,"
           PRINTLN "please be patient."
       ELSE
           PAGEON
           PRINTLN "The SysOp has been paged, continue"
       ENDIF

  **See Also**
    * :PPL:`CHAT` – Enter chat mode
    * :PPL:`PAGEOFF` – Disable page
    * :PPL:`PAGEON` – Enable page

PCBDAT (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING PCBDAT()`

  Return the path and file name of the PCBOARD.DAT file.

  **Returns**
    Full path to PCBOARD.DAT for current node.

  **Remarks**
    Returns path to master configuration file. Use with READLINE() to extract 
    specific configuration values from this text-based file.

  **Example**

    .. code-block:: PPL

       STRING s
       LET s = READLINE(PCBDAT(),1)
       PRINTLN "PCBOARD.DAT version info: ",s

  **See Also**
    * :PPL:`GETENV()` – Get environment variable
    * :PPL:`READLINE()` – Read file line

PCBNODE (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER PCBNODE()`

  Return the current node number.

  **Returns**
    Node number (1 to maximum licensed nodes).

  **Remarks**
    Returns effective node number for current session. May differ from PCBOARD.DAT 
    value if /FLOAT or /NODE switches used. Useful for creating unique temporary filenames.

  **Example**

    .. code-block:: PPL

       STRING file
       LET file = "TMP"+STRING(PCBNODE())+".$$$"
       DELETE file

  **See Also**
    * :PPL:`MAXNODE()` – Get maximum nodes

PEEKB (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER PEEKB(INTEGER addr)`

  Return the value of a byte at a specified memory address.

  **Parameters**
    * :PPL:`addr` – Memory address to read

  **Returns**
    Byte value (0-255) at address.

  **Remarks**
    Direct memory access for reading system BIOS data or low-level hardware inspection.

  **Example**

    .. code-block:: PPL

       PRINTLN "Video mode: ", PEEKB(MKADDR(0x40, 0x49))

  **See Also**
    * :PPL:`MKADDR()` – Create memory address
    * :PPL:`PEEKW()` – Peek word
    * :PPL:`PEEKDW()` – Peek double word
    * :PPL:`POKEB()` – Poke byte

PEEKDW (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER PEEKDW(INTEGER addr)`

  Return the value of a double word at a specified memory address.

  **Parameters**
    * :PPL:`addr` – Memory address to read

  **Returns**
    Signed double word value (-2,147,483,648 to +2,147,483,647).

  **Remarks**
    Direct memory access for reading 32-bit system values.

  **Example**

    .. code-block:: PPL

       PRINTLN "Timer ticks: ", PEEKDW(MKADDR(0x40, 0x6C))

  **See Also**
    * :PPL:`MKADDR()` – Create memory address
    * :PPL:`PEEKB()` – Peek byte
    * :PPL:`PEEKW()` – Peek word
    * :PPL:`POKEDW()` – Poke double word

PEEKW (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER PEEKW(INTEGER addr)`

  Return the value of a word at a specified memory address.

  **Parameters**
    * :PPL:`addr` – Memory address to read

  **Returns**
    Word value (0-65,535) at address.

  **Remarks**
    Direct memory access for reading 16-bit system values.

  **Example**

    .. code-block:: PPL

       PRINTLN "Memory size: ", PEEKW(MKADDR(0x40, 0x13))

  **See Also**
    * :PPL:`MKADDR()` – Create memory address
    * :PPL:`PEEKB()` – Peek byte
    * :PPL:`PEEKDW()` – Peek double word
    * :PPL:`POKEW()` – Poke word

PPENAME (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING PPENAME()`

  Return the base name of an executing PPE file.

  **Returns**
    PPE filename without path or extension.

  **Remarks**
    Returns current PPE's base name. Useful for creating matching data files 
    (e.g., CONFIG.CFG for CONFIG.PPE).

  **Example**

    .. code-block:: PPL

       STRING s
       FOPEN 1, PPEPATH()+PPENAME()+".CFG",O_RD,S_DN
       FGET 1,s
       FCLOSE 1

  **See Also**
    * :PPL:`PPEPATH()` – Get PPE path

PPEPATH (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING PPEPATH()`

  Return the path of an executing PPE file.

  **Returns**
    PPE directory path without filename.

  **Remarks**
    Returns current PPE's directory. Use to locate configuration or data files 
    relative to the PPE location.

  **Example**

    .. code-block:: PPL

       FOPEN 1, PPEPATH()+PPENAME()+".CFG",O_RD,S_DN

  **See Also**
    * :PPL:`HELPPATH()` – Get help files path
    * :PPL:`PPENAME()` – Get PPE name
    * :PPL:`SLPATH()` – Get security levels path

PSA (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN PSA(INTEGER num)`

  Determine whether or not a given PSA is installed.

  **Parameters**
    * :PPL:`num` – PSA number (1-6): 1=Alias, 2=Verification, 3=Address, 
      4=Password, 5=Statistics, 6=Notes

  **Returns**
    TRUE if specified PSA (PCBoard Supported Allocation) is installed, FALSE otherwise.

  **Remarks**
    Check availability of optional extended user data areas before accessing them.

  **Example**

    .. code-block:: PPL

       STRING ynStr(1)
       LET ynStr(0) = "NO"
       LET ynStr(1) = "YES"
       PRINTLN "Alias enabled? ", ynStr(PSA(1))
       PRINTLN "Verification enabled? ", ynStr(PSA(2))

  **See Also**
    * :PPL:`VER()` – Get version

RANDOM (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER RANDOM(INTEGER max)`

  Return a random value between 0 and a specified limit.

  **Parameters**
    * :PPL:`max` – Maximum value (inclusive)

  **Returns**
    Pseudo-random integer from 0 to max.

  **Remarks**
    Generates random numbers for games, statistics, or randomized display effects.

  **Example**

    .. code-block:: PPL

       INTEGER x, y
       x = 1 + RANDOM(50)
       y = 1 + RANDOM(22)
       COLOR 1 + RANDOM(14)
       ANSIPOS x, y
       PRINT "Random position!"

  **See Also**
    * :PPL:`ABS()` – Absolute value

READLINE (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING READLINE(STRING file, INTEGER line)`

  Read a specific line number from a text file.

  **Parameters**
    * :PPL:`file` – File path
    * :PPL:`line` – Line number (1-based)

  **Returns**
    Contents of specified line, or empty string if line doesn't exist.

  **Remarks**
    Quick line access without explicit file handling. Caches last file/line for 
    efficient sequential reads. File remains open until PPE exits.

  **Example**

    .. code-block:: PPL

       PRINTLN "System IRQ: ", READLINE(PCBDAT(), 158)
       PRINTLN "Base IO: ", READLINE(PCBDAT(), 159)

  **See Also**
    * :PPL:`EXIST()` – Check file existence
    * :PPL:`FILEINF()` – Get file info
    * :PPL:`PCBDAT()` – Get config file path

REG... (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION <VARIANT> REG...()`

  Get the value of a CPU register.

  **Returns**
    * BOOLEAN for :PPL:`REGCF()` – TRUE if carry flag set
    * INTEGER for all others – Register value

  **Available Functions**
    * :PPL:`REGAH()`, :PPL:`REGAL()`, :PPL:`REGBH()`, :PPL:`REGBL()`, 
      :PPL:`REGCH()`, :PPL:`REGCL()`, :PPL:`REGDH()`, :PPL:`REGDL()` – Byte registers (0-255)
    * :PPL:`REGAX()`, :PPL:`REGBX()`, :PPL:`REGCX()`, :PPL:`REGDX()`, 
      :PPL:`REGDI()`, :PPL:`REGSI()`, :PPL:`REGDS()`, :PPL:`REGES()` – Word registers (0-65,535)
    * :PPL:`REGF()` – Processor flags (Carry, Parity, Auxiliary, Zero, Sign, 
      Trap, Interrupt, Direction, Overflow)
    * :PPL:`REGCF()` – Carry flag only (BOOLEAN)

  **Remarks**
    Read CPU register values after DOINTR() calls. REGF() bit values: 
    Carry=0x0001, Parity=0x0004, Auxiliary=0x0010, Zero=0x0040, Sign=0x0080, 
    Trap=0x0100, Interrupt=0x0200, Direction=0x0400, Overflow=0x0800.

  **Example**

    .. code-block:: PPL

       ‘ Create subdirectory - DOS function 39h
       INTEGER addr
       STRING path
       LET path = "C:\$TMPDIR$" VARADDR path, addr
       DOINTR 21h,39h,0,0,addr%00010000h, 0,0,0,addr/00010000h,0
       IF (REGCF() & (REGAX() = 3)) THEN
       PRINTLN "Error: Path not found"
       ELSE IF (REGCF() & (REGAX() = 5)) THEN
       PRINTLN "Error: Access Denied"
       ELSE IF (REGCF()) THEN
       PRINTLN "Error: Unknown Error"
       ELSE
       PRINTLN "Directory successfully created...”
       ENDIF 

  **See Also**
    * :PPL:`DOINTR` – Execute interrupt
    * :PPL:`MKADDR()` – Make memory address

REPLACE() Function (1.00)
----------------------

Replaces all occurrences of a character in a string with another character.

**Syntax**

.. code-block:: PPL

   REPLACE(str, old, new)

**Parameters**

- ``str`` - String expression to process
- ``old`` - Character to find and replace
- ``new`` - Character to replace with

**Returns**

Returns ``str`` with all occurrences of ``old`` replaced by ``new``.

**Remarks**

Searches a string for a given character and replaces all instances with another character. Useful for text formatting and string manipulation tasks.

**Example**

.. code-block:: PPL

   PRINTLN "Your internet address on this system is:"
   PRINTLN REPLACE(LOWER(U_NAME()), " ", "."), "@clarkdev.com"

**See Also**

[`STRIP()`](#strip-function), [`STRIPATX()`](#stripatx-function)

REPLACESTR() Function (2.00)
----------------------------

Replaces all occurrences of a substring with another substring in a string.

**Syntax**

.. code-block:: PPL

   REPLACESTR(str, search, replace)

**Parameters**

- ``str`` - String expression to process
- ``search`` - Substring to find and replace
- ``replace`` - Substring to replace with

**Returns**

Returns ``str`` with all occurrences of ``search`` replaced by ``replace``.

**Remarks**

Similar to REPLACE() but operates on substrings instead of single characters. Searches a string for all instances of a substring and replaces them with another substring. Useful for text processing and string manipulation tasks.

**Example**

.. code-block:: PPL

   STRING msg
   LET msg = "Hello World! World is great!"
   PRINTLN REPLACESTR(msg, "World", "Universe")
   ' Prints: "Hello Universe! Universe is great!"

**See Also**

[`REPLACE()`](#replace-function), [`STRIPSTR()`](#stripstr-function), [`STRIP()`](#strip-function)


RIGHT() Function (1.00)
-----------------------

Returns the rightmost characters from a string.

**Syntax**

.. code-block:: PPL

   RIGHT(str, chars)

**Parameters**

- ``str`` - String expression to extract from
- ``chars`` - Number of characters to extract from the right end

**Returns**

Returns a string with the rightmost ``chars`` characters of ``str``.

**Remarks**

Returns a substring with the rightmost characters of a specified string. If ``chars`` is ≤ 0, returns an empty string. If ``chars`` exceeds the string length, spaces are added to pad the result.

**Example**

.. code-block:: PPL

   STRING s
   FOPEN 1, "DATA.TXT", O_RD, S_DN
   WHILE (!FERR(1)) DO
      FGET 1, s
      PRINT RTRIM(LEFT(s, 25), " "), " - "
      PRINTLN RIGHT(s, LEN(s) - 25)
   ENDWHILE
   FCLOSE 1

**See Also**

[`LEFT()`](#left-function), [`MID()`](#mid-function)


RTRIM() Function (1.00)
-----------------------

Removes a specified character from the right end of a string.

**Syntax**

.. code-block:: PPL

   RTRIM(str, ch)

**Parameters**

- ``str`` - String expression to trim
- ``ch`` - Character to strip from the right end

**Returns**

Returns the trimmed string.

**Remarks**

Strips a specified character from the right end of a string and returns the trimmed result. Commonly used to remove trailing spaces or other characters.

**Example**

.. code-block:: PPL

   STRING s
   LET s = " TEST "
   PRINTLN RTRIM(s, " ")  ' Will print " TEST"

**See Also**

[`LTRIM()`](#ltrim-function), [`TRIM()`](#trim-function)


S2I() Function (1.00)
---------------------

Converts a string in a specified number base to an integer.

**Syntax**

.. code-block:: PPL

   S2I(str, base)

**Parameters**

- ``str`` - String expression to convert
- ``base`` - Number base (2 through 36) to convert from

**Returns**

Returns ``str`` converted from the specified number base to an integer.

**Remarks**

Converts a string in any number base from 2 to 36 to an integer. For example, ``S2I("1010", 2)`` returns 10; ``S2I("Z", 36)`` returns 35. Useful for parsing numbers stored in non-decimal formats.

**Example**

.. code-block:: PPL

   INTEGER i
   STRING s
   INPUTTEXT "Enter a string (any base)", s, 0X0E, 40
   FOR i = 2 TO 36
      PRINTLN s, " = ", S2I(s, i), " base ", i
   NEXT

**See Also**

[`I2S()`](#i2s-function)


SCRTEXT() Function (1.00)
-------------------------

Reads text and attribute information directly from screen memory.

**Syntax**

.. code-block:: PPL

   SCRTEXT(x, y, len, color)

**Parameters**

- ``x`` - X coordinate (column) to read from
- ``y`` - Y coordinate (row) to read from
- ``len`` - Length in columns to read
- ``color`` - TRUE to include color codes, FALSE otherwise

**Returns**

Returns the specified region of screen memory as a string.

**Remarks**

Useful for saving portions of screen memory with or without color information. Color information is included as embedded @X codes when ``color`` is TRUE. Due to the 256 character string limit, limit length to 51 characters or less when including color information to avoid exceeding the limit.

**Example**

.. code-block:: PPL

   ' Scroll the screen left 5 columns and down 3 rows
   INTEGER r
   STRING s
   FOR r = 20 TO 1 STEP -1
      LET s = SCRTEXT(6, r, 75, TRUE)
      ANSIPOS 1, r + 3
      CLREOL
      PRINT s
   NEXT

**See Also**

[`INSTR()`](#instr-function), [`LEN()`](#len-function), [`SPACE()`](#space-function)


SEC() Function (1.00)
---------------------

Returns the second component from a time expression.

**Syntax**

.. code-block:: PPL

   SEC(texp)

**Parameters**

- ``texp`` - Time expression

**Returns**

Returns the second of the minute from the specified time (0-59).

**Remarks**

Extracts the second component from a TIME value. Useful for time parsing and display formatting.

**Example**

.. code-block:: PPL

   PRINTLN "The second is ", SEC(TIME())

**See Also**

[`HOUR()`](#hour-function), [`MIN()`](#min-function), [`TIME()`](#time-function)


SHOWSTAT() Function (1.00)
--------------------------

Determines if data is being displayed on the screen.

**Syntax**

.. code-block:: PPL

   SHOWSTAT()

**Returns**

Returns TRUE if data is being shown on the display, FALSE otherwise.

**Remarks**

Determines the current display status. Used with OPENCAP, CLOSECAP, SHOWON, and SHOWOFF statements to control screen output and capture operations. Useful for automating features while capturing output to files.

**Example**

.. code-block:: PPL

   BOOLEAN ss
   LET ss = SHOWSTAT()
   SHOWOFF
   OPENCAP "CAP" + STRING(PCBNODE()), ocFlag
   IF (ocFlag) THEN
      DIR "U;NS"
      CLOSECAP
      KBDSTUFF "FLAG CAP" + STRING(PCBNODE()) + CHR(13)
   ENDIF
   IF (ss) THEN SHOWON ELSE SHOWOFF

**See Also**

[`CLOSECAP`](#closecap-statement), [`OPENCAP`](#opencap-statement), [`SHOWOFF`](#showoff-statement), [`SHOWON`](#showon-statement)


SLPATH() Function (1.00)
------------------------

Returns the path of login security files as defined in PCBSetup.

**Syntax**

.. code-block:: PPL

   SLPATH()

**Returns**

Returns the path of the PCBoard login security files.

**Remarks**

Returns the path where login security files are located as defined in PCBSetup. Can be used to create and modify security files dynamically.

**Example**

.. code-block:: PPL

   FAPPEND 1, SLPATH() + STRING(CURSEC()), O_WR, S_DB
   FPUTLN 1, U_NAME()
   FCLOSE 1

**See Also**

[`HELPPATH()`](#helppath-function), [`PPEPATH()`](#ppepath-function), [`TEMPPATH()`](#temppath-function)


SPACE() Function (1.00)
-----------------------

Creates a string with a specified number of spaces.

**Syntax**

.. code-block:: PPL

   SPACE(len)

**Parameters**

- ``len`` - Number of spaces for the new string (0-256)

**Returns**

Returns a string of ``len`` spaces.

**Remarks**

Creates a string of the specified length filled with spaces. Useful for formatting screen displays and writing formatted data to files.

**Example**

.. code-block:: PPL

   PRINT RANDOM(9), SPACE(5), RANDOM(9), SPACE(5), RANDOM(9)

**See Also**

[`INSTR()`](#instr-function), [`LEN()`](#len-function), [`STRING()`](#string-function)


STRING() Function (1.00)
------------------------

Converts any expression to a string.

**Syntax**

.. code-block:: PPL

   STRING(exp)

**Parameters**

- ``exp`` - Any expression

**Returns**

Returns ``exp`` formatted as a string.

**Remarks**

Converts any expression to string format. Useful when appending non-string types to strings via the + operator, forcing compatible type addition. PPL automatically converts incompatible types when possible, so this function is mainly needed for explicit string concatenation.

**Example**

.. code-block:: PPL

   INTEGER i
   STRING s(5)
   FOR i = 1 TO 5
      LET s(i) = "This is string " + STRING(i)
   NEXT

**See Also**

[`I2S()`](#i2s-function), [`S2I()`](#s2i-function)


STRIP() Function (1.00)
-----------------------

Removes all occurrences of a character from a string.

**Syntax**

.. code-block:: PPL

   STRIP(str, ch)

**Parameters**

- ``str`` - String expression to process
- ``ch`` - Character to remove from ``str``

**Returns**

Returns ``str`` with all occurrences of ``ch`` removed.

**Remarks**

Strips all instances of a selected character from a string. Useful for removing formatting characters, such as slashes and hyphens from date strings.

**Example**

.. code-block:: PPL

   STRING s
   INPUTSTR "Enter date (MM-DD-YY)", s, 0X0E, 8, "0123456789-", DEFS
   LET s = STRIP(s, "-")
   PRINTLN "Date (MMDDYY): ", s

**See Also**

[`REPLACE()`](#replace-function), [`STRIPATX()`](#stripatx-function)


STRIPATX() Function (1.00)
--------------------------

Removes @X color codes from a string.

**Syntax**

.. code-block:: PPL

   STRIPATX(sexp)

**Parameters**

- ``sexp`` - String expression

**Returns**

Returns ``sexp`` with all @X codes removed.

**Remarks**

Strips PCBoard @X color codes from a string. Useful for logging information to files without the @X codes used in screen display.

**Example**

.. code-block:: PPL

   STRING Question, Answer
   LET Question = "@X0EWhat is your street address?"
   PRINTLN Question
   INPUT "", Answer
   FPUTLN 0, "Q: ", STRIPATX(Question)
   FPUTLN 0, "A: ", Answer

**See Also**

[`REPLACE()`](#replace-function), [`STRIP()`](#strip-function)

STRIPSTR() Function (2.00)
--------------------------

Removes all occurrences of a substring from a string.

**Syntax**

.. code-block:: PPL

   STRIPSTR(str, search)

**Parameters**

- ``str`` - String expression to process
- ``search`` - Substring to remove from ``str``

**Returns**

Returns ``str`` with all occurrences of ``search`` removed.

**Remarks**

Similar to STRIP() but operates on substrings instead of single characters. Removes all instances of a specified substring from a string. Useful for cleaning up strings by removing multi-character patterns.

**Example**

.. code-block:: PPL

   STRING filename
   LET filename = "document_backup_old_backup.txt"
   PRINTLN STRIPSTR(filename, "_backup")
   ' Prints: "document_old.txt"

**See Also**

[`STRIP()`](#strip-function), [`REPLACESTR()`](#replacestr-function), [`REPLACE()`](#replace-function)

SYSOPSEC() Function (1.00)
--------------------------

Returns the SysOp security level as defined in PCBSetup.

**Syntax**

.. code-block:: PPL

   SYSOPSEC()

**Returns**

Returns the SysOp security level.

**Remarks**

Returns the configured SysOp security level from PCBSetup. Useful for limiting functionality in PPL applications to users with security levels greater than or equal to the SysOp level.

**Example**

.. code-block:: PPL

   INTEGER min
   IF (CURSEC() >= SYSOPSEC()) THEN
      LET min = 60
   ELSE
      LET min = 5
   ENDIF
   ADJTIME min

**See Also**

[`CURSEC()`](#cursec-function)


TEMPPATH() Function (1.00)
--------------------------

Returns the path to the temporary work directory as defined in PCBSetup.

**Syntax**

.. code-block:: PPL

   TEMPPATH()

**Returns**

Returns the path of the node temporary work files area.

**Remarks**

Returns the path for temporary work files as defined in PCBSetup. Often points to a RAM drive or other fast local storage, making it ideal for small temporary files that need not be kept permanently.

**Example**

.. code-block:: PPL

   INTEGER rc
   SHELL TRUE, rc, "DIR", ">" + TEMPPATH() + "TMPDIR"
   DISPFILE TEMPPATH() + "TMPDIR", DEFS
   DELETE TEMPPATH() + "TMPDIR"

**See Also**

[`HELPPATH()`](#helppath-function), [`PPEPATH()`](#ppepath-function), [`SLPATH()`](#slpath-function)


TIME() Function (1.00)
----------------------

Returns the current time.

**Syntax**

.. code-block:: PPL

   TIME()

**Returns**

Returns the current time.

**Remarks**

Returns the current time represented internally as seconds elapsed since midnight (0-86399). 00:00:00 has a value of 0, 00:01:00 has a value of 60, 01:00:00 has a value of 3600, and 23:59:59 has a value of 86399. May be displayed, stored, or assigned to an integer for arithmetic operations.

**Example**

.. code-block:: PPL

   PRINTLN "The time is ", TIME()

**See Also**

[`DATE()`](#date-function), [`HOUR()`](#hour-function), [`MIN()`](#min-function), [`SEC()`](#sec-function), [`TIMEAP()`](#timeap-function)


TIMEAP() Function (1.00)
------------------------

Converts a time value to 12-hour AM/PM format.

**Syntax**

.. code-block:: PPL

   TIMEAP(texp)

**Parameters**

- ``texp`` - Time expression

**Returns**

Returns a string formatted in 12-hour AM/PM format (HH:MM:SS XM).

**Remarks**

TIME values default to military time format ("HH:MM:SS"). This function converts them to 12-hour AM/PM format, where HH = hour, MM = minute, SS = second, and X = A or P.

**Example**

.. code-block:: PPL

   PRINTLN "The current time is ", TIMEAP(TIME())

**See Also**

[`TIME()`](#time-function)

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


TOKCOUNT() Function (1.00)
--------------------------

Returns the number of tokens pending.

**Syntax**

.. code-block:: PPL

   TOKCOUNT()

**Returns**

Returns the number of tokens available.

**Remarks**

Returns the number of tokens available via GETTOKEN statement and GETTOKEN() function. The count decrements after each token retrieval until reaching 0. TOKENIZE overwrites pending tokens and reinitializes the count; TOKENSTR() clears the count to 0 and returns all tokens.

**Example**

.. code-block:: PPL

   PRINTLN "There are ", TOKCOUNT(), " tokens"
   WHILE (TOKCOUNT() > 0)
      PRINTLN GETTOKEN()
   ENDWHILE

**See Also**

[`GETTOKEN`](#gettoken-statement), [`GETTOKEN()`](#gettoken-function), [`TOKENIZE`](#tokenize-statement), [`TOKENSTR()`](#tokenstr-function)


TOKENSTR() Function (1.00)
--------------------------

Rebuilds and returns a previously tokenized string.

**Syntax**

.. code-block:: PPL

   TOKENSTR()

**Returns**

Returns the rebuilt string with semi-colon separators between tokens.

**Remarks**

Takes all pending tokens and builds a string with semi-colon separators. For example, tokens "R", "A", and "S" are returned as "R;A;S". Regardless of the original separator, semi-colons are always used in the rebuilt string.

**Example**

.. code-block:: PPL

   STRING cmdline
   INPUT "Command", cmdline
   TOKENIZE cmdline
   PRINTLN "You entered ", TOKCOUNT(), " tokens"
   PRINTLN "Original string: ", cmdline
   PRINTLN " TOKENSTR(): ", TOKENSTR()

**See Also**

[`GETTOKEN`](#gettoken-statement), [`GETTOKEN()`](#gettoken-function), [`TOKCOUNT()`](#tokcount-function), [`TOKENIZE`](#tokenize-statement)


TRIM() Function (1.00)
----------------------

Removes a specified character from both ends of a string.

**Syntax**

.. code-block:: PPL

   TRIM(str, ch)

**Parameters**

- ``str`` - String expression to trim
- ``ch`` - Character to strip from both ends

**Returns**

Returns the trimmed string.

**Remarks**

Strips a specified character from both ends of a string and returns the trimmed result. Commonly used to remove leading and trailing spaces or other characters.

**Example**

.. code-block:: PPL

   STRING s
   LET s = " TEST "
   PRINTLN TRIM(s, " ")  ' Will print "TEST"

**See Also**

[`LTRIM()`](#ltrim-function), [`RTRIM()`](#rtrim-function)



TOKCOUNT (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER TOKCOUNT()`

  **Returns**
    Remaining token count in current parse buffer.

TOKENSTR (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING TOKENSTR()`

  **Returns**
    Unconsumed token remainder as a string.

TOBIGSTR (2.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION BIGSTR TOBIGSTR(<ANY> value)`

  **Returns**
    :PPL:`value` coerced to BIGSTR.

TOSTRING (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING STRING(<ANY> value)`

  **Returns**
    String form of :PPL:`value` (numbers decimal, BOOLEAN 0/1).

UN_...() Functions (1.00)
-------------------------

Returns information about a node from the USERNET file.

**Syntax**

.. code-block:: PPL

   UN_CITY()
   UN_NAME()
   UN_OPER()
   UN_STAT()

**Returns**

Returns a string with the requested information from the USERNET.XXX file:

- ``UN_CITY()`` - Returns the city field
- ``UN_NAME()`` - Returns the user name field
- ``UN_OPER()`` - Returns the operation text field
- ``UN_STAT()`` - Returns the status field

**Remarks**

These four functions return information from the USERNET file. The information is only meaningful after executing RDUNET for a specific node. Use these in conjunction with RDUNET to read node information and WRUNET to write node information.

**Example**

.. code-block:: PPL

   RDUNET PCBNODE()
   WRUNET PCBNODE(), UN_STAT(), UN_NAME(), UN_CITY(), "Running " + PPENAME(), ""
   
   RDUNET 1
   WRUNET 1, UN_STAT(), UN_NAME(), UN_CITY(), UN_OPER(), "Hello there node 1"

**See Also**

[`BROADCAST`](#broadcast-statement), [`RDUNET`](#rdunet-statement), [`WRUNET`](#wrunet-statement)

UPPER (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION BIGSTR UPPER(BIGSTR str)`

  Converts lowercase characters in a string to uppercase.

  **Parameters**
    * :PPL:`str` – String to convert

  **Returns**
    String with all lowercase characters converted to uppercase.

  **Remarks**
    Useful for case-insensitive string comparisons and formatting. UPPER("string") 
    returns "STRING". Essential for comparing user input without regard to case.

  **Example**

    .. code-block:: PPL

       STRING s
       WHILE (UPPER(s) <> "QUIT") DO
           INPUT "Text",s
           PRINTLN LOWER(s)
       ENDWHILE

  **See Also**
    * :PPL:`LOWER()` – Convert to lowercase

U_ADDR (1.00)
~~~~~~~~~~~~~
  :PPL:`STRING ARRAY U_ADDR(INTEGER index)`

  Access the current user's address information.

  **Parameters**
    * :PPL:`index` – Array subscript (0-5):
      * 0: Address Line 1 (50 characters max)
      * 1: Address Line 2 (50 characters max)
      * 2: City (25 characters max)
      * 3: State (10 characters max)
      * 4: ZIP Code (10 characters max)
      * 5: Country (15 characters max)

  **Returns**
    STRING containing the requested address field.

  **Remarks**
    This array is filled with information from the current user's record when GETUSER 
    is executed. Changes can be written back with PUTUSER. The array is empty until 
    GETUSER is processed and changes aren't saved until PUTUSER is processed. The array 
    only has meaningful information if the address PSA is installed. Check with PSA(3).

  **Example**

    .. code-block:: PPL

       IF (PSA(3)) THEN
           GETUSER
           INPUT "Addr 1",U_ADDR(0)
           INPUT "Addr 2",U_ADDR(1)
           INPUT "City  ",U_ADDR(2)
           INPUT "State ",U_ADDR(3)
           INPUT "ZIP   ",U_ADDR(4)
           INPUT "Cntry ",U_ADDR(5)
           PUTUSER
       ENDIF

  **See Also**
    * :PPL:`GETUSER` – Load user record
    * :PPL:`PSA()` – Check PSA availability
    * :PPL:`PUTUSER` – Save user record

U_BDL (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_BDL()`

  Access the total number of bytes downloaded by the current user.

  **Returns**
    Current user's total bytes downloaded.

  **Remarks**
    Returns information useful for modifying PCBoard's built-in ratio management system 
    and the view user information command. Unlike the predefined U_... user variables, 
    this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You have ULed ",U_BUL()," bytes and DLed ",U_BDL()," bytes."

  **See Also**
    * :PPL:`U_BDLDAY()` – Bytes downloaded today
    * :PPL:`U_BUL()` – Bytes uploaded
    * :PPL:`U_FDL()` – Files downloaded
    * :PPL:`U_FUL()` – Files uploaded

U_BDLDAY (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_BDLDAY()`

  Access the number of bytes downloaded by the current user today.

  **Returns**
    Current user's bytes downloaded today.

  **Remarks**
    Returns information useful for modifying PCBoard's built-in ratio management system 
    and the view user information command. Unlike the predefined U_... user variables, 
    this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You have downloaded ",U_BDLDAY()," bytes today."

  **See Also**
    * :PPL:`U_BDL()` – Total bytes downloaded
    * :PPL:`U_BUL()` – Total bytes uploaded
    * :PPL:`U_FDL()` – Total files downloaded
    * :PPL:`U_FUL()` – Total files uploaded

U_BUL (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_BUL()`

  Access the total number of bytes uploaded by the current user.

  **Returns**
    Current user's total bytes uploaded.

  **Remarks**
    Returns information useful for modifying PCBoard's built-in ratio management system 
    and the view user information command. Unlike the predefined U_... user variables, 
    this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You have ULed ",U_BUL()," bytes and DLed ",U_BDL()," bytes."

  **See Also**
    * :PPL:`U_BDL()` – Total bytes downloaded
    * :PPL:`U_BDLDAY()` – Bytes downloaded today
    * :PPL:`U_FDL()` – Total files downloaded
    * :PPL:`U_FUL()` – Total files uploaded

U_FDL (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_FDL()`

  Access the total number of files downloaded by the current user.

  **Returns**
    Current user's total files downloaded.

  **Remarks**
    Returns information useful for modifying PCBoard's built-in ratio management system 
    and the view user information command. Unlike the predefined U_... user variables, 
    this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You have ULed ",U_FUL()," files and DLed ",U_FDL()," files."

  **See Also**
    * :PPL:`U_BDL()` – Total bytes downloaded
    * :PPL:`U_BDLDAY()` – Bytes downloaded today
    * :PPL:`U_BUL()` – Total bytes uploaded
    * :PPL:`U_FUL()` – Total files uploaded

U_FUL (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_FUL()`

  Access the total number of files uploaded by the current user.

  **Returns**
    Current user's total files uploaded.

  **Remarks**
    Returns information useful for modifying PCBoard's built-in ratio management system 
    and the view user information command. Unlike the predefined U_... user variables, 
    this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You have ULed ",U_FUL()," files and DLed ",U_FDL()," files."

  **See Also**
    * :PPL:`U_BDL()` – Total bytes downloaded
    * :PPL:`U_BDLDAY()` – Bytes downloaded today
    * :PPL:`U_BUL()` – Total bytes uploaded
    * :PPL:`U_FDL()` – Total files downloaded

U_INCONF (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN U_INCONF(INTEGER rec, INTEGER conf)`

  Determine if a user is registered in a conference.

  **Parameters**
    * :PPL:`rec` – Record number of the user to check
    * :PPL:`conf` – Conference number to check

  **Returns**
    TRUE if the user is registered in the specified conference, FALSE otherwise.

  **Remarks**
    Sometimes necessary to know if a user is registered in a conference (for example, 
    when entering a message to a particular user). Before calling this function you need 
    to find the user's record number from the USERS file with the U_RECNUM() function.

  **Example**

    .. code-block:: PPL

       INTEGER i,rec
       STRING un,ynStr(1)
       LET ynStr(0) = "NO"
       LET ynStr(1) = "YES"
       INPUT "User name",un
       NEWLINE
       LET rec = U_RECNUM(un)
       FOR i = 1 TO 10
           PRINTLN un," in conf ",i,": ",ynStr(U_INCONF(rec,i))
       NEXT

  **See Also**
    * :PPL:`U_RECNUM()` – Get user record number

U_LDATE (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION DATE U_LDATE()`

  Access the last log on date of a user.

  **Returns**
    Current user's last log on date.

  **Remarks**
    PCBoard tracks the last log on date for each user. This function returns that date 
    for the user currently online. Unlike the predefined U_... user variables, this 
    function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You last logged on ",U_LDATE(),"."

  **See Also**
    * :PPL:`U_LDIR()` – Last directory accessed
    * :PPL:`U_LTIME()` – Last log on time


U_LDIR (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION DATE U_LDIR()`

  Access the latest file date found in a file scan by a user.

  **Returns**
    Latest file date found by the current user.

  **Remarks**
    PCBoard tracks the latest file found by each user. This function returns that date 
    for the user currently online. Unlike the predefined U_... user variables, this 
    function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "Latest file found was dated ",U_LDIR(),"."

  **See Also**
    * :PPL:`U_LDATE()` – Last logon date
    * :PPL:`U_LTIME()` – Last logon time

U_LOGONS (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_LOGONS()`

  Access the total number of system logons by the current user.

  **Returns**
    Current user's total system logons.

  **Remarks**
    PCBoard tracks the total number of logons for each user. This function returns that 
    number for the user currently online. Unlike the predefined U_... user variables, 
    this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You have logged on to @BOARDNAME@ ",U_LOGONS()," times."

  **See Also**
    * :PPL:`CALLNUM()` – Get caller number
    * :PPL:`LOGGEDON()` – Check if logged on
    * :PPL:`ONLOCAL()` – Check if local session

U_LTIME (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION TIME U_LTIME()`

  Access the time of day that a user last logged on.

  **Returns**
    Time of day of the current user's last log on.

  **Remarks**
    PCBoard tracks the last time of day of the last log on for each user. This function 
    returns that time for the user currently online. Unlike the predefined U_... user 
    variables, this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You last logged on at ",U_LTIME(),"."

  **See Also**
    * :PPL:`U_LDATE()` – Last logon date
    * :PPL:`U_LDIR()` – Latest file date found

U_MSGRD (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_MSGRD()`

  Access the total number of messages read by the current user.

  **Returns**
    Current user's total messages read.

  **Remarks**
    PCBoard tracks the total number of messages read by each user. This function returns 
    that number for the user currently online. One quick idea for use: a message/file 
    ratio enforcement door. Unlike the predefined U_... user variables, this function 
    does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       IF ((U_MSGRD()+U_MSGWR())/U_FDL() > 10) THEN
           PRINTLN "You need to do more messaging!!!"
           END
       ENDIF

  **See Also**
    * :PPL:`U_MSGWR()` – Messages written

U_MSGWR (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_MSGWR()`

  Access the total number of messages written by the current user.

  **Returns**
    Current user's total messages written.

  **Remarks**
    PCBoard tracks the total number of messages written by each user. This function 
    returns that number for the user currently online. One quick idea for use: a 
    message/file ratio enforcement door. Unlike the predefined U_... user variables, 
    this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       IF ((U_MSGRD()+U_MSGWR())/U_FDL() > 10) THEN
           PRINTLN "You need to do more messaging!!!"
           END
       ENDIF

  **See Also**
    * :PPL:`U_MSGRD()` – Messages read

U_NAME (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING U_NAME()`

  Access the current user's name.

  **Returns**
    String with the current user's name.

  **Remarks**
    Perhaps the most important piece of information about a caller is their name. The 
    user name differentiates a user from every other user on the BBS and can be used 
    to track PPE user information that must be kept separate from all other users' 
    information. Unlike the predefined U_... user variables, this function does not 
    require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       IF (U_NAME() = "JOHN DOE") THEN
           PRINTLN "I know who you are! Welcome!"
           GETUSER
           LET U_SEC = 110
           PUTUSER
           PRINTLN "Automatically upgraded!"
       ENDIF

  **See Also**
    * :PPL:`CURCONF()` – Get current conference
    * :PPL:`MESSAGE` – Send message statement

U_NOTES (1.00)
~~~~~~~~~~~~~~
  :PPL:`STRING ARRAY U_NOTES(INTEGER index)`

  Allow reading and writing of current user notes.

  **Parameters**
    * :PPL:`index` – Array subscript (0-4): SysOp definable user notes (60 characters max)

  **Returns**
    STRING containing the requested note field.

  **Remarks**
    This array is filled with information from the current user's record when GETUSER 
    is executed. It may then be changed and written back with PUTUSER. The array is 
    empty until GETUSER is processed and changes aren't written until PUTUSER is 
    processed. The array only has meaningful information if the notes PSA is installed. 
    Check with PSA(6).

  **Example**

    .. code-block:: PPL

       INTEGER i
       IF (PSA(6)) THEN
           GETUSER
           FOR i = 0 TO 4
               PRINTLN "Note ",i+1,": ",U_NOTES(i)
           NEXT
       ENDIF

  **See Also**
    * :PPL:`GETUSER` – Load user record
    * :PPL:`PSA()` – Check PSA availability
    * :PPL:`PUTUSER` – Save user record

U_PWDHIST (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING U_PWDHIST(INTEGER num)`

  Access the last three passwords used by the current user.

  **Parameters**
    * :PPL:`num` – Password number from history (1-3: 1=most recent, 3=least recent)

  **Returns**
    Specified password from the history.

  **Remarks**
    PCBoard has the ability to track the last three passwords used by each user. This 
    function returns one of those passwords from the history for the user currently 
    online. Unlike the predefined U_... user variables, this function does not require 
    GETUSER to return valid information. However, it does require that the password 
    PSA has been installed to return meaningful information. Check with PSA(4).

  **Example**

    .. code-block:: PPL

       INTEGER i
       IF (PSA(4)) THEN
           FOR i = 1 TO 3
               PRINTLN "Password history ",i,": ",U_PWDHIST(i)
           NEXT
       ENDIF

  **See Also**
    * :PPL:`NEWPWD` – Change password statement
    * :PPL:`PSA()` – Check PSA availability
    * :PPL:`U_PWD` – Current password variable
    * :PPL:`U_PWDEXP` – Password expiration variable
    * :PPL:`U_PWDLC()` – Last password change date
    * :PPL:`U_PWDTC()` – Password change count

U_PWDLC (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION DATE U_PWDLC()`

  Access the last date the user changed their password.

  **Returns**
    Last date the user changed their password.

  **Remarks**
    PCBoard has the ability to track the last date of a password change for each user. 
    This function returns that date for the user currently online. Unlike the predefined 
    U_... user variables, this function does not require GETUSER to return valid 
    information. However, it does require that the password PSA has been installed 
    to return meaningful information. Check with PSA(4).

  **Example**

    .. code-block:: PPL

       IF (PSA(4)) PRINTLN "You last changed your password on ",U_PWDLC(),"."

  **See Also**
    * :PPL:`NEWPWD` – Change password statement
    * :PPL:`PSA()` – Check PSA availability
    * :PPL:`U_PWD` – Current password variable
    * :PPL:`U_PWDEXP` – Password expiration variable
    * :PPL:`U_PWDHIST()` – Password history
    * :PPL:`U_PWDTC()` – Password change count

U_PWDTC (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_PWDTC()`

  Access the number of times the user has changed their password.

  **Returns**
    Number of times the user has changed their password.

  **Remarks**
    PCBoard has the ability to track the total number of times each user changes their 
    password. This function returns that count for the user currently online. Unlike 
    the predefined U_... user variables, this function does not require GETUSER to 
    return valid information. However, it does require that the password PSA has been 
    installed to return meaningful information. Check with PSA(4).

  **Example**

    .. code-block:: PPL

       IF (PSA(4)) THEN
           PRINTLN "You have changed your password ",U_PWDTC()," times."
       ENDIF

  **See Also**
    * :PPL:`NEWPWD` – Change password statement
    * :PPL:`PSA()` – Check PSA availability
    * :PPL:`U_PWD` – Current password variable
    * :PPL:`U_PWDEXP` – Password expiration variable
    * :PPL:`U_PWDHIST()` – Password history
    * :PPL:`U_PWDLC()` – Last password change date

U_RECNUM (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_RECNUM(STRING user)`

  Determine if a user is registered on the system and get their record number.

  **Parameters**
    * :PPL:`user` – User name to search for

  **Returns**
    Record number of the user in the USERS file if found, or -1 if not found.

  **Remarks**
    This function serves two purposes. First, to determine whether or not a given user 
    name is registered on the system. If -1 is returned, the user isn't in the user files. 
    Second, to get the user's record number for the U_INCONF() function to determine 
    whether or not a user is registered in a given conference.

  **Example**

    .. code-block:: PPL

       INTEGER i,rec
       STRING un,ynStr(1)
       LET ynStr(0) = "NO"
       LET ynStr(1) = "YES"
       INPUT "User name",un
       NEWLINE
       LET rec = U_RECNUM(un)
       FOR i = 1 TO 10
           PRINTLN un," in conf ",i,": ",ynStr(U_INCONF(rec,i))
       NEXT

  **See Also**
    * :PPL:`U_INCONF()` – Check conference registration

U_STAT (1.00)
~~~~~~~~~~~~~
  :PPL:`FUNCTION <VARIANT> U_STAT(INTEGER stat)`

  Access a statistic about the current user.

  **Parameters**
    * :PPL:`stat` – Statistic to retrieve (1-15)

  **Returns**
    * DATE for stat=1: First date the user called the system
    * INTEGER for stat 2-15:
      * 2: Number of times the user has paged the SysOp
      * 3: Number of group chats the user has participated in
      * 4: Number of comments left by the user
      * 5: Number of 300 bps connects by the user
      * 6: Number of 1200 bps connects by the user
      * 7: Number of 2400 bps connects by the user
      * 8: Number of connects 2400 < speed ≤ 9600 bps
      * 9: Number of connects 9600 < speed ≤ 14400 bps
      * 10: Number of security violations by the user
      * 11: Number of "not registered in conference" warnings
      * 12: Number of times download limit has been reached
      * 13: Number of "file not found" warnings
      * 14: Number of password errors to access account
      * 15: Number of verify errors to access account

  **Remarks**
    PCBoard has the ability to track a number of statistics about the user. This 
    function returns the desired statistic for the user currently online. Unlike the 
    predefined U_... user variables, this function does not require GETUSER to return 
    valid information. However, it does require that the statistics PSA has been 
    installed to return meaningful information. Check with PSA(5).

  **Example**

    .. code-block:: PPL

       STRING label
       INTEGER i
       FOPEN 1,PPEPATH()+"STATTEXT",O_RD,S_DN
       FOR i = 1 TO 15
           FGET 1,label
           PRINTLN label," = ",U_STAT(i)
       NEXT
       FCLOSE 1

  **See Also**
    * :PPL:`PSA()` – Check PSA availability

U_TIMEON (1.00)
~~~~~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER U_TIMEON()`

  Access the user's time online today in minutes.

  **Returns**
    User's time online today in minutes.

  **Remarks**
    PCBoard tracks the user's time online each day. This function returns the elapsed 
    time for the user currently online. Unlike the predefined U_... user variables, 
    this function does not require GETUSER to return valid information.

  **Example**

    .. code-block:: PPL

       PRINTLN "You have been online for ",U_TIMEON()," total minutes today."

  **See Also**
    * :PPL:`ADJTIME` – Adjust time remaining
    * :PPL:`MINLEFT()` – Minutes left
    * :PPL:`MINON()` – Minutes online this session

VALCC (1.00)
~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN VALCC(STRING sexp)`

  Tests a string for credit card number format validity.

  **Parameters**
    * :PPL:`sexp` – String to test

  **Returns**
    TRUE if the string is a valid credit card number format, FALSE otherwise.

  **Remarks**
    This function takes a string and attempts to identify it as a credit card number. 
    If the number is invalid for any reason (insufficient digits or bad checksum, 
    primarily) then this function returns FALSE, otherwise it returns TRUE.

  **Example**

    .. code-block:: PPL

       STRING s
       WHILE (!VALCC(s)) DO
           INPUT "CC #",s
           NEWLINES 2
       ENDWHILE
       PRINTLN CCTYPE(s)," - ",FMTCC(s)

  **See Also**
    * :PPL:`CCTYPE()` – Get card type
    * :PPL:`FMTCC()` – Format card number
    * :PPL:`VALDATE()` – Validate date format
    * :PPL:`VALTIME()` – Validate time format

VALDATE (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN VALDATE(STRING sexp)`

  Tests a string for date format validity.

  **Parameters**
    * :PPL:`sexp` – String to test

  **Returns**
    TRUE if the string is a valid date format, FALSE otherwise.

  **Remarks**
    PPL does its best to convert incompatible types automatically. Converting a STRING 
    to a DATE type is particularly problematic because of the virtually unlimited 
    numbers of strings possible. This function checks date validity and format.

  **Example**

    .. code-block:: PPL

       STRING s
       WHILE (!VALDATE(s)) DO
           INPUT "Date",s
           NEWLINES 2
       ENDWHILE
       DATE d
       LET d = s
       PRINTLN s," ",d

  **See Also**
    * :PPL:`VALCC()` – Validate credit card
    * :PPL:`VALTIME()` – Validate time format

VALTIME (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION BOOLEAN VALTIME(STRING sexp)`

  Tests a string for time format validity.

  **Parameters**
    * :PPL:`sexp` – String to test

  **Returns**
    TRUE if the string is a valid time format, FALSE otherwise.

  **Remarks**
    PPL does its best to convert incompatible types automatically. Converting a STRING 
    to a TIME type is particularly problematic. This function checks to make sure that 
    the hour is from 0 to 23, the minute is from 0 to 59, and the second (optional) 
    is from 0 to 59. Each field must be separated by a colon.

  **Example**

    .. code-block:: PPL

       STRING s
       WHILE (!VALTIME(s)) DO
           INPUT "Time",s
           NEWLINES 2
       ENDWHILE
       TIME t
       LET t = s
       PRINTLN s," ",t

  **See Also**
    * :PPL:`VALCC()` – Validate credit card
    * :PPL:`VALDATE()` – Validate date format

VER (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER VER()`

  Get the version of PPL available.

  **Returns**
    Version number of PPL running.

  **Remarks**
    As time passes, new features will be added to PCBoard and PPL. This function returns 
    the version of PCBoard (and PPL). For PCBoard version 15.0 this value is 1500. 
    The major version is accessible via VER()/100, and the minor version via VER()%100. 
    Everything documented herein is available for versions ≥ 1500.

  **Example**

    .. code-block:: PPL

       IF (VER() < 1600) THEN
           PRINTLN "PCBoard Version 16.0 required for this PPE file"
           END
       ENDIF

  **See Also**
    * :PPL:`PSA()` – Check PSA availability

XOR (1.00)
~~~~~~~~~~
  :PPL:`FUNCTION INTEGER XOR(INTEGER iexp1, INTEGER iexp2)`

  Calculate the bitwise XOR (exclusive or) of two integer arguments.

  **Parameters**
    * :PPL:`iexp1` – First integer expression
    * :PPL:`iexp2` – Second integer expression

  **Returns**
    Bitwise XOR of iexp1 and iexp2.

  **Remarks**
    This function may be used to toggle selected bits in an integer expression by 
    XORing the expression with a mask that has the bits to toggle set to 1 and 
    the bits to ignore set to 0.

  **Example**

    .. code-block:: PPL

       ; Toggle the bits in the low byte
       PRINTLN XOR(0x1248,0x00FF)
       ; Toggle a flag
       INTEGER flag
       LET flag = XOR(flag,1)

  **See Also**
    * :PPL:`AND()` – Bitwise AND
    * :PPL:`NOT()` – Bitwise NOT
    * :PPL:`OR()` – Bitwise OR

YEAR (1.00)
~~~~~~~~~~~
  :PPL:`FUNCTION INTEGER YEAR(DATE dexp)`

  Extracts the year from a specified date.

  **Parameters**
    * :PPL:`dexp` – Date expression

  **Returns**
    Year from the specified date expression (1900-2079).

  **Remarks**
    This function allows you to extract a particular piece of information about a 
    DATE value, in this case the year of the date.

  **Example**

    .. code-block:: PPL

       PRINTLN "This year is: ",YEAR(DATE())

  **See Also**
    * :PPL:`DATE()` – Get current date
    * :PPL:`DAY()` – Extract day
    * :PPL:`DOW()` – Day of week
    * :PPL:`MONTH()` – Extract month

YESCHAR (1.00)
~~~~~~~~~~~~~~
  :PPL:`FUNCTION STRING YESCHAR()`

  Get the yes response character for the current language.

  **Returns**
    Yes character for the current language.

  **Remarks**
    Support for foreign language yes/no responses can be easily added by using this 
    function to determine what an affirmative response should be instead of hardcoding 
    the English "Y" character.

  **Example**

    .. code-block:: PPL

       STRING ans
       LET ans = YESCHAR()
       INPUTSTR "Run program now",ans,@X0E,1,"",AUTO+YESNO
       IF (ans = NOCHAR()) END

  **See Also**
    * :PPL:`NOCHAR()` – Get no character
    * :PPL:`YESNO` – Yes/no input flag constant    