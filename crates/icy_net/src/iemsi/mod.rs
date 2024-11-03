#![allow(dead_code, clippy::wildcard_imports, clippy::needless_range_loop)]
// IEMSI autologin implementation http://ftsc.org/docs/fsc-0056.001
pub mod dat;
pub use dat::*;

pub mod ici;
pub use ici::*;

pub mod isi;
pub use isi::*;

pub mod server;
pub use server::*;

use crate::{
    crc::{get_crc16, get_crc32, update_crc32},
    NetError,
};

/// EMSI Inquiry is transmitted by the calling system to identify it as
/// EMSI capable. If an `EMSI_REQ` sequence is received in response, it is
/// safe to assume the answering system to be EMSI capable.
pub const EMSI_INQ: &[u8; 15] = b"**EMSI_INQC816\r";

/// EMSI Request is transmitted by the answering system in response to an
/// EMSI Inquiry sequence. It should also be transmitted prior to or
/// immediately following the answering system has identified itself by
/// transmitting its program name and/or banner. If the calling system
/// receives an EMSI Request sequence, it can safely assume that the
/// answering system is EMSI capable.
pub const EMSI_REQ: &[u8; 15] = b"**EMSI_REQA77E\r";

/// EMSI Client is used by terminal emulation software to force a mailer
/// front-end to bypass any unnecessary mail session negotiation and
/// treat the call as an incoming human caller. The `EMSI_CLI` sequence may
/// not be issued by any software attempting to establish a mail session
/// between two systems and must only be acted upon by an answering
/// system.
pub const EMSI_CLI: &[u8; 15] = b"**EMSI_CLIFA8C\r";

/// EMSI Heartbeat is used to prevent unnecessary timeouts from occurring
/// while attempting to handshake. It is most commonly used when the
/// answering system turns around to transmit its `EMSI_DAT` packet. It is
/// quite normal that any of the timers of the calling system (which at
/// this stage is waiting for an `EMSI_DAT` packet) expires while the
/// answering system is processing the recently received `EMSI_DAT` packet.
pub const EMSI_HBT: &[u8; 15] = b"**EMSI_HBTEAEE\r";

/// EMSI ACK is transmitted by either system as a positive
/// acknowledgement of the valid receipt of a `EMSI_DAT` packet. This should
/// only be used as a response to `EMSI_DAT` and not any other packet.
/// Redundant `EMSI_ACK` sequences should be ignored.
pub const EMSI_ACK: &[u8; 15] = b"**EMSI_ACKA490\r";
pub const EMSI_2ACK: &[u8; 30] = b"**EMSI_ACKA490\r**EMSI_ACKA490\r";

/// EMSI NAK is transmitted by either system as a negative
/// acknowledgement of the valid receipt of a `EMSI_DAT` packet. This
/// should only be used as a response to `EMSI_DAT` and not any other
/// packet. Redundant `EMSI_NAK` packets should be ignored.
pub const EMSI_NAK: &[u8; 15] = b"**EMSI_NAKEEC3\r";
pub const EMSI_NAK_WITH_CLEAR: &[u8; 30] = b"**EMSI_NAKEEC3\r              \r";

/// Similar to `EMSI_REQ` which is used by mailer software to negotiate a
/// mail session. IRQ identifies the Server as being capable of
/// negotiating an IEMSI session. When the Client detects an IRQ sequence
/// in its inbound data stream, it attempts to negotiate an IEMSI
/// session.
pub const EMSI_IRQ: &[u8; 15] = b"**EMSI_IRQ8E08\r";
pub const EMSI_IRQ_WITH_CLEAR: &[u8; 30] = b"**EMSI_IRQ8E08\r              \r";

/// The IIR (Interactive Interrupt Request) sequence is used by either
/// Client or Server to abort the current negotiation. This could be
/// during the initial IEMSI handshake or during other interactions
/// between the Client and the Server.
pub const EMSI_IIR: &[u8; 15] = b"**EMSI_IIR61E2\r";

/// The CHT sequence is used by the Server to instruct the Client
/// software to enter its full-screen conversation mode function (CHAT).
/// Whether or not the Client software supports this is indicated in the
/// ICI packet.
///
/// If the Server transmits this sequence to the Client, it must wait for
/// an `EMSI_ACK` prior to engaging its conversation mode. If no `EMSI_ACK`
/// sequence is received with ten seconds, it is safe to assume that the
/// Client does not support `EMSI_CHT`. If, however, an `EMSI_NAK` sequence
/// is received from the Client, the Server must re-transmit the
/// `EMSI_CHT` sequence. Once the on-line conversation function has been
/// sucessfully activated, the Server must not echo any received
/// characters back to the Client.
pub const EMSI_CHT: &[u8; 15] = b"**EMSI_CHTF5D4\r";

pub fn get_crc32string(block: &[u8]) -> String {
    let crc = get_crc32(block);
    format!("{:08X}", !crc)
}

pub fn get_crc16string(block: &[u8]) -> String {
    let crc = get_crc16(block);
    format!("{crc:04X}")
}

pub fn get_length_string(len: usize) -> String {
    format!("{len:04X}")
}

/// The ISM packet is used to transfer ASCII images from the Server to
/// the Client. These images can then be recalled by the Client when
/// the Server needs to display a previously displayed image.
/// This will be further described in future revisions of this document.
/// SPOILER: There will me no future revisions :)
pub fn _encode_ism(data: &[u8]) -> Vec<u8> {
    let mut block = Vec::new();
    block.extend_from_slice(format!("EMSI_ISM{:X}", data.len()).as_bytes());
    block.extend_from_slice(data);
    let crc = get_crc16(&block);

    let mut result = Vec::new();
    result.extend_from_slice(b"**");
    result.extend_from_slice(&block);
    result.push((crc >> 8) as u8);
    result.push(u8::try_from(crc & 0xFF).unwrap());
    result.push(b'\r');
    result
}

impl Default for ICIUserSettings {
    fn default() -> Self {
        Self {
            alias: String::default(),
            location: String::default(),
            data_phone: String::default(),
            voice_phone: String::default(),
            birth_date: String::default(),

            name: String::default(),
            password: String::default(),
        }
    }
}

#[derive(Default)]
pub struct IEmsi {
    irq_requested: bool,
    nak_requested: bool,
    pub retries: usize,
    pub isi: Option<EmsiISI>,

    stars_read: i32,
    irq_seq: usize,
    isi_seq: usize,
    nak_seq: usize,
    isi_len: usize,
    isi_crc: usize,
    isi_check_crc: u32,
    pub got_invalid_isi: bool,
    isi_data: Vec<u8>,

    pub user_settings: ICIUserSettings,
    pub terminal_settings: ICITerminalSettings,
    pub aborted: bool,
    logged_in: bool,
}

// **EMSI_ISI<len><data><crc32><CR>
const ISI_START: &[u8; 8] = b"EMSI_ISI";

impl IEmsi {
    pub fn parse_char(&mut self, ch: u8) -> crate::Result<bool> {
        if self.stars_read >= 2 {
            if self.isi_seq > 7 {
                match self.isi_seq {
                    8..=11 => {
                        self.isi_check_crc = update_crc32(self.isi_check_crc, ch);
                        self.isi_len = self.isi_len * 16 + get_value(ch);
                        self.isi_seq += 1;
                        return Ok(false);
                    }
                    12.. => {
                        if self.isi_seq < self.isi_len + 12 {
                            // Read data
                            self.isi_check_crc = update_crc32(self.isi_check_crc, ch);
                            self.isi_data.push(ch);
                        } else if self.isi_seq < self.isi_len + 12 + 8 {
                            // Read CRC
                            self.isi_crc = self.isi_crc * 16 + get_value(ch);
                        } else if self.isi_seq >= self.isi_len + 12 + 8 {
                            // end - should be marked with b'\r'
                            if ch == b'\r' {
                                if self.isi_crc == self.isi_check_crc as usize {
                                    let group = parse_emsi_blocks(&self.isi_data)?;
                                    if group.len() == 8 {
                                        // valid ISI !!!
                                        self.isi = Some(EmsiISI {
                                            id: group[0].clone(),
                                            name: group[1].clone(),
                                            location: group[2].clone(),
                                            operator: group[3].clone(),
                                            localtime: group[4].clone(),
                                            notice: group[5].clone(),
                                            wait: group[6].clone(),
                                            capabilities: group[7].clone(),
                                        });
                                        self.stars_read = 0;
                                        self.reset_sequences();
                                        return Ok(true);
                                    }
                                    self.got_invalid_isi = true;
                                } else {
                                    self.got_invalid_isi = true;
                                }
                            }
                            self.stars_read = 0;
                            self.reset_sequences();
                        }
                        self.isi_seq += 1;
                        return Ok(false);
                    }
                    _ => {}
                }
                return Ok(false);
            }
            let mut got_seq = false;

            if ch == ISI_START[self.isi_seq] {
                self.isi_check_crc = update_crc32(self.isi_check_crc, ch);
                self.isi_seq += 1;
                self.isi_len = 0;
                got_seq = true;
            } else {
                self.isi_seq = 0;
            }

            if ch == EMSI_NAK[2 + self.nak_seq] {
                self.nak_seq += 1;
                if self.nak_seq + 2 >= EMSI_IRQ.len() {
                    self.nak_requested = true;
                    self.stars_read = 0;
                    self.reset_sequences();
                }
                got_seq = true;
            } else {
                self.nak_seq = 0;
            }

            if ch == EMSI_IRQ[2 + self.irq_seq] {
                self.irq_seq += 1;
                if self.irq_seq + 2 >= EMSI_NAK.len() {
                    self.irq_requested = true;
                    self.stars_read = 0;
                    self.reset_sequences();
                }
                got_seq = true;
            } else {
                self.irq_seq = 0;
            }

            if got_seq {
                return Ok(false);
            }
            self.stars_read = 0;
            self.reset_sequences();
        }

        if ch == b'*' {
            self.stars_read += 1;
            self.reset_sequences();
            return Ok(false);
        }
        self.stars_read = 0;

        Ok(false)
    }

    /// Tries to login with IEMSI with a given username & password.
    /// # Returns
    /// - `Ok(None)` if the login is still in progress.
    /// - `Ok(Some(data))` if the login is successful. The data is the IEMSI data to send to the server.
    pub fn try_login(&mut self, user_name: &str, password: &str, ch: u8) -> crate::Result<Option<Vec<u8>>> {
        if self.aborted {
            return Ok(None);
        }
        if let Some(data) = self.advance_char(user_name, password, ch)? {
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    pub fn advance_char(&mut self, user_name: &str, password: &str, ch: u8) -> crate::Result<Option<Vec<u8>>> {
        if self.aborted {
            return Ok(None);
        }
        self.parse_char(ch)?;
        if self.irq_requested {
            self.irq_requested = false;
            // self.log_file.push("Starting IEMSI negotiation…".to_string());
            self.user_settings.name = user_name.to_string();
            self.user_settings.password = password.to_string();
            let data = encode_ici(&self.user_settings, &self.terminal_settings, &ICIRequests::default())?;
            return Ok(Some(data));
        } else if let Some(_isi) = &self.isi {
            // self.log_file.push("Receiving valid IEMSI server info…".to_string());
            // self.log_file.push(format!("Name:{} Location:{} Operator:{} Notice:{} System:{}", isi.name, isi.location, isi.operator, isi.notice, isi.id));
            self.aborted = true;
            self.logged_in = true;
            return Ok(Some(EMSI_2ACK.to_vec()));
        } else if self.got_invalid_isi {
            self.got_invalid_isi = false;
            // self.log_file.push("Got invalid IEMSI server info…".to_string());
            self.aborted = true;
            self.logged_in = true;
            return Ok(Some(EMSI_2ACK.to_vec()));
        } else if self.nak_requested {
            self.nak_requested = false;
            if self.retries < 2 {
                // self.log_file.push("IEMSI retry…".to_string());
                self.user_settings.name = user_name.to_string();
                self.user_settings.password = password.to_string();
                let data = encode_ici(&self.user_settings, &self.terminal_settings, &ICIRequests::default())?;
                self.retries += 1;
                return Ok(Some(data));
            }
            // self.log_file.push("IEMSI aborted…".to_string());
            self.aborted = true;
            return Ok(Some(EMSI_IIR.to_vec()));
        }
        Ok(None)
    }

    fn reset_sequences(&mut self) {
        self.irq_seq = 0;
        self.nak_seq = 0;
        self.isi_seq = 0;
        self.isi_crc = 0;
        self.isi_check_crc = 0xFFFF_FFFF;
        self.isi_data.clear();
    }
}

fn get_value(ch: u8) -> usize {
    let res = match ch {
        b'0'..=b'9' => ch - b'0',
        b'a'..=b'f' => 10 + ch - b'a',
        b'A'..=b'F' => 10 + ch - b'A',
        _ => 0,
    };
    res as usize
}

fn parse_emsi_blocks(data: &[u8]) -> crate::Result<Vec<String>> {
    let mut res = Vec::new();
    let mut i = 0;
    let mut str = String::new();
    let mut in_string = false;

    while i < data.len() {
        if data[i] == b'}' {
            if i + 1 < data.len() && data[i + 1] == b'}' {
                str.push('}');
                i += 2;
                continue;
            }
            i += 1;
            res.push(str.clone());
            str.clear();
            in_string = false;
            continue;
        }

        if data[i] == b'{' && !in_string {
            in_string = true;
            i += 1;
            continue;
        }

        if data[i] == b'\\' {
            if i + 1 < data.len() && data[i + 1] == b'\\' {
                str.push('\\');
                i += 2;
                continue;
            }
            if i + 2 < data.len() {
                let b = u32::try_from(get_value(data[i + 1]) * 16 + get_value(data[i + 2])).unwrap();
                str.push(char::from_u32(b).unwrap());
                i += 3;
                continue;
            }
            return Err(NetError::InvalidEscapeInEmsi.into());
        }

        str.push(char::from_u32(u32::from(data[i])).unwrap());
        i += 1;
    }
    Ok(res)
}

fn get_hex(n: u32) -> u8 {
    if n < 10 {
        return b'0' + u8::try_from(n).unwrap();
    }
    b'A' + u8::try_from(n - 10).unwrap()
}

fn encode_emsi(data: &[&str]) -> crate::Result<Vec<u8>> {
    let mut res = Vec::new();
    for i in 0..data.len() {
        let d = data[i];
        res.push(b'{');
        for ch in d.chars() {
            if ch == '}' {
                res.extend_from_slice(b"}}");
                continue;
            }
            if ch == '\\' {
                res.extend_from_slice(b"\\\\");
                continue;
            }
            let val = ch as u32;
            if val > 255 {
                return Err(NetError::NoUnicodeInEmsi.into());
            }
            // control codes.
            if val < 32 || val == 127 {
                res.push(b'\\');
                res.push(get_hex((val >> 4) & 0xF));
                res.push(get_hex(val & 0xF));
                continue;
            }

            res.push((val & 0xFF) as u8);
        }
        res.push(b'}');
    }

    Ok(res)
}
