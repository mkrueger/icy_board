use std::fs;

use crate::{
    Res,
    datetime::IcbDate,
    icy_board::{
        doors::DOOR_COM_PORT,
        state::{GraphicsMode, IcyBoardState},
        user_base::{FSEMode, User},
    },
    tables::{export_cp437_string, import_cp437_string},
};
use chrono::{Timelike, Utc};
pub async fn create_pcboard(state: &IcyBoardState, path: &std::path::Path) -> Res<()> {
    create_pcboard_sys(state, path)?;
    create_user_sys(state, path, "").await?;

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

/// Writes USER.SYS into `path`. `tpa_name` names the third party application the
/// record is meant for, and is empty when no TPA record was asked for.
pub async fn create_user_sys(state: &IcyBoardState, path: &std::path::Path, tpa_name: &str) -> Res<()> {
    let mut contents = Vec::new();

    // HEADER
    contents.extend(u16::to_le_bytes(1530)); // PCBoard version number (i.e. 1500)
    contents.extend(u32::to_le_bytes(state.session.cur_user_id as u32)); // Record number from USER's file
    contents.extend(u16::to_le_bytes(crate::icy_board::users::PcbUserRecord::RECORD_SIZE as u16)); // Size of "fixed" user record (current size)
    contents.extend(u16::to_le_bytes(5)); // SizeOfBitFields
    contents.extend(export_cp437_string(tpa_name, 15, b' ')); // Name of the Third Party Application (if any)
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

/// Reads a USER.SYS back after something else may have changed it.
///
/// Only the fields a door has any business changing are taken over; the rest of
/// the record is what we wrote out ourselves and is left alone.
pub fn read_user_sys(user: &mut User, path: &std::path::Path) -> Res<()> {
    let path = path.join("USER.SYS");
    let contents = fs::read(&path)?;

    // Fixed header, then the user record laid out exactly as `create_user_sys` writes it.
    let mut r = Reader { data: &contents, pos: 36 };

    user.name = r.string(26);
    user.city_or_state = r.string(25);
    r.skip(13); // password - passwords do not come back in from a door
    user.bus_data_phone = r.string(14);
    user.home_voice_phone = r.string(14);
    r.skip(2); // last on date
    r.skip(1); // expert mode
    let protocol = r.u8();
    if protocol.is_ascii_graphic() {
        user.protocol = (protocol as char).to_string();
    }
    let packet_flag = r.u8();
    user.flags.msg_clear = packet_flag & (1 << 1) != 0;
    user.flags.fse_mode = if packet_flag & (1 << 3) == 0 {
        FSEMode::Ask
    } else if packet_flag & (1 << 4) != 0 {
        FSEMode::Yes
    } else {
        FSEMode::No
    };
    user.flags.scroll_msg_body = packet_flag & (1 << 5) != 0;
    user.flags.use_short_filedescr = packet_flag & (1 << 6) != 0;
    user.flags.wide_editor = packet_flag & (1 << 7) != 0;

    r.skip(2); // last DIR scan date
    user.security_level = r.u32().min(u8::MAX as u32) as u8;
    user.stats.num_times_on = r.u16() as u64;
    user.page_len = r.u8() as u16;
    user.stats.num_uploads = r.u16() as u64;
    user.stats.num_downloads = r.u16() as u64;
    user.stats.today_dnld_bytes = r.u32() as u64;
    user.user_comment = r.string(31);
    user.sysop_comment = r.string(31);
    r.skip(4); // daily download bytes, written a second time
    r.skip(4); // elapsed time on
    r.skip(2); // registration expiration date
    r.skip(4); // expired security level
    r.skip(2); // last conference
    user.stats.total_dnld_bytes = r.u32() as u64;
    user.stats.total_upld_bytes = r.u32() as u64;
    r.skip(1); // delete this record
    r.skip(4); // record number
    r.skip(1);
    r.skip(8); // reserved
    user.stats.messages_read = r.u32() as u64;
    user.stats.messages_left = r.u32() as u64;

    if r.u8() != 0 {
        user.alias = r.string(26);
    } else {
        r.skip(26);
    }
    if r.u8() != 0 {
        user.street1 = r.string(51);
        user.street2 = r.string(51);
        user.city = r.string(26);
        user.state = r.string(11);
        user.zip = r.string(11);
        user.country = r.string(16);
    } else {
        r.skip(166);
    }
    r.skip(1); // password support
    if r.u8() != 0 {
        user.verify_answer = r.string(26);
    }
    Ok(())
}

/// Walks a USER.SYS record, treating a truncated file as all zeroes.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, len: usize) -> &[u8] {
        let start = self.pos.min(self.data.len());
        let end = (start + len).min(self.data.len());
        self.pos += len;
        &self.data[start..end]
    }

    fn skip(&mut self, len: usize) {
        self.pos += len;
    }

    fn string(&mut self, len: usize) -> String {
        import_cp437_string(self.take(len), true)
    }

    fn u8(&mut self) -> u8 {
        self.take(1).first().copied().unwrap_or(0)
    }

    fn u16(&mut self) -> u16 {
        let bytes = self.take(2);
        if bytes.len() < 2 { 0 } else { u16::from_le_bytes([bytes[0], bytes[1]]) }
    }

    fn u32(&mut self) -> u32 {
        let bytes = self.take(4);
        if bytes.len() < 4 {
            0
        } else {
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        }
    }
}
