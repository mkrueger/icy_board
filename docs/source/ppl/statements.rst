.. role:: PPL(code)
   :language: PPL


Statements
----------

ACCOUNT (3.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT ACCOUNT(INTEGER field, INTEGER value)`

  Update user accounting debit/credit values.

  **Parameters**
    * :PPL:`field` – Accounting field (0-17, use constants START_BAL through SEC_DROP)
    * :PPL:`value` – Amount of credits to add to the field

  **Field Constants**
    ================  ===  ============================================
    Constant          Val  Description
    ================  ===  ============================================
    START_BAL         0    User's starting balance
    START_SESSION     1    Starting balance for this session
    DEB_CALL          2    Debit for this call
    DEB_TIME          3    Debit for time online
    DEB_MSGREAD       4    Debit for reading messages
    DEB_MSGCAP        5    Debit for capturing messages
    DEB_MSGWRITE      6    Debit for writing messages
    DEB_MSGECHOED     7    Debit for echoed messages
    DEB_MSGPRIVATE    8    Debit for private messages
    DEB_DOWNFILE      9    Debit for downloading files
    DEB_DOWNBYTES     10   Debit for downloading bytes
    DEB_CHAT          11   Debit for chat time
    DEB_TPU           12   Debit for TPU
    DEB_SPECIAL       13   Special debit
    CRED_UPFILE       14   Credit for uploading files
    CRED_UPBYTES      15   Credit for uploading bytes
    CRED_SPECIAL      16   Special credit
    SEC_DROP          17   Security level to drop to at 0 credits
    ================  ===  ============================================

  **Remarks**
    Updates accounting values in memory only. This statement modifies debit/credit fields 
    directly without creating audit trail records. For full transaction logging with 
    descriptions and unit costs, use RECORDUSAGE instead.

    The value parameter is added to the current field value (use negative values to subtract).

  **Example**

    .. code-block:: PPL

       ; Record 10 credits used for chatting
       ACCOUNT DEB_CHAT, 10
       
       ; Add download byte charges
       INTEGER fileSize
       fileSize = FILEINF("MYFILE.ZIP", 2)
       ACCOUNT DEB_DOWNBYTES, fileSize
       
       ; Give upload credit (add to credits)
       ACCOUNT CRED_UPFILE, 1

  **See Also**
    * :PPL:`ACCOUNT()` function – Retrieve accounting values
    * :PPL:`RECORDUSAGE` – Update accounting with detailed logging
    * :PPL:`PCBACCOUNT()` – Get charge rates
    * :PPL:`PCBACCSTAT()` – Check accounting status

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


ADJTIME (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT ADJTIME(INTEGER minutes)`

  Adjust the user's time up or down for the current session.

  **Parameters**
    * :PPL:`minutes` – Number of minutes to adjust (positive adds time, negative deducts time)

  **Remarks**
    Rewards or penalizes the user with more or less time. The adjustment only applies to the 
    current call and is not saved after hangup, except it's reflected in time online today. 
    Time can only be added if the user's time has not been adjusted for an event. Time can 
    always be subtracted.

  **Example**

    .. code-block:: PPL

       STRING yn
       INPUTYN "Do you wish to gamble 5 minutes for 10",yn,@X0E
       IF (yn = YESCHAR()) THEN
           IF (RANDOM(1) = 1) THEN
               PRINTLN "You *WON*! 10 extra minutes awarded."
               ADJTIME 10
           ELSE
               PRINTLN "You lost. Sorry, but I have to take 5 minutes now."
               ADJTIME -5
           ENDIF
       ELSE
           PRINTLN "Chicken! ;)"
       ENDIF

  **See Also**
    * :PPL:`MINLEFT()` – Minutes remaining
    * :PPL:`MINON()` – Minutes online
    * :PPL:`U_TIMEON()` – User's time online today

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


ANSIPOS (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT ANSIPOS(INTEGER xpos, INTEGER ypos)`

  Position the cursor anywhere on the screen using ANSI escape sequences.

  **Parameters**
    * :PPL:`xpos` – Screen column (1-80)
    * :PPL:`ypos` – Screen row (1-23)

  **Remarks**
    Positions the cursor at the specified (X,Y) coordinate but only if the caller has ANSI 
    support. If ANSI is not available, this statement is ignored. Check ANSION() before 
    using if ANSI positioning is required.

  **Example**

    .. code-block:: PPL

       CLS
       IF (ANSION()) THEN
           ANSIPOS 1,1
           PRINTLN "This starts at (1,1)"
           ANSIPOS 3,3
           PRINTLN "This starts at (3,3)"
           ANSIPOS 2,2
           PRINTLN "And *THIS* starts at (2,2)"
       ENDIF

  **See Also**
    * :PPL:`ANSION()` – Check ANSI availability
    * :PPL:`BACKUP` – Move cursor backward
    * :PPL:`FORWARD` – Move cursor forward
    * :PPL:`GETX()` – Get cursor column
    * :PPL:`GETY()` – Get cursor row

BACKUP (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT BACKUP(INTEGER numcols)`

  Move the cursor backward a specified number of columns.

  **Parameters**
    * :PPL:`numcols` – Number of columns to move backward (1-79)

  **Remarks**
    Moves the cursor backward non-destructively. Works with or without ANSI - uses ANSI 
    positioning if available, otherwise uses backspace characters. Cannot move beyond 
    column 1 without ANSI. Has no effect if already at column 1.

  **Example**

    .. code-block:: PPL

       PRINT "Rolling dice -- "
       FOR i = 1 TO 10
           LET d1 = RANDOM(5) + 1
           LET d2 = RANDOM(5) + 1
           PRINT d1,"-",d2
           BACKUP 3
       NEXT
       NEWLINE

  **See Also**
    * :PPL:`ANSION()` – Check ANSI availability
    * :PPL:`ANSIPOS` – Position cursor
    * :PPL:`FORWARD` – Move cursor forward
    * :PPL:`GETX()` – Get cursor column
    * :PPL:`GETY()` – Get cursor row

BLT (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT BLT(INTEGER bltnum)`

  Display a specified bulletin number to the user.

  **Parameters**
    * :PPL:`bltnum` – Bulletin number to display (1 through max bulletins)

  **Remarks**
    Displays the specified bulletin from the BLT.LST file for the current conference. 
    If the bulletin number is invalid, nothing is displayed.

  **Example**

    .. code-block:: PPL

       INTEGER num
       INPUT "Bulletin to view",num
       BLT num

  **See Also**
    * :PPL:`DIR` – Display directory
    * :PPL:`JOIN` – Join conference
    * :PPL:`QUEST` – Run questionnaire

BROADCAST (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT BROADCAST(INTEGER lonode, INTEGER hinode, STRING message)`

  Broadcast a single line message to a range of nodes.

  **Parameters**
    * :PPL:`lonode` – Low node number for broadcast range
    * :PPL:`hinode` – High node number for broadcast range
    * :PPL:`message` – Message text to broadcast

  **Remarks**
    Functions like the PCBoard BROADCAST command (normally SysOp only). Allows 
    programmatic broadcasting without giving users manual broadcast ability.

  **Example**

    .. code-block:: PPL

       ; Broadcast to a specific node
       BROADCAST 5,5,"This broadcast from "+STRING(PCBNODE())
       
       ; Broadcast to a range of nodes
       BROADCAST 4,8,"Stand-by for log off in 10 seconds"
       
       ; Broadcast to all nodes
       BROADCAST 1,65535,"Hello all!"

  **See Also**
    * :PPL:`RDUNET` – Read USERNET record
    * :PPL:`UN_CITY()` – Get USERNET city field
    * :PPL:`UN_NAME()` – Get USERNET name field
    * :PPL:`UN_OPER()` – Get USERNET operation field
    * :PPL:`UN_STAT()` – Get USERNET status field
    * :PPL:`WRUNET` – Write USERNET record

BYE (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT BYE`

  Log the user off immediately without confirmation prompts.

  **Remarks**
    Logs off the user as if they typed the BYE command. Unlike GOODBYE, this assumes the user 
    really wants to log off and skips download warnings and confirmation prompts. Intended for 
    providing the same functionality as PCBoard prompts where G or BYE can be entered.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "What do you want to do",s
       IF (s = "G") THEN GOODBYE
       ELSEIF (s = "BYE") THEN BYE
       ELSE KBDSTUFF s
       ENDIF

  **See Also**
    * :PPL:`DTROFF` – Turn off DTR signal
    * :PPL:`DTRON` – Turn on DTR signal
    * :PPL:`GOODBYE` – Log off with confirmation
    * :PPL:`HANGUP` – Immediate disconnect

CALL (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT CALL(STRING filename)`

  Execute another PPE file and return to the current PPE.

  **Parameters**
    * :PPL:`filename` – Complete path and filename of PPE file to execute

  **Remarks**
    Loads and runs another PPE file, then returns control to the statement after the CALL. 
    The second PPE is completely separate from the first. Pass values via TOKENIZE statement. 
    Return values require creating your own parameter passing convention (e.g., via files).

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "What PPE file do you wish to run",s
       CALL "C:\PCB\PPE\"+s+".PPE"

  **See Also**
    * :PPL:`SHELL` – Execute external program
    * :PPL:`TOKENIZE` – Tokenize string for parameter passing

CDCHKOFF (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT CDCHKOFF`

  Turn off automatic carrier detect checking.

  **Remarks**
    Disables PCBoard's automatic carrier detection. Useful for applications that need to 
    continue processing after hangup (e.g., callback verification). Use CDCHKON to re-enable 
    checking when the carrier-independent section is complete.

  **Example**

    .. code-block:: PPL

       CDCHKOFF
       DTROFF
       DELAY 18
       DTRON
       SENDMODEM "ATDT1800DATAFON"+CHR(13)
       WAITFOR "CONNECT",60
       CDCHKON

  **See Also**
    * :PPL:`CDCHKON` – Turn on carrier checking
    * :PPL:`CDON()` – Check carrier status
    * :PPL:`KBDCHKOFF` – Turn off keyboard checking
    * :PPL:`KBDCHKON` – Turn on keyboard checking

CDCHKON (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT CDCHKON`

  Turn on carrier detect checking.

  **Remarks**
    Re-enables PCBoard's automatic carrier detection after it was disabled with CDCHKOFF. 
    Should be called after completing code sections that need to run regardless of carrier status.

  **Example**

    .. code-block:: PPL

       CDCHKOFF
       ; Carrier-independent code here
       CDCHKON

  **See Also**
    * :PPL:`CDCHKOFF` – Turn off carrier checking
    * :PPL:`CDON()` – Check carrier status
    * :PPL:`KBDCHKOFF` – Turn off keyboard checking
    * :PPL:`KBDCHKON` – Turn on keyboard checking

CHAT (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT CHAT`

  Enter SysOp chat mode.

  **Remarks**
    Starts a chat session between the SysOp and user. Generally used after confirming SysOp 
    availability via paging. The SysOp can still initiate chat with F10 or the O command. 
    Users cannot exit chat mode themselves.

  **Example**

    .. code-block:: PPL

       PAGEON
       FOR i = 1 TO 10
           PRINT "@BEEP@"
           DELAY 18
           IF (INKEY() = " ") THEN
               CHAT
               GOTO exit
           ENDIF
       NEXT
       :exit

  **See Also**
    * :PPL:`PAGEON` – Enable operator paging
    * :PPL:`PAGEOFF` – Disable operator paging
    * :PPL:`PAGESTAT()` – Check page status

CHDIR (3.20)
~~~~~~~~~~~~
  :PPL:`STATEMENT CHDIR(STRING path)`

  Changes the current working directory.

CLOSECAP (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT CLOSECAP`

  Close the screen capture file.

  **Remarks**
    Closes the capture file opened with OPENCAP and stops screen capturing. Useful for 
    capturing command output for later viewing or download. Use with SHOWON/SHOWOFF to 
    control whether output is displayed while being captured.

  **Example**

    .. code-block:: PPL

       BOOLEAN ss, ocFlag
       LET ss = SHOWSTAT()
       SHOWOFF
       OPENCAP "CAP"+STRING(PCBNODE()),ocFlag
       IF (ocFlag) THEN
           DIR "U;NS"
           CLOSECAP
           KBDSTUFF "FLAG CAP"+STRING(PCBNODE())+CHR(13)
       ENDIF
       IF (ss) THEN SHOWON ELSE SHOWOFF

  **See Also**
    * :PPL:`OPENCAP` – Open capture file
    * :PPL:`SHOWOFF` – Hide display output
    * :PPL:`SHOWON` – Show display output
    * :PPL:`SHOWSTAT()` – Check display status

CLREOL (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT CLREOL`

  Clear from cursor position to end of line using current color.

  **Remarks**
    In graphics/ANSI mode, sends ANSI clear-to-end-of-line sequence. In non-ANSI mode, 
    writes spaces to column 80 then backspaces to original position. Does not clear 
    column 80 in non-ANSI mode to keep cursor on current line.

  **Example**

    .. code-block:: PPL

       COLOR @X47
       CLS
       PRINT "This is some sample text. (This will disappear.)"
       WHILE (INKEY() = "") DELAY 1
       BACKUP 22
       COLOR @X1F
       CLREOL
       PRINT "This goes to the end of the line."

  **See Also**
    * :PPL:`CLS` – Clear screen

CLS (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT CLS`

  Clear the screen using the current color.

  **Remarks**
    In graphics/ANSI mode, sends ANSI clear screen sequence. In non-ANSI mode, sends 
    ASCII 12 (form feed) character. Not all terminals support form feed, so some users 
    may see the character instead of a cleared screen.

  **Example**

    .. code-block:: PPL

       COLOR @X47
       CLS
       PRINTLN "Welcome to a clean screen"

  **See Also**
    * :PPL:`CLREOL` – Clear to end of line

CLREOL (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT CLREOL`

  Clears from the current cursor position to the end of the line.

COLOR (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT COLOR(INTEGER newcolor)`

  Change the current active color.

  **Parameters**
    * :PPL:`newcolor` – New color value (use @X codes or numeric values)

  **Remarks**
    Changes the color used by PCBoard and sends appropriate ANSI sequences to the remote 
    terminal. Only affects color if user is in graphics mode; ignored in non-graphics mode.

  **Example**

    .. code-block:: PPL

       COLOR @X47
       CLS
       PRINT "This is some sample text. (This will disappear.)"
       WHILE (INKEY() = "") DELAY 1
       BACKUP 22
       COLOR @X1F
       CLREOL
       PRINT "This goes to the end of the line."

  **See Also**
    * :PPL:`CURCOLOR()` – Get current color
    * :PPL:`DEFCOLOR` – Set default color
    * :PPL:`DEFCOLOR()` – Get default color

CONFFLAG (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT CONFFLAG(INTEGER confnum, INTEGER flags)`

  Set specified flags in a conference for the current user.

  **Parameters**
    * :PPL:`confnum` – Conference number to affect
    * :PPL:`flags` – Flags to set (F_REG, F_EXP, F_SEL, F_MW, F_SYS)

  **Remarks**
    Each user has five flags per conference controlling registration, expired status, 
    selection, mail waiting, and SysOp privileges. Use predefined constants F_REG, 
    F_EXP, F_SEL, F_MW, and F_SYS. Add constants together to set multiple flags.

  **Example**

    .. code-block:: PPL

       ; Automatically register them in selected conferences
       INTEGER i
       FOR i = 1 TO 10
           CONFFLAG i,F_REG+F_EXP+F_SEL
       NEXT
       FOR i = 11 TO 20
           CONFFLAG i,F_REG+F_SEL
       NEXT

  **See Also**
    * :PPL:`CONFUNFLAG` – Clear conference flags

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

CONFUNFLAG (1.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT CONFUNFLAG(INTEGER confnum, INTEGER flags)`

  Clear specified flags in a conference for the current user.

  **Parameters**
    * :PPL:`confnum` – Conference number to affect
    * :PPL:`flags` – Flags to clear (F_REG, F_EXP, F_SEL, F_MW, F_SYS)

  **Remarks**
    Clears user's conference flags controlling registration, expired status, selection, 
    mail waiting, and SysOp privileges. Use predefined constants F_REG, F_EXP, F_SEL, 
    F_MW, and F_SYS. Add constants together to clear multiple flags.

  **Example**

    .. code-block:: PPL

       ; Automatically deregister them from selected conferences
       INTEGER i
       FOR i = 1 TO 10
           CONFUNFLAG i,F_REG+F_EXP+F_SEL
       NEXT
       FOR i = 11 TO 20
           CONFUNFLAG i,F_REG+F_SEL
       NEXT

  **See Also**
    * :PPL:`CONFFLAG` – Set conference flags

DBGLEVEL (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT DBGLEVEL(INTEGER level)`

  Set a new debug level for PCBoard.

  **Parameters**
    * :PPL:`level` – Debug level (0=none, 1-3=increasing debug info)

  **Remarks**
    Controls debug information written to the caller's log. Level 0 disables debug 
    output. Levels 1 through 3 provide increasing amounts of debug information. 
    Useful for debugging PPL programs. Changes debug level without requiring 
    SysOp to exit and modify BOARD.BAT.

  **Example**

    .. code-block:: PPL

       INTEGER newlvl
       INPUT "New level",newlvl
       NEWLINE
       DBGLEVEL newlvl

  **See Also**
    * :PPL:`DBGLEVEL()` – Get current debug level
    * :PPL:`LOG` – Write to log file

DEC (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT DEC(VAR var)`

  Decrement the value of a variable by 1.

  **Parameters**
    * :PPL:`var` – Variable to decrement

  **Remarks**
    Provides a shorter, more efficient method of decreasing a value by 1 than using 
    LET i = i - 1. Commonly used in countdown loops and counters.

  **Example**

    .. code-block:: PPL

       INTEGER i
       PRINTLN "Countdown:"
       LET i = 10
       WHILE (i >= 0) DO
           PRINTLN "T minus ",i
           DEC i
       ENDWHILE

  **See Also**
    * :PPL:`INC` – Increment variable

DEFCOLOR (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT DEFCOLOR`

  Change the current color to the system default color.

  **Remarks**
    Changes the color to the system default and sends appropriate ANSI sequences. 
    Equivalent to COLOR DEFCOLOR(). Only affects color if user is in graphics mode; 
    ignored in non-graphics mode.

  **Example**

    .. code-block:: PPL

       COLOR @X47
       CLS
       PRINT "This is some sample text. (This will disappear.)"
       WHILE (INKEY() = "") DELAY 1
       BACKUP 22
       DEFCOLOR
       CLREOL
       PRINT "This goes to the end of the line."

  **See Also**
    * :PPL:`COLOR` – Set color
    * :PPL:`CURCOLOR()` – Get current color
    * :PPL:`DEFCOLOR()` – Get default color

DELAY (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT DELAY(INTEGER ticks)`

  Pause execution for a specified number of clock ticks.

  **Parameters**
    * :PPL:`ticks` – Number of clock ticks to pause (18.2 ticks = 1 second)

  **Remarks**
    Pauses execution for a precise time interval. One clock tick is approximately 1/18.2 
    seconds. To delay for one second, use DELAY 18. For runtime calculations, use 
    (seconds * 182) / 10 since PPL doesn't support floating point.

  **Example**

    .. code-block:: PPL

       INTEGER i
       PRINTLN "Countdown:"
       LET i = 10
       WHILE (i >= 0) DO
           PRINTLN "T minus ",i
           DEC i
           DELAY 18
       ENDWHILE

  **See Also**
    * :PPL:`SOUND` – Generate sound

DELETE (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT DELETE(STRING file)`

  Delete a specified file from disk.

  **Parameters**
    * :PPL:`file` – Drive, path and filename to delete

  **Remarks**
    Deletes files from disk. Useful for cleaning up temporary files created by your PPE. 
    Always clean up temporary files to avoid cluttering the system.

  **Example**

    .. code-block:: PPL

       INTEGER retcode
       STRING s
       FCREATE 1,"TMP.LST",O_WR,S_DB
       ; ... write data ...
       FCLOSE 1
       SHELL 1,retcode,"SORT","< TMP.LST > TMP.SRT"
       DISPFILE "TMP.SRT",DEFS
       DELETE "TMP.LST"
       DELETE "TMP.SRT"

  **See Also**
    * :PPL:`EXIST()` – Check file existence
    * :PPL:`FILEINF()` – Get file information
    * :PPL:`RENAME` – Rename file

DELUSER (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT DELUSER`

  Flag the current user for deletion.

  **Remarks**
    Sets the delete flag for the user record. The user will be packed out during the next 
    pack operation. To prevent re-login before packing, use GETUSER, set U_SEC and 
    U_EXPSEC to 0, then PUTUSER.

  **Example**

    .. code-block:: PPL

       GETUSER
       IF (U_CMNT2 = "BAD USER") THEN
           PRINTLN "User flagged for deletion..."
           DELUSER
           LET U_SEC = 0
           LET U_EXPSEC = 0
           PUTUSER
       ENDIF

  **See Also**
    * :PPL:`GETUSER` – Load user record
    * :PPL:`PUTUSER` – Save user record
    * :PPL:`U_SEC` – User security level
    * :PPL:`U_EXPSEC` – User expired security

DIR (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT DIR(STRING cmds)`

  Execute the file directories command with sub-commands.

  **Parameters**
    * :PPL:`cmds` – Sub-commands for the file directory (e.g., "N;S;A;NS")

  **Remarks**
    Accesses file directories (F command) under PPE control. Destroys any previously 
    tokenized string expression. Save tokens before using DIR if needed.

  **Example**

    .. code-block:: PPL

       INTEGER retcode
       SHOWOFF
       OPENCAP "NEWFILES.LST",retcode
       KBDSTUFF CHR(13)
       DIR "N;S;A;NS"
       CLOSECAP
       SHOWON
       SHELL TRUE,retcode,"PKZIP","-mex NEWFILES NEWFILES.LST"
       KBDSTUFF "FLAG NEWFILES.ZIP"

  **See Also**
    * :PPL:`BLT` – Display bulletin
    * :PPL:`JOIN` – Join conference
    * :PPL:`QUEST` – Run questionnaire

DISPFILE (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT DISPFILE(STRING file, INTEGER flags)`

  Display a file with optional alternate file searching.

  **Parameters**
    * :PPL:`file` – Filename or base filename to display
    * :PPL:`flags` – Alternate file flags (0=none, GRAPH=1, SEC=2, LANG=4, or combinations)

  **Remarks**
    Displays a file to the user. Can search for alternate security, graphics, and/or 
    language specific files based on flags. Use 0 for no alternate searching, or combine 
    GRAPH, SEC, and LANG flags for multiple searches.

  **Example**

    .. code-block:: PPL

       STRING s
       DISPFILE "MNUA",SEC+GRAPH+LANG
       INPUT "Option",s

  **See Also**
    * :PPL:`DISPSTR` – Display string
    * :PPL:`DISPTEXT` – Display PCBTEXT prompt
    * :PPL:`OPTEXT` – Display with options

DISPSTR (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT DISPSTR(STRING str)`

  Display a string, file, or execute a PPE.

  **Parameters**
    * :PPL:`str` – String to display, %filename to display file, or !PPEfile to execute

  **Remarks**
    Displays a string to the user. If string begins with %, displays the specified file. 
    If string begins with !, executes the specified PPE file.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "String",s
       DISPSTR s
       DISPSTR "Regular string"
       DISPSTR "%C:\PCB\GEN\BRDM"
       DISPSTR "!"+PPEPATH()+"SUBSCR.PPE"

  **See Also**
    * :PPL:`DISPFILE` – Display file
    * :PPL:`DISPTEXT` – Display PCBTEXT prompt

DISPTEXT (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT DISPTEXT(INTEGER rec, INTEGER flags)`

  Display a prompt from the PCBTEXT file.

  **Parameters**
    * :PPL:`rec` – PCBTEXT record number to display
    * :PPL:`flags` – Display flags (BELL, DEFS, LFAFTER, LFBEFORE, LOGIT, LOGITLEFT, NEWLINE)

  **Remarks**
    Displays any prompt from the PCBTEXT file according to display flags. Combine flags 
    with + operator for multiple effects.

  **Example**

    .. code-block:: PPL

       DISPTEXT 192,BELL+NEWLINE+LOGIT
       HANGUP

  **See Also**
    * :PPL:`DISPFILE` – Display file
    * :PPL:`DISPSTR` – Display string

DOINTR (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT DOINTR(INTEGER int, INTEGER ax, INTEGER bx, INTEGER cx, INTEGER dx, INTEGER si, INTEGER di, INTEGER flags, INTEGER ds, INTEGER es)`

  Generate a system interrupt.

  **Parameters**
    * :PPL:`int` – Interrupt number (0-255)
    * :PPL:`ax,bx,cx,dx,si,di` – General purpose register values
    * :PPL:`flags` – Processor status register
    * :PPL:`ds,es` – Segment register values

  **Remarks**
    Provides access to system services via BIOS, DOS, or third-party interfaces. Return 
    values accessible via REG...() functions. WARNING: Can be destructive if used 
    improperly. Use at your own risk!

  **Example**

    .. code-block:: PPL

       ; Create subdirectory - DOS function 39h
       INTEGER addr
       STRING path
       LET path = "C:\$TMPDIR$"
       VARADDR path,addr
       DOINTR 0x21,0x39,0,0,addr*0x10000,0,0,0,addr/0x10000,0
       IF (REGCF() & (REGAX() = 3)) THEN
           PRINTLN "Error: Path not found"
       ENDIF

  **See Also**
    * :PPL:`B2W()` – Byte to word conversion
    * :PPL:`REG...()` – Register access functions

DTROFF (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT DTROFF`

  Turn off the serial port DTR signal.

  **Remarks**
    Turns off DTR signal, causing most modems to hang up. Used when you need to hangup 
    without PCBoard's logoff processing. Should remain off for at least 9 clock ticks 
    (~0.5 seconds) for modem to react.

  **Example**

    .. code-block:: PPL

       KBDCHKOFF
       CDCHKOFF
       DTROFF
       DELAY 18
       DTRON
       SENDMODEM "ATDT5551212"
       WAITFOR "CONNECT",flag,60
       CDCHKON
       KBDCHKON

  **See Also**
    * :PPL:`BYE` – Log off immediately
    * :PPL:`DTRON` – Turn on DTR signal
    * :PPL:`GOODBYE` – Log off with confirmation
    * :PPL:`HANGUP` – Disconnect immediately

DTRON (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT DTRON`

  Turn on the serial port DTR signal.

  **Remarks**
    Turns on DTR signal after using DTROFF. DTR should remain off for at least 9 clock 
    ticks before turning back on to ensure modem has time to react.

  **Example**

    .. code-block:: PPL

       DTROFF
       DELAY 18
       DTRON

  **See Also**
    * :PPL:`BYE` – Log off immediately
    * :PPL:`DTROFF` – Turn off DTR signal
    * :PPL:`GOODBYE` – Log off with confirmation
    * :PPL:`HANGUP` – Disconnect immediately

END (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT END`

  Terminate PPE execution.

  **Remarks**
    Normally terminates PPE execution. Automatically inserted at end of source if not 
    present. For script questionnaires, saves any responses written to channel 0 to the 
    script answer file.

  **Example**

    .. code-block:: PPL

       DATE d
       LET d = "01-20-93"
       IF (DATE() < d) THEN
           PRINTLN "Your calendar is off!"
           END
       ENDIF
       PRINTLN "Processing continues..."

  **See Also**
    * :PPL:`RETURN` – Return from subroutine
    * :PPL:`STOP` – Stop execution

FAPPEND (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FAPPEND(INTEGER chan, STRING file, INTEGER am, INTEGER sm)`

  Open a file for append access.

  **Parameters**
    * :PPL:`chan` – Channel number (0-7)
    * :PPL:`file` – File specification to open
    * :PPL:`am` – Access mode (O_RD, O_WR, O_RW)
    * :PPL:`sm` – Share mode (S_DN, S_DR, S_DW, S_DB)

  **Remarks**
    Opens a file for appending data to the end without destroying existing content. Creates 
    the file if it doesn't exist. Channel 0 is reserved for script questionnaires but 
    available otherwise. FAPPEND requires O_RW access regardless of specification.

  **Example**

    .. code-block:: PPL

       FAPPEND 1,"C:\PCB\MAIN\PPE.LOG",O_RW,S_DB
       FPUTLN 1,"Ran "+PPENAME()+" on "+STRING(DATE())+" at "+STRING(TIME())
       FCLOSE 1

  **See Also**
    * :PPL:`FCLOSE` – Close file
    * :PPL:`FCREATE` – Create file
    * :PPL:`FOPEN` – Open file
    * :PPL:`FREWIND` – Rewind file

FCLOSE (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT FCLOSE(INTEGER chan)`

  Close an open file.

  **Parameters**
    * :PPL:`chan` – Open channel to close (0-7)

  **Remarks**
    Closes a file channel opened with FCREATE, FOPEN, or FAPPEND. PPL automatically 
    closes all open files at program end, but explicit closing is recommended when 
    processing multiple files.

  **Example**

    .. code-block:: PPL

       FOPEN 1,"C:\PCB\MAIN\PPE.LOG",O_RD,S_DW
       FGET 1,hdr
       FCLOSE 1
       IF (hdr <> "Creating PPE.LOG file...") THEN
           PRINTLN "Error: PPE.LOG invalid"
           END
       ENDIF

  **See Also**
    * :PPL:`FAPPEND` – Append to file
    * :PPL:`FCREATE` – Create file
    * :PPL:`FOPEN` – Open file

FCLOSEALL (3.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FCLOSEALL`

  Close all open file channels.

  **Remarks**
    Closes all file channels (0-7) that are currently open. Useful for cleanup when working 
    with multiple files or ensuring all files are closed before program termination. The 
    statement automatically checks which channels are open and closes only those. While PPL 
    automatically closes all open files at program end, explicit closing with FCLOSEALL is 
    recommended for proper resource management.

  **Example**

    .. code-block:: PPL

       FOPEN 1, "AUTOEXEC.BAT", O_RD, S_DW
       FOPEN 2, "CONFIG.SYS", O_RD, S_DW
       FCREATE 3, "OUTPUT.TXT", O_WR, S_DN
       
       ; Process files...
       
       FCLOSEALL  ; Close all three files at once

  **See Also**
    * :PPL:`FCLOSE` – Close specific file channel
    * :PPL:`FOPEN` – Open existing file
    * :PPL:`FCREATE` – Create new file
    * :PPL:`FAPPEND` – Open file for append

FCREATE (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FCREATE(INTEGER chan, STRING file, INTEGER am, INTEGER sm)`

  Create and open a new file.

  **Parameters**
    * :PPL:`chan` – Channel number (0-7)
    * :PPL:`file` – File specification to create
    * :PPL:`am` – Access mode (O_RD, O_WR, O_RW)
    * :PPL:`sm` – Share mode (S_DN, S_DR, S_DW, S_DB)

  **Remarks**
    Creates a new file, destroying any existing file with the same name. Channel 0 is 
    reserved for script questionnaires but available otherwise. Using O_RD doesn't make 
    sense for a newly created empty file.

  **Example**

    .. code-block:: PPL

       FCREATE 1,"C:\PCB\MAIN\PPE.LOG",O_WR,S_DN
       FPUTLN 1,"Creating PPE.LOG file..."
       FCLOSE 1

  **See Also**
    * :PPL:`FAPPEND` – Append to file
    * :PPL:`FCLOSE` – Close file
    * :PPL:`FOPEN` – Open file

FDEFIN (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT FDEFIN(INTEGER chan)`

  Set default input file channel.

  **Parameters**
    * :PPL:`chan` – Channel to use as default for FD* input statements (0-7)

  **Remarks**
    Specifies which file channel to use for FDGET and FDREAD statements. Eliminates need 
    to specify channel in every read operation, improving performance and code clarity 
    when reading from a single file. Channel must be open before being set as default.

  **Example**

    .. code-block:: PPL

       STRING line
       
       FOPEN 1, "INPUT.DAT", O_RD, S_DW
       FDEFIN 1  ; Set channel 1 as default input
       
       ; Now can use FDGET without channel parameter
       FDGET line
       WHILE (!FERR(1)) DO
           PRINTLN line
           FDGET line
       ENDWHILE
       
       FCLOSE 1

  **See Also**
    * :PPL:`FDEFOUT` – Set default output channel
    * :PPL:`FDGET` – Get line using default channel
    * :PPL:`FDREAD` – Read binary using default channel

FDEFOUT (2.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FDEFOUT(INTEGER chan)`

  Set default output file channel.

  **Parameters**
    * :PPL:`chan` – Channel to use as default for FD* output statements (0-7)

  **Remarks**
    Specifies which file channel to use for FDPUT, FDPUTLN, and FDPUTPAD statements. 
    Eliminates need to specify channel in every write operation, improving performance 
    and code clarity when writing to a single file. Channel must be open with write 
    access before being set as default.

  **Example**

    .. code-block:: PPL

       FOPEN 2, "OUTPUT.DAT", O_WR, S_DN
       FDEFOUT 2  ; Set channel 2 as default output
       
       ; Now can use FD* output without channel parameter
       FDPUTLN "Header Line"
       FDPUTLN "Data Line 1"
       FDPUTPAD "Padded", 20
       
       FCLOSE 2

  **See Also**
    * :PPL:`FDEFIN` – Set default input channel
    * :PPL:`FDPUT` – Put using default channel (if implemented)
    * :PPL:`FDPUTLN` – Put line using default channel (if implemented)
    * :PPL:`FDPUTPAD` – Put padded using default channel

FDGET (2.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT FDGET(VAR var)`

  Read line from default input channel.

  **Parameters**
    * :PPL:`var` – Variable to receive the line

  **Remarks**
    Reads a text line from the file channel set by FDEFIN. Functionally identical to 
    FGET but uses the default channel instead of requiring a channel parameter. Improves 
    performance when reading multiple lines from the same file.

  **Example**

    .. code-block:: PPL

       STRING data
       INTEGER count
       
       FOPEN 1, "DATA.TXT", O_RD, S_DW
       FDEFIN 1
       
       count = 0
       FDGET data  ; Read from default channel 1
       WHILE (!FERR(1)) DO
           INC count
           PRINTLN "Line ", count, ": ", data
           FDGET data
       ENDWHILE
       
       FCLOSE 1

  **See Also**
    * :PPL:`FDEFIN` – Set default input channel
    * :PPL:`FGET` – Read with explicit channel
    * :PPL:`FDREAD` – Binary read from default channel

FDREAD (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT FDREAD(VAR var, INTEGER size)`

  Read binary data from default input channel.

  **Parameters**
    * :PPL:`var` – Variable to receive data
    * :PPL:`size` – Number of bytes to read (0-2048)

  **Remarks**
    Reads binary data from the file channel set by FDEFIN. Functionally identical to 
    FREAD but uses the default channel. Improves performance for multiple binary reads 
    from the same file.

  **Example**

    .. code-block:: PPL

       STRING header
       INTEGER value
       
       FOPEN 1, "BINARY.DAT", O_RD, S_DW
       FDEFIN 1
       
       ; Read using default channel
       FDREAD header, 10    ; 10-byte header
       FDREAD value, 2      ; 2-byte integer
       
       PRINTLN "Header: ", header
       PRINTLN "Value: ", value
       
       FCLOSE 1

  **See Also**
    * :PPL:`FDEFIN` – Set default input channel
    * :PPL:`FREAD` – Read with explicit channel
    * :PPL:`FDGET` – Text read from default channel

FDPUTPAD (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FDPUTPAD(ANY exp, INTEGER width)`

  Write padded line to default output channel.

  **Parameters**
    * :PPL:`exp` – Expression to write
    * :PPL:`width` – Field width for padding (-256 to 256)

  **Remarks**
    Writes expression padded to specified width using the channel set by FDEFOUT. 
    Positive width right-justifies (left-pads), negative width left-justifies (right-pads). 
    Functionally identical to FPUTPAD but uses default channel.

  **Example**

    .. code-block:: PPL

       FOPEN 2, "REPORT.TXT", O_WR, S_DN
       FDEFOUT 2
       
       ; Write formatted report using default channel
       FDPUTPAD "Name", -20
       FDPUTPAD "Score", 10
       FDPUTPAD "Grade", 8
       
       FDPUTPAD U_NAME(), -20
       FDPUTPAD CURSEC(), 10
       FDPUTPAD "A", 8
       
       FCLOSE 2

  **See Also**
    * :PPL:`FDEFOUT` – Set default output channel
    * :PPL:`FPUTPAD` – Write padded with explicit channel
    * :PPL:`FDPUTLN` – Write line to default channel (if implemented)
FDPUT (2.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT FDPUT(ANY exp [, ANY exp...])`

  Write expression(s) to default output channel without newline.

  **Parameters**
    * :PPL:`exp` – Expression(s) to write (at least one required)

  **Remarks**
    Writes expressions to the file channel set by FDEFOUT. Functionally identical to 
    FPUT but uses the default channel instead of requiring a channel parameter. Does 
    not append carriage return/line feed. Improves code clarity when writing multiple 
    items to the same file.

  **Example**

    .. code-block:: PPL

       FOPEN 2, "DATA.TXT", O_WR, S_DN
       FDEFOUT 2
       
       ; Write using default channel
       FDPUT U_NAME(), " "
       FDPUT DATE(), " " 
       FDPUT TIME()
       ; Results in single line: "John Doe 01-15-24 14:30:00"
       
       FCLOSE 2

  **See Also**
    * :PPL:`FDEFOUT` – Set default output channel
    * :PPL:`FPUT` – Write with explicit channel
    * :PPL:`FDPUTLN` – Write line to default channel

FDPUTLN (2.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FDPUTLN([ANY exp...])`

  Write expression(s) to default output channel with newline.

  **Parameters**
    * :PPL:`exp` – Expression(s) to write (optional)

  **Remarks**
    Writes expressions to the file channel set by FDEFOUT with carriage return/line feed 
    appended. Functionally identical to FPUTLN but uses the default channel. Can be called 
    without arguments to write a blank line. Simplifies code when writing multiple lines 
    to the same file.

  **Example**

    .. code-block:: PPL

       FOPEN 2, "LOG.TXT", O_WR, S_DN
       FDEFOUT 2
       
       ; Write log entries using default channel
       FDPUTLN "Session started at ", TIME()
       FDPUTLN "User: ", U_NAME()
       FDPUTLN  ; Blank line
       FDPUTLN "Actions:"
       
       FCLOSE 2

  **See Also**
    * :PPL:`FDEFOUT` – Set default output channel
    * :PPL:`FPUTLN` – Write line with explicit channel
    * :PPL:`FDPUT` – Write without newline

FDPUTPAD (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FDPUTPAD(ANY exp, INTEGER width)`

  Write padded line to default output channel.

  **Parameters**
    * :PPL:`exp` – Expression to write
    * :PPL:`width` – Field width for padding (-256 to 256)

  **Remarks**
    Writes expression padded to specified width using the channel set by FDEFOUT. 
    Positive width right-justifies (left-pads), negative width left-justifies (right-pads). 
    Functionally identical to FPUTPAD but uses default channel. Appends newline after 
    padded text.

  **Example**

    .. code-block:: PPL

       FOPEN 2, "REPORT.TXT", O_WR, S_DN
       FDEFOUT 2
       
       ; Write formatted report using default channel
       FDPUTPAD "Name", -20
       FDPUTPAD "Score", 10
       FDPUTPAD "Grade", 8
       
       FDPUTPAD U_NAME(), -20
       FDPUTPAD CURSEC(), 10
       FDPUTPAD "A", 8
       
       FCLOSE 2

  **See Also**
    * :PPL:`FDEFOUT` – Set default output channel
    * :PPL:`FPUTPAD` – Write padded with explicit channel
    * :PPL:`FDPUTLN` – Write line to default channel

FDWRITE (2.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FDWRITE(ANY exp, INTEGER size)`

  Write binary data to default output channel.

  **Parameters**
    * :PPL:`exp` – Expression to write
    * :PPL:`size` – Number of bytes to write

  **Remarks**
    Writes binary data to the file channel set by FDEFOUT. Functionally identical to 
    FWRITE but uses the default channel. Expression is evaluated and written as binary 
    bytes, not text. Essential for binary file operations when working with a single 
    output file. File pointer advances by number of bytes written.

  **Example**

    .. code-block:: PPL

       INTEGER recordNum
       STRING header
       
       FOPEN 2, "BINARY.DAT", O_WR, S_DN
       FDEFOUT 2
       
       ; Write binary data using default channel
       header = "DATAFILE01"
       FDWRITE header, 10      ; Fixed 10-byte header
       
       recordNum = 100
       FDWRITE recordNum, 2    ; 2-byte integer
       
       ; Write multiple records
       FOR i = 1 TO recordNum
           FDWRITE i, 2
           FDWRITE "REC", 3
       NEXT
       
       FCLOSE 2

  **See Also**
    * :PPL:`FDEFOUT` – Set default output channel
    * :PPL:`FWRITE` – Write binary with explicit channel
    * :PPL:`FDREAD` – Read binary from default input
    * :PPL:`FSEEK` – Position file pointer

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

FFLUSH (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT FFLUSH(INTEGER chan)`

  Flush file buffer to disk immediately.

  **Parameters**
    * :PPL:`chan` – Open file channel to flush (0-7)

  **Remarks**
    Forces all buffered data for the specified channel to be written to disk immediately. 
    Useful for ensuring critical data is saved before continuing, especially in multi-user 
    environments or when sharing files between processes. Without FFLUSH, data may remain 
    in memory buffers until the file is closed or the buffer fills.

  **Example**

    .. code-block:: PPL

       FAPPEND 1, "ACTIVITY.LOG", O_RW, S_DB
       FPUTLN 1, "Critical event at ", TIME()
       FFLUSH 1  ; Ensure log entry is written immediately
       ; Continue processing...
       FCLOSE 1

  **See Also**
    * :PPL:`FCLOSE` – Close file (automatically flushes)
    * :PPL:`FOPEN` – Open file
    * :PPL:`FPUT` / :PPL:`FPUTLN` – Write to file

FGET (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT FGET(INTEGER chan, VAR var)`

  Get (read) a line from an open file.

  **Parameters**
    * :PPL:`chan` – Channel to read from (0-7)
    * :PPL:`var` – Variable to store the line read

  **Remarks**
    Reads information a line at a time from a file opened with read access. If multiple 
    fields exist on the line, you must parse them manually. Sets file error flag if 
    end of file is reached.

  **Example**

    .. code-block:: PPL

       INTEGER i
       STRING s
       FOPEN 1,"FILE.DAT",O_RD,S_DW
       IF (FERR(1)) THEN
           PRINTLN "Error, exiting..."
           END
       ENDIF
       FGET 1,s
       WHILE (!FERR(1)) DO
           INC i
           PRINTLN "Line ",RIGHT(i,3),": ",s
           FGET 1,s
       ENDWHILE
       FCLOSE 1

  **See Also**
    * :PPL:`FPUT` – Write to file
    * :PPL:`FPUTLN` – Write line to file
    * :PPL:`FPUTPAD` – Write padded line

FOPEN (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT FOPEN(INTEGER chan, STRING file, INTEGER am, INTEGER sm)`

  Open an existing file.

  **Parameters**
    * :PPL:`chan` – Channel number (0-7)
    * :PPL:`file` – File specification to open
    * :PPL:`am` – Access mode (O_RD, O_WR, O_RW)
    * :PPL:`sm` – Share mode (S_DN, S_DR, S_DW, S_DB)

  **Remarks**
    Opens a file for read/write access with specified sharing. O_RD expects the file to 
    exist; O_WR and O_RW create the file if it doesn't exist. Channel 0 is reserved for 
    script questionnaires but available otherwise.

  **Example**

    .. code-block:: PPL

       STRING hdr
       FOPEN 1,"C:\PCB\MAIN\PPE.LOG",O_RD,S_DW
       FGET 1,hdr
       FCLOSE 1
       IF (hdr <> "Creating PPE.LOG file...") THEN
           PRINTLN "Error: PPE.LOG invalid"
           END
       ENDIF

  **See Also**
    * :PPL:`FAPPEND` – Open for append
    * :PPL:`FCLOSE` – Close file
    * :PPL:`FCREATE` – Create new file
    * :PPL:`FREWIND` – Rewind file

FORWARD (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FORWARD(INTEGER numcols)`

  Move the cursor forward a specified number of columns.

  **Parameters**
    * :PPL:`numcols` – Number of columns to move forward (1-79)

  **Remarks**
    Moves cursor forward non-destructively. Uses ANSI positioning if available, otherwise 
    re-displays existing characters. Cannot move beyond column 80. Has no effect if 
    already at column 80.

  **Example**

    .. code-block:: PPL

       PRINT "PIRNT is wrong"
       DELAY 5*182/10
       BACKUP 13
       PRINT "PRI"
       FORWARD 6
       PRINT "RIGHT"
       DELAY 5*182/10
       NEWLINE

  **See Also**
    * :PPL:`ANSION()` – Check ANSI support
    * :PPL:`ANSIPOS` – Position cursor
    * :PPL:`BACKUP` – Move backward
    * :PPL:`GETX()` – Get cursor column
    * :PPL:`GETY()` – Get cursor row

FREAD (2.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT FREAD(INTEGER chan, VAR var, INTEGER size)`

  Read binary data from file.

  **Parameters**
    * :PPL:`chan` – Open file channel (0-7)
    * :PPL:`var` – Variable to receive data
    * :PPL:`size` – Number of bytes to read (0-2048)

  **Remarks**
    Reads raw binary data from current file position. Unlike FGET which reads text lines, 
    FREAD reads exact byte counts regardless of content. Useful for binary formats, fixed-
    length records, or data structures. File pointer advances by number of bytes read.

  **Example**

    .. code-block:: PPL

       STRING header
       INTEGER recordSize
       
       FOPEN 1, "BINARY.DAT", O_RD, S_DW
       
       ; Read 10-byte header
       FREAD 1, header, 10
       
       ; Read 2-byte integer (size field)
       FREAD 1, recordSize, 2
       
       FCLOSE 1

  **See Also**
    * :PPL:`FWRITE` – Write binary data
    * :PPL:`FSEEK` – Position file pointer
    * :PPL:`FDREAD` – Read with default channel

FSEEK (2.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT FSEEK(INTEGER chan, INTEGER bytes, INTEGER position)`

  Move file pointer to specific position.

  **Parameters**
    * :PPL:`chan` – Open file channel (0-7)
    * :PPL:`bytes` – Number of bytes to move (positive or negative)
    * :PPL:`position` – Base position: SEEK_SET (0), SEEK_CUR (1), or SEEK_END (2)

  **Position Constants**
    * :PPL:`SEEK_SET` (0) – Beginning of file
    * :PPL:`SEEK_CUR` (1) – Current file position
    * :PPL:`SEEK_END` (2) – End of file

  **Remarks**
    Positions the file pointer for random access operations. Allows moving forward or 
    backward from the specified base position. Essential for binary file operations and 
    updating specific records in data files.

  **Example**

    .. code-block:: PPL

       FOPEN 1, "DATA.DAT", O_RW, S_DN
       
       ; Move to byte 100 from start
       FSEEK 1, 100, SEEK_SET
       
       ; Move back 50 bytes from current position
       FSEEK 1, -50, SEEK_CUR
       
       ; Position 10 bytes before end of file
       FSEEK 1, -10, SEEK_END
       
       FCLOSE 1

  **See Also**
    * :PPL:`FREAD` – Read binary data
    * :PPL:`FWRITE` – Write binary data
    * :PPL:`FREWIND` – Return to beginning


FPUT (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT FPUT(INTEGER chan, ANY exp [, ANY exp...])`

  Write expression(s) to an open file without newline.

  **Parameters**
    * :PPL:`chan` – Channel to write to (0-7)
    * :PPL:`exp` – Expression(s) to write (at least one required)

  **Remarks**
    Evaluates one or more expressions of any type and writes results to the specified 
    channel. Does not append carriage return/line feed. At least one expression required.

  **Example**

    .. code-block:: PPL

       FAPPEND 1,"FILE.DAT",O_WR,S_DB
       FPUT 1,U_NAME()," ",DATE()
       FPUT 1," Logged!"
       FCLOSE 1

  **See Also**
    * :PPL:`FGET` – Read from file
    * :PPL:`FPUTLN` – Write line with newline
    * :PPL:`FPUTPAD` – Write padded line

FPUTLN (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT FPUTLN(INTEGER chan [, ANY exp...])`

  Write expression(s) to an open file with newline.

  **Parameters**
    * :PPL:`chan` – Channel to write to (0-7)
    * :PPL:`exp` – Expression(s) to write (optional)

  **Remarks**
    Evaluates zero or more expressions and writes results to the specified channel with 
    carriage return/line feed appended. Can be called with just channel number to write 
    blank line.

  **Example**

    .. code-block:: PPL

       FAPPEND 1,"FILE.DAT",O_WR,S_DB
       FPUTLN 1,U_NAME()," ",DATE()," ",TIME()," ",CURSEC()
       FPUTLN 1
       FPUTLN 1,"Have a nice"+" day!"
       FCLOSE 1

  **See Also**
    * :PPL:`FGET` – Read from file
    * :PPL:`FPUT` – Write without newline
    * :PPL:`FPUTPAD` – Write padded line

FPUTPAD (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FPUTPAD(INTEGER chan, ANY exp, INTEGER width)`

  Write a padded line of specified width to a file.

  **Parameters**
    * :PPL:`chan` – Channel to write to (0-7)
    * :PPL:`exp` – Expression to write
    * :PPL:`width` – Width for padding (-256 to 256)

  **Remarks**
    Writes expression padded to specified width with spaces, then appends newline. 
    Positive width: right-justified (left-padded). Negative width: left-justified 
    (right-padded).

  **Example**

    .. code-block:: PPL

       FAPPEND 1,"FILE.DAT",O_WR,S_DB
       FPUTPAD 1,U_NAME(),40
       FPUTPAD 1,DATE(),20
       FPUTPAD 1,TIME(),-20
       FCLOSE 1

  **See Also**
    * :PPL:`FGET` – Read from file
    * :PPL:`FPUT` – Write without newline
    * :PPL:`FPUTLN` – Write with newline

FREALTUSER (3.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FREALTUSER`

  Free the alternate user record loaded by GETALTUSER.

  **Remarks**
    Since only one GETALTUSER can be active at a time, FREALTUSER releases the 
    alternate user record, allowing other processes that need GETALTUSER (such as 
    the MESSAGE command) to function properly. Always call FREALTUSER after you're 
    done with the alternate user data to avoid blocking other operations.

  **Example**

    .. code-block:: PPL

       STRING name
       
       ; Load alternate user record
       GETALTUSER 20
       name = U_NAME()
       
       ; Free the record before MESSAGE command
       FREALTUSER
       
       ; Now MESSAGE can use GETALTUSER internally
       MESSAGE 1, name, "Subject", "R", 0, FALSE, FALSE, "message.txt"

  **See Also**
    * :PPL:`GETALTUSER` – Load alternate user record
    * :PPL:`MESSAGE` – Send message
    * :PPL:`GETUSER` – Load current user record
    * :PPL:`PUTUSER` – Save user record

FRESHLINE (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FRESHLINE`

  Move cursor to a fresh line for output.

  **Remarks**
    Checks if cursor is in column 1. If not, calls NEWLINE to move to next line. 
    Ensures clean line before continuing output.

  **Example**

    .. code-block:: PPL

       INTEGER i, end
       LET end = RANDOM(20)
       FOR i = 1 TO end
           PRINT RIGHT(RANDOM(10000),8)
       NEXT
       FRESHLINE
       PRINTLN "Now we continue..."

  **See Also**
    * :PPL:`NEWLINE` – Move to next line
    * :PPL:`NEWLINES` – Move multiple lines

FREWIND (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT FREWIND(INTEGER chan)`

  Rewind an open file to the beginning.

  **Parameters**
    * :PPL:`chan` – Open channel to rewind (0-7)

  **Remarks**
    Rewinds file channel opened with FCREATE, FOPEN, or FAPPEND. Flushes buffers, 
    commits file to disk, and repositions pointer to beginning. Useful for reprocessing 
    a file without closing and reopening.

  **Example**

    .. code-block:: PPL

       STRING s
       FAPPEND 1,"C:\PCB\MAIN\PPE.LOG",O_RW,S_DN
       FPUTLN 1,U_NAME()
       FREWIND 1
       WHILE (!FERR(1)) DO
           FGET 1,s
           PRINTLN s
       ENDWHILE
       FCLOSE 1

  **See Also**
    * :PPL:`FAPPEND` – Open for append
    * :PPL:`FCLOSE` – Close file
    * :PPL:`FCREATE` – Create file
    * :PPL:`FOPEN` – Open file

FWRITE (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT FWRITE(INTEGER chan, ANY exp, INTEGER size)`

  Write binary data to file.

  **Parameters**
    * :PPL:`chan` – Open file channel (0-7)
    * :PPL:`exp` – Expression to write
    * :PPL:`size` – Number of bytes to write

  **Remarks**
    Writes raw binary data to current file position. Expression is evaluated and written 
    as binary bytes, not text. Essential for creating binary files, fixed-length records, 
    or structured data files. File pointer advances by number of bytes written.

  **Example**

    .. code-block:: PPL

       INTEGER recordNum
       STRING data
       
       FOPEN 1, "BINARY.DAT", O_WR, S_DN
       
       ; Write fixed-size header
       data = "DATAFILE01"
       FWRITE 1, data, 10
       
       ; Write record count as 2-byte integer
       recordNum = 100
       FWRITE 1, recordNum, 2
       
       FCLOSE 1

  **See Also**
    * :PPL:`FREAD` – Read binary data
    * :PPL:`FSEEK` – Position file pointer
    * :PPL:`FDWRITE` – Write with default channel (if implemented)

GETTOKEN (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT GETTOKEN(VAR var)`

  Retrieve the next token from a tokenized string.

  **Parameters**
    * :PPL:`var` – Variable to store the retrieved token

  **Remarks**
    Retrieves tokens one at a time from a string previously processed with TOKENIZE. 
    The token count decreases with each retrieval. Use TOKCOUNT() to check remaining 
    tokens.

  **Example**

    .. code-block:: PPL

       STRING cmdline
       INPUT "Command",cmdline
       TOKENIZE cmdline
       PRINTLN "You entered ",TOKCOUNT()," tokens"
       WHILE (TOKCOUNT() > 0) DO
           GETTOKEN cmdline
           PRINTLN "Token: ",CHR(34),cmdline,CHR(34)
       ENDWHILE

  **See Also**
    * :PPL:`GETTOKEN()` – Function version
    * :PPL:`TOKCOUNT()` – Count remaining tokens
    * :PPL:`TOKENIZE` – Parse string into tokens
    * :PPL:`TOKENSTR()` – Rebuild tokenized string

GETUSER (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT GETUSER`

  Fill predeclared user variables with values from current user record.

  **Remarks**
    Loads current user's information into predeclared U_XXX variables. Variables are 
    undefined until GETUSER is executed. Changes don't take effect until PUTUSER 
    is called.

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
    * :PPL:`PUTUSER` – Save user record
    * :PPL:`GETALTUSER` – Load alternate user
    * :PPL:`U_...` variables

GOODBYE (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT GOODBYE`

  Log user off with confirmation and download warnings.

  **Remarks**
    Logs off user as if they typed G command. Warns about flagged files and optionally 
    confirms logoff. Performs same processing as PCBoard's G command.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "What do you want to do",s
       IF (s = "G") THEN GOODBYE
       ELSEIF (s = "BYE") THEN BYE
       ELSE KBDSTUFF s
       ENDIF

  **See Also**
    * :PPL:`BYE` – Immediate logoff
    * :PPL:`DTROFF` – Turn off DTR
    * :PPL:`DTRON` – Turn on DTR
    * :PPL:`HANGUP` – Disconnect immediately

GOSUB (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT GOSUB(LABEL label)`

  Transfer control to subroutine and save return address.

  **Parameters**
    * :PPL:`label` – Label to jump to

  **Remarks**
    Saves address of next line and transfers control to specified label. RETURN statement 
    at end of subroutine resumes execution at line following GOSUB. Useful for code reuse.

  **Example**

    .. code-block:: PPL

       STRING Question, Answer
       LET Question = "What is your street address..."
       GOSUB ask
       LET Question = "What is your city, state and zip..."
       GOSUB ask
       END
       
       :ask
       LET Answer = ""
       PRINTLN "@X0E",Question
       INPUT "",Answer
       NEWLINES 2
       FPUTLN 0,"Q: ",STRIPATX(Question)
       FPUTLN 0,"A: ",Answer
       RETURN

  **See Also**
    * :PPL:`GOTO` – Unconditional jump
    * :PPL:`RETURN` – Return from subroutine
    * :PPL:`FOR`/`NEXT` – Loop statements
    * :PPL:`IF`/`ENDIF` – Conditional statements
    * :PPL:`WHILE`/`ENDWHILE` – Loop statements

GOTO (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT GOTO(LABEL label)`

  Transfer program control unconditionally.

  **Parameters**
    * :PPL:`label` – Label to jump to

  **Remarks**
    Unconditional jump to specified label. Often overused - consider using structured 
    programming constructs (IF, WHILE, FOR) instead. Useful for exiting deeply nested 
    loops on critical errors.

  **Example**

    .. code-block:: PPL

       INTEGER i
       STRING s
       FOPEN 1,"FILE.DAT",O_RD,S_DW
       WHILE (UPPER(s) <> "QUIT") DO
           FGET 1,s
           IF (FERR(1)) THEN
               PRINTLN "Error, aborting..."
               GOTO exit
           ENDIF
           INC i
           PRINTLN "Line ",i,": ",s
       ENDWHILE
       :exit
       FCLOSE 1

  **See Also**
    * :PPL:`GOSUB` – Call subroutine
    * :PPL:`FOR`/`NEXT` – Loop statements
    * :PPL:`IF`/`ENDIF` – Conditional statements
    * :PPL:`WHILE`/`ENDWHILE` – Loop statements

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

HANGUP (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT HANGUP`

  Immediately disconnect user with abnormal logoff.

  **Remarks**
    Immediately hangs up on caller without delay or notice. Performs logoff processing 
    and logs abnormal logoff to caller's log.

  **Example**

    .. code-block:: PPL

       STRING s
       INPUT "What do you want to do",s
       IF (s = "G") THEN GOODBYE
       ELSEIF (s = "BYE") THEN BYE
       ELSEIF (s = "HANG") THEN HANGUP
       ELSE KBDSTUFF s
       ENDIF

  **See Also**
    * :PPL:`BYE` – Immediate logoff
    * :PPL:`DTROFF` – Turn off DTR
    * :PPL:`DTRON` – Turn on DTR
    * :PPL:`GOODBYE` – Normal logoff

INC (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT INC(VAR var)`

  Increment a variable by 1.

  **Parameters**
    * :PPL:`var` – Variable to increment

  **Remarks**
    Provides shorter, more efficient method of increasing a value by 1 than using 
    LET i = i + 1. Commonly used in loops and counters.

  **Example**

    .. code-block:: PPL

       INTEGER i
       PRINTLN "Countdown:"
       LET i = 0
       WHILE (i <= 10) DO
           PRINTLN "T plus ",i
           INC i
       ENDWHILE

  **See Also**
    * :PPL:`DEC` – Decrement variable

INPUT (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT INPUT(STRING prompt, VAR var)`

  Prompt user for text input.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input

  **Remarks**
    Accepts any string up to 60 characters. Displays parentheses around input field in 
    ANSI mode. Limit prompts to 15 characters or less due to parentheses.

  **Example**

    .. code-block:: PPL

       BOOLEAN b
       DATE d
       INTEGER i
       MONEY m
       STRING s
       TIME t
       INPUT "Enter BOOLEAN",b
       INPUT "Enter DATE",d
       INPUT "Enter INTEGER",i
       INPUT "Enter MONEY",m
       INPUT "Enter STRING",s
       INPUT "Enter TIME",t

  **See Also**
    * :PPL:`INPUTSTR` – Input with validation
    * :PPL:`INPUTTEXT` – Multi-line input
    * :PPL:`PROMPTSTR` – Display prompt

INPUTCC (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTCC(STRING prompt, VAR var, INTEGER color)`

  Input credit card number with validation.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input
    * :PPL:`color` – Display color

  **Remarks**
    Accepts credit card number input. Valid characters: "0123456789". Maximum length: 16. 
    Limit prompt to 80-16-4=60 characters.

  **See Also**
    * :PPL:`INPUTDATE` – Input date
    * :PPL:`INPUTINT` – Input integer
    * :PPL:`INPUTMONEY` – Input money
    * :PPL:`INPUTTIME` – Input time
    * :PPL:`INPUTYN` – Input yes/no

INPUTDATE (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTDATE(STRING prompt, VAR var, INTEGER color)`

  Input date with validation.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input
    * :PPL:`color` – Display color

  **Remarks**
    Accepts date input. Valid characters: "0123456789-/". Maximum length: 8. 
    Limit prompt to 80-8-4=68 characters.

INPUTINT (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTINT(STRING prompt, VAR var, INTEGER color)`

  Input integer with validation.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input
    * :PPL:`color` – Display color

  **Remarks**
    Accepts integer input. Valid characters: "0123456789+-". Maximum length: 11. 
    Limit prompt to 80-11-4=65 characters.

INPUTMONEY (1.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTMONEY(STRING prompt, VAR var, INTEGER color)`

  Input money amount with validation.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input
    * :PPL:`color` – Display color

  **Remarks**
    Accepts money input. Valid characters: "0123456789+-$.". Maximum length: 13. 
    Limit prompt to 80-13-4=63 characters.

INPUTTIME (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTTIME(STRING prompt, VAR var, INTEGER color)`

  Input time with validation.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input
    * :PPL:`color` – Display color

  **Remarks**
    Accepts time input. Valid characters: "0123456789:". Maximum length: 8. 
    Limit prompt to 80-8-4=68 characters.

INPUTYN (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTYN(STRING prompt, VAR var, INTEGER color)`

  Input yes/no response with language support.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input
    * :PPL:`color` – Display color

  **Remarks**
    Accepts yes/no input. Valid characters depend on language selection (usually "YN" 
    for English). Maximum length: 1. Characters defined in PCBML.DAT for each language.

  **Example**

    .. code-block:: PPL

       STRING yn
       INPUTYN "Continue (Y/N)",yn,@X0E
       IF (yn = YESCHAR()) THEN
           PRINTLN "Continuing..."
       ENDIF

INPUTSTR (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTSTR(STRING prompt, VAR var, INTEGER color, INTEGER len, STRING valid, INTEGER flags)`

  Prompt user for formatted text input with validation.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input
    * :PPL:`color` – Display color for prompt
    * :PPL:`len` – Maximum input length
    * :PPL:`valid` – Valid characters allowed
    * :PPL:`flags` – Input behavior flags

  **Remarks**
    Accepts string input up to specified length. Only characters in valid parameter are accepted. 
    Flags modify prompt display and input behavior. Use predefined mask functions for common 
    character sets: MASK_ALNUM(), MASK_ALPHA(), MASK_ASCII(), MASK_FILE(), MASK_NUM(), 
    MASK_PATH(), MASK_PWD(). Flag values: AUTO, DEFS, ECHODOTS, ERASELINE, FIELDLEN, 
    GUIDE, HIGHASCII, LFAFTER, LFBEFORE, NEWLINE, NOCLEAR, STACKED, UPCASE, WORDWRAP, YESNO.

  **Example**

    .. code-block:: PPL

       BOOLEAN b
       DATE d
       INTEGER i
       MONEY m
       STRING s
       TIME t
       INPUTSTR "Enter BOOLEAN",b,@X0E,1,"10",LFBEFORE+NEWLINE
       INPUTSTR "Enter DATE",d,@X0F,8,"0123456789-",NEWLINE+NOCLEAR
       INPUTSTR "Enter INTEGER",i,@X07,20,MASK_NUM(),NEWLINE
       INPUTSTR "Enter MONEY",m,@X08,9,MASK_NUM()+".",NEWLINE+DEFS+FIELDLEN
       INPUTSTR "Enter STRING",s,@X09,63,MASK_ASCII(),NEWLINE+FIELDLEN+GUIDE
       INPUTSTR "Enter TIME",t,@X0A,5,"0123456789"+":",NEWLINE+LFAFTER

  **See Also**
    * :PPL:`INPUT` – Basic input
    * :PPL:`INPUTTEXT` – Simpler text input
    * :PPL:`PROMPTSTR` – Display prompt

INPUTTEXT (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT INPUTTEXT(STRING prompt, VAR var, INTEGER color, INTEGER len)`

  Prompt user for text input with specified length and color.

  **Parameters**
    * :PPL:`prompt` – Prompt to display
    * :PPL:`var` – Variable to store input
    * :PPL:`color` – Display color for prompt
    * :PPL:`len` – Maximum input length

  **Remarks**
    Accepts any string input up to specified length. Displays parentheses around input field 
    in ANSI mode. Limit prompts to (80-len-4) characters or less to accommodate parentheses.

  **Example**

    .. code-block:: PPL

       BOOLEAN b
       DATE d
       INTEGER i
       MONEY m
       STRING s
       TIME t
       INPUTTEXT "Enter BOOLEAN",b,@X0E,1
       INPUTTEXT "Enter DATE",d,@X0F,8
       INPUTTEXT "Enter INTEGER",i,@X07,20
       INPUTTEXT "Enter MONEY",m,@X08,9
       INPUTTEXT "Enter STRING",s,@X09,63
       INPUTTEXT "Enter TIME",t,@X0A,5

  **See Also**
    * :PPL:`INPUT` – Basic input
    * :PPL:`INPUTSTR` – Advanced formatted input
    * :PPL:`PROMPTSTR` – Display prompt

JOIN (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT JOIN(STRING cmds)`

  Execute the join conference command with sub-commands.

  **Parameters**
    * :PPL:`cmds` – Sub-commands for the join conference command

  **Remarks**
    Accesses the join conference command (J command) under PPE control. Destroys any 
    previously tokenized string expression. Save tokens before using JOIN if needed.

  **Example**

    .. code-block:: PPL

       STRING yn
       INPUTYN "Join SysOp conference",yn,@X0E
       IF (yn = YESCHAR()) JOIN "4"

  **See Also**
    * :PPL:`BLT` – Display bulletin
    * :PPL:`DIR` – File directories
    * :PPL:`QUEST` – Run questionnaire

KBDCHKOFF (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDCHKOFF`

  Turn off keyboard timeout checking.

  **Remarks**
    Disables PCBoard's automatic keyboard timeout detection. Use for processes that take 
    time without user interaction. Re-enable with KBDCHKON when done to prevent PCBoard 
    from recycling due to perceived inactivity.

  **Example**

    .. code-block:: PPL

       KBDCHKOFF
       WHILE (RANDOM(10000) <> 0) PRINT "."  ; Time-consuming process
       KBDCHKON

  **See Also**
    * :PPL:`CDCHKOFF` – Turn off carrier checking
    * :PPL:`CDCHKON` – Turn on carrier checking
    * :PPL:`KBDCHKON` – Turn on keyboard checking

KBDCHKON (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDCHKON`

  Turn on keyboard timeout checking.

  **Remarks**
    Re-enables PCBoard's automatic keyboard timeout detection after it was disabled with 
    KBDCHKOFF. Should be called after completing processes that don't require user input.

  **Example**

    .. code-block:: PPL

       KBDCHKOFF
       ; Long process without user input
       KBDCHKON

  **See Also**
    * :PPL:`CDCHKOFF` – Turn off carrier checking
    * :PPL:`CDCHKON` – Turn on carrier checking
    * :PPL:`KBDCHKOFF` – Turn off keyboard checking

KBDFILE (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDFILE(STRING file)`

  Stuff the contents of a text file into the keyboard buffer.

  **Parameters**
    * :PPL:`file` – Filename whose contents to stuff into keyboard buffer

  **Remarks**
    Feeds file contents to PCBoard as if typed by user. Useful for command sequences 
    exceeding 256 characters (KBDSTUFF limit).

  **Example**

    .. code-block:: PPL

       INTEGER retcode
       SHOWOFF
       OPENCAP "NEWFILES.LST",retcode
       KBDSTUFF CHR(13)
       DIR "N;S;A;NS"
       CLOSECAP
       SHOWON
       SHELL TRUE,retcode,"PKZIP","-mex NEWFILES NEWFILES.LST"
       KBDFILE "FLAGFILE.CMD"

  **See Also**
    * :PPL:`KBDSTUFF` – Stuff string to keyboard

KBDSTUFF (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDSTUFF(STRING str)`

  Stuff a string into the keyboard buffer.

  **Parameters**
    * :PPL:`str` – String to stuff into keyboard buffer (max 256 chars)

  **Remarks**
    Feeds keystrokes to PCBoard as if typed by user. Useful for replacing commands or 
    chaining built-in operations. Cannot access CMD.LST defined commands. Use KBDFILE 
    for sequences over 256 characters.

  **Example**

    .. code-block:: PPL

       INTEGER retcode
       SHOWOFF
       OPENCAP "NEWFILES.LST",retcode
       KBDSTUFF CHR(13)
       DIR "N;S;A;NS"
       CLOSECAP
       SHOWON
       KBDSTUFF "FLAG NEWFILES.ZIP"

  **See Also**
    * :PPL:`KBDFILE` – Stuff file contents

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

LOG (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT LOG(STRING msg, BOOLEAN left)`

  Log a message to the caller's log.

  **Parameters**
    * :PPL:`msg` – Message to write to log
    * :PPL:`left` – TRUE for left-justified, FALSE to indent 6 spaces

  **Remarks**
    Keeps SysOp informed of user actions and helps track PPE debugging information.

  **Example**

    .. code-block:: PPL

       BOOLEAN flag
       PRINT "Type QUIT to exit..."
       WAITFOR "QUIT",flag,60
       IF (!flag) LOG "User did not type QUIT",FALSE
       LOG "***EXITING PPE***",TRUE

  **See Also**
    * :PPL:`DBGLEVEL` – Set debug level
    * :PPL:`DBGLEVEL()` – Get debug level

MESSAGE (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT MESSAGE(INTEGER conf, STRING to, STRING from, STRING sub, STRING sec, DATE pack, BOOLEAN rr, BOOLEAN echo, STRING file)`

  Enter a message under PPL control.

  **Parameters**
    * :PPL:`conf` – Conference number for message
    * :PPL:`to` – Recipient name
    * :PPL:`from` – Sender name
    * :PPL:`sub` – Message subject
    * :PPL:`sec` – Security ("N"=none, "R"=receiver only)
    * :PPL:`pack` – Packout date (0 for none)
    * :PPL:`rr` – Return receipt flag
    * :PPL:`echo` – Echo message flag
    * :PPL:`file` – Path to text file for message body

  **Remarks**
    Allows leaving messages from any name to any user. Useful for notifications that should 
    be downloaded in QWK packets or might be missed as on-screen messages.

  **Example**

    .. code-block:: PPL

       IF (CURSEC() < 20) THEN
           MESSAGE 0,U_NAME(),"SYSOP","REGISTER","R",DATE(),TRUE,FALSE,"REG.TXT"
       ENDIF

  **See Also**
    * :PPL:`CURCONF()` – Current conference
    * :PPL:`U_NAME()` – User name

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

MORE (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT MORE`

  Pause display and ask user how to continue.

  **Remarks**
    Prompts user to continue (Y), abort (N), or continue non-stop (NS). Displays prompt 
    196 from PCBTEXT. Language-specific responses supported.

  **Example**

    .. code-block:: PPL

       PRINTLN "Your account has expired!"
       PRINTLN "You are about to be logged off"
       MORE
       PRINTLN "Call me voice to renew your subscription"

  **See Also**
    * :PPL:`ABORT()` – Check abort status
    * :PPL:`DISPTEXT` – Display PCBTEXT prompt
    * :PPL:`WAIT` – Wait for keypress

MOVEMSG (3.20)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT MOVEMSG(INTEGER fromConf, INTEGER msgNum, INTEGER toConf)`

  Moves a message between conferences (permissions & existence required).

MPRINT (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT MPRINT(ANY exp [, ANY exp...])`

  Print to modem only without newline.

  **Parameters**
    * :PPL:`exp` – Expression(s) to print (at least one required)

  **Remarks**
    Sends output only to modem, not local display. Does not interpret @ codes. ANSI 
    interpreted if remote caller has ANSI support. At least one expression required.

  **Example**

    .. code-block:: PPL

       MPRINT "The PPE file is "
       MPRINT PPENAME(),"."

  **See Also**
    * :PPL:`MPRINTLN` – Print to modem with newline
    * :PPL:`PRINT` – Print to screen
    * :PPL:`SPRINT` – Print to local only

MPRINTLN (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT MPRINTLN([ANY exp...])`

  Print to modem only with newline.

  **Parameters**
    * :PPL:`exp` – Expression(s) to print (optional)

  **Remarks**
    Sends output only to modem with newline appended. Does not interpret @ codes. 
    Can be called without arguments to print blank line.

  **Example**

    .. code-block:: PPL

       MPRINTLN "The path is ",PPEPATH(),"."
       MPRINTLN

  **See Also**
    * :PPL:`MPRINT` – Print to modem without newline
    * :PPL:`PRINTLN` – Print to screen with newline
    * :PPL:`SPRINTLN` – Print to local with newline

NEWLINE (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT NEWLINE`

  Move cursor to beginning of next line.

  **Remarks**
    Moves to next line regardless of current cursor position, scrolling if necessary. 
    Unlike FRESHLINE which only moves if not at column 1.

  **Example**

    .. code-block:: PPL

       INTEGER i, end
       LET end = RANDOM(20)
       FOR i = 1 TO end
           PRINT RIGHT(RANDOM(10000),8)
       NEXT
       FRESHLINE
       NEWLINE
       PRINTLN "Now we continue with a blank line between"

  **See Also**
    * :PPL:`FRESHLINE` – Ensure fresh line
    * :PPL:`NEWLINES` – Multiple newlines

NEWLINES (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT NEWLINES(INTEGER count)`

  Execute multiple NEWLINE statements.

  **Parameters**
    * :PPL:`count` – Number of newlines to execute

  **Remarks**
    Convenient for executing multiple or variable NEWLINE statements for screen formatting. 
    Automatically executes specified number of NEWLINEs without loops or multiple statements.

  **Example**

    .. code-block:: PPL

       INTEGER i, end
       LET end = RANDOM(20)
       FOR i = 1 TO end
           PRINT RIGHT(RANDOM(10000),8)
       NEXT
       FRESHLINE
       NEWLINES 5
       PRINTLN "Now we continue with 5 blank lines between"

  **See Also**
    * :PPL:`FRESHLINE` – Ensure fresh line
    * :PPL:`NEWLINE` – Single newline

NEWPWD (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT NEWPWD(STRING pwd, VAR BOOLEAN var)`

  Change user's password with PSA support.

  **Parameters**
    * :PPL:`pwd` – New password
    * :PPL:`var` – Returns TRUE if changed, FALSE if failed

  **Remarks**
    Changes password with full PSA (Password Security Application) support. Validates 
    password, checks history, updates expiration dates, and increments change counter. 
    Sets var to FALSE if password fails validity tests.

  **Example**

    .. code-block:: PPL

       BOOLEAN changed
       STRING pwd
       INPUTSTR "Enter a new password",pwd,@X0E,12,MASK_PWD(),ECHODOTS
       NEWLINE
       NEWPWD pwd,changed
       IF (!changed) PRINTLN "Password not changed"

  **See Also**
    * :PPL:`MASK_PWD()` – Password character mask
    * :PPL:`U_PWD` – User password variable
    * :PPL:`U_PWDEXP` – Password expiration

OPENCAP (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT OPENCAP(STRING file, VAR BOOLEAN ocFlag)`

  Open screen capture file.

  **Parameters**
    * :PPL:`file` – Capture filename
    * :PPL:`ocFlag` – Returns TRUE if opened successfully

  **Remarks**
    Opens a file to capture screen output. Use with SHOWON/SHOWOFF to control display 
    while capturing. Close with CLOSECAP when done.

  **Example**

    .. code-block:: PPL

       BOOLEAN ss, ocFlag
       LET ss = SHOWSTAT()
       SHOWOFF
       OPENCAP "CAP"+STRING(PCBNODE()),ocFlag
       IF (ocFlag) THEN
           DIR "U;NS"
           CLOSECAP
           KBDSTUFF "FLAG CAP"+STRING(PCBNODE())+CHR(13)
       ENDIF
       IF (ss) THEN SHOWON ELSE SHOWOFF

  **See Also**
    * :PPL:`CLOSECAP` – Close capture file
    * :PPL:`SHOWOFF` – Hide display
    * :PPL:`SHOWON` – Show display
    * :PPL:`SHOWSTAT()` – Check display status

OPTEXT (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT OPTEXT(STRING str)`

  Set text for @OPTEXT@ macro.

  **Parameters**
    * :PPL:`str` – Text to use for @OPTEXT@

  **Remarks**
    Sets the text used by @OPTEXT@ macro in prompts and display files. Text must be used 
    immediately after setting (in print statement or display file).

  **Example**

    .. code-block:: PPL

       OPTEXT STRING(DATE())+" & "+STRING(TIME())
       PRINTLN "The date and time are @OPTEXT@"
       DISPFILE "FILE",GRAPH+SEC+LANG

  **See Also**
    * :PPL:`DISPFILE` – Display file
    * :PPL:`DISPSTR` – Display string
    * :PPL:`PRINT` – Print statement

PAGEOFF (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT PAGEOFF`

  Turn off SysOp paged indicator.

  **Remarks**
    Turns off the paged indicator. Used with PAGEON, CHAT, and PAGESTAT() to implement 
    custom operator page functionality.

  **Example**

    .. code-block:: PPL

       PAGEON
       FOR i = 1 TO 10
           PRINT "@BEEP@"
           DELAY 18
           IF (INKEY() = " ") THEN
               PAGEOFF
               SHELL TRUE,i,"SUPERCHT",""
               GOTO exit
           ENDIF
       NEXT
       :exit

  **See Also**
    * :PPL:`CHAT` – Enter chat mode
    * :PPL:`PAGEON` – Turn on paging
    * :PPL:`PAGESTAT()` – Check page status

PAGEON (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT PAGEON`

  Turn on SysOp paged indicator and update statistics.

  **Remarks**
    Turns on paged indicator and updates caller's statistics PSA if installed. Used with 
    PAGEOFF, CHAT, and PAGESTAT() for custom page functionality.

  **Example**

    .. code-block:: PPL

       PAGEON
       FOR i = 1 TO 10
           PRINT "@BEEP@"
           DELAY 18
           IF (INKEY() = " ") THEN
               CHAT
               GOTO exit
           ENDIF
       NEXT
       :exit

  **See Also**
    * :PPL:`CHAT` – Enter chat mode
    * :PPL:`PAGEOFF` – Turn off paging
    * :PPL:`PAGESTAT()` – Check page status

POKEB (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT POKEB(INTEGER addr, INTEGER value)`

  Write a byte to memory address.

  **Parameters**
    * :PPL:`addr` – Memory address
    * :PPL:`value` – Byte value to write (0-255)

  **Remarks**
    Writes a byte value directly to memory. Complements PEEKB() function for low-level 
    memory access.

  **Example**

    .. code-block:: PPL

       BOOLEAN flag
       INTEGER addr
       VARADDR flag,addr
       POKEB addr,TRUE  ; Set flag to TRUE the hard way

  **See Also**
    * :PPL:`PEEKB()` – Read byte from memory
    * :PPL:`POKEDW` – Write double word
    * :PPL:`POKEW` – Write word
    * :PPL:`VARADDR` – Get variable address

POKEDW (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT POKEDW(INTEGER addr, INTEGER value)`

  Write a double word to memory address.

  **Parameters**
    * :PPL:`addr` – Memory address
    * :PPL:`value` – Double word value (-2,147,483,648 to +2,147,483,647)

  **Remarks**
    Writes a 32-bit value directly to memory. Complements PEEKDW() function.

  **Example**

    .. code-block:: PPL

       MONEY amt
       INTEGER addr
       VARADDR amt,addr
       POKEDW addr,123456  ; Set amt to $1234.56

  **See Also**
    * :PPL:`PEEKDW()` – Read double word
    * :PPL:`POKEB` – Write byte
    * :PPL:`POKEW` – Write word

POKEW (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT POKEW(INTEGER addr, INTEGER value)`

  Write a word to memory address.

  **Parameters**
    * :PPL:`addr` – Memory address
    * :PPL:`value` – Word value (0-65,535)

  **Remarks**
    Writes a 16-bit value directly to memory. Complements PEEKW() function.

  **Example**

    .. code-block:: PPL

       DATE dob
       INTEGER addr
       VARADDR dob,addr
       POKEW addr,MKDATE(1967,10,31)  ; Set date of birth

  **See Also**
    * :PPL:`PEEKW()` – Read word from memory
    * :PPL:`POKEB` – Write byte
    * :PPL:`POKEDW` – Write double word

POP (1.00)
~~~~~~~~~~
  :PPL:`STATEMENT POP(VAR var [, VAR var...])`

  Pop values from stack into variables.

  **Parameters**
    * :PPL:`var` – Variable(s) to receive popped values

  **Remarks**
    Retrieves values previously pushed with PUSH statement. Used for parameter passing, 
    creating 'local' variables, or reversing argument order. Values popped in LIFO order.

  **Example**

    .. code-block:: PPL

       INTEGER i, tc
       STRING s
       LET tc = TOKCOUNT()
       WHILE (TOKCOUNT() > 0) PUSH GETTOKEN()  ; Push in order
       FOR i = 1 TO tc
           POP s  ; Pop in reverse
           PRINTLN s
       NEXT

  **See Also**
    * :PPL:`PUSH` – Push values to stack

PRINT (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT PRINT(ANY exp [, ANY exp...])`

  Print to screen without newline.

  **Parameters**
    * :PPL:`exp` – Expression(s) to print (at least one required)

  **Remarks**
    Evaluates and displays expressions without newline. Processes @ codes. At least one 
    expression required.

  **Example**

    .. code-block:: PPL

       PRINT "The PPE file is "
       PRINT PPENAME(),"."
       PRINT "@X1FThis is bright white on blue..."

  **See Also**
    * :PPL:`PRINTLN` – Print with newline
    * :PPL:`MPRINT` – Print to modem only
    * :PPL:`SPRINT` – Print to local only

PRINTLN (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT PRINTLN([ANY exp...])`

  Print to screen with newline.

  **Parameters**
    * :PPL:`exp` – Expression(s) to print (optional)

  **Remarks**
    Evaluates and displays expressions with newline appended. Processes @ codes. Can be 
    called without arguments for blank line.

  **Example**

    .. code-block:: PPL

       PRINTLN "The path is ",PPEPATH(),"."
       PRINTLN
       PRINTLN "@X0EHow do you like it @FIRST@?"

  **See Also**
    * :PPL:`PRINT` – Print without newline
    * :PPL:`MPRINTLN` – Print to modem with newline
    * :PPL:`SPRINTLN` – Print to local with newline

PROMPTSTR (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT PROMPTSTR(INTEGER prompt, VAR var, INTEGER len, STRING valid, INTEGER flags)`

  Prompt using PCBTEXT entry with validation.

  **Parameters**
    * :PPL:`prompt` – PCBTEXT prompt number
    * :PPL:`var` – Variable for input
    * :PPL:`len` – Maximum input length
    * :PPL:`valid` – Valid characters
    * :PPL:`flags` – Input behavior flags

  **Remarks**
    Uses PCBTEXT prompt with color. Validates input against character mask. Flag values: 
    AUTO, DEFS, ECHODOTS, ERASELINE, FIELDLEN, GUIDE, HIGHASCII, LFAFTER, LFBEFORE, 
    NEWLINE, NOCLEAR, STACKED, UPCASE, WORDWRAP, YESNO.

  **Example**

    .. code-block:: PPL

       STRING s
       PROMPTSTR 706,s,63,MASK_ASCII(),NEWLINE+FIELDLEN+GUIDE

  **See Also**
    * :PPL:`INPUT` – Basic input
    * :PPL:`INPUTSTR` – Advanced input
    * :PPL:`INPUTTEXT` – Text input

PUSH (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT PUSH(ANY exp [, ANY exp...])`

  Push values onto stack.

  **Parameters**
    * :PPL:`exp` – Expression(s) to push (at least one required)

  **Remarks**
    Evaluates expressions and pushes results onto stack for temporary storage. Retrieved 
    with POP statement. Used for parameter passing, local variables, or reversing arguments.

  **Example**

    .. code-block:: PPL

       INTEGER v
       PRINT "A cube with dimensions 2x3x4"
       PUSH 2,3,4  ; Pass parameters
       GOSUB vol
       POP v  ; Get result
       PRINTLN " has volume ",v
       END
       
       :vol
       INTEGER w,h,d
       POP d,h,w  ; Get parameters
       PUSH w*h*d  ; Return result
       RETURN

  **See Also**
    * :PPL:`POP` – Pop values from stack

// ...existing code...

PUTUSER (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT PUTUSER`

  Copy values from predeclared user variables to user record.

  **Remarks**
    Saves changes made to U_XXX variables back to the user record. Variables must first be 
    populated with GETUSER. Changes are not permanent until PUTUSER is called.

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

QUEST (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT QUEST(INTEGER scrnum)`

  Allow the user to answer a script questionnaire.

  **Parameters**
    * :PPL:`scrnum` – Script number to run (1 to max available)

  **Remarks**
    Presents the specified script questionnaire from SCR.LST for the current conference. 
    If script number is invalid, nothing is displayed.

  **Example**

    .. code-block:: PPL

       INTEGER num
       INPUT "Script to answer",num
       QUEST num

  **See Also**
    * :PPL:`BLT` – Display bulletin
    * :PPL:`DIR` – File directory
    * :PPL:`JOIN` – Join conference

QWKLIMITS (3.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT QWKLIMITS(INTEGER field, INTEGER limit)`

  Modify QWK packet limits for the current user.

  **Parameters**
    * :PPL:`field` – Field to modify (0-3, use constants below)
    * :PPL:`limit` – New limit value

  **Field Constants**
    =================  =====  ================================================
    Constant           Value  Description
    =================  =====  ================================================
    MAXMSGS            0      Maximum messages per QWK packet
    CMAXMSGS           1      Maximum messages per conference in packet
    ATTACH_LIM_U       2      Personal attachment size limit (bytes)
    ATTACH_LIM_P       3      Public attachment size limit (bytes)
    =================  =====  ================================================

  **Remarks**
    Modifies QWK packet download limits for the current user. Changes are only saved 
    after calling PUTUSER. For MAXMSGS and CMAXMSGS, if you specify a value higher 
    than configured in PCBSetup, the PCBSetup values will be used instead. The 
    attachment limits control the maximum total size of file attachments the user 
    can include in their QWK packets.

  **Example**

    .. code-block:: PPL

       GETUSER
       
       ; Set maximum messages per packet to 500
       QWKLIMITS MAXMSGS, 500
       
       ; Limit messages per conference to 100
       QWKLIMITS CMAXMSGS, 100
       
       ; Set attachment limits based on security level
       IF (CURSEC() >= 50) THEN
           QWKLIMITS ATTACH_LIM_U, 1048576  ; 1MB personal attachments
           QWKLIMITS ATTACH_LIM_P, 2097152  ; 2MB public attachments
       ELSE
           QWKLIMITS ATTACH_LIM_U, 102400   ; 100KB personal attachments
           QWKLIMITS ATTACH_LIM_P, 512000   ; 500KB public attachments
       ENDIF
       
       PUTUSER

  **See Also**
    * :PPL:`GETUSER` – Load user record
    * :PPL:`PUTUSER` – Save user record
    * :PPL:`MESSAGE` – Send message

RDUNET (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT RDUNET(INTEGER node)`

  Read information from USERNET file for a specific node.

  **Parameters**
    * :PPL:`node` – Node number to read

  **Remarks**
    Reads USERNET.XXX file entry for specified node. Used for internode communications, 
    preventing simultaneous logins, and by the BROADCAST command. After reading, use 
    UN_XXX() functions to access the data.

  **Example**

    .. code-block:: PPL

       RDUNET PCBNODE()
       WRUNET PCBNODE(),UN_STAT(),UN_NAME(),UN_CITY(),"Running "+PPENAME(),""
       RDUNET 1
       WRUNET 1,UN_STAT(),UN_NAME(),UN_CITY(),UN_OPER(),"Hello there node 1"

  **See Also**
    * :PPL:`BROADCAST` – Send message to nodes
    * :PPL:`UN_...()` – USERNET field functions
    * :PPL:`WRUNET` – Write USERNET record

RDUSYS (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT RDUSYS`

  Read a USERS.SYS file from disk.

  **Remarks**
    Reads USERS.SYS file back into memory after a DOOR application may have modified it. 
    Should only be used after SHELL statement that was preceded by WRUSYS.

  **Example**

    .. code-block:: PPL

       INTEGER ret
       WRUSYS
       SHELL FALSE,ret,"MYAPP.EXE",""
       RDUSYS

  **See Also**
    * :PPL:`SHELL` – Execute external program
    * :PPL:`WRUSYS` – Write USERS.SYS file

RECORDUSAGE (3.00)
~~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT RECORDUSAGE(INTEGER field, STRING desc1, STRING desc2, DWORD unitcost, INTEGER value)`

  Update user accounting and log transaction details.

  **Parameters**
    * :PPL:`field` – Accounting field (2-16, use DEB_/CRED_ constants)
    * :PPL:`desc1` – Primary description of the charge
    * :PPL:`desc2` – Secondary description or details
    * :PPL:`unitcost` – Cost per unit
    * :PPL:`value` – Number of units

  **Field Constants**
    ================  ===  ============================================
    Constant          Val  Description
    ================  ===  ============================================
    DEB_CALL          2    Debit for this call
    DEB_TIME          3    Debit for time online
    DEB_MSGREAD       4    Debit for reading messages
    DEB_MSGCAP        5    Debit for capturing messages
    DEB_MSGWRITE      6    Debit for writing messages
    DEB_MSGECHOED     7    Debit for echoed messages
    DEB_MSGPRIVATE    8    Debit for private messages
    DEB_DOWNFILE      9    Debit for downloading files
    DEB_DOWNBYTES     10   Debit for downloading bytes
    DEB_CHAT          11   Debit for chat time
    DEB_TPU           12   Debit for TPU
    DEB_SPECIAL       13   Special debit
    CRED_UPFILE       14   Credit for uploading files
    CRED_UPBYTES      15   Credit for uploading bytes
    CRED_SPECIAL      16   Special credit
    ================  ===  ============================================

  **Remarks**
    Updates accounting debit/credit values and writes detailed transaction records to the 
    accounting log file. Unlike the ACCOUNT statement which only updates in-memory values, 
    RECORDUSAGE also creates an audit trail with descriptions, unit costs, and quantities 
    for billing and reporting purposes. The total charge (unitcost * value) is added to 
    the specified field.

  **Example**

    .. code-block:: PPL

       ; Record chat charges with details
       RECORDUSAGE DEB_CHAT, "Debit for chat", "Using PPE", 10, 10
       ; This charges 100 credits (10 units * 10 per unit)
       
       ; Record file download with details
       INTEGER fileSize
       fileSize = FILEINF("MYFILE.ZIP", 2) / 1024  ; Size in KB
       RECORDUSAGE DEB_DOWNBYTES, "Downloaded MYFILE.ZIP", "From Area 5", 1, fileSize
       
       ; Give upload credit
       RECORDUSAGE CRED_UPFILE, "Upload credit", "NEWFILE.ZIP", 25, 1

  **See Also**
    * :PPL:`ACCOUNT` statement – Update accounting without logging
    * :PPL:`ACCOUNT()` function – Retrieve accounting values
    * :PPL:`PCBACCOUNT()` – Get charge rates
    * :PPL:`PCBACCSTAT()` – Check accounting status

RENAME (3.20)
~~~~~~~~~~~~~
  :PPL:`STATEMENT RENAME(STRING old, STRING new)`

  Rename or move a file.

  **Parameters**
    * :PPL:`old` – Old path and/or filename
    * :PPL:`new` – New path and/or filename

  **Remarks**
    Renames or moves a file on the same drive. Unlike DOS RENAME, doesn't accept wildcards. 
    Can move files between directories on same drive but not between drives.

  **Example**

    .. code-block:: PPL

       ; Swap PCBOARD.DAT & NXT files
       RENAME "PCBOARD.DAT","PCBOARD.TMP"
       RENAME "PCBOARD.NXT","PCBOARD.DAT"
       RENAME "PCBOARD.TMP","PCBOARD.NXT"
       
       ; Move file to backup directory
       RENAME "PPE.LOG","LOGBAK\"+I2S(DATE()*86400+TIME(),36)

  **See Also**
    * :PPL:`DELETE` – Delete file
    * :PPL:`EXIST()` – Check file existence
    * :PPL:`FILEINF()` – Get file information

RESETDISP (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT RESETDISP`

  Reset display to allow more information after an abort.

  **Remarks**
    Resets display after user aborts with MORE? prompt or ^K/^X. No further information 
    displays until RESETDISP is called. Use ABORT() to check if reset is needed.

  **Example**

    .. code-block:: PPL

       INTEGER i
       STARTDISP FCL
       ; While user has not aborted, continue
       WHILE (!ABORT()) DO
           PRINTLN "I is equal to ",i
           INC i
       ENDWHILE
       RESETDISP

  **See Also**
    * :PPL:`ABORT()` – Check abort status
    * :PPL:`STARTDISP` – Start display mode

RESTSCRN (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT RESTSCRN`

  Restore screen from previously saved buffer.

  **Remarks**
    Restores screen saved with SAVESCRN. Works regardless of ANSI availability. Screen 
    is saved up to cursor position and restored using standard teletype scrolling. Memory 
    allocated by SAVESCRN is freed.

  **Example**

    .. code-block:: PPL

       SAVESCRN
       CLS
       PRINTLN "We interrupt your regular BBS session"
       PRINTLN "with this important message:"
       NEWLINE
       PRINTLN "A subscription to this system only costs $5!"
       PRINTLN "Subscribe today!"
       NEWLINES 2
       WAIT
       RESTSCRN

  **See Also**
    * :PPL:`SAVESCRN` – Save screen to buffer

RETURN (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT RETURN`

  Transfer control back to previously saved address.

  **Remarks**
    Returns execution to the line following the most recent GOSUB. Used at end of 
    subroutines to resume main program flow.

  **Example**

    .. code-block:: PPL

       STRING Question, Answer
       LET Question = "What is your street address..."
       GOSUB ask
       LET Question = "What is your city, state and zip..."
       GOSUB ask
       END
       
       :ask  ; Subroutine
       LET Answer = ""
       PRINTLN "@X0E",Question
       INPUT "",Answer
       NEWLINES 2
       FPUTLN 0,"Q: ",STRIPATX(Question)
       FPUTLN 0,"A: ",Answer
       RETURN

  **See Also**
    * :PPL:`END` – End program
    * :PPL:`GOSUB` – Call subroutine
    * :PPL:`GOTO` – Jump to label

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

SAVESCRN (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SAVESCRN`

  Save screen to buffer for later restoration.

  **Remarks**
    Saves current screen up to cursor position. Allocates memory for buffer. Must be 
    followed by RESTSCRN to free memory. Works regardless of ANSI availability.

  **Example**

    .. code-block:: PPL

       SAVESCRN
       CLS
       PRINTLN "We interrupt your regular BBS session"
       PRINTLN "with this important message:"
       NEWLINE
       PRINTLN "A subscription costs only $5!"
       PRINTLN "Subscribe today!"
       NEWLINES 2
       WAIT
       RESTSCRN

  **See Also**
    * :PPL:`RESTSCRN` – Restore screen

SENDMODEM (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SENDMODEM(STRING str)`

  Send a string to the modem.

  **Parameters**
    * :PPL:`str` – String to send to modem

  **Remarks**
    Sends commands or data to modem. Primary use is sending commands when no one is 
    online (e.g., callback PPL). Does not automatically append carriage return, allowing 
    multi-stage command building.

  **Example**

    .. code-block:: PPL

       BOOLEAN flag
       CDCHKOFF
       KBDCHKOFF
       DTROFF
       DELAY 18
       DTRON
       SENDMODEM "ATDT"
       SENDMODEM "5551212"
       SENDMODEM CHR(13)
       WAITFOR "CONNECT",flag,60
       IF (!flag) LOG "No CONNECT after 60 seconds",FALSE
       KBDCHKON
       CDCHKON

  **See Also**
    * :PPL:`WAITFOR` – Wait for modem response

SETBANKBAL (3.20)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SETBANKBAL(INTEGER userRec, MONEY amount)`

  Adjusts stored “bank” balance (economy/game feature – semantics engine-defined).

SETENV (3.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT SETENV(STRING envvar)`

  Set a DOS environment variable.

  **Parameters**
    * :PPL:`envvar` – Environment variable assignment in format "NAME=VALUE"

  **Remarks**
    Sets a DOS environment variable for inter-PPE communication. The variable assignment must 
    be in the format "NAME=VALUE". Environment variables set within PPL are NOT available to 
    DOOR applications and will be cleared when PCBoard recycles through DOS. PPL programmers 
    should clear environment variables when no longer needed to avoid memory waste. This 
    provides a simple method for different PPE files to share data during a session.

  **Example**

    .. code-block:: PPL

       STRING s
       LET s = "STAN=Stan"
       SETENV s
       
       ; Later in this or another PPE...
       IF (GETENV("STAN") = "Stan") THEN
           PRINTLN "Environment variable STAN = Stan"
       ENDIF
       
       ; Clear when done
       SETENV "STAN="

  **See Also**
    * :PPL:`GETENV()` – Get environment variable value
    * :PPL:`CALL` – Execute another PPE
    * :PPL:`SHELL` – Execute external program

SETLMR (3.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT SETLMR(INTEGER conf, INTEGER msg)`

  Set the Last Message Read pointer for a conference.

  **Parameters**
    * :PPL:`conf` – Conference number to set LMR for
    * :PPL:`msg` – Message number to set as last read

  **Remarks**
    Sets the user's Last Message Read (LMR) pointer for the specified conference. Useful for 
    new user setup to prevent them from seeing or replying to very old messages. If the 
    conference number exceeds the highest available conference, it defaults to the highest 
    conference. If the message number exceeds the highest message in that conference, it 
    defaults to the highest message number.

  **Example**

    .. code-block:: PPL

       INTEGER conf, msg
       IF (newuser = TRUE) THEN           ; If new user
           WHILE (conf <= HICONFNUM()) DO ; Set all LMRs to
               JOIN conf                   ; HI_MSG - 10
               SETLMR conf, HIMSGNUM()-10
               INC conf
           ENDWHILE
       ENDIF

  **See Also**
    * :PPL:`HICONFNUM()` – Get highest conference number
    * :PPL:`HIMSGNUM()` – Get highest message number
    * :PPL:`LOWMSGNUM()` – Get lowest message number  
    * :PPL:`NUMACTMSG()` – Get number of active messages
    * :PPL:`JOIN` – Join conference

SHELL (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT SHELL(BOOLEAN viacc, VAR INTEGER retcode, STRING prog, STRING cmds)`

  Execute an external program or batch file.

  **Parameters**
    * :PPL:`viacc` – TRUE to shell via COMMAND.COM, FALSE for direct execution
    * :PPL:`retcode` – Variable to store return code
    * :PPL:`prog` – Program filename to execute
    * :PPL:`cmds` – Command line arguments

  **Remarks**
    Runs COM, EXE, or BAT files. If viacc is TRUE, PATH is searched and extensions are 
    assumed. If FALSE, full path and extension required. Return code only meaningful 
    when viacc is FALSE.

  **Example**

    .. code-block:: PPL

       INTEGER rc
       SHELL TRUE,rc,"DOOR",""
       
       ; Direct execution with full path
       INTEGER rc
       STRING p,c
       LET p = "DOORWAY.EXE"
       LET c = "com2 /v:d^O /m:600 /g:on /o: /k:v0 /x: /c:dos"
       SHELL FALSE,rc,p,c

  **See Also**
    * :PPL:`CALL` – Execute another PPE
    * :PPL:`RDUSYS` – Read USERS.SYS
    * :PPL:`WRUSYS` – Write USERS.SYS

SHORTDESC (3.20)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SHORTDESC(STRING text)`

  **Description**
    Sets a short descriptive string for the PPE (shown in sysop listings / logs).

SHOWOFF (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT SHOWOFF`

  Turn off display output.

  **Remarks**
    Disables writing to local and remote displays. Used with OPENCAP/CLOSECAP to capture 
    output without displaying it. Useful for automating features and allowing download of 
    capture files.

  **Example**

    .. code-block:: PPL

       BOOLEAN ss, ocFlag
       LET ss = SHOWSTAT()
       SHOWOFF
       OPENCAP "CAP"+STRING(PCBNODE()),ocFlag
       IF (ocFlag) THEN
           DIR "U;NS"
           CLOSECAP
           KBDSTUFF "FLAG CAP"+STRING(PCBNODE())+CHR(13)
       ENDIF
       IF (ss) THEN SHOWON ELSE SHOWOFF

  **See Also**
    * :PPL:`CLOSECAP` – Close capture file
    * :PPL:`OPENCAP` – Open capture file
    * :PPL:`SHOWON` – Turn on display
    * :PPL:`SHOWSTAT()` – Get display status

SHOWON (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT SHOWON`

  Turn on display output.

  **Remarks**
    Re-enables writing to local and remote displays after SHOWOFF. Used with capture 
    functions to control when output is visible.

  **Example**

    .. code-block:: PPL

       BOOLEAN ss, ocFlag
       LET ss = SHOWSTAT()
       SHOWOFF
       OPENCAP "CAP"+STRING(PCBNODE()),ocFlag
       IF (ocFlag) THEN
           DIR "U;NS"
           CLOSECAP
           KBDSTUFF "FLAG CAP"+STRING(PCBNODE())+CHR(13)
       ENDIF
       IF (ss) THEN SHOWON ELSE SHOWOFF

  **See Also**
    * :PPL:`CLOSECAP` – Close capture file
    * :PPL:`OPENCAP` – Open capture file
    * :PPL:`SHOWOFF` – Turn off display
    * :PPL:`SHOWSTAT()` – Get display status

SOUND (1.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT SOUND(INTEGER freq)`

  Turn on speaker at specified frequency.

  **Parameters**
    * :PPL:`freq` – Frequency in hertz (0 to turn off)

  **Remarks**
    Generates tones on local PC speaker only. No effect on remote computer. Works only with 
    built-in speaker, not sound cards. Pass frequency in hertz to generate tone, or 0 to turn off.

  **Example**

    .. code-block:: PPL

       PAGEON
       FOR i = 1 TO 10
           MPRINT CHR(7)
           SOUND 440  ; A note
           DELAY 9
           SOUND 0    ; Turn off
           DELAY 9
           IF (INKEY() = " ") THEN
               CHAT
               GOTO exit
           ENDIF
       NEXT
       :exit

  **See Also**
    * :PPL:`DELAY` – Pause execution
    * :PPL:`SOUNDDELAY` – Sound with duration

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

SPRINT (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT SPRINT(ANY exp [, ANY exp...])`

  Print to local screen only without newline.

  **Parameters**
    * :PPL:`exp` – Expression(s) to print (at least one required)

  **Remarks**
    Sends output only to local BBS display for SysOp viewing. Does not interpret @ codes 
    but complete ANSI sequences are interpreted. At least one expression required.

  **Example**

    .. code-block:: PPL

       SPRINT "The PPE file is "
       SPRINT PPENAME(),"."

  **See Also**
    * :PPL:`SPRINTLN` – Print local with newline
    * :PPL:`PRINT` – Print to both screens
    * :PPL:`MPRINT` – Print to modem only

SPRINTLN (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SPRINTLN([ANY exp...])`

  Print to local screen only with newline.

  **Parameters**
    * :PPL:`exp` – Expression(s) to print (optional)

  **Remarks**
    Sends output only to local BBS display with newline appended. Does not interpret @ codes 
    but complete ANSI sequences are interpreted. Can be called without arguments for blank line.

  **Example**

    .. code-block:: PPL

       SPRINTLN "The path is ",PPEPATH(),"."
       SPRINTLN "The date is ",DATE()," and time is ",TIME(),"."
       SPRINTLN

  **See Also**
    * :PPL:`SPRINT` – Print local without newline
    * :PPL:`PRINTLN` – Print to both with newline
    * :PPL:`MPRINTLN` – Print to modem with newline

STACKABORT (3.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT STACKABORT(BOOLEAN abort)`

  Control whether to abort execution on stack errors.

  **Parameters**
    * :PPL:`abort` – TRUE to abort on stack error (default), FALSE to continue

  **Remarks**
    Allows the programmer to control PPE behavior when a stack error occurs. When set 
    to TRUE (default), the PPE will terminate execution after a stack error. When set 
    to FALSE, the PPE will attempt to continue running. CAUTION: If execution continues 
    after a stack error, program behavior becomes unpredictable. PPL will prevent system 
    memory corruption but cannot guarantee correct program operation. Use STACKERR() to 
    check for errors and STACKLEFT() to prevent them.

  **Example**

    .. code-block:: PPL

       STACKABORT FALSE  ; Try to continue on stack errors
       
       ; Risky recursive operation
       PROCEDURE riskyRecursion(INTEGER depth)
           IF (STACKERR()) THEN
               PRINTLN "Stack error detected, stopping recursion"
               RETURN
           ENDIF
           
           IF (STACKLEFT() < STK_LIMIT) THEN
               PRINTLN "Stack space low, stopping at depth ", depth
               RETURN
           ENDIF
           
           riskyRecursion(depth + 1)
       ENDPROC

  **See Also**
    * :PPL:`STACKERR()` – Check for stack errors
    * :PPL:`STACKLEFT()` – Check remaining stack space
    * :PPL:`STK_LIMIT` – Stack limit constant

STARTDISP (1.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT STARTDISP(INTEGER mode)`

  Start display routines in specified mode.

  **Parameters**
    * :PPL:`mode` – Display mode (FNS=force non-stop, FCL=force count lines, NC=no change)

  **Remarks**
    Controls PCBoard's display mode. FNS displays without pausing. FCL counts lines and 
    pauses every screenful. NC reinitializes counters without changing mode.

  **Example**

    .. code-block:: PPL

       STARTDISP FCL
       FOR i = 1 TO 100
           PRINTLN "Line ",i
       NEXT
       
       STARTDISP FNS
       FOR i = 1 TO 100
           PRINTLN "Line ",i
       NEXT
       
       STARTDISP NC
       FOR i = 1 TO 100
           PRINTLN "Line ",i
       NEXT

  **See Also**
    * :PPL:`ABORT()` – Check abort status
    * :PPL:`RESETDISP` – Reset after abort

STOP (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT STOP`

  Abort PPE execution without saving script output.

  **Remarks**
    Abnormally terminates PPE execution. Unlike END, does not save channel 0 output to 
    script answer file. Use when you need to abort without saving partial results.

  **Example**

    .. code-block:: PPL

       STRING Question, Answer
       LET Question = "What is your street address..."
       GOSUB ask
       INPUTYN "Save address",Answer,@X0E
       IF (Answer = NOCHAR()) STOP
       END
       
       :ask
       LET Answer = ""
       PRINTLN "@X0E",Question
       INPUT "",Answer
       NEWLINES 2
       FPUTLN 0,"Q: ",STRIPATX(Question)
       FPUTLN 0,"A: ",Answer
       RETURN

  **See Also**
    * :PPL:`END` – Normal termination
    * :PPL:`RETURN` – Return from subroutine

TOKENIZE (1.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT TOKENIZE(STRING sexp)`

  Split string into tokens separated by semicolons or spaces.

  **Parameters**
    * :PPL:`sexp` – String expression to tokenize

  **Remarks**
    Breaks command line into individual tokens like PCBoard's command stacking. Tokens 
    are accessed with TOKCOUNT() for count and GETTOKEN statement or function to retrieve. 
    Allows processing of multiple stacked commands.

  **Example**

    .. code-block:: PPL

       STRING cmdline
       INPUT "Command",cmdline
       TOKENIZE cmdline
       PRINTLN "You entered ",TOKCOUNT()," tokens"
       WHILE (TOKCOUNT() > 0) 
           PRINTLN "Token: ",CHR(34),GETTOKEN(),CHR(34)
       ENDWHILE

  **See Also**
    * :PPL:`GETTOKEN` – Retrieve next token
    * :PPL:`GETTOKEN()` – Function version
    * :PPL:`TOKCOUNT()` – Count remaining tokens
    * :PPL:`TOKENSTR()` – Rebuild token string

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


VARADDR (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT VARADDR(VAR src, VAR INTEGER dest)`

  Get the complete memory address of a variable.

  **Parameters**
    * :PPL:`src` – Variable to get the address of
    * :PPL:`dest` – Variable to store the address

  **Remarks**
    Primarily useful with DOINTR statement for passing memory addresses to interrupts. 
    Gets the complete segment:offset address as a single value.

  **Example**

    .. code-block:: PPL

       ; Create subdirectory - DOS function 39h
       INTEGER addr
       STRING path
       LET path = "C:\$TMPDIR$"
       VARADDR path,addr
       DOINTR 0x21,0x39,0,0,addr*0x10000,0,0,0,addr/0x10000,0
       IF (REGCF() & (REGAX() = 3)) THEN
           PRINTLN "Error: Path not found"
       ELSEIF (REGCF() & (REGAX() = 5)) THEN
           PRINTLN "Error: Access Denied"
       ELSEIF (REGCF()) THEN
           PRINTLN "Error: Unknown Error"
       ELSE
           PRINTLN "Directory successfully created..."
       ENDIF

  **See Also**
    * :PPL:`DOINTR` – Generate interrupt
    * :PPL:`VAROFF` – Get offset address
    * :PPL:`VARSEG` – Get segment address
    * :PPL:`MKADDR()` – Make address from segment:offset

VAROFF (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT VAROFF(VAR src, VAR INTEGER dest)`

  Get the offset address of a variable.

  **Parameters**
    * :PPL:`src` – Variable to get the offset of
    * :PPL:`dest` – Variable to store the offset

  **Remarks**
    Gets the offset portion of a variable's memory address. Used with VARSEG for 
    interrupt programming when separate segment and offset values are needed.

  **Example**

    .. code-block:: PPL

       ; Create subdirectory - DOS function 39h
       INTEGER saddr, oaddr
       STRING path
       LET path = "C:\$TMPDIR$"
       VARSEG path,saddr
       VAROFF path,oaddr
       DOINTR 0x21,0x39,0,0,oaddr,0,0,0,saddr,0
       IF (REGCF() & (REGAX() = 3)) THEN
           PRINTLN "Error: Path not found"
       ELSEIF (REGCF() & (REGAX() = 5)) THEN
           PRINTLN "Error: Access Denied"
       ELSEIF (REGCF()) THEN
           PRINTLN "Error: Unknown Error"
       ELSE
           PRINTLN "Directory successfully created..."
       ENDIF

  **See Also**
    * :PPL:`DOINTR` – Generate interrupt
    * :PPL:`VARADDR` – Get complete address
    * :PPL:`VARSEG` – Get segment address

VARSEG (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT VARSEG(VAR src, VAR INTEGER dest)`

  Get the segment address of a variable.

  **Parameters**
    * :PPL:`src` – Variable to get the segment of
    * :PPL:`dest` – Variable to store the segment

  **Remarks**
    Gets the segment portion of a variable's memory address. Used with VAROFF for 
    interrupt programming when separate segment and offset values are needed.

  **Example**

    .. code-block:: PPL

       ; Create subdirectory - DOS function 39h
       INTEGER saddr, oaddr
       STRING path
       LET path = "C:\$TMPDIR$"
       VARSEG path,saddr
       VAROFF path,oaddr
       DOINTR 0x21,0x39,0,0,oaddr,0,0,0,saddr,0
       IF (REGCF() & (REGAX() = 3)) THEN
           PRINTLN "Error: Path not found"
       ELSEIF (REGCF() & (REGAX() = 5)) THEN
           PRINTLN "Error: Access Denied"
       ELSEIF (REGCF()) THEN
           PRINTLN "Error: Unknown Error"
       ELSE
           PRINTLN "Directory successfully created..."
       ENDIF

  **See Also**
    * :PPL:`DOINTR` – Generate interrupt
    * :PPL:`VARADDR` – Get complete address
    * :PPL:`VAROFF` – Get offset address

WAIT (1.00)
~~~~~~~~~~~
  :PPL:`STATEMENT WAIT`

  Wait for user to press ENTER.

  **Remarks**
    Pauses execution and waits for user to press ENTER. Displays prompt 418 from 
    PCBTEXT file in current language to indicate what's expected.

  **Example**

    .. code-block:: PPL

       PRINTLN "Your account has expired!"
       PRINTLN "You are about to be logged off"
       WAIT

  **See Also**
    * :PPL:`DISPTEXT` – Display PCBTEXT prompt
    * :PPL:`INKEY()` – Get single keypress
    * :PPL:`MORE` – Pause with options
    * :PPL:`PROMPTSTR` – Prompt for input

WAITFOR (1.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT WAITFOR(STRING str, VAR BOOLEAN flag, INTEGER sec)`

  Wait for specific text from modem.

  **Parameters**
    * :PPL:`str` – Text to wait for (case-insensitive)
    * :PPL:`flag` – Returns TRUE if found, FALSE if timeout
    * :PPL:`sec` – Maximum seconds to wait

  **Remarks**
    Waits for specific text from modem (e.g., modem responses, terminal replies). 
    Returns FALSE immediately if local caller. Case-insensitive matching: "connect" 
    matches "CONNECT". Sets flag to FALSE if timeout or no remote caller.

  **Example**

    .. code-block:: PPL

       BOOLEAN flag
       KBDCHKOFF
       CDCHKOFF
       DTROFF
       DELAY 18
       DTRON
       SENDMODEM "ATDT5551212"
       SENDMODEM CHR(13)
       WAITFOR "CONNECT",flag,60
       IF (!flag) SPRINTLN "No connect found in 60 seconds"
       CDCHKON
       KBDCHKON

  **See Also**
    * :PPL:`DELAY` – Pause execution
    * :PPL:`MGETBYTE()` – Get byte from modem
    * :PPL:`SENDMODEM` – Send to modem

WEBREQUEST (4.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT WEBREQUEST(STRING url, STRING filename)`

  Downloads data from a web server and saves it to a file.

  **Parameters**
    * :PPL:`url` – Complete URL to download (including protocol)
    * :PPL:`filename` – Local file path to save the downloaded data

  **Remarks**
    Performs an HTTP GET request and saves the response directly to a file. This statement 
    is ideal for downloading large files, binary data, or any content that exceeds PPL's 
    string limitations. Supports both HTTP and HTTPS protocols with automatic handling of 
    common redirects.
    
    The download is synchronous - the PPE will wait until the download completes or times 
    out before continuing. For large files, consider informing users of the download progress 
    or expected wait time. The function overwrites existing files without warning.
    
    Network operations require appropriate system permissions. The download directory must 
    be writable by the BBS process. Some installations may restrict web requests or limit 
    accessible URLs for security reasons.

  **Example**

    .. code-block:: PPL

       STRING tempFile, updateFile
       BOOLEAN success
       
       ; Download a text file
       tempFile = TEMPPATH() + "news_" + STRING(PCBNODE()) + ".txt"
       WEBREQUEST "https://example.com/bbsnews.txt", tempFile
       IF (EXIST(tempFile)) THEN
           DISPFILE tempFile, DEFS
           DELETE tempFile
       ENDIF
       
       ; Download binary file (ZIP archive)
       updateFile = TEMPPATH() + "update.zip"
       PRINTLN "Downloading update package..."
       WEBREQUEST "https://updates.bbs.com/latest.zip", updateFile
       IF (EXIST(updateFile)) THEN
           INTEGER size
           size = FILEINF(updateFile, 4)
           PRINTLN "Downloaded ", size, " bytes"
           ; Process the downloaded file
           SHELL TRUE, rc, "unzip -o " + updateFile
       ELSE
           PRINTLN "Download failed!"
       ENDIF
       
       ; Fetch and save JSON data
       STRING dataFile
       dataFile = PPEPATH() + "userdata.json"
       WEBREQUEST "https://api.service.com/users/current", dataFile
       IF (EXIST(dataFile) & FILEINF(dataFile, 4) > 0) THEN
           ; Process JSON file
           PRINTLN "User data updated"
       ENDIF
       
       ; Download with error checking
       STRING url, dest
       url = "https://mirror.bbs.com/files/welcome.ans"
       dest = HELPPATH() + "welcome.ans"
       WEBREQUEST url, dest
       IF (!EXIST(dest) | FILEINF(dest, 4) = 0) THEN
           LOG "Failed to download: " + url, FALSE
       ENDIF

  **See Also**
    * :PPL:`WEBREQUEST()` function – Get response as string
    * :PPL:`EXIST()` – Check if download succeeded
    * :PPL:`FILEINF()` – Get file information
    * :PPL:`TEMPPATH()` – Get temporary directory
    * :PPL:`DELETE` – Clean up downloaded files

WRUNET (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT WRUNET(INTEGER node, STRING stat, STRING name, STRING city, STRING oper, STRING br)`

  Write information to USERNET file for a node.

  **Parameters**
    * :PPL:`node` – Node number to update
    * :PPL:`stat` – Node status
    * :PPL:`name` – User name on node
    * :PPL:`city` – User city
    * :PPL:`oper` – Operation text
    * :PPL:`br` – Broadcast message text

  **Remarks**
    Updates USERNET.XXX file entry for specified node. Used for internode communication, 
    updating operation text during PPE execution, or broadcasting messages to other nodes.

  **Example**

    .. code-block:: PPL

       RDUNET PCBNODE()
       WRUNET PCBNODE(),UN_STAT(),UN_NAME(),UN_CITY(),"Running "+PPENAME(),""
       
       ; Send message to node 1
       RDUNET 1
       WRUNET 1,UN_STAT(),UN_NAME(),UN_CITY(),UN_OPER(),"Hello there node 1"

  **See Also**
    * :PPL:`BROADCAST` – Broadcast to nodes
    * :PPL:`RDUNET` – Read USERNET record
    * :PPL:`UN_...()` – USERNET field functions

WRUSYS (1.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT WRUSYS`

  Write USERS.SYS file to disk.

  **Remarks**
    Creates USERS.SYS file for DOOR applications. Use before SHELL statement to run 
    doors. If door modifies USERS.SYS, use RDUSYS after SHELL to read changes. 
    Cannot create TPA record with this statement.

  **Example**

    .. code-block:: PPL

       INTEGER ret
       WRUSYS
       SHELL FALSE,ret,"MYAPP.EXE",""
       RDUSYS

  **See Also**
    * :PPL:`RDUSYS` – Read USERS.SYS
    * :PPL:`SHELL` – Execute external program


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

REDIM (2.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT REDIM(VAR array, INTEGER dim1 [, INTEGER dim2, INTEGER dim3])`

  Dynamically resize an array at runtime.

  **Parameters**
    * :PPL:`array` – Previously declared array to resize
    * :PPL:`dim1,dim2,dim3` – New dimensions (must match original dimension count)

  **Remarks**
    Array must be declared with the desired number of dimensions before using REDIM. 
    Cannot change the number of dimensions, only their sizes. Existing data may be lost 
    when resizing.

  **Example**

    .. code-block:: PPL

       STRING s(1,1,1)
       REDIM s,5,5,5
       LET s(4,4,4) = "Hello, World!"
       PRINTLN s(4,4,4)

  **See Also**
    * Array declarations

APPEND (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT APPEND(STRING srcfile, STRING dstfile)`

  Append one file's contents to another.

  **Parameters**
    * :PPL:`srcfile` – Source file to read from
    * :PPL:`dstfile` – Destination file to append to

  **Remarks**
    Appends the entire contents of srcfile to the end of dstfile. Creates dstfile if it 
    doesn't exist. Leaves srcfile unchanged.

  **Example**

    .. code-block:: PPL

       APPEND "TODAY.LOG", "MASTER.LOG"
       DELETE "TODAY.LOG"

  **See Also**
    * :PPL:`COPY` – Copy file
    * :PPL:`FAPPEND` – Open file for append

COPY (2.00)
~~~~~~~~~~~
  :PPL:`STATEMENT COPY(STRING srcfile, STRING dstfile)`

  Copy a file to a new location.

  **Parameters**
    * :PPL:`srcfile` – Source file to copy
    * :PPL:`dstfile` – Destination file (overwrites if exists)

  **Remarks**
    Creates or overwrites dstfile with the contents of srcfile. Source file remains unchanged.

  **Example**

    .. code-block:: PPL

       COPY "CONFIG.DAT", "CONFIG.BAK"

  **See Also**
    * :PPL:`APPEND` – Append files
    * :PPL:`RENAME` – Rename/move file
    * :PPL:`DELETE` – Delete file

LASTIN (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT LASTIN(INTEGER conf)`

  Set user's last conference visited.

  **Parameters**
    * :PPL:`conf` – Conference number

  **Remarks**
    Forces the user's "last in" conference value. Useful in logon scripts to start users 
    in a specific conference regardless of where they were when they logged off.

  **Example**

    .. code-block:: PPL

       ; Force new users to main conference
       IF (U_EXPERT = "N") THEN
           LASTIN 0
       ENDIF

  **See Also**
    * :PPL:`JOIN` – Join conference
    * :PPL:`CURCONF()` – Current conference

FLAG (2.00)
~~~~~~~~~~~
  :PPL:`STATEMENT FLAG(STRING filename)`

  Flag a file for download.

  **Parameters**
    * :PPL:`filename` – Full path and filename to flag

  **Remarks**
    Directly flags any file for download, bypassing FSEC and DLPATH.LST restrictions. 
    Allows flagging files not normally accessible through standard download areas.

  **Example**

    .. code-block:: PPL

       FLAG "C:\PRIVATE\REPORT.ZIP"
       PRINTLN "File flagged. Use D command to download."

  **See Also**
    * :PPL:`DOWNLOAD` – Download files
    * :PPL:`FLAGCNT()` – Count flagged files

DOWNLOAD (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT DOWNLOAD(STRING cmds)`

  Initiate file download with commands.

  **Parameters**
    * :PPL:`cmds` – Download commands (same format as D or DB command)

  **Remarks**
    Processes download commands as if typed by user. Files must be accessible per FSEC 
    and DLPATH.LST unless previously flagged with FLAG statement.

  **Example**

    .. code-block:: PPL

       ; Download flagged files
       IF (FLAGCNT() > 0) THEN
           DOWNLOAD "D;Y"
       ENDIF

  **See Also**
    * :PPL:`FLAG` – Flag files
    * :PPL:`FLAGCNT()` – Count flagged files

WRUSYSDOOR (2.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT WRUSYSDOOR(STRING doorname)`

  Write USERS.SYS with TPA record for doors.

  **Parameters**
    * :PPL:`doorname` – Name of door application

  **Remarks**
    Creates USERS.SYS file with TPA (Third Party Application) record for enhanced door 
    compatibility. Some doors require TPA information for advanced features.

  **Example**

    .. code-block:: PPL

       INTEGER ret
       WRUSYSDOOR "TRADEWARS"
       SHELL FALSE,ret,"TW2002.EXE",""
       RDUSYS

  **See Also**
    * :PPL:`WRUSYS` – Standard USERS.SYS
    * :PPL:`RDUSYS` – Read USERS.SYS

KBDSTRING (2.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDSTRING(STRING str)`

  Stuff string to keyboard with echo.

  **Parameters**
    * :PPL:`str` – String to stuff (max 256 chars)

  **Remarks**
    Like KBDSTUFF but echoes characters to display as if typed. Useful when you want 
    the user to see what's being automated.

  **Example**

    .. code-block:: PPL

       KBDSTRING "DIR N;S"
       ; User sees: DIR N;S

  **See Also**
    * :PPL:`KBDSTUFF` – Stuff without echo
    * :PPL:`KBDFLUSH` – Clear keyboard buffer

KBDFLUSH (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KBDFLUSH`

  Clear local keyboard and stuffed keystroke buffers.

  **Remarks**
    Clears both the local keyboard buffer and any keystrokes stuffed with KBDSTUFF or 
    KBDSTRING. Does not affect modem input.

  **Example**

    .. code-block:: PPL

       KBDFLUSH  ; Clear any pending input
       INPUT "Enter choice", choice

  **See Also**
    * :PPL:`MDMFLUSH` – Clear modem buffer
    * :PPL:`KEYFLUSH` – Clear all buffers

MDMFLUSH (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT MDMFLUSH`

  Clear incoming modem buffer.

  **Remarks**
    Flushes all pending input from the modem. Does not affect local keyboard or stuffed 
    keystrokes.

  **Example**

    .. code-block:: PPL

       MDMFLUSH  ; Ignore any typed-ahead input
       INPUT "Fresh prompt", answer

  **See Also**
    * :PPL:`KBDFLUSH` – Clear keyboard buffer
    * :PPL:`KEYFLUSH` – Clear all buffers

KEYFLUSH (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT KEYFLUSH`

  Clear all input buffers.

  **Remarks**
    Flushes local keyboard buffer, stuffed keystrokes, and incoming modem buffer. 
    Ensures no pending input affects the next prompt.

  **Example**

    .. code-block:: PPL

       KEYFLUSH  ; Start fresh
       INPUTYN "Continue (Y/N)", yn, @X0E

  **See Also**
    * :PPL:`KBDFLUSH` – Clear keyboard only
    * :PPL:`MDMFLUSH` – Clear modem only

ALIAS (2.00)
~~~~~~~~~~~~
  :PPL:`STATEMENT ALIAS(BOOLEAN enable)`

  Control alias usage for current user.

  **Parameters**
    * :PPL:`enable` – TRUE to enable alias, FALSE to use real name

  **Remarks**
    Toggles between alias and real name if user and conference support aliases. Has no 
    effect if aliases not allowed.

  **Example**

    .. code-block:: PPL

       IF (CONFALIAS() & USERALIAS()) THEN
           ALIAS TRUE  ; Use alias
           PRINTLN "Now using alias: ", U_ALIAS
       ENDIF

  **See Also**
    * :PPL:`ALIAS()` – Get alias status
    * :PPL:`CONFALIAS()` – Check conference setting
    * :PPL:`USERALIAS()` – Check user permission

LANG (2.00)
~~~~~~~~~~~
  :PPL:`STATEMENT LANG(INTEGER langnum)`

  Change user's language.

  **Parameters**
    * :PPL:`langnum` – Language number to activate

  **Remarks**
    Changes the active language for prompts and displays. Language must be configured 
    in PCBML.DAT.

  **Example**

    .. code-block:: PPL

       LANG 2  ; Switch to language 2
       DISPTEXT 149, LFAFTER+NEWLINE

  **See Also**
    * :PPL:`LANGEXT()` – Get language extension

ADJBYTES (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT ADJBYTES(INTEGER bytes)`

  Adjust user's total and daily download bytes.

  **Parameters**
    * :PPL:`bytes` – Bytes to add (positive) or subtract (negative)

  **Remarks**
    Modifies both total and daily download byte counters. Use negative values to give 
    back credit, positive to charge.

  **Example**

    .. code-block:: PPL

       ; Give back 1MB credit
       ADJBYTES -1048576

  **See Also**
    * :PPL:`ADJDBYTES` – Adjust daily only
    * :PPL:`ADJTBYTES` – Adjust total only

ADJDBYTES (2.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT ADJDBYTES(INTEGER bytes)`

  Adjust user's daily download bytes only.

  **Parameters**
    * :PPL:`bytes` – Bytes to add (positive) or subtract (negative)

  **Example**

    .. code-block:: PPL

       ADJDBYTES -524288  ; Credit 512KB today only

  **See Also**
    * :PPL:`ADJBYTES` – Adjust both counters

ADJTBYTES (2.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT ADJTBYTES(INTEGER bytes)`

  Adjust user's total download bytes only.

  **Parameters**
    * :PPL:`bytes` – Bytes to add (positive) or subtract (negative)

  **Example**

    .. code-block:: PPL

       ADJTBYTES 1048576  ; Charge 1MB total

  **See Also**
    * :PPL:`ADJBYTES` – Adjust both counters

ADJTFILES (2.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT ADJTFILES(INTEGER files)`

  Adjust user's total download file count.

  **Parameters**
    * :PPL:`files` – Files to add (positive) or subtract (negative)

  **Example**

    .. code-block:: PPL

       ADJTFILES -5  ; Credit 5 files

PUTALTUSER (2.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT PUTALTUSER`

  Save alternate user record.

  **Remarks**
    Alias for PUTUSER when working with alternate user loaded via GETALTUSER. Makes code 
    clearer when updating alternate users.

  **Example**

    .. code-block:: PPL

       GETALTUSER 50
       LET U_SEC = 20
       PUTALTUSER  ; Save changes to user 50

  **See Also**
    * :PPL:`GETALTUSER` – Load alternate user
    * :PPL:`PUTUSER` – Save user record

GETALTUSER (2.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT GETALTUSER(INTEGER recnum)`

  Load alternate user record.

  **Parameters**
    * :PPL:`recnum` – User record number to load

  **Remarks**
    Loads specified user's data into U_XXX variables. User statements/functions redirect 
    to this user until FREALTUSER called or another GETALTUSER issued. Invalid record 
    numbers revert to current user. Changes to online users take effect after logoff. 
    ADJTIME always affects current user only.

  **Example**

    .. code-block:: PPL

       GETALTUSER 100
       PRINTLN "User 100 is: ", U_NAME()
       LET U_SEC = 30
       PUTALTUSER
       FREALTUSER

  **See Also**
    * :PPL:`FREALTUSER` – Release alternate user
    * :PPL:`CURUSER()` – Check active user

FREALTUSER (2.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT FREALTUSER`

  Release alternate user and restore current user.

  **Remarks**
    Frees memory used by GETALTUSER and reverts to current caller's data. Required before 
    MESSAGE statement if GETALTUSER was used, as MESSAGE needs alternate user access internally.

  **Example**

    .. code-block:: PPL

       GETALTUSER 20
       STRING name = U_NAME()
       FREALTUSER  ; Must free before MESSAGE
       MESSAGE 1, name, "Subject", "R", 0, FALSE, FALSE, "msg.txt"

  **See Also**
    * :PPL:`GETALTUSER` – Load alternate user

BITCLEAR (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT BITCLEAR(VAR var, INTEGER bit)`

  Clear a specific bit in a variable.

  **Parameters**
    * :PPL:`var` – Variable containing the bit to clear
    * :PPL:`bit` – Bit number to clear (0-based)

  **Remarks**
    Primarily designed for BIGSTR variables (up to 2048 bytes = 16384 bits). Works with 
    other data types but use caution when bit-twiddling non-string variables as it may 
    corrupt their values. If bit number is invalid (negative or beyond variable size), 
    no operation occurs.

  **Example**

    .. code-block:: PPL

       BIGSTR flags
       flags = CHR(255) + CHR(255)  ; All bits set
       
       ; Clear specific bits
       BITCLEAR flags, 0   ; Clear bit 0
       BITCLEAR flags, 8   ; Clear bit 8
       
       ; Use as flags array
       INTEGER i
       FOR i = 0 TO 100
           IF (ISBITSET(flags, i)) THEN
               PRINTLN "Flag ", i, " is ON"
           ENDIF
       NEXT

  **See Also**
    * :PPL:`BITSET` – Set a bit
    * :PPL:`ISBITSET()` – Check bit status

BITSET (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT BITSET(VAR var, INTEGER bit)`

  Set a specific bit in a variable.

  **Parameters**
    * :PPL:`var` – Variable containing the bit to set
    * :PPL:`bit` – Bit number to set (0-based)

  **Remarks**
    Primarily designed for BIGSTR variables (up to 2048 bytes = 16384 bits). Works with 
    other data types but use caution when bit-twiddling non-string variables as it may 
    corrupt their values. If bit number is invalid (negative or beyond variable size), 
    no operation occurs.

  **Example**

    .. code-block:: PPL

       BIGSTR userFlags
       
       ; Track user preferences as bits
       BITSET userFlags, 0   ; Email notifications
       BITSET userFlags, 1   ; Auto-signature
       BITSET userFlags, 2   ; Expert mode
       
       ; Store up to 16384 boolean flags in one BIGSTR

  **See Also**
    * :PPL:`BITCLEAR` – Clear a bit
    * :PPL:`ISBITSET()` – Check bit status

MOUSEREG (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT MOUSEREG(INTEGER num, INTEGER x1, INTEGER y1, INTEGER x2, INTEGER y2, INTEGER fontX, INTEGER fontY, BOOLEAN invert, BOOLEAN clear, STRING text)`

  Define a RIP mouse-clickable region.

  **Parameters**
    * :PPL:`num` – RIP region number
    * :PPL:`x1,y1` – Upper-left coordinates
    * :PPL:`x2,y2` – Lower-right coordinates  
    * :PPL:`fontX` – Character width in pixels
    * :PPL:`fontY` – Character height in pixels
    * :PPL:`invert` – TRUE to invert region when clicked
    * :PPL:`clear` – TRUE to clear and fullscreen text window
    * :PPL:`text` – Text to transmit when region clicked

  **Remarks**
    Creates clickable regions for RIP graphics terminals. When user clicks the defined 
    region, the terminal sends the specified text as if typed. Only works with RIP-enabled 
    terminals.

  **Example**

    .. code-block:: PPL

       ; Create clickable menu buttons
       MOUSEREG 1, 10, 10, 100, 30, 8, 14, TRUE, FALSE, "1"
       MOUSEREG 2, 10, 40, 100, 60, 8, 14, TRUE, FALSE, "2"
       MOUSEREG 3, 10, 70, 100, 90, 8, 14, TRUE, FALSE, "Q"

  **See Also**
    * :PPL:`RIPDETECT()` – Check RIP support

SCRFILE (2.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT SCRFILE(VAR INTEGER line, VAR STRING filename)`

  Find a filename on the current screen display.

  **Parameters**
    * :PPL:`line` – On entry: starting line (1=top); On exit: line where found or 0
    * :PPL:`filename` – On exit: filename if found, empty otherwise

  **Remarks**
    Searches the visible screen for filenames starting at the specified line. Useful for 
    file listing screens where users can select files by pointing. Sets line to 0 and 
    filename to empty if no filename found.

  **Example**

    .. code-block:: PPL

       INTEGER lineNum
       STRING fname
       
       DIR "N;NS"  ; Display new files
       lineNum = 1
       SCRFILE lineNum, fname
       
       IF (lineNum > 0) THEN
           PRINTLN "Found file: ", fname, " on line ", lineNum
           FLAG fname
       ENDIF

  **See Also**
    * :PPL:`DIR` – Display file directory
    * :PPL:`FLAG` – Flag file for download

SORT (2.00)
~~~~~~~~~~~
  :PPL:`STATEMENT SORT(ARRAY sortArray, VAR INTEGER pointerArray)`

  Sort array contents using pointer array.

  **Parameters**
    * :PPL:`sortArray` – One-dimensional array to sort (any type)
    * :PPL:`pointerArray` – Integer array to hold sorted indices

  **Remarks**
    Creates a pointer array containing indices to access sortArray in sorted order. 
    Original array remains unchanged. Both arrays must be one-dimensional and 
    pointerArray must be INTEGER type.

  **Example**

    .. code-block:: PPL

       STRING s(99)    ; 100 elements (0-99)
       INTEGER p(99)   ; Pointer array
       INTEGER i
       
       ; Fill array with data
       FOR i = 0 TO 99
           FGET 1, s(i)
       NEXT
       
       SORT s, p
       
       ; Display in sorted order
       PRINTLN "Sorted list:"
       FOR i = 0 TO 99
           PRINTLN s(p(i))  ; Access via pointer
       NEXT

  **See Also**
    * Array declarations

SEARCHINIT (2.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SEARCHINIT(STRING criteria, BOOLEAN caseSensitive)`

  Initialize Boyer-Moore search parameters.

  **Parameters**
    * :PPL:`criteria` – Search criteria in PCBoard format ("THIS & THAT | BOB")
    * :PPL:`caseSensitive` – TRUE for case-sensitive, FALSE otherwise

  **Remarks**
    Prepares optimized Boyer-Moore search algorithm for fast text searching. Supports 
    PCBoard search syntax with & (AND), | (OR) operators. Must call before SEARCHFIND.

  **Example**

    .. code-block:: PPL

       ; Search for messages containing both words
       SEARCHINIT "PPL & SCRIPT", FALSE
       
       STRING msgText
       BOOLEAN found
       FGET 1, msgText
       SEARCHFIND msgText, found
       
       IF (found) THEN
           PRFOUNDLN msgText  ; Highlight found words
       ENDIF
       
       SEARCHSTOP

  **See Also**
    * :PPL:`SEARCHFIND` – Perform search
    * :PPL:`SEARCHSTOP` – Clear search
    * :PPL:`PRFOUND` – Print with highlighting

SEARCHFIND (2.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SEARCHFIND(STRING buffer, VAR BOOLEAN found)`

  Search text using Boyer-Moore algorithm.

  **Parameters**
    * :PPL:`buffer` – Text to search
    * :PPL:`found` – Returns TRUE if criteria found, FALSE otherwise

  **Remarks**
    Performs fast Boyer-Moore search using criteria from SEARCHINIT. More efficient than 
    simple string searching for large texts or multiple searches.

  **Example**

    .. code-block:: PPL

       SEARCHINIT "SYSOP | ADMIN", FALSE
       
       STRING line
       BOOLEAN hasAuth
       
       WHILE (!FERR(1)) DO
           FGET 1, line
           SEARCHFIND line, hasAuth
           IF (hasAuth) THEN
               PRFOUNDLN line  ; Print with highlighting
           ENDIF
       ENDWHILE

  **See Also**
    * :PPL:`SEARCHINIT` – Initialize search
    * :PPL:`SEARCHSTOP` – Clear search
    * :PPL:`PRFOUND` – Print with highlighting

SEARCHSTOP (2.00)
~~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT SEARCHSTOP`

  Clear search criteria and free resources.

  **Remarks**
    Clears Boyer-Moore search criteria set by SEARCHINIT. Call when done searching to 
    free allocated memory. Automatically called at program end.

  **Example**

    .. code-block:: PPL

       SEARCHINIT "ERROR | WARNING", TRUE
       ; ... perform searches ...
       SEARCHSTOP  ; Clean up

  **See Also**
    * :PPL:`SEARCHINIT` – Initialize search
    * :PPL:`SEARCHFIND` – Perform search

PRFOUND (2.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT PRFOUND(ANY exp [, ANY exp...])`

  Print with search term highlighting.

  **Parameters**
    * :PPL:`exp` – Expression(s) to print

  **Remarks**
    Like PRINT but automatically highlights words found by the last SEARCHFIND. Only 
    highlights if last search was successful. No highlighting if search failed.

  **Example**

    .. code-block:: PPL

       SEARCHINIT "PPL | SCRIPT", FALSE
       STRING text = "Learning PPL script programming"
       BOOLEAN found
       
       SEARCHFIND text, found
       IF (found) THEN
           PRFOUND text  ; "PPL" and "script" highlighted
       ENDIF

  **See Also**
    * :PPL:`PRFOUNDLN` – Print line with highlighting
    * :PPL:`SEARCHFIND` – Perform search
    * :PPL:`PRINT` – Normal print

PRFOUNDLN (2.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT PRFOUNDLN([ANY exp...])`

  Print line with search term highlighting.

  **Parameters**
    * :PPL:`exp` – Expression(s) to print (optional)

  **Remarks**
    Like PRINTLN but automatically highlights words found by the last SEARCHFIND. Only 
    highlights if last search was successful. Appends newline after output.

  **Example**

    .. code-block:: PPL

       SEARCHINIT "ERROR | FAIL", TRUE
       
       STRING logLine
       BOOLEAN hasError
       
       FOPEN 1, "SYSTEM.LOG", O_RD, S_DW
       WHILE (!FERR(1)) DO
           FGET 1, logLine
           SEARCHFIND logLine, hasError
           IF (hasError) THEN
               PRFOUNDLN logLine  ; Highlight ERROR/FAIL
           ELSE
               PRINTLN logLine    ; Normal display
           ENDIF
       ENDWHILE

  **See Also**
    * :PPL:`PRFOUND` – Print without newline
    * :PPL:`SEARCHFIND` – Perform search
    * :PPL:`PRINTLN` – Normal print line    

TPAGET (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT TPAGET(STRING keyword, VAR STRING infoVar)`

  Get static string information from a named TPA.

  **Parameters**
    * :PPL:`keyword` – TPA keyword identifier
    * :PPL:`infoVar` – Variable to receive the information

  **Remarks**
    Retrieves string data from a Third Party Application's static storage area. TPAs 
    can store persistent data between calls. The keyword identifies which TPA's data 
    to access.

  **Example**

    .. code-block:: PPL

       STRING doorData
       TPAGET "TRADEWARS", doorData
       PRINTLN "TW2002 data: ", doorData

  **See Also**
    * :PPL:`TPAPUT` – Store TPA string data
    * :PPL:`TPAREAD` – Read TPA binary data
    * :PPL:`TPACGET` – Get conference TPA data

TPAPUT (2.00)
~~~~~~~~~~~~~
  :PPL:`STATEMENT TPAPUT(STRING keyword, ANY infoExpr)`

  Put static string information to a named TPA.

  **Parameters**
    * :PPL:`keyword` – TPA keyword identifier
    * :PPL:`infoExpr` – Expression to store

  **Remarks**
    Stores string data to a Third Party Application's static storage area. Data 
    persists between TPA calls and user sessions.

  **Example**

    .. code-block:: PPL

       TPAPUT "TRADEWARS", "PLAYER:" + U_NAME() + ";CREDITS:1000"

  **See Also**
    * :PPL:`TPAGET` – Retrieve TPA string data
    * :PPL:`TPAWRITE` – Write TPA binary data

TPACGET (2.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT TPACGET(STRING keyword, VAR STRING infoVar, INTEGER confNum)`

  Get conference-specific string data from a TPA.

  **Parameters**
    * :PPL:`keyword` – TPA keyword identifier
    * :PPL:`infoVar` – Variable to receive the information
    * :PPL:`confNum` – Conference number

  **Remarks**
    Retrieves string data from a TPA's conference-specific storage. Allows TPAs to 
    maintain separate data for each conference.

  **Example**

    .. code-block:: PPL

       STRING gameData
       INTEGER conf = CURCONF()
       TPACGET "RPGDOOR", gameData, conf
       PRINTLN "RPG data for conference ", conf, ": ", gameData

  **See Also**
    * :PPL:`TPACPUT` – Store conference TPA data
    * :PPL:`TPACREAD` – Read conference binary data

TPACPUT (2.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT TPACPUT(STRING keyword, ANY infoExpr, INTEGER confNum)`

  Put conference-specific string data to a TPA.

  **Parameters**
    * :PPL:`keyword` – TPA keyword identifier
    * :PPL:`infoExpr` – Expression to store
    * :PPL:`confNum` – Conference number

  **Remarks**
    Stores string data to a TPA's conference-specific storage area. Each conference 
    maintains separate TPA data.

  **Example**

    .. code-block:: PPL

       INTEGER conf = CURCONF()
       TPACPUT "RPGDOOR", "LEVEL:5;HP:100", conf

  **See Also**
    * :PPL:`TPACGET` – Retrieve conference TPA data
    * :PPL:`TPACWRITE` – Write conference binary data

TPAREAD (2.00)
~~~~~~~~~~~~~~
  :PPL:`STATEMENT TPAREAD(STRING keyword, VAR infoVar)`

  Read static binary information from a TPA.

  **Parameters**
    * :PPL:`keyword` – TPA keyword identifier
    * :PPL:`infoVar` – Variable to receive data (any type)

  **Remarks**
    Reads binary data from a TPA's static storage. Unlike TPAGET which handles strings, 
    TPAREAD preserves binary data types.

  **Example**

    .. code-block:: PPL

       INTEGER score
       REAL multiplier
       TPAREAD "GAME", score
       TPAREAD "GAMEMULT", multiplier
       PRINTLN "Score: ", score * multiplier

  **See Also**
    * :PPL:`TPAWRITE` – Write TPA binary data
    * :PPL:`TPAGET` – Get TPA string data

TPAWRITE (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT TPAWRITE(STRING keyword, ANY infoExpr)`

  Write static binary information to a TPA.

  **Parameters**
    * :PPL:`keyword` – TPA keyword identifier
    * :PPL:`infoExpr` – Expression to store (any type)

  **Remarks**
    Writes binary data to a TPA's static storage. Preserves data types for numeric 
    and other non-string values.

  **Example**

    .. code-block:: PPL

       INTEGER highScore = 50000
       TPAWRITE "HIGHSCORE", highScore
       
       REAL ratio = 1.5
       TPAWRITE "BONUSRATIO", ratio

  **See Also**
    * :PPL:`TPAREAD` – Read TPA binary data
    * :PPL:`TPAPUT` – Put TPA string data

TPACREAD (2.00)
~~~~~~~~~~~~~~~
  :PPL:`STATEMENT TPACREAD(STRING keyword, VAR infoVar, INTEGER confNum)`

  Read conference-specific binary data from a TPA.

  **Parameters**
    * :PPL:`keyword` – TPA keyword identifier
    * :PPL:`infoVar` – Variable to receive data (any type)
    * :PPL:`confNum` – Conference number

  **Remarks**
    Reads binary data from a TPA's conference-specific storage. Maintains separate 
    data for each conference with type preservation.

  **Example**

    .. code-block:: PPL

       INTEGER level, conf
       conf = CURCONF()
       TPACREAD "USERLEVEL", level, conf
       PRINTLN "User level in conf ", conf, ": ", level

  **See Also**
    * :PPL:`TPACWRITE` – Write conference binary data
    * :PPL:`TPACGET` – Get conference string data

TPACWRITE (2.00)
~~~~~~~~~~~~~~~~
  :PPL:`STATEMENT TPACWRITE(STRING keyword, ANY infoExpr, INTEGER confNum)`

  Write conference-specific binary data to a TPA.

  **Parameters**
    * :PPL:`keyword` – TPA keyword identifier
    * :PPL:`infoExpr` – Expression to store (any type)
    * :PPL:`confNum` – Conference number

  **Remarks**
    Writes binary data to a TPA's conference-specific storage. Each conference 
    maintains separate TPA data with preserved types.

  **Example**

    .. code-block:: PPL

       INTEGER points = 100
       REAL bonus = 2.5
       INTEGER conf = CURCONF()
       
       TPACWRITE "POINTS", points, conf
       TPACWRITE "BONUS", bonus, conf

  **See Also**
    * :PPL:`TPACREAD` – Read conference binary data
    * :PPL:`TPACPUT` – Put conference string data