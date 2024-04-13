#![allow(dead_code)]

use serial::{prelude::*, CharSize, FlowControl, StopBits};
use std::io::{self, Read, Write};

use crate::{Connection, ConnectionType};

#[derive(Clone, Debug, PartialEq)]
pub struct Serial {
    pub device: String,
    pub baud_rate: usize,

    pub char_size: CharSize,
    pub stop_bits: StopBits,
    pub parity: serial::Parity,

    pub flow_control: FlowControl,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModemConfiguration {
    pub init_string: String,
    pub dial_string: String,
}

pub struct ModemConnection {
    modem: ModemConfiguration,
    serial: Serial,
    port: Box<dyn serial::SerialPort>,
}

impl ModemConnection {
    pub fn open(serial: Serial, modem: ModemConfiguration, call_number: String) -> crate::Result<Self> {
        let mut port = serial::open(&serial.device)?;
        port.reconfigure(&|settings| {
            settings.set_baud_rate(serial::BaudRate::from_speed(serial.baud_rate))?;
            settings.set_char_size(serial.char_size);
            settings.set_parity(serial.parity);
            settings.set_stop_bits(serial.stop_bits);
            settings.set_flow_control(serial.flow_control);
            Ok(())
        })?;
        port.write_all(modem.init_string.as_bytes())?;
        port.write_all(b"\n")?;
        port.write_all(modem.dial_string.as_bytes())?;
        port.write_all(call_number.as_bytes())?;
        port.write_all(b"\n")?;
        Ok(Self {
            serial,
            modem,
            port: Box::new(port),
        })
    }
}

impl Connection for ModemConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Modem
    }

    fn shutdown(&mut self) -> crate::Result<()> {
        self.port.set_dtr(false)?;
        self.port.set_dtr(true)?;
        Ok(())
    }
}

impl Read for ModemConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.port.read(buf)
    }
}

impl Write for ModemConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.port.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.port.flush()
    }
}
