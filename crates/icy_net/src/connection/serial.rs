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
