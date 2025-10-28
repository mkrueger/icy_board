#![allow(dead_code)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serial2_tokio::{SerialPort, Settings};

use crate::{Connection, ConnectionState, ConnectionType};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharSize {
    #[default]
    Bits8,
    Bits7,
    Bits6,
    Bits5,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopBits {
    #[default]
    One,
    Two,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Parity {
    #[default]
    None,
    Odd,
    Even,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowControl {
    #[default]
    None,
    XonXoff,
    RtsCts,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Serial {
    pub device: String,
    pub baud_rate: u32,

    pub char_size: CharSize,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
}

impl Serial {
    pub fn open(&self) -> crate::Result<SerialPort> {
        let port = SerialPort::open(&self.device, move |mut settings: Settings| {
            settings.set_raw();
            settings.set_baud_rate(self.baud_rate)?;
            match self.char_size {
                CharSize::Bits5 => settings.set_char_size(serial2_tokio::CharSize::Bits5),
                CharSize::Bits6 => settings.set_char_size(serial2_tokio::CharSize::Bits6),
                CharSize::Bits7 => settings.set_char_size(serial2_tokio::CharSize::Bits7),
                CharSize::Bits8 => settings.set_char_size(serial2_tokio::CharSize::Bits8),
            }
            match self.stop_bits {
                StopBits::One => settings.set_stop_bits(serial2_tokio::StopBits::One),
                StopBits::Two => settings.set_stop_bits(serial2_tokio::StopBits::Two),
            }
            match self.parity {
                Parity::None => settings.set_parity(serial2_tokio::Parity::None),
                Parity::Odd => settings.set_parity(serial2_tokio::Parity::Odd),
                Parity::Even => settings.set_parity(serial2_tokio::Parity::Even),
            }

            match self.flow_control {
                FlowControl::None => settings.set_flow_control(serial2_tokio::FlowControl::None),
                FlowControl::XonXoff => settings.set_flow_control(serial2_tokio::FlowControl::XonXoff),
                FlowControl::RtsCts => settings.set_flow_control(serial2_tokio::FlowControl::RtsCts),
            }
            Ok(settings)
        })?;
        Ok(port)
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
        if !self.port.read_dsr().unwrap_or_default() {
            return Ok(0);
        }
        let res = self.port.read(buf).await?;
        //  println!("Read {:?} bytes", &buf[..res]);
        Ok(res)
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
        self.port.set_dtr(false)?;
        Ok(())
    }
}
