#![allow(dead_code)]

use std::{io, time::Duration};

use async_trait::async_trait;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
};

use super::{Connection, ConnectionType};

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
pub enum TerminalEmulation {
    #[default]
    Ansi,
    Avatar,
    Ascii,
    PETscii,
    ATAscii,
    ViewData,
    Mode7,
    Rip,
    Skypix,
    IGS,
}

#[derive(Debug, Clone)]
pub struct TermCaps {
    pub window_size: (u16, u16), // width, height
    pub terminal: TerminalEmulation,
}

pub struct TelnetConnection {
    tcp_stream: TcpStream,
    caps: TermCaps,
    state: ParserState,
}

impl TelnetConnection {
    pub async fn open<A: ToSocketAddrs>(addr: &A, caps: TermCaps, timeout: Duration) -> crate::Result<Self> {
        let result = tokio::time::timeout(timeout, TcpStream::connect(addr)).await;
        match result {
            Ok(tcp_stream) => match tcp_stream {
                Ok(tcp_stream) => Ok(Self {
                    tcp_stream,
                    caps,
                    state: ParserState::Data,
                }),
                Err(err) => Err(Box::new(err)),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    pub fn accept(tcp_stream: TcpStream) -> crate::Result<Self> {
        Ok(Self {
            tcp_stream,
            caps: TermCaps {
                window_size: (0, 0),
                terminal: TerminalEmulation::Ansi,
            },
            state: ParserState::Data,
        })
    }

    async fn parse(&mut self, data: &mut [u8]) -> io::Result<usize> {
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
                                let mut buf = vec![telnet_cmd::IAC, telnet_cmd::SB, telnet_option::TERMINAL_TYPE, terminal_type::IS];

                                match self.caps.terminal {
                                    //  :TODO: Let's extend this to allow for some of the semi-standard BBS IDs, e.g. "xterm" (ANSI), "ansi-256-color", etc.
                                    TerminalEmulation::Ansi => buf.extend_from_slice(b"ANSI"),
                                    TerminalEmulation::PETscii => buf.extend_from_slice(b"PETSCII"),
                                    TerminalEmulation::ATAscii => buf.extend_from_slice(b"ATASCII"),
                                    TerminalEmulation::ViewData => buf.extend_from_slice(b"VIEWDATA"),
                                    TerminalEmulation::Ascii => buf.extend_from_slice(b"RAW"),
                                    TerminalEmulation::Avatar => buf.extend_from_slice(b"AVATAR"),
                                    TerminalEmulation::Rip => buf.extend_from_slice(b"RIP"),
                                    TerminalEmulation::Skypix => buf.extend_from_slice(b"SKYPIX"),
                                    TerminalEmulation::IGS => buf.extend_from_slice(b"IGS"),
                                    TerminalEmulation::Mode7 => buf.extend_from_slice(b"MODE7"),
                                }
                                buf.extend([telnet_cmd::IAC, telnet_cmd::SE]);
                                self.tcp_stream.write_all(&buf).await?;
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
                        self.tcp_stream.write_all(&telnet_cmd::make_cmd(telnet_cmd::NOP)).await?;
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
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DO, telnet_option::TRANSMIT_BINARY))
                                .await?;
                        }
                        telnet_option::ECHO => {
                            self.tcp_stream
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DO, telnet_option::ECHO))
                                .await?;
                        }
                        telnet_option::SUPPRESS_GO_AHEAD => {
                            self.tcp_stream
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DO, telnet_option::SUPPRESS_GO_AHEAD))
                                .await?;
                        }
                        opt => {
                            log::warn!("unsupported will option {}", telnet_option::to_string(opt));
                            self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::DONT, opt)).await?;
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
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::WILL, telnet_option::TRANSMIT_BINARY))
                                .await?;
                        }
                        telnet_option::TERMINAL_TYPE => {
                            self.tcp_stream
                                .write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::WILL, telnet_option::TERMINAL_TYPE))
                                .await?;
                        }
                        telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE => {
                            // NAWS: send our current window size
                            let mut buf: Vec<u8> = telnet_cmd::make_cmd_with_option(telnet_cmd::SB, telnet_option::NEGOTIATE_ABOUT_WINDOW_SIZE).to_vec();
                            // Note: big endian bytes are correct.
                            buf.extend(self.caps.window_size.0.to_be_bytes());
                            buf.extend(self.caps.window_size.1.to_be_bytes());
                            buf.push(telnet_cmd::IAC);
                            buf.push(telnet_cmd::SE);
                            self.tcp_stream.write_all(&buf).await?;
                        }
                        _ => {
                            log::warn!("unsupported do option {}", telnet_option::to_string(opt));
                            self.tcp_stream.write_all(&telnet_cmd::make_cmd_with_option(telnet_cmd::WONT, opt)).await?;
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

#[async_trait]
impl Connection for TelnetConnection {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Telnet
    }

    async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        match self.tcp_stream.read(buf).await {
            Ok(size) => {
                let result = self.parse(&mut buf[0..size]).await?;
                /* if result == 0 {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "").into());
                }*/
                Ok(result)
            }
            Err(err) => {
                log::error!("telnet error reading from TCP stream: {}", err);
                if err.kind() == io::ErrorKind::WouldBlock {
                    return Ok(0);
                }
                return Err(err.into());
            }
        }
    }

    async fn send(&mut self, buf: &[u8]) -> crate::Result<()> {
        let mut dst = Vec::new();
        for b in buf {
            if *b == telnet_cmd::IAC {
                dst.extend_from_slice(&[telnet_cmd::IAC, telnet_cmd::IAC]);
            } else {
                dst.push(*b);
            }
        }
        self.tcp_stream.write_all(&dst).await?;
        Ok(())
    }
    async fn shutdown(&mut self) -> crate::Result<()> {
        self.tcp_stream.shutdown().await?;
        Ok(())
    }
}
