use std::fs;

use crate::{
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        icb_config::IcbColor,
        state::{GraphicsMode, IcyBoardState},
    },
    Res,
};

///  GAP, 52-line format
pub async fn create_door_sys(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    contents.push_str(&format!("COM{}:\r\n", DOOR_COM_PORT)); // COM Port
    contents.push_str(&format!("{}\r\n", DOOR_BPS_RATE)); // Com Port Speed
    contents.push_str("8\r\n"); // Data bits
    contents.push_str(&format!("{}\r\n", state.node + 1)); // Node number
    contents.push_str("Y\r\n"); // Screen display On
    contents.push_str("N\r\n"); // Printer toggle Off
    contents.push_str("N\r\n"); // Page bell Off
    contents.push_str("N\r\n"); // Caller alarm Off
    contents.push_str(&format!("{}\r\n", state.session.user_name)); // User full name
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().city_or_state)); // User location
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().home_voice_phone)); // Home/voice telephone number
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().bus_data_phone)); // Work/data telephone number
    contents.push_str(&format!("{}\r\n", state.door_user_password().await));
    contents.push_str(&format!("{}\r\n", state.session.cur_security)); // Security level
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_times_on)); // User's total number of calls to the system
    contents.push_str(&format!(
        "{}\r\n",
        state.session.current_user.as_ref().unwrap().stats.last_on.format("%m/%d/%y")
    )); // User's last call date
    contents.push_str(&format!("{}\r\n", state.session.seconds_left()));
    contents.push_str(&format!("{}\r\n", state.session.minutes_left()));

    let emulation = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "NG",
        _ => "GR",
    };
    contents.push_str(&format!("{}\r\n", emulation)); //Graphics mode (GR=ANSI, NG=ASCII)
    contents.push_str(&format!("{}\r\n", state.session.page_len)); // Screen length
    contents.push_str(if state.session.expert_mode { "Y\r\n" } else { "N\r\n" }); // User Mode
    contents.push_str("\r\n"); // Always blank
    contents.push_str("\r\n"); // Always blank

    contents.push_str("01/01/99\r\n"); // expiration date
    contents.push_str(&format!("{}\r\n", state.session.cur_user_id + 1)); // User's record number
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().protocol)); // Default protocol
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_uploads)); // User's total number of uploads
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_downloads)); // User's total number of downloads
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.today_dnld_bytes / 1024)); // User's daily download kilobytes total
    contents.push_str(&format!("999999\r\n")); // Daily download kilobyte limit
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().birth_date.format("%m/%d/%y"))); // User's date of birth
    contents.push_str("C:\\HOME\r\n"); // Path to the user database files
    contents.push_str("C:\\MSGS\r\n"); // Path to the message database files
    contents.push_str(&format!("{}\r\n", state.get_board().await.config.sysop.name)); // Sysop name
    contents.push_str(&format!("{}\r\n", state.session.alias_name)); // User's handle (alias)
    contents.push_str("00:00\r\n"); // Next event starting time
    contents.push_str("Y\r\n"); // Error-free connection (Y=Yes N=No)
    contents.push_str("N\r\n"); // Always set to N
    contents.push_str("Y\r\n"); // Always set to Y
    let default_color = match state.get_board().await.config.color_configuration.default {
        IcbColor::None => 7,
        IcbColor::Dos(col) => col % 15,
        IcbColor::IcyEngine(_) => 7,
    };
    contents.push_str(&format!("{}\r\n", default_color)); // BBS Default fg Color
    contents.push_str("0\r\n"); // Always set to 0
    contents.push_str("01/01/70\r\n"); // Last new files scan date
    contents.push_str(&format!("{}\r\n", state.session.login_date.format("%H:%M"))); // Time of this call
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.last_on.format("%H:%M"))); // Time of last call
    contents.push_str("32768\r\n"); // Always set to 32768
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.today_num_downloads)); // Number of files downloaded today
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.total_upld_bytes / 1024)); // Total kilobytes uploaded
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.total_dnld_bytes / 1024)); // Total kilobytes downloaded
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().user_comment)); // Comment stored in user record
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.total_doors_executed)); // Doors Opened
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().stats.messages_left)); // Total number of messages posted

    let path = path.join("DOOR.SYS");
    log::info!("create DOOR.SYS: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

/*
Filename:	DOOR.SYS
Description:  A standard exit file created in the current
            (node) directory.  This is a standard ASCII
            text file used by many external programs.
            Although this exit file is extremely detailed
            and includes information that cannot be
            generated by every BBS type, efforts were made
            to include as much information as possible.
            The format RemoteAccess uses for this file is
            as follows:

Line 1:  Communications port (COM0: if local)
Line 2:  BPS rate
Line 3:  Data bits/Parity 7 or 8
Line 4:  Node number
Line 5:  DTE rate (locked rate)
Line 6:  Screen display (snoop) (Y=On N=Off)
Line 7:  Printer toggle (Y=On N=Off)
Line 8:  Page bell (Y=On N=Off)
Line 9:  Caller alarm (Y=On N=Off)
Line 10: User full name
Line 11: User location
Line 12: Home/voice telephone number
Line 13: Work/data telephone number
Line 14: Password (not displayed)
Line 15: Security level
Line 16: User's total number of calls to the system
Line 17: User's last call date
Line 18: Seconds remaining this call
Line 19: Minutes remaining this call
Line 20: Graphics mode (GR=ANSI, NG=ASCII, 7E=7)
Line 21: Screen length
Line 22: Expert mode (always set to N)
Line 23: Joined Conferences
Line 24: Selected Conference
Line 25: Usner expiration date
Line 26: User's record number
Line 27: Default protocol
Line 28: User's total number of uploads
Line 29: User's total number of downloads
Line 30: User's daily download kilobytes total
Line 31: Daily download kilobyte limit
Line 32: User's date of birth
Line 33: Path to the user database files
Line 34: Path to the message database files
Line 35: Sysop full name
Line 36: User's handle (alias)
Line 37: Next event starting time
Line 38: Error-free connection (Y=Yes N=No)
Line 39: Always set to N
Line 40: Use Record Locking? / Always set to Y
Line 41: Text color as defined in RACONFIG
Line 42: Remaining Minutes (Always set to 0)
Line 43: Last new files scan date
Line 44: Time of this call
Line 45: Time of last call
Line 46: Max Daily Files/ Always set to 32768
Line 47: Number of files downloaded today
Line 48: Total kilobytes uploaded
Line 49: Total kilobytes downloaded
Line 50: Comment stored in user record
Line 51: Doors Opened
Line 52: Total number of messages posted

Example DOOR.SYS:

COM1:
9600
8
1
19200
Y
N
N
Y
John Parlin
Brooklyn Center, MN, USA
012-345-6789
012-345-9876
100
379
04-19-93
18780
313
GR
24
N
12-31-93
0
Z
3
7
0
8192
03-25-60
\RA\MSGBASE\
\RA\MSGBASE\
Bruce Morse
Bruce Morse
01:55
Y
N
Y
7
0
04-19-93
14:37
07:30
32768
0
396
580
Regular user
0
296

*/
