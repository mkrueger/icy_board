#![allow(dead_code)]
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    ops::{Deref, DerefMut},
    path::Path,
};

use byteorder::WriteBytesExt;
use icy_ppe::{
    tables::{import_cp437_string, UNICODE_TO_CP437},
    Res,
};
use strum_macros::{Display, EnumString};
use thiserror::Error;
use toml::Value;

use crate::vm::errors::IcyError;

use super::icb_config::IcbColor;

#[repr(usize)]
#[derive(Clone, Copy, EnumString, Display, PartialEq, Eq)]
pub enum IceText {
    UnusedStatusLine,
    /// `Leave a comment for the sysop (Enter)=no`
    LeaveComment = 1,
    /// `Brand of CPU you are using`
    CommentFieldPrompt = 2,
    /// `Access Denied - Upcoming Event Pending ...`
    AccessDeniedForEvent = 3,
    /// `Time Limit Exceeded.`
    TimelimitExceeded = 4,
    /// `Access Denied - Unauthorized name match on @USER@!`
    UnauthorizedName = 5,
    /// `Access Denied - You are Locked Out of this System!`
    LockedOut = 6,
    /// `Access Denied - Excessive Password Failures!`
    DeniedWrongPassword = 7,
    /// `Access Denied - Refused to Complete Registration!`
    DeniedRefuseToRegister = 8,
    /// `Access Denied - Remote DOS Password Failure!`
    DeniedPasswordFailed = 9,
    /// `Access Denied - @USER@ is in use on another Node!`
    NameAlreadyInUse = 10,
    /// `Sorry, @FIRST@, this is a `Closed' Board ...`
    ClosedBoard = 11,
    /// `Security Violation(s) - Disconnecting ...`
    SecurityViolation = 12,
    /// `There is presently no room for your text, @FIRST@ ...`
    NoRoomForText = 13,
    /// `A message to `ALL' cannot be protected, @FIRST@ ...`
    CantProtectMessageToAll = 14,
    /// `Callers must know p/w to read message!  Continue (Enter)=no`
    CallerMustKnowPassword = 15,
    /// `Protocol Unavailable - Modem Type Wrong!`
    WrongModemType = 16,
    /// `Local Download Path`
    LocalDownloadPath = 17,
    /// `Password to Read Message (ENTER alone aborts)`
    PasswordToReadMessage = 18,
    /// `Local Upload Path or Filename`
    LocalUploadPath = 19,
    /// `Printer Off-Line ...`
    PrinterOffLine = 20,
    /// `Enter the Security password desired`
    SecurityPassword = 21,
    /// `Reply to Msgs: (#), (#), (Enter)=abort`
    ReplyToMessages = 22,
    /// `Path error in system configuration!`
    PathErrorInSystemConfiguration = 23,
    /// `Selection not available in local mode.`
    NotAvailableLocally = 24,
    /// `Temporary Sysop privileges removed ...`
    SysopLevelRemoved = 25,
    /// `Temporary Sysop privileges granted this call only!`
    SysopLevelGranted = 26,
    /// `Graphics mode is not available, @FIRST@ ...`
    GraphicsUnavailable = 27,
    /// `Automatic Lockout completed ...`
    AutomaticLockout = 28,
    /// `!|*|Y01000400|c0A|@0005RIPscrip |Y01000200|c0B|@3W0Fmode on|g0006`
    RIPModeOn = 29,
    /// `Text Entry is Full ...`
    TextEntryFull = 30,
    /// `ASCII transfer is not available on a binary file!`
    ASCIINotAvailableOnBinaryFile = 31,
    /// `NOTE: Transfer must end with a (Ctrl-Z)!`
    MustEndWithCTRLZ = 32,
    /// `None`
    None = 33,
    /// `NO CONNECT / @OPTEXT@`
    NoConnect = 34,
    /// `FULL COUNTDOWN`
    FullCountdown = 35,
    /// `NO CARRIER`
    NoCarrier = 36,
    /// `Disk Full - System presently unavailable!`
    DiskFull = 37,
    /// `ALL NODES ARE BUSY AT THIS TIME PLEASE TRY LATER`
    NodesBusy = 38,
    /// `ALL`
    AllName = 39,
    /// `Uploading text file directly into the message editor...`
    UploadMode = 40,
    /// `~(Echo)`
    Echo = 41,
    /// `~(A)`
    AllConfIndicator = 42,
    /// `~(R/O)`
    ReadonlyIndicator = 43,
    /// `Error reading Survey`
    ErrorReadingSurvey = 44,
    /// `Error reading Message base in use!`
    MessageBaseInUse = 45,
    /// `Unable to exit to DOS - no REMOTE.SYS file found!`
    NoREMOTESYSFile = 46,
    /// `No record available to update!`
    NoRecordAvailableToUpdate = 47,
    /// `Messages Successfully Packed & Purified!`
    MessagesSuccessfullyPacked = 48,
    /// `Enter `handle' to be used for Node Chat`
    HandleForChat = 49,
    /// `File Attachment: @OPTEXT@`
    Attachment = 50,
    /// `Enter a longer description of the file please!`
    LongerDescription = 51,
    /// `No Surveys are available, @FIRST@ ...`
    NoSurveysAvailable = 52,
    /// `(R) to re-enter your name or (C) to continue logon as a new user`
    ReEnterName = 53,
    /// `Would you like to register with us, @FIRST@? (Enter)=yes`
    Register = 54,
    /// `Keyboard Time Expired!`
    KeyboardTimeExpired = 55,
    /// `Invalid Entry!  Please try again, @FIRST@ ...`
    InvalidEntry = 56,
    /// `Sorry, @FIRST@, uploads are Private ...`
    UploadsArePrivate = 57,
    /// `Online Registration being performed - please wait, @FIRST@ ...`
    OnlineUpgrade = 58,
    /// `Automatic Disconnect Completed!`
    AutoDisconnectNow = 59,
    /// `Full USER Name to Find (Enter)=none`
    FullUserNameToFind = 60,
    /// `Enter the filename to Download (Enter)=none`
    FileNameToDownload = 61,
    /// `Enter the filename to View (Enter)=none`
    TextViewFileName = 62,
    /// `Menu Letter/Command you need Help with (Enter)=none`
    HelpPrompt = 63,
    /// `Conference # to join (Enter)=none`
    JoinConferenceNumber = 64,
    /// `Node # to Auto-Logoff (Enter)=none`
    NodeNumberToLogoff = 65,
    /// `Node # to View, (A)ll or (Enter)=none`
    NodeToView = 66,
    /// `Survey # to Answer (Enter)=none`
    QuestionNumberToAnswer = 67,
    /// `Enter the Filename to Upload (Enter)=none`
    FileNameToUpload = 68,
    /// `Enter the DOOR # to Open (Enter)=none`
    DOORNumber = 69,
    /// `Enter the Text to Scan for (Enter)=none`
    TextToScanFor = 70,
    /// `Search Filename (wildcards are OK) (Enter)=none`
    SearchFileName = 71,
    /// `Date as (mmddyy) to search from (Enter)=`
    DateToSearch = 72,
    /// `Private Discussion`
    PrivateTopic = 73,
    /// `Sorry, @FIRST@, you are not registered in Conference @OPTEXT@`
    NotRegisteredInConference = 74,
    /// `Open Discussion`
    OpenTopic = 75,
    /// `CAUTION: Auto-Disconnect in (@OPTEXT@) min.!`
    AutoDisconnect = 76,
    /// `Message # to Activate (Enter)=none`
    MessageNumberToActivate = 77,
    /// `Sorry, @FIRST@, no mail found to read ...`
    NoMailFound = 78,
    /// `Pack the message base (Enter)=no`
    PackTheMessageBase = 79,
    /// `Delete the caller log (Enter)=no`
    DeleteCallersLog = 80,
    /// `Purge older than (Enter)=010180`
    PurgeOlderThan1980 = 81,
    /// `Renumber during repack (Enter)=no`
    RenumberDuringPack = 82,
    /// `NEW low starting Message # (Enter)=abort`
    NewLowMessageNumber = 83,
    /// `Complete the Survey (Enter)=no`
    CompleteQuestion = 84,
    /// `Do you want to delete this line (Enter)=no`
    WantToDeleteLine = 85,
    /// `Pack the user file (Enter)=no`
    PackTheUsersFile = 86,
    /// `(@OPTEXT@) already exists.  Remove it (Y=Yes, N=No, C=Continue)`
    Overwrite = 87,
    /// `View other Conference members (Enter)=no`
    ViewConferenceMembers = 88,
    /// `Purge RECEIVED+PRIVATE Msgs (Enter)=no`
    PurgePrivateReceived = 89,
    /// `Exit to DOS (Enter)=no`
    ExitToDOS = 90,
    /// `Reference Message number purification proceeding ...`
    ReferenceMessagePurififcation = 91,
    /// `Wrong password entered, @FIRST@ ...`
    WrongPasswordEntered = 92,
    /// `There is no such line, @FIRST@ ...`
    NoSuchLineNumber = 93,
    /// `Sorry, @FIRST@, there is no Message #`
    NoSuchMessageNumber = 94,
    /// `Sorry, @FIRST@, you cannot kill Message #`
    YouCanNotKillMessage = 95,
    /// `Invalid Selection, @FIRST@!  Please try again ...`
    InvalidSelection = 96,
    /// `Page ends in 30 seconds.  (Ctrl-K) Aborts .`
    Paging = 97,
    /// `Channel @OPTEXT@:~`
    ChannelText = 98,
    /// `@FIRST@, press (Enter) for `no change' to any item ...`
    EnterNoChange = 99,
    /// `SHELL completed on (@OPTEXT@).`
    ShellCompleted = 100,
    /// `Page Length now set to @OPTEXT@.`
    PageLengthSetTo = 101,
    /// `That user is not registered in this Conference ...`
    UserNotRegisteredInConference = 102,
    /// `@INCONF@Survey (@OPTEXT@) info saved.`
    SurveyInfoSaved = 103,
    /// `Automatic Logoff Completed ...`
    AutoLogoff = 104,
    /// `Keep `LOCKED OUT' users (Enter)=yes`
    KeepLockedOut = 105,
    /// `Purge older than (Enter)=010180`
    PurgeOlderThan = 106,
    /// `Keep whose security => (Enter)=100`
    KeepSecurity = 107,
    /// `User file successfully packed.`
    UsersFilePacked = 108,
    /// `Joining channel #@OPTEXT@.`
    JoiningChannel = 109,
    /// `Sorry, @FIRST@, the two passwords do not match!`
    PasswordsDontMatch = 110,
    /// `Re-enter password to verify`
    ReEnterPassword = 111,
    /// `Enter user's Security Level`
    EnterUserSecurityLevel = 112,
    /// `Business or data phone # is`
    BusDataPhone = 113,
    /// `Home or voice phone # is`
    HomeVoicePhone = 114,
    /// `Registered in Conferences`
    RegisteredInConferences = 115,
    /// `@FIRST@, your time has been adjusted for our event.`
    TimeAdjusted = 116,
    /// `Response to the Question is Required!`
    ResponseRequired = 117,
    /// `Sorry, @FIRST@, only real names are allowed ...`
    RealNamesOnly = 118,
    /// `Insufficient security for Viewing (@OPTEXT@)`
    InsufficientSecurityToView = 119,
    /// `Sorry, @FIRST@, DOOR selected is not available.`
    DOORNotAvailable = 120,
    /// `Sorry, @FIRST@, no Network is active.`
    NoNetworkActive = 121,
    /// `Enter (U) for status while awaiting other caller ...`
    StatusWhileAwaitingOtherCaller = 122,
    /// `Thanks, @FIRST@, your registration information is saved.`
    RegisteredInfoSaved = 123,
    /// `Sorry, @FIRST@, PACK not available from remote ...`
    PackNotAailableFromRemote = 124,
    /// `Error saving message ...`
    ErrorSavingMessage = 125,
    /// `Error in user file ...`
    ErrorInUsersFile = 126,
    /// `CHANNEL BUSY!  Please wait a moment before typing...`
    ChannelBusy = 127,
    /// `Sorry, the Sysop is not currently available for CHAT!`
    SysopUnAvailable = 128,
    /// `Sysop - Busy`
    SysopBusy = 129,
    /// `User - Busy`
    UserBusy = 130,
    /// `DOS - Busy`
    DOSBusy = 131,
    /// `DOS - Not Busy`
    DOSNotBusy = 132,
    /// `Printer is Off`
    TurnPrinterOn = 133,
    /// `Alarm is Off`
    TurnAlarmOn = 134,
    /// `Page Bell is Off`
    TurnPageOn = 135,
    /// `@OPTEXT@ has changed to a new handle:~`
    NewHandle = 136,
    /// `Bulletins have been updated since last time on, @FIRST@.`
    BulletinsUpdated = 137,
    /// `Insufficient time remaining to download (@OPTEXT@)`
    NoTimeForDownload = 138,
    /// `Insufficient disk space for complete upload!`
    InsufficientUploadSpace = 139,
    /// `Sorry, @FIRST@, uploads are currently disabled!`
    UploadsDisabled = 140,
    /// `7-E-1 Operation in Effect.`
    Ineffect71 = 141,
    /// `DOS function to execute (Enter)=none`
    DOSFunction = 142,
    /// `Enter your logon password below to continue ...`
    EnterLogonPassword = 143,
    /// `Excessive Main Command Line Errors!`
    ExcessiveErrors = 144,
    /// `(Ctrl-K) or (Ctrl-X) Aborts, (Ctrl-S) Suspends.`
    AbortKeys = 145,
    /// `Enter new length (0)=continuous, (Enter)=no change`
    EnterPageLength = 146,
    /// `Message Base Error!  Attempting to continue ...`
    MessageBaseError = 147,
    /// `Password (Dots will echo)`
    YourPassword = 148,
    /// `Do you want graphics (Enter)=no`
    WantGraphics = 149,
    /// `(R)e-enter user's name or (C)ontinue`
    ReEnterUsersName = 150,
    /// `Checking user file - please wait ...`
    CheckingUserFile = 151,
    /// `Password (one word please)`
    NewPassword = 152,
    /// `Checking file transfer request.  Please wait, @FIRST@ ...`
    CheckingFileTransfer = 153,
    /// `(@OPTEXT@) already exists on the system.`
    DuplicateFile = 154,
    /// `Msg Scan: (A)ll, (C)urrent, (S)ince, (Q)uick, (L)ong, (Enter)=abort`
    MessagesCanPrompt = 155,
    /// `Join Group CHAT Session (Enter)=no`
    JoinGroupChat = 156,
    /// `Awaiting Event Timer - All activity suspended ...`
    WaitingForEvent = 157,
    /// `Sts Msg#    Ref#    To              From            Subject`
    FiveScanHeader = 158,
    /// `(@OPTEXT@) Sorry, @FIRST@, download bytes left available are @BYTESLEFT@`
    BytesLeftAre = 159,
    /// `Before beginning, enter a description of: @OPTEXT@`
    EnterDescription = 160,
    /// `Begin description with (/) to make upload `Private'.`
    SlashForPrivate = 161,
    /// `Thanks for the file(s), @FIRST@!`
    ThanksForTheFiles = 162,
    /// `(A) (C) (D) (E) (F) (H) (I) (L) (Q) (S) (SA) (SC) (SK) (SN) (U)`
    MessageCommandExpertmode = 163,
    /// `(A)bort, (C)ont Line Editor, (D)elete, (E)dit, (F)ull Scrn Editor, (H)elp,`
    MessageCommandNovice1 = 164,
    /// `Modem Reset Error ..`
    ModemError = 165,
    /// `Thanks for calling, @FIRST@!`
    ThanksForCalling = 166,
    /// `(A), (C), (D), (F), (L), (P), (Q), (S), (U), (#), (+/-)`
    UsermodeExpertmode = 167,
    /// `A)dd, C)hange, D)el, F)ind, L)ist, P)rint, Q)uit, S)can, U)ndel, (#)`
    UsermodeNoExpert = 168,
    /// `To:~`
    To = 169,
    /// `From:~`
    From = 170,
    /// `Subj:~`
    Subject = 171,
    /// `Unable to write USER Record - Aborting!`
    UnableToWriteUserRecord = 172,
    /// `@OPTEXT@ has changed the topic to:~`
    NewTopic = 173,
    /// `Number of Users Purged:`
    NumberOfUsersPurged = 174,
    /// `Sysop CHAT active at~`
    SysopChatActive = 175,
    /// `Transfer Successful, @FIRST@.`
    TransferSuccessful = 176,
    /// `(@OPTEXT@) is an invalid filename!`
    InvalidFileName = 177,
    /// `(@X0C@TIMELEFT@@X0A min. left) (H)elp, Chat Command`
    ChatPromptExpertmode = 178,
    /// `(@X0C@TIMELEFT@@X0A min. left) (H)elp, Chat Command`
    ChatPromptNovice = 179,
    /// `This channel is now private.  Outsiders cannot join.`
    TopicNowPrivate = 180,
    /// `That channel is private.  You cannot join it at this time.`
    ChannelIsPrivate = 181,
    /// `7-E-1 Operation Not Available!  Call using 8-N-1 Modem Settings!`
    Notavailable7e1 = 182,
    /// `PCBPACK Security Level Fail!`
    PCBPackSecurityLevelFail = 183,
    /// `Reloading PCBoard.  Please wait ...`
    ReloadingPCBoard = 184,
    /// `User Record Number Bad!`
    UserRecordNumberIsBad = 185,
    /// `Press ESC to enter command mode.`
    PressESCForCommand = 186,
    /// `Aborted using~`
    AbortedUsing = 187,
    /// `Completed using~`
    CompletedUsing = 188,
    /// `@OPTEXT@ has made the discussion private.`
    MadeTopicPrivate = 189,
    /// `Ring Detected -~`
    RingDetected = 190,
    /// `Press (Space) to acknowledge Page, (Esc) when done.`
    SpaceToAcknowledge = 191,
    /// `Minutes Used: @OPTEXT@`
    MinutesUsed = 192,
    /// `Subject (Enter)=no change`
    NewSubject = 193,
    /// `Message Security (H)=help`
    MessageSecurity = 194,
    /// `Msg # to Begin Search from (@OPTEXT@)`
    MessageSearchFrom = 195,
    /// `(@TIMELEFT@ min left), (H)elp, More`
    MorePrompt = 196,
    /// `(@TIMELEFT@ min left), (H)elp, End of Message Command`
    EndOfMessage = 197,
    /// `Default Protocol Desired (Enter)=no change`
    DesiredProtocol = 198,
    /// `To (Enter)=`ALL'`
    MessageTo = 199,
    /// `Subject (Enter)=abort`
    MessageSubject = 200,
    /// `Error in filename request!`
    ErrorInFilenameRequest = 201,
    /// `Welcome back to PCBoard~`
    WelcomeBackToPCBoard = 202,
    /// `Creating New Message Index File .`
    CreatingNewMessageIndex = 203,
    /// `Online update of user's record completed.`
    OnlineUpgradeDone = 204,
    /// `Only ASCII and Kermit Protocols supported at 7-E-1!`
    Protocol7E1Error = 205,
    /// `Number of Msgs Purged :`
    NumberOfMsgsPurged = 206,
    /// `Number of Bytes Purged:`
    NumberOfBytesPurged = 207,
    /// `@X07@OPTEXT@ @X0Cnot found in user file.`
    NotInUsersFile = 208,
    /// `Two lines left before message is full, @FIRST@ ...`
    TwoLinesLeft = 209,
    /// `@OPTEXT@ has entered this channel.`
    EnteredChannel = 210,
    /// `@OPTEXT@ has left this channel.`
    LeftChannel = 211,
    /// `(V)iew, (P)rint, (S)can, (D)elete Caller Log, (Enter)=quit`
    ViewCallers = 212,
    /// `(V)iew, (P)rint Users, (Enter)=quit`
    ViewPrintUsers = 213,
    /// `) was not found in line`
    WasNotFoundInLine = 214,
    /// `Carrier Lost ...`
    CarrierLost = 215,
    /// `(Enter) Starts, (Ctrl-X) Aborts`
    EnterStarts = 216,
    /// `(Ctrl-X) Aborts Transfer`
    AbortsTransfer = 217,
    /// `File Size is `0' - Aborting ...`
    FileSizeIsZero = 218,
    /// `Sysop CHAT ended at~`
    SysopChatEnded = 219,
    /// `Edit: (T)o, (F)rom, (S)ubject, Refer (N)um, (R)ead, (P)rotection`
    EditHeader = 220,
    /// ``Echo' Message, (Enter)=@OPTEXT@`
    EchoMessage = 221,
    /// `Text Entry Command`
    TextEntryCommand = 222,
    /// `(H)elp, (1-@NUMDIR@), File List Command`
    FileListCommand = 223,
    /// `(H)elp, (1-@NUMBLT@), Bulletin List Command`
    BulletinListCommand = 224,
    /// `~wants to CHAT with you on Channel @OPTEXT@.`
    WantsToChat = 225,
    /// `Respond by typing (CHAT) (G) and then select channel @OPTEXT@.`
    ToRespondToChat = 226,
    /// `Node CHAT: (G)roup, (U)navailable, (H)elp, (Enter)=none`
    NodeChatUPrompt = 227,
    /// `Node CHAT: (G)roup, (A)vailable, (H)elp, (Enter)=none`
    NodeChatAPrompt = 228,
    /// `Sorry, @FIRST@, Node requested is not available for CHAT.`
    NodeNotAvailable = 229,
    /// `Node CHAT ended at @OPTEXT@`
    NodeChatEnded = 230,
    /// `Node CHAT entered at @OPTEXT@`
    NodeChatEntered = 231,
    /// `Enter a new `topic' for Channel @OPTEXT@`
    TopicForChat = 232,
    /// `Refused to register.`
    RefusedToRegister = 233,
    /// `Enter channel number (1-255), (L)ist channels or (Q)uit chat`
    NewChannel = 234,
    /// `Monitor Mode is now off.`
    MonitorOff = 235,
    /// `Monitor Mode is now on.`
    MonitorOn = 236,
    /// `There are no channels in use at this time.`
    NoChannelsInUse = 237,
    /// `Enter your text. (Enter) alone to end.~`
    MessageEnterText = 238,
    /// `Out to DOS`
    OutToDOS = 239,
    /// `Filename to View (Enter)=none`
    ArchiveViewFileName = 240,
    /// `Error executing View of file (@OPTEXT@)`
    ErrorViewingFile = 241,
    /// `View executed on file (@OPTEXT@)`
    ViewExecutedOnFile = 242,
    /// `Checking View request.  Please wait, @FIRST@ ...`
    CheckingArchiveView = 243,
    /// `Chatting with Sysop`
    ChatWithSysop = 244,
    /// `Default Protocol set to~`
    DefaultProtocol = 245,
    /// `Error reading PCBPROT.DAT!  Aborting ...`
    ErrorInPCBPROTDATFile = 246,
    /// `Answering Script`
    AnswerScript = 247,
    /// `Enter the number of the node to CALL into chat`
    NodeToCall = 248,
    /// `Enter node (#) to ignore, (C)ancel or (W)ho`
    GetIgnoreList = 249,
    /// `Ignore Mode Cancelled`
    IgnoreCancelled = 250,
    /// `This channel is now public.`
    TopicNowPublic = 251,
    /// `Rcvd Broadcast Message`
    ReceivedMessage = 252,
    /// `Loading PCBPack module.  Please wait ...`
    LoadingPackModule = 253,
    /// `PCBPack module not found!  Pack request unsuccessful ...`
    PackModuleMissing = 254,
    /// `Ignoring Nodes:~`
    IgnoringNodes = 255,
    /// `User already exists in the data base.`
    UserExists = 256,
    /// `What is the user's first name`
    UsersFirstName = 257,
    /// `What is the user's last name`
    UsersLastName = 258,
    /// `What is your first name`
    YourFirstName = 259,
    /// `What is your last name`
    YourLastName = 260,
    /// `Silent Mode is now off.`
    SilentOff = 261,
    /// `Silent Mode is now on.`
    SilentOn = 262,
    /// `Send private message to Node (#), (C)ancel or (S)how users`
    SendToNode = 263,
    /// `Please wait - Adding @USER@ to Quick Index File ...`
    AddToIndex = 264,
    /// `City and State calling from`
    CityState = 265,
    /// `Reg. Exp. Date (as mmddyy)`
    ExpiredDate = 266,
    /// `Exp. Reg. Security Level`
    ExpiredSecurityLevel = 267,
    /// `Exp. Reg. Conference Info.`
    ExpiredRegisteredConference = 268,
    /// `Special Comment/Record Info`
    SpecialComment = 269,
    /// `Sorry, @OPTEXT@ bps is not supported ...`
    BaudNotSupported = 270,
    /// `@OPTEXT@ bps is supported from @OFFHOURS@.`
    BaudSupportedFrom = 271,
    /// `System is currently unavailable ...`
    SystemUnavailable = 272,
    /// `@FIRST@, your record has been updated with changes made.`
    UserRecordUpdated = 273,
    /// `Node # to Drop to DOS (Enter)=none`
    NodeNumberToDrop = 274,
    /// `Sorry, @FIRST@, no Bulletins are presently available.`
    NoBulletinsAvailable = 275,
    /// `Sorry, @FIRST@, no DOORS are presently available.`
    NoDOORSAvailable = 276,
    /// `Sorry, @FIRST@, no Directories are presently available.`
    NoDirectoriesAvailable = 277,
    /// `Sorry, @FIRST@, no Conferences are presently available!`
    NoConferenceAvailable = 278,
    /// `---   ---------------------   -----------------------------`
    UsernetUnderline = 279,
    /// `Protocol Type for Transfer, (Enter) or (N)=abort`
    ProtocolForTransfer = 280,
    /// `(@OPTEXT@) is an invalid Conference selection!`
    InvalidConferenceNumber = 281,
    /// `(@OPTEXT@) is an invalid DOOR selection!`
    InvalidDOOR = 282,
    /// `Scanning Directory`
    ScanningDirectory = 283,
    /// `Page Length is currently set to`
    CurrentPageLength = 284,
    /// `Uploads Not Accepted on File Format (@OPTEXT@)`
    BadUploadFormat = 285,
    /// `(#)   Status                  User`
    UserNetHeader = 286,
    /// `Graphics mode is now on, @FIRST@ ...`
    GraphicsOn = 287,
    /// `Graphics mode is now off, @FIRST@ ...`
    GraphicsOff = 288,
    /// `Logoff Information Saved.`
    LogoffInfoSaved = 289,
    /// `Transfer Aborted ...`
    TransferAborted = 290,
    /// `Sorry, @FIRST@, you entered an invalid Bulletin #!`
    InvalidBulletinNumber = 291,
    /// `Sorry, @FIRST@, you entered an invalid Directory #!`
    InvalidFileNumber = 292,
    /// `Event @OPTEXT@ ran at~`
    EventRan = 293,
    /// `Caller Exited to DOS at @OPTEXT@`
    ExitedToDOSAt = 294,
    /// `Viewed Text file (@OPTEXT@)`
    TextFileViewed = 295,
    /// `Scan Message Base Since `Last Read' (Enter)=yes`
    ScanMessageBase = 296,
    /// `New Info`
    NewInfo = 297,
    /// `Enter text of private message below:`
    TextToSend = 298,
    /// `PRIVATE MESSAGE!  @X0BTo respond privately use: SEND @OPTEXT@`
    ReceivedPrivateMessage = 299,
    /// `Filename to Download (Enter)=@OPTEXT@`
    DefaultFileNameToDownload = 300,
    /// `Carbon Copy To (Enter)=none`
    CarbonCopyTo = 301,
    /// `Opened Door (@OPTEXT@) at~`
    OpenedDOOR = 302,
    /// `No Caller this Node`
    NoCaller = 303,
    /// `Available for CHAT`
    Available = 304,
    /// `Out of Code in DOOR`
    InADOOR = 305,
    /// `Auto Logoff Pending`
    LogoffPending = 306,
    /// `DOS Recycle Pending`
    RecycleBBS = 307,
    /// `Unavailable for CHAT`
    Unavailable = 308,
    /// `Transferring a File`
    Transfer = 309,
    /// `Entering a Message`
    EnterMessage = 310,
    /// `CHATTING with Group`
    GroupChat = 311,
    /// `Drop to DOS Pending`
    DropDOSDelayed = 312,
    /// `@OPTEXT@ has made the discussion public.`
    MadeTopicPublic = 313,
    /// `Channel 1 cannot be made private.`
    CantBePrivate = 314,
    /// `Incorrect Download Password for File (@OPTEXT@)`
    BadDownloadPassword = 315,
    /// `Incorrect Upload Password for File (@OPTEXT@)`
    BadUploadPassword = 316,
    /// `Incorrect View Password for File (@OPTEXT@)`
    BadViewPassword = 317,
    /// `Incorrect Password for DOOR (@OPTEXT@)`
    BadPasswordForDOOR = 318,
    /// `That node is not in group chat!`
    NodeNotInChat = 319,
    /// `Preparing file(s) for download.  Please wait.`
    AnnounceCopy = 320,
    /// `Bulletin Read:~`
    BullettinRead = 321,
    /// `Input File Name`
    InputFileName = 322,
    /// `File Selected: ~`
    FileSelected = 323,
    /// `Protocol Type: ~`
    ProtocolType = 324,
    /// `Select Conference(s)`
    SelectConferences = 325,
    /// `(@OPTEXT@) not found on disk!`
    NotFoundOnDisk = 326,
    /// `I'm back!  Thanks for waiting~`
    ThanksForWaiting = 327,
    /// `@OPTEXT@ Joined`
    ConferenceJoined = 328,
    /// `Local password`
    LocalPassword = 329,
    /// `Enter the Message # to Kill (Enter)=none`
    MessageNumberToKill = 330,
    /// `Punctuation Error:  Quotes may be necessary around the search criteria.`
    PunctuationError = 331,
    /// `Back from DOS at @OPTEXT@`
    BackFromDOS = 332,
    /// `Hello, this is @OPTEXT@.~~`
    HelloThisIs = 333,
    /// `Message Restored:~`
    MessageRestored = 334,
    /// `Message Killed:~`
    MessageKilled = 335,
    /// `Comment Left:~`
    CommentLeft = 336,
    /// `Message Left:~`
    MessageLeft = 337,
    /// `Saving Comment #`
    SavingComment = 338,
    /// `Saving Message #`
    SavingMessage = 339,
    /// `(Enter) continues with display`
    MorehelpEnter = 340,
    /// `(Y) yes, continue with display`
    MorehelpYes = 341,
    /// `(N) no, stop displaying this text`
    MorehelpNo = 342,
    /// `(NS) continue reading in non-stop mode`
    MorehelpNonstop = 343,
    /// `Drop to DOS - Urgent`
    DropDOSNow = 344,
    /// `Force logoff now (`N'=wait until caller logs off, then drop to DOS)`
    DropNow = 345,
    /// `(V) View a file, then continue displaying files`
    MorehelpView = 346,
    /// `# Msgs Found:`
    TotalMessagesFound = 347,
    /// `Node # to Recycle through DOS (Enter)=none`
    RecycleThruDOS = 348,
    /// `Password Failure (@OPTEXT@)`
    PasswordFailure = 349,
    /// `Expert mode is now on, @FIRST@ ...`
    ExpertmodeOn = 350,
    /// `Expert mode is now off, @FIRST@ ...`
    ExpertmodeOff = 351,
    /// `Files: (1-@NUMDIR@), (A), (U), (Enter)=none`
    FileNumberExpertmode = 352,
    /// `Files: (1-@NUMDIR@), (A)ll, (U)ploads, (Enter)=none`
    FileNumberNovice = 353,
    /// `Alt-> F=Out I=In N=Next X=DOS P=Printer T=FmFeed F1/F2=Time F9/F10=Security`
    HelpLine1 = 354,
    /// `1=SyPrv 2=LkOut 3=Print 4=Bell 5=SHELL 6=Reg 7=Alarm 8=HngUp 9=Screen 10=Chat`
    HelpLine2 = 355,
    /// `Network Delay - Please Wait, @FIRST@ ...`
    NetworkDelay = 356,
    /// `Download Time:~`
    DownloadTime = 357,
    /// `Download Size:~`
    DownloadSize = 358,
    /// `Total Will Be:~`
    TotalWillBe = 359,
    /// `Upload Drive :`
    UploadDrive = 360,
    /// `Bytes Free Disk Space`
    FreeDiskSpace = 361,
    /// `Thread Read Terminated ...`
    ThreadReadTerminated = 362,
    /// `Message Number Memorized ...`
    MessageNumberMemorized = 363,
    /// `Message Text Search Terminated ...`
    MessageSearchTerminated = 364,
    /// `Insufficient memory for CHAT buffers.`
    InsufficientMEMForChat = 365,
    /// `Upload Status:`
    UploadStatus = 366,
    /// `Screened Before Posting`
    ScreenEditor = 367,
    /// `Posted Immediately`
    PostedImmediately = 368,
    /// `Caller Num.:`
    CallerNumber = 369,
    /// `Unable to open CHAT file.`
    CantOpenChatFile = 370,
    /// `Lst Date On:~`
    LastDateOne = 371,
    /// `Expire Date:~`
    ExpiredAt = 372,
    /// `# Times On :`
    NumberTimesOn = 373,
    /// `Page Length:`
    PageLength = 374,
    /// `Expert Mode: On`
    ExpertmodeModeOn = 375,
    /// `Expert Mode: Off`
    ExpertmodeModeOff = 376,
    /// `Security Lv:`
    SecurityLevel = 377,
    /// `# Downloads:`
    NumberDownloads = 378,
    /// `# Uploads  :`
    NumberUploads = 379,
    /// `Bytes Avail:`
    BytesAvailable = 380,
    /// `L/Msg. Read:`
    LastMessageRead = 381,
    /// `High Msg. #:`
    HighMessageNumber = 382,
    /// `Active Msgs:`
    NumberActiveMessages = 383,
    /// `Tr/Protocol:~`
    TransferProtocol = 384,
    /// `Operational Languages Available:`
    LanguageAvailable = 385,
    /// `Alternative Language Operation Not Available.`
    LanguageALTNotAvailable = 386,
    /// `Enter Language # to use (Enter)=no change`
    LanguageEnterNumber = 387,
    /// `English Language is now active.`
    LanguageActive = 388,
    /// `Language Requested Not Available.`
    LanguageNotAvailable = 389,
    /// `minutes (approximate)`
    Minutes = 390,
    /// `SHELL Batch file Missing (`
    ShellBatchMissing = 391,
    /// `Resetting Packet ...`
    ResetPacket = 392,
    /// `Resetting Modem ...`
    ResetModem = 393,
    /// `System is Ready For Callers`
    SystemAvailable = 394,
    /// `min. left)`
    MinutesLeft = 395,
    /// `(@X0C@TIMELEFT@@X0E min. left) @INCONF@Command`
    CommandPrompt = 396,
    /// `Scanning~`
    Scanning = 397,
    /// `Scanning Main (0)`
    ScanningMain = 398,
    /// `Conference`
    Conference = 399,
    /// `Main Board`
    Mainboard = 400,
    /// `Error in Chat File format, aborting...`
    ChatFormatError = 401,
    /// `Found: @X07@OPTEXT@`
    FoundName = 402,
    /// `Abort your entry (Enter)=no`
    MessageAbort = 403,
    /// `Text Entry Aborted ...`
    MessageAborted = 404,
    /// `Delete Line #`
    DeleteLineNumber = 405,
    /// `Sorry, @FIRST@, unable to Insert another line!`
    CanNotInsert = 406,
    /// `Before Line #`
    InsertBeforeNumber = 407,
    /// `Which line do you want to edit, @FIRST@`
    EditLineNumber = 408,
    /// `Operator Paged at~`
    OperatorPaged = 409,
    /// `Hanging Up Phone ...`
    HangingUpPhone = 410,
    /// `Going Off-Hook ...`
    GoingOffHook = 411,
    /// `@OPTEXT@ Abandoned`
    ConferenceAbandoned = 412,
    /// `(@OPTEXT@) Menu Selection is not available, @FIRST@.`
    MenuSelectionUnavailable = 413,
    /// `(@OPTEXT@) Enter the password for uploading`
    PasswordForUpload = 414,
    /// `Enter password to open the DOOR`
    PasswordForDOOR = 415,
    /// `(@OPTEXT@) Enter the password for viewing`
    PasswordForView = 416,
    /// `(@OPTEXT@) Enter the password for downloading`
    PasswordForDownload = 417,
    /// `Press (Enter) to continue`
    PressEnter = 418,
    /// `Insufficient security for DOOR (@OPTEXT@)`
    InsufficientSecurityForDOOR = 419,
    /// `Edit: (T)o, (F)rom, (S)ubject, Refer (N)um, (R)ead, (P)rotection, (E)cho`
    EditHeaderEcho = 420,
    /// `Sorry, @FIRST@, no NEWS file is available!`
    NoNews = 421,
    /// `Msgs For You:~`
    MessagesForYou = 422,
    /// `Msgs From You:~`
    MessagesFromYou = 423,
    /// `(H)elp, (@OPTEXT@), Message Scan Command`
    MessagesCanCommand = 424,
    /// `(H)elp, (@OPTEXT@), Message Read Command`
    MessageReadCommand = 425,
    /// `No message number has been memorized!`
    NotMemorized = 426,
    /// `The user may be busy.  Attempt to CALL into chat anyway`
    CallAnyway = 427,
    /// `Insufficient Download Security for (@OPTEXT@)`
    InsufficientSecurityToDownloadFile = 428,
    /// `Caller Log Viewed`
    CallersLogViewed = 429,
    /// `Directory Scan for (@OPTEXT@)`
    DirectoryScan = 430,
    /// `User File Viewed`
    UsersFileViewed = 431,
    /// `Sorry, @FIRST@, Insufficient Security for `From' edit!`
    InsufficientSecurityForFromEdit = 432,
    /// `This system is set to test uploaded files.  This can be a lengthy process.`
    BeginUploadTest = 433,
    /// `(H)ang up now, or press any other key to remain online...`
    HangupNowOrWait = 434,
    /// `Hanging up now.  Thank you for the upload(s).`
    HangingUpNow = 435,
    /// `has exited to DOS.  Please wait ...`
    SysopExitedToDOS = 436,
    /// `Upload aborted ...`
    UploadAborted = 437,
    /// `Enter (Oldtext;Newtext) or (Enter) alone for `no change'.`
    OldTextNewText = 438,
    /// `Date:                               Number:`
    MessageDateNumber = 439,
    /// `To:`
    MessageToLine = 440,
    /// `HAS REPLIES`
    MessageReplies = 441,
    /// `From:`
    MessageFrom = 442,
    /// `(N/A)`
    MessageNA = 443,
    /// `NO`
    MessageNotRead = 444,
    /// `RECEIVER ONLY`
    MessageReceiverOnly = 445,
    /// `GROUP PASSWORD`
    MessageGroupPassword = 446,
    /// `SENDER PASSWORD`
    MessageSenderPassword = 447,
    /// `PUBLIC MESSAGE`
    MessagePublic = 448,
    /// `Uploading will lose the Flagged Files list.  Continue with upload`
    ContinueUpload = 449,
    /// `(Error Correcting Modem Detected)`
    ErrorCorrecting = 450,
    /// `Refer#`
    MessageReferNumber = 451,
    /// `Read:`
    MessageRead = 452,
    /// `Subj:`
    MessageSubjectLine = 453,
    /// `Not Using Sysop Record #1 - Pack not Available!`
    NotRecNumberOne = 454,
    /// `Status:`
    MessageStatus = 455,
    /// `Pack Message Base Before Continuing!`
    PackMessageBaseBeforeContinuing = 456,
    /// `No Security Level Match in PWRD File!`
    NoMatchingPassword = 457,
    /// `Free Msg #s:`
    NumberFreeMessages = 458,
    /// `Low Msg. # :`
    LowMessageNumber = 459,
    /// `Defined #'s:`
    NumberDefinedMessages = 460,
    /// `Generate ONLY a New Index File (Enter)=no`
    GenerateNewIndex = 461,
    /// `Assign More Message BLOCKS using PCBSetup!`
    AssignMoreMessageBlocks = 462,
    /// `Chat Echo disabled.  Use the ECHO command to re-enable it.`
    EchoDisabled = 463,
    /// `Chat Echo enabled.`
    EchoEnabled = 464,
    /// `Move Message to what Conference #`
    MovedMessageToConference = 465,
    /// `/ FILE`
    MessageFile = 466,
    /// `Message Moved: #`
    MessageMoved = 467,
    /// `/ LIST`
    MessageList = 468,
    /// `Recent Uploads`
    RecentUploadS = 469,
    /// `(Esc) Exit, (↑) (↓) (PgUp) (PgDn) (Home) (End) Scrolls, (Space) Tags/Untags`
    ScrollKeys = 470,
    /// `Scroll Lines Above Cursor:`
    ScrollAbove = 471,
    /// `Scroll Lines Below Cursor:`
    ScrollBelow = 472,
    /// `(Generic Message)`
    GenericMessage = 473,
    /// `(G)oodbye after Batch, (A)bort, Change (P)rotocol, (Enter)=continue`
    GoodbyeAfterUpload = 474,
    /// `(Ready to Send in Batch Mode)`
    ReadyToSendBatch = 475,
    /// `The file received is wider than 79 characters.`
    MessageTooWide = 476,
    /// `(S)ave or (A)bort message`
    MessageTooWideSave = 477,
    /// `Batch Transfer Ended.`
    BatchTransferEnded = 478,
    /// `Output File Name`
    OutputFileName = 479,
    /// `Batch Download Time:~`
    BatchDownloadTime = 480,
    /// `Batch Download Size:~`
    BatchDownloadSize = 481,
    /// `Batch Protocol Type: ~`
    BatchProtocol = 482,
    /// `(A)ll Abort Detected. Continue with scan (Enter)=yes`
    ResumeAll = 483,
    /// `Conf:                            Read Type: GENERAL`
    MessagesGeneral = 484,
    /// `Conf:                            Read Type: THREAD READ`
    MessagesThread = 485,
    /// `Conf:                            Read Type: TEXT SCAN`
    MessagesTextScan = 486,
    /// `Conf:                            Read Type: MAIL FOR YOU`
    MessagesReadForYou = 487,
    /// `Conf:                            Read Type: MAIL FROM YOU`
    MessagesReadFromYou = 488,
    /// `Conf:                            Read Type: TO OR FROM YOU`
    MessagesReadToOrFrom = 489,
    /// `(G)oodbye after Download, (A)bort Download (Enter)=continue`
    ByeAfterDownload = 490,
    /// `Attached File: @X07@OPTEXT@~~`
    AttachedFile = 491,
    /// `Capture Aborted.  Read Pointers Restored ...`
    PointersRestored = 492,
    /// `Goodbye in 10 seconds. (H)ang up now or (Ctrl-K) to cancel`
    ByeIn10Seconds = 493,
    /// `Capture file is empty - nothing to download.`
    CaptureFileIsEmpty = 494,
    /// `Total Messages Captured for Download ->`
    TotalMessagesInCapture = 495,
    /// `Sending File(s) - Start your download ...`
    SendingFiles = 496,
    /// `Logging into System`
    LogIntoSystem = 497,
    /// `Use Full Screen Editor`
    UseFullScreen = 498,
    /// `Full Screen Editor requires ANSI capability`
    RequiresAnsi = 499,
    /// `Download Flagged Files`
    DownloadTagged = 500,
    /// `New Bulletin(s):`
    NewBulletins = 501,
    /// `Attempted to log on as:~`
    NamesTried = 502,
    /// `Perform (S)oundex Search, (R)e-enter User's Name or (C)ontinue`
    DoSoundexSearch = 503,
    /// `(U)se Found Name, (S)earch for more, (R)e-enter Name or (C)ontinue`
    UseFoundName = 504,
    /// `QUOTE: Starting line number, (Q) to Quit`
    QuoteStart = 505,
    /// `QUOTE: Ending line number, (Q) to Quit`
    QuoteEnd = 506,
    /// `Compressing capture file, please wait...`
    Compressing = 507,
    /// `Insufficient Memory for Operation @OPTEXT@`
    InsufficientMemory = 508,
    /// `Sysop - Not Busy`
    SysopNotBusy = 509,
    /// `User - Not Busy`
    UserNotBusy = 510,
    /// `PCBSysMgr`
    PCBSysMgr = 511,
    /// `PCBFiler`
    PCBFiler = 512,
    /// `PCBSetup`
    PCBSetup = 513,
    /// `SYSTEM Statistics`
    SetStatsLocal = 514,
    /// `PCBMoni`
    PCBMonitor = 515,
    /// `Reset Stats`
    ResetStats = 516,
    /// `No Printer`
    NoPrinter = 517,
    /// `Printer is On`
    TurnPrinterOff = 518,
    /// `Alarm is On`
    TurnAlarmOff = 519,
    /// `Page Bell is On`
    TurnPageOff = 520,
    /// `Log in as a regular user.  Callers will get a busy signal.`
    UserBusyDescription = 521,
    /// `Log in as a regular user.  RING Alert will be activated.`
    UserNotBusyDescription = 522,
    /// `Toggle printer off or on.  When on, caller log is echoed to printer.`
    TogglePrinterDescription = 523,
    /// `Run PCBoard System Manager for User File maintenance.`
    PCBSysmgrDescription = 524,
    /// `Choose between System-Wide and Node-Specific Statistics Display`
    SetStatsDescription = 525,
    /// `Log in as the Sysop.  Callers will get a busy signal.`
    SysopBusyDescription = 526,
    /// `Log in as the Sysop.  RING Alert will be activated.`
    SysopNotBusyDescription = 527,
    /// `Toggle `Page Bell' on or off.  System will BEEP when caller pages you.`
    TogglePageDescription = 528,
    /// `Run PCBFiler for File Directory maintenance.`
    PCBFilerDescription = 529,
    /// `Run PCBMoni to monitor NODE activity`
    PCBMoniDescription = 530,
    /// `Drop to DOS.  Callers will get a busy signal.`
    DOSBusyDescription = 531,
    /// `Drop to DOS.  Callers will NOT get a busy signal.  Phone will ring!`
    DOSNotBusyDescription = 532,
    /// `Toggle `Caller Alarm' on or off.  System BEEPs as caller logs on, etc.`
    ToggleAlarmDescription = 533,
    /// `Run PCBSetup to change PCBoard configuration.`
    PCBSetupDescription = 534,
    /// `Reset Node/System Statistics to Zero`
    ResetStatsDescription = 535,
    /// `No Modem Selected`
    NoModemSelected = 536,
    /// `Last Caller:`
    LastCaller = 537,
    /// `Calls:`
    NumberCalls = 538,
    /// `Msgs:`
    NumberMessages = 539,
    /// `D/Ls:`
    NumberDownload = 540,
    /// `U/Ls:`
    NumberUpload = 541,
    /// `Press (Esc) to Exit  (Ctrl-Z) for Help`
    EscToExit = 542,
    /// `(Ins/Ctrl-V) Mode: Overwrite`
    INSForInsert = 543,
    /// `(Ins/Ctrl-V) Mode: Insert`
    INSForOverwrite = 544,
    /// `(@OPTEXT@), Set your Last Message Read to`
    SetLastMessageReadPointer = 545,
    /// `Last Message Read now set to @OPTEXT@.`
    LastMessageReadSetTo = 546,
    /// `Unlimited`
    Unlimited = 547,
    /// `(F) Flag a file for later download, then continue displaying files`
    MorehelpFlag = 548,
    /// `(@TIMELEFT@ min left), (H)elp, (V)iew, (F)lag, More`
    FilesMorePrompt = 549,
    /// `(A)bort, (E)dit, (G)oodbye, (L)ist, (P)rotocol, (Enter)=continue`
    GoodbyeAfterDownload = 550,
    /// `(A)dd a File, (R)emove a File, (L)ist Batch, (Enter)=continue`
    EditBatch = 551,
    /// `(@OPTEXT@), Remove file number(s)`
    RemoveFileNumber = 552,
    /// `Removed from Batch: @OPTEXT@`
    RemovedFile = 553,
    /// `(@OPTEXT@) duplicates a file already in the batch!`
    DuplicateBatchFile = 554,
    /// `Batch limit reached.  @OPTEXT@ was not added to the batch.`
    BatchLimitReached = 555,
    /// `Clear the screen between each message`
    CLSBetweenMessages = 556,
    /// `Enter the filename to flag for download (Enter)=none`
    FlagForDownload = 557,
    /// `User Name                  Location              Last On`
    UsersHeader = 558,
    /// `Enter text to scan for (ENTER alone finds all registered users)`
    UserScan = 559,
    /// `LOCAL Statistics`
    SetStatsSystem = 560,
    /// `Delete this record`
    DeleteRecord = 561,
    /// `Enter Conference Numbers, (S)elect All, (D)eselect All or (Q)uit`
    ConferenceNumbers = 562,
    /// `Enter Conference Numbers or (Q)uit`
    ConferenceNumbers2 = 563,
    /// `Conf. Flags (R=Reg, X=Exp Reg, S=Scan, C=Conf Sysop, N=Net Status)`
    SelectConferenceFlags = 564,
    /// `Selecting ALL Conferences, Please Wait`
    SelectingAll = 565,
    /// `@FIRST@, you have new mail waiting in the following conference(s):`
    MailWaitingIn = 566,
    /// `Read messages TO or FROM (enter user's name)`
    UserSearchName = 567,
    /// `Both First and Last Names are required!`
    RequireTwoNames = 568,
    /// `Copy Message to what Conference #`
    CopyMessageToConference = 569,
    /// `Message Copied: #`
    MessageCopied = 570,
    /// `Would you like to leave a comment to the Sysop instead`
    CommentInstead = 571,
    /// `FREE`
    FreeDownload = 572,
    /// `/ NO TIME`
    NoTimeCharge = 573,
    /// `Enter the filename to Test (Enter)=none`
    TestFileName = 574,
    /// `Verifying @OPTEXT@...`
    VerifyingFile = 575,
    /// `passed.`
    Passed = 576,
    /// `FAILED!`
    Failed = 577,
    /// `Verification FAILED on @OPTEXT@!`
    FileVerifyFailed = 578,
    /// `Paging the Sysop`
    PagingSysop = 579,
    /// `Conference                                                          To   Total`
    ScanHeader1 = 580,
    /// `#   Name                                                          You  Found`
    ScanHeader2 = 581,
    /// `----- ------------------------------------------------------------ ----- -----`
    ScanHeader3 = 582,
    /// `Full Screen Editor Default: (Y)es, (N)o, (A)sk`
    SetFSEDefault = 583,
    /// `(H)elp, (@OPTEXT@), Message Read Command`
    MessageReadCommandExpert = 584,
    /// `(H)elp, (1-@NUMDIR@), File List Command`
    FileListCommandExpert = 585,
    /// `Conference                                                   Last   High`
    ConferenceHeader1 = 586,
    /// `#   Name                                                   Read   Num. Flags`
    ConferenceHeader2 = 587,
    /// `CTTY mode is now on, @FIRST@ ...`
    CTTYOn = 588,
    /// `ANSI mode is now on, @FIRST@ ...`
    AnsiOn = 589,
    /// `Modem: @OPTEXT@`
    Modem = 590,
    /// `Caller Number: @OPTEXT@`
    CallerNumberOpt = 591,
    /// `Caller Security: @OPTEXT@`
    CallerSecurity = 592,
    /// `of`
    Separator = 593,
    /// `Loading @OPTEXT@, please wait...`
    OpeningDOOR = 594,
    /// `Did you forget your password?`
    WrongPassword = 595,
    /// `Would you like to leave a comment to the sysop before logging off`
    WrongPasswordComment = 596,
    /// `COMMENT: PASSWORD FAILURE`
    WrongPasswordSubject = 597,
    /// `Is this correct`
    IsThisCorrect = 598,
    /// `Please try using a different name.  If desired, try adding a middle initial.`
    ChangeNames = 599,
    /// `Lock caller out and disconnect`
    LockCallerOut = 600,
    /// `Disconnect caller`
    DisconnectNow = 601,
    /// `Give caller sysop privileges`
    GiveSysopPrivate = 602,
    /// `Warning:  You have files flagged for download!`
    FilesAreFlagged = 603,
    /// `Proceed with running the @OPTEXT@ door`
    ContinueDOOR = 604,
    /// `Proceed with logoff`
    ContinueLogoff = 605,
    /// `@INCONF@has been selected.`
    Selected = 606,
    /// `@INCONF@has been deselected.`
    DeSelected = 607,
    /// `Searching for @OPTEXT@`
    SearchingFor = 608,
    /// `Upload Credits: 0.0 minutes, 0 bytes`
    UploadCredits = 609,
    /// `Could not find @X07@OPTEXT@ @X0Cin the user file.`
    CouldntFindInUsers = 610,
    /// `(H)elp, (1-@NUMBLT@), Bulletin List Command`
    BulletinListCommandExpertmode = 611,
    /// `(@TIMELEFT@ min left), (H)elp, End of Message Command`
    EndOfMessageExpertmode = 612,
    /// `(H)elp, (@OPTEXT@), Message Scan Command`
    MessagesCanCommandExpertmode = 613,
    /// `(maximum per-conference limit reached)`
    MaxMessagesPerConference = 614,
    /// `(maximum message limit reached)`
    MaxMessages = 615,
    /// `(Rcvd Files: @RFILES@  Bytes: @RBYTES@  CPS: @RCPS@  Combined CPS: @BICPS@)`
    BiReceive = 616,
    /// `Batch Upld: @RFILES@  Bytes: @RBYTES@  Avg. CPS: @RCPS@ (@BICPS@)`
    BiReceiveLog = 617,
    /// `Batch Upld: @RFILES@  Bytes: @RBYTES@  Avg. CPS: @RCPS@`
    ReceiveLog = 618,
    /// `Batch Dnld: @SFILES@  Bytes: @SBYTES@  Avg. CPS: @SCPS@`
    SendLog = 619,
    /// `~~(Files: @SFILES@  Bytes: @SBYTES@  Avg. CPS: @SCPS@)`
    BatchSend = 620,
    /// `~~(Files: @RFILES@  Bytes: @RBYTES@  Avg. CPS: @RCPS@)`
    BatchReceive = 621,
    /// `~~(Avg. CPS: @SCPS@)`
    SendAverageCPS = 622,
    /// `~~(Avg. CPS: @RCPS@)`
    ReceiveAverageCPS = 623,
    /// `Additional Commands: (@X0FFLAG@X0A) for download, (@X0FV@X0A)iew file.`
    ShowAttachCommands = 624,
    /// `Size: @X07@OPTEXT@`
    FileSize = 625,
    /// `That handle is not allowed, please pick another!`
    PickAnotherHandle = 626,
    /// `Scroll multi-screen messages`
    ScrollMessageBody = 627,
    /// `Use long headers when reading messages`
    UseBigHeaders = 628,
    /// `(I)nsert, (L)ist Msg, (Q)uote, (S)ave, (SA)Save Attach, (U)pload Msg`
    MessageCommandNovice2 = 629,
    /// `Request Return Receipt`
    RequireReturnReceipt = 630,
    /// `Return Receipt Requested.~~`
    ReturnReceiptRequired = 631,
    /// `Generating Receipt, please wait...`
    GenerateReceipt = 632,
    /// `Your message number   : @X07`
    YourMessageNumber = 633,
    /// `Which was addressed to: @X07`
    AddressedTo = 634,
    /// `Was received/read on  : @X07`
    ReceivedOn = 635,
    /// `Route message to`
    RoutedTo = 636,
    /// `Set editor workspace default to 79 columns`
    DefaultWideMessages = 637,
    /// `This conference is READ-ONLY!`
    ConferenceIsReadOnly = 638,
    /// `Private messages are not allowed in this conference.`
    NoPrivateMessages = 639,
    /// `Conference Password`
    PasswordToJoin = 640,
    /// `The message already contains a File Attachment!`
    AlreadyAttached = 641,
    /// `Carbon List To (Enter)=none`
    CarbonList = 642,
    /// `cc: @OPTEXT@`
    CarbonName = 643,
    /// `Read messages TO (enter user's name)`
    UserSearchToName = 644,
    /// `Read messages FROM (enter user's name)`
    UserSearchFromName = 645,
    /// `The File Attachment is missing from disk and cannot be flagged.`
    AttachmentMissing = 646,
    /// `File Attachments are not allowed in this conference.`
    AttachNotAllOWed = 647,
    /// `Carbon Copy List maximum reached!`
    CarbonLimitReached = 648,
    /// `Errors encountered, PPE aborted...`
    ErrorsInPPE = 649,
    /// `Unable to allocate memory (@OPTEXT@).`
    NoMemoryForPPE = 650,
    /// `Invalid token encountered (@OPTEXT@).`
    InvalidTokenInPPE = 651,
    /// `Error occurred executing PPE (@OPTEXT@).`
    ErrorExecPPE = 652,
    /// `An error occurred evaluating an expression.`
    EvalErrorInPPE = 653,
    /// `Error loading PPE (@OPTEXT@) from disk.`
    ErrorLoadingPPE = 654,
    /// `Error appending (@OPTEXT@) to answer file.`
    AppendErrorInPPE = 655,
    /// `Error deleting file (@OPTEXT@).`
    DeleteErrorInPPE = 656,
    /// `Updating index file, please wait...`
    UpdatingIndex = 657,
    /// `(72 chars/line, @OPTEXT@ lines maximum)`
    Columns72 = 658,
    /// `(79 chars/line, @OPTEXT@ lines maximum)`
    Columns79 = 659,
    /// `(45 chars/line, @OPTEXT@ lines maximum)`
    Columns45 = 660,
    /// `(Carbon List)`
    CarbonListMessage = 661,
    /// `Read your personal mail now: (Y)es, (N)o, (A)ll new mail`
    ReadMailNow = 662,
    /// `Scanning for new mail...`
    ScanningForMail = 663,
    /// `You have new mail waiting.  Please read it NOW.`
    ForceReadMail = 664,
    /// `Scanning for new bulletins...`
    ScanningBulletins = 665,
    /// `Your current Download:Upload file ratio is  : @X0F@FILERATIO@`
    FileRatio = 666,
    /// `Your current Download:Upload byte ratio is  : @X0F@BYTERATIO@`
    ByteRatio = 667,
    /// `Your Download:Upload ratio must remain below: @X0F@OPTEXT@`
    RatioLimit = 668,
    /// `Downloading @OPTEXT@ would exceed your file ratio.`
    FileRatioExceeded = 669,
    /// `Downloading @OPTEXT@ would exceed your byte ratio.`
    ByteRatioExceeded = 670,
    /// `Number of files downloaded so far: @X0F@DLFILES@`
    FilesDownloaded = 671,
    /// `Number of bytes downloaded so far: @X0F@DLBYTES@`
    BytesDownloaded = 672,
    /// `Your download limit is set at    : @X0F@OPTEXT@`
    DownloadLimit = 673,
    /// `Downloading @OPTEXT@ would exceed your file limit.`
    FileLimitExceeded = 674,
    /// `Downloading @OPTEXT@ would exceed your byte limit.`
    ByteLimitExceeded = 675,
    /// `Creating QWK Packet, please wait...`
    CreatingQWK = 676,
    /// `Error compressing file.`
    ErrorCompressing = 677,
    /// `QWK Commands: (D)ownload, (U)pload`
    QWKCommands = 678,
    /// `Extracting messages, please wait...`
    ExtractingMessages = 679,
    /// `Error extracting messages.`
    ErrorExtracting = 680,
    /// `Invalid menu option type encountered (@OPTEXT@)`
    InvalidMenuOption = 681,
    /// `@OPTEXT@ Menu Command ('MENU' for options)`
    MenuCommand = 682,
    /// `Running Event`
    RunningEvent = 683,
    /// `Event finished at~`
    EventFinished = 684,
    /// `Message(s) successfully inserted.`
    ReplySuccessful = 685,
    /// `Message insertion failed.`
    ReplyFailed = 686,
    /// `Files matching (@OPTEXT@) are not allowed on this system.`
    FileNotAllowed = 687,
    /// `Date when message should be Packed-Out (mmddyy)`
    EnterPackDate = 688,
    /// `Message Pack Date: @OPTEXT@`
    DateToPackOut = 689,
    /// `Alias Name (enter=no change)`
    GetAliasName = 690,
    /// `Hiding identity change, please wait...`
    HidingIdentity = 691,
    /// `Changed name to @OPTEXT@.`
    ChangedNameTo = 692,
    /// `Your true identity is protected while you remain in this conference.`
    IdentityProtected = 693,
    /// `Changed alias to @OPTEXT@.`
    ChangedAliasTo = 694,
    /// `Attempted to use @OPTEXT@ as an alias.`
    AttemptedAlias = 695,
    /// `Byte Ratio :~`
    ShowByteRatio = 696,
    /// `File Ratio :~`
    ShowFileRatio = 697,
    /// `Please enter your Address (2 lines provided for Street Address):`
    EnterAddress = 698,
    /// `Street (1 of 2)`
    Street1 = 699,
    /// `Street (2 of 2)`
    Street2 = 700,
    /// `City`
    City = 701,
    /// `State`
    State = 702,
    /// `Zip Code`
    Zip = 703,
    /// `Country`
    Country = 704,
    /// `You have already used that password.  Please pick another one.`
    PreviouslyUsedPassword = 705,
    /// `Enter your mother's maiden name`
    EnterVerifyText = 706,
    /// `For verification, enter your mother's maiden name`
    EnterCompareText = 707,
    /// `Password too short!  Must be @OPTEXT@ characters or longer, try again.`
    PasswordTooShort = 708,
    /// `Your password cannot be a subset of your name, try again.`
    NeedUniquePassword = 709,
    /// `Your password has expired.  Please enter a new password now.`
    PasswordExpired = 710,
    /// `Your password will expire in @OPTEXT@ days.  Use the (W) command to change it.`
    PasswordWillExpired = 711,
    /// `Date as (mmddyy) to search from (Enter)=`
    MessagesDateTOSearch = 712,
    /// `Return Receipt:~`
    ReceiptLeft = 713,
    /// `Forwarded From: @X07@OPTEXT@`
    ForwardFrom = 714,
    /// `Forwarded By  : @X07@OPTEXT@`
    ForwardBY = 715,
    /// `Insufficient Credits!  Credits Needed @OPTEXT@, Credits Left @CREDLEFT@.`
    InsufficientCredits = 716,
    /// `Credits Used: @CREDNOW:9R@`
    CreditsUsed = 717,
    /// `Credits Left: @CREDLEFT:9R@`
    CreditsLeft = 718,
    /// `You have exceeded your credit limit!`
    CreditExceeded = 719,
    /// `Your security level has been changed to @SECURITY@.`
    SecurityChanged = 720,
    /// `~NO COST`
    NoCost = 721,
    /// `RETURN RECEIPT`
    FromReturnReceipt = 722,
    /// `COMMENT`
    Comment = 723,
    /// `Error in FROM name: @OPTEXT@`
    ErrorInFromName = 724,
    /// `~Msg#    Ref#    To              From            Subject`
    QuickScanHeader = 725,
    /// `FILE FAILED VERIFICATION CHECK`
    FileFailed = 726,
    /// `Uploaded by: @OPTEXT@`
    UploadedBy = 727,
    /// `(@FNUM@) Enter the filename to Download (Enter)=none`
    FileNameToDownloadBatch = 728,
    /// `(@FNUM@) Enter the Filename to Upload (Enter)=none`
    FileNameToUploadBatch = 729,
    /// `------------------------  ------------------------  ---------------`
    UserScanLine = 730,
    /// `(Net)~`
    NetStatus = 731,
    /// `Set Message Capture Limit (System Max=@OPTEXT@)`
    PersonalMessageLimit = 732,
    /// `Set Per-Conference Message Capture Limit (System Max=@OPTEXT@)`
    PersonalConferenceLimit = 733,
    /// `Maximum Size to Auto-Include Personal Attachments in QWK Packet`
    PersonalQWKLimit = 734,
    /// `Maximum Size to Auto-Include Attachments in QWK Packet`
    PublicQWKLimit = 735,
    /// `Post-to Groups`
    DestNewsGroup = 736,
    /// `Follow-up Groups`
    FollowupNewsGroup = 737,
    /// `Handling Mail`
    HandlingMail = 738,
    /// `(@TIMELEFT@ min left), (V)iew, (F)lag, (S)how, More`
    FilesShowPrompt = 739,
    /// `Enter DIR Command`
    EnterDirCommand = 740,
    /// `No files found.`
    NoFilesFound = 741,
    /// `Directory of @OPTEXT@`
    DirectoryOf = 742,
    /// `Short Description Mode in effect`
    ShortInEffect = 743,
    /// `Long Description Mode in effect`
    LongInEffect = 744,
    /// `Show long description of what file`
    ShowLongDescription = 745,
    /// `Set default file description to SHORT (one-line) format`
    UseShortDescription = 746,
    /// `Enter your gender (M/F)`
    EnterGender = 747,
    /// `Enter your birthdate`
    EnterBirthdate = 748,
    /// `Enter your E-mail address`
    EnterEmail = 749,
    /// `Enter your WEB address`
    EnterWebAddress = 750,
    /// `Enter color (B)lue,(G)reen,(C)yan,(R)ed,(M)agenta,(Y)ellow,(W)hite,(+/-)`
    EnterColor = 751,
    /// `Enter node # or Handle`
    EnterNode = 752,

    /// `Conf:                                 Area:`
    MessagesConfArea = 753,
}

const LAST_ENTRY: usize = 753;

impl IceText {
    pub fn from(i: usize) -> Self {
        unsafe { std::mem::transmute(i) }
    }
}

#[derive(Error, Debug)]
pub enum TextError {
    #[error("invalid file size")]
    InvalidFileSize,

    #[error("invalid PCBTEXT file")]
    NoValidPCBTEXTFile,

    #[error("Invalid ICETEXT Entry ({0}) without text")]
    IceTextEntryWithoutText(String),

    #[error("ICETEXT Entry ({0}) already defined")]
    IceTextEntryAlreadyDefined(String),

    #[error("ICETEXT Entry ({0}) not defined")]
    IceTextEntryNotDefined(String),

    #[error("invalid ICETEXT file")]
    NoValidIceTextFile,

    #[error("invalid ICETEXT entry ({0})")]
    IceTextEntryInvalid(String),
}

#[derive(Clone, Copy, PartialEq, Default, Display, EnumString)]
pub enum IcbTextJustification {
    #[default]
    Left,
    Right,
    Center,
}

#[derive(Clone, Copy, PartialEq, Default, Display, Debug, EnumString)]
pub enum IcbTextStyle {
    #[default]
    Plain,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    White,
}

impl IcbTextStyle {
    pub fn to_color(&self) -> IcbColor {
        match self {
            IcbTextStyle::Plain => IcbColor::None,
            IcbTextStyle::Red => IcbColor::Dos(12),
            IcbTextStyle::Green => IcbColor::Dos(10),
            IcbTextStyle::Yellow => IcbColor::Dos(14),
            IcbTextStyle::Blue => IcbColor::Dos(9),
            IcbTextStyle::Purple => IcbColor::Dos(13),
            IcbTextStyle::Cyan => IcbColor::Dos(11),
            IcbTextStyle::White => IcbColor::Dos(15),
        }
    }

    pub fn from_pcboard_byte(b: u8) -> Self {
        match b {
            b'1' => IcbTextStyle::Red,
            b'2' => IcbTextStyle::Green,
            b'3' => IcbTextStyle::Yellow,
            b'4' => IcbTextStyle::Blue,
            b'5' => IcbTextStyle::Purple,
            b'6' => IcbTextStyle::Cyan,
            b'7' => IcbTextStyle::White,
            _ => IcbTextStyle::Plain,
        }
    }
}

#[derive(Clone, Default)]
pub struct TextEntry {
    pub style: IcbTextStyle,
    pub text: String,
    pub justification: IcbTextJustification,
}

const HEADER: &str = "# IcyBoard text file v1.0\n";

lazy_static::lazy_static! {
    pub static ref DEFAULT_DISPLAY_TEXT: IcbTextFile = {
        let data = include_bytes!("../data/ICBTEXT.toml");
        IcbTextFile::deserialize(data, String::new()).unwrap()
    };
}

#[derive(Clone, Default)]
pub struct IcbTextFile {
    entries: Vec<TextEntry>,
}

impl IcbTextFile {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn get_display_text(&self, message_number: IceText) -> Result<TextEntry, TextError> {
        if let Some(entry) = self.entries.get(message_number as usize) {
            Ok(entry.clone())
        } else {
            DEFAULT_DISPLAY_TEXT.get_display_text(message_number)
        }
    }

    pub fn load<P: AsRef<Path>>(path: &P) -> Res<Self> {
        match fs::read(path) {
            Ok(data) => Self::deserialize(&data, path.as_ref().display().to_string()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(IcyError::FileNotFound(path.as_ref().to_string_lossy().to_string()).into())
                } else {
                    log::error!(
                        "Error loading icb text file '{}': {}",
                        path.as_ref().display(),
                        e
                    );
                    Err(IcyError::ErrorLoadingFile(
                        path.as_ref().to_string_lossy().to_string(),
                        e.to_string(),
                    )
                    .into())
                }
            }
        }
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn save<P: AsRef<Path>>(&self, path: &P) -> Res<()> {
        let mut txt = HEADER.to_string();
        txt.push('\n');

        for (i, entry) in self.entries.iter().enumerate().skip(1) {
            let line = format!("[{}]\n", IceText::from(i));
            txt.push_str(&line);
            let line = format!("text = \"{}\"\n", escape_toml(&entry.text));
            txt.push_str(&line);

            if entry.style != IcbTextStyle::Plain {
                let line = format!("style = \"{}\"\n", entry.style);
                txt.push_str(&line);
            }
            if entry.justification != IcbTextJustification::Left {
                let line = format!("justify = \"{}\"\n", entry.justification);
                txt.push_str(&line);
            }
            txt.push('\n');
        }
        fs::write(path, &txt)?;
        Ok(())
    }

    fn deserialize(data: &[u8], file: String) -> Res<Self> {
        let entries = if data.starts_with(&HEADER.as_bytes()[..9]) {
            load_ice_format(data, file)?
        } else {
            import_pcboard_format(data, file)?
        };
        Ok(Self { entries })
    }

    pub fn export_pcboard_format<P: AsRef<Path>>(&self, file_name: &P) -> Res<()> {
        let mut fs = BufWriter::new(File::create(file_name)?);
        let header = TextEntry::new(
            IcbTextStyle::Red,
            PCBTEXT_HEADER,
            IcbTextJustification::Left,
        );
        header.serialize_pcb_format(&mut fs)?;
        for entry in self.entries.iter().skip(1) {
            entry.serialize_pcb_format(&mut fs)?;
        }
        Ok(())
    }
}

impl Deref for IcbTextFile {
    type Target = Vec<TextEntry>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for IcbTextFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

impl TextEntry {
    fn new(
        color: IcbTextStyle,
        text: impl Into<String>,
        justification: IcbTextJustification,
    ) -> Self {
        Self {
            style: color,
            text: text.into(),
            justification,
        }
    }

    fn serialize_pcb_format(&self, fs: &mut BufWriter<File>) -> Res<()> {
        let pcb_char = match self.style {
            IcbTextStyle::Plain => b'0',
            IcbTextStyle::Red => b'1',
            IcbTextStyle::Green => b'2',
            IcbTextStyle::Yellow => b'3',
            IcbTextStyle::Blue => b'4',
            IcbTextStyle::Purple => b'5',
            IcbTextStyle::Cyan => b'6',
            IcbTextStyle::White => b'7',
        };
        fs.write_u8(pcb_char)?;
        let mut output_buffer = [b' '; BIN_ENTRY_SIZE - 1];
        let b = b' ';
        for (i, ch) in self.text.chars().take(BIN_ENTRY_SIZE - 1).enumerate() {
            output_buffer[i] = *UNICODE_TO_CP437.get(&ch).unwrap_or(&b);
        }
        fs.write_all(&output_buffer)?;
        Ok(())
    }
}

fn load_ice_format(data: &[u8], file: String) -> Res<Vec<TextEntry>> {
    let text = std::str::from_utf8(data)?;
    match text.parse::<Value>() {
        Ok(Value::Table(entries)) => {
            let mut res = vec![None; LAST_ENTRY + 1];
            res[0] = Some(TextEntry::default());
            for (k, v) in entries {
                let Ok(ice_text) = k.parse::<IceText>() else {
                    return Err(Box::new(TextError::IceTextEntryInvalid(k.clone())));
                };

                if let Value::Table(values) = v {
                    let text = if let Some(Value::String(text)) = values.get("text") {
                        text.clone()
                    } else {
                        return Err(Box::new(TextError::IceTextEntryWithoutText(k.clone())));
                    };

                    let mut entry = TextEntry::new(
                        IcbTextStyle::default(),
                        text,
                        IcbTextJustification::default(),
                    );

                    if let Some(Value::String(text)) = values.get("style") {
                        entry.style = text.parse::<IcbTextStyle>()?;
                    }

                    if let Some(Value::String(text)) = values.get("justify") {
                        entry.justification = text.parse::<IcbTextJustification>()?;
                    }
                    if res[ice_text as usize].is_some() {
                        return Err(Box::new(TextError::IceTextEntryAlreadyDefined(k.clone())));
                    }
                    res[ice_text as usize] = Some(entry);
                }
            }
            Ok(res.into_iter().flatten().collect())
        }
        Ok(_) => {
            log::error!("IcbText file doesn't conatin a table ({})", file);
            Err(Box::new(TextError::NoValidIceTextFile))
        }
        Err(err) => {
            log::error!("Error parsing icb text file ({}): {} ", file, err);
            Err(Box::new(TextError::NoValidIceTextFile))
        }
    }
}

const BIN_ENTRY_SIZE: usize = 0x50;

/// PCBoard hard coded justifications
const RIGHT_JUSTIFY_RECORDS: [usize; 30] = [
    20, 23, 44, 47, 48, 81, 91, 108, 115, 122, 124, 147, 151, 172, 174, 183, 184, 201, 202, 205,
    206, 207, 362, 364, 391, 392, 410, 432, 456, 462,
];

const PCBTEXT_HEADER: &str = "PCBoard version 14.5 & 15.0 & 15.2 & 15.3 & 15.4 PCBTEXT file";

fn import_pcboard_format(data: &[u8], file: String) -> Result<Vec<TextEntry>, TextError> {
    let mut res = Vec::new();
    if data.len() % BIN_ENTRY_SIZE != 0 {
        log::error!("Invalid file size for PCBoard text file ({})", file);
        return Err(TextError::InvalidFileSize);
    }
    for (i, chunk) in data.chunks(BIN_ENTRY_SIZE).enumerate() {
        let text = import_cp437_string(&chunk[1..], true);
        if i == 0 && !text.starts_with("PCBoard version") {
            log::error!("Invalid PCBoard text file ({})", file);
            return Err(TextError::NoValidPCBTEXTFile);
        }
        let color = IcbTextStyle::from_pcboard_byte(chunk[0]);
        let justification = if RIGHT_JUSTIFY_RECORDS.contains(&i) {
            IcbTextJustification::Right
        } else {
            IcbTextJustification::Left
        };
        res.push(TextEntry::new(color, text, justification));
    }
    Ok(res)
}

pub fn escape_toml(input: &str) -> String {
    let mut res = String::new();
    for c in input.chars() {
        if c == '\\' {
            res.push_str("\\\\");
        } else if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c == ' ' {
            res.push(c);
        } else {
            res.push_str(&format!("\\u{:04x}", c as u32));
        }
    }
    res
}
