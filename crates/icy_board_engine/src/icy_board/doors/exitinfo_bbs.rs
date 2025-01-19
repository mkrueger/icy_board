use crate::{
    icy_board::{
        doors::DOOR_BPS_RATE,
        state::{GraphicsMode, IcyBoardState},
    },
    tables::export_cp437_string,
    Res,
};
use chrono::Local;
use icy_engine::TextPane;
use icy_net::crc::get_crc32;
use std::fs;

/// EXITINFO.BBS format from the RemoteAccess software. (2.62 extensions)
pub async fn create_exitinfo_bbs(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = Vec::new();
    contents.extend(u16::to_le_bytes(DOOR_BPS_RATE as u16));
    let Some(user) = &state.session.current_user else {
        return Err("No current user".into());
    };

    // SYSINFO
    let board = state.get_board().await;
    contents.extend(u32::to_le_bytes(board.statistics.total.calls as u32));
    if let Some(last) = board.statistics.last_callers.last() {
        contents.extend(export_cp437_string(&last.user_name, 35, 0));
    } else {
        contents.extend(export_cp437_string("", 35, 0));
    }
    if let Some(last) = board.statistics.last_callers.last() {
        contents.extend(export_cp437_string(&last.user_name, 35, 0));
    } else {
        contents.extend(export_cp437_string("", 35, 0));
    }
    // extra space
    contents.extend([0; 92].iter());

    // OldTIMELOGrecord
    contents.extend(state.session.login_date.format("%m-%d-%y").to_string().as_bytes());
    for _i in 0..24 {
        // BusyPerHour
        contents.extend(u16::to_le_bytes(0));
    }
    for _i in 0..7 {
        // BusyPerDay
        contents.extend(u16::to_le_bytes(0));
    }

    // USERSrecord
    contents.extend(export_cp437_string(&user.get_name(), 35, 0));
    contents.extend(export_cp437_string(&user.city_or_state, 25, 0));
    contents.extend(export_cp437_string("", 50, 0)); // Organisation
    contents.extend(export_cp437_string(&user.street1, 50, 0));
    contents.extend(export_cp437_string(&user.street2, 50, 0));
    contents.extend(export_cp437_string(&user.city, 50, 0));
    contents.extend(export_cp437_string(&user.alias, 35, 0));
    contents.extend(export_cp437_string(&user.user_comment, 80, 0));
    let pwd_crc = get_crc32(user.password.password.to_string().as_bytes());
    contents.extend(u32::to_le_bytes(pwd_crc));
    contents.extend(export_cp437_string(&user.bus_data_phone, 15, 0));
    contents.extend(export_cp437_string(&user.home_voice_phone, 15, 0));
    contents.extend(user.stats.last_on.format("%m-%d-%y").to_string().as_bytes());
    let mut attribute = 0;
    match state.session.disp_options.grapics_mode {
        GraphicsMode::Ansi | GraphicsMode::Avatar | GraphicsMode::Rip => attribute |= 1 << 3,
        _ => {}
    }
    contents.push(attribute);

    let mut attribute2 = 0;
    if state.session.disp_options.grapics_mode == GraphicsMode::Avatar {
        attribute2 |= 1 << 1;
    }
    contents.push(attribute2);
    contents.extend(u32::to_le_bytes(0)); // Credit
    contents.extend(u32::to_le_bytes(0)); // Pending
    contents.extend(u16::to_le_bytes(user.stats.messages_left as u16));
    contents.extend(u16::to_le_bytes(state.session.cur_security as u16));
    contents.extend(u32::to_le_bytes(0)); // LastRead
    contents.extend(u32::to_le_bytes(user.stats.num_times_on as u32));
    contents.extend(u32::to_le_bytes(user.stats.num_uploads as u32));
    contents.extend(u32::to_le_bytes(user.stats.num_downloads as u32));
    contents.extend(u32::to_le_bytes((user.stats.total_upld_bytes / 1024) as u32));
    contents.extend(u32::to_le_bytes((user.stats.total_dnld_bytes / 1024) as u32));
    contents.extend(u32::to_le_bytes((user.stats.today_dnld_bytes / 1024) as u32));
    contents.extend(u32::to_le_bytes(0)); // Elapsed
    contents.extend(u16::to_le_bytes(state.session.page_len as u16));
    contents.push(0); // LastPwdChange
    contents.extend(u16::to_le_bytes(0)); // Group

    // CombinedInfo ?
    for _i in 0..200 {
        contents.extend(u16::to_le_bytes(0));
    }
    contents.extend(user.stats.first_date_on.format("%m-%d-%y").to_string().as_bytes());
    contents.extend(user.birth_date.format("%m-%d-%y").to_string().as_bytes());
    contents.extend(user.stats.first_date_on.format("%m-%d-%y").to_string().as_bytes()); // SubDate?
    contents.push(state.display_screen().buffer.get_width() as u8);
    contents.push(0); // Language
    contents.push(2); // DateFormat // MM-DD-YY - see below
    contents.extend(export_cp437_string("", 35, 0)); // ForwardTo
    contents.extend(u16::to_le_bytes(0)); // MsgArea
    contents.extend(u16::to_le_bytes(0)); // FileArea
    contents.push(user.protocol.chars().next().unwrap_or(' ') as u8);
    contents.extend(u16::to_le_bytes(state.session.current_conference_number)); // FileGroup
    contents.push(0); // LastDOBCheck
    contents.extend(u16::to_le_bytes(state.session.current_conference_number)); // MsgGroup
    contents.push(0); // Attribute3
    contents.extend(export_cp437_string(&state.door_user_password().await, 15, 0)); // Password

    contents.extend(user.stats.last_on.format("%C").to_string().as_bytes());
    contents.extend(user.stats.first_date_on.format("%C").to_string().as_bytes());
    contents.extend(user.birth_date.format("%C").to_string().as_bytes());
    contents.extend(user.stats.first_date_on.format("%C").to_string().as_bytes()); // SubDate ?

    // free space
    for _i in 0..19 {
        contents.push(0);
    }

    // EVENTrecord
    contents.push(2); // Disabled event
    contents.extend(b"00:00");
    contents.push(0); // ErrorLevel
    contents.push(0); // Days
    contents.push(0); // Forced
    contents.extend(b"00-00-00"); // MM-DD-YY' New 'MMKDDCYY'
                                  // END OF EVENTrecord

    contents.push(0); // NetMailEntered
    contents.push(0); // EchoMailEntered

    contents.extend(state.session.login_date.format("%H:%M").to_string().as_bytes());
    contents.extend(state.session.login_date.format("%m-%d-%y").to_string().as_bytes()); // SubDate?
    contents.extend(u16::to_le_bytes(state.session.minutes_left() as u16));
    contents.extend(u32::to_le_bytes(user.security_level as u32));
    contents.extend(u32::to_le_bytes(state.session.cur_user_id as u32));

    contents.extend(u16::to_le_bytes(0)); // ReadThru
    contents.extend(u16::to_le_bytes(0)); // NumberPages
    contents.extend(u16::to_le_bytes(0)); // DownloadLimit
    contents.extend(Local::now().format("%H:%M").to_string().as_bytes()); // TimeOfCreation
                                                                          // Same as above - seems that can be used for login process doors.
    contents.extend(u32::to_le_bytes(pwd_crc));
    contents.push(0); // WantChat

    contents.extend(u32::to_le_bytes(0)); // DeductedTime
                                          // MenuStack ?
    for _i in 0..50 {
        contents.extend(export_cp437_string("", 8, 0));
    }
    contents.push(0); // MenuStackPointer

    // UserXIinfo - free space
    for _i in 0..200 {
        contents.push(0);
    }
    contents.push(1); // ErrorFreeConnect
    contents.push(0); // SysopNext

    if let Some(ici) = &state.session.emsi {
        contents.push(1); // EMSI_Session
        contents.extend(export_cp437_string(&ici.term.get_crtdef_string(), 40, 0));
        contents.extend(export_cp437_string(&ici.term.protocols, 40, 0));
        contents.extend(export_cp437_string(&ici.term.get_cap_string(), 40, 0));
        contents.extend(export_cp437_string(&ici.requests.get_requests_string(), 40, 0));
        contents.extend(export_cp437_string(&ici.term.software, 40, 0));
    } else {
        contents.push(0); // EMSI_Session
        contents.extend(export_cp437_string(&"", 40, 0));
        contents.extend(export_cp437_string(&"", 40, 0));
        contents.extend(export_cp437_string(&"", 40, 0));
        contents.extend(export_cp437_string(&"", 40, 0));
        contents.extend(export_cp437_string(&"", 40, 0));
    }

    contents.push(0); // Hold_Attr1
    contents.push(0); // Hold_Attr2
    contents.push(0); // Hold_Len

    contents.extend(export_cp437_string(&"", 80, 0)); // PageReason
    contents.push(0); // StatusLine
    contents.extend(export_cp437_string(&"", 8, 0)); // LastCostMenu
    contents.extend(u16::to_le_bytes(0)); // MenuCostPerMin

    if state.session.disp_options.grapics_mode == GraphicsMode::Avatar {
        contents.push(1); // DoesAVT
    } else {
        contents.push(0); // DoesAVT
    }

    if state.session.disp_options.grapics_mode == GraphicsMode::Rip {
        contents.push(1); // RIPmode
        contents.push(0); // RIPVersion
    } else {
        contents.push(0); // RIPmode
        contents.push(0); // RIPVersion
    }

    contents.extend(user.stats.last_on.format("%C").to_string().as_bytes());
    contents.extend(state.session.login_date.format("%C").to_string().as_bytes()); // SubDate ?

    // ExtraSpace - free space
    for _i in 0..79 {
        contents.push(0);
    }

    let path = path.join("EXITINFO.BBS");
    log::info!("create EXITINFO.BBS: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

/* From RemoteAccess Docs:
  EXITINFOrecord = record  {format changes slightly
             Baud             : Word;
             SysInfo          : SYSINFOrecord;      { not affected }
             OldTimeLogInfo   : OldTIMELOGrecord;   { Century Prefix below }
             UserInfo         : USERSrecord;        { Format change - size unchanged }
             EventInfo        : EVENTrecord;        { Format change - size unchanged }
             NetMailEntered,
             EchoMailEntered  : Boolean;
             LoginTime        : Time;
             OldLoginDate     : OldDate;            { Century Prefix below }
             TimeLimit        : Word;
             LoginSec         : LongInt;
             UserRecord       : Integer;
             ReadThru,
             NumberPages,
             DownloadLimit    : Word;
             TimeOfCreation   : Time;
             LogonPasswordCRC : LongInt;
             WantChat         : Boolean;

             DeductedTime     : Integer;
             MenuStack        : Array[1..50] of String[8];
             MenuStackPointer : Byte;
             UserXIinfo       : USERSXIrecord;
             ErrorFreeConnect,
             SysopNext        : Boolean;

             EMSI_Session     : Boolean;        { These fields hold  }
             EMSI_Crtdef,                       { data related to an }
             EMSI_Protocols,                    { EMSI session       }
             EMSI_Capabilities,
             EMSI_Requests,
             EMSI_Software    : String[40];
             Hold_Attr1,
             Hold_Attr2,
             Hold_Len         : Byte;

             PageReason       : String[80];
             StatusLine       : Byte;
             LastCostMenu     : String[8];
             MenuCostPerMin   : Word;

             DoesAVT,
             RIPmode          : Boolean;
             RIPVersion       : Byte;

       PrefixTimeLogInfo_StartDate,       { 'KC' From TimeLogRecord.StartDate }
       PrefixLoginDate  : String[2];      { 'KC' From USERSrecord.LoginDate   }
             ExtraSpace       : Array[1..79] of Byte;  { 79 was 85 }
         end;

SYSINFOrecord  = record       { unchanged at ver 2.6 }
    TotalCalls     : LongInt;
    LastCaller,
    LastHandle     : MSGTOIDXrecord;
    ExtraSpace     : array[1..92] of Byte;
end;
MSGTOIDXrecord = String[35]; { unchanged at ver 2.6 }

Time           = String[5];
{}OldDate        = String[8];

{} OldTIMELOGrecord  = record  { Old style record for use     }
{}                             {  exclusively in EXITINFO.BBS }
{}         OldStartDate   : OldDate;  {MM-DD-YY}
{}         OldBusyPerHour : array[0..23] of Word;
{}         OldBusyPerDay  : array[0..6] of Word;
{}       end;
COMBINEDrecord = array[1..200] of Word;    { unchanged at ver 2.6 }

USERSrecord    = record  {Format change only - no change to file size }
             Name           : MSGTOIDXrecord;
             Location       : String[25];
             Organisation,
             Address1,
             Address2,
             Address3       : String[50];
             Handle         : String[35];
             Comment        : String[80];
             PasswordCRC    : LongInt;
             DataPhone,
             VoicePhone     : String[15];
             LastTime       : Time;
{}                   OldLastDate    : OldDate;  { MM-DD-YY format unchanged }

             Attribute,

              { Bit 0 : Deleted
                 1 : Clear screen
                 2 : More prompt
                 3 : ANSI
                 4 : No-kill
                 5 : Xfer priority
                 6 : Full screen msg editor
                 7 : Quiet mode }

             Attribute2     : Byte;

              { Bit 0 : Hot-keys
                 1 : AVT/0
                 2 : Full screen message viewer
                 3 : Hidden from userlist
                 4 : Page priority
                 5 : No echomail in mailbox scan
                 6 : Guest account
                 7 : Post bill enabled }

             Flags          : FlagType;
             Credit,
             Pending        : LongInt;
             MsgsPosted,
             Security       : Word;
             LastRead,
             NoCalls,
             Uploads,
             Downloads,
             UploadsK,
             DownloadsK,
             TodayK         : LongInt;
             Elapsed        : Integer;
             ScreenLength   : Word;
             LastPwdChange  : Byte;
             Group          : Word;
             CombinedInfo   : COMBINEDrecord;
{}                   OldFirstDate,
{}                   OldBirthDate,
{}                   OldSubDate     : OldDate;   {MM-DD-YY format unchanged}
             ScreenWidth,
             Language,
             DateFormat     : Byte;

        { DateFormat   Returns Date Name }
          {  Value       Format Picture   }
          {    1          'DD-MM-YY '     }
          {    2          'MM-DD-YY '     }
          {    3          'YY-MM-DD '     }
          {    4          'DD-Mmm-YY'     }
          {                               }
{}            {    5          'DD-MM-YYYY '   }
{}            {    6          'MM-DD-YYYY '   }
{}            {    7          'YYYY-MM-DD '   }
{}            {    8          'DD-Mmm-YYYY'   }
{}            { Values of 5 - 8 added at Version 2.60 }

             ForwardTo      : String[35];
             MsgArea,
             FileArea       : Word;
             DefaultProtocol: Char;
             FileGroup      : Word;
             LastDOBCheck   : Byte;
             Sex            : Byte;
             XIrecord       : LongInt;
             MsgGroup       : Word;

             Attribute3     : Byte;

              { Bit 0 : Mailbox check: scan selected areas only }

             Password       : String[15];

{}         PrefixLastDate,                 { here is where 'KC' information }
{}         PrefixFirstDate,                {  is stored for the indicated   }
{}         PrefixBirthDate,                {  variables.                    }
{}         PrefixSubDate  : PrefixDate;    {                                }

{}         FreeSpace      : Array[1..19] of Byte;  { 19 was 31 pre y2k version }
           end;
EVENTrecord    = record  { This file changes format - size unchanged }
             Status         : Byte; { 0=Deleted 1=Enabled 2=Disabled }
             StartTime      : Time;
             ErrorLevel     : Byte;
             Days           : Byte;
             Forced         : Boolean;
{}                   LastTimeRun    : OldDate;  {Old 'MM-DD-YY' New 'MMKDDCYY'}
           end;
  USERSXIrecord  = record     { unchanged at ver 2.6 }
             FreeSpace      : Array[1..200] of Byte;
           end;


*/
