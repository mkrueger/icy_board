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
hint-statement-confunflag=todo
hint-statement-dispfile=todo
hint-statement-input=todo
hint-statement-fcreate=todo
hint-statement-fopen=todo
hint-statement-fappend=todo
hint-statement-fclose=todo
hint-statement-fget=todo
hint-statement-fput=todo
hint-statement-fputln=todo
hint-statement-resetdisp=todo
hint-statement-startdisp=todo
hint-statement-fputpad=todo
hint-statement-hangup=todo
hint-statement-getuser=todo
hint-statement-putuser=todo
hint-statement-defcolor=todo
hint-statement-delete=todo
hint-statement-deluser=todo
hint-statement-adjtime=
    Adjust the users time up or down
    ### Syntax
    ```ADJTIME minutes```
    ```minutes``` An integer expression containing the number of minutes to adjust the time left by. > 0 will add time, < 0 will deduct time.
    The added/deducted time is only applied to the curent call.
hint-statement-log=todo
hint-statement-inputstr=todo
hint-statement-inputyn=todo
hint-statement-inputmoney=todo
hint-statement-inputint=todo
hint-statement-inputcc=todo
hint-statement-inputdate=todo
hint-statement-inputtime=todo
hint-statement-gosub=todo
hint-statement-return=todo
hint-statement-promptstr=todo
hint-statement-dtron=todo
hint-statement-dtroff=todo
hint-statement-cdchkon=todo
hint-statement-cdchkoff=todo
hint-statement-delay=todo
hint-statement-sendmodem=todo
hint-statement-inc=todo
hint-statement-dec=todo
hint-statement-newline=todo
hint-statement-newlines=todo
hint-statement-tokenize=
    Tokenize string “string” into individual items separated by semi-colons or spaces

    ### See also
    `GetToken, TokenStr, TokCount`
hint-statement-gettoken=
    ### Returns
    The next string token from a prior call to `Tokenize` (Same as the `GETTOKEN` statement but can be used in an expression without prior assignement to a variable)
    
    ### Example
    `GETTOKEN VAR`
    
    Get a token from a previous call to Tokenize and assign it to `VAR`
hint-statement-shell=todo
hint-statement-disptext=todo
hint-statement-stop=todo
hint-statement-inputtext=todo
hint-statement-beep=todo
hint-statement-push=todo
hint-statement-pop=todo
hint-statement-kbdstuff=todo
hint-statement-call=Load and execute PPE filename specified by @1
hint-statement-join=Performs a join conference command, passing it @1 as arguments
hint-statement-quest=Do script questionnaire @1
hint-statement-blt=Display bulletin number @1
hint-statement-dir=todo
hint-statement-kbdfile=todo
hint-statement-bye=todo
hint-statement-goodbye=todo
hint-statement-broadcast=Broadcast message @3 to nodes from @1 to @2 inclusive
hint-statement-waitfor=todo
hint-statement-kbdchkon=todo
hint-statement-kbdchkoff=todo
hint-statement-optext=todo
hint-statement-dispstr=todo
hint-statement-rdunet=todo
hint-statement-wrunet=todo
hint-statement-dointr=todo
hint-statement-varseg=todo
hint-statement-varoff=todo
hint-statement-pokeb=todo
hint-statement-pokew=todo
hint-statement-varaddr=todo
hint-statement-ansipos=
    Move the cursor to column @1 and row @2

    ```
    1 <= @1 <= 80  
    1 <= @2 <= 23 (Because of the status lines)  
    ```
    (1,1) is the top left corner of the screen
hint-statement-backup=todo
hint-statement-forward=Move the cursor forward @1 columns without going past column 80
hint-statement-freshline=todo
hint-statement-wrusys=todo
hint-statement-rdusys=todo
hint-statement-newpwd=todo
hint-statement-opencap=todo
hint-statement-closecap=todo
hint-statement-message=todo
hint-statement-savescrn=todo
hint-statement-restscrn=todo
hint-statement-sound=todo
hint-statement-chat=todo
hint-statement-sprint=todo
hint-statement-sprintln=todo
hint-statement-mprint=todo
hint-statement-mprintln=todo
hint-statement-rename=todo
hint-statement-frewind=todo
hint-statement-pokedw=todo
hint-statement-dbglevel=todo
hint-statement-showon=todo
hint-statement-showoff=todo
hint-statement-pageon=todo
hint-statement-pageoff=todo
hint-statement-fseek=todo
hint-statement-fflush=todo
hint-statement-fread=todo
hint-statement-fwrite=todo
hint-statement-fdefin=todo
hint-statement-fdefout=todo
hint-statement-fdget=todo
hint-statement-fdput=todo
hint-statement-fdputln=todo
hint-statement-fdputpad=todo
hint-statement-fdread=todo
hint-statement-fdwrite=todo
hint-statement-adjbytes=todo
hint-statement-kbdstring=todo
hint-statement-alias=todo
hint-statement-redim=todo
hint-statement-append=todo
hint-statement-copy=todo
hint-statement-kbdflush=todo
hint-statement-mdmflush=todo
hint-statement-keyflush=todo
hint-statement-lastin=todo
hint-statement-flag=todo
hint-statement-download=todo
hint-statement-wrusysdoor=todo
hint-statement-getaltuser=todo
hint-statement-adjdbytes=todo
hint-statement-adjtbytes=todo
hint-statement-adjtfiles=todo
hint-statement-lang=todo
hint-statement-sort=todo
hint-statement-mousereg=
    Set up a RIP mouse region on the remote terminal.
    
    @1    = Is the RIP region number  
    @2, @3  = The (X,Y) coordinates of the upper-left of the region  
    @4, @5  = The (X,Y) coordinates of the lower-right of the region  
    @6  = The width of each character in pixels  
    @7  = The height of each character in pixels  
    @8 = A boolean flag (TRUE to invert the region when clicked)  
    @9  = A boolean flag (TRUE to clear and full screen the text window)  
    @10   = Text that the remote terminal should transmit when the region is clicked  
hint-statement-scrfile=todo
hint-statement-searchinit=todo
hint-statement-searchfind=todo
hint-statement-searchstop=todo
hint-statement-prfound=todo
hint-statement-prfoundln=todo
hint-statement-tpaget=todo
hint-statement-tpaput=todo
hint-statement-tpacget=todo
hint-statement-tpacput=todo
hint-statement-tparead=todo
hint-statement-tpawrite=todo
hint-statement-tpacread=todo
hint-statement-tpacwrite=todo
hint-statement-bitset=todo
hint-statement-bitclear=todo
hint-statement-brag=todo
hint-statement-frealtuser=todo
hint-statement-setlmr=todo
hint-statement-setenv=todo
hint-statement-fcloseall=todo
hint-statement-declare=todo
hint-statement-function=todo
hint-statement-procedure=todo
hint-statement-pcall=todo
hint-statement-fpclr=todo
hint-statement-begin=todo
hint-statement-fend=todo
hint-statement-static=todo
hint-statement-stackabort=todo
hint-statement-dcreate=todo
hint-statement-dopen=todo
hint-statement-dclose=todo
hint-statement-dsetalias=todo
hint-statement-dpack=todo
hint-statement-dcloseall=todo
hint-statement-dlock=todo
hint-statement-dlockr=todo
hint-statement-dlockg=todo
hint-statement-dunlock=todo
hint-statement-dncreate=todo
hint-statement-dnopen=todo
hint-statement-dnclose=todo
hint-statement-dncloseall=todo
hint-statement-dnew=todo
hint-statement-dadd=todo
hint-statement-dappend=todo
hint-statement-dtop=todo
hint-statement-dgo=todo
hint-statement-dbottom=todo
hint-statement-dskip=todo
hint-statement-dblank=todo
hint-statement-ddelete=todo
hint-statement-drecall=todo
hint-statement-dtag=todo
hint-statement-dseek=todo
hint-statement-dfblank=todo
hint-statement-dget=todo
hint-statement-dput=todo
hint-statement-dfcopy=todo
hint-statement-eval=todo
hint-statement-account=todo
hint-statement-recordusage=todo
hint-statement-msgtofile=todo
hint-statement-qwklimits=todo
hint-statement-command=todo
hint-statement-uselmrs=todo
hint-statement-confinfo=
    This statement can be used to modify a field in the conference
    configuration.

    ### Parameters
    @1 = The conference number to get information about
    @2 = Conference field to modify. (See note)
    @3 = New value to store in field

    ### Fields
     1: STRING     Conference Name
     2: BOOLEAN    Public Conference
     8: INTEGER    Required Security if public
    40: STRING     Password to Join if Private;
    43: STRING     Auto-Register Flags
    11: INTEGER    Number of Message Blocks
    12: STRING     Name/Loc of MSGS File
    13: STRING     Name/Loc of Users's Menu
    14: STRING     Name/Loc of Sysops Menu
    15: STRING     Name/Loc of NEWS file.
    41: STRING     Name/Loc of Conf INTRO File
    42: STRING     Location for Attachments
    16: INTEGER    Public Upload Sort
    17: STRING     Name/Loc upload DIR file
    18: STRING     Location of Public Uploads
    19: INTEGER    Private Upload Sort
    20: STRING     Name/Loc Private Upload DIR file
    21: STRING     Location of private uploads
    22: STRING     Doors Menu
    23: STRING     Doors File
    24: STRING     Bulletin Menu
    25: STRING     Bulletin File
    26: STRING     Script Menu
    27: STRING     Script File
    28: STRING     Directories Menu
    29: STRING     Directories File
    30: STRING     Download Paths File
     3: BOOLEAN    Auto Rejoin
     4: BOOLEAN    View Other Users
     5: BOOLEAN    Make Uploads Private
     6: BOOLEAN    Make All Messages Private
     7: BOOLEAN    Echo Mail in Conf
    31: BOOLEAN    Force Echo on All Messages
    50: BYTE       Type of NetMail Conference
    48: BOOLEAN    Allow Internet (long) TO: Names
    32: BOOLEAN    Make Conference Read Only
    33: BOOLEAN    Disallow Private Messages
    35: BOOLEAN    Place Origin Info In Messages
    36: BOOLEAN    Prompt For Route info
    37: BOOLEAN    Allow Aliases to be used
    38: BOOLEAN    Show INTRO in 'R A' Scan
    47: BOOLEAN    Maintain Old MSGS.NDX File
    46: STRING     Conf-Specific CMD.LST File
     9: INTEGER    Additional Conference Security
    10: INTEGER    Additional Conference Time
    44: BYTE       Level to Save File Attachment
    39: INTEGER    Level to Enter a Message
    34: INTEGER    Level to Request Return Receipt
    49: BYTE       Level to Enter Carbon List Msgs
    45: BYTE       Carbon Copy List Limit
    52: DREAL      Charge Per Minute
    53: DREAL      Charge per Message Read
    54: DREAL      Charge per Message Written
    51: INTEGER    Last Message Exported
hint-statement-adjtubytes=
    This statement can be used to adjust a users total uploads bytes
    up or down.
    
    ### Parameters
    @1 = Number of bytes to adjust current users upload bytes with.
    This can be a positive or negative value
hint-statement-grafmode=
    This statement can be used to change a users graphics mode while online.

    ### Parameters
    @1 = The graphics mode to change to.
    1 = If user has ANSI capabilities it will change graphics to color ANSI
    2 = Will attempt to put user in color ansi regardless of ansi ability
    3 = Puts user in ansi-black and white mode
    4 = Puts user in non-ansi black and white mode
    5 = If user has RIP ability, will put user in RIP mode.
    (IcyBoard: 6 = Avatar mode
    )

hint-statement-adduser=
    ### Parameters
    @1 = name of the new user to add
    @2 = TRUE instructs PCBoard to leave the new user's variables active, as if a GETALTUSER were executed
    using the new user record number. FALSE will restore the current users variables.

    ### Remarks
    This statement allows PPL to create a new user record, filling
    in all fields but the name with pcboard default values.
hint-statement-killmsg=
    ### Parameters
    @1 = conference number in which the doomed message resides.
    @2 = message number to kill
hint-statement-chdir=todo
hint-statement-mkdir=Creates a new directory
    ### Parameters
    @1 = directory to create 
hint-statement-rmdir=todo
hint-statement-fdowraka=todo
hint-statement-fdoaddaka=todo
hint-statement-fdowrorg=todo
hint-statement-fdoaddorg=todo
hint-statement-fdoqmod=This statement can be used to modify fido queue records.todo
hint-statement-fdoqadd=This statement can be used to add entries to the FIDO queue.
hint-statement-fdoqdel=This statement can be used to delete fido queue records.
hint-statement-sounddelay=
    ### Parameters
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

    ### Parameters
    @1 = A boolean expression stating if the short description set on.
hint-statement-movemsg=
    Moves the message from its current location to the end of the message base.

    ### Parameters
    @1 = conference number in which the message resides
    @2 = message number to move
    @3 = A Boolean expression stating where the message should be
    move or not.  TRUE if it will be moved, FALSE if the message is to be copied.
hint-statement-setbankbal=
    Sets the value of a specified field.
    ### Parameters
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
hint-function-instr=todo
hint-function-abort=todo
hint-function-ltrim=todo
hint-function-rtrim=todo
hint-function-trim=todo
hint-function-random=todo
hint-function-date=todo
hint-function-time=todo
hint-function-u_name=todo
hint-function-u_ldate=todo
hint-function-u_ltime=todo
hint-function-u_ldir=todo
hint-function-u_logons=todo
hint-function-u_ful=todo
hint-function-u_fdl=todo
hint-function-u_bdlday=todo
hint-function-u_timeon=todo
hint-function-u_bdl=todo
hint-function-u_bul=todo
hint-function-year=todo
hint-function-month=todo
hint-function-day=todo
hint-function-dow=todo
hint-function-hour=todo
hint-function-min=todo
hint-function-sec=todo
hint-function-timeap=todo
hint-function-ver=todo
hint-function-nochar=todo
hint-function-yeschar=todo
hint-function-stripatx=todo
hint-function-replace=todo
hint-function-strip=todo
hint-function-inkey=todo
hint-function-tostring=todo
hint-function-mask_pwd=todo
hint-function-mask_alpha=todo
hint-function-mask_num=todo
hint-function-mask_alnum=todo
hint-function-mask_file=todo
hint-function-mask_path=todo
hint-function-mask_ascii=todo
hint-function-curconf=todo
hint-function-pcbdat=todo
hint-function-ppepath=todo
hint-function-valdate=todo
hint-function-valtime=todo
hint-function-u_msgrd=todo
hint-function-u_msgwr=todo
hint-function-pcbnode=todo
hint-function-readline=todo
hint-function-sysopsec=todo
hint-function-onlocal=todo
hint-function-un_stat=todo
hint-function-un_name=todo
hint-function-un_city=todo
hint-function-un_oper=todo
hint-function-cursec=todo
hint-function-gettoken=todo
hint-function-minleft=todo
hint-function-minon=todo
hint-function-getenv=todo
hint-function-callid=todo
hint-function-regal=todo
hint-function-regah=todo
hint-function-regbl=todo
hint-function-regbh=todo
hint-function-regcl=todo
hint-function-regch=todo
hint-function-regdl=todo
hint-function-regdh=todo
hint-function-regax=todo
hint-function-regbx=todo
hint-function-regcx=todo
hint-function-regdx=todo
hint-function-regsi=todo
hint-function-regdi=todo
hint-function-regf=todo
hint-function-regcf=todo
hint-function-regds=todo
hint-function-reges=todo
hint-function-b2w=todo
hint-function-peekb=todo
hint-function-peekw=todo
hint-function-mkaddr=todo
hint-function-exist=todo
hint-function-i2s=
    ### Returns
    Returns a string representing the integer value @1 converted to base @2
hint-function-s2i=todo
hint-function-carrier=todo
hint-function-tokenstr=todo
hint-function-cdon=todo
hint-function-langext=todo
hint-function-ansion=todo
hint-function-valcc=todo
hint-function-fmtcc=todo
hint-function-cctype=todo
hint-function-getx=todo
hint-function-gety=todo
hint-function-band=todo
hint-function-bor=todo
hint-function-bxor=todo
hint-function-bnot=todo
hint-function-u_pwdhist=todo
hint-function-u_pwdlc=todo
hint-function-u_pwdtc=todo
hint-function-u_stat=todo
hint-function-defcolor=todo
hint-function-abs=todo
hint-function-grafmode=todo
hint-function-psa=todo
hint-function-fileinf=todo
hint-function-ppename=todo
hint-function-mkdate=todo
hint-function-curcolor=todo
hint-function-kinkey=todo
hint-function-minkey=todo
hint-function-maxnode=todo
hint-function-slpath=todo
hint-function-helppath=todo
hint-function-temppath=todo
hint-function-modem=todo
hint-function-loggedon=todo
hint-function-callnum=todo
hint-function-mgetbyte=todo
hint-function-tokcount=todo
hint-function-u_recnum=todo
hint-function-u_inconf=todo
hint-function-peekdw=todo
hint-function-dbglevel=todo
hint-function-scrtext=
    ### Returns
    Returns a string of @3 characters from the screen at @1, @2.
    If @3 is `TRUE` then the string will be returned with all @ codes intact.
hint-function-showstat=todo
hint-function-pagestat=todo
hint-function-replacestr=todo
hint-function-stripstr=todo
hint-function-tobigstr=todo
hint-function-toboolean=todo
hint-function-tobyte=todo
hint-function-todate=todo
hint-function-todreal=todo
hint-function-toedate=todo
hint-function-tointeger=todo
hint-function-tomoney=todo
hint-function-toreal=todo
hint-function-tosbyte=todo
hint-function-tosword=todo
hint-function-totime=todo
hint-function-tounsigned=todo
hint-function-toword=todo
hint-function-mixed=Converts a string to mixed (or proper name) case
hint-function-alias=todo
hint-function-confreg=todo
hint-function-confexp=todo
hint-function-confsel=todo
hint-function-confsys=todo
hint-function-confmw=todo
hint-function-lprinted=todo
hint-function-isnonstop=todo
hint-function-errcorrect=todo
hint-function-confalias=todo
hint-function-useralias=todo
hint-function-curuser=todo
hint-function-u_lmr=todo
hint-function-chatstat=todo
hint-function-defans=todo
hint-function-lastans=todo
hint-function-meganum=todo
hint-function-evttimeadj=todo
hint-function-isbitset=todo
hint-function-fmtreal=todo
hint-function-flagcnt=todo
hint-function-kbdbufsize=todo
hint-function-pplbufsize=todo
hint-function-kbdfilused=todo
hint-function-lomsgnum=todo
hint-function-himsgnum=todo
hint-function-drivespace=todo
hint-function-outbytes=todo
hint-function-hiconfnum=todo
hint-function-inbytes=todo
hint-function-crc32=todo
hint-function-pcbmac=todo
hint-function-actmsgnum=
    ### Returns
    Returns number of active messages in current conference

    ### Example
    ```
    integer i
    println "There are ",ACTMSGNUM()," messages in conference ",CURCONF()
    ```
hint-function-stackleft=todo
hint-function-stackerr=todo
hint-function-dgetalias=todo
hint-function-dbof=todo
hint-function-dchanged=todo
hint-function-ddecimals=todo
hint-function-ddeleted=todo
hint-function-deof=todo
hint-function-derr=todo
hint-function-dfields=todo
hint-function-dlength=todo
hint-function-dname=todo
hint-function-dreccount=todo
hint-function-drecno=todo
hint-function-dtype=todo
hint-function-fnext=todo
hint-function-dnext=todo
hint-function-toddate=todo
hint-function-dcloseall=todo
hint-function-dopen=todo
hint-function-dclose=todo
hint-function-dsetalias=todo
hint-function-dpack=todo
hint-function-dlockf=todo
hint-function-dlock=todo
hint-function-dlockr=todo
hint-function-dunlock=todo
hint-function-dnopen=todo
hint-function-dnclose=todo
hint-function-dncloseall=todo
hint-function-dnew=todo
hint-function-dadd=todo
hint-function-dappend=todo
hint-function-dtop=todo
hint-function-dgo=todo
hint-function-dbottom=todo
hint-function-dskip=todo
hint-function-dblank=todo
hint-function-ddelete=todo
hint-function-drecall=todo
hint-function-dtag=todo
hint-function-dseek=todo
hint-function-dfblank=todo
hint-function-dget=todo
hint-function-dput=todo
hint-function-dfcopy=todo
hint-function-dselect=todo
hint-function-dchkstat=todo
hint-function-pcbaccount=todo
hint-function-pcbaccstat=todo
hint-function-derrmsg=todo
hint-function-account=todo
hint-function-scanmsghdr=todo
hint-function-checkrip=todo
hint-function-ripver=todo
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

    ### Fields
     1: STRING     Conference Name
     2: BOOLEAN    Public Conference
     8: INTEGER    Required Security if public
    40: STRING     Password to Join if Private;
    43: STRING     Auto-Register Flags
    11: INTEGER    Number of Message Blocks
    12: STRING     Name/Loc of MSGS File
    13: STRING     Name/Loc of Users's Menu
    14: STRING     Name/Loc of Sysops Menu
    15: STRING     Name/Loc of NEWS file.
    41: STRING     Name/Loc of Conf INTRO File
    42: STRING     Location for Attachments
    16: INTEGER    Public Upload Sort
    17: STRING     Name/Loc upload DIR file
    18: STRING     Location of Public Uploads
    19: INTEGER    Private Upload Sort
    20: STRING     Name/Loc Private Upload DIR file
    21: STRING     Location of private uploads
    22: STRING     Doors Menu
    23: STRING     Doors File
    24: STRING     Bulletin Menu
    25: STRING     Bulletin File
    26: STRING     Script Menu
    27: STRING     Script File
    28: STRING     Directories Menu
    29: STRING     Directories File
    30: STRING     Download Paths File
     3: BOOLEAN    Auto Rejoin
     4: BOOLEAN    View Other Users
     5: BOOLEAN    Make Uploads Private
     6: BOOLEAN    Make All Messages Private
     7: BOOLEAN    Echo Mail in Conf
    31: BOOLEAN    Force Echo on All Messages
    50: BYTE       Type of NetMail Conference
    48: BOOLEAN    Allow Internet (long) TO: Names
    32: BOOLEAN    Make Conference Read Only
    33: BOOLEAN    Disallow Private Messages
    35: BOOLEAN    Place Origin Info In Messages
    36: BOOLEAN    Prompt For Route info
    37: BOOLEAN    Allow Aliases to be used
    38: BOOLEAN    Show INTRO in 'R A' Scan
    47: BOOLEAN    Maintain Old MSGS.NDX File
    46: STRING     Conf-Specific CMD.LST File
     9: INTEGER    Additional Conference Security
    10: INTEGER    Additional Conference Time
    44: BYTE       Level to Save File Attachment
    39: INTEGER    Level to Enter a Message
    34: INTEGER    Level to Request Return Receipt
    49: BYTE       Level to Enter Carbon List Msgs
    45: BYTE       Carbon Copy List Limit
    52: DREAL      Charge Per Minute
    53: DREAL      Charge per Message Read
    54: DREAL      Charge per Message Written
    51: INTEGER    Last Message Exported

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
hint-function-instrr=todo
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

    ### Fields
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
hint-function-memberreference=todo
hint-function-membercall=todo
hint-function-newconfinfo=todo
