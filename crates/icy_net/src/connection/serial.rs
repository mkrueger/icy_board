#![allow(dead_code)]

use serial::prelude::*;
use std::io::{self, Read, Write};

use crate::{modem::Serial, Connection, ConnectionType};

pub struct SerialConnection {
    serial: Serial,
    port: Box<dyn serial::SerialPort>,
}

impl SerialConnection {
    pub fn open(serial: Serial) -> crate::Result<Self> {
        let mut port = serial::open(&serial.device)?;
        port.reconfigure(&|settings| {
            settings.set_baud_rate(serial::BaudRate::from_speed(serial.baud_rate))?;
            settings.set_char_size(serial.char_size);
            settings.set_parity(serial.parity);
            settings.set_stop_bits(serial.stop_bits);
            settings.set_flow_control(serial.flow_control);
            Ok(())
        })?;
        Ok(Self { serial, port: Box::new(port) })
    }
}

impl Connection for SerialConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Modem
    }

    fn shutdown(&mut self) -> crate::Result<()> {
        self.port.set_dtr(false)?;
        self.port.set_dtr(true)?;
        Ok(())
    }
}

impl Read for SerialConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.port.read(buf)
    }
}

impl Write for SerialConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.port.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.port.flush()
    }
}
