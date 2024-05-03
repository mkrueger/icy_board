use std::fs;

use crate::{
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
    Res,
};

/// Solar Realms doorfile.sr format
pub fn create_doorfile_sr(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    contents.push_str(&format!("{}\r\n", state.session.get_username_or_alias())); // Complete name or handle of user

    let emulation = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "0",
        _ => "1",
    };
    contents.push_str(&format!("{}\r\n", emulation)); // ANSI status:  1 = yes, 0 = no, -1 = don't know
    contents.push_str("1\r\n"); // IBM Graphic characters:  1 = yes, 0 = no, -1 = unknown
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().city_or_state)); // Calling From
    contents.push_str(&format!("{}\r\n", state.session.page_len)); // Page length of screen, in lines.  Assume 25 if unknown
    contents.push_str(&format!("{}\r\n", DOOR_BPS_RATE)); // Baud Rate
    contents.push_str(&format!("{}\r\n", DOOR_COM_PORT)); // Baud Rate
    contents.push_str(&format!("{}\r\n", state.session.minutes_left())); //Time Limit:  (in minutes); -1 if unknown.
    contents.push_str(&format!("{}\r\n", state.session.user_name)); // Real name (the same as line 1 if not known)

    let path = path.join("DOORFILE.SR");
    log::info!("create DOORFILE.SR: {}", path.display());
    fs::write(path, contents)?;

    Ok(())
}

/*
[From 93Oct20 SRDOOR.DOC 4.0 documentation]

Note: the SRDOOR.DOC does not mention using 0 on line 6 for local, it's in the SRDOOR.EXE 4.1 though.

    You can also write your own program to convert your favorite door file to DOORFILE.SR. The DOORFILE.SR format is:

(line 1): Complete name or handle of user
(line 2): ANSI status:  1 = yes, 0 = no, -1 = don't know
(line 3): IBM Graphic characters:  1 = yes, 0 = no, -1 = unknown
(line 4): Page length of screen, in lines.  Assume 25 if unknown
(line 5): Baud Rate:  300, 1200, 2400, 9600, 19200, etc.
(line 6): Com Port:  1, 2, 3, or 4; 0 if local.
(line 7): Time Limit:  (in minutes); -1 if unknown.
(line 8): Real name (the same as line 1 if not known)
*/
