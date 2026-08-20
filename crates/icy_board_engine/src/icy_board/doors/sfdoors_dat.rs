use std::fs;

use chrono::{Local, Timelike};

use crate::{
    Res,
    icy_board::{
        doors::{DOOR_BPS_RATE, DOOR_COM_PORT},
        state::{GraphicsMode, IcyBoardState},
    },
};
use std::fmt::Write as _;

/// `SpitFire` BBS
pub async fn create_sfdoors_dat(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    let _ = write!(contents, "{}\r\n", state.session.cur_user_id);
    let _ = write!(contents, "{}\r\n", state.session.user_name);
    let _ = write!(contents, "{}\r\n", state.door_user_password().await);
    let _ = write!(contents, "{}\r\n", state.session.get_first_name());
    let _ = write!(contents, "{DOOR_BPS_RATE}\r\n");
    let _ = write!(contents, "{DOOR_COM_PORT}\r\n");
    let _ = write!(contents, "{}\r\n", state.session.minutes_left()); // User Time Left
    let _ = write!(contents, "{}\r\n", Local::now().time().num_seconds_from_midnight()); // Seconds since midnight (now)
    contents.push_str("C:\\SFBBS\\\r\n"); // Spitfire Directory ?
    let graphics_mode = match state.session.disp_options.grapics_mode {
        GraphicsMode::Ctty => "FALSE",
        _ => "TRUE",
    };
    let _ = write!(contents, "{graphics_mode}\r\n");
    let _ = write!(contents, "{}\r\n", state.session.cur_security);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_uploads);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.num_downloads);
    let _ = write!(contents, "{}\r\n", state.session.minutes_left());
    let _ = write!(contents, "{}\r\n", state.session.login_date.time().num_seconds_from_midnight()); // Secs since midnight (logon)
    contents.push_str("0\r\n"); // Extra time in seconds
    contents.push_str("FALSȨ\r\n"); // Sysop next
    contents.push_str("FALSȨ\r\n"); // From Front-end
    let _ = write!(contents, "{}\r\n", if state.session.is_local { "TRUE" } else { "FALSE" });
    let _ = write!(contents, "{DOOR_BPS_RATE}\r\n");
    contents.push_str("FALSE\r\n"); // Error correcting connection
    let _ = write!(contents, "{}\r\n", state.session.current_conference_number);
    contents.push_str("1\r\n"); // Last File Area
    let _ = write!(contents, "{}\r\n", state.node + 1);

    contents.push_str("32768\r\n"); // Downloads allowed per day
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.today_num_downloads);
    contents.push_str("1000000\r\n"); // Download bytes allowed/day
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.today_dnld_bytes);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.total_upld_bytes / 1024);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().stats.total_dnld_bytes / 1024);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().home_voice_phone);
    let _ = write!(contents, "{}\r\n", state.session.current_user.as_ref().unwrap().city_or_state);

    contents.push_str("3600\r\n"); // Minutes Allowed per day
    contents.push_str("FALSE\r\n"); // ?
    contents.push_str("FALSE\r\n"); // ?
    contents.push_str("32767\r\n"); // ?
    contents.push_str("1\r\n"); // COM PORT IRQ Number
    contents.push_str("1000\r\n"); // Serial I/O Port Number
    contents.push_str("00-00-80\r\n"); // Subscription Date

    let path = path.join("SFDOORS.DAT");
    log::info!("create SFDOORS.DAT: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

/*
# Source: IcyBoard/From what I guessed out of a Spitfire BBS install.

1                # User number
Omni Brain       # User name
SECRET           # Password
Omni             # First name
0                # Baud Rate (0 = Local)
1                # COM Port
29               # Time left in minutes
49347            # Now time (secs since midnight)
C:\SFBBS\        # Spitfire Home Path
FALSE            # Graphics TRUE/FALSE
255              # Security level
0                # Total uploads
0                # Total downloads
29               # Minutes allowed this call
49328            # Logon time (secs since midnight)
0                # Extra time in seconds
FALSE            # Sysop Next TRUE/FALSE
FALSE            # Unknown
TRUE             # NOT DTE Locked TRUE/FALSE
2400             # DTE Rate
FALSE            # Unknown
1                # Message Conference
1                # Last File Area
1                # Node number
20               # Downloads allowed per day
0                # Downloads today
1000000          # Download kb allowed per day
0                # Downloaded kb today
0                # Kbytes uploaded
0                # Kbytes downloaded
412-343-1243     # Phone Number
Fwefwefwe        # Address
60               # Minutes allowed per day
FALSE            # Unknown
FALSE            # Unknown
32767            # Unknown
4                # COM Port IRQ Number
1000             # Serial I/O Port Number, usually hex (1000=3E8, 760=2F8)
00-00-80         # Subscription Date (00-00-80 == OFF)

*/
