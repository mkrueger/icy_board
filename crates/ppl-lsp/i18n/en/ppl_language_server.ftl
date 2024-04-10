hint-type-boolean=
    BOOLEAN
    
    unsigned character (1 byte) 0 = FALSE, non-0 = TRUE
hint-type-date=
    DATE
    
    unsigned integer (2 bytes) PCBoard julian date (count of days since 1/1/1900) 
hint-type-ddate=
    DDATE

    Signed long integer for julian date. DDATE is for use with DBase date fields.
    It holds a long integer for julian dates. When coerced to string type it is in the format CCYYMMDD or 19940527
hint-type-integer=
    INTEGER / SDWORD / LONG

    signed long integer (4 bytes) Range: -2,147,483,648 → +2,147,483,647
hint-type-money=
    MONEY
    
    signed long integer (4 bytes) Range: -$21,474,836.48 → +$21,474,836.47
hint-type-string=
    STRING
    
    far character pointer (4 bytes) NULL is an empty string non-NULL points to a string of some length less than or equal to 256
hint-type-time=
    TIME
    
    signed long integer (4 bytes) Count of seconds since midnight
hint-type-bigstr=
    BIGSTR
    
    Allows up to 2048 characters per big string (up from 256 for STRING variables) May include CHR(0) characters in the middle of the big string (unlike STRING variables which may not)
hint-type-edate=
    EDATE
    
    Julian date in earth date format Deals with dates formatted YYMM.DD Range: Same as DATE
hint-type-float=
    REAL / FLOAT

    4-byte floating point number Range: +/-3.4E-38 - +/-3.4E+38 (7-digit precision)
hint-type-double=
    DREAL / DOUBLE

    8-byte floating point number Range: +/-1.7E-308 - +/-1.7E+308 (15-digit precision)
hint-type-unsigned=
    UNSIGNED / DWORD / UDWORD

    4-byte unsigned integer Range: 0 - 4,294,967,295
hint-type-byte=
    BYTE / UBYTE

    1-byte unsigned integer Range: 0 - 255
hint-type-word=
    WORD / UWORD

    2-byte unsigned integer Range: 0 - 65,535
hint-type-sbyte=
    SBYTE / SHORT

    1-byte signed Integer Range: -128 - 127
hint-type-sword=
    SWORD / INT

    2-byte signed integer Range: -32,768 - 32,767

hint-statement-end=todo
hint-statement-cls=todo
hint-statement-clreol=todo
hint-statement-more=todo
hint-statement-wait=todo
hint-statement-color=todo
hint-statement-goto=todo
hint-statement-let=todo
hint-statement-print=
    ### Function
    Print a line to the screen
    ### Syntax
    ```PRINT exp[, exp]*```
    ### Remarks
    This statement will process all @ codes and display them as expected.
hint-statement-println=
    ### Function
    Print a line to the screen and append a newline to the end of the expression(s).
    ### Syntax
    ```PRINTLN [exp[, exp]*]?```
    ### Remarks
    This statement will process all @ codes and display them as expected.
hint-statement-confflag=todo
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
    ### Function
    Adjust the users time up or down
    ### Syntax
    ```ADJTIME minutes```
    ```minutes``` An integer expression containing the number of minutes to adjust the time left by. > 0 will add time, < 0 will deduct time.
    ### Remarks
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
hint-statement-tokenize=todo
hint-statement-gettoken=todo
hint-statement-shell=todo
hint-statement-disptext=todo
hint-statement-stop=todo
hint-statement-inputtext=todo
hint-statement-beep=todo
hint-statement-push=todo
hint-statement-pop=todo
hint-statement-kbdstuff=todo
hint-statement-call=todo
hint-statement-join=todo
hint-statement-quest=todo
hint-statement-blt=todo
hint-statement-dir=todo
hint-statement-kbdfile=todo
hint-statement-bye=todo
hint-statement-goodbye=todo
hint-statement-broadcast=todo
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
hint-statement-ansipos=todo
hint-statement-backup=todo
hint-statement-forward=todo
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
hint-statement-mousereg=todo
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
