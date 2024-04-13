use std::fmt;

use super::get_crc16string;

/// The TCH sequence is used by the Server to instruct the Client
/// software to terminate its full-screen conversation mode function
/// (CHAT).
///
/// If the Server transmits this sequence to the Client, it must wait for
/// an `EMSI_ACK` prior to leaving its conversation mode. If no `EMSI_ACK`
/// sequence is received with ten seconds, a second `EMSI_TCH` sequence
/// should be issued before the Server resumes operation. If, however, an
/// `EMSI_NAK` sequence is received from the Client, the Server must
/// re-transmit the `EMSI_TCH` sequence.
pub const EMSI_TCH: &[u8; 15] = b"**EMSI_TCH3C60\r";

pub struct EmsiDAT {
    pub system_address_list: String,
    pub password: String,
    pub link_codes: String,
    pub compatibility_codes: String,
    pub mailer_product_code: String,
    pub mailer_name: String,
    pub mailer_version: String,
    pub mailer_serial_number: String,
    pub extra_field: Vec<String>,
}

impl std::fmt::Display for EmsiDAT {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        let v = self.encode();
        write!(f, "{}", std::str::from_utf8(&v).unwrap())
    }
}

impl EmsiDAT {
    pub fn new() -> Self {
        EmsiDAT {
            system_address_list: String::new(),
            password: String::new(),
            link_codes: String::new(),
            compatibility_codes: String::new(),
            mailer_product_code: String::new(),
            mailer_name: String::new(),
            mailer_version: String::new(),
            mailer_serial_number: String::new(),
            extra_field: Vec::new(),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let data = format!(
            "{{EMSI}}{{{}}}{{{}}}{{{}}}{{{}}}{{{}}}{{{}}}{{{}}}{{{}}}",
            self.system_address_list,
            self.password,
            self.link_codes,
            self.compatibility_codes,
            self.mailer_product_code,
            self.mailer_name,
            self.mailer_version,
            self.mailer_serial_number
        );

        // todo: etxra fields - are they even used ?

        let block = format!("EMSI_DAT{:04X}{}", data.len(), data);
        let mut result = Vec::new();
        result.extend_from_slice(b"**EMSI_DAT");
        let bytes = block.as_bytes();
        result.extend_from_slice(bytes);
        result.extend_from_slice(get_crc16string(bytes).as_bytes());
        result.push(b'\r');
        result
    }
}
