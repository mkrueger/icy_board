use std::fs;

use crate::{
    Res,
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
};

/// TriBBS doorfile format
pub async fn create_tribbs_sys(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    contents.push_str(&format!("{}\r\n", state.session.cur_user_id));
    contents.push_str(&format!("{}\r\n", state.session.user_name));
    contents.push_str(&format!("{}\r\n", state.door_user_password().await));
    contents.push_str(&format!("{}\r\n", state.session.cur_security));
    contents.push_str(&format!("{}\r\n", if state.session.expert_mode() { "Y" } else { "N" }));
    let ansi = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "N",
        _ => "Y",
    };
    contents.push_str(&format!("{}\r\n", ansi));
    contents.push_str(&format!("{}\r\n", state.session.minutes_left()));
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().home_voice_phone));
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().city_or_state));
    contents.push_str(&format!("{}\r\n", state.node));
    contents.push_str(&format!("{}\r\n", DOOR_COM_PORT));
    contents.push_str(&format!("{}\r\n", DOOR_BPS_RATE));
    contents.push_str(&format!("{}\r\n", DOOR_BPS_RATE));
    contents.push_str("Y\r\n"); // ?
    contents.push_str("Y\r\n"); // Error correcting connection
    let board = state.get_board().await;
    contents.push_str(&format!("{}\r\n", board.config.board.name));
    contents.push_str(&format!("{}\r\n", board.config.sysop.name));
    contents.push_str(&format!("{}\r\n", state.session.alias_name));
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
