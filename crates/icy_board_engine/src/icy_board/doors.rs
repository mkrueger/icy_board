use std::{
    fs,
    ops::{Deref, DerefMut},
};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue},
    executable::{VariableType, VariableValue},
    tables::export_cp437_string,
    Res,
};

use super::{security::RequiredSecurity, IcyBoardSerializer};
use async_trait::async_trait;
use chrono::{Local, Timelike, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BBSLink {
    pub system_code: String,
    pub auth_code: String,
    pub sheme_code: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum DoorServerAccount {
    BBSLink(BBSLink),
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub enum DoorType {
    #[default]
    Local,
    BBSlink,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq, Debug)]
pub enum DropFile {
    #[default]
    None,
    PCBoard,
    DoorSys,
    Door32Sys,
    DorInfo,
    CallInfo,
    DoorFileSR,
    CurruserBBS,
    // currently unsupported (on request)

    // EXITINFO.BBS  QuickBBS (write/read)
    // CHAIN.TXT     WWIV (write-only)
    // SFDOORS.DAT SpitFire (write-only)
    // TRIBBS.SYS TriBBS (write-only)
    // USERINFO.DAT WildCat!
    // JUMPER.DAT WildCat! 2AM BBS
    // INFO.BBS  Phoenix BBS
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Door {
    pub name: String,
    pub description: String,
    pub password: String,
    pub securiy_level: RequiredSecurity,

    pub door_type: DoorType,
    pub path: String,
    #[serde(default)]
    pub use_shell_execute: bool,

    #[serde(default)]
    pub drop_file: DropFile,
}
impl Door {
    pub fn create_drop_file(&self, state: &super::state::IcyBoardState, path: &std::path::Path, door_number: usize) -> Res<()> {
        match self.drop_file {
            DropFile::None => Ok(()),
            DropFile::PCBoard => create_pcboard_sys(state, path),
            DropFile::DoorSys => create_door_sys(state, path),
            DropFile::Door32Sys => create_door32_sys(state, path),
            DropFile::DorInfo => create_dorinfo(state, path, 1),
            DropFile::CallInfo => create_callinfo_bbs(state, path, door_number),
            DropFile::DoorFileSR => create_doorfile_sr(state, path, door_number),
            DropFile::CurruserBBS => create_curruser_bbs(state, path, door_number),

            _ => {
                log::error!("drop file type currently {:?} unsupported", self.drop_file);
                Err("drop file unsupported".into())
            }
        }
    }
}

fn create_pcboard_sys(state: &super::state::IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = Vec::new();
    contents.extend(b"-1"); // DISPLAY ON
    contents.extend(b" 0"); // Printer OFF
    contents.extend(b" 0"); // Page Bell
    contents.extend(b" 0"); // Caller Alarm OFF
    contents.push(b' '); // Sysop Flag (" ", "N"=sysop next, "X"=exit dos)
    contents.extend(b"-1"); // Error Corrected ON

    if state.session.disp_options.grapics_mode == super::state::GraphicsMode::Ctty {
        contents.push(b'N');
    } else {
        contents.push(b'Y'); // CTY mode
    }
    contents.push(b'U'); // Node Chat Status unavailable
    contents.extend(b"57600"); // DTE Port Speed (5 chars)
    contents.extend(b"Local"); // Connect Speed (5 chars)
    contents.extend(u16::to_le_bytes(state.session.cur_user as u16)); // Users record number
    contents.extend(export_cp437_string(&state.session.get_first_name(), 15, b' ')); // User's First Name (padded to 15 characters)
    contents.extend(export_cp437_string(&"SECRET", 12, b' ')); // User's Password (padded to 12 characters)
    contents.extend(u16::to_le_bytes((state.session.login_date.time().num_seconds_from_midnight() / 60) as u16)); // Time User Logged On (in minutes since midnight)
    contents.extend(u16::to_le_bytes(0)); // Time used so far today (negative number of minutes)
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
    contents.extend(u16::to_le_bytes(999)); // Calculated Minutes Remaining
    contents.push(if state.node > 255 { 255 } else { state.node as u8 }); // Node Number
    contents.extend(b"00:00"); // Event Time
    contents.extend(b" 0"); // Is Event Active - Off
    contents.extend(b"  "); // Reserved
    contents.extend([0, 0, 0, 0]); // Memorized Message Number
    contents.push(0); // Comm Port Number (0=none, 1-8)
    contents.push(0); // Reserved for PCBoard
    contents.push(0); // Unknown
                      // Use ANSI (1 = Yes, 0 = No)
    if state.session.disp_options.grapics_mode == super::state::GraphicsMode::Ctty {
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
    contents.extend(u16::to_le_bytes(state.session.current_conference_number as u16)); // Conference Area user was in (up to 65535)

    contents.push(0); // High Conference Areas the user has joined
    contents.push(0); // High Conference Areas the user has scanned
    contents.extend(u16::to_le_bytes(state.node as u16)); // Node Number if offset 111 is set to 255

    let path = path.join("PCBOARD.SYS");
    log::info!("create pcboard.sys: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

fn create_door_sys(state: &super::state::IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    contents.push_str("COM1:\r\n"); // Communications port (COM0: if local)
    contents.push_str("38400\r\n"); // BPS rate
    contents.push_str("8\r\n"); // Data bits
    contents.push_str(&format!("{}\r\n", state.node + 1)); // Node number
    contents.push_str("Y\r\n"); // Screen display On
    contents.push_str("N\r\n"); // Printer toggle Off
    contents.push_str("N\r\n"); // Page bell Off
    contents.push_str("N\r\n"); // Caller alarm Off
    contents.push_str(&format!("{}\r\n", state.session.user_name)); // User full name
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().city_or_state)); // User location
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().home_voice_phone)); // Home/voice telephone number
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().bus_data_phone)); // Work/data telephone number
    contents.push_str("SECRET\r\n"); // Password (not displayed)
    contents.push_str(&format!("{}\r\n", state.session.cur_security)); // Security level
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.num_times_on)); // User's total number of calls to the system
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.last_on.format("%m/%d/%C"))); // User's last call date
    contents.push_str(&format!("{}\r\n", 999 * 60)); // Seconds remaining this call
    contents.push_str(&format!("{}\r\n", 999)); // Minutes remaining this call

    let emulation = match state.session.disp_options.grapics_mode {
        super::state::GraphicsMode::Ctty => "NG",
        _ => "GR",
    };
    contents.push_str(&format!("{}\r\n", emulation)); //Graphics mode (GR=ANSI, NG=ASCII)
    contents.push_str(&format!("{}\r\n", state.session.page_len)); // Screen length
    contents.push_str(if state.session.expert_mode { "Y\r\n" } else { "N\r\n" }); // User Mode
    contents.push_str("\r\n"); // Always blank
    contents.push_str("\r\n"); // Always blank

    contents.push_str("01/01/99\r\n"); // expiration date
    contents.push_str(&format!("{}\r\n", state.session.cur_user + 1)); // User's record number
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().protocol)); // Default protocol
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.num_uploads)); // User's total number of uploads
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.num_downloads)); // User's total number of downloads
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.today_dnld_bytes / 1024)); // User's daily download kilobytes total
    contents.push_str(&format!("999999\r\n")); // Daily download kilobyte limit
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().birth_date.format("%m/%d/%C"))); // User's date of birth
    contents.push_str("C:\\HOME\r\n"); // Path to the user database files
    contents.push_str("C:\\MSGS\r\n"); // Path to the message database files
    contents.push_str(&format!("{}\r\n", state.board.lock().unwrap().config.sysop.name)); // Sysop name
    contents.push_str(&format!("{}\r\n", state.session.alias_name)); // User's handle (alias)
    contents.push_str("00:00\r\n"); // Next event starting time
    contents.push_str("Y\r\n"); // Error-free connection (Y=Yes N=No)
    contents.push_str("N\r\n"); // Always set to N
    contents.push_str("Y\r\n"); // Always set to Y
    let default_color = match state.board.lock().unwrap().config.color_configuration.default {
        super::icb_config::IcbColor::None => 7,
        super::icb_config::IcbColor::Dos(col) => col % 15,
        super::icb_config::IcbColor::IcyEngine(_) => 7,
    };
    contents.push_str(&format!("{}\r\n", default_color)); // BBS Default fg Color
    contents.push_str("0\r\n"); // Always set to 0
    contents.push_str("01/01/70\r\n"); // Last new files scan date
    contents.push_str(&format!("{}\r\n", state.session.login_date.format("%H:%M"))); // Time of this call
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.last_on.format("%H:%M"))); // Time of last call
    contents.push_str("32768\r\n"); // Always set to 32768
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.today_num_downloads)); // Number of files downloaded today
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.total_upld_bytes / 1024)); // Total kilobytes uploaded
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.total_dnld_bytes / 1024)); // Total kilobytes downloaded
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().user_comment)); // Comment stored in user record
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.total_doors_executed)); // Number of files downloaded today
    contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.messages_left)); // Total number of messages posted

    let path = path.join("DOOR.SYS");
    log::info!("create DOOR.SYS: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

fn create_door32_sys(state: &super::state::IcyBoardState, path: &std::path::Path) -> Res<()> {
    let mut contents = String::new();
    contents.push_str("0\r\n"); // Line 1 : Comm type (0=local, 1=serial, 2=telnet)
    contents.push_str("0\r\n"); // Line 2 : Comm or socket handle
    contents.push_str("115200\r\n"); // Line 3 : Baud rate
    contents.push_str(&format!("Icy Board {}\r\n", crate::VERSION.to_string())); // Line 4 : BBSID (software name and version)
    contents.push_str(&format!("{}\r\n", state.session.cur_user + 1)); // Line 5 : User record position (1-based)
    contents.push_str(&format!("{}\r\n", state.session.user_name)); // Line 6 : User's real name
    contents.push_str(&format!("{}\r\n", state.session.alias_name)); // Line 7 : User's handle/alias
    contents.push_str(&format!("{}\r\n", state.session.cur_security)); // Line 8 : User's security level
    contents.push_str(&format!("{}\r\n", 999)); // Line 9 : User's time left (in minutes)

    let emulation = match state.session.disp_options.grapics_mode {
        super::state::GraphicsMode::Ctty => 0,
        super::state::GraphicsMode::Ansi => 1,
        super::state::GraphicsMode::Graphics => 4,
        super::state::GraphicsMode::Avatar => 2,
        super::state::GraphicsMode::Rip => 3,
    };
    contents.push_str(&format!("{}\r\n", emulation)); // Line 10: Emulation *See Below
                                                      // 0 = Ascii
                                                      // 1 = Ansi
                                                      // 2 = Avatar
                                                      // 3 = RIP
                                                      // 4 = Max Graphics

    contents.push_str(&format!("{}\r\n", state.node + 1)); // Line 11: Current node number

    let path = path.join("door32.sys");
    log::info!("create door32.sys: {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

fn create_dorinfo(state: &super::state::IcyBoardState, path: &std::path::Path, node: i32) -> Res<()> {
    let mut contents = String::new();
    if let Ok(board) = state.board.lock() {
        contents.push_str(&format!("{}\r\n", board.config.board.name)); // System name
        contents.push_str(&format!("{}\r\n", board.users[0].get_first_name())); // Sysop first name
        contents.push_str(&format!("{}\r\n", board.users[0].get_last_name())); // Sysop last name
        contents.push_str("COM0\r\n"); // Communications port in use (COM0 if local)
        contents.push_str("57600 BAUD-R,N,8,1\r\n"); // Communications port settings
        contents.push_str("0\r\n"); // Reserved (always zero)
        contents.push_str(&format!("{}\r\n", state.session.get_first_name())); // User first name
        contents.push_str(&format!("{}\r\n", state.session.get_last_name())); // User last name
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().get_first_name())); // User first name
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().get_last_name())); // User last name
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().city_or_state)); // User location
        let emulation = match state.session.disp_options.grapics_mode {
            super::state::GraphicsMode::Ctty => 0,
            super::state::GraphicsMode::Avatar => 2,
            _ => 1,
        };
        contents.push_str(&format!("{}\r\n", emulation)); // User emulation (0=ASCII, 1=ANSI, 2=AVATAR)
        contents.push_str(&format!("{}\r\n", state.session.cur_security)); // User security level
        contents.push_str(&format!("{}\r\n", 999)); // User time remaining (in minutes)
        contents.push_str("-1\r\n"); // EOF

        let path = path.join("DORINFO1.DEF");
        log::info!("create dorinfo1.def: {}", path.display());
        fs::write(path, contents)?;
    } else {
        return Err("Board not found".into());
    }
    Ok(())
}

fn create_callinfo_bbs(state: &super::state::IcyBoardState, path: &std::path::Path, door_number: usize) -> Res<()> {
    let mut contents = String::new();
    if let Ok(board) = state.board.lock() {
        contents.push_str(&format!("{}\r\n", state.session.user_name)); // User Name
        contents.push_str("5\r\n"); // Baud 300=1, 1200=2, 2400=0, 9600=3, 19200=4, Local=5
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().city_or_state)); // Calling From
        contents.push_str(&format!("{}\r\n", state.session.cur_security)); // User security level
        contents.push_str("999\r\n"); // User Time Left
        let emulation = match state.session.disp_options.grapics_mode {
            super::state::GraphicsMode::Ctty => "MONO",
            _ => "COLOR",
        };
        contents.push_str(&format!("{}\r\n", emulation)); // Color or Mono
        contents.push_str("SECRET\r\n"); // Password
        contents.push_str(&format!("{}\r\n", state.session.cur_user + 1)); // User Reference Number
        contents.push_str("0\r\n"); // User Time On
        contents.push_str(&format!("{}\r\n", state.session.login_date.format("%H:%M"))); // Time Str
        contents.push_str(&format!("{}\r\n", state.session.login_date.format("%H:%M %m/%d%/%C"))); // Time-Date
        contents.push_str("\r\n"); // Conference Joined
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.today_num_downloads)); // Daily Downloads
        contents.push_str(&format!("{}\r\n", 999)); // Max Downloads
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.today_dnld_bytes / 1024)); // Daily Download K
        contents.push_str(&format!("{}\r\n", 999 * 1024)); // Max Downloads KB
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().home_voice_phone)); // Phone Number
        contents.push_str(&format!("{}\r\n", Local::now().format("%m/%d%/%C %H:%M"))); // Date-Time
        let emulation = if state.session.expert_mode { "EXPERT" } else { "NOVICE" };
        contents.push_str(&format!("{}\r\n", emulation)); // Novice or Expert
        contents.push_str("All\r\n"); // Transfer Method  All, Ymodem, Ymodem/G, Xmodem, Xmodem/CRC, Xmodem-1K, Xmodem-1K/G, Ascii
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.last_on.format("%m/%d%/%C"))); // Last New Date
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.num_times_on)); // Times on
        contents.push_str(&format!("{}\r\n", state.session.page_len)); // Lines per Page
        contents.push_str("42\r\n"); // Highest Message Read
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.num_uploads)); // Uploads
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().stats.num_downloads)); // Downloads
        contents.push_str("8\r\n"); // Databits (7 or 8)
        contents.push_str("LOCAL\r\n"); // LOCAL or REMOTE
        contents.push_str("COM0\r\n"); // COM Port
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().birth_date.format("%m/%d%/%C"))); // Birth Date
        contents.push_str("38400\r\n"); // Com Port Speed
        contents.push_str("TRUE\r\n"); // Already Connected
        contents.push_str("Normal Connection \r\n"); // MNP/ARQ or Normal Connection
        contents.push_str(&format!("{}\r\n", Utc::now().format("%m/%d%/%C %H:%M"))); // Date Time (Global)
        contents.push_str(&format!("{}\r\n", state.node + 1)); // Node ID
        contents.push_str(&format!("{}\r\n", door_number)); // Door Number

        let path = path.join("CALLINFO.BBS");
        log::info!("create callinfo.bbs: {}", path.display());
        fs::write(path, contents)?;
    } else {
        return Err("Board not found".into());
    }
    Ok(())
}

fn create_doorfile_sr(state: &super::state::IcyBoardState, path: &std::path::Path, door_number: usize) -> Res<()> {
    let mut contents = String::new();
    if let Ok(board) = state.board.lock() {
        contents.push_str(&format!("{}\r\n", state.session.get_username_or_alias())); // Complete name or handle of user

        let emulation = match state.session.disp_options.grapics_mode {
            super::state::GraphicsMode::Ctty => "0",
            _ => "1",
        };
        contents.push_str(&format!("{}\r\n", emulation)); // ANSI status:  1 = yes, 0 = no, -1 = don't know
        contents.push_str("1\r\n"); // IBM Graphic characters:  1 = yes, 0 = no, -1 = unknown
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().city_or_state)); // Calling From
        contents.push_str(&format!("{}\r\n", state.session.page_len)); // Page length of screen, in lines.  Assume 25 if unknown
        contents.push_str("38400\r\n"); // Baud Rate
        contents.push_str("0\r\n"); // Com Port:  1, 2, 3, or 4; 0 if local.
        contents.push_str("-1\r\n"); // Time Limit:  (in minutes); -1 if unknown.
        contents.push_str(&format!("{}\r\n", state.session.user_name)); // Real name (the same as line 1 if not known)

        let path = path.join("DOORFILE.SR");
        log::info!("create DOORFILE.SR: {}", path.display());
        fs::write(path, contents)?;
    } else {
        return Err("Board not found".into());
    }
    Ok(())
}

fn create_curruser_bbs(state: &super::state::IcyBoardState, path: &std::path::Path, door_number: usize) -> Res<()> {
    let mut contents = String::new();
    if let Ok(board) = state.board.lock() {
        contents.push_str(&format!("{}\r\n", state.session.user_name));
        contents.push_str(&format!("{}\r\n", state.session.cur_security));
        contents.push_str(&format!("{}\r\n", state.session.cur_user));
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().home_voice_phone));
        contents.push_str(&format!("{}\r\n", state.current_user.as_ref().unwrap().city_or_state));
        contents.push_str("1\r\n");
        contents.push_str("38400\r\n"); // Baud Rate
        contents.push_str("N\r\n");
        contents.push_str("8\r\n");
        contents.push_str("1\r\n");
        contents.push_str("\r\n");
        contents.push_str("DOORM.MNU\r\n");
        contents.push_str("999\r\n");

        let emulation = match state.session.disp_options.grapics_mode {
            super::state::GraphicsMode::Ctty => "NONE",
            super::state::GraphicsMode::Ansi => "IBM",
            _ => "ANSI",
        };
        contents.push_str(&format!("{}\r\n", emulation));

        let path = path.join("CURRUSER.BBS");
        log::info!("create CURRUSER.BBS: {}", path.display());
        fs::write(path, contents)?;
    } else {
        return Err("Board not found".into());
    }
    Ok(())
}

impl UserData for Door {
    const TYPE_NAME: &'static str = "Door";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(NAME.clone(), VariableType::String, false);
        registry.add_property(DESCRIPTION.clone(), VariableType::String, false);
        registry.add_property(PASSWORD.clone(), VariableType::String, false);
        registry.add_function(HAS_ACCESS.clone(), Vec::new(), VariableType::Boolean);
    }
}

#[async_trait]
impl UserDataValue for Door {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *NAME {
            return Ok(VariableValue::new_string(self.name.clone()));
        }
        if *name == *DESCRIPTION {
            return Ok(VariableValue::new_string(self.description.clone()));
        }
        if *name == *PASSWORD {
            return Ok(VariableValue::new_string(self.password.clone()));
        }
        log::error!("Invalid user data call on Door ({})", name);
        Ok(VariableValue::new_int(-1))
    }

    fn set_property_value(&mut self, _vm: &mut crate::vm::VirtualMachine, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        log::error!("Invalid set field call on Door ({})", name);
        Ok(())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *HAS_ACCESS {
            let res = self.securiy_level.user_can_access(&vm.icy_board_state.session);
            return Ok(VariableValue::new_bool(res));
        }
        log::error!("Invalid function call on Door ({})", name);
        Err("Function not found".into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        log::error!("Invalid method call on Door ({})", name);
        Err("Function not found".into())
    }
}

lazy_static::lazy_static! {
    pub static ref NAME: unicase::Ascii<String> = unicase::Ascii::new("Name".to_string());
    pub static ref DESCRIPTION: unicase::Ascii<String> = unicase::Ascii::new("Description".to_string());
    pub static ref PASSWORD: unicase::Ascii<String> = unicase::Ascii::new("Password".to_string());
    pub static ref HAS_ACCESS: unicase::Ascii<String> = unicase::Ascii::new("HasAccess".to_string());
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct DoorList {
    #[serde(rename = "account")]
    pub accounts: Vec<DoorServerAccount>,

    #[serde(rename = "door")]
    pub doors: Vec<Door>,
}

impl Deref for DoorList {
    type Target = Vec<Door>;
    fn deref(&self) -> &Self::Target {
        &self.doors
    }
}

impl DerefMut for DoorList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.doors
    }
}

impl IcyBoardSerializer for DoorList {
    const FILE_TYPE: &'static str = "doors";
}
