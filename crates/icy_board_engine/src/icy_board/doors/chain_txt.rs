use std::fs;

use chrono::{Timelike, Utc};
use icy_engine::TextPane;

use crate::{
    Res,
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
};
use std::fmt::Write as _;

/// CALLINFO.BBS format from the WWIV software.
pub async fn create_chain_txt(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    let board = state.get_board().await;
    let _ = write!(contents, "{}\r\n", state.session.cur_user_id);
    let _ = write!(contents, "{}\r\n", state.session.alias_name);
    let _ = write!(contents, "{}\r\n", state.session.user_name);
    contents.push_str("\r\n"); // User callsign (HAM radio)

    let day = &state.session.current_user.as_ref().unwrap().birth_date;
    let _ = write!(contents, "{}\r\n", Utc::now().years_since(*day).unwrap_or(0)); // Age

    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().gender);
    contents.push_str("0\r\n"); // Users Gold
    let _ = write!(
        contents,
        "{}\r\n",
        state.session.current_user.as_ref().unwrap().stats.last_on.format("%m/%d/%y")
    ); // User's last call date
    let _ = write!(contents, "{}\r\n", state.display_screen().buffer.width());
    let _ = write!(contents, "{}\r\n", state.display_screen().buffer.height());
    let _ = write!(contents, "{}\r\n", state.session.cur_security);
    contents.push_str("0\r\n");
    if state.session.is_sysop {
        contents.push_str("1\r\n");
    } else {
        contents.push_str("0\r\n");
    }
    let emulation = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "0",
        _ => "1",
    };
    let _ = write!(contents, "{emulation}\r\n");
    let _ = write!(contents, "{}\r\n", i32::from(!state.session.is_local));
    let _ = write!(contents, "{}\r\n", state.session.seconds_left()); // seconds till logofff
    contents.push_str("C:\\WWIV\\GFILES\\\r\n");
    contents.push_str("C:\\WWIV\\DATA\\\r\n");
    contents.push_str("890519.LOG \r\n");
    let _ = write!(contents, "{DOOR_BPS_RATE}\r\n"); // Com Port Speed
    let _ = write!(contents, "{DOOR_COM_PORT}\r\n"); // COM Port
    let _ = write!(contents, "{}\r\n", board.config.board.name);
    let _ = write!(contents, "{}\r\n", board.config.sysop.name);
    let _ = write!(contents, "{}\r\n", state.session.login_date.time().num_seconds_from_midnight());
    let _ = write!(contents, "{}\r\n", (Utc::now() - state.session.login_date).num_seconds());
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.total_upld_bytes / 1024);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_uploads);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.today_dnld_bytes / 1024);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_downloads);
    contents.push_str("8N1\r\n"); // Data bits/Parity/Stop bits
    let _ = write!(contents, "{DOOR_BPS_RATE}\r\n"); // Com Port Speed
    let _ = write!(contents, "{}\r\n", state.node); // Node number

    let path = path.join("CHAIN.TXT");
    log::info!("create CHAIN.TXT: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

/*
From wwiv source code:
CHAIN.TXT Definition File by MrBill.
-----------CHAIN.TXT-----------------------------------
1                  User number
MRBILL             User alias
Bill               User real name
                   User callsign (HAM radio)
21                 User age
M                  User sex
16097.00           User gold
05/19/89           User last logon date
80                 User colums
25                 User width
255                User security level (0-255)
1                  1 if Co-SysOp, 0 if not
1                  1 if SysOp, 0 if not
1                  1 if ANSI, 0 if not
0                  1 if at remote, 0 if local console
2225.78            User number of seconds left till logoff
F:\WWIV\GFILES\    System GFILES directory (gen. txt files)
F:\WWIV\DATA\      System DATA directory
890519.LOG         System log of the day
2400               User baud rate
2                  System com port
MrBill's Abode     System name
MrBill             System SysOp
83680              Time user logged on/# of secs. from midn
554                User number of seconds on system so far
5050               User number of uploaded k
22                 User number of uploads
42                 User amount of downloaded k
1                  User number of downloads
8N1                User parity
2400               Com port baud rate
7400               WWIVnet node number

*/
