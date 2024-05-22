use std::fs;

use crate::{
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
    Res,
};

/// 2AM BBS
pub async fn create_jumper_dat(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();

    let board = state.get_board().await;
    contents.push_str(&format!("{}\r\n", board.config.board.name));
    contents.push_str(&format!("{}\r\n", board.config.sysop.name));
    contents.push_str(&format!("{}\r\n", state.session.user_name));
    contents.push_str(&format!("{}\r\n", state.session.cur_user_id));
    contents.push_str(&format!("{}\r\n", state.session.get_first_name()));
    contents.push_str(&format!("{}\r\n", state.session.get_last_name()));
    contents.push_str(&format!("{}\r\n", state.session.current_user.as_ref().unwrap().city_or_state));
    contents.push_str(&format!("{}\r\n", state.session.minutes_left()));
    contents.push_str(&format!("{}\r\n", DOOR_COM_PORT));
    contents.push_str(&format!("{}\r\n", DOOR_BPS_RATE));
    contents.push_str("0\r\n"); // Number of nulls needed
    contents.push_str("FALSE\r\n"); // Linefeeds
    contents.push_str("FALSE\r\n"); // Upper Case only
    contents.push_str("TRUE\r\n"); // 80 COLUMNS
    contents.push_str("TRUE\r\n"); // IBM Graphisc
    let graphics_mode = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "FALSE",
        _ => "TRUE",
    };
    contents.push_str(&format!("{}\r\n", graphics_mode));
    contents.push_str("FALSE\r\n"); // System bell

    let path = path.join("JUMPER.DAT");
    log::info!("create JUMPER.DAT: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

/*
# Source: Official documentation (2AMSYS3.DOC)

11.1.2  JUMP_DISK:JUMPER.DAT

This is a file that the BBS writes before it sends a user through a door.
It contains some information about the user that any jumpdoor program can
make use of if it wishes.  The file is a text file and contains the
following (each item on a separate line):

    1. System Name
    2. System Owner
    3. Username
    4. Usernumber (i.e. account number)
    5. User's first name (from application)
    6. User's last name (from application)
    7. User's City, State
    8. Time Remaining
    9. Communications port (1 or 2).
    10. Baud rate ('300','1200','2400')
    11. Number of nulls needed (an integer from 0 to 15). *)
    12. Linefeeds?  'TRUE' if user needs linefeeds.  Otherwise: 'FALSE'.
    13. Upper Case only? 'TRUE' or 'FALSE'.
    14. 80 columns?  'TRUE' or 'FALSE'.
    15. IBM Graphics?  'TRUE' or 'FALSE'
    16. Ansi menus?  'TRUE' if the user can display ANSI, otherwise
    'FALSE'.
    17. Bell?  'TRUE' or 'FALSE'. Current setting of the system bell.

*) From doc I guess that's for configuring modem init nulls. Never had a modem that needed that nor do I know
any other sofware tools that use that.
*/
