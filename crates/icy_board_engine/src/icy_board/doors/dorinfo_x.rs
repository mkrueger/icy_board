use std::fs;

use crate::{
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
    Res,
};

/// RBBS/QuickBBS
pub async fn create_dorinfo(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    let board = state.get_board().await;
    contents.push_str(&format!("{}\r\n", board.config.board.name)); // System name
    contents.push_str(&format!("{}\r\n", board.users[0].get_first_name())); // Sysop first name
    contents.push_str(&format!("{}\r\n", board.users[0].get_last_name())); // Sysop last name
    contents.push_str(&format!("COM{}\r\n", DOOR_COM_PORT)); // Communications port in use (COM0 if local)
    contents.push_str(&format!("{} BAUD-R,N,8,1\r\n\r\n", DOOR_BPS_RATE)); // Communications port settings
    contents.push_str("0\r\n"); // Reserved (always zero)
    contents.push_str(&format!("{}\r\n", state.session.get_first_name())); // User first name
    contents.push_str(&format!("{}\r\n", state.session.get_last_name())); // User last name
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().get_first_name())); // User first name
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().get_last_name())); // User last name
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().city_or_state)); // User location
    let emulation = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => 0,
        GraphicsMode::Avatar => 2,
        _ => 1,
    };
    contents.push_str(&format!("{}\r\n", emulation)); // User emulation (0=ASCII, 1=ANSI, 2=AVATAR)
    contents.push_str(&format!("{}\r\n", state.session.cur_security)); // User security level
    contents.push_str(&format!("{}\r\n", state.session.minutes_left())); // User time remaining (in minutes)
    contents.push_str("-1\r\n"); // EOF

    let file_name = format!("DORINFO{}.DEF", state.node + 1);
    let path = path.join(&file_name);
    fs::write(path, contents)?;

    Ok(())
}

/*
From RemoteAccess docs:
.--------------------------------------------------------------------------.
Filename:	DORINFO1.DEF
Description:  A standard exit file created in the current
            (node) directory.  This is a standard ASCII
            text file and has the following format:

Line 1:  System name
Line 2:  Sysop first name
Line 3:  Sysop last name
Line 4:  Communications port in use (COM0 if local)
Line 5:  Communications port settings:
            BPS rate,parity,data bits,stop bits
        The BPS rate is specified as 0 during local sessions
        and is followed by the word BAUD.  During error-
        free connects, the word BAUD is followed by -R.
        The parity setting is always set to N for no parity.
        The data bits are always set to 8 and the stop bits
        are always set to 1.
Line 6:  Reserved (always zero)
Line 7:  User first name
Line 8:  User last name
Line 9:  User location
Line 10: User emulation (0=ASCII, 1=ANSI, 2=AVATAR)
Line 11: User security level
Line 12: User time remaining (in minutes)
Line 13: -1 EOF

Example DORINFO1.DEF:

REMOTEACCESS CENTRAL
ANDREW
MILNER
COM1
19200 BAUD-R,N,8,1
0
JOHN
PARLIN
BROOKLYN CENTER, MN, USA
1
100
60
-1
*/
