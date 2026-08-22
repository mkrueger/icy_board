use std::{error::Error, str::FromStr};

use logos::Logos;
use strum_macros::EnumIter;

#[derive(Debug, PartialEq, EnumIter)]
pub enum MacroCommand {
    Alias,

    /// Any More? prompts displayed after this macro will be interpreted as @PAUSE@ codes.
    /// If the user does not respond to the More? prompt within 10 seconds, the default of Y is accepted.
    AutoMore,

    /// This macro sends an audible tone ( CTRL-G ) to the remote caller.
    /// In order for you to hear this audible tone, you must have the Bell turned on at the call waiting screen.
    Beep,

    /// Used internally by `PCBoard` to display file transfer statistics.
    BICPS,

    /// Displays the name of the BBS. This information is stored in `ICBSetup` > System Information
    BoardName,

    /// Displays the connect speed that `ICBoard` announced at login.
    /// For error-correcting connections, the opening port speed will be displayed.
    /// Otherwise, the actual carrier speed will be displayed.
    /// If you want to always display the carrier speed of the caller, use the @CARRIER@ macro instead.
    /// Example output: 38400
    BPS,

    /// Displays the byte ratio for the current caller.
    /// The format for the output is downloads:uploads. For example, if a caller has downloaded 10,000
    /// bytes and uploaded 2,000 bytes, they have a 5:1 byte ratio. In other words, for each five bytes they download they
    /// have uploaded one byte. Example output: 5:1
    ByteCredit,

    /// Displays the daily download byte limit of the caller. Example output: 737,280
    ByteLimit,

    /// Displays the byte ratio for the current caller. The format for the output is downloads:uploads.
    /// For example, if a caller has downloaded 10,000 bytes and uploaded 2,000 bytes, they have a 5:1 byte ratio.
    /// In other words, for each five bytes they download they have uploaded one byte. Example output: 5:1
    ByteRatio,

    /// Displays the number of bytes the caller has left for the day. If the caller has been given unlimited downloads,
    /// the word Unlimited will be displayed instead of the actual amount of bytes. Example output: 325,567
    BytesLeft,

    /// Displays the connect speed of the caller as returned by the modem. If the user is on via a local login, the opening port speed is reported.
    /// Example output: 14400
    Carrier,

    /// This macro will display the caller's city. This information is stored inside of the user file in the City field.
    /// Example output: MURRAY, UT
    City,

    /// Clears the current text line to the end of the line (the last column on the display screen).
    /// This is useful if you want to make sure there is no other text to the right of the @CLREOL@ macro or if you are using a
    /// background color in your color displays and want to have it extend to the end of the text line.
    ClrEol,

    /// Clears the screen (i.e., no information is displayed on the screen immediately after this macro is used).
    Cls,

    /// Displays the current conference name. This is quite similar to the @INCONF@ macro except it does not display the conference number.
    /// Example output:Main Board
    ConfName,

    /// Displays the current conference number Example output: 3
    ConfNum,

    CredLeft,

    CredNow,

    CredStart,

    CredUsed,

    /// Displays the last message number the caller read, not necessarily the highest message number the caller has read.
    /// For example, if the caller had read to message number 532 but decided to go back and read message number 1, this
    /// macro would display 1 because that is the last message the caller actually read. Example output: 32,322
    CurMsgNum,

    /// This macro will display the business/data phone number stored in each user record. The actual text is not formatted
    /// – it is taken straight from the user record. Example output: 801-261-8976
    DataPhone,

    /// Displays the number of bytes downloaded on the current day. If the value displayed is negative, the caller has been given
    /// credit for an upload. Negative bytes are added to the daily byte limit of the caller.
    ///
    /// For example, if you used this macro and it displayed -52,322, that would mean that the caller could download
    /// 52,322 bytes in addition to their normal daily limit. Example output: 332,356
    DayBytes,

    /// Delays for nn tenths of a second. If you enter @DELAY:50@, `PCBoard` will pause for 5 seconds (50 * .10 = 5.0).
    /// You can enter any value between 0 and 255 meaning that you can pause between 0 and 25.5 seconds.
    Delay(u16),

    DirName,

    DirNum,

    /// Displays the total number of bytes that the caller has downloaded from the system. Example output: 2,352,532
    DlBytes,

    /// Displays the total number of files the caller has downloaded from the system. Example output: 1,054
    DlFiles,

    Env(String),

    /// Displays the time (in 24 hour format) that the next system event will run.
    /// If you do not have your system scheduled to run an event or no events are defined,
    /// the time displayed will be 00:00. Example output: 14:30
    Event,

    /// This macro will display the expiration date of the current caller.
    /// If a user has no expiration date or if you have disabled subscription mode,
    /// 00-00-00 will be displayed. Example output: 08-30-94
    ExpDate,

    /// Displays the number of days until the caller's subscription will expire.
    /// If the user does not have an expiration date or you have disasbled subscription mode,
    /// nothing will be displayed on the screen. Example output: 325
    ExpDays,

    FBytes,

    FFiles,

    FileCredit,

    /// Displays the file ratio for the current caller. The format for the output is downloads:uploads.
    /// For example, if a caller has downloaded 150 files and uploaded 30 files, they would have a 5:1 file ratio.
    /// In other words, for each five files they download, they have uploaded one file. Example output: 5:1
    FileRatio,

    /// This macro will display the first name of the caller. When the caller's first name is displayed,
    /// the first letter will be capitalized and all subsequent letters in the first name will be lower case.
    /// Example output: Stanley
    First,

    /// This macro will display the exact same information as @FIRST@ only the first name will be displayed in all
    /// upper case letters. Example output: STANLEY
    FirstU,

    FNum,

    /// Displays the amount of drive space in bytes that are available on the private upload drive of the current conference.
    /// Example output: 122,346,098
    FreeSpace,

    /// This macro will disconnect the current caller. When the caller is disconnected, the number of minutes used and the Thanks
    /// for calling message will be displayed. This macro may only be used in display files but not in messages.
    ///
    /// In order for `PCBoard` to recognize this macro, it must begin at the first character of the line.
    Hangup,

    /// This macro will display the business/data phone number stored in each user record. The actual text is not formatted –
    ///  it is taken straight from the user record. Example output: 202-555-1212
    HomePhone,

    /// Displays the highest message number in the conference that the caller is currently in. Example output: 11,523
    HighMSGNum,

    IName,

    /// Displays the conference name and conference number that the caller is currently in. In addition, the word Conference will
    /// be added to end of the conference name and number.
    /// The only exception is the Main Board conference which will simply display Main Board.
    /// If you do not want the word Conference to be added, use the @CONFNUM@ and @CONFNAME@ macros instead.
    /// Example output: Support (1) Conference
    InConf,

    /// Displays the number of kilobytes the caller has available for the rest of the day.
    /// If a user has earned upload bytes, they will be added to their normal daily limit.
    /// Example output: 768
    KBLeft,

    /// This macro will display the total number of kilobytes the caller is normally allowed to download each day.
    /// Example output: 2,048
    KBLimit,

    /// Displays the name and city of the last caller to the current node. Example output: JIM SMITH (ANYWHERE, USA)
    LastCallerNode,

    /// Displays the name and city of the last caller to the entire system (all nodes are searched). Example output: BOB JONES (ANYPLACE, USA)
    LastCallerSystem,

    /// This macro will display the last date the caller was on the BBS. The current call will not be taken into account. Example output: 09-25-93
    LastDateOn,

    /// This macro will display the last time the caller was on the BBS. The time is displayed in 24-hour format. This macro will not take the current call into account. Example output: 16:32
    LastTimeOn,

    /// Displays the highest message number the caller has read in the current conference.
    /// This is sometimes referred to as the Last Message Read pointer and is stored inside of the user's record.
    /// Example output: 7,938
    LMR,

    LogDate,

    LogTime,

    /// Displays the lowest message number in the conference the caller is currently in. Example output: 11,523
    LowMsgNum,

    MaxBytes,

    MaxFiles,

    /// Displays the total number of minutes the caller has remaining for this session.
    /// If a user has flagged files for download, the estimated time that it will take to transfer those files is reflected in the @MINLEFT@ macro.
    /// In other words, if a caller has 35 minutes left for the session and they flag 10 minutes worth of files, the @MINLEFT@ macro will display 25.
    /// If you do not want to take the flagged files into account, use the @TIMELEFT@ macro instead. Example output: 34
    MinLeft,

    /// Displays a More? prompt. This is the same prompt that is normally displayed when the screen is full.
    /// You can manually insert a More? prompt to help control the display of a text file. Example output: (57 min left), (H)elp, More?
    More,

    /// Displays the number of messages the user has entered on the BBS system. This information is stored in the user record.
    MsgLeft,

    /// Displays the number of messages the user has read on the BBS system. This information is stored in the user record.
    MsgRead,

    /// 'n' char localized
    NoChar,

    /// Displays the node number the caller is currently using. Example output: 10
    Node,

    /// This macro will display the total number of bulletins available in the current conference. Example output: 32
    NumBLT,

    /// Displays the number of calls the BBS system has answered. Example output: 124,329
    NumCalls,

    NumConf,

    /// Displays the total number of file directories available in the current conference. Example output: 60
    NumDir,

    /// Displays the total number of message areas available in the current conference. Example output: 60
    NumArea,

    /// This macro will display the number of times the caller has called this BBS system. This number is stored in the user record. Example output: 365
    NumTimesOn,

    /// This macro will display the hours you allow lower speed callers to call your system. All hour displays are listed in 24 hour format.
    /// These hours are defined in `PCBSetup` > Modem Information > Allowed Access Speeds. Example output: 01:00-06:00
    OffHours,

    /// This macro is used throughout PCBTEXT to pass information from `PCBoard` into the records in PCBTEXT.
    /// You should only use this macro within those records that were designed to use @OPTEXT@.
    OpText,

    /// Displays a More? prompt. If the caller does not respond to the prompt within 10 seconds,
    /// the default value of Y will be assumed and the display will continue. Example output: (38 min left), (H)elp, More?
    Pause,

    /// Disables the More? prompt while displaying the file.
    /// If you use this command, make sure you use the @PON@ command at the end of the file to enable the more prompts again.  
    POFF,

    /// Enables the More? prompt while displaying a file. The default is to always display a More? prompt. However, if you use the @POFF@ macro,
    /// you will want to use the @PON@ macro to enable More? prompts again.
    PON,

    /// Moves the cursor to position nn on the current line. If the cursor is already beyond the position entered, no action is taken.
    /// Otherwise, spaces are printed until the cursor reaches the position you specify.
    POS(u16),

    /// Displays the protocol letter the caller has chosen fortheir default protocol. Example output: Z
    ProLTR,

    /// Displays the description of the default protocol the caller has selected. Example output: Zmodem
    ProDesc,

    PwxDate,

    PwxDays,

    /// This macro will disable CTRL-X / CTRL-K checking. Thismeans the caller will not be able to interrupt or abort the display of the file.
    /// Any More? prompts that are displayed after a @QOFF@ macro will be turned into Press (Enter) to continue? prompts
    /// because the caller cannot abort the display.
    QOFF,

    /// This macro will enable CTRL-X / CTRL-K checking. You should use this macro whenever you have turned off the checking and wish to
    /// turn it back on again.
    QON,

    RatioBytes,

    RatioFiles,

    /// Used internally by `ICBoard` for file transfer statistics.
    RBytes,

    /// Used internally by `ICBoard` for file transfer statistics.
    RCPS,

    /// The real name of the user.
    Real,

    /// Used internally by `ICBoard` for file transfer statistics.
    RFiles,

    /// Used internally by `ICBoard` for file transfer statistics.
    SBytes,

    /// Used internally by `ICBoard` for file transfer statistics.
    SCPS,

    /// Displays the security level of the user.
    /// Any security level adjustments added when joining a conference will be reflected in the value displayed. Example output: 60
    Security,

    /// Used internally by `PCBoard` for file transfer statistics.
    SFiles,

    /// Displays the current date. Example output: 09-23-93
    SysDate,

    /// Displays the beginning time a user may page the `SysOp` for chat.
    /// This time is defined in `PCBSetup` > Configuration Options > Limits.
    /// All time displays are in 24-hour format. Example output: 18:00
    SysopIn,

    /// This macro will display the ending time a user may page the `SysOp` for chat.
    /// This time is defined in `PCBSetup` > Configuration Options > Limits..
    /// All time displays are in 24-hour format. Example output: 17:00
    SysopOut,

    /// Displays the current time. All time displays are in 24-hour format. Example output: 15:32
    SysTime,

    /// Displays the total time in minutes a user can use per day/session.
    /// This limit is defined in the PWRD file in `PCBSetup`. Example output: 60
    TimeLimit,

    /// Displays the amount of time the caller has left for this session.
    /// This macro does not take into account any files the user has flagged for download.
    /// If you wish to take flagged files into account, use the @MINLEFT@ macro instead. Example output: 32
    TimeLeft,

    /// Displays the number of minutes used during the current call. Example output: 12
    TimeUsed,

    /// Displays the total amount of time (in minutes) that the caller has used on the system for the day. Example output: 128
    TotalTime,

    /// Displays the total number of bytes the user has uploaded to the BBS. Example output: 36,928,674
    UpBytes,

    /// Displays the total number of files that the user has uploaded to the BBS. Example output: 352
    UpFiles,

    /// Opens an OSC 8 hyperlink when a URI is present, or closes it otherwise.
    Url(Option<String>),

    /// This macro will display the full user name of the caller in all uppercase letters.
    /// You can also use this macro in the TO: field of a message. If you do, your single generic message will appear to
    /// each user to be addressed personally to them..Example output: EDWARD B. SMITH
    User,

    /// This macro will display a Press (Enter) to continue? prompt to the user. Of course, the only way to continue past
    /// this prompt is for the caller to press EMTER.
    Wait,

    /// This macro will display the exact same thing as if the user had typed WHO at the `PCBoard` command prompt.
    /// The display includes all active node numbers and who is on each of the respective nodes. NOTE: Once the @WHO@ macro is used,
    /// the page line counter is reset to maximize screen output. Therefore, if you put the @WHO@ macro in the middle of the
    /// screen, be aware that the top of the screen may scroll off. To prevent this, you should put an @PAUSE@ or an @MORE@
    /// just before the @WHO@.
    Who,

    XOff,

    XON,

    YesChar,

    /// @X00 Color code
    SwitchColor(u8),

    /// New in ICB : Displays the name of the current graphics mode. Example output: ANSI
    GfxMode,

    /// New in ICB : Current Message Area name,
    AreaName,

    /// New in ICB : Current Message Area number,
    AreaNum,

    /// New in ICB : Current Message Area number,
    SysopName,

    /// New in ICB : `IcyBoard` version number
    Version,
}

#[derive(Debug, PartialEq)]
pub enum MacroJustification {
    LeftJustify,
    RightJustify,
    Center,
}

#[derive(Debug, Logos)]
pub enum PcbToken {
    #[token("ALIAS", |_| MacroCommand::Alias, ignore(case))]
    #[token("AUTOMORE", |_| MacroCommand::AutoMore, ignore(case))]
    #[token("BEEP", |_| MacroCommand::Beep, ignore(case))]
    #[token("BICPS", |_| MacroCommand::BICPS, ignore(case))]
    #[token("BOARDNAME", |_| MacroCommand::BoardName, ignore(case))]
    #[token("BPS", |_| MacroCommand::BPS, ignore(case))]
    #[token("BYTECREDIT", |_| MacroCommand::ByteCredit, ignore(case))]
    #[token("BYTELIMIT", |_| MacroCommand::ByteLimit, ignore(case))]
    #[token("BYTERATIO", |_| MacroCommand::ByteRatio, ignore(case))]
    #[token("BYTESLEFT", |_| MacroCommand::BytesLeft, ignore(case))]
    #[token("CARRIER", |_| MacroCommand::Carrier, ignore(case))]
    #[token("CITY", |_| MacroCommand::City, ignore(case))]
    #[token("CLREOL", |_| MacroCommand::ClrEol, ignore(case))]
    #[token("CLS", |_| MacroCommand::Cls, ignore(case))]
    #[token("CONFNAME", |_| MacroCommand::ConfName, ignore(case))]
    #[token("CONFNUM", |_| MacroCommand::ConfNum, ignore(case))]
    #[token("CREDLEFT", |_| MacroCommand::CredLeft, ignore(case))]
    #[token("CREDNOW", |_| MacroCommand::CredNow, ignore(case))]
    #[token("CREDSTART", |_| MacroCommand::CredStart, ignore(case))]
    #[token("CREDUSED", |_| MacroCommand::CredUsed, ignore(case))]
    #[token("CURMSGNUM", |_| MacroCommand::CurMsgNum, ignore(case))]
    #[token("DATAPHONE", |_| MacroCommand::DataPhone, ignore(case))]
    #[token("DAYBYTES", |_| MacroCommand::DayBytes, ignore(case))]
    #[regex("DELAY:\\d+", |lex| MacroCommand::Delay(get_macro_number(6, lex.slice())), ignore(case))]
    #[token("DIRNAME", |_| MacroCommand::DirName, ignore(case))]
    #[token("DIRNUM", |_| MacroCommand::DirNum, ignore(case))]
    #[token("DLBYTES", |_| MacroCommand::DlBytes, ignore(case))]
    #[token("DLFILES", |_| MacroCommand::DlFiles, ignore(case))]
    #[regex("ENV=([_\\w]+)", |lex| MacroCommand::Env( lex.slice().to_owned().split_off(4)), ignore(case))]
    #[token("EVENT", |_| MacroCommand::Event, ignore(case))]
    #[token("EXPDATE", |_| MacroCommand::ExpDate, ignore(case))]
    #[token("EXPDAYS", |_| MacroCommand::ExpDays, ignore(case))]
    #[token("FBYTES", |_| MacroCommand::FBytes, ignore(case))]
    #[token("FFILES", |_| MacroCommand::FFiles, ignore(case))]
    #[token("FILECREDIT", |_| MacroCommand::FileCredit, ignore(case))]
    #[token("FILERATIO", |_| MacroCommand::FileRatio, ignore(case))]
    #[token("FIRST", |_| MacroCommand::First, ignore(case))]
    #[token("FIRSTU", |_| MacroCommand::FirstU, ignore(case))]
    #[token("FNUM", |_| MacroCommand::FNum, ignore(case))]
    #[token("FREESPACE", |_| MacroCommand::FreeSpace, ignore(case))]
    #[token("GFXMODE", |_| MacroCommand::GfxMode, ignore(case))]
    #[token("AREANAME", |_| MacroCommand::AreaName, ignore(case))]
    #[token("AREANUM", |_| MacroCommand::AreaNum, ignore(case))]
    #[token("HANGUP", |_| MacroCommand::Hangup, ignore(case))]
    #[token("HOMEPHONE", |_| MacroCommand::HomePhone, ignore(case))]
    #[token("HIGHMSGNUM", |_| MacroCommand::HighMSGNum, ignore(case))]
    #[token("INAME", |_| MacroCommand::IName, ignore(case))]
    #[token("INCONF", |_| MacroCommand::InConf, ignore(case))]
    #[token("KBLEFT", |_| MacroCommand::KBLeft, ignore(case))]
    #[token("KBLIMIT", |_| MacroCommand::KBLimit, ignore(case))]
    #[token("LASTCALLERNODE", |_| MacroCommand::LastCallerNode, ignore(case))]
    #[token("LASTCALLERSYSTEM", |_| MacroCommand::LastCallerSystem, ignore(case))]
    #[token("LASTDATEON", |_| MacroCommand::LastDateOn, ignore(case))]
    #[token("LASTTIMEON", |_| MacroCommand::LastTimeOn, ignore(case))]
    #[token("LMR", |_| MacroCommand::LMR, ignore(case))]
    #[token("LOGDATE", |_| MacroCommand::LogDate, ignore(case))]
    #[token("LOGTIME", |_| MacroCommand::LogTime, ignore(case))]
    #[token("LOWMSGNUM", |_| MacroCommand::LowMsgNum, ignore(case))]
    #[token("MAXBYTES", |_| MacroCommand::MaxBytes, ignore(case))]
    #[token("MAXFILES", |_| MacroCommand::MaxFiles, ignore(case))]
    #[token("MINLEFT", |_| MacroCommand::MinLeft, ignore(case))]
    #[token("MORE", |_| MacroCommand::More, ignore(case))]
    #[token("MSGLEFT", |_| MacroCommand::MsgLeft, ignore(case))]
    #[token("MSGREAD", |_| MacroCommand::MsgRead, ignore(case))]
    #[token("NOCHAR", |_| MacroCommand::NoChar, ignore(case))]
    #[token("NODE", |_| MacroCommand::Node, ignore(case))]
    #[token("NUMBLT", |_| MacroCommand::NumBLT, ignore(case))]
    #[token("NUMCALLS", |_| MacroCommand::NumCalls, ignore(case))]
    #[token("NUMCONF", |_| MacroCommand::NumConf, ignore(case))]
    #[token("NUMDIR", |_| MacroCommand::NumDir, ignore(case))]
    #[token("NUMAREA", |_| MacroCommand::NumArea, ignore(case))]
    #[token("NUMTIMESON", |_| MacroCommand::NumTimesOn, ignore(case))]
    #[token("OFFHOURS", |_| MacroCommand::OffHours, ignore(case))]
    #[token("OPTEXT", |_| MacroCommand::OpText, ignore(case))]
    #[token("PAUSE", |_| MacroCommand::Pause, ignore(case))]
    #[token("POFF", |_| MacroCommand::POFF, ignore(case))]
    #[token("PON", |_| MacroCommand::PON, ignore(case))]
    #[regex("POS:\\d+", |lex| MacroCommand::POS(get_macro_number(4, lex.slice())), ignore(case))]
    #[token("PROLTR", |_| MacroCommand::ProLTR, ignore(case))]
    #[token("PRODESC", |_| MacroCommand::ProDesc, ignore(case))]
    #[token("PWXDATE", |_| MacroCommand::PwxDate, ignore(case))]
    #[token("PWXDAYS", |_| MacroCommand::PwxDays, ignore(case))]
    #[token("QOFF", |_| MacroCommand::QOFF, ignore(case))]
    #[token("QON", |_| MacroCommand::QON, ignore(case))]
    #[token("RATIOBYTES", |_| MacroCommand::RatioBytes, ignore(case))]
    #[token("RATIOFILES", |_| MacroCommand::RatioFiles, ignore(case))]
    #[token("RBYTES", |_| MacroCommand::RBytes, ignore(case))]
    #[token("RCPS", |_| MacroCommand::RCPS, ignore(case))]
    #[token("REAL", |_| MacroCommand::Real, ignore(case))]
    #[token("RFILES", |_| MacroCommand::RFiles, ignore(case))]
    #[token("SBYTES", |_| MacroCommand::SBytes, ignore(case))]
    #[token("SCPS", |_| MacroCommand::SCPS, ignore(case))]
    #[token("SECURITY", |_| MacroCommand::Security, ignore(case))]
    #[token("SFILES", |_| MacroCommand::SFiles, ignore(case))]
    #[token("SYSDATE", |_| MacroCommand::SysDate, ignore(case))]
    #[token("SYSOPIN", |_| MacroCommand::SysopIn, ignore(case))]
    #[token("SYSOPOUT", |_| MacroCommand::SysopOut, ignore(case))]
    #[token("SYSOPNAME", |_| MacroCommand::SysopName, ignore(case))]
    #[token("SYSTIME", |_| MacroCommand::SysTime, ignore(case))]
    #[token("TIMELIMIT", |_| MacroCommand::TimeLimit, ignore(case))]
    #[token("TIMELEFT", |_| MacroCommand::TimeLeft, ignore(case))]
    #[token("TIMEUSED", |_| MacroCommand::TimeUsed, ignore(case))]
    #[token("TOTALTIME", |_| MacroCommand::TotalTime, ignore(case))]
    #[token("UPBYTES", |_| MacroCommand::UpBytes, ignore(case))]
    #[token("UPFILES", |_| MacroCommand::UpFiles, ignore(case))]
    #[token("URL", |_| MacroCommand::Url(None), ignore(case))]
    #[regex("URL:[^\\x00-\\x20]+", |lex| MacroCommand::Url(Some(lex.slice()[4..].to_owned())), ignore(case))]
    #[token("USER", |_| MacroCommand::User, ignore(case))]
    #[token("VERSION", |_| MacroCommand::Version, ignore(case))]
    #[token("WAIT", |_| MacroCommand::Wait, ignore(case))]
    #[token("WHO", |_| MacroCommand::Who, ignore(case))]
    #[token("XOFF", |_| MacroCommand::XOff, ignore(case))]
    #[token("XON", |_| MacroCommand::XON, ignore(case))]
    #[token("YESCHAR", |_| MacroCommand::YesChar, ignore(case))]
    #[regex("X([a-fA-F0-9]{2})", |lex| MacroCommand::SwitchColor(get_macrohex_color(lex.slice())), ignore(case))]
    Macro(MacroCommand),

    #[regex(":(\\d)+(T)?[C|R]?", |lex| {
        parse_formatting(lex.slice())
    }, ignore(case))]
    Format((u16, bool, MacroJustification)),
}

fn parse_formatting(str: &str) -> (u16, bool, MacroJustification) {
    let mut num: u16 = 0;
    let mut is_trunc = false;
    let mut justification = MacroJustification::LeftJustify;
    let mut parse_num = true;
    for c in str.chars().skip(1) {
        if parse_num {
            if let Some(digit) = c.to_digit(10) {
                num = num.saturating_mul(10_u16).saturating_add(digit as u16);
                continue;
            }
            parse_num = false;
        }
        if c == 'T' || c == 't' {
            is_trunc = true;
        } else if c == 'C' || c == 'c' {
            justification = MacroJustification::Center;
        } else if c == 'R' || c == 'r' {
            justification = MacroJustification::RightJustify;
        }
    }
    (num, is_trunc, justification)
}

fn get_macro_number(offset: usize, s: &str) -> u16 {
    s[offset..].parse::<u16>().unwrap_or(0)
}

fn get_macrohex_color(hex_color: &str) -> u8 {
    u8::from_str_radix(&hex_color[1..], 16).unwrap_or(0)
}

pub struct Macro {
    pub command: MacroCommand,
    pub justification: MacroJustification,
    pub length: u16,
    pub truncate: bool,
}

impl Macro {
    pub fn format_value(&self, val: &str) -> String {
        let mut result = val.to_string();

        match self.justification {
            MacroJustification::LeftJustify => {
                result = format!("{:<width$}", result.trim_start(), width = self.length as usize);
            }
            MacroJustification::RightJustify => {
                result = format!("{:>width$}", result.trim_end(), width = self.length as usize);
            }
            MacroJustification::Center => {
                result = format!("{:^width$}", result.trim(), width = self.length as usize);
            }
        }

        if self.truncate {
            result.truncate(self.length as usize);
        }
        result
    }
}

impl FromStr for Macro {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lexer = PcbToken::lexer(s);

        let Some(Ok(PcbToken::Macro(command))) = lexer.next() else {
            return Err("Invalid macro format".into());
        };
        let is_url = matches!(command, MacroCommand::Url(_));
        if let MacroCommand::Url(Some(uri)) = &command
            && uri.chars().any(|character| character.is_control() || character.is_whitespace())
        {
            return Err("Invalid URL macro".into());
        }

        let mut justification = MacroJustification::LeftJustify;
        let mut length = 0;
        let mut truncate = false;

        match lexer.next() {
            Some(Ok(PcbToken::Format((len, trunc, just)))) => {
                length = len;
                truncate = trunc;
                justification = just;
            }
            Some(_) if is_url => return Err("Invalid macro format".into()),
            Some(_) => {}
            None => {}
        }

        if is_url && lexer.next().is_some() {
            return Err("Invalid macro format".into());
        }

        Ok(Macro {
            command,
            justification,
            length,
            truncate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_macro() {
        let macro_str = "DELAY:12";
        let macro_parsed: Macro = macro_str.parse().unwrap();
        assert_eq!(macro_parsed.command, MacroCommand::Delay(12));
    }

    #[test]
    fn parse_complex_macro() {
        let macro_str = "ENV=FOOBAR:209TR";
        let macro_parsed: Macro = macro_str.parse().unwrap();
        assert_eq!(macro_parsed.command, MacroCommand::Env("FOOBAR".to_string()));
        assert_eq!(macro_parsed.justification, MacroJustification::RightJustify);
        assert_eq!(macro_parsed.length, 209);
        assert!(macro_parsed.truncate);
    }

    #[test]
    fn test_right_justify() {
        let macro_str = "CLS:20R";
        let macro_parsed: Macro = macro_str.parse().unwrap();
        let formatted = macro_parsed.format_value("Hello");
        assert_eq!(formatted, "               Hello");
    }

    #[test]
    fn test_left_justify() {
        let macro_str = "CLS:20";
        let macro_parsed: Macro = macro_str.parse().unwrap();
        let formatted = macro_parsed.format_value("Hello");
        assert_eq!(formatted, "Hello               ");
    }

    #[test]
    fn test_center_justify() {
        let macro_str = "CLS:20C";
        let macro_parsed: Macro = macro_str.parse().unwrap();
        let formatted = macro_parsed.format_value("Hello");
        assert_eq!(formatted, "       Hello        ");
    }

    #[test]
    fn lexer_test_macro() {
        let mut lexer = PcbToken::lexer("YESCHAR");
        let token = lexer.next().unwrap().unwrap();
        assert!(matches!(token, PcbToken::Macro(MacroCommand::YesChar)));
    }

    #[test]
    fn lexer_test_env() {
        let mut lexer = PcbToken::lexer("ENV=TEST");
        let token = lexer.next().unwrap().unwrap();
        if let PcbToken::Macro(MacroCommand::Env(env)) = token {
            assert_eq!(env, "TEST".to_string());
        } else {
            panic!("Expected ENV macro");
        }
    }

    #[test]
    fn parses_url_hyperlink_boundaries() {
        let open: Macro = "URL:https://example.com/a:b?x=1".parse().unwrap();
        assert_eq!(open.command, MacroCommand::Url(Some("https://example.com/a:b?x=1".to_string())));

        let close: Macro = "URL".parse().unwrap();
        assert_eq!(close.command, MacroCommand::Url(None));
    }

    #[test]
    fn url_rejects_whitespace_and_control_characters() {
        assert!("URL:https://example.com/a b".parse::<Macro>().is_err());
        assert!("URL:https://example.com/\x1b\\payload".parse::<Macro>().is_err());
        assert!("URL:https://example.com/\u{009c}payload".parse::<Macro>().is_err());
    }

    #[test]
    fn lexer_test_delay() {
        let mut lexer = PcbToken::lexer("DELAY:12");
        let token = lexer.next().unwrap().unwrap();
        if let PcbToken::Macro(MacroCommand::Delay(i)) = token {
            assert_eq!(i, 12);
        } else {
            panic!("Expected Delay macro");
        }
    }

    #[test]
    fn lexer_test_pos() {
        let mut lexer = PcbToken::lexer("pos:50");
        let token = lexer.next().unwrap().unwrap();
        if let PcbToken::Macro(MacroCommand::POS(i)) = token {
            assert_eq!(i, 50);
        } else {
            panic!("Expected POS macro");
        }
    }

    #[test]
    fn lexer_test_color_code() {
        let mut lexer = PcbToken::lexer("XFf");
        let token = lexer.next().unwrap().unwrap();
        if let PcbToken::Macro(MacroCommand::SwitchColor(i)) = token {
            assert_eq!(i, 255);
        } else {
            panic!("Expected POS macro");
        }
    }

    #[test]
    fn lexer_test_parse_format() {
        let mut lexer = PcbToken::lexer(":209");
        let token = lexer.next().unwrap().unwrap();
        if let PcbToken::Format((len, truncate, justify)) = token {
            assert_eq!(len, 209);
            assert!(!truncate);
            assert_eq!(justify, MacroJustification::LeftJustify);
        } else {
            panic!("Expected POS macro");
        }
    }

    #[test]
    fn lexer_test_parse_truncate() {
        let mut lexer = PcbToken::lexer(":209T");
        let token = lexer.next().unwrap().unwrap();
        if let PcbToken::Format((len, truncate, justify)) = token {
            assert_eq!(len, 209);
            assert!(truncate);
            assert_eq!(justify, MacroJustification::LeftJustify);
        } else {
            panic!("Expected POS macro");
        }
    }

    #[test]
    fn lexer_test_parse_justify() {
        let mut lexer = PcbToken::lexer(":9C");
        let token = lexer.next().unwrap().unwrap();
        if let PcbToken::Format((len, truncate, justify)) = token {
            assert_eq!(len, 9);
            assert!(!truncate);
            assert_eq!(justify, MacroJustification::Center);
        } else {
            panic!("Expected POS macro");
        }
    }

    #[test]
    fn lexer_test_parse_trunkjustify() {
        let mut lexer = PcbToken::lexer(":99TR");
        let token = lexer.next().unwrap().unwrap();
        if let PcbToken::Format((len, truncate, justify)) = token {
            assert_eq!(len, 99);
            assert!(truncate);
            assert_eq!(justify, MacroJustification::RightJustify);
        } else {
            panic!("Expected POS macro");
        }
    }
}
