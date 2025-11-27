#![allow(dead_code)]

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serial2_tokio::SerialPort;
use tokio::{io::AsyncWriteExt, time::timeout};

use crate::{
    Connection, ConnectionState, ConnectionType,
    serial::{FlowControl, Format, Serial},
};

/// Parsed modem command element
#[derive(Clone, Debug, PartialEq)]
pub enum ModemCommand {
    /// Raw text to send as-is
    Text(String),
    /// Pause for specified duration (~ = 1 second)
    Pause(Duration),
    /// Control character (^M = CR, ^J = LF, ^[ = ESC, etc.)
    Control(u8),
}

impl ModemCommand {
    /// Parse a modem command string into a list of commands
    /// Supports:
    /// - `~` for 1 second pause
    /// - `^X` for control characters (^M = CR, ^[ = ESC, etc.)
    /// - Regular text
    pub fn parse(input: &str) -> Vec<ModemCommand> {
        let mut commands = Vec::new();
        let mut chars = input.chars().peekable();
        let mut text_buffer = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '~' => {
                    // Flush text buffer
                    if !text_buffer.is_empty() {
                        commands.push(ModemCommand::Text(std::mem::take(&mut text_buffer)));
                    }
                    // Count consecutive tildes for longer pauses
                    let mut pause_secs = 1;
                    while chars.peek() == Some(&'~') {
                        chars.next();
                        pause_secs += 1;
                    }
                    commands.push(ModemCommand::Pause(Duration::from_secs(pause_secs)));
                }
                '^' => {
                    if let Some(&next_ch) = chars.peek() {
                        // Flush text buffer
                        if !text_buffer.is_empty() {
                            commands.push(ModemCommand::Text(std::mem::take(&mut text_buffer)));
                        }
                        chars.next();
                        // Convert ^X to control character (^A = 1, ^B = 2, ..., ^Z = 26, ^[ = 27, etc.)
                        let ctrl_char = match next_ch.to_ascii_uppercase() {
                            '@' => 0, // ^@ = NUL
                            'A'..='Z' => next_ch.to_ascii_uppercase() as u8 - b'A' + 1,
                            '[' => 27,  // ^[ = ESC
                            '\\' => 28, // ^\
                            ']' => 29,  // ^]
                            '^' => 30,  // ^^
                            '_' => 31,  // ^_
                            '?' => 127, // ^? = DEL
                            _ => {
                                // Not a valid control sequence, treat as literal ^
                                text_buffer.push('^');
                                text_buffer.push(next_ch);
                                continue;
                            }
                        };
                        commands.push(ModemCommand::Control(ctrl_char));
                    } else {
                        // Lone ^ at end, treat as literal
                        text_buffer.push('^');
                    }
                }
                _ => {
                    text_buffer.push(ch);
                }
            }
        }

        // Flush remaining text
        if !text_buffer.is_empty() {
            commands.push(ModemCommand::Text(text_buffer));
        }

        commands
    }

    /// Convert commands back to bytes for sending
    pub fn to_bytes(commands: &[ModemCommand]) -> Vec<u8> {
        let mut result = Vec::new();
        for cmd in commands {
            match cmd {
                ModemCommand::Text(s) => result.extend_from_slice(s.as_bytes()),
                ModemCommand::Pause(_) => {} // Pauses are handled separately during send
                ModemCommand::Control(c) => result.push(*c),
            }
        }
        result
    }
}

/// Modem response codes for parsing modem output
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModemResponse {
    /// The response string to match (e.g., "CONNECT", "NO CARRIER")
    pub pattern: String,
    /// The type of response this represents
    pub response_type: ModemResponseType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

impl ModemResponse {
    pub fn new(pattern: impl Into<String>, response_type: ModemResponseType) -> Self {
        Self {
            pattern: pattern.into(),
            response_type,
        }
    }
}

fn default_modem_responses() -> Vec<ModemResponse> {
    vec![
        ModemResponse::new("CONNECT", ModemResponseType::Connect),
        ModemResponse::new("OK", ModemResponseType::Ok),
        ModemResponse::new("RING", ModemResponseType::Ring),
        ModemResponse::new("NO CARRIER", ModemResponseType::NoCarrier),
        ModemResponse::new("ERROR", ModemResponseType::Error),
        ModemResponse::new("NO DIAL TONE", ModemResponseType::NoDialtone),
        ModemResponse::new("BUSY", ModemResponseType::Busy),
        ModemResponse::new("NO ANSWER", ModemResponseType::NoAnswer),
    ]
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
    pub init_command: String,

    #[serde(default = "default_dial_prefix")]
    pub dial_prefix: String,

    #[serde(default = "default_dial_postfix")]
    pub dial_postfix: String,

    #[serde(default = "default_hangup_command")]
    pub hangup_command: String,

    #[serde(default = "default_modem_responses")]
    pub modem_responses: Vec<ModemResponse>,
}

fn default_init_command() -> String {
    "ATZ^M".to_string()
}

fn default_dial_prefix() -> String {
    "ATDT".to_string()
}

fn default_dial_postfix() -> String {
    "^M".to_string()
}

fn default_hangup_command() -> String {
    "~~~+++~~~ATH0^M".to_string()
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
            dial_postfix: default_dial_postfix(),
            hangup_command: default_hangup_command(),
            modem_responses: default_modem_responses(),
        }
    }
}

impl ModemConfiguration {
    /// Parse a modem response from the given text
    /// Returns the first matching response type, if any
    pub fn parse_response(&self, text: &str) -> Option<&ModemResponse> {
        let text_upper = text.to_uppercase();
        self.modem_responses.iter().find(|r| text_upper.contains(&r.pattern.to_uppercase()))
    }
}

pub struct ModemConnection {
    modem: ModemConfiguration,
    port: Box<SerialPort>,
}

impl ModemConnection {
    pub async fn open(modem: ModemConfiguration, call_number: String) -> crate::Result<Self> {
        let serial: Serial = modem.clone().into();
        let port = serial.open()?;
        port.write_all(modem.init_command.as_bytes()).await?;
        port.write_all(b"\n").await?;
        port.write_all(modem.dial_prefix.as_bytes()).await?;
        port.write_all(call_number.as_bytes()).await?;
        port.write_all(b"\n").await?;
        Ok(Self { modem, port: Box::new(port) })
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
        // Non-blocking: return immediately if no data available
        match timeout(Duration::from_millis(1), self.port.read(buf)).await {
            Ok(Ok(n)) => Ok(n),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Ok(0), // Timeout = no data available
        }
    }

    async fn poll(&mut self) -> crate::Result<ConnectionState> {
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
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        self.port.write_all(buf).await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> crate::Result<()> {
        self.port.shutdown().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let cmds = ModemCommand::parse("ATZ");
        assert_eq!(cmds, vec![ModemCommand::Text("ATZ".to_string())]);
    }

    #[test]
    fn test_parse_carriage_return() {
        let cmds = ModemCommand::parse("ATZ^M");
        assert_eq!(
            cmds,
            vec![
                ModemCommand::Text("ATZ".to_string()),
                ModemCommand::Control(13), // CR
            ]
        );
    }

    #[test]
    fn test_parse_line_feed() {
        let cmds = ModemCommand::parse("^J");
        assert_eq!(cmds, vec![ModemCommand::Control(10)]); // LF
    }

    #[test]
    fn test_parse_escape() {
        let cmds = ModemCommand::parse("^[");
        assert_eq!(cmds, vec![ModemCommand::Control(27)]); // ESC
    }

    #[test]
    fn test_parse_single_pause() {
        let cmds = ModemCommand::parse("~");
        assert_eq!(cmds, vec![ModemCommand::Pause(Duration::from_secs(1))]);
    }

    #[test]
    fn test_parse_multiple_pauses() {
        let cmds = ModemCommand::parse("~~~");
        assert_eq!(cmds, vec![ModemCommand::Pause(Duration::from_secs(3))]);
    }

    #[test]
    fn test_parse_hangup_command() {
        // Default hangup: ~~~+++~~~ATH0^M
        let cmds = ModemCommand::parse("~~~+++~~~ATH0^M");
        assert_eq!(
            cmds,
            vec![
                ModemCommand::Pause(Duration::from_secs(3)),
                ModemCommand::Text("+++".to_string()),
                ModemCommand::Pause(Duration::from_secs(3)),
                ModemCommand::Text("ATH0".to_string()),
                ModemCommand::Control(13),
            ]
        );
    }

    #[test]
    fn test_parse_init_command() {
        let cmds = ModemCommand::parse("ATZ^M");
        assert_eq!(cmds, vec![ModemCommand::Text("ATZ".to_string()), ModemCommand::Control(13),]);
    }

    #[test]
    fn test_parse_dial_command() {
        let cmds = ModemCommand::parse("ATDT555-1234^M");
        assert_eq!(cmds, vec![ModemCommand::Text("ATDT555-1234".to_string()), ModemCommand::Control(13),]);
    }

    #[test]
    fn test_parse_control_characters() {
        // Test various control characters
        assert_eq!(ModemCommand::parse("^@"), vec![ModemCommand::Control(0)]); // NUL
        assert_eq!(ModemCommand::parse("^A"), vec![ModemCommand::Control(1)]); // SOH
        assert_eq!(ModemCommand::parse("^G"), vec![ModemCommand::Control(7)]); // BEL
        assert_eq!(ModemCommand::parse("^H"), vec![ModemCommand::Control(8)]); // BS
        assert_eq!(ModemCommand::parse("^I"), vec![ModemCommand::Control(9)]); // TAB
        assert_eq!(ModemCommand::parse("^M"), vec![ModemCommand::Control(13)]); // CR
        assert_eq!(ModemCommand::parse("^Z"), vec![ModemCommand::Control(26)]); // SUB/EOF
        assert_eq!(ModemCommand::parse("^?"), vec![ModemCommand::Control(127)]); // DEL
    }

    #[test]
    fn test_parse_lowercase_control() {
        // Control characters should be case-insensitive
        assert_eq!(ModemCommand::parse("^m"), vec![ModemCommand::Control(13)]);
        assert_eq!(ModemCommand::parse("^j"), vec![ModemCommand::Control(10)]);
    }

    #[test]
    fn test_parse_invalid_control_sequence() {
        // Invalid control sequences should be treated as literal text
        let cmds = ModemCommand::parse("^1");
        assert_eq!(cmds, vec![ModemCommand::Text("^1".to_string())]);
    }

    #[test]
    fn test_parse_lone_caret() {
        let cmds = ModemCommand::parse("test^");
        assert_eq!(cmds, vec![ModemCommand::Text("test^".to_string())]);
    }

    #[test]
    fn test_parse_empty_string() {
        let cmds = ModemCommand::parse("");
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_parse_mixed_pauses_and_text() {
        let cmds = ModemCommand::parse("AT~OK~~END");
        assert_eq!(
            cmds,
            vec![
                ModemCommand::Text("AT".to_string()),
                ModemCommand::Pause(Duration::from_secs(1)),
                ModemCommand::Text("OK".to_string()),
                ModemCommand::Pause(Duration::from_secs(2)),
                ModemCommand::Text("END".to_string()),
            ]
        );
    }

    #[test]
    fn test_to_bytes_text() {
        let cmds = vec![ModemCommand::Text("ATZ".to_string())];
        assert_eq!(ModemCommand::to_bytes(&cmds), b"ATZ");
    }

    #[test]
    fn test_to_bytes_control() {
        let cmds = vec![ModemCommand::Text("ATZ".to_string()), ModemCommand::Control(13)];
        assert_eq!(ModemCommand::to_bytes(&cmds), b"ATZ\r");
    }

    #[test]
    fn test_to_bytes_pause_ignored() {
        let cmds = vec![
            ModemCommand::Text("AT".to_string()),
            ModemCommand::Pause(Duration::from_secs(1)),
            ModemCommand::Text("Z".to_string()),
        ];
        // Pauses are not included in bytes output
        assert_eq!(ModemCommand::to_bytes(&cmds), b"ATZ");
    }

    #[test]
    fn test_to_bytes_complex() {
        let cmds = ModemCommand::parse("~~~+++~~~ATH0^M");
        let bytes = ModemCommand::to_bytes(&cmds);
        assert_eq!(bytes, b"+++ATH0\r");
    }
}
