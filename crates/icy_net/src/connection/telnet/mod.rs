use std::{
    io::{self, ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::{Connection, ConnectionType, NetError};

mod telnet_cmd;
mod telnet_option;

#[derive(Debug)]
enum ParserState {
    Data,
    Iac,
    Will,
    Wont,
    Do,
    Dont,
    SubCommand(i32),
}

mod terminal_type {
    pub const IS: u8 = 0x00;
    pub const SEND: u8 = 0x01;
    // pub const MAXLN: usize = 40;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Terminal {
    #[default]
    Ansi,
    Avatar,
    Ascii,
    PETscii,
    ATAscii,
    ViewData,
    Mode7,
    Rip,
    IGS,
}

pub struct TermCaps {
    pub window_size: (u16, u16), // width, height
    pub terminal: Terminal,
}

pub struct TelnetConnection {
    tcp_stream: TcpStream,
    caps: TermCaps,
    state: ParserState,
}

impl TelnetConnection {
    pub fn open<A: ToSocketAddrs>(addr: &A, caps: TermCaps, timeout: Duration) -> crate::Result<Self> {
        for mut addr in addr.to_socket_addrs()? {
            if addr.port() == 0 {
                addr.set_port(23);
            }
            let tcp_stream = TcpStream::connect_timeout(&addr, timeout)?;
            tcp_stream.set_nonblocking(true)?;

            tcp_stream.set_write_timeout(Some(timeout))?;
            tcp_stream.set_read_timeout(Some(timeout))?;

            return Ok(Self {
                tcp_stream,
                caps,
                state: ParserState::Data,
            });
        }
        Err(NetError::CouldNotConnect.into())
    }

    pub fn accept(tcp_stream: TcpStream) -> crate::Result<Self> {
        tcp_stream.set_nonblocking(true)?;

        Ok(Self {
            tcp_stream,
            caps: TermCaps {
                window_size: (0, 0),
                terminal: Terminal::Ansi,
            },
            state: ParserState::Data,
        })
    }

    fn parse(&mut self, data: &mut [u8]) -> io::Result<usize> {
        let mut write_ptr = 0;
        for i in 0..data.len() {
            let b = data[i];
            match self.state {
                ParserState::Data => {
                    if b == telnet_cmd::IAC {
                        self.state = ParserState::Iac;
                    } else {
                        data[write_ptr] = b;
                        write_ptr += 1;
                    }
                }

                ParserState::SubCommand(cmd) => {
                    match b {
                        telnet_cmd::IAC => {}
                        telnet_cmd::SE => {
                            self.state = ParserState::Data;
                        }
                        terminal_type::SEND => {
                            // Send
                            if cmd == telnet_option::TERMINAL_TYPE as i32 {
                                let mut buf: Vec<u8> = vec![telnet_cmd::IAC, telnet_cmd::SB, telnet_option::TERMINAL_TYPE, terminal_type::IS];

                                match self.caps.terminal {
                                    //  :TODO: Let's extend this to allow for some of the semi-standard BBS IDs, e.g. "xterm" (ANSI), "ansi-256-color", etc.
                                    Terminal::Ansi => buf.extend_from_slice(b"ANSI"),
                                    Terminal::PETscii => buf.extend_from_slice(b"PETSCII"),
                                    Terminal::ATAscii => buf.extend_from_slice(b"ATASCII"),
                                    Terminal::ViewData => buf.extend_from_slice(b"VIEWDATA"),
                                    Terminal::Ascii => buf.extend_from_slice(b"RAW"),
                                    Terminal::Avatar => buf.extend_from_slice(b"AVATAR"),
                                    Terminal::Rip => buf.extend_from_slice(b"RIP"),
                                    Terminal::IGS => buf.extend_from_slice(b"IGS"),
                                    Terminal::Mode7 => buf.extend_from_slice(b"MODE7"),
                                }
                                data[write_ptr] = telnet_cmd::IAC;
                                write_ptr += 1;
                                data[write_ptr] = telnet_cmd::SE;
                                write_ptr += 1;

                                self.tcp_stream.write_all(&buf)?;
                            }
                        }
                        24 => {
                            // Terminal type
                            self.state = ParserState::SubCommand(telnet_option::TERMINAL_TYPE as i32);
                        }
                        _ => {}
                    }
                }
                ParserState::Iac => match telnet_cmd::check(b) {
                    Ok(telnet_cmd::AYT) => {
                        self.state = ParserState::Data;
                        self.tcp_stream.write_all(&telnet_cmd::make_cmd(telnet_cmd::NOP))?;
                    }
                    Ok(telnet_cmd::SE | telnet_cmd::NOP | telnet_cmd::GA) => {
                        self.state = ParserState::Data;
                    }
                    Ok(telnet_cmd::IAC) => {
                        data[write_ptr] = 0xFF;
                        write_ptr += 1;
                        self.state = ParserState::Data;
                    }
                    Ok(telnet_cmd::WILL) => {
                        self.state = ParserState::Will;
                    }
                    Ok(telnet_cmd::WONT) => {
                        self.state = ParserState::Wont;
                    }
                    Ok(telnet_cmd::DO) => {
                        self.state = ParserState::Do;
                    }
                    Ok(telnet_cmd::DONT) => {
                        self.state = ParserState::Dont;
                    }
                    Ok(telnet_cmd::SB) => {
                        self.state = ParserState::SubCommand(-1);
                    }
                    Err(err) => {
                        log::error!("error parsing IAC: {}", err);
                        self.state = ParserState::Data;
                    }
                    Ok(cmd) => {
                        log::error!("unsupported IAC: {}", telnet_cmd::to_string(cmd));
                        self.state = ParserState::Data;
                    }
                },
                ParserState::Will => {
                    self.state = ParserState::Data;
                    match telnet_option::check(b)? {
                        telnet_option::TRANSMIT_BINARY => {
                            self.tcp_stream
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DO, telnet_option::TRANSMIT_BINARY))?;
                        }
                        telnet_option::ECHO => {
                            self.tcp_stream
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DO, telnet_option::ECHO))?;
                        }
                        telnet_option::SUPPRESS_GO_AHEAD => {
                            self.tcp_stream
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DO, telnet_option::SUPPRESS_GO_AHEAD))?;
                        }
                        opt => {
                            log::warn!("unsupported will option {}", telnet_option::to_string(opt));
                            self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DONT, opt))?;
                        }
                    }
                }
                ParserState::Wont => {
                    let opt = telnet_option::check(b)?;
                    log::info!("Wont {opt:?}");
                    self.state = ParserState::Data;
                }
                ParserState::Do => {
                    self.state = ParserState::Data;
                    let opt = telnet_option::check(b)?;
                    match opt {
                        telnet_option::TRANSMIT_BINARY => {
                            self.tcp_stream
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::WILL, telnet_option::TRANSMIT_BINARY))?;
                        }
                        telnet_option::TERMINAL_TYPE => {
                            self.tcp_stream
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::WILL, telnet_option::TERMINAL_TYPE))?;
                        }
                        telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE => {
                            // NAWS: send our current window size
                            let mut buf: Vec<u8> = telnet_cmd::make_cmd_with_option(telnet_cmd::SB, telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE).to_vec();
                            // Note: big endian bytes are correct.
                            buf.extend(self.caps.window_size.0.to_be_bytes());
                            buf.extend(self.caps.window_size.1.to_be_bytes());
                            buf.push(telnet_cmd::IAC);
                            buf.push(telnet_cmd::SE);
                            self.tcp_stream.write_all(&buf)?;
                        }
                        _ => {
                            log::warn!("unsupported do option {}", telnet_option::to_string(opt));
                            self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::WONT, opt))?;
                        }
                    }
                }
                ParserState::Dont => {
                    let opt = telnet_option::check(b)?;
                    log::info!("Dont {opt:?}");
                    self.state = ParserState::Data;
                }
            }
        }
        Ok(write_ptr)
    }
}

impl Connection for TelnetConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Telnet
    }

    fn shutdown(&mut self) -> crate::Result<()> {
        self.tcp_stream.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}

impl Read for TelnetConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.tcp_stream.read(buf) {
            Ok(size) => {
                if size == 0 {
                    return Ok(0);
                }
                let size = self.parse(&mut buf[..size])?;
                Ok(size)
            }
            Err(ref e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    return Ok(0);
                }
                Err(io::Error::new(ErrorKind::ConnectionAborted, format!("Connection aborted: {e}")))
            }
        }
    }
}

impl Write for TelnetConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut data = Vec::with_capacity(buf.len());
        for b in buf {
            if *b == telnet_cmd::IAC {
                data.extend_from_slice(&[telnet_cmd::IAC, telnet_cmd::IAC]);
            } else {
                data.push(*b);
            }
        }

        self.tcp_stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tcp_stream.flush()
    }
}
