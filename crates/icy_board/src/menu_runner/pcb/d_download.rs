use std::{
    io::{self, Read, Write},
    path::PathBuf,
    thread,
    time::Duration,
};

use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::{
    icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};
use icy_net::Connection;

impl PcbBoardCommand {
    pub fn download(&mut self, _action: &Command) -> Res<()> {
        if self.state.session.flagged_files.is_empty() {
            self.state.println(TerminalTarget::Both, "No files flagged for download.")?;
            self.state.new_line()?;
            self.state.press_enter()?;
            self.display_menu = true;
        }

        let download_tagged = self.state.input_field(
            IceText::DownloadTagged,
            1,
            "",
            &"",
            Some(self.state.session.yes_char.to_string()),
            display_flags::NEWLINE | display_flags::UPCASE | display_flags::YESNO | display_flags::FIELDLEN,
        )?;

        if download_tagged == self.state.session.no_char.to_uppercase().to_string() {
            self.state.new_line()?;
            self.state.press_enter()?;
            self.display_menu = true;
            return Ok(());
        }

        let protocol_str: String = self.state.current_user.as_ref().unwrap().protocol.clone();
        let mut protocol = None;
        if let Ok(board) = self.state.board.lock() {
            for p in &board.protocols.protocols {
                if p.is_enabled && p.char_code == protocol_str {
                    protocol = Some(p.send_command.clone());
                    break;
                }
            }
        }

        if let Some(protocol) = protocol {
            let mut prot = protocol.create();
            let files: Vec<PathBuf> = self.state.session.flagged_files.drain().collect();
            for f in &files {
                if !f.exists() {
                    log::error!("File not found: {:?}", f);
                    self.state.session.op_text = f.file_name().unwrap().to_string_lossy().to_string();
                    self.state.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE)?;
                    self.state.new_line()?;
                    self.state.press_enter()?;
                    return Ok(());
                }
            }
            match prot.initiate_send(&mut *self.state.connection, &files) {
                Ok(mut state) => {
                    let mut c = BlockingConnection {
                        conn: &mut self.state.connection,
                    };
                    while !state.is_finished {
                        if let Err(e) = prot.update_transfer(&mut c, &mut state) {
                            log::error!("Error while updating file transfer with {:?} : {}", protocol, e);
                            self.state.display_text(IceText::TransferAborted, display_flags::NEWLINE)?;
                            break;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error while initiating file transfer with {:?} : {}", protocol, e);
                    self.state.println(TerminalTarget::Both, &format!("Error: {}", e))?;
                }
            }
        } else {
            self.state.println(TerminalTarget::Both, "Protocol not found.")?;
        }
        self.state.new_line()?;
        self.state.press_enter()?;
        Ok(())
    }
}

pub struct BlockingConnection<'a> {
    pub conn: &'a mut Box<dyn Connection>,
}

impl<'a> Connection for BlockingConnection<'a> {
    fn get_connection_type(&self) -> icy_net::ConnectionType {
        self.conn.get_connection_type()
    }

    fn shutdown(&mut self) -> icy_net::Result<()> {
        Ok(())
    }

    fn read_u8(&mut self) -> icy_net::Result<u8> {
        let mut buf = [0; 1];
        while let Err(_) = self.read_exact(&mut buf) {
            thread::sleep(Duration::from_millis(10));
        }
        Ok(buf[0])
    }
}

impl<'a> Read for BlockingConnection<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.conn.read(buf)
    }
}

impl<'a> Write for BlockingConnection<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.conn.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.conn.flush()
    }
}
