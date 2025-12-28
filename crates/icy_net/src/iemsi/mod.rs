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
    Connection, NetError,
    crc::{get_crc16, get_crc32, update_crc32},
};

/// Result of IEMSI client login attempt
#[derive(Debug, Clone)]
pub enum IEmsiLoginResult {
    /// No IEMSI sequence detected yet, continue scanning
    Pending,
    /// EMSI_IRQ detected, handshake should be started
    IrqDetected,
    /// Login successful, contains server information
    Success(EmsiISI),
    /// Login failed (timeout, max retries, or aborted)
    Failed,
}

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

/// IEMSI client state machine for detecting EMSI_IRQ in incoming data stream.
///
/// This struct is used to scan incoming bytes for IEMSI sequences.
/// Once `EMSI_IRQ` is detected, use `complete_iemsi_handshake()` to perform
/// the actual handshake over the connection.
#[derive(Default)]
pub struct IEmsi {
    stars_read: i32,
    irq_seq: usize,
    /// Set to true when EMSI_IRQ is detected in the input stream
    pub irq_detected: bool,
}

impl IEmsi {
    /// Scans a single byte for EMSI_IRQ sequence.
    ///
    /// Call this for each incoming byte. When `irq_detected` becomes true,
    /// call `complete_iemsi_handshake()` to perform the IEMSI handshake.
    ///
    /// # Returns
    /// - `true` if EMSI_IRQ was just detected
    /// - `false` if still scanning
    pub fn scan_byte(&mut self, ch: u8) -> bool {
        if self.irq_detected {
            return true;
        }

        if self.stars_read >= 2 {
            // Convert to uppercase for case-insensitive comparison
            // Some BBSes send lowercase IEMSI sequences
            let ch_upper = ch.to_ascii_uppercase();

            if ch_upper == EMSI_IRQ[2 + self.irq_seq] {
                self.irq_seq += 1;
                if self.irq_seq + 2 >= EMSI_IRQ.len() {
                    self.irq_detected = true;
                    self.stars_read = 0;
                    self.irq_seq = 0;
                    return true;
                }
                return false;
            } else {
                self.irq_seq = 0;
            }
            self.stars_read = 0;
        }

        if ch == b'*' {
            self.stars_read += 1;
            self.irq_seq = 0;
            return false;
        }
        self.stars_read = 0;

        false
    }

    /// Reset the scanner state (e.g., after a failed handshake to try again)
    pub fn reset(&mut self) {
        self.stars_read = 0;
        self.irq_seq = 0;
        self.irq_detected = false;
    }
}

/// Internal state machine for parsing EMSI_ISI and EMSI_NAK during handshake
#[derive(Default)]
struct IsiParser {
    stars_read: i32,
    isi_seq: usize,
    nak_seq: usize,
    ack_seq: usize,
    isi_len: usize,
    isi_crc: usize,
    isi_check_crc: u32,
    isi_data: Vec<u8>,
    nak_detected: bool,
    ack_detected: bool,
    isi: Option<EmsiISI>,
    got_invalid_isi: bool,
    /// Counter for non-EMSI bytes received
    garbage_bytes: usize,
    /// Flag set when too much garbage is received (server doesn't support IEMSI)
    pub abort_detected: bool,
}

// **EMSI_ISI<len><data><crc32><CR>
const ISI_START: &[u8; 8] = b"EMSI_ISI";

impl IsiParser {
    fn new() -> Self {
        Self {
            isi_check_crc: 0xFFFF_FFFF,
            ..Default::default()
        }
    }

    fn parse_byte(&mut self, ch: u8) -> crate::Result<()> {
        // Debug: print every byte
        if ch.is_ascii_graphic() || ch == b' ' {
            print!("{}", ch as char);
        } else {
            print!("[{:02X}]", ch);
        }

        if self.stars_read >= 2 {
            if self.isi_seq > 7 {
                match self.isi_seq {
                    8..=11 => {
                        self.isi_check_crc = update_crc32(self.isi_check_crc, ch);
                        self.isi_len = self.isi_len * 16 + get_value(ch);
                        self.isi_seq += 1;
                        return Ok(());
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
                            println!("\n[IEMSI Parser] ISI complete, checking CRC...");
                            if ch == b'\r' {
                                if self.isi_crc == self.isi_check_crc as usize {
                                    let group = parse_emsi_blocks(&self.isi_data)?;
                                    println!("[IEMSI Parser] ISI has {} blocks", group.len());
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
                                        return Ok(());
                                    }
                                    self.got_invalid_isi = true;
                                } else {
                                    println!("[IEMSI Parser] CRC mismatch: expected {:08X}, got {:08X}", self.isi_crc, self.isi_check_crc);
                                    self.got_invalid_isi = true;
                                }
                            }
                            self.stars_read = 0;
                            self.reset_sequences();
                        }
                        self.isi_seq += 1;
                        return Ok(());
                    }
                    _ => {}
                }
                return Ok(());
            }
            let mut got_seq = false;

            // Convert to uppercase for case-insensitive comparison
            let ch_upper = ch.to_ascii_uppercase();

            if ch_upper == ISI_START[self.isi_seq] {
                self.isi_check_crc = update_crc32(self.isi_check_crc, ch);
                self.isi_seq += 1;
                self.isi_len = 0;
                got_seq = true;
            } else {
                if self.isi_seq > 0 {
                    println!(
                        "\n[IEMSI Parser] ISI sequence broken at pos {}, got '{}' expected '{}'",
                        self.isi_seq, ch_upper as char, ISI_START[self.isi_seq] as char
                    );
                }
                self.isi_seq = 0;
            }

            if ch_upper == EMSI_NAK[2 + self.nak_seq] {
                self.nak_seq += 1;
                if self.nak_seq + 2 >= EMSI_NAK.len() {
                    println!("\n[IEMSI Parser] NAK detected!");
                    self.nak_detected = true;
                    self.stars_read = 0;
                    self.reset_sequences();
                }
                got_seq = true;
            } else {
                self.nak_seq = 0;
            }

            // Also detect ACK (some servers send ACK before ISI)
            if ch_upper == EMSI_ACK[2 + self.ack_seq] {
                self.ack_seq += 1;
                if self.ack_seq + 2 >= EMSI_ACK.len() {
                    println!("\n[IEMSI Parser] ACK detected (ignoring, waiting for ISI)");
                    self.ack_detected = true;
                    self.stars_read = 0;
                    self.reset_sequences();
                }
                got_seq = true;
            } else {
                self.ack_seq = 0;
            }

            if got_seq {
                return Ok(());
            }
            self.stars_read = 0;
            self.reset_sequences();
        }

        if ch == b'*' {
            self.stars_read += 1;
            self.garbage_bytes = 0; // Reset garbage counter on potential EMSI start
            self.reset_sequences();
            return Ok(());
        }
        self.stars_read = 0;

        Ok(())
    }

    fn reset_sequences(&mut self) {
        self.isi_seq = 0;
        self.nak_seq = 0;
        self.ack_seq = 0;
        self.isi_crc = 0;
        self.isi_check_crc = 0xFFFF_FFFF;
        self.isi_data.clear();
    }
}

/// Completes the IEMSI handshake after EMSI_IRQ has been detected.
///
/// This function takes over the connection and performs the full IEMSI
/// handshake:
/// 1. Sends EMSI_ICI (client info)
/// 2. Waits for EMSI_ISI (server info) or EMSI_NAK (retry request)
/// 3. Sends 2x EMSI_ACK on success
///
/// # Arguments
/// * `com` - The connection to use for communication
/// * `user_settings` - User information to send to the server
/// * `terminal_settings` - Terminal capabilities to advertise
/// * `timeout_ms` - Maximum time to wait for server response (recommended: 5000)
///
/// # Returns
/// * `Ok(Some(EmsiISI))` - Handshake successful, contains server info
/// * `Ok(None)` - Handshake failed (timeout, max retries, or error)
pub async fn complete_iemsi_handshake(
    com: &mut Box<dyn Connection>,
    user_settings: &ICIUserSettings,
    terminal_settings: &ICITerminalSettings,
    timeout_ms: u64,
) -> crate::Result<Option<EmsiISI>> {
    const MAX_RETRIES: usize = 2;
    let mut retries = 0;

    println!("[IEMSI Client] Starting client handshake");

    // Send initial ICI (NOT IIR - that means abort!)
    let ici_data = encode_ici(user_settings, terminal_settings, &ICIRequests::default())?;
    println!(
        "[IEMSI Client] >> Sending EMSI_ICI ({} bytes): {:?}",
        ici_data.len(),
        String::from_utf8_lossy(&ici_data)
    );
    com.send(&ici_data).await?;

    let mut buf = [0u8; 1024];
    let mut parser = IsiParser::new();
    let start = std::time::Instant::now();

    while start.elapsed().as_millis() < timeout_ms as u128 {
        let size = com.try_read(&mut buf).await?;
        if size == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            continue;
        }

        println!("[IEMSI Client] << Received {} bytes: {:?}", size, String::from_utf8_lossy(&buf[..size.min(80)]));

        for &ch in &buf[..size] {
            parser.parse_byte(ch)?;

            // Check if server is sending garbage (ANSI login screen) instead of EMSI_ISI
            if parser.abort_detected {
                println!("[IEMSI Client] Server doesn't support IEMSI, aborting handshake");
                return Ok(None);
            }

            // Check if we got a valid ISI
            if let Some(isi) = parser.isi.take() {
                println!("[IEMSI Client] << Received valid EMSI_ISI: name='{}' location='{}'", isi.name, isi.location);
                println!("[IEMSI Client] >> Sending 2x EMSI_ACK");
                com.send(EMSI_2ACK).await?;
                return Ok(Some(isi));
            }

            // Check for invalid ISI (CRC error but still accept)
            if parser.got_invalid_isi {
                parser.got_invalid_isi = false;
                println!("[IEMSI Client] << Received invalid EMSI_ISI (CRC error)");
                println!("[IEMSI Client] >> Sending 2x EMSI_ACK anyway");
                com.send(EMSI_2ACK).await?;
                // Return a default/empty ISI to indicate login was accepted
                return Ok(Some(EmsiISI::default()));
            }

            // Check for NAK - server wants us to resend ICI
            if parser.nak_detected {
                parser.nak_detected = false;
                println!("[IEMSI Client] << Received EMSI_NAK");

                if retries < MAX_RETRIES {
                    retries += 1;
                    let ici_data = encode_ici(user_settings, terminal_settings, &ICIRequests::default())?;
                    println!("[IEMSI Client] >> Resending EMSI_ICI (retry {}/{})", retries, MAX_RETRIES);
                    com.send(&ici_data).await?;
                } else {
                    println!("[IEMSI Client] >> Max retries reached, aborting");
                    com.send(EMSI_IIR).await?; // Send IIR to abort
                    return Ok(None);
                }
            }
        }
    }

    println!("[IEMSI Client] Handshake timeout after {}ms", timeout_ms);
    Ok(None)
}

/// Convenience function to attempt IEMSI login.
///
/// This function combines scanning for EMSI_IRQ and completing the handshake.
/// It reads from the connection until either:
/// - IEMSI handshake completes successfully
/// - Timeout expires
/// - An error occurs
///
/// # Arguments
/// * `com` - The connection to use
/// * `user_settings` - User credentials and info
/// * `terminal_settings` - Terminal capabilities
/// * `timeout_ms` - Total timeout for the entire process
///
/// # Returns
/// * `IEmsiLoginResult::Success(EmsiISI)` - Login successful
/// * `IEmsiLoginResult::Failed` - Login failed or timed out
/// * `IEmsiLoginResult::Pending` - Not used by this function
pub async fn try_iemsi_client_login(
    com: &mut Box<dyn Connection>,
    user_settings: &ICIUserSettings,
    terminal_settings: &ICITerminalSettings,
    timeout_ms: u64,
) -> crate::Result<IEmsiLoginResult> {
    let mut scanner = IEmsi::default();
    let mut buf = [0u8; 1024];
    let start = std::time::Instant::now();

    // Phase 1: Scan for EMSI_IRQ
    while start.elapsed().as_millis() < timeout_ms as u128 {
        let size = com.try_read(&mut buf).await?;
        if size == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            continue;
        }

        for &ch in &buf[..size] {
            if scanner.scan_byte(ch) {
                println!("[IEMSI Client] << Detected EMSI_IRQ");

                // Phase 2: Complete handshake
                let remaining_time = timeout_ms.saturating_sub(start.elapsed().as_millis() as u64);
                if let Some(isi) = complete_iemsi_handshake(com, user_settings, terminal_settings, remaining_time).await? {
                    return Ok(IEmsiLoginResult::Success(isi));
                } else {
                    return Ok(IEmsiLoginResult::Failed);
                }
            }
        }
    }

    Ok(IEmsiLoginResult::Failed)
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
