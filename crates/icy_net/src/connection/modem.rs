#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serial2_tokio::SerialPort;
use tokio::{io::AsyncWriteExt, time::timeout};

use crate::{
    Connection, ConnectionState, ConnectionType,
    serial::{FlowControl, Format, Serial},
};

/// Result of validating a modem command string
#[derive(Clone, Debug, PartialEq)]
pub enum ModemCommandValidationResult {
    /// The command string is valid
    Valid,
    /// Invalid control sequence (e.g., ^1, ^!)
    /// Contains the invalid character and position
    InvalidControlSequence { char: char, position: usize },
    /// Incomplete control sequence (^ at end of string)
    /// Contains the position of the lone caret
    IncompleteControlSequence { position: usize },
    /// Invalid character (non-ASCII, > 127)
    /// Contains the invalid character and position
    InvalidCharacter { char: char, position: usize },
    /// Invalid hex escape sequence (\x must be followed by exactly 2 hex digits)
    /// Contains the position of the backslash
    InvalidHexSequence { position: usize },
}

impl ModemCommandValidationResult {
    /// Returns true if the validation result is valid
    pub fn is_valid(&self) -> bool {
        matches!(self, ModemCommandValidationResult::Valid)
    }
}

/// A single token in a modem command string
#[derive(Clone, Debug, PartialEq)]
pub enum ModemCommandToken {
    /// Raw text to send as-is
    Text(String),
    /// Pause for specified duration (~ = 1 second)
    Pause(Duration),
    /// Control character (^M = CR, ^J = LF, ^[ = ESC, etc.)
    Control(u8),
    /// Hex byte value (\xNN notation)
    Byte(u8),
}

impl ModemCommandToken {
    /// Convert a control character code to its ^X representation
    fn control_to_string(c: u8) -> String {
        match c {
            0 => "^@".to_string(),
            1..=26 => format!("^{}", (b'A' + c - 1) as char),
            27 => "^[".to_string(),
            28 => "^\\".to_string(),
            29 => "^]".to_string(),
            30 => "^^".to_string(),
            31 => "^_".to_string(),
            127 => "^?".to_string(),
            // For Control tokens with values outside ^X range, use hex
            _ => format!("\\x{:02x}", c),
        }
    }
}

impl std::fmt::Display for ModemCommandToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModemCommandToken::Text(s) => write!(f, "{}", s),
            ModemCommandToken::Pause(d) => {
                let secs = d.as_secs();
                for _ in 0..secs {
                    write!(f, "~")?;
                }
                Ok(())
            }
            ModemCommandToken::Control(c) => {
                match c {
                    0 => write!(f, "^@"),
                    1..=26 => write!(f, "^{}", (b'A' + c - 1) as char),
                    27 => write!(f, "^["),
                    28 => write!(f, "^\\"),
                    29 => write!(f, "^]"),
                    30 => write!(f, "^^"),
                    31 => write!(f, "^_"),
                    127 => write!(f, "^?"),
                    // For Control tokens with values outside ^X range, use hex
                    _ => write!(f, "\\x{:02X}", c),
                }
            }
            ModemCommandToken::Byte(b) => write!(f, "\\x{:02X}", b),
        }
    }
}

/// A parsed modem command string containing multiple tokens
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModemCommand {
    tokens: Vec<ModemCommandToken>,
}

impl Serialize for ModemCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as the string representation
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ModemCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ModemCommand::try_parse(&s).map_err(|e| {
            serde::de::Error::custom(match e {
                ModemCommandValidationResult::Valid => unreachable!(),
                ModemCommandValidationResult::InvalidControlSequence { char, position } => {
                    format!("Invalid control sequence '^{}' at position {}", char, position)
                }
                ModemCommandValidationResult::IncompleteControlSequence { position } => {
                    format!("Incomplete control sequence at position {}", position)
                }
                ModemCommandValidationResult::InvalidCharacter { char, position } => {
                    format!("Invalid character '{}' at position {}", char, position)
                }
                ModemCommandValidationResult::InvalidHexSequence { position } => {
                    format!("Invalid hex sequence at position {}", position)
                }
            })
        })
    }
}

impl ModemCommand {
    /// Create a new empty modem command
    pub fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    /// Create a modem command from a vector of tokens
    pub fn from_tokens(tokens: Vec<ModemCommandToken>) -> Self {
        Self { tokens }
    }

    /// Try to parse a modem command string into tokens.
    /// Returns an error if the input contains invalid sequences.
    ///
    /// Supports:
    /// - `~` for 1 second pause
    /// - `^X` for control characters (^M = CR, ^[ = ESC, etc.)
    /// - `\xNN` for hex byte values (\x00 - \xFF)
    /// - Regular text (ASCII only)
    pub fn try_parse(input: &str) -> Result<Self, ModemCommandValidationResult> {
        // First validate
        let validation = Self::validate(input);
        if !validation.is_valid() {
            return Err(validation);
        }

        let mut tokens = Vec::new();
        let mut chars = input.chars().peekable();
        let mut text_buffer = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '~' => {
                    // Flush text buffer
                    if !text_buffer.is_empty() {
                        tokens.push(ModemCommandToken::Text(std::mem::take(&mut text_buffer)));
                    }
                    // Count consecutive tildes for longer pauses
                    let mut pause_secs = 1;
                    while chars.peek() == Some(&'~') {
                        chars.next();
                        pause_secs += 1;
                    }
                    tokens.push(ModemCommandToken::Pause(Duration::from_secs(pause_secs)));
                }
                '\\' => {
                    // Check for \xNN hex escape
                    if chars.peek() == Some(&'x') {
                        chars.next(); // consume 'x'
                        let mut hex_str = String::new();
                        for _ in 0..2 {
                            if let Some(&c) = chars.peek() {
                                if c.is_ascii_hexdigit() {
                                    hex_str.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        }
                        if hex_str.len() == 2 {
                            if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                                // Flush text buffer
                                if !text_buffer.is_empty() {
                                    tokens.push(ModemCommandToken::Text(std::mem::take(&mut text_buffer)));
                                }
                                tokens.push(ModemCommandToken::Byte(byte));
                                continue;
                            }
                        }
                        // Should not reach here after validation
                        text_buffer.push('\\');
                        text_buffer.push('x');
                        text_buffer.push_str(&hex_str);
                    } else {
                        // Just a backslash, treat as literal
                        text_buffer.push('\\');
                    }
                }
                '^' => {
                    if let Some(&next_ch) = chars.peek() {
                        // Flush text buffer
                        if !text_buffer.is_empty() {
                            tokens.push(ModemCommandToken::Text(std::mem::take(&mut text_buffer)));
                        }
                        chars.next();
                        // Convert ^X to control character
                        let ctrl_char = match next_ch.to_ascii_uppercase() {
                            '@' => 0,
                            'A'..='Z' => next_ch.to_ascii_uppercase() as u8 - b'A' + 1,
                            '[' => 27,
                            '\\' => 28,
                            ']' => 29,
                            '^' => 30,
                            '_' => 31,
                            '?' => 127,
                            _ => unreachable!("validation should have caught this"),
                        };
                        tokens.push(ModemCommandToken::Control(ctrl_char));
                    }
                    // Lone ^ at end should have been caught by validation
                }
                _ => {
                    text_buffer.push(ch);
                }
            }
        }

        // Flush remaining text
        if !text_buffer.is_empty() {
            tokens.push(ModemCommandToken::Text(text_buffer));
        }

        Ok(Self { tokens })
    }

    /// Get the tokens in this command
    pub fn tokens(&self) -> &[ModemCommandToken] {
        &self.tokens
    }

    /// Get mutable access to the tokens
    pub fn tokens_mut(&mut self) -> &mut Vec<ModemCommandToken> {
        &mut self.tokens
    }

    /// Check if the command is empty
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Get the number of tokens
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Add a token to the command
    pub fn push(&mut self, token: ModemCommandToken) {
        self.tokens.push(token);
    }

    /// Add text to the command
    pub fn push_text(&mut self, text: impl Into<String>) {
        self.tokens.push(ModemCommandToken::Text(text.into()));
    }

    /// Add a pause to the command
    pub fn push_pause(&mut self, duration: Duration) {
        self.tokens.push(ModemCommandToken::Pause(duration));
    }

    /// Add a control character to the command
    pub fn push_control(&mut self, c: u8) {
        self.tokens.push(ModemCommandToken::Control(c));
    }

    /// Add a hex byte to the command (\xNN notation)
    pub fn push_byte(&mut self, b: u8) {
        self.tokens.push(ModemCommandToken::Byte(b));
    }

    /// Convert the command to bytes for sending (pauses are omitted)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for token in &self.tokens {
            match token {
                ModemCommandToken::Text(s) => result.extend_from_slice(s.as_bytes()),
                ModemCommandToken::Pause(_) => {} // Pauses are handled separately during send
                ModemCommandToken::Control(c) => result.push(*c),
                ModemCommandToken::Byte(b) => result.push(*b),
            }
        }
        result
    }

    /// Send the command to a writer, respecting pause tokens
    pub async fn send<W: tokio::io::AsyncWriteExt + Unpin>(&self, writer: &mut W) -> std::io::Result<()> {
        for token in &self.tokens {
            match token {
                ModemCommandToken::Text(s) => writer.write_all(s.as_bytes()).await?,
                ModemCommandToken::Pause(d) => tokio::time::sleep(*d).await,
                ModemCommandToken::Control(c) => writer.write_all(&[*c]).await?,
                ModemCommandToken::Byte(b) => writer.write_all(&[*b]).await?,
            }
        }
        Ok(())
    }

    /// Iterate over the tokens
    pub fn iter(&self) -> impl Iterator<Item = &ModemCommandToken> {
        self.tokens.iter()
    }

    /// Validate if the input string is a valid modem command string.
    /// Returns `ModemCommandValidationResult::Valid` if valid, or a specific error variant.
    ///
    /// A valid modem command string:
    /// - Can only contain ASCII characters (0-127) in text
    /// - `~` represents a pause (always valid)
    /// - `^X` must be followed by a valid control character specifier:
    ///   - `@` (NUL), `A`-`Z`, `[` (ESC), `\`, `]`, `^`, `_`, `?` (DEL)
    /// - `\xNN` for hex byte values (must be exactly 2 hex digits)
    pub fn validate(input: &str) -> ModemCommandValidationResult {
        let mut chars = input.chars().peekable();
        let mut position = 0;

        while let Some(ch) = chars.next() {
            // Check for non-ASCII characters
            if !ch.is_ascii() {
                return ModemCommandValidationResult::InvalidCharacter { char: ch, position };
            }

            match ch {
                '\\' => {
                    let backslash_pos = position;
                    if chars.peek() == Some(&'x') {
                        chars.next(); // consume 'x'
                        position += 1;

                        // Must have exactly 2 hex digits
                        let mut hex_count = 0;
                        for _ in 0..2 {
                            if let Some(&c) = chars.peek() {
                                if c.is_ascii_hexdigit() {
                                    chars.next();
                                    position += 1;
                                    hex_count += 1;
                                } else {
                                    break;
                                }
                            }
                        }
                        if hex_count != 2 {
                            return ModemCommandValidationResult::InvalidHexSequence { position: backslash_pos };
                        }
                    }
                    // Plain backslash is valid
                }
                '^' => {
                    if let Some(&next_ch) = chars.peek() {
                        let upper = next_ch.to_ascii_uppercase();
                        let valid = matches!(upper, '@' | 'A'..='Z' | '[' | '\\' | ']' | '^' | '_' | '?');
                        if !valid {
                            return ModemCommandValidationResult::InvalidControlSequence { char: next_ch, position };
                        }
                        chars.next();
                        position += 1;
                    } else {
                        // Lone ^ at end is invalid
                        return ModemCommandValidationResult::IncompleteControlSequence { position };
                    }
                }
                _ => {}
            }
            position += 1;
        }

        ModemCommandValidationResult::Valid
    }

    /// Check if the input string is a valid modem command string.
    /// Returns `true` if valid, `false` otherwise.
    pub fn is_valid(input: &str) -> bool {
        Self::validate(input).is_valid()
    }
}

impl std::fmt::Display for ModemCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for token in &self.tokens {
            write!(f, "{}", token)?;
        }
        Ok(())
    }
}

impl std::str::FromStr for ModemCommand {
    type Err = ModemCommandValidationResult;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_parse(s)
    }
}

impl TryFrom<&str> for ModemCommand {
    type Error = ModemCommandValidationResult;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_parse(s)
    }
}

impl TryFrom<String> for ModemCommand {
    type Error = ModemCommandValidationResult;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_parse(&s)
    }
}

impl IntoIterator for ModemCommand {
    type Item = ModemCommandToken;
    type IntoIter = std::vec::IntoIter<ModemCommandToken>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

impl<'a> IntoIterator for &'a ModemCommand {
    type Item = &'a ModemCommandToken;
    type IntoIter = std::slice::Iter<'a, ModemCommandToken>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.iter()
    }
}

/// Modem response types
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModemResponseType {
    /// Connection established (e.g., "CONNECT", "CONNECT 9600")
    Connect,
    /// OK response from modem
    Ok,
    /// Ring detected
    Ring,
    /// No carrier - connection failed or dropped
    NoCarrier,
    /// Error from modem
    Error,
    /// No dial tone
    NoDialtone,
    /// Line is busy
    Busy,
    /// No answer from remote
    NoAnswer,
}

fn default_modem_responses() -> HashMap<ModemResponseType, String> {
    HashMap::from([
        (ModemResponseType::Connect, "CONNECT".to_string()),
        (ModemResponseType::Ok, "OK".to_string()),
        (ModemResponseType::Ring, "RING".to_string()),
        (ModemResponseType::NoCarrier, "NO CARRIER".to_string()),
        (ModemResponseType::Error, "ERROR".to_string()),
        (ModemResponseType::NoDialtone, "NO DIAL TONE".to_string()),
        (ModemResponseType::Busy, "BUSY".to_string()),
        (ModemResponseType::NoAnswer, "NO ANSWER".to_string()),
    ])
}

fn default_init_command() -> ModemCommand {
    ModemCommand::try_parse("ATZ^M").unwrap()
}

fn default_dial_prefix() -> ModemCommand {
    ModemCommand::try_parse("ATDT").unwrap()
}

fn default_dial_suffix() -> ModemCommand {
    ModemCommand::try_parse("^M").unwrap()
}

fn default_hangup_command() -> ModemCommand {
    ModemCommand::try_parse("~~~+++~~~ATH0^M").unwrap()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModemConfiguration {
    pub name: String,
    pub device: String,
    pub baud_rate: u32,

    #[serde(default)]
    pub format: Format,

    #[serde(default)]
    pub flow_control: FlowControl,

    #[serde(default = "default_init_command")]
    pub init_command: ModemCommand,

    #[serde(default = "default_dial_prefix")]
    pub dial_prefix: ModemCommand,

    #[serde(default = "default_dial_suffix")]
    pub dial_suffix: ModemCommand,

    #[serde(default = "default_hangup_command")]
    pub hangup_command: ModemCommand,

    #[serde(default = "default_modem_responses")]
    pub modem_responses: HashMap<ModemResponseType, String>,
}

impl Default for ModemConfiguration {
    fn default() -> Self {
        Self {
            name: "Modem".to_string(),
            device: if cfg!(target_os = "windows") {
                "COM1:".to_string()
            } else {
                "/dev/ttyUSB0".to_string()
            },
            baud_rate: 57600,
            format: Default::default(),
            flow_control: Default::default(),
            init_command: default_init_command(),
            dial_prefix: default_dial_prefix(),
            dial_suffix: default_dial_suffix(),
            hangup_command: default_hangup_command(),
            modem_responses: default_modem_responses(),
        }
    }
}

impl ModemConfiguration {
    /// Parse a modem response from the given text
    /// Returns the first matching response type, if any
    pub fn parse_response(&self, text: &str) -> Option<ModemResponseType> {
        let text_upper = text.to_uppercase();
        self.modem_responses
            .iter()
            .find(|(_, pattern)| text_upper.contains(&pattern.to_uppercase()))
            .map(|(response_type, _)| response_type.clone())
    }
}

pub struct ModemConnection {
    modem: ModemConfiguration,
    port: Box<SerialPort>,
}

impl ModemConnection {
    pub async fn open(modem: ModemConfiguration, call_number: String) -> crate::Result<Self> {
        let serial: Serial = modem.clone().into();
        let port: SerialPort = serial.open()?;

        let mut conn = Self { modem, port: Box::new(port) };
        conn.init().await?;
        conn.dial(&call_number).await?;
        Ok(conn)
    }

    /// Initialize the modem by sending the init command
    pub async fn init(&mut self) -> crate::Result<()> {
        self.modem.init_command.send(&mut *self.port).await?;
        Ok(())
    }

    /// Dial a phone number
    /// Sends: dial_prefix + number + dial_suffix
    pub async fn dial(&mut self, number: &str) -> crate::Result<()> {
        self.modem.dial_prefix.send(&mut *self.port).await?;
        self.port.write_all(number.as_bytes()).await?;
        self.modem.dial_suffix.send(&mut *self.port).await?;
        Ok(())
    }

    /// Hang up the modem connection
    /// Sends the hangup command (typically ~~~+++~~~ATH0^M)
    pub async fn hangup(&mut self) -> crate::Result<()> {
        self.modem.hangup_command.send(&mut *self.port).await?;
        Ok(())
    }
}

#[async_trait]
impl Connection for ModemConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Modem
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        let res = self.port.read(buf).await?;
        //  println!("Read {:?} bytes", &buf[..res]);
        Ok(res)
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        // Use a reasonable timeout for serial communication
        // 50ms gives the hardware time to buffer data
        match timeout(Duration::from_millis(50), self.port.read(buf)).await {
            Ok(Ok(n)) => Ok(n),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Ok(0), // Timeout = no data available
        }
    }

    async fn poll(&mut self) -> crate::Result<ConnectionState> {
        Ok(ConnectionState::Connected)

        /* TODO
        // Check carrier detect (CD) - this indicates if we have an active connection
        // Most modems drop CD when the remote side hangs up
        let carrier_detect = self.port.read_cd().unwrap_or(false);

        // Check data set ready (DSR) - this indicates if the modem is powered on and ready
        let data_set_ready = self.port.read_dsr().unwrap_or(false);

        // A modem connection is considered connected if:
        // 1. The modem is ready (DSR is high)
        // 2. We have carrier detect (CD is high)
        if data_set_ready && carrier_detect {
            Ok(ConnectionState::Connected)
        } else {
            // Log why we're disconnected for debugging
            if !data_set_ready {
                log::debug!("Modem connection lost: DSR signal is low (modem not ready)");
            }
            if !carrier_detect {
                log::debug!("Modem connection lost: CD signal is low (no carrier)");
            }
            Ok(ConnectionState::Disconnected)
        }*/
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        self.port.write_all(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.hangup().await?;
        self.port.shutdown().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let cmd = ModemCommand::try_parse("ATZ").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Text("ATZ".to_string())]);
    }

    #[test]
    fn test_parse_carriage_return() {
        let cmd = ModemCommand::try_parse("ATZ^M").unwrap();
        assert_eq!(
            cmd.tokens(),
            &[
                ModemCommandToken::Text("ATZ".to_string()),
                ModemCommandToken::Control(13), // CR
            ]
        );
    }

    #[test]
    fn test_parse_line_feed() {
        let cmd = ModemCommand::try_parse("^J").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Control(10)]); // LF
    }

    #[test]
    fn test_parse_escape() {
        let cmd = ModemCommand::try_parse("^[").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Control(27)]); // ESC
    }

    #[test]
    fn test_parse_single_pause() {
        let cmd = ModemCommand::try_parse("~").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Pause(Duration::from_secs(1))]);
    }

    #[test]
    fn test_parse_multiple_pauses() {
        let cmd = ModemCommand::try_parse("~~~").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Pause(Duration::from_secs(3))]);
    }

    #[test]
    fn test_parse_hangup_command() {
        // Default hangup: ~~~+++~~~ATH0^M
        let cmd = ModemCommand::try_parse("~~~+++~~~ATH0^M").unwrap();
        assert_eq!(
            cmd.tokens(),
            &[
                ModemCommandToken::Pause(Duration::from_secs(3)),
                ModemCommandToken::Text("+++".to_string()),
                ModemCommandToken::Pause(Duration::from_secs(3)),
                ModemCommandToken::Text("ATH0".to_string()),
                ModemCommandToken::Control(13),
            ]
        );
    }

    #[test]
    fn test_parse_init_command() {
        let cmd = ModemCommand::try_parse("ATZ^M").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Text("ATZ".to_string()), ModemCommandToken::Control(13)]);
    }

    #[test]
    fn test_parse_dial_command() {
        let cmd = ModemCommand::try_parse("ATDT555-1234^M").unwrap();
        assert_eq!(
            cmd.tokens(),
            &[ModemCommandToken::Text("ATDT555-1234".to_string()), ModemCommandToken::Control(13)]
        );
    }

    #[test]
    fn test_parse_control_characters() {
        // Test various control characters
        assert_eq!(ModemCommand::try_parse("^@").unwrap().tokens(), &[ModemCommandToken::Control(0)]); // NUL
        assert_eq!(ModemCommand::try_parse("^A").unwrap().tokens(), &[ModemCommandToken::Control(1)]); // SOH
        assert_eq!(ModemCommand::try_parse("^G").unwrap().tokens(), &[ModemCommandToken::Control(7)]); // BEL
        assert_eq!(ModemCommand::try_parse("^H").unwrap().tokens(), &[ModemCommandToken::Control(8)]); // BS
        assert_eq!(ModemCommand::try_parse("^I").unwrap().tokens(), &[ModemCommandToken::Control(9)]); // TAB
        assert_eq!(ModemCommand::try_parse("^M").unwrap().tokens(), &[ModemCommandToken::Control(13)]); // CR
        assert_eq!(ModemCommand::try_parse("^Z").unwrap().tokens(), &[ModemCommandToken::Control(26)]); // SUB/EOF
        assert_eq!(ModemCommand::try_parse("^?").unwrap().tokens(), &[ModemCommandToken::Control(127)]); // DEL
    }

    #[test]
    fn test_parse_lowercase_control() {
        // Control characters should be case-insensitive
        assert_eq!(ModemCommand::try_parse("^m").unwrap().tokens(), &[ModemCommandToken::Control(13)]);
        assert_eq!(ModemCommand::try_parse("^j").unwrap().tokens(), &[ModemCommandToken::Control(10)]);
    }

    #[test]
    fn test_parse_invalid_control_sequence() {
        // Invalid control sequences should return an error
        assert!(ModemCommand::try_parse("^1").is_err());
    }

    #[test]
    fn test_parse_lone_caret() {
        // Lone caret at end should return an error
        assert!(ModemCommand::try_parse("test^").is_err());
    }

    #[test]
    fn test_parse_empty_string() {
        let cmd = ModemCommand::try_parse("").unwrap();
        assert!(cmd.is_empty());
    }

    #[test]
    fn test_parse_mixed_pauses_and_text() {
        let cmd = ModemCommand::try_parse("AT~OK~~END").unwrap();
        assert_eq!(
            cmd.tokens(),
            &[
                ModemCommandToken::Text("AT".to_string()),
                ModemCommandToken::Pause(Duration::from_secs(1)),
                ModemCommandToken::Text("OK".to_string()),
                ModemCommandToken::Pause(Duration::from_secs(2)),
                ModemCommandToken::Text("END".to_string()),
            ]
        );
    }

    #[test]
    fn test_to_bytes_text() {
        let cmd = ModemCommand::try_parse("ATZ").unwrap();
        assert_eq!(cmd.to_bytes(), b"ATZ");
    }

    #[test]
    fn test_to_bytes_control() {
        let cmd = ModemCommand::try_parse("ATZ^M").unwrap();
        assert_eq!(cmd.to_bytes(), b"ATZ\r");
    }

    #[test]
    fn test_to_bytes_pause_ignored() {
        let cmd = ModemCommand::try_parse("AT~Z").unwrap();
        // Pauses are not included in bytes output
        assert_eq!(cmd.to_bytes(), b"ATZ");
    }

    #[test]
    fn test_to_bytes_complex() {
        let cmd = ModemCommand::try_parse("~~~+++~~~ATH0^M").unwrap();
        assert_eq!(cmd.to_bytes(), b"+++ATH0\r");
    }

    #[test]
    fn test_display() {
        let cmd = ModemCommand::try_parse("ATZ^M").unwrap();
        assert_eq!(cmd.to_string(), "ATZ^M");
    }

    #[test]
    fn test_display_pause() {
        let cmd = ModemCommand::try_parse("~~~+++~~~ATH0^M").unwrap();
        assert_eq!(cmd.to_string(), "~~~+++~~~ATH0^M");
    }

    #[test]
    fn test_from_str() {
        let cmd: ModemCommand = "ATZ^M".parse().unwrap();
        assert_eq!(cmd.to_bytes(), b"ATZ\r");
    }

    #[test]
    fn test_try_from_string() {
        let cmd = ModemCommand::try_from("ATZ^M".to_string()).unwrap();
        assert_eq!(cmd.to_bytes(), b"ATZ\r");
    }

    #[test]
    fn test_iter() {
        let cmd = ModemCommand::try_parse("AT^M").unwrap();
        let tokens: Vec<_> = cmd.iter().collect();
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn test_into_iter() {
        let cmd = ModemCommand::try_parse("AT^M").unwrap();
        let tokens: Vec<_> = cmd.into_iter().collect();
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn test_push_methods() {
        let mut cmd = ModemCommand::new();
        cmd.push_text("ATZ");
        cmd.push_control(13);
        cmd.push_pause(Duration::from_secs(1));

        assert_eq!(cmd.len(), 3);
        assert_eq!(cmd.to_bytes(), b"ATZ\r");
    }

    #[test]
    fn test_validate_valid_commands() {
        assert!(ModemCommand::validate("ATZ").is_valid());
        assert!(ModemCommand::validate("ATZ^M").is_valid());
        assert!(ModemCommand::validate("~~~+++~~~ATH0^M").is_valid());
        assert!(ModemCommand::validate("AT^[K").is_valid()); // ESC
        assert!(ModemCommand::validate("^@^A^Z^[^]^^").is_valid());
        assert!(ModemCommand::validate("^?").is_valid()); // DEL
        assert!(ModemCommand::validate("").is_valid()); // Empty is valid
    }

    #[test]
    fn test_validate_invalid_control_sequence() {
        assert_eq!(
            ModemCommand::validate("AT^1Z"),
            ModemCommandValidationResult::InvalidControlSequence { char: '1', position: 2 }
        );
        assert_eq!(
            ModemCommand::validate("^!"),
            ModemCommandValidationResult::InvalidControlSequence { char: '!', position: 0 }
        );
        assert_eq!(
            ModemCommand::validate("^{"),
            ModemCommandValidationResult::InvalidControlSequence { char: '{', position: 0 }
        );
    }

    #[test]
    fn test_validate_incomplete_control_sequence() {
        assert_eq!(
            ModemCommand::validate("^"),
            ModemCommandValidationResult::IncompleteControlSequence { position: 0 }
        );
        assert_eq!(
            ModemCommand::validate("test^"),
            ModemCommandValidationResult::IncompleteControlSequence { position: 4 }
        );
        assert_eq!(
            ModemCommand::validate("ATZ^M^"),
            ModemCommandValidationResult::IncompleteControlSequence { position: 5 }
        );
    }

    #[test]
    fn test_is_valid() {
        assert!(ModemCommand::is_valid("ATZ^M"));
        assert!(ModemCommand::is_valid("~~~+++"));
        assert!(!ModemCommand::is_valid("^1"));
        assert!(!ModemCommand::is_valid("^!test"));
    }

    #[test]
    fn test_validate_invalid_character() {
        // Non-ASCII characters should be rejected
        assert_eq!(
            ModemCommand::validate("ATäZ"),
            ModemCommandValidationResult::InvalidCharacter { char: 'ä', position: 2 }
        );
        assert_eq!(
            ModemCommand::validate("日本語"),
            ModemCommandValidationResult::InvalidCharacter { char: '日', position: 0 }
        );
        assert_eq!(
            ModemCommand::validate("test™"),
            ModemCommandValidationResult::InvalidCharacter { char: '™', position: 4 }
        );
        // Emoji
        assert_eq!(
            ModemCommand::validate("AT😀"),
            ModemCommandValidationResult::InvalidCharacter { char: '😀', position: 2 }
        );
    }

    // Tests for \xNN hex escape sequences
    #[test]
    fn test_parse_hex_escape() {
        let cmd = ModemCommand::try_parse("\\x1B").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Byte(0x1B)]); // ESC
    }

    #[test]
    fn test_parse_hex_escape_high_ascii() {
        let cmd = ModemCommand::try_parse("\\x80\\xFF").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Byte(0x80), ModemCommandToken::Byte(0xFF),]);
    }

    #[test]
    fn test_parse_hex_escape_lowercase() {
        let cmd = ModemCommand::try_parse("\\x0d\\x0a").unwrap();
        assert_eq!(
            cmd.tokens(),
            &[
                ModemCommandToken::Byte(0x0D), // CR
                ModemCommandToken::Byte(0x0A), // LF
            ]
        );
    }

    #[test]
    fn test_parse_hex_escape_mixed() {
        let cmd = ModemCommand::try_parse("AT\\x1B[2J^M").unwrap();
        assert_eq!(
            cmd.tokens(),
            &[
                ModemCommandToken::Text("AT".to_string()),
                ModemCommandToken::Byte(0x1B), // ESC
                ModemCommandToken::Text("[2J".to_string()),
                ModemCommandToken::Control(13), // CR
            ]
        );
    }

    #[test]
    fn test_parse_hex_escape_invalid_returns_error() {
        // Not enough hex digits - returns error
        assert!(ModemCommand::try_parse("\\xG").is_err());
        assert!(ModemCommand::try_parse("\\x1").is_err());
        assert!(ModemCommand::try_parse("\\x").is_err());
    }

    #[test]
    fn test_parse_backslash_alone() {
        let cmd = ModemCommand::try_parse("test\\path").unwrap();
        assert_eq!(cmd.tokens(), &[ModemCommandToken::Text("test\\path".to_string())]);
    }

    #[test]
    fn test_display_hex_escape() {
        // Byte tokens always display as \xNN
        let mut cmd = ModemCommand::new();
        cmd.push_byte(0x80);
        cmd.push_byte(0xFF);
        assert_eq!(cmd.to_string(), "\\x80\\xFF");
    }

    #[test]
    fn test_display_control_vs_byte() {
        // Control chars 0-31 and 127 use ^X notation
        let mut cmd = ModemCommand::new();
        cmd.push_control(13); // CR
        cmd.push_control(27); // ESC
        assert_eq!(cmd.to_string(), "^M^[");

        // Byte tokens always use \xNN notation
        let mut cmd2 = ModemCommand::new();
        cmd2.push_byte(13); // Same value, but as Byte
        cmd2.push_byte(128);
        assert_eq!(cmd2.to_string(), "\\x0D\\x80");
    }

    #[test]
    fn test_to_bytes_hex_escape() {
        let cmd = ModemCommand::try_parse("AT\\x1B[2J").unwrap();
        assert_eq!(cmd.to_bytes(), b"AT\x1B[2J");
    }

    #[test]
    fn test_validate_hex_escape_valid() {
        assert!(ModemCommand::validate("\\x00").is_valid());
        assert!(ModemCommand::validate("\\xFF").is_valid());
        assert!(ModemCommand::validate("\\x1B").is_valid());
        assert!(ModemCommand::validate("AT\\x0D\\x0A").is_valid());
        assert!(ModemCommand::validate("\\x80\\x81\\x82").is_valid());
    }

    #[test]
    fn test_validate_hex_escape_invalid() {
        // Not enough hex digits
        assert_eq!(ModemCommand::validate("\\x"), ModemCommandValidationResult::InvalidHexSequence { position: 0 });
        assert_eq!(ModemCommand::validate("\\x1"), ModemCommandValidationResult::InvalidHexSequence { position: 0 });
        assert_eq!(
            ModemCommand::validate("AT\\xG"),
            ModemCommandValidationResult::InvalidHexSequence { position: 2 }
        );
        assert_eq!(
            ModemCommand::validate("\\x1G"),
            ModemCommandValidationResult::InvalidHexSequence { position: 0 }
        );
    }

    #[test]
    fn test_validate_backslash_without_x() {
        // Plain backslash is valid
        assert!(ModemCommand::validate("test\\path").is_valid());
        assert!(ModemCommand::validate("\\").is_valid());
        assert!(ModemCommand::validate("\\\\").is_valid());
    }

    #[test]
    fn test_roundtrip_hex() {
        // Parse and display should roundtrip for high ASCII
        let original = "AT\\x80\\xFF^M";
        let cmd = ModemCommand::try_parse(original).unwrap();
        assert_eq!(cmd.to_string(), original);
    }

    #[test]
    fn test_serde_serialize() {
        let cmd = ModemCommand::try_parse("ATZ^M").unwrap();
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, "\"ATZ^M\"");
    }

    #[test]
    fn test_serde_deserialize() {
        let json = "\"ATZ^M\"";
        let cmd: ModemCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.to_bytes(), b"ATZ\r");
    }

    #[test]
    fn test_serde_roundtrip() {
        let original = ModemCommand::try_parse("~~~+++~~~ATH0^M").unwrap();
        let json = serde_json::to_string(&original).unwrap();
        let restored: ModemCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_serde_roundtrip_hex() {
        let original = ModemCommand::try_parse("AT\\x80\\xFF^M").unwrap();
        let json = serde_json::to_string(&original).unwrap();
        let restored: ModemCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_serde_deserialize_invalid() {
        let json = "\"^1\""; // Invalid control sequence
        let result: Result<ModemCommand, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
