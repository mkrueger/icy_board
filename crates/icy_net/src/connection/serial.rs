#![allow(dead_code)]

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serial2_tokio::{SerialPort, Settings};
use tokio::{io::AsyncWriteExt, time::timeout};

use crate::{Connection, ConnectionState, ConnectionType};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CharSize {
    #[default]
    Bits8,
    Bits7,
    Bits6,
    Bits5,
}

impl From<CharSize> for char {
    fn from(cs: CharSize) -> char {
        match cs {
            CharSize::Bits5 => '5',
            CharSize::Bits6 => '6',
            CharSize::Bits7 => '7',
            CharSize::Bits8 => '8',
        }
    }
}

impl TryFrom<char> for CharSize {
    type Error = String;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            '5' => Ok(CharSize::Bits5),
            '6' => Ok(CharSize::Bits6),
            '7' => Ok(CharSize::Bits7),
            '8' => Ok(CharSize::Bits8),
            _ => Err(format!("Invalid char size '{}': expected 5-8", c)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StopBits {
    #[default]
    One,
    Two,
}

impl From<StopBits> for char {
    fn from(sb: StopBits) -> char {
        match sb {
            StopBits::One => '1',
            StopBits::Two => '2',
        }
    }
}

impl TryFrom<char> for StopBits {
    type Error = String;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            '1' => Ok(StopBits::One),
            '2' => Ok(StopBits::Two),
            _ => Err(format!("Invalid stop bits '{}': expected 1/2", c)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Parity {
    #[default]
    None,
    Odd,
    Even,
}

impl From<Parity> for char {
    fn from(p: Parity) -> char {
        match p {
            Parity::None => 'N',
            Parity::Odd => 'O',
            Parity::Even => 'E',
        }
    }
}

impl TryFrom<char> for Parity {
    type Error = String;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c.to_ascii_uppercase() {
            'N' => Ok(Parity::None),
            'O' => Ok(Parity::Odd),
            'E' => Ok(Parity::Even),
            _ => Err(format!("Invalid parity '{}': expected N/O/E", c)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowControl {
    #[default]
    None,
    XonXoff,
    RtsCts,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Format {
    pub char_size: CharSize,
    pub stop_bits: StopBits,
    pub parity: Parity,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", char::from(self.char_size), char::from(self.parity), char::from(self.stop_bits))
    }
}

impl std::str::FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() != 3 {
            return Err(format!("Invalid format string '{}': expected 3 characters (e.g., 8N1)", s));
        }

        let chars: Vec<char> = s.chars().collect();

        let char_size = CharSize::try_from(chars[0])?;
        let parity = Parity::try_from(chars[1])?;
        let stop_bits = StopBits::try_from(chars[2])?;

        Ok(Format { char_size, stop_bits, parity })
    }
}

impl<'de> Deserialize<'de> for Format {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for Format {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Serial {
    pub device: String,
    pub baud_rate: u32,
    #[serde(default)]
    pub format: Format,
    #[serde(default)]
    pub flow_control: FlowControl,
}

impl Default for Serial {
    fn default() -> Self {
        Self {
            device: if cfg!(target_os = "windows") {
                "COM1:".to_string()
            } else {
                "/dev/ttyUSB0".to_string()
            },
            baud_rate: 57600,
            format: Default::default(),
            flow_control: Default::default(),
        }
    }
}

impl Serial {
    pub fn open(&self) -> crate::Result<SerialPort> {
        let port = SerialPort::open(&self.device, move |mut settings: Settings| {
            settings.set_raw();
            settings.set_baud_rate(self.baud_rate)?;
            match self.format.char_size {
                CharSize::Bits5 => settings.set_char_size(serial2_tokio::CharSize::Bits5),
                CharSize::Bits6 => settings.set_char_size(serial2_tokio::CharSize::Bits6),
                CharSize::Bits7 => settings.set_char_size(serial2_tokio::CharSize::Bits7),
                CharSize::Bits8 => settings.set_char_size(serial2_tokio::CharSize::Bits8),
            }
            match self.format.stop_bits {
                StopBits::One => settings.set_stop_bits(serial2_tokio::StopBits::One),
                StopBits::Two => settings.set_stop_bits(serial2_tokio::StopBits::Two),
            }
            match self.format.parity {
                Parity::None => settings.set_parity(serial2_tokio::Parity::None),
                Parity::Odd => settings.set_parity(serial2_tokio::Parity::Odd),
                Parity::Even => settings.set_parity(serial2_tokio::Parity::Even),
            }

            match self.flow_control {
                FlowControl::None => settings.set_flow_control(serial2_tokio::FlowControl::None),
                FlowControl::XonXoff => settings.set_flow_control(serial2_tokio::FlowControl::XonXoff),
                FlowControl::RtsCts => settings.set_flow_control(serial2_tokio::FlowControl::RtsCts),
            }
            /*
            // Set CLOCAL (ignore modem control lines) - essential for local serial communication
            // Without this, reads may block waiting for carrier detect
            #[cfg(unix)]
            {
                let termios = settings.as_termios_mut();
                termios.c_cflag |= libc_unix::CLOCAL; // Ignore modem control lines
                termios.c_cflag |= libc_unix::CREAD;  // Enable receiver
                termios.c_cflag &= !libc_unix::HUPCL; // Don't hang up on close

                // VMIN=1, VTIME=0: blocking read, return when at least 1 byte available
                termios.c_cc[libc_unix::VMIN as usize] = 1;
                termios.c_cc[libc_unix::VTIME as usize] = 0;
            }*/

            Ok(settings)
        })?;

        // Set DTR (Data Terminal Ready) - tells the modem we're ready
        // This is essential for hardware modems to respond
        if let Err(e) = port.set_dtr(true) {
            log::warn!("Failed to set DTR: {}", e);
        }

        // Set RTS (Request To Send) - enables communication
        // Only set if not using hardware flow control (RTS/CTS)
        if self.flow_control != FlowControl::RtsCts {
            if let Err(e) = port.set_rts(true) {
                log::warn!("Failed to set RTS: {}", e);
            }
        }

        // Discard any garbage in the buffers
        let _ = port.discard_buffers();
        Ok(port)
    }
}

impl From<super::modem::ModemConfiguration> for Serial {
    fn from(modem: super::modem::ModemConfiguration) -> Self {
        Serial {
            device: modem.device,
            baud_rate: modem.baud_rate,
            format: modem.format,
            flow_control: modem.flow_control,
        }
    }
}

pub struct SerialConnection {
    port: Box<SerialPort>,
}

impl SerialConnection {
    pub fn open(serial: Serial) -> crate::Result<Self> {
        Ok(Self {
            port: Box::new(serial.open()?),
        })
    }
}

#[async_trait]
impl Connection for SerialConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Serial
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        let res = self.port.read(buf).await?;
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
        // always connected for serial
        Ok(ConnectionState::Connected)
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
