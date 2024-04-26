use crate::{
    telnet::{TermCaps, TerminalEmulation},
    NetError,
};

use super::{encode_emsi, get_crc32string, get_length_string, EMSI_2ACK};

/// The ICI packet is used by the Client to transmit its configuration
/// and Server-related information to the Server. It contains Server
/// parameters, Client options, and Client capabilities.
/// Note that the information in the `EMSI_ICI` packet may not exceed 2,048 bytes.
pub struct ICIUserSettings {
    ///  The name of the user (Client). This must be treated case insensitively by the Server.
    pub name: String,

    ///  The alias (AKA) of the user (Client). This must be treated case insensitively by the Server.
    pub alias: String,

    /// The password for the user. This must be treated case insensitively by the Server.
    pub password: String,

    /// The geographical location of the user, ie. Stockholm, Sweden.
    pub location: String,

    /// Unformatted data and voice telephone numbers of the user. Unformatted
    /// is defined as the full telephone number, including country and local
    /// area code. Eg. 46-8-90510 is a telephone number in Stockholm, Sweden.
    pub data_phone: String,

    /// Unformatted data and voice telephone numbers of the user. Unformatted
    /// is defined as the full telephone number, including country and local
    /// area code. Eg. 46-8-90510 is a telephone number in Stockholm, Sweden.
    pub voice_phone: String,

    /// Hexadecimal string representing a long integer containing the birth-date of the user in UNIX notation (number of seconds since midnight,
    /// Jan 1 1970). This must be treated case insensitively by the Server
    pub birth_date: String,
}

pub struct ICITerminalSettings {
    /// Consisting of four sub-fields separated by commas, this field
    /// contains from left to right: The requested terminal emulation
    /// protocol, the number of rows of the user's CRT, the number of columns
    /// of the user's CRT, and the number of ASCII NUL (00H) characters the
    /// user's software requires to be transmitted between each line of text.
    ///
    /// The following terminal emulation protocols are defined:
    ///
    ///  AVT0    AVATAR/0+. Used in conjunction with ANSI. If AVT0 is
    ///          specified by the Client, support for ANSI X3.64 emulation
    ///          should be assumed to be present.
    ///  ANSI    ANSI X3.64
    ///  VT52    DEC VT52
    ///  VT100   DEC VT100
    ///  TTY     No terminal emulation, also referred to as RAW mode.
    pub term_caps: TermCaps,

    /// The file transfer protocol option specifies the preferred method of
    /// transferring files between the Client and the Server in either
    /// direction. The Client presents all transfer protocols it is capable
    /// of supporting and the Server chooses the most appropriate protocol.
    ///
    ///     DZA*    DirectZAP (Zmodem variant)
    ///     ZAP     ZedZap (Zmodem variant)
    ///     ZMO     Zmodem w/1,024 byte data packets
    ///     SLK     SEAlink
    ///     KER     Kermit
    ///
    /// (*) DirectZAP is a variant of ZedZap. The difference is that the
    /// transmitter only escapes CAN (18H). It is not recommended to use the
    /// DirectZAP protocol when the Client and the Server are connected via a
    /// packet switching network, or via another layer sensitive to control
    /// characters such as XON and XOFF.
    pub protocols: String,

    /// The capabilities of the user's software. If more than one capability
    /// is listed, each capability is separated by a comma.
    /// The following capability codes are defined:
    ///     CHT     Can do full-screen on-line conversation (CHAT).
    ///     MNU     Can do ASCII image download (see ISM packet).
    ///     TAB     Can handle TAB (ASCII 09H) characters.
    ///     ASCII8  Can handle 8-bit IBM PC ASCII characters.
    pub can_chat: bool,
    pub can_download_ascii: bool,
    pub can_tab_char: bool,
    pub can_ascii8: bool,

    /// The name, version number, and optionally the serial number of the
    /// user's software. Eg. {FrontDoor,2.00,AE000001}.
    pub software: String,

    /// Used for character translation between the Server and the Client.
    /// This field has not been completely defined yet and should always be
    /// transmitted as {} (empty).
    pub xlattabl: String,
}

impl Default for ICITerminalSettings {
    fn default() -> Self {
        Self {
            term_caps: TermCaps {
                window_size: (80, 25),
                terminal: TerminalEmulation::Ansi,
            },
            protocols: Default::default(),
            can_chat: true,
            can_download_ascii: false,
            can_tab_char: true,
            can_ascii8: true,
            software: Default::default(),
            xlattabl: Default::default(),
        }
    }
}

impl ICITerminalSettings {
    pub fn get_crtdef_string(&self) -> String {
        let term = match self.term_caps.terminal {
            TerminalEmulation::Ansi => "ANSI",
            TerminalEmulation::Avatar => "AVT0",
            _ => "TTY",
        };
        // 0 == the number of ASCII NUL (00H) characters the user's software requires to be transmitted between each line of text.
        format!("{},{},{},0", term, self.term_caps.window_size.1, self.term_caps.window_size.0)
    }

    fn get_cap_string(&self) -> String {
        let mut res = String::new();

        if self.can_chat {
            res.push_str("CHT");
        }

        if self.can_download_ascii {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("MNU");
        }

        if self.can_tab_char {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("TAB");
        }

        if self.can_ascii8 {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("ASCII8");
        }

        res
    }
}

pub struct ICIRequests {
    /// NEWS    Show bulletins, announcements, etc.
    pub news: bool,
    /// MAIL    Check for new mail.
    pub mail: bool,
    /// FILE    Check for new files.
    pub file: bool,
    /// HOT     Hot-Keys.
    pub hot_keys: bool,
    /// CLR     Screen clearing.
    pub clear_screen: bool,
    /// HUSH    Do not disturb.
    pub hush: bool,
    /// MORE    Page pausing, often referred to as "More".
    pub more: bool,
    /// FSED    Full-screen editor.
    pub full_screen_editor: bool,
    /// XPRS    <reserved>.
    pub xprs: bool,
}

impl Default for ICIRequests {
    fn default() -> Self {
        Self {
            news: true,
            mail: Default::default(),
            file: Default::default(),
            hot_keys: true,
            clear_screen: true,
            hush: Default::default(),
            more: true,
            full_screen_editor: true,
            xprs: Default::default(),
        }
    }
}

impl ICIRequests {
    fn get_requests_string(&self) -> String {
        let mut res = String::new();

        if self.hot_keys {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("HOT");
        }

        if self.hush {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("HUSH");
        }

        if self.more {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("MORE");
        }

        if self.full_screen_editor {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("FSED");
        }

        if self.news {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("NEWS");
        }

        if self.mail {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("MAIL");
        }

        if self.file {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("FILE");
        }

        if self.clear_screen {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("CLR");
        }

        if self.xprs {
            if !res.is_empty() {
                res.push(',')
            }
            res.push_str("XPRS");
        }

        res
    }
}

const MAX_SIZE: usize = 2048;

pub fn encode_ici(user_settings: &ICIUserSettings, term_settings: &ICITerminalSettings, requests: &ICIRequests) -> crate::Result<Vec<u8>> {
    // **EMSI_ICI<len><data><crc32><CR>
    let data = encode_emsi(&[
        &user_settings.name,
        &user_settings.alias,
        &user_settings.location,
        &user_settings.data_phone,
        &user_settings.voice_phone,
        &user_settings.password,
        &user_settings.birth_date,
        &term_settings.get_crtdef_string(),
        &term_settings.protocols,
        &term_settings.get_cap_string(),
        &requests.get_requests_string(),
        &term_settings.software,
        &term_settings.xlattabl,
    ])?;

    if data.len() > MAX_SIZE {
        return Err(NetError::MaximumEmsiICIExceeded(data.len()).into());
    }
    let mut result = Vec::new();
    result.extend_from_slice(b"**EMSI_ICI");
    result.extend_from_slice(get_length_string(data.len()).as_bytes());
    result.extend_from_slice(&data);
    result.extend_from_slice(get_crc32string(&result[2..]).as_bytes());
    result.push(b'\r');
    // need to send 2*ACK for the ici to be recognized - see the spec
    result.extend_from_slice(EMSI_2ACK);
    Ok(result)
}
