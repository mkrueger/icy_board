use crate::{
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
    Res,
};
use chrono::{Local, Utc};
use std::fs;

/// CALLINFO.BBS format from the RBBS software.
pub async fn create_callinfo_bbs(state: &IcyBoardState, path: &std::path::Path, door_number: usize) -> Res<()> {
    let mut contents = String::new();
    contents.push_str(&format!("{}\r\n", state.session.user_name)); // User Name
    contents.push_str("5\r\n"); // Baud 300=1, 1200=2, 2400=0, 9600=3, 19200=4, Local=5
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().city_or_state)); // Calling From
    contents.push_str(&format!("{}\r\n", state.session.cur_security)); // User security level
    contents.push_str(&format!("{}\r\n", state.session.minutes_left())); // User Time Left
    let emulation = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "MONO",
        _ => "COLOR",
    };
    contents.push_str(&format!("{}\r\n", emulation)); // Color or Mono
    contents.push_str(&format!("{}\r\n", state.door_user_password().await));
    contents.push_str(&format!("{}\r\n", state.session.cur_user_id + 1)); // User Reference Number
    contents.push_str("0\r\n"); // User Time On
    contents.push_str(&format!("{}\r\n", state.session.login_date.format("%H:%M"))); // Time Str
    contents.push_str(&format!("{}\r\n", state.session.login_date.format("%H:%M %m/%d%/%y"))); // Time-Date
    contents.push_str(&format!("{}\r\n", state.session.current_conference_number)); // Conference Joined
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.today_num_downloads)); // Daily Downloads
    contents.push_str(&format!("{}\r\n", 999)); // Max Downloads
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.today_dnld_bytes / 1024)); // Daily Download K
    contents.push_str(&format!("{}\r\n", 999 * 1024)); // Max Downloads KB
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().home_voice_phone)); // Phone Number
    contents.push_str(&format!("{}\r\n", Local::now().format("%m/%d%/%y %H:%M"))); // Date-Time
    let emulation = if state.session.expert_mode { "EXPERT" } else { "NOVICE" };
    contents.push_str(&format!("{}\r\n", emulation)); // Novice or Expert
    contents.push_str("All\r\n"); // Transfer Method  All, Ymodem, Ymodem/G, Xmodem, Xmodem/CRC, Xmodem-1K, Xmodem-1K/G, Ascii
    contents.push_str(&format!(
        "{}\r\n",
        state.session.current_user.as_ref().unwrap().stats.last_on.format("%m/%d%/%y")
    )); // Last New Date
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_times_on)); // Times on
    contents.push_str(&format!("{}\r\n", state.session.page_len)); // Lines per Page
    contents.push_str("42\r\n"); // Highest Message Read
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_uploads)); // Uploads
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_downloads)); // Downloads
    contents.push_str("8\r\n"); // Databits (7 or 8)
    if state.session.is_local {
        contents.push_str("LOCAL\r\n"); // LOCAL or REMOTE
    } else {
        contents.push_str("REMOTE\r\n"); // LOCAL or REMOTE
    }
    contents.push_str(&format!("COM{}\r\n", DOOR_COM_PORT)); // COM Port
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().birth_date.format("%m/%d%/%y"))); // Birth Date
    contents.push_str(&format!("{}\r\n", DOOR_BPS_RATE)); // Com Port Speed
    contents.push_str("TRUE\r\n"); // Already Connected
    contents.push_str("Normal Connection \r\n"); // MNP/ARQ or Normal Connection
    contents.push_str(&format!("{}\r\n", Utc::now().format("%m/%d%/%y %H:%M"))); // Date Time (Global)
    contents.push_str(&format!("{}\r\n", state.node + 1)); // Node ID
    contents.push_str(&format!("{}\r\n", door_number)); // Door Number

    let path = path.join("CALLINFO.BBS");
    log::info!("create callinfo.bbs: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

/* From Synchrnoet

Line 	Example 	Description 	Comment
01 	Jim Harrer 	User Name
02 	5 	Baud 	300=1, 1200=2, 2400=0, 9600=3, 19200=4, Local=5
03 	Bakersfield, CA 	Calling From
04 	1000 	Security Level
05 	120 	User Time Left
06 	MONO 	Color or Mono
07 	WCATIS#1 	Password
08 	1 	User Reference Number
09 	0 	Time On
10 	12:44 	Time Str
11 	12:44 05/08/89 	Time-Date 	{Changed in v2.0 }
12 	ABCDEFGHIJKLMNOYZ 	Conference Joined
13 	0 	Daily Downloads
14 	100 	Max Downloads
15 	0 	Daily Download K
16 	10 	Max Download K
17 	555-555-5555 	Phone Number
18 	05/08/89 12:44 	Date-Time
19 	NOVICE 	Novice or Expert
20 	All 	Transfer Method 	All, Ymodem, Ymodem/G, Xmodem, Xmodem/CRC, Xmodem-1K, Xmodem-1K/G, Ascii
21 	04/24/89 	Last New Date
22 	190 	Times on
23 	23 	Lines per Page
24 	808 	Highest Message Read
25 	0 	Uploads
26 	2 	Downloads
27 	8 	Databits (7 or 8)
28 	LOCAL 	LOCAL or REMOTE
29 	COM0 	COM Port
30 	06/26/58 	Birth Date
31 	38400 	Com Port Speed 	Baud Init or Baud Rate
32 	FALSE 	Already Connected
33 	Normal Connection 	MNP/ARQ or Normal Connection
34 	05/08/89 13:44 	Date Time (Global)
35 	1 	Node ID
36 	0 	Door Number

'''
procedure Save_Caller_Info;
  { Save the callers information when exiting to a live program or
    Dropping to DOS from remote. }
  var
    FileOut        : Text;
    DateTime       : String;
    OldSlash       : Char;
  begin
    Assign(FileOut, HomePath+'CALLINFO.BBS');
    Rewrite(FileOut);
    CheckError('Rewriting CALLINFO.BBS');
    Update_User_Info_Before_LogOff;
    with User do
      begin
        OldSlash := SlashChar;
        SlashChar := '/';
        WriteLn(FileOut, UserName);                                  { Line 1 }
        case Baud of
          B300 : WriteLn(FileOut, '1');                              { Line 2 }
          B1200 : WriteLn(FileOut, '2');
          B2400 : WriteLn(FileOut, '0');
          B9600 : WriteLn(FileOut, '3');
          B19200 : WriteLn(FileOut, '4');
          BLocal : WriteLn(FileOut, '5');
        end;
        WriteLn(FileOut, CallingFrom);                               { Line 3 }
        WriteLn(FileOut, SecurityLevel);                             { Line 4 }
        WriteLn(FileOut, User.TimeLeft);                             { Line 5 }
        if ColorMenus then WriteLn(FileOut, 'COLOR')
        else                                                         { Line 6 }
          WriteLn(FileOut, 'MONO');
        WriteLn(FileOut, Password);                                  { Line 7 }
        WriteLn(FileOut, UserRefNum);                                { Line 8 }
        WriteLn(FileOut, TimeOn);                                    { Line 9 }
        WriteLn(FileOut, TimeStr);                                  { Line 10 }
        with DateTimeCalled do
          DateTime := TimeToTimeString('hh:mm', T)+' '+DatetoDateString('mm/dd/yy', D);
        WriteLn(FileOut, DateTime);                                 { Line 11 }
        WriteLn(FileOut, ConfJoined);                               { Line 12 }
        WriteLn(FileOut, DailyDL);                                  { Line 13 }
        WriteLn(FileOut, Cfig.SecMaxDL[Array_Level]);               { Line 14 }
        WriteLn(FileOut, Sc(DailyDK));                              { Line 15 }
        WriteLn(FileOut, Sc(Cfig.SecMaxDK[Array_Level]/1024));      { Line 16 }
        WriteLn(FileOut, User.PhoneNumber);                         { Line 17 }
        WriteLn(FileOut, DatetoDateString('mm/dd/yy', TimeDate.D)+Space+
                TimeToTimeString('hh:mm', TimeDate.T));             { Line 18 }
        if User.Xpert = Novice then WriteLn(FileOut, 'NOVICE')
        else                                                        { Line 19 }
          WriteLn(FileOut, 'EXPERT');
        case User.TransferMethod of                                 { Line 20 }
          All : WriteLn(FileOut, 'All');
          Ymodem : WriteLn(FileOut, 'Ymodem');
          YmodemG : WriteLn(FileOut, 'Ymodem/G');
          Xmodem : WriteLn(FileOut, 'Xmodem');
          XmodemCRC : WriteLn(FileOut, 'Xmodem/CRC');
          Xmodem1K : WriteLn(FileOut, 'Xmodem-1K');
          Xmodem1KG : WriteLn(FileOut, 'Xmodem-1K/G');
          ASCii : WriteLn(FileOut, 'Ascii');
        end;
        WriteLn(FileOut, DatetoDateString('mm/dd/yy', LastNew.D));  { Line 21 }
        WriteLn(FileOut, Sc(TimesOn));                              { Line 22 }
        WriteLn(FileOut, Sc(LinesPerPage));                         { Line 23 }
        WriteLn(FileOut, Sc(UsersHighestMsgRead(HighMsg, ConfJoined))); { Line 24 }
        WriteLn(FileOut, Sc(Uploads));                              { Line 25 }
        WriteLn(FileOut, Sc(Downloads));                            { Line 26 }
        if DataBits = SevenBits then WriteLn(FileOut, '7  { Databits } ')
        else WriteLn(FileOut, '8  { Databits }');                   { Line 27 }
        if Local then WriteLn(FileOut, 'LOCAL')
        else WriteLn(FileOut, 'REMOTE');                            { Line 28 }
        WriteLn(FileOut, 'COM'+Sc(Cfig.CommPort));                  { Line 29 }
        WriteLn(FileOut, DatetoDateString('mm/dd/yy', BirthDate));  { Line 30 }
        { Write Comm Port Speed }
        if Cfig.FixedRate then WriteLn(FileOut, Cfig.BaudInit)
        else
          WriteLn(FileOut, BaudRate);                               { Line 31 }
        WriteLn(FileOut, AlreadyConnected);                         { Line 32 }
        if MNP_Connection then WriteLn(FileOut, 'MNP/ARQ Connection')
        else WriteLn(FileOut, 'Normal Connection');                 { Line 33 }
        with GlobalNinfo.TimeOff do
          DateTime := DatetoDateString('mm/dd/yy', D)+' '+TimeToTimeString('hh:mm', T);
        WriteLn(FileOut, DateTime);                                 { Line 34 }
        WriteLn(FileOut, Cfig.NodeID);                              { Line 35 }
        WriteLn(FileOut, DoorNumber);                               { Line 36 }
        SlashChar := OldSlash;
      end;                                                             { With }
    Close(FileOut);
    CheckError('Closing CALLINFO.BBS File');
  end;  { Save_Caller_Info }
'''
*/
