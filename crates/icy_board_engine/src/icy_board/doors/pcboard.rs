use std::fs;

use crate::{
    Res,
    datetime::IcbDate,
    icy_board::{
        doors::DOOR_COM_PORT,
        state::{GraphicsMode, IcyBoardState},
        user_base::FSEMode,
    },
    tables::export_cp437_string,
};
use chrono::{Timelike, Utc};
pub async fn create_pcboard(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    create_pcboard_sys(state, path)?;
    create_user_sys(state, path).await?;

    Ok(())
}

fn create_pcboard_sys(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = Vec::new();
    contents.extend(b"-1"); // DISPLAY ON
    contents.extend(b" 0"); // Printer OFF
    contents.extend(b" 0"); // Page Bell
    contents.extend(b" 0"); // Caller Alarm OFF
    contents.push(b' '); // Sysop Flag (" ", "N"=sysop next, "X"=exit dos)
    contents.extend(b"-1"); // Error Corrected ON

    if state.session.disp_options.grapics_mode == GraphicsMode::Ctty {
        contents.push(b'N');
    } else {
        contents.push(b'Y'); // CTY mode
    }
    contents.push(b'U'); // Node Chat Status unavailable
    contents.extend(b"57600"); // DTE Port Speed (5 chars)
    contents.extend(b"Local"); // Connect Speed (5 chars)
    contents.extend(u16::to_le_bytes(state.session.cur_user_id as u16)); // Users record number
    contents.extend(export_cp437_string(&state.session.get_first_name(), 15, b' ')); // User's First Name (padded to 15 characters)
    contents.extend(export_cp437_string(&"SECRET", 12, b' ')); // User's Password (padded to 12 characters)
    contents.extend(u16::to_le_bytes((state.session.login_date.time().num_seconds_from_midnight() / 60) as u16)); // Time User Logged On (in minutes since midnight)
    contents.extend(u16::to_le_bytes((Utc::now() - state.session.login_date).num_minutes() as u16)); // Time used so far today (negative number of minutes)
    contents.extend(state.session.login_date.format("%H:%M").to_string().as_bytes()); // Time User Logged On (in "HH:MM" format)
    contents.extend(u16::to_le_bytes(32767)); // Time Allowed On (from PWRD file)
    contents.extend(u16::to_le_bytes(32767)); // Allowed K-Bytes for Download
    contents.push(if state.session.current_conference_number <= 255 {
        state.session.current_conference_number as u8
    } else {
        0
    }); // Conference Area user was in (if <= 255)

    contents.extend([0, 0, 0, 0, 0]); // Conference Areas the user has joined this session - 5 bytes
    contents.extend([0, 0, 0, 0, 0]); // Conference Areas the user has scanned this session - 5 bytes
    contents.extend(u16::to_le_bytes(state.session.current_conference.add_conference_time as u16)); // Conference Add Time in minutes
    contents.extend(u16::to_le_bytes(0)); // Upload/Sysop CHAT Credit Minutes
    contents.extend(export_cp437_string(&state.session.language, 4, b' ')); // Language Extension
    contents.extend(export_cp437_string(&state.session.user_name, 25, b' ')); // User's Full Name (padded to 25 characters)
    contents.extend(u16::to_le_bytes(state.session.minutes_left() as u16)); // Calculated Minutes Remaining
    contents.push(if state.node > 255 { 255 } else { state.node as u8 }); // Node Number
    contents.extend(b"00:00"); // Event Time
    contents.extend(b" 0"); // Is Event Active - Off
    contents.extend(b"  "); // Reserved
    contents.extend([0, 0, 0, 0]); // Memorized Message Number
    contents.push(DOOR_COM_PORT); // Comm Port Number (0=none, 1-8)
    contents.push(0); // Reserved for PCBoard
    contents.push(0); // Unknown
    // Use ANSI (1 = Yes, 0 = No)
    if state.session.disp_options.grapics_mode == GraphicsMode::Ctty {
        contents.push(0);
    } else {
        contents.push(1);
    }

    contents.extend(u16::to_le_bytes(1)); // Country Code
    contents.extend(u16::to_le_bytes(1)); // Code Page
    contents.push(state.session.yes_char as u8);
    contents.push(state.session.no_char as u8);
    contents.push(0); // Language 0 = None
    contents.extend([0, 0, 0]); // Reserved
    contents.push(0); // Caller Exited to DOS - NO
    contents.push(0); // Reserved for PCBoard
    contents.push(0); // Stop Uploads - NO
    contents.extend(u16::to_le_bytes(state.session.current_conference_number)); // Conference Area user was in (up to 65535)

    contents.push(0); // High Conference Areas the user has joined
    contents.push(0); // High Conference Areas the user has scanned
    contents.extend(u16::to_le_bytes(state.node as u16)); // Node Number if offset 111 is set to 255

    let path = path.join("PCBOARD.SYS");
    log::info!("create PCBOARD.SYS: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

async fn create_user_sys(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = Vec::new();

    // HEADER
    contents.extend(u16::to_le_bytes(1530)); // PCBoard version number (i.e. 1500)
    contents.extend(u32::to_le_bytes(state.session.cur_user_id as u32)); // Record number from USER's file
    contents.extend(u16::to_le_bytes(crate::icy_board::users::PcbUserRecord::RECORD_SIZE as u16)); // Size of "fixed" user record (current size)
    contents.extend(u16::to_le_bytes(5)); // SizeOfBitFields
    contents.extend(export_cp437_string("", 15, b' ')); // Name of the Third Party Application (if any)
    contents.extend(u16::to_le_bytes(0)); // Version number for the application (if any)
    contents.extend(u16::to_le_bytes(0)); // Size of a "fixed length" record (if any)
    contents.extend(u16::to_le_bytes(0)); // Size of each conference record (if any)
    contents.extend(u32::to_le_bytes(0)); // Offset of AppRec into USERS.INF record (if any)
    contents.push(0); //  TRUE if the USERS.SYS file has been updated

    if let Some(user) = &state.session.current_user {
        contents.extend(export_cp437_string(&user.name, 26, 0));
        contents.extend(export_cp437_string(&user.city_or_state, 25, 0));
        contents.extend(export_cp437_string(&state.door_user_password().await, 13, 0));
        contents.extend(export_cp437_string(&user.bus_data_phone, 14, 0));
        contents.extend(export_cp437_string(&user.home_voice_phone, 14, 0));
        contents.extend(u16::to_le_bytes(IcbDate::from_utc(&user.stats.last_on).to_pcboard_date() as u16));
        if state.session.expert_mode() {
            contents.push(1);
        } else {
            contents.push(0);
        }
        contents.push(user.protocol.chars().next().unwrap_or(' ') as u8);

        let mut packet_flag = 0;

        if user.flags.msg_clear {
            packet_flag |= 1 << 1;
        }

        match user.flags.fse_mode {
            FSEMode::Yes => {
                packet_flag |= 1 << 3;
                packet_flag |= 1 << 4;
            }
            FSEMode::No => {
                packet_flag |= 1 << 3;
            }
            FSEMode::Ask => {}
        }
        if user.flags.scroll_msg_body {
            packet_flag |= 1 << 5;
        }
        if user.flags.use_short_filedescr {
            packet_flag |= 1 << 6;
        }
        if user.flags.wide_editor {
            packet_flag |= 1 << 7;
        }
        contents.push(packet_flag);

        contents.extend(u16::to_le_bytes(0)); // Date for Last DIR Scan (most recent file)
        contents.extend(u32::to_le_bytes(state.session.cur_security as u32)); // Security Level
        contents.extend(u16::to_le_bytes(user.stats.num_times_on as u16));
        contents.push(state.session.page_len as u8);
        contents.extend(u16::to_le_bytes(user.stats.num_uploads as u16));
        contents.extend(u16::to_le_bytes(user.stats.num_downloads as u16));
        contents.extend(u32::to_le_bytes(user.stats.today_dnld_bytes as u32));
        contents.extend(export_cp437_string(&user.user_comment, 31, 0));
        contents.extend(export_cp437_string(&user.sysop_comment, 31, 0));
        contents.extend(u32::to_le_bytes(user.stats.today_dnld_bytes as u32));
        contents.extend(u32::to_le_bytes((Utc::now() - state.session.login_date).num_minutes() as u32));
        contents.extend(u16::to_le_bytes(0)); // Julian date for Registration Expiration Date
        contents.extend(u32::to_le_bytes(0)); // Expired Security Level
        contents.extend(u16::to_le_bytes(0)); // LastConference
        contents.extend(u32::to_le_bytes(user.stats.total_dnld_bytes as u32));
        contents.extend(u32::to_le_bytes(user.stats.total_upld_bytes as u32));
        contents.push(0); //1=delete this record, 0=keep
        contents.extend(u32::to_le_bytes(state.session.cur_user_id as u32)); // Record Number in USERS.INF file
        contents.push(0);
        contents.extend(&[0; 8]); // Reserved
        contents.extend(u32::to_le_bytes(user.stats.messages_read as u32));
        contents.extend(u32::to_le_bytes(user.stats.messages_left as u32));
        contents.push(1); // Alias support
        contents.extend(export_cp437_string(&user.alias, 26, 0));
        contents.push(1); // AddressSupport
        contents.extend(export_cp437_string(&user.street1, 51, 0));
        contents.extend(export_cp437_string(&user.street2, 51, 0));
        contents.extend(export_cp437_string(&user.city, 26, 0));
        contents.extend(export_cp437_string(&user.state, 11, 0));
        contents.extend(export_cp437_string(&user.zip, 11, 0));
        contents.extend(export_cp437_string(&user.country, 16, 0));

        contents.push(0); // PasswordSupport
        contents.push(1); // VerifySupport
        contents.extend(export_cp437_string(&user.verify_answer, 26, 0));
        contents.push(0); // StatsSuppport
        contents.push(0); // NotesSupport
        contents.push(0); // AccountSupport
        contents.push(0); // QwkSupport
    }

    let path = path.join("USER.SYS");
    log::info!("create USER.SYS: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}
