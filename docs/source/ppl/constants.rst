Constants
---------

+--------------------+-----------+--------------------------------------------------------------+
| Name               | Value Hex | Description                                                  |
+====================+===========+==============================================================+
| ACC_CUR_BAL        | 0004      | Current accounting balance (selector / field id)             |
+--------------------+-----------+--------------------------------------------------------------+
| ACC_MSGREAD        | 0002      | Accounting selector: messages read                           |
+--------------------+-----------+--------------------------------------------------------------+
| ACC_MSGWRITE       | 0003      | Accounting selector: messages written                        |
+--------------------+-----------+--------------------------------------------------------------+
| ACC_STAT           | 0000      | Accounting status selector                                   |
+--------------------+-----------+--------------------------------------------------------------+
| ACC_TIME           | 0001      | Accounting selector: time factor                             |
+--------------------+-----------+--------------------------------------------------------------+
| ATTACH_LIM_P       | 0003      | Attachment level (private save limit)                        |
+--------------------+-----------+--------------------------------------------------------------+
| ATTACH_LIM_U       | 0002      | Attachment level (upload save limit)                         |
+--------------------+-----------+--------------------------------------------------------------+
| AUTO               | 2000      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (automatically press enter after 10 seconds of no user input)|
+--------------------+-----------+--------------------------------------------------------------+
| BELL               | 0800      | Parameter passed to DISPTEXT statement (sound a bell when    |
|                    |           | prompt displayed)                                            |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_CALL          | 0001      | Charge type: per call                                        |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_CHAT          | 0004      | Charge type: group chat minutes                              |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_DOWNBYTES     | 000B      | Charge type: bytes downloaded                                |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_DOWNFILE      | 000A      | Charge type: file downloaded                                 |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_MSGCAP        | 0006      | Charge type: message captured (QWK, etc.)                    |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_MSGECHOED     | 0008      | Charge type: echoed message written                          |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_MSGPRIVATE    | 0009      | Charge type: private message written                         |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_MSGREAD       | 0005      | Charge type: message read                                    |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_MSGWRITE      | 0007      | Charge type: message written                                 |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_PEAKTIME      | 0003      | Charge type: peak time minutes                               |
+--------------------+-----------+--------------------------------------------------------------+
| CHRG_TIME          | 0002      | Charge type: normal time minutes                             |
+--------------------+-----------+--------------------------------------------------------------+
| CRC_FILE           | 0001      | CRC mode: file                                               |
+--------------------+-----------+--------------------------------------------------------------+
| CRC_STR            | 0000      | CRC mode: string                                             |
+--------------------+-----------+--------------------------------------------------------------+
| CRED_SPECIAL       | 0010      | Credit type: special                                         |
+--------------------+-----------+--------------------------------------------------------------+
| CRED_UPBYTES       | 000F      | Credit type: bytes uploaded                                  |
+--------------------+-----------+--------------------------------------------------------------+
| CRED_UPFILE        | 000E      | Credit type: file uploaded                                   |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_CALL           | 0002      | Debit log selector: per call                                 |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_CHAT           | 000B      | Debit log selector: chat                                     |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_DOWNBYTES      | 000A      | Debit log selector: bytes downloaded                         |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_DOWNFILE       | 0009      | Debit log selector: file downloaded                          |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_MSGCAP         | 0005      | Debit log selector: message captured                         |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_MSGECHOED      | 0007      | Debit log selector: echoed message                           |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_MSGPRIVATE     | 0008      | Debit log selector: private message                          |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_MSGREAD        | 0004      | Debit log selector: message read                             |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_MSGWRITE       | 0006      | Debit log selector: message written                          |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_SPECIAL        | 000D      | Debit log selector: special                                  |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_TIME           | 0003      | Debit log selector: time                                     |
+--------------------+-----------+--------------------------------------------------------------+
| DEB_TPU            | 000C      | Debit log selector: time per upload (legacy)                 |
+--------------------+-----------+--------------------------------------------------------------+
| DEFS               | 0000      | Parameter passed to various statements for default values    |
+--------------------+-----------+--------------------------------------------------------------+
| ECHODOTS           | 0001      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (echo dots instead of user input)                            |
+--------------------+-----------+--------------------------------------------------------------+
| ERASELINE          | 0020      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (erase the current line when user presses enter)             |
+--------------------+-----------+--------------------------------------------------------------+
| F_EXP              | 0002      | Expired subscription access allowed flag for CONFFLAG and    |
|                    |           | CONFUNFLAG                                                   |
+--------------------+-----------+--------------------------------------------------------------+
| F_MW               | 0010      | Mail waiting flag for CONFFLAG and CONFUNFLAG                |
+--------------------+-----------+--------------------------------------------------------------+
| F_NET              | 0020      | Net flag / network related (legacy)                          |
+--------------------+-----------+--------------------------------------------------------------+
| F_REG              | 0001      | Registered access allowed flag for CONFFLAG and CONFUNFLAG   |
+--------------------+-----------+--------------------------------------------------------------+
| F_SEL              | 0004      | Conference selected flag for CONFFLAG and CONFUNFLAG         |
+--------------------+-----------+--------------------------------------------------------------+
| F_SYS              | 0008      | Conference SysOp access flag for CONFFLAG and CONFUNFLAG     |
+--------------------+-----------+--------------------------------------------------------------+
| FALSE              | 0000      | BOOLEAN FALSE value                                          |
+--------------------+-----------+--------------------------------------------------------------+
| FCL                | 0002      | Value passed to STARTDISP to force line counting display     |
+--------------------+-----------+--------------------------------------------------------------+
| FIELDLEN           | 0002      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (displays parenthesis to show input field width if ANSI      |
|                    |           | enabled)                                                     |
+--------------------+-----------+--------------------------------------------------------------+
| FNS                | 0001      | Value passed to STARTDISP to force non-stop display          |
+--------------------+-----------+--------------------------------------------------------------+
| GRAPH              | 0001      | Parameter passed to DISPFILE statement to search for         |
|                    |           | graphics specific files                                      |
+--------------------+-----------+--------------------------------------------------------------+
| GUIDE              | 0004      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (displays parenthesis above current line if FIELDLEN used    |
|                    |           | and ANSI not enabled)                                        |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_ACTIVE         | 000E      | Message header field id: active status                       |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_BLOCKS         | 0004      | Message header field id: blocks count                        |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_DATE           | 0005      | Message header field id: date                                |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_ECHO           | 000F      | Message header field id: echo info                           |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_FROM           | 000B      | Message header field id: from                                |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_MSGNUM         | 0002      | Message header field id: number                              |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_MSGREF         | 0003      | Message header field id: reference                           |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_PWD            | 000D      | Message header field id: password (legacy)                   |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_REPLY          | 000A      | Message header field id: reply marker                        |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_RPLYDATE       | 0008      | Message header field id: reply date                          |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_RPLYTIME       | 0009      | Message header field id: reply time                          |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_STATUS         | 0001      | Message header field id: status flags                        |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_SUBJ           | 000C      | Message header field id: subject                             |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_TIME           | 0006      | Message header field id: time                                |
+--------------------+-----------+--------------------------------------------------------------+
| HDR_TO             | 0007      | Message header field id: to                                  |
+--------------------+-----------+--------------------------------------------------------------+
| HIGHASCII          | 1000      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (allow high ascii characters, regardless of current valid    |
|                    |           | character set, if disable high ascii filter set to yes)      |
+--------------------+-----------+--------------------------------------------------------------+
| LANG               | 0004      | Parameter passed to DISPFILE statement to search for         |
|                    |           | language specific files                                      |
+--------------------+-----------+--------------------------------------------------------------+
| LFAFTER            | 0100      | Parameter passed to INPUTSTR, PROMPTSTR and DISPTEXT         |
|                    |           | statements (send an extra line feed after user presses enter)|
+--------------------+-----------+--------------------------------------------------------------+
| LFBEFORE           | 0080      | Parameter passed to INPUTSTR, PROMPTSTR and DISPTEXT         |
|                    |           | statements (send an extra line feed before prompt display)   |
+--------------------+-----------+--------------------------------------------------------------+
| LOGIT              | 8000      | Parameter passed to DISPTEXT statement (log text to          |
|                    |           | callers log)                                                 |
+--------------------+-----------+--------------------------------------------------------------+
| LOGITLEFT          | 10000     | Parameter passed to DISPTEXT statement (log text to          |
|                    |           | callers log, forcing left justification)                     |
+--------------------+-----------+--------------------------------------------------------------+
| MAXMSGS            | 0000      | Message counter selector (max)                               |
+--------------------+-----------+--------------------------------------------------------------+
| NC                 | 0000      | Value passed to STARTDISP to not change display mode         |
+--------------------+-----------+--------------------------------------------------------------+
| NEWBALANCE         | 0000      | Accounting summary: new balance                              |
+--------------------+-----------+--------------------------------------------------------------+
| NEWLINE            | 0040      | Parameter passed to INPUTSTR, PROMPTSTR and DISPTEXT         |
|                    |           | statements (send a line feed after user presses enter)       |
+--------------------+-----------+--------------------------------------------------------------+
| NO_USER            | FFFFFFFF  | Sentinel "no user" (-1)                                      |
+--------------------+-----------+--------------------------------------------------------------+
| NOCLEAR            | 0400      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (don't clear field at first keypress regardless of ANSI)     |
+--------------------+-----------+--------------------------------------------------------------+
| O_RD               | 0000      | Parameter passed to FCREATE/FOPEN/FAPPEND to open a file     |
|                    |           | in read only mode                                            |
+--------------------+-----------+--------------------------------------------------------------+
| O_RW               | 0002      | Parameter passed to FCREATE/FOPEN/FAPPEND to open a file     |
|                    |           | in read and write mode                                       |
+--------------------+-----------+--------------------------------------------------------------+
| O_WR               | 0001      | Parameter passed to FCREATE/FOPEN/FAPPEND to open a file     |
|                    |           | in write only mode                                           |
+--------------------+-----------+--------------------------------------------------------------+
| PAY_UPBYTES        | 000D      | Payback type: bytes uploaded                                 |
+--------------------+-----------+--------------------------------------------------------------+
| PAY_UPFILE         | 000C      | Payback type: file uploaded                                  |
+--------------------+-----------+--------------------------------------------------------------+
| S_DB               | 0003      | Parameter passed to FCREATE/FOPEN/FAPPEND to deny read and   |
|                    |           | write (both) access from other processes                     |
+--------------------+-----------+--------------------------------------------------------------+
| S_DN               | 0000      | Parameter passed to FCREATE/FOPEN/FAPPEND to allow read and  |
|                    |           | write (deny none) access from other processes                |
+--------------------+-----------+--------------------------------------------------------------+
| S_DR               | 0001      | Parameter passed to FCREATE/FOPEN/FAPPEND to deny read       |
|                    |           | access from other processes                                  |
+--------------------+-----------+--------------------------------------------------------------+
| S_DW               | 0002      | Parameter passed to FCREATE/FOPEN/FAPPEND to deny write      |
|                    |           | access from other processes                                  |
+--------------------+-----------+--------------------------------------------------------------+
| SEC                | 0002      | Parameter passed to DISPFILE statement to search for         |
|                    |           | security specific files                                      |
+--------------------+-----------+--------------------------------------------------------------+
| SEC_DROP           | 0011      | Security drop indicator (legacy)                             |
+--------------------+-----------+--------------------------------------------------------------+
| SEEK_CUR           | 0001      | Seek origin: current                                         |
+--------------------+-----------+--------------------------------------------------------------+
| SEEK_END           | 0002      | Seek origin: end                                             |
+--------------------+-----------+--------------------------------------------------------------+
| SEEK_SET           | 0000      | Seek origin: start                                           |
+--------------------+-----------+--------------------------------------------------------------+
| STACKED            | 0010      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (allow semi-colons and spaces in addition to valid           |
|                    |           | character set passed)                                        |
+--------------------+-----------+--------------------------------------------------------------+
| START_BAL          | 0000      | Start-of-session accounting slot                             |
+--------------------+-----------+--------------------------------------------------------------+
| START_SESSION      | 0001      | Start-of-session selector                                    |
+--------------------+-----------+--------------------------------------------------------------+
| STK_LIMIT          | 17BE      | Stack size limit (engine)                                    |
+--------------------+-----------+--------------------------------------------------------------+
| TRUE               | 0001      | BOOLEAN TRUE value                                           |
+--------------------+-----------+--------------------------------------------------------------+
| UPCASE             | 0008      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (force user input to upper case)                             |
+--------------------+-----------+--------------------------------------------------------------+
| WARNLEVEL          | 000E      | Accounting warn threshold id                                 |
+--------------------+-----------+--------------------------------------------------------------+
| WORDWRAP           | 0200      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (if user hits end of line, save the text at the end of       |
|                    |           | the line for future use)                                     |
+--------------------+-----------+--------------------------------------------------------------+
| YESNO              | 4000      | Parameter passed to INPUTSTR and PROMPTSTR statements        |
|                    |           | (Only allow international yes/no responses)                  |
+--------------------+-----------+--------------------------------------------------------------+

Variables
---------

These variables provide read/write access to user account data. Variables are accessed through PPL statements and functions.
User variables are typically accessed via GETUSER/PUTUSER statements.

+--------------------+------------+---------------+-------------------------------------------------------+
| Variable           | Type       | Since Version | Description                                           |
+====================+============+===============+=======================================================+
| U_ACCOUNT          | INTEGER[17]| 3.00          | User accounting data array (17 elements)              |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_ADDR             | STRING[6]  | 1.00          | User address lines (6-element array)                  |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_ALIAS            | STRING     | 1.00          | Alias (if the SysOp has enabled alias use)            |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_BDPHONE          | STRING     | 1.00          | Business/data phone number                            |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_BIRTHDATE        | STRING     | 3.40          | Birth date                                            |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_CITY             | STRING     | 1.00          | City/state information                                |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_CLS              | BOOLEAN    | 1.00          | Clear screen between messages preference              |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_CMNT1            | STRING     | 1.00          | Comment line 1                                        |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_CMNT2            | STRING     | 1.00          | Comment line 2                                        |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_DEF79            | BOOLEAN    | 1.00          | Default to 79 column mode                             |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_EMAIL            | STRING     | 3.40          | Email address                                         |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_EXPDATE          | DATE       | 1.00          | Account expiration date                               |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_EXPERT           | BOOLEAN    | 1.00          | Expert mode flag                                      |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_EXPSEC           | INTEGER    | 1.00          | Security level after expiration                       |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_FSE              | BOOLEAN    | 1.00          | Full screen editor preference                         |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_FSEP             | BOOLEAN    | 1.00          | Full screen editor prompt preference                  |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_GENDER           | STRING     | 3.40          | User's gender                                         |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_HVPHONE          | STRING     | 1.00          | Home/voice phone number                               |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_LONGHDR          | BOOLEAN    | 1.00          | Long message header preference                        |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_NOTES            | STRING[5]  | 1.00          | SysOp notes about user (5-element array)              |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_PAGELEN          | INTEGER    | 1.00          | Page length (lines per screen)                        |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_PWD              | STRING     | 1.00          | User's password [1]                                   |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_PWDEXP           | DATE       | 1.00          | Password expiration date                              |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_SCROLL           | BOOLEAN    | 1.00          | Screen scrolling preference                           |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_SEC              | INTEGER    | 1.00          | Security level                                        |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_SHORTDESC        | BOOLEAN    | 3.40          | Short description preference                          |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_TRANS            | STRING     | 1.00          | Transfer protocol preference                          |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_VER              | STRING     | 1.00          | User verification string                              |
+--------------------+------------+---------------+-------------------------------------------------------+
| U_WEB              | STRING     | 3.40          | User's website URL                                    |
+--------------------+------------+---------------+-------------------------------------------------------+

.. [1] U_PWD value is usually '******' for crypted passwords. PlainText only supported if system configured for it.