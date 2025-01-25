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

hint-statement-end=
    `END`
    
    Ends the program execution
hint-statement-cls=
    `CLS`
    
    Clears the screen
hint-statement-clreol=
    `CLREOL`
    
    Clears to the end of the line
hint-statement-more=
    `MORE`
    
    Pauses and waits for a keypress (Displays a MORE? prompt)
hint-statement-wait=
    `WAIT`
    
    Pauses and waits for a keypress
hint-statement-color=
    `COLOR col:integer`
    
    Sets the text color to `col`    
hint-statement-goto=
    `GOTO label`
    
    Jumps to the label specified
hint-statement-let=
    `LET var1=exp`
    
    Assigns the value of `exp` to `var1`
hint-statement-print=
    `PRINT exp[, exp]*`

    Print a line to the screen

    ### Remarks
    This statement will process all @ codes and display them as expected.
hint-statement-println=
    `PRINTLN [exp[, exp]*]?`

    Print a line to the screen and append a newline to the end of the expression(s).

    ### Remarks
    This statement will process all @ codes and display them as expected.
hint-statement-confflag=
    `CONFFLAG conf:integer, flags:integer`
    
    Turn on the conference `conf` flags specified by `flags`
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
    `TOKENIZE str:string`

    Tokenize string “string” into individual items separated by semi-colons or spaces

    ### See also
    `GetToken, TokenStr, TokCount`
hint-statement-gettoken=
    `GETTOKEN() :STRING`

    Returns the next string token from a prior call to `Tokenize` (Same as the `GETTOKEN` statement but can be used in an expression without prior assignement to a variable)
    
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
hint-statement-call=
    `CALL ppename`

    Load and execute PPE filename specified by “ppename”
hint-statement-join=
    `JOIN conf:integer`

    Performs a join conference command, passing it “conf” as arguments
hint-statement-quest=
    `QUEST nr:integer`

    Do script questionnaire “nr”
hint-statement-blt=
    `BLT bltnr:integer`

    Display bulletin number “bltnr”
hint-statement-dir=todo
hint-statement-kbdfile=todo
hint-statement-bye=todo
hint-statement-goodbye=todo
hint-statement-broadcast=
    `BROADCAST var1:integer, var2:integer, message:string`
    
    Broadcast message `message` to nodes from `var1` to `var2` inclusive
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
    `ANSIPOS col:integer, row:integer`
    
    Move the cursor to column `col` and row `row`

    ```
    1 <= col <= 80  
    1 <= row <= 23 (Because of the status lines)  
    ```
    (1,1) is the top left corner of the screen
hint-statement-backup=todo
hint-statement-forward=
    `FORWARD var:integer`
    
    Move the cursor forward `var` columns without going past column 80
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
    `MOUSEREG num,x1,y1,x2,y2,fontX,fontY,invert,clear,text`

    Set up a RIP mouse region on the remote terminal.

    `num`    = Is the RIP region number  
    `x1`,y1  = The (X,Y) coordinates of the upper-left of the region  
    `x2`,y2  = The (X,Y) coordinates of the lower-right of the region  
    `fontX`  = The width of each character in pixels  
    `fontY`  = The height of each character in pixels  
    `invert` = A boolean flag (TRUE to invert the region when clicked)  
    `clear`  = A boolean flag (TRUE to clear and full screen the text window)  
    `text`   = Text that the remote terminal should transmit when the region is clicked  
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
hint-statement-confinfo=todo
hint-statement-adjtubytes=todo
hint-statement-grafmode=todo
hint-statement-adduser=todo
hint-statement-killmsg=todo
hint-statement-chdir=todo
hint-statement-mkdir=todo
hint-statement-redir=todo
hint-statement-fdowraka=todo
hint-statement-fdoaddaka=todo
hint-statement-fdowrorg=todo
hint-statement-fdoaddorg=todo
hint-statement-fdoqmod=todo
hint-statement-fdoqadd=todo
hint-statement-fdoqdel=todo
hint-statement-sounddelay=todo
hint-statement-shortdesc=todo
hint-statement-movemsg=todo
hint-statement-setbankbal=todo

hint-function-len=
    ### Function
    `LEN(var1:bigstr) :INTEGER`

    #Remarks
    Returns the length of the string `var1`
hint-function-lower=
    ### Function
    `LOWER(var1:bigstr) :BIGSTR`

    Returns the string `var1` converted to lower case
hint-function-upper=
    ### Function
    `UPPER(var1:bigstr) :BIGSTR`

    Returns the string `var1` converted to upper case
hint-function-mid=
    ### Function
    `MID(str:bigstr,start:integer,length:integer) :BIGSTR`

    Returns a substring of `str` starting at position `start` and `length` characters long
hint-function-left=
    ### Function
    `LEFT(str:bigstr,length:integer) :BIGSTR`

    Returns the leftmost `length` characters of `str`
hint-function-right=
    ### Function
    `RIGHT(str:bigstr,length:integer) :BIGSTR`

    Returns the rightmost `length` characters of `str`
hint-function-space=
    ### Function
    `SPACE(length:integer) :BIGSTR`

    Returns a string of `length` spaces
hint-function-ferr=todo
hint-function-chr=
    ### Function
    `CHR(var1:integer) : BIGSTR`

    Returns a single character long string of the character represented by ASCII code var (0-255)
hint-function-asc=
    ### Function
    `ASC(var1:bigstr) :INTEGER`

    Returns the ASCII value of the first character in `var1`
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
    ### Function
    `I2S(var1:integer,var2:integer) :STRING`

    Returns a string representing the integer value `var1` converted to base `var2`
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
    ### Function
    `SCRTEXT(col:integer, row:integer, len:integer, code:boolean) : STRING`

    Returns a string of `len` characters from the screen at `col`, `row`. 
    If `code` is `TRUE` then the string will be returned with all @ codes intact.
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
hint-function-mixed=
    ### Function
    `MIXED(var1:string)`

    Converts a string to mixed (or proper name) case
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
    ### Function
    `ACTMSGNUM()`

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
    ### Function
    `FINDFIRST(filespec:bigstr) :BIGSTR`

    Find the first occurence of filespec in a directory. Used in conjunction with FindNext to get a directory listing.
hint-function-findnext=
    ### Function
    `FINDNEXT() :BIGSTR`

    Find the next occurence of filespec (used with FindFirst) in a directory.
hint-function-uselmrs=todo
hint-function-confinfo=todo
hint-function-tinkey=todo
hint-function-cwd=todo
hint-function-instrr=todo
hint-function-fdordaka=todo
hint-function-fdordorg=todo
hint-function-fdordarea=todo
hint-function-fdoqrd=todo
hint-function-getdrive=todo
hint-function-setdrive=todo
hint-function-bs2i=todo
hint-function-bd2i=todo
hint-function-i2bs=todo
hint-function-i2bd=todo
hint-function-ftell=todo
hint-function-os=todo
hint-function-short_desc=todo
hint-function-getbankbal=todo
hint-function-getmsghdr=todo
hint-function-setmsghdr=todo
hint-function-memberreference=todo
hint-function-membercall=todo
hint-function-newconfinfo=todo