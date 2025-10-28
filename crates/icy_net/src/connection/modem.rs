#![allow(dead_code)]

use async_trait::async_trait;
use serial2_tokio::SerialPort;

use crate::{Connection, ConnectionState, ConnectionType, serial::Serial};

#[derive(Clone, Debug, PartialEq)]
pub struct ModemConfiguration {
    pub init_string: String,
    pub dial_string: String,
}

pub struct ModemConnection {
    modem: ModemConfiguration,
    port: Box<SerialPort>,
}

impl ModemConnection {
    pub async fn open(serial: Serial, modem: ModemConfiguration, call_number: String) -> crate::Result<Self> {
        let port = serial.open()?;
        port.write_all(modem.init_string.as_bytes()).await?;
        port.write_all(b"\n").await?;
        port.write_all(modem.dial_string.as_bytes()).await?;
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
