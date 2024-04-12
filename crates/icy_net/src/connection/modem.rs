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

/*
impl AsyncRead for ModemConnection {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        loop {
            let data = self.get_mut().port.read(buf.initialize_unfilled());
            match data {
                Ok(bytes_read) => {
                    buf.advance(bytes_read);
                    return Poll::Ready(Ok(()));
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    return Poll::Ready(Err(err));
                }
            }
        }
    }
}

impl AsyncWrite for ModemConnection {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        loop {
            match self.get_mut().port.write(buf) {
                Ok(result) => return Poll::Ready(Ok(result)),
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    return Poll::Ready(Err(err));
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            match self.get_mut().port.flush() {
                Ok(()) => return Poll::Ready(Ok(())),
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    return Poll::Ready(Err(err));
                }
            }
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let _ = self.poll_flush(cx)?;
        Ok(()).into()
    }
}


*/