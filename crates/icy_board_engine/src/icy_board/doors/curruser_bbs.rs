use std::fs;

use crate::{
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
    Res,
};

/// CALLINFO.BBS format from the RyBBS software.
pub fn create_curruser_bbs(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    contents.push_str(&format!("{}\r\n", state.session.user_name));
    contents.push_str(&format!("{}\r\n", state.session.cur_security));
    contents.push_str(&format!("{}\r\n", state.session.cur_user_id));
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().home_voice_phone));
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().city_or_state));
    contents.push_str(&format!("{}\r\n", DOOR_COM_PORT)); // COM Port
    contents.push_str(&format!("{}\r\n", DOOR_BPS_RATE)); // Com Port Speed
    contents.push_str("N\r\n");
    contents.push_str("8\r\n");
    contents.push_str("1\r\n");
    contents.push_str("\r\n");
    contents.push_str("DOORM.MNU\r\n");
    contents.push_str(&format!("{}\r\n", state.session.minutes_left()));

    let emulation = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "NONE",
        GraphicsMode::Ansi => "IBM",
        _ => "ANSI",
    };
    contents.push_str(&format!("{}\r\n", emulation));

    let path = path.join("CURRUSER.BBS");
    log::info!("create CURRUSER.BBS: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

/*
# Source: IcyBoard/From what I guessed out of a RyBBS install.

JOHN DOE     User Name
20           User Security
1            User Record number (begins with 0)
555-444-333  Telephone Number
City         Location
1            COM PORT
19200        BPS
N            Parity
8            Data Bits
1            Stop Bits
             Empty/Unknown
SYSOPM.MNU   Current Menu
44           TIME LEFT in mins
ANSI         GFX MODE "ANSI", "IBM" (for ascii) or "NONE"
*/
