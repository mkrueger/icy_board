/// End of subnegotiation parameters.
pub const SE: u8 = 0xF0;

/// No operation.
pub const NOP: u8 = 0xF1;

/// The data stream portion of a Synch.
/// This should always be accompanied
/// by a TCP Urgent notification.
pub const DATA_MARK: u8 = 0xF2;

/// NVT character BRK
pub const BREAK: u8 = 0xF3;

/// The function Interrupt Process
pub const IP: u8 = 0xF4;

// The function Abort output
pub const AO: u8 = 0xF5;

// The function Are You There
pub const AYT: u8 = 0xF6;

// The function Erase character
pub const EC: u8 = 0xF7;

// The function Erase line
pub const EL: u8 = 0xF8;

// The Go ahead signal.
pub const GA: u8 = 0xF9;

// Indicates that what follows is subnegotiation of the indicated option.
pub const SB: u8 = 0xFA;

///  (option code)
/// Indicates the desire to begin performing, or confirmation that you are now performing, the indicated option.
pub const WILL: u8 = 0xFB;

/// (option code)
/// Indicates the refusal to perform, or continue performing, the indicated option.
pub const WONT: u8 = 0xFC;

/// (option code)
/// Indicates the request that the other party perform, or confirmation that you are expecting
/// the other party to perform, the indicated option.
pub const DO: u8 = 0xFD;

/// (option code)
/// Indicates the demand that the other party stop performing,
/// or confirmation that you are no longer expecting the other party
/// to perform, the indicated option.
pub const DONT: u8 = 0xFE;

/// Data Byte 255.
pub const IAC: u8 = 0xFF;

pub fn make_cmd(byte: u8) -> [u8; 2] {
    [IAC, byte]
}

pub fn make_cmd_with_option(byte: u8, option: u8) -> [u8; 3] {
    [IAC, byte, option]
}

pub fn check(byte: u8) -> crate::Result<u8> {
    match byte {
        0xF0..=0xFF => Ok(byte),
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unknown IAC: {byte}/x{byte:02X}"),
        ))),
    }
}

pub fn to_string(byte: u8) -> &'static str {
    match byte {
        SE => "SE",
        NOP => "Nop",
        DATA_MARK => "DataMark",
        BREAK => "Break",
        IP => "IP",
        AO => "AO",
        AYT => "Ayt",
        EC => "EC",
        EL => "EL",
        GA => "GA",
        SB => "SB",
        WILL => "Will",
        WONT => "Wont",
        DO => "DO",
        DONT => "Dont",
        IAC => "Iac",
        _ => "unknown",
    }
}
