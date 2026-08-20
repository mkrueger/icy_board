use std::fs;

use crate::{
    Res,
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
};
use std::fmt::Write as _;

/// `TriBBS` doorfile format
pub async fn create_tribbs_sys(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    let _ = write!(contents, "{}\r\n", state.session.cur_user_id);
    let _ = write!(contents, "{}\r\n", state.session.user_name);
    let _ = write!(contents, "{}\r\n", state.door_user_password().await);
    let _ = write!(contents, "{}\r\n", state.session.cur_security);
    let _ = write!(contents, "{}\r\n", if state.session.expert_mode() { "Y" } else { "N" });
    let ansi = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "N",
        _ => "Y",
    };
    let _ = write!(contents, "{ansi}\r\n");
    let _ = write!(contents, "{}\r\n", state.session.minutes_left());
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().home_voice_phone);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().city_or_state);
    let _ = write!(contents, "{}\r\n", state.node);
    let _ = write!(contents, "{DOOR_COM_PORT}\r\n");
    let _ = write!(contents, "{DOOR_BPS_RATE}\r\n");
    let _ = write!(contents, "{DOOR_BPS_RATE}\r\n");
    contents.push_str("Y\r\n"); // ?
    contents.push_str("Y\r\n"); // Error correcting connection
    let board = state.get_board().await;
    let _ = write!(contents, "{}\r\n", board.config.board.name);
    let _ = write!(contents, "{}\r\n", board.config.sysop.name);
    let _ = write!(contents, "{}\r\n", state.session.alias_name);
    let path = path.join("TRIBBS.SYS");
    log::info!("create TRIBBS.SYS: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

/*
1                 User's record number
John              User's name
Secret            User's password
255               User's level
Y                 Expert Y/N
Y                 ANSI   Y/N
999               Minutes left
99934543          User's phone number
city              User's city and state
1                 Node number
1                 Serial port
19200             Baud rate
19200             Locked rate
Y                 Unknown
Y                 Error correcting connection
Icy Shadow BBS    Board's name
Sysop             Sysop's name
Anonymous         User's alias

*/
