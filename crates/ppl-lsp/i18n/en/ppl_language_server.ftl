hint-type-boolean=unsigned character (1 byte) 0 = FALSE, non-0 = TRUE
hint-type-date=unsigned integer (2 bytes) PCBoard julian date (count of days since 1/1/1900) 
hint-type-ddate=
    Signed long integer for julian date. DDATE is for use with DBase date fields.
    It holds a long integer for julian dates. When coerced to string type it is in the format CCYYMMDD or 19940527
hint-type-integer=signed long integer (4 bytes) Range: -2,147,483,648 → +2,147,483,647
hint-type-money=signed long integer (4 bytes) Range: -$21,474,836.48 → +$21,474,836.47
hint-type-string=far character pointer (4 bytes) NULL is an empty string non-NULL points to a string of some length less than or equal to 256
hint-type-time=signed long integer (4 bytes) Count of seconds since midnight
hint-type-bigstr=Allows up to 2048 characters per big string (up from 256 for STRING variables) May include CHR(0) characters in the middle of the big string (unlike STRING variables which may not)
hint-type-edate=Julian date in earth date format Deals with dates formatted YYMM.DD Range: Same as DATE
hint-type-float=4-byte floating point number Range: +/-3.4E-38 - +/-3.4E+38 (7-digit precision)
hint-type-double=8-byte floating point number Range: +/-1.7E-308 - +/-1.7E+308 (15-digit precision)
hint-type-unsigned=4-byte unsigned integer Range: 0 - 4,294,967,295
hint-type-byte=1-byte unsigned integer Range: 0 - 255
hint-type-word=2-byte unsigned integer Range: 0 - 65,535
hint-type-sbyte=1-byte signed Integer Range: -128 - 127
hint-type-sword=2-byte signed integer Range: -32,768 - 32,767

hint-statement-end=Ends the program execution
hint-statement-cls=Clears the screen
hint-statement-clreol=Clears to the end of the line
hint-statement-more=Pauses and waits for a keypress (Displays a MORE? prompt)
hint-statement-wait=Pauses and waits for a keypress
hint-statement-color=Sets the text color to @1
hint-statement-goto=Jumps to the label specified
hint-statement-let=Assigns the value of `exp` to `var1`
hint-statement-print=
    Print a line to the screen

    ### Remarks
    This statement will process all @ codes and display them as expected.
hint-statement-println=
    Print a line to the screen and append a newline to the end of the expression(s).

    ### Remarks
    This statement will process all @ codes and display them as expected.
hint-statement-confflag=Turn on the conference @1 flags specified by @2
hint-statement-confunflag=Turn off the conference @1 flags specified by @2
hint-statement-dispfile=
    Display file @1 with @2 alternate file flags
    ### Valid Flags
    - `GRAPH`
    - `SEC`
    - `LANG`
hint-statement-input=Display @1 and get input from user, assigning it to @2 (60 characters maximum)
hint-statement-fcreate=
    Use channel @1 to create and open file @2 in access mode @3 and share mode @4
    | Valid | Values |
    | :--- | :--- |
    | Channels     | `0` - `7` (`0` is used for surveys) |
    | Access Modes | `O_RD`, `O_WR`, `O_RW` (should use `O_WR`) |
    | Share Modes  | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fopen=
    Use channel @1 to open file @2 in access mode @3 and share mode @4
    | Valid | Values |
    | :--- | :--- |
    | Channels     | `0` - `7` (`0` is used for surveys) |
    | Access Modes | `O_RD`, `O_WR`, `O_RW` (should use `O_WR`) |
    | Share Modes  | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fappend=
    Use channel @1 to append to file @2 in access mode @3 and share mode @4
    | Valid | Values |
    | :--- | :--- |
    | Channels     | `0` - `7` (`0` is used for surveys) |
    | Access Modes | `O_RD`, `O_WR`, `O_RW` (should use `O_WR`) |
    | Share Modes  | `S_DN`, `S_DR`, `S_DW`, `S_DB` |
hint-statement-fclose=
    Close channel @1

    Accept channel -1 as the `ReadLine()` function 'channel' and close it
hint-statement-fget=Read a line from channel @1 and assign it to @2
hint-statement-fput=Write one or more @2 out to channel @1
hint-statement-fputln=Write one or more @2 out to channel @1 and terminate with a carriage return/line feed pair
hint-statement-resetdisp=Reset the display after an user abort
hint-statement-startdisp=
    Start display monitoring in mode @1
    ### Valid Modes
    - `NC`
    - `FNS`
    - `FCL`
hint-statement-fputpad=Write out @2, padding or truncating to length @3 as needed, to channel @1
hint-statement-hangup=Hangup on the user without any notification
hint-statement-getuser=Fill the predefined variables (U_…) with current information from the user record
hint-statement-putuser=
    Write the information from the predefined variables (U_…) to the user record
    This statement is only intended to update user information if a successful GetUser or GetAltUser was issued previously.
    This was done to ensure that information for the current user wasn't written to another user or vice versa.
hint-statement-defcolor=Resets the current color to the system default
hint-statement-delete=Deletes the filename specified by @1 (`ERASE` is a synonym)
hint-statement-deluser=Flags the current user record for deletion
hint-statement-adjtime=Add or subtract @1 minutes to the users time available this session
hint-statement-log=Write string @1 to the callers log, left justified if @2 is `TRUE`
hint-statement-inputstr=
    Display @1 in color @3 and get a string (maximum length @4, valid characters @5, flags @6) from the user, assigning it to @2

    ### Valid Flags
    `ECHODOTS`, `FIELDLEN`, `GUIDE`, `UPCASE`, `STACKED`, `ERASELINE`, `NEWLINE`, `LFBEFORE`, `LFAFTER`, `WORDWRAP`, `NOCLEAR`, `HIGHASCII`, `AUTO`, `YESNO`  
hint-statement-inputyn=
    Display @1 in color @3 and get a yes/no response from the user, assigning it to @1 (1 characters maximum, valid characters determined by language)
hint-statement-inputmoney=
    Display @1 in color @3 and get a money formatted string from the user, assigning it to @1 (13 characters maximum, valid characters `0-9 $ .`)
hint-statement-inputint=
    Display @1 in color @3 and get an integer formatted string from the user, assigning it to @1 (11 characters maximum, valid characters `0-9`)
hint-statement-inputcc=
    Display @1 in color @3 and get a credit card formatted string from the user, assigning it to @1 (16 characters maximum, valid characters `0-9`)
hint-statement-inputdate=
    Display @1 in color @3 and get a date formatted string from the user, assigning it to @1 (8 characters maximum, valid characters `0-9 - /`)
hint-statement-inputtime=
    Display @1 in color @3 and get a time formatted string from the user, assigning it to @1 (8 characters maximum, valid characters `0-9 :`)
hint-statement-gosub=Transfer control to `LABEL`, marking the current PPE location for a future Return statement (`GO SUB` is a synonym)
hint-statement-return=Return to the statement after the last `GoSub` or, if no `GoSub` is waiting for a `RETURN`, end the PPE
hint-statement-promptstr=
    Display PCBTEXT entry @1 and get a string (maximum length @3, valid characters @4, flags @5) from the user, assigning it to @1
    ### Valid Flags
    `ECHODOTS`, `FIELDLEN`, `GUIDE`, `UPCASE`, `STACKED`, `ERASELINE`, `NEWLINE`, `LFBEFORE`, `LFAFTER`, `WORDWRAP`, `NOCLEAR`, `HIGHASCII`, `AUTO`, `YESNO`  
hint-statement-dtron=Turn on the DTR signal
hint-statement-dtroff=
    Turn off the DTR signal,

    Note: on most modems, lowering DTR will cause modem to hangup… this is a good way if you want to simulate a bad connection,
    and then hangup without goodbye screens… This is the best way for you, the nice sysop, to free your line quickly… :)
hint-statement-cdchkon=Turn on carrier detect checking
hint-statement-cdchkoff=Turn off carrier detect checking
hint-statement-delay=Pause for @1 clock ticks (1 clock tick = 1/18.2 second)
hint-statement-sendmodem=Send the text in @1 out to the modem
hint-statement-inc=Increment the value of @1
hint-statement-dec=Decrement the value of @1
hint-statement-newline=Write a newline to the display
hint-statement-newlines=Write @1 newlines to the display
hint-statement-tokenize=Tokenize string @1 into individual items separated by semi-colons or spaces
hint-statement-gettoken=
    ### Returns
    The next string token from a prior call to `Tokenize` (Same as the `GETTOKEN` statement but can be used in an expression without prior assignement to a variable)
    
    ### Example
    `GETTOKEN VAR`
    
    Get a token from a previous call to Tokenize and assign it to `VAR`
hint-statement-shell=
    Shell (via COMMAND.COM if @1 is `TRUE`) to program/command @2 with arguments @3, saving the return value in @1
    NOTE: If @1 is `TRUE`, the value assigned to @1 will be the return code of COMMAND.COM, not @3)
hint-statement-disptext=
    Display PCBTEXT prompt @1 using flags @2

    ### Valid Flags
    `NEWLINE`, `LFBEFORE`, `LFAFTER`, `BELL`, `LOGIT`, `LOGITLEFT`
hint-statement-stop=Abort PPE execution without appending answers (channel 0) to the answer file
hint-statement-inputtext=Display @1 in color @3 and get a string (maximum length @4) from the user, assigning it to @1
hint-statement-beep=Beeps the speaker
hint-statement-push=Push a list of evaluated expressions onto the stack
hint-statement-pop=Pop values (previously pushed onto the stack) into a list of variables
hint-statement-kbdstuff=Stuff the keyboard buffer with the contents of @1
hint-statement-call=Load and execute PPE filename specified by @1
hint-statement-join=Performs a join conference command, passing it @1 as arguments
hint-statement-quest=Do script questionnaire @1
hint-statement-blt=Display bulletin number @1
hint-statement-dir=Performs a file directory command, passing it @1 as arguments
hint-statement-kbdfile=Stuff the keyboard buffer with the contents of file @1
hint-statement-bye=Same as having the user type BYE from the command prompt
hint-statement-goodbye=Same as having the user type G from the command prompt
hint-statement-broadcast=Broadcast message @3 to nodes from @1 to @2 inclusive
hint-statement-waitfor=
    Wait up to @3 seconds for the string @1, assigned `TRUE` to @1 if the string is found in the time specified or `FALSE` if the string is not found (`WAIT FOR` is a synonym)
hint-statement-kbdchkon=Turn on keyboard time out checking
hint-statement-kbdchkoff=Turn off keyboard time out checking
hint-statement-optext=Writes string @1 into the `@OPTEXT@` macro
hint-statement-dispstr=Display file if @1 is `“%filename”`, execute PPE if @1 is `“!filename”`, or display string @1
hint-statement-rdunet=Read information from USERNET.XXX for node @1
hint-statement-wrunet=
    Write information to USERNET.XXX for node @1, where @2 is the new node status,
     @3 is the new node user name, 
     @4 is the new node city, 
     @5 is the new node operation text, 
     and @6 is broadcast text
hint-statement-dointr=Generate interrupt number “intr” (0-255) with the register values passed as parameters
hint-statement-varseg=Assign the segment address of @1 to @2
hint-statement-varoff=Assign the offset address of @1 to @2
hint-statement-pokeb=Assign the value @2 (0-255) to memory address @1 (POKE is a synonym)
hint-statement-pokew=Assign the value @2 (0-65535) to memory address @1
hint-statement-varaddr=Assign the address (segment and offset) of @1 to @2
hint-statement-ansipos=
    Move the cursor to column @1 and row @2

    ```
    1 <= @1 <= 80  
    1 <= @2 <= 23 (Because of the status lines)  
    ```
    (1,1) is the top left corner of the screen
hint-statement-backup=Backup (move the cursor to the left) @1 columns without going past column 1
hint-statement-forward=Move the cursor forward @1 columns without going past column 80
hint-statement-freshline=If the cursor is not in column 1, do a newline
hint-statement-wrusys=Writes (creates) a USERS.SYS file which can be used by a SHELLed application
hint-statement-rdusys=Reads a USERS.SYS file, if present, and updates the users record
hint-statement-newpwd=todo
hint-statement-opencap=
    Open @1 and capture all screen output to it.
    If an error occurs creating or opening @1, @2 is set to `TRUE`, otherwise @2 is set to `FALSE`.
hint-statement-closecap=Close the capture file previously opened with OpenCap
hint-statement-message=
    Write a message in conference @1, to user @2 (empty string defaults to current caller), 
    from user @3 (empty string defaults to current caller), subject @4, 
    security in @5 ("N" or "R"; "N" is the default),
    pack out date in @6 (0 for no pack out date), 
    @7 True if return receipt desired, 
    @8 TRUE if message should be echoed, and
    @9 is the filename to use for the message text
hint-statement-savescrn=Save the current screen in a buffer for later restoration with the RestScrn
hint-statement-restscrn=Restore the screen from the buffer previously saved with SaveScrn
hint-statement-sound=
    Turn on the BBS PC speaker at the frequency (1-65535) specified by @1 (or turn it off if the frequency is 0)
hint-statement-chat=Initiate SysOp chat mode
hint-statement-sprint=
    Display one or more string expressions on the BBS screen only (this statement does not send anything to the modem)
hint-statement-sprintln=
    Display zero or more string expressions on the BBS screen only and follow with a newline (this statement does not send anything to the modem)
hint-statement-mprint=
    Display one or more string expressions on the callers screen only (this statement does not send anything to the BBS screen)
hint-statement-mprintln=
    Display zero or more string expressions on the callers screen only and follow with a newline (this statement does not send anything to the BBS screen)
hint-statement-rename=Rename file @1 to @2
hint-statement-frewind=Rewind channel @1 after flushing buffers and committing the file to disk.
hint-statement-pokedw=Assign the value @2 (-2147483648 - +2147483647) to memory address @1
hint-statement-dbglevel=Assign the debug level to @1
hint-statement-showon=Turns on display of information to the screen
hint-statement-showoff=Turns off display of information to the screen
hint-statement-pageon=Turn on the SysOp paged indicator (flashing p on status line)
hint-statement-pageoff=Turn off the SysOp paged indicator (flashing p on status line)
hint-statement-fseek=
    Position to any random location within a file
    @2 is the number of bytes to move (+/-) relative to position
    @3 is the base location to start the seek from:

    `SEEK_SET (0)` for the beginning of the file

    `SEEK_CUR (1)` for the current file pointer location  

    `SEEK_END (2)` for the end of the file  
hint-statement-fflush=flush a specified channel changes to disk
hint-statement-fread=
    Read binary data from a file.

    @1 is the channel number

    @2 is the variable to store the data

    @3 is the number of bytes to read
hint-statement-fwrite=
    Write binary data to a file

    @1 is the channel number

    @2 is the expression whose result should be written

    @3 is the size of data to write to var
hint-statement-fdefin=Specify a default input file channel (used to speed up file input)
hint-statement-fdefout=Specify a default output file channel (used to speed up file output)
hint-statement-fdget=Default channel input statement: use the exact same arguments as FGet except a channel parameter (the channel specified by FDefIn is assumed)
hint-statement-fdput=Default channel output statement: use the exact same arguments as FPut except a channel parameter (the channel specified by FDefOut is assumed)
hint-statement-fdputln=Default channel output statement: use the exact same arguments as FPutLn except a channel parameter (the channel specified by FDefOut is assumed)
hint-statement-fdputpad=Default channel output statement: use the exact same arguments as FPutPad except a channel parameter (the channel specified by FDefOut is assumed)
hint-statement-fdread=Default channel input statement: use the exact same arguments as FRead except a channel parameter (the channel specified by FDefIn is assumed)
hint-statement-fdwrite=Default channel output statement: use the exact same arguments as FWrite except a channel parameter (the channel specified by FDefOut is assumed)
hint-statement-adjbytes=
    Adjust the users total and daily download.

    To subtract bytes use a negative number for bytes.

    To add bytes use a positive number.
hint-statement-kbdstring=Stuff strings to the keyboard (just like KbdStuff except 'keystrokes' are echoed to the display)
hint-statement-alias=todo
hint-statement-redim=todo
hint-statement-append=Append the contents of one file to another file.
hint-statement-copy=Copy the contents of one file to another file.
hint-statement-kbdflush=Flush the local keyboard buffer and any stuffed keystroke buffers. It takes no arguments.
hint-statement-mdmflush=Flush the incoming modem buffer. It takes no arguments.
hint-statement-keyflush=Flush both the local buffers and the incoming modem buffer. It takes no arguments.
hint-statement-lastin=Set the users last conference in value. It can be used during the logon process to force the user into a particular conference at start up (for example, from a logon script).
hint-statement-flag=Allow flagging files for download directly from a PPE.
hint-statement-download=
    Downloading files from PPL.
    
    The string passed to DOWNLOAD is a list of commands in the same format as what a user would type after a D or DB command.

    If a file name for download is specified here it must be downloadable according to the criteria established in the FSEC and DLPATH.LST files.

    If it is necessary to download a file not normally available via the FSEC and/or DLPATH.LST files the FLAG statement may be used to force it into the list of files to download.
hint-statement-wrusysdoor=Write a USERS.SYS file with a TPA record for a DOOR application.
hint-statement-getaltuser=
    Get the information for an alternate user.

    It will fill the user variables with information from the specified user record as well as redirect user statements and functions.

    If an attempt is made to get a record number that doesn't exist, 
    the user functions will revert to the current user and the user variables will be invalidated as though no GetUser/GetAltUser 
    statement had been issued (though they will continue to maintain any value held). 

    `PutUser`/`PutAltUser` should be issued to commit any variable changes to the user record.
    Additionally, there is at least one statement that will not affect alternate users: `AdjTime`. 
    
    It is restricted to the current user online.
    
    Also, if the alternate user is online, changes to the record won't take hold until after the user has logged off. 
    Also, if there is not enough memory available (primarily for the last message read pointers) this statement will fail.
hint-statement-adjdbytes=
    Adjust the users daily download bytes.

    To subtract bytes use a negative number for bytes.

    To add bytes use a positive number.
hint-statement-adjtbytes=
    Adjust the users total download bytes.
    
    To subtract bytes use a negative number for bytes.

    To add bytes use a positive number.
hint-statement-adjtfiles=
    Adjust the users total download files.

    To subtract files use a negative number for files.

    To add files use a positive number.
hint-statement-lang=Change the language in use by the current user.
hint-statement-sort=
    Sort the contents of an array into a pointer array.

    Note that sortArray and pointerArray are restricted to one (1) dimensional arrays
hint-statement-mousereg=
    Set up a RIP mouse region on the remote terminal.
    
    | | |
    | --- | --- |
    | @1 | Is the RIP region number| 
    | @2, @3 | The (X,Y) coordinates of the upper-left of the region |
    | @4, @5 | The (X,Y) coordinates of the lower-right of the region |
    | @6 | The width of each character in pixels |
    | @7 | The height of each character in pixels |
    | @8 | A boolean flag (TRUE to invert the region when clicked) |
    | @9 | A boolean flag (TRUE to clear and full screen the text window) | 
    | @10 | Text that the remote terminal should transmit when the region is clicked |
hint-statement-scrfile=Find a file name and line number that is currently on the screen.
hint-statement-searchinit=Initialize search parameters for a faster BOYER-MOORE search algorithm.
hint-statement-searchfind=Execute a BOYER-MOORE search on a text buffer using criteria previously defined with a SearchInit statement.
hint-statement-searchstop=Clears out previously entered search criteria. It takes no parameters.
hint-statement-prfound=These work just like Print and PrintLn but, if the last SearchFind statement resulted in a match, it will automatically highlight found words.
hint-statement-prfoundln=These work just like Print and PrintLn but, if the last SearchFind statement resulted in a match, it will automatically highlight found words.
hint-statement-tpaget=Get static information from a named TPA in string format.
hint-statement-tpaput=Put static information to a named TPA in string format.
hint-statement-tpacget=
    Get information from a named TPA for a specified conference in string format.

    @1 The keyword of the TPA to use  

    @2 The variable into which to store the information  

    @3 The conference number for which to retrieve information  
hint-statement-tpacput=
    Put information to a named TPA for a specified conference in string format.
    
    @1 The keyword of the TPA to use  

    @2 The expression to write to store the TPA

    @3 The conference number for which to retrieve information  
hint-statement-tparead=
    Get static information from a named TPA.

    @1 The keyword of the TPA to use  

    @2 The variable into which to store the information  
hint-statement-tpawrite=
    Put static information to a named TPA.

    @1 The keyword of the TPA to use  

    @2 The expression to write to store the TPA
hint-statement-tpacread=
    Get information from a named TPA for a specified conference.

    @1 The keyword of the TPA to use  

    @2 The variable into which to store the information

    @3 The conference number for which to retrieve information  
hint-statement-tpacwrite=
    Put information to a named TPA for a specified conference.
    
    @1 The keyword of the TPA to use  

    @2 The expression to write to store the TPA

    @3 The conference number for which to retrieve information  
hint-statement-bitset=
    Set a specified bit from a variable.

    This statement is primarily intended to be used with BIGSTR variables which can be up to 2048 bytes long. 
    However, it will work with other data types as well if desired.
    
    Just be aware of the potential problems in 'bit twidling' non-string buffers and then trying to access them later as their 'intended' 
    type without re-initializing the variable. 
    
    If the bit parameter (an integer from 0 to the number of bits in the object) is invalid no processing takes place.
hint-statement-bitclear=
    Clears a specified bit from a variable.

    This statement is primarily intended to be used with BIGSTR variables which can be up to 2048 bytes long.
    
    However, it will work with other data types as well if desired. Just be aware of the potential problems in 'bit twidling' 
    non-string buffers and then trying to access them later as their 'intended' type without re-initializing the variable.
    
    If the bit parameter (an integer from 0 to the number of bits in the object) is invalid no processing takes place.
hint-statement-brag=todo
hint-statement-frealtuser=
    Since only one `GETALTUSER` can be active at one time, `FREALTUSER` can allow other processes which need to use `GETALTUSER` (such as the `MESSAGE` commend) to do so.
hint-statement-setlmr=
    Set the last read pointers for the specified conference.

    If @1 is greater than the number of actual confrences @1 will default to the highest conference number.
    
    If @2 is greater than the highest message number in that conference, it will default to the highest message number in that conference. 
    This could be used to set a new users mesg pointers to recent messages so they aren't replying to 3 years old messages. 
    A useful feature would be to get the high conference number.
hint-statement-setenv=
    Set an environment variable.

    String format is:`"VAR=VALUE"`
hint-statement-fcloseall=Closes all file channels
hint-statement-stackabort=
    This allows the programmer to tell the runtime module to try its best to continue executing after a stack error has occurred.
    
    If it is passed `FALSE`, it will abort execution after a stack error. If it is passed `TRUE` the PPE will continue to run.

    > [!CAUTION]
    > If you continue to execute after a stack error, program execution will be unpredictable.
    > PPL will not allow system memory to be corrupted because of a stack error.
hint-statement-dcreate=create DBF file
hint-statement-dopen=open DBF file
hint-statement-dclose=close DBF file
hint-statement-dsetalias=set DBF alias
hint-statement-dpack=pack DBF file
hint-statement-dcloseall=close all NDX files
hint-statement-dlock=lock DBF file
hint-statement-dlockr=lock a record
hint-statement-dlockg=lock a group of records
hint-statement-dunlock=unlock any current locks
hint-statement-dncreate=create NDX file
hint-statement-dnopen=open NDX file
hint-statement-dnclose=close NDX file
hint-statement-dncloseall=close all NDX files
hint-statement-dnew=start a new record
hint-statement-dadd=add the new record
hint-statement-dappend=append a blank record
hint-statement-dtop=go to top record
hint-statement-dgo=go to specific record
hint-statement-dbottom=go to bottom record
hint-statement-dskip=skip +/- a number of records
hint-statement-dblank=blank the record
hint-statement-ddelete=delete the record
hint-statement-drecall=recall the record
hint-statement-dtag=select a tag
hint-statement-dseek=
    returns error status ( 0|1 )
    ; or seek success (0 = Error
    ; 1 = success, 2 = following record
    ; 3 = end of file )
hint-statement-dfblank=blank a named field
hint-statement-dget=get a value from a named field
hint-statement-dput=put a value to a named field
hint-statement-dfcopy=copy a field to a field
hint-statement-account=
    @1 is a value between 0-14. Using system constants is recommended.
    
    @2 is the amount of credits to add or subtract to field the field
hint-statement-recordusage=
    @1 is the field number to access (using DEB… consts) descr1 is the descripttion of the charge descr2 is a subdescription of the charge unitcost 
    is the cost per unit value is the number of units Recordusage will update debit values in PCBoard as well as record descriptions and other 
    information in an accounting file.
    
    Valid values for the field parameter are 2-16. The constants corresponding with these values (DEB???) could and should be used here. 
    
    (see the Accounting section for a list of consts)

hint-statement-msgtofile=
    Writes a message into a file.

    This statement will take the given message and write it to a text file. 
    The file's first 15 lines will contain standard header information. (One field per line) The headers are formatted to make parsing easier.
    The 16th line will state how many extended headers are present. The following line(s) will contain extended headers.
    (one per line) Finally, after the extended headers, will be a line containing “Message body:”. Everything after that is the body of the message.

hint-statement-qwklimits=
    This statement allows the PPL programmer to modify a users QWK limits. Four fields can be modified with their statement.
hint-statement-command=
    Process a command as if it were typed on the prompt.

    @1: A boolean value indcating whether or not to try to find the command in CMD.LST.
    If TRUE and the command is not in CMD.LST, it will try the standard commands automatically, failing if the command does not exist.

    @2: A string value with the command and parameters to execute. (like “R A Y O S”)

    NOTES!!! Not all portions of PCBoard are re-entrant. For example, you should not try to have two message editor processes active at the same time (in other words, you shouldn't launch the message editor from within a MNU and then launch a PPE from a shifted function key that tries to enter another message). So you'll need to be carefull about nested COMMAND (or equivalent) calls. But sequential processing should not be a problem at all.
    If it is determined at some point in the future that allowing this flexibility causes more problems than it solves, the COMMAND statement will be scaled back to ensure that attempts are not made to re-enter code. So use it well and wisely!
hint-statement-uselmrs=todo
hint-statement-confinfo=
    This statement can be used to modify a field in the conference
    configuration.

    @1 = The conference number to get information about
    
    @2 = Conference field to modify. (See note)
    
    @3 = New value to store in field

    { conference_access_constants }

hint-statement-adjtubytes=
    This statement can be used to adjust a users total uploads bytes
    up or down.
    
    @1 = Number of bytes to adjust current users upload bytes with.
    This can be a positive or negative value
hint-statement-grafmode=
    This statement can be used to change a users graphics mode while online.

    @1 = The graphics mode to change to.
    1 = If user has ANSI capabilities it will change graphics to color ANSI
    2 = Will attempt to put user in color ansi regardless of ansi ability
    3 = Puts user in ansi-black and white mode
    4 = Puts user in non-ansi black and white mode
    5 = If user has RIP ability, will put user in RIP mode.
    (IcyBoard: 6 = Avatar mode
    )

hint-statement-adduser=
    @1 = name of the new user to add
    @2 = TRUE instructs PCBoard to leave the new user's variables active, as if a GETALTUSER were executed
    using the new user record number. FALSE will restore the current users variables.

    ### Remarks
    This statement allows PPL to create a new user record, filling
    in all fields but the name with pcboard default values.
hint-statement-killmsg=
    @1 = conference number in which the doomed message resides.
    @2 = message number to kill
hint-statement-chdir=Changes to directory
hint-statement-mkdir=Creates a new directory
    
    @1 = directory to create 
hint-statement-rmdir=
    Removes a directory

    @1 = directory to remove 

    ### Note
    The directory must be empty before removing it.
hint-statement-fdowraka=todo
hint-statement-fdoaddaka=todo
hint-statement-fdowrorg=todo
hint-statement-fdoaddorg=todo
hint-statement-fdoqmod=This statement can be used to modify fido queue records.todo
hint-statement-fdoqadd=This statement can be used to add entries to the FIDO queue.
hint-statement-fdoqdel=This statement can be used to delete fido queue records.
hint-statement-sounddelay=
    @1 = frequency at which to sound the PC speaker
    @2 = length, in clock ticks (18 = 1 second), to leave the speaker on

    ### Remarks
    This function was added to replace the
    ```
    SOUND 500
    SOUND 0
    ```
    combination required for DOS, since this functionality is not available under OS/2.
hint-statement-shortdesc=
    Sets the current user's status for viewing short (one line) or full file descriptions.

    @1 = A boolean expression stating if the short description set on.
hint-statement-movemsg=
    Moves the message from its current location to the end of the message base.

    @1 = conference number in which the message resides
    @2 = message number to move
    @3 = A Boolean expression stating where the message should be
    move or not.  TRUE if it will be moved, FALSE if the message is to be copied.
hint-statement-setbankbal=
    Sets the value of a specified field.

    @1 An interger expression stating the field to get.
    @2 An interger expression stating the value that the specified field is to set to.

    ### Fields
    Time Fields (in minutes)
    ------------------------
        0 = Last Deposit Date
        1 = Last Withdrawal Date
        2 = Last Transaction Amount (in minutes)
        3 = Amount Saved (their time balance in their account)
        4 = Max Withdrawal (the max a user can withdraw in a day)
        5 = Max Stored Amount (Maximum time allowed to be stored)

    Byte Fields (in K bytes)
    ------------------------
        6 = Last Deposit Date
        7 = Last Withdrawal Date
        8 = Last Transaction Amount (in K bytes)
        9 = Amount Saved (their K byte balance in their account)
        10 = Max Withdrawal (the max a user can withdraw in a day)
        11 = Max Stored Amount (Maximum K bytes allowed to be stored)
hint-statement-webrequest=
    @1 = An string expression stating the url to get data from.
    @2 = An string expression stating the file to save the data to.

hint-function-len=
    ### Returns
    Returns the length of the string @1
hint-function-lower=
    ### Returns
    Returns the string @1 converted to lower case
hint-function-upper=
    ### Returns
    Returns the string @1 converted to upper case
hint-function-mid=
    ### Returns
    Returns a substring of @1 starting at position @2 and @3 characters long
hint-function-left=
    ### Returns
    Returns the leftmost @2 characters of @1
hint-function-right=
    ### Returns
    Returns the rightmost @2 characters of @1
hint-function-space=
    ### Returns
    Returns a string of @1 spaces
hint-function-ferr=todo
hint-function-chr=
    ### Returns
    Returns a single character long string of the character represented by ASCII code var (0-255)
hint-function-asc=
    ### Returns
    Returns the ASCII value of the first character in @1
hint-function-instr=Returns the position of @2 in @1 `(1-LEN(@1))` or `0` if @2 not in @1
hint-function-abort=Returns a flag indicating whether or not the user aborted the display of data via ^K / ^X or answering no to a MORE? prompt
hint-function-ltrim=Returns a string of @1 with the first character of @2 trimmed from the left
hint-function-rtrim=Returns a string of @1 with the first character of @2 trimmed from the right
hint-function-trim=Returns a string of @1 with the first character of @2 trimmed from both ends
hint-function-random=Returns a random number between 0 and @2 inclusive
hint-function-date=Returns todays date
hint-function-time=Returns the current time
hint-function-u_name=Returns the current users name
hint-function-u_ldate=Returns the current users last date on the system
hint-function-u_ltime=Returns the current users last time on the system
hint-function-u_ldir=Returns the current users last directory scan date
hint-function-u_logons=Returns the current users number of times logged on
hint-function-u_ful=Returns the current users number of files uploaded
hint-function-u_fdl=Returns the current users number of files downloaded
hint-function-u_bdlday=Returns the current users number of bytes downloaded today
hint-function-u_timeon=Returns the current users time online today in minutes
hint-function-u_bdl=Returns the current users number of bytes downloaded
hint-function-u_bul=Returns the current users number of bytes downloaded
hint-function-year=Returns the year (1900-2079) of @1
hint-function-month=Returns the month of the year (1-12) of @1
hint-function-day=Returns the day of the month (1-31) of @1
hint-function-dow=Returns the day of the week (0 = Sunday, 6 = Saturday) that @1 fell on
hint-function-hour=Returns the hour of the day (0-23) of @1
hint-function-min=Returns the minute of the hour (0-59) of @1
hint-function-sec=Returns the second of the minute (0-59) of @1
hint-function-timeap=Returns a string representing the time @1 in civilian format (XX:XX:XX AM)
hint-function-ver=Returns the version number of PCBoard that is running
hint-function-nochar=Returns the current language no character
hint-function-yeschar=Returns the current language yes character
hint-function-stripatx=Returns a string of @1 with all @X codes removed
hint-function-replace=Returns a string of @1 with all occurences of the first character of @2 replaced by the first character of @3
hint-function-strip=Returns a string of @1 with all occurrences of the first character of @2 removed
hint-function-inkey=Returns the next keypress as a single character long string, or a string with the name of the function or cursor control key
hint-function-tostring=Converts an expression to a `STRING` type
hint-function-mask_pwd=Returns a valid character mask for input statements of passwords
hint-function-mask_alpha=Returns a valid character mask for input statements of A through Z and a through z
hint-function-mask_num=Returns a valid character mask for input statements of 0 through 9
hint-function-mask_alnum=Returns a valid character mask for input statements of A through Z, a through z, and 0 through 9
hint-function-mask_file=Returns a valid character mask for input statements of file names
hint-function-mask_path=Returns a valid character mask for input statements of path names
hint-function-mask_ascii=Returns a valid character mask for input statements of space (“ ”) through tilde (“~”)
hint-function-curconf=Returns the current conference number
hint-function-pcbdat=Returns a string with the path and file name of PCBOARD.DAT
hint-function-ppepath=Returns a string with the path (no file name) of the currently executing PPE file
hint-function-valdate=Returns `TRUE` if @1 is in a valid date format
hint-function-valtime=Returns `TRUE` if @1 is in a valid time format
hint-function-u_msgrd=Returns the number of messages the user has read
hint-function-u_msgwr=Returns the number of messages the user has written
hint-function-pcbnode=Returns the node number
hint-function-readline=Read and return line number @2 from file @1
hint-function-sysopsec=Returns the SysOp security defined in PCBOARD.DAT
hint-function-onlocal=Returns `TRUE` if the user is on locally
hint-function-un_stat=Returns a nodes status from USERNET.XXX after a RdUnet statement
hint-function-un_name=Returns a nodes user name from USERNET.XXX after a RdUnet statement
hint-function-un_city=Returns a nodes city from USERNET.XXX after a RdUnet statement
hint-function-un_oper=Returns a nodes operation text from USERNET.XXX after a RdUnet statement
hint-function-cursec=Returns the users current security level
hint-function-gettoken=
    Returns the next string token from a prior call to `Tokenize` (Same as the `GETTOKEN` statement but can be used in an expression without prior assignement to a variable)
hint-function-minleft=Returns the current callers minutes left to use online
hint-function-minon=Returns the current callers minutes online so far this session
hint-function-getenv=Returns the value of the environment variable named by @1
hint-function-callid=Returns the caller ID string
hint-function-regal=Returns the value of the AL register after a DoIntr statement
hint-function-regah=Returns the value of the AH register after a DoIntr statement
hint-function-regbl=Returns the value of the BL register after a DoIntr statement
hint-function-regbh=Returns the value of the BH register after a DoIntr statement
hint-function-regcl=Returns the value of the CL register after a DoIntr statement
hint-function-regch=Returns the value of the CH register after a DoIntr statement
hint-function-regdl=Returns the value of the DL register after a DoIntr statement
hint-function-regdh=Returns the value of the DH register after a DoIntr statement
hint-function-regax=Returns the value of the AX register after a DoIntr statement
hint-function-regbx=Returns the value of the BX register after a DoIntr statement
hint-function-regcx=Returns the value of the CX register after a DoIntr statement
hint-function-regdx=Returns the value of the DX register after a DoIntr statement
hint-function-regsi=Returns the value of the SI register after a DoIntr statement
hint-function-regdi=Returns the value of the DI register after a DoIntr statement
hint-function-regf=Returns the value of the flags register after a DoIntr statement
hint-function-regcf=Returns the value of the carry flag register after a DoIntr statement
hint-function-regds=Returns the value of the DS register after a DoIntr statement
hint-function-reges=Returns the value of the ES register after a DoIntr statement
hint-function-b2w=
    Returns a word built from two byte sized values by the formula:
    `(@1*0100h+@2)`
hint-function-peekb=Returns a byte value (0-255) located at memory address @1 (PEEK is a synonym)
hint-function-peekw=Returns a word value (0-65535) located at memory address @1
hint-function-mkaddr=
    Returns a segment:offset address as a long integer built from two word sized values by the formula:
    `@1*00010000h+@2`
hint-function-exist=Returns a boolean `TRUE` value if the file @1 exists
hint-function-i2s=Returns a string representing the integer value @1 converted to base @2
hint-function-s2i=Returns an integer representing the string @1 converted from base @2
hint-function-carrier=Returns the carrier speed as reported by the modem to PCBoard
hint-function-tokenstr=Returns a previously tokenized string reconstructed with semi-colons separating the component tokens
hint-function-cdon=Returns `TRUE` if the carrier detect signal is on, `FALSE`
hint-function-langext=Returns the file extension for the users language selection
hint-function-ansion=Returns `TRUE` if the user is on locally
hint-function-valcc=Returns `TRUE` if @1 is a valid credit card number
hint-function-fmtcc=Returns a formatted credit card number based on @1
hint-function-cctype=Returns the issuer of credit card number @1
hint-function-getx=Returns the current column (X position) of the cursor on the display
hint-function-gety=Returns the current row (Y position) of the cursor on the display
hint-function-band=Returns the bitwise and of two integer expressions
hint-function-bor=Returns the bitwise or of two integer expressions
hint-function-bxor=Returns the bitwise exclusive-or of two integer expressions
hint-function-bnot=Returns the bitwise complement (all bits inverted) of an integer expression
hint-function-u_pwdhist=Returns the specified password from the password history Valid values for @1 are 1 through 3
hint-function-u_pwdlc=Returns the date of the last password change
hint-function-u_pwdtc=Returns the number of times the password has been changed
hint-function-u_stat=
    Returns a statistic about the user that is tracked by PCBoard
    Valid values for @1 are 1 through 15
    |||
    | --- | --- |
    | 1 | first date the user called the system |
    | 2 | number of SysOp pages the user has requested |
    | 3 | number of group chats the user has participated in |
    | 4 | number of comments the user has left |
    | 5 | number of 300 bps connects |
    | 6 | number of 1200 bps connects |
    | 7 | bumber of 2400 bps connects |
    | 8 | number of 9600 bps connects |
    | 9 | number of 14400 bps connects |
    | 10 | number of security violations |
    | 11 | number of “not registered in conference” warnings |
    | 12 | number of times the users download limit has been reached |
    | 13 | number of “file not found” warnings |
    | 14 | number of password errors the user has had |
    | 15 | number of verify errors the user has had |

hint-function-defcolor=Returns system default color.
hint-function-abs=Returns the absolute value of @1
hint-function-grafmode=
    Returns a character indicating the users graphics status

    | Value | Meaning |
    | :--- | :--- |
    | R | RIPscrip supported |
    | G | ANSI graphics (color and positioning) supported |
    | A | ANSI positioning (no color) supported |
    | N | No graphics (RIP or ANSI) supported |

hint-function-psa=
    Returns the value of the specified PSA variable

    @1 = The PSA variable to retrieve

    ### PSA
    | | |
    | :--- | :--- |
    | 1 | Alias Support Enabled |
    | 2 | Verify Support Enabled |
    | 3 | Address Support Enabled |
    | 4 | Password Support Enabled |
    | 5 | Statistics Support Enabled |
    | 6 | Notes Support Enabled |
hint-function-fileinf=
    Returns information about the file specified by @1
    
    @1 = The file to get information about

    @2 = The information to return

    ### Valid Options
    | | |
    | :--- | :--- |
    | 1 | Return TRUE if file exists |
    | 2 | Return file date stamp |
    | 3 | Return file time stamp |
    | 4 | Return file size |
    | 5 | Return file attributes 1) |
    | 6 | Return file drive |
    | 7 | Return file path |
    | 8 | Return file base name |
    | 9 | Return file extension |

    | 1) File Attribute | |
    | :--- | :--- |
    | 01h | Read Only |
    | 02h | Hidden |
    | 04h | System |
    | 20h | Archive |
hint-function-ppename=Returns the name of the currently executing PPE file minus the path and extension
hint-function-mkdate=Returns a date with the year specified by year (1900-2079), month specified by month (1-12), and day specified by day (1-31).
hint-function-curcolor=Returns the current color (0-255) in use by the ANSI driver
hint-function-kinkey=Returns the next keypress from the BBS keyboard as a single character long string, or a string with the name of the function or cursor control key
hint-function-minkey=Returns the next keypress from the remote caller as a single character long string, or a string with the name of the function or cursor control key
hint-function-maxnode=Returns the maximum node possible with the current software (ie, /2 would return 2, /10 would return 10, etc)
hint-function-slpath=Returns the path, as specified in PCBSetup, to the login security files
hint-function-helppath=Returns the path, as specified in PCBSetup, to the help files
hint-function-temppath=Returns the path, as specified in PCBSetup, to the temporary work directory
hint-function-modem=Returns the modem connect string as reported by the modem to PCBoard
hint-function-loggedon=Returns `TRUE` if the user has already logged on to the BBS, `FALSE` otherwise
hint-function-callnum=Returns the caller number of the current user.
hint-function-mgetbyte=Returns the value of the next byte from the modem (0-255) or -1 if there are no bytes waiting for input
hint-function-tokcount=Returns the number of tokens available via the GetToken statement and/or function
hint-function-u_recnum=Returns the user record number (0-65535) for user name user or -1 if user is not registered on this system.
hint-function-u_inconf=Returns `TRUE` if user record number @1 is registered in conference @2
hint-function-peekdw=Returns a signed integer value (-2147483648 - +2147483647) located at memory address “var”
hint-function-dbglevel=Returns the debug level in effect
hint-function-scrtext=
    ### Returns
    Returns a string of @3 characters from the screen at @1, @2.
    If @3 is `TRUE` then the string will be returned with all @ codes intact.
hint-function-showstat=Returns `TRUE` if writing to the display is active, `FALSE` if writing to the display is disabled
hint-function-pagestat=Returns `TRUE` if the user has paged the SysOp (or PageOn has been issued), `FALSE` otherwise (or PageOff has been issued)
hint-function-replacestr=
    It functions just like the Replace function except that a complete sub-string may be specified for both search and replace
hint-function-stripstr=
    Functions just like the Strip function except that a complete sub-string may be specified for search
hint-function-tobigstr=Converts an expression to a `BIGSTR` type
hint-function-toboolean=Converts an expression to a `BOOLEAN` type
hint-function-tobyte=Converts an expression to a `BYTE` type
hint-function-todate=Converts an expression to a `DATE` type
hint-function-todreal=Converts an expression to a `DREAL` type
hint-function-toedate=Converts an expression to a `EDATE` type
hint-function-tointeger=Converts an expression to a `INTEGER` type
hint-function-tomoney=Converts an expression to a `MONEY` type
hint-function-toreal=Converts an expression to a `REAL` type
hint-function-tosbyte=Converts an expression to a `SBYTE` type
hint-function-tosword=Converts an expression to a `SWORD` type
hint-function-totime=Converts an expression to a `TIME` type
hint-function-tounsigned=Converts an expression to a `UNSIGNED` type
hint-function-toword=Converts an expression to a `WORD` type
hint-function-mixed=Converts a string to mixed (or proper name) case
hint-function-alias=Return the users current ALIAS setting (TRUE = alias use on, FALSE = alias use off)
hint-function-confreg=Returns TRUE if users registered flag is set, FALSE otherwise
hint-function-confexp=Returns TRUE if users expired flag is set, FALSE otherwise
hint-function-confsel=Returns TRUE if user has selected the conference, FALSE otherwise
hint-function-confsys=Returns TRUE if user has conference SysOp access, FALSE otherwise
hint-function-confmw=Returns TRUE if user has mail waiting in conference confnum, FALSE otherwise
hint-function-lprinted=Return the number of lines printed on the display
hint-function-isnonstop=Return whether or not the display is currently in non-stop mode (ie, did the user type NS as part of their command line)
hint-function-errcorrect=Returns TRUE if a session is determined to be error corrected (or FALSE for non-error corrected sessions).
hint-function-confalias=Return TRUE if the current conference is configured to allow aliases
hint-function-useralias=Return TRUE if the current user is allowed to use an alias
hint-function-curuser=
    Determine what users information, if any, is available via the user variables. It takes no arguments and returns one of the following values:
    NO_USER (-1) - User variables are currently undefined  
    CUR_USER (0) - User variables are for the current user  
    Other        - The record number of an alternate user for whom user  variables are defined 
hint-function-u_lmr=function to return the number of the last message read for the specified conference.
hint-function-chatstat=Return the current users chat availability status (TRUE means available, FALSE means unavailable).
hint-function-defans=Returns the last default answer passed to an Input statement. For example, this allows a PPE to determine what the default answer would have been had a PCBTEXT prompt not been replaced with a PPE.
hint-function-lastans=function to return the last answer accepted by an Input statement.
hint-function-meganum=Converts a decimal number (from 0 to 1295) to a hexa-tri-decimal number, or meganum.
hint-function-evttimeadj=Detects if the users time has been adjusted for an upcoming event. This is useful to detect if a users time left can be increased with the AdjTime statement.
hint-function-isbitset=
    Check the status of a specified bit in a variable.
    This function is primarily intended to be used with BIGSTR variables which can be up to 2048 bytes long.
    However, it will work with other data types (and expressions) as well if desired.
hint-function-fmtreal=
    Formats REAL/DREAL values for display purposes.
    ### Parameters
    realExp	A REAL/DREAL floating point expression
    fieldWidth	The minimum number of characters to display
    decimalPlaces	The number of characters to display to the right of the decimal point

hint-function-flagcnt=Return the number of files flagged for download.
hint-function-kbdbufsize=Return the number of key presses pending in the KbdString buffer
hint-function-pplbufsize=Returns the number of key presses pending in the KbdStuff buffer.
hint-function-kbdfilused=todo
hint-function-lomsgnum=Returns the low message number for the current conference.
hint-function-himsgnum=Returns the high message number for the current conference.
hint-function-drivespace=Return Val: Amount of divespace left of drive drivespec. 
hint-function-outbytes=Returns the number of bytes waiting in the modems output buffer Not available in local mode.
hint-function-hiconfnum=Returns the highest conference number available on the board
hint-function-inbytes=Returns number of bytes waiting in the modem input buffer Not available in local mode.
hint-function-crc32=Returns an UNSIGNED value of the CRC of a file or string.
hint-function-pcbmac=
    Returns a BIGSTR containing the expanded text of a PCB MACRO

    ### PCB MACROS not supported
    @automore@ @beep@ @clreol@ @cls@ @delay@ @more@ @pause@ @poff@ @pon@ @pos@ @qoff@ @qon@ @wait@ @who@ @x@
hint-function-actmsgnum=
    ### Returns
    Returns number of active messages in current conference

    ### Example
    ```
    integer i
    println "There are ",ACTMSGNUM()," messages in conference ",CURCONF()
    ```
hint-function-stackleft=Returns the number of bytes left on the system stack.
hint-function-stackerr=Returns a boolean value which indicates a stack error has occured if TRUE.
hint-function-dgetalias=return the current alias
hint-function-dbof=return the begin of file statustodo
hint-function-dchanged=return the changed flag
hint-function-ddecimals=return decimals of named field
hint-function-ddeleted=return the deleted flag
hint-function-deof=return the end of file status
hint-function-derr=return error flag for channel
hint-function-dfields=return count of fields
hint-function-dlength=return length of named field
hint-function-dname=return name of numbered field
hint-function-dreccount=return the number of recordstodo
hint-function-drecno=return the current record number
hint-function-dtype=return type of named field
hint-function-fnext=Returns an available file channel. -1 when none are available.
hint-function-dnext=todo
hint-function-toddate=Converts a date to a string in the format MM/DD/YYYY
hint-function-dcloseall=close all DBF files
hint-function-dopen=open DBF file
hint-function-dclose=close DBF file
hint-function-dsetalias=set DBF alias
hint-function-dpack=pack DBF file
hint-function-dlockf=lock DBF file
hint-function-dlock=lock DBF file
hint-function-dlockr=lock a recordtodo
hint-function-dunlock=unlock any current locks
hint-function-dnopen=open NDX file
hint-function-dnclose=close NDX file
hint-function-dncloseall=close all NDX files
hint-function-dnew=start a new record
hint-function-dadd=add the new record
hint-function-dappend=append a blank record
hint-function-dtop=go to top record
hint-function-dgo=go to specific record
hint-function-dbottom=go to bottom record
hint-function-dskip=skip +/- a number of records
hint-function-dblank=blank the record
hint-function-ddelete=delete the record
hint-function-drecall=recall the record
hint-function-dtag=select a tag
hint-function-dseek=
    returns error status ( 0|1 )
    or seek success (0 = Error
    1 = success, 2 = following record
    3 = end of file )
hint-function-dfblank=blank a named field
hint-function-dget=get a value from a named field
hint-function-dput=put a value to a named field
hint-function-dfcopy=copy a field to a field
hint-function-dselect=returns channel associated with alias
hint-function-dchkstat=todo
hint-function-pcbaccount=
    Returns what PCBoard will charge a user for a certain activity. These are values the SysOp assigns in PCBsetup when accounting is configures and enabled.
    Valid values for the field parameter are 0-14. Use of the corresponding constants is encouraged. (see the Accounting section)

    { accounting_constants }

hint-function-pcbaccstat=
    Returns value in status field
    This function can and should be used in conjunction with the ACC_??? constants as the field parameter. Valid values for field are 0-3. 

 | Field | dec | Field Description |
 | :--- |  :--- | :--- |
 | `ACC_STAT`   | `0`  | Returns status of the “Enable Accounting” switch in the PWRD file.  |
 | `ACC_TIME`   | `1`  | The amount of ADDITIONAL units to charge |
 | `ACC_MSGR`   | `2`  | The amount to charge in ADDITION for each message read in the current conference. |
 | `ACC_MSGW`   | `3`  | The amount to charge in ADDITION for each message entered in the current conference. |

hint-function-derrmsg=returns last DBase error text
hint-function-account=Returns amount of credits charged for services corresponding to the field parameter.

hint-function-scanmsghdr=
    Returns the first message number in the message base which matches the search criteria.

    { message_header_constants }
hint-function-checkrip=Returns `TRUE` if the terminal has RIP.
hint-function-ripver=
    Returns a string containing the RIP version. If no RIP is available "0" is returned.
hint-function-qwklimits=todo
hint-function-findfirst=
    Find the first occurence of filespec in a directory. Used in conjunction with FindNext to get a directory listing.

    ### Parameters
    @1 = A string expression with the path and file name to access information about.
    Quite often this expression involves a DOS wildcard (e.g., *.*, *.BAT, etc.)

    ### Returns
    The first filename matching the filename criteria.

    ### Remarks
    This function is designed to help locate files matching a specific
    criteria.  For example, you may want to delete all files matching *.BAK
    in the current directory.  This can be done easily because
    FINDFIRST() locates the first match, while FINDNEXT() locates
    additional matches.

    It should be noted that only the filenames are returned.  If you need
    additional information such as date, time, or size of the file, use
    the FILEINF() function.

hint-function-findnext=
    This function determines if there are any more files matching a specified pattern.

    ### Returns
    The next filename matching the filename criteria or an
    empty string if there are no more matching files.

    ### Remarks
    This function is designed to continue where the FINDFIRST() function
    leaves off because it locates any additional files matching the pattern
    last searched for.  There are no more matching files when the return
    value is null or an empty string. Because you do not know how many
    matching files there are, a WHILE loop is usually involved in gathering
    all of the filenames.

    It should be noted that only the filenames are returned.  If you need
    additional information such as date, time, or size of the file, use the
    FILEINF() function.
hint-function-uselmrs=
    ### Parameters
    @1 = Instructs PCBoard NOT to load an alternate users LMRS
    when a GETALTUSER is executed.

    ### Remarks
    This statment can save a significant amount of memory when a GETALTUSER
    is executed at a later time. When GETALTUSER is executed, it loads
    the users LMR's by default. If you have a significant number of conferences
    on your system, this can require a great deal of memory. Since PCBoard
    is so rich with features, it can take most if not all available
    conventional memory, leaving PPEs out to dry. If an alternate users
    LMRs are not needed by the PPE application, then you can use this
    statement to tell PCBoard not to load the LMR data.

    See also the FUNCTION USELMRS, this will return the current status
    of USELMRS. Eg, if the funtion USELMRS returns TRUE, then a GETALTUSEr
    will load LMRS, if it returns FALSE, LMRS will not be loaded.
hint-function-confinfo=
    This statement can be used to access a field in the conference
    configuration.

    ### Parameters
    @1 = The conference number to get information about
    @2 = Conference field to modify. (See note)

    { conference_access_constants }
    
hint-function-tinkey=
    ### Parameters
    @1 = Number of clock ticks to wait for input.

    ### Returns
    Input entered by user

    ### Remarks
    Y1 is the number of clock ticks you wish `TINEKY` to wait for input
    before timing out. 1 second = 18 ticks (approx)
    
    A tick value of 0 will cause `TINKEY` to wait indefinatly for input with
    a maximum timout time of about 4 hours. Carrier loss will also terminate
    `TINKEY`.
hint-function-cwd=
    ### Returns
    The current working directory
hint-function-instrr=
    Returns the right most position of @2 in @1 `(1-LEN(@1))` or `0` if @2 not in @1
hint-function-fdordaka=todo
hint-function-fdordorg=todo
hint-function-fdordarea=todo
hint-function-fdoqrd=todo
hint-function-getdrive=
    ### Returns
    The current drive letter

    ### Remarks
    Drive numbers correspond to drive letters in the following way
    A: = 0
    B: = 1
    C: = 2
    …
hint-function-setdrive=todo
hint-function-bs2i=
    Converts a 4 byte bsreal to a PPL integer.

    ### Parameters
    @1 is a BIGSTR type since BIGSTR types can contain
    binary data. For this function, PPL will convert the first
    4 bytes of the BIGSTR into an INTEGER variable and retun
    it.

    ### Returns
    Returns a converted 4 byte bsreal in the form of a 4 byte integer.
hint-function-bd2i=Converts an 8 byte bdreal to a PPL integer.
hint-function-i2bs=Converts a 4 byte PPL INTEGER into a 4 byte bsreal and stores it in a BIGSTR.
hint-function-i2bd=Converts a 4 byte PPL INTEGER into an 8 byte bdreal and stores it.
hint-function-ftell=
        `FTELL` returns the current file pointer offset for the specified
        file channel. If the channel is not open, it will return 0.
        Otherwise it will return the current position in the open file.

        ### Parameters
        @1 - The file channel to process
        
        ### Returns
        4 byte signed integer containing the file pointer offset
        of the file attached to channel.
hint-function-os=
        ### Returns
        An Integer indicating which operating system/pcboard version
        the PPE is currently running under.
        1=DOS, 2 = OS2, 0 = unknown.
hint-function-short_desc=
    ### Returns
    TRUE if the user has short file descriptions set to on else it returns FALSE.
hint-function-getbankbal=
    ### Parameters
    @1 The field to get.

    ### Returns
    Returns the value of a specified field.

    ### Fields

    Time Fields (in minutes)
    ------------------------
        0 = Last Deposit Date
        1 = Last Withdrawal Date
        2 = Last Transaction Amount (in minutes)
        3 = Amount Saved (their time balance in their account)
        4 = Max Withdrawal (the max a user can withdraw in a day)
        5 = Max Stored Amount (Maximum time allowed to be stored)

    Byte Fields (in K bytes)
    ------------------------
        6 = Last Deposit Date
        7 = Last Withdrawal Date
        8 = Last Transaction Amount (in K bytes)
        9 = Amount Saved (their K byte balance in their account)
        10 = Max Withdrawal (the max a user can withdraw in a day)
        11 = Max Stored Amount (Maximum K bytes allowed to be stored)

hint-function-getmsghdr=
    ### Parameters
    @1 = conference number of the message base
    @2 = A double expression stating the message number of the message to get the message header value.
    @3 = The field to get.

    ### Returns
    Returns the value of the specified field.

    { message_header_constants }
hint-function-setmsghdr=
    ### Parameters
    @1 = An integer expression stating the conference number of the message base.
    @2 = A double expression stating the message number of the message to set the message header value.
    @3 = An integer expression between 1 and 5 representing the field to get.
    @4 = A string expression containing the data to insert into the specified field.

    ### Fields
    1 = 'To' field
    2 = 'From' field
    3 = 'Subject' field
    4 = 'Password' field
    5 = 'Echo' Flag

    ### Returns
    Returns the value of the message number.  If the message will
    fit in the same place as the original then it will be the same.
    If modefied header change will not fit in the original message
    header then it will insert the message to the end of the message
    base.
hint-function-newconfinfo=todo
hint-function-areaid=Generates a tuple conference/area to identify a message base.
hint-function-webrequest=Gets data from a web server and returns it as a string.
    ### Parameters
    @1 = An string expression stating the url to get data from.

    ### Returns
    The web request value as STRING.
hint-function-len_dim=
    @1 = The array to get the length of
    @2 = The dimension to get the length of
    ### Returns
        Returns the length of the array @1 on dimension @2

hint-const-true=BOOLEAN `TRUE` value
hint-const-false=BOOLEAN `FALSE` value
hint-const-stk_limit=This constant was added so the PPL programmer could determine how close they are getting to the stack limit when using recursion.
hint-const-attach_lim_p=Public attach bytes limit
hint-const-attach_lim_u=Personal attach bytes limit
hint-const-f_net=todo
hint-const-cmaxmsgs=Max Messages per conference
hint-const-maxmsgs=Max messages per qwk packet
hint-const-cur_user=Parameter passed to `CURUSER()`/Return by `GetUser` - User variables are for the current user
hint-const-no_user=Return by `GetUser` - variables are currently undefined
hint-const-acc_cur_bal=Return the up to the minute user balance.todo
hint-const-acc_stat=
    Returns status of the "Enable Accounting" switch in the PWRD file.  

    0=Accounting disabled (N)
    1=Tracking (T), and 2=Enabled (Y).
hint-const-acc_time=The amount of ADDITIONAL units to charge per minute while in the current conference.
hint-const-acc_msgread=The amount to charge in ADDITION for each message read in the current conference.
hint-const-acc_msgwrite=The amount to charge in ADDITION for each message entered in the current conference.
hint-const-defs=Parameter passed to various statements for default values
hint-const-bell=Parameter passed to ``DISPTEXT`` statement (sound a bell when prompt displayed)
hint-const-logit=Parameter passed to `DISPTEXT` statement (log text to callers log)
hint-const-logitleft=Parameter passed to `DISPTEXT` statement (log text to callers log, forcing left justification)
hint-const-auto=Parameter passed to ``INPUTSTR`` and ``PROMPTSTR`` statements (automatically press enter after 10 seconds of no user input)
hint-const-echodots=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (echo dots instead of user input)
hint-const-eraseline=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (erase the current line when user presses enter)
hint-const-fieldlen=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (displays parenthesis to show input field width if ANSI enabled)
hint-const-guide=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (displays parenthesis above current line if FIELDLEN used and ANSI not enabled
hint-const-highascii=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (allow high ascii characters, regardless of current valid character set, if disable high ascii filter set to yes)
hint-const-lfafter=Parameter passed to `INPUTSTR`, `PROMPTSTR` and `DISPTEXT` statements (send an extra line feed after user presses enter)
hint-const-lfbefore=Parameter passed to `INPUTSTR`, `PROMPTSTR` and `DISPTEXT` statements (send an extra line feed before prompt display)
hint-const-newline=Parameter passed to `INPUTSTR`, `PROMPTSTR` and `DISPTEXT` statements (send a line feed after user presses enter)
hint-const-noclear=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (don't clear field at first keypress regardless of ANSI)
hint-const-stacked=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (allow semi-colons and spaces in addition to valid character set passed)
hint-const-upcase=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (force user input to upper case)
hint-const-wordwrap=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (if user hits end of line, save the text at the end of the line for future use)
hint-const-yesno=Parameter passed to `INPUTSTR` and `PROMPTSTR` statements (Only allow international yes/no responses)
hint-const-newbalance=Credits Given to a new user account
hint-const-chrg_call=Credits charged for a call
hint-const-chrg_time=Credits charged for time used (in minutes)
hint-const-chrg_peaktime=Credits charged for peak time used
hint-const-chrg_chat=Credits charged for chat session
hint-const-chrg_msgread=Credits charged for reading a message
hint-const-chrg_msgcap=Credits charged for capturing a message
hint-const-chrg_msgwrite=Credits charged for writing a message
hint-const-chrg_msgechoed=Credits charged for writing an echoed message
hint-const-chrg_msgprivate=Credits charged for writing a private message
hint-const-chrg_downfile=Credits charged for downloading a file
hint-const-chrg_downbytes=Credits charged for downloading bytes
hint-const-pay_upfile=Credits given for uploading a file
hint-const-pay_upbytes=Credits given for uploading bytes
hint-const-warnlevel=Credit threshold for low credit warning
hint-const-crc_file=
    These constants were added to avoid confusion when telling the function `CRC32` what it is taking the CRC of.
    CRCFILE tells `CRC32` to calculate the CRC of the file contained within the string argument.
    CRCFILE has a value of 1 (`TRUE`)
hint-const-crc_str=
    These constants were added to avoid confusion when telling the function `CRC32` what it is taking the CRC of.
    CRCSTR tells `CRC32` to calculate the CRC of the string argument itself.
    CRCSTR has a value of 0 (`FALSE`)
hint-const-start_bal=Users starting balance.
hint-const-start_session=Users starting balance for this session.
hint-const-deb_call=Debit for this call
hint-const-deb_time=Debit for time on
hint-const-deb_msgread=Debit for reading message
hint-const-deb_msgcap=Debit for capturing a messagetodo
hint-const-deb_msgwrite=Debit for writing a message
hint-const-deb_msgechoed=Debit for echoed message
hint-const-deb_msgprivate=Debit for writing private message
hint-const-deb_downfile=Debit for downloading a filetodo
hint-const-deb_downbytes=Debit for downloading bytes
hint-const-deb_chat=Debit for chat
hint-const-deb_tpu=Debit for TPU
hint-const-deb_special=Debit special
hint-const-cred_upfile=Credit for uploading a file
hint-const-cred_upbytes=Credit for uploading bytes
hint-const-cred_special=Credit special
hint-const-sec_drop=Security level to drop to at 0 credits
hint-const-f_exp=Expired subscription access allowed flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-f_mw=Mail waiting flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-f_reg=Registered access allowed flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-f_sel=Conference selected flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-f_sys=Conference SysOp access flag for `CONFFLAG` and `CONFUNFLAG`
hint-const-fcl=Value passed to `STARTDISP` to force line counting display
hint-const-fns=Value passed to `STARTDISP` to force non-stop display
hint-const-nc=Value passed to `STARTDISP` to not change display mode
hint-const-graph=Parameter passed to `DISPFILE` statement to search for graphics specific files
hint-const-sec=Parameter passed to `DISPFILE` statement to search for security specific files
hint-const-lang=Parameter passed to `DISPFILE` statement to search for language specific files
hint-const-hdr_active=Message active flag field
hint-const-hdr_blocks=Number of 128 byte blocks in message
hint-const-hdr_date=Date message was written
hint-const-hdr_echo=Echoed message flag
hint-const-hdr_from=Who the message is from
hint-const-hdr_msgnum=Message number
hint-const-hdr_msgref=Reference messagetodo
hint-const-hdr_pwd=Message password
hint-const-hdr_reply=Message reply flag
hint-const-hdr_rplydate=Reply message date
hint-const-hdr_rplytime=Reply message time
hint-const-hdr_status=Message status
hint-const-hdr_subj=Message subject
hint-const-hdr_time=Message time
hint-const-hdr_to=Message to field
hint-const-o_rd=Parameter passed to `FCREATE/FOPEN/FAPPEND` to open a file in read only mode
hint-const-o_rw=Parameter passed to `FCREATE/FOPEN/FAPPEND` to open a file in read and write mode
hint-const-o_wr=Parameter passed to `FCREATE/FOPEN/FAPPEND` to open a file in write only mode
hint-const-seek_cur=for the current file pointer location
hint-const-seek_end=for the end of the file
hint-const-seek_set=for the beginning of the file
hint-const-s_db=Parameter passed to `FCREATE/FOPEN/FAPPEND` to deny read and write (both) access from other processes
hint-const-s_dn=Parameter passed to `FCREATE/FOPEN/FAPPEND` to allow read and write (deny none) access from other processes
hint-const-s_dr=Parameter passed to `FCREATE/FOPEN/FAPPEND` to deny read access from other processes
hint-const-s_dw=Parameter passed to `FCREATE/FOPEN/FAPPEND` to deny write access from other processes


# Tables 

message_header_constants= 
 ### Message Header Field Access Constants
 
 | Field | hex | dec | Field Description |
 | :--- | :--- | :--- | :--- |
 | `HDR_ACTIVE`   | `0x0E` | `14`  | Message active flag field |
 | `HDR_BLOCKS`   | `0x04` | `4`   | Number of 128 byte blocks in message |
 | `HDR_DATE`     | `0x05` | `5`   | Date message was written |
 | `HDR_ECHO`     | `0x0F` | `15`  | Echoed message flag |
 | `HDR_FROM`     | `0x0B` | `11`  | Who the message is from |
 | `HDR_MSGNUM`   | `0x02` | `2`   | Message number | 
 | `HDR_MSGREF`   | `0x03` | `3`   | Reference message |
 | `HDR_PWD`      | `0x0D` | `13`  | Message password |
 | `HDR_REPLY`    | `0x0A` | `10`  | Message reply flag |
 | `HDR_RPLYDATE` | `0x08` | `8`   | Reply message date |
 | `HDR_RPLYTIME` | `0x09` | `9`   | Reply message time |
 | `HDR_STATUS`   | `0x01` | `1`   | Message status |
 | `HDR_SUBJ`     | `0x0C` | `12`  | Message subject |
 | `HDR_TIME`     | `0x06` | `6`   | Message time |
 | `HDR_TO`       | `0x07` | `7`   | Receiver of the message |
 
conference_access_constants=
 ### Fields
 | Value | Purpose | Type |
 | ---: | :--- | :--- |
 |1| Conference Name | STRING |
 |2| Public Conference|BOOLEAN |
 |3| Auto Rejoin|BOOLEAN |
 |4| View Other Users|BOOLEAN |
 |5| Make Uploads Private|BOOLEAN |
 |6| Make All Messages Private|BOOLEAN |
 |7| Echo Mail in Conf|BOOLEAN |
 |8| Required Security if public|INTEGER |
 |9| Additional Conference Security|INTEGER |
 |10| Additional Conference Time| INTEGER |
 |11| Number of Message Blocks| INTEGER |
 |12| Name/Loc of MSGS File| STRING |
 |13| Name/Loc of Users's Menu| STRING |
 |14| Name/Loc of Sysops Menu| STRING |
 |15| Name/Loc of NEWS file.| STRING |
 |16| Public Upload Sort| INTEGER |
 |17| Name/Loc upload DIR file| STRING |
 |18| Location of Public Uploads| STRING |
 |19| Private Upload Sort| INTEGER |
 |20| Name/Loc Private Upload DIR file| STRING |
 |21| Location of private uploads| STRING |
 |22| Doors Menu| STRING |
 |23| Doors File| STRING |
 |24| Bulletin Menu| STRING |
 |25| Bulletin File| STRING |
 |26| Script Menu| STRING |
 |27| Script File| STRING |
 |28| Directories Menu| STRING |
 |29| Directories File| STRING |
 |30| Download Paths File| STRING |
 |31| Force Echo on All Messages| BOOLEAN |
 |32| Make Conference Read Only| BOOLEAN |
 |33| Disallow Private Messages| BOOLEAN |
 |34| Level to Request Return Receipt| INTEGER |
 |35| Place Origin Info In Messages| BOOLEAN |
 |36| Prompt For Route info| BOOLEAN |
 |37| Allow Aliases to be used| BOOLEAN |
 |38| Show INTRO in 'R A' Scan| BOOLEAN |
 |39| Level to Enter a Message| INTEGER |
 |40| Password to Join if Private;| STRING |
 |41| Name/Loc of Conf INTRO File| STRING |
 |42| Location for Attachments| STRING |
 |43| Auto-Register Flags| STRING |
 |44| Level to Save File Attachment| BYTE |
 |45| Carbon Copy List Limit| BYTE |
 |46| Conf-Specific CMD.LST File| STRING |
 |47| Maintain Old MSGS.NDX File| BOOLEAN |
 |48| Allow Internet (long) TO: Names| BOOLEAN |
 |49| Level to Enter Carbon List Msgs| BYTE |
 |50| Type of NetMail Conference| BYTE |
 |51| Last Message Exported| INTEGER |
 |52| Charge Per Minute| DREAL |
 |53| Charge per Message Read| DREAL |
 |54| Charge per Message Written| DREAL |

accounting_constants= 
 ### Accounting Information
 
 | Field | dec | Field Description |
 | :--- |  :--- | :--- |
 | `NEWBALANCE`      | `0`  | Credits Given to a new user account |
 | `CHRG_CALL`       | `1`  | Credits charged for a call |
 | `CHRG_TIME`       | `2`  | Credits charged for time used (in minutes) |
 | `CHRG_PEAKTIME`   | `3`  | Credits charged for peak time used |
 | `CHRG_CHAT`       | `4`  | Credits charged for chat session |
 | `CHRG_MSGREAD`    | `5`  | Credits charged for reading a message | 
 | `CHRG_MSGCAP`     | `6`  | Credits charged for capturing a message |
 | `CHRG_MSGWRITE`   | `7`  | Credits charged for writing a message |
 | `CHRG_MSGECHOED`  | `8`  | Credits charged for writing an echoed message |
 | `CHRG_MSGPRIVATE` | `9`  | Credits charged for writing a private message |
 | `CHRG_DOWNFILE`   | `10` | Credits charged for downloading a file |
 | `CHRG_DOWNBYTES`  | `11` | Credits charged for downloading bytes |
 | `PAY_UPFILE`      | `12` | Credits given for uploading a file |
 | `PAY_UPBYTES`     | `13` | Credits given for uploading bytes |
 | `WARNLEVEL`       | `14` | Credit threshold for low credit warning |
