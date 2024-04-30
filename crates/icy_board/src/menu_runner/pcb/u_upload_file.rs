use crate::{menu_runner::PcbBoardCommand, Res};
use chrono::Utc;
use dizbase::file_base::{file_info::FileInfo, FileBase};
use icy_board_engine::{
    icy_board::{
        commands::Command,
        icb_text::IceText,
        state::{
            functions::{display_flags, MASK_ASCII},
            UserActivity,
        },
    },
    vm::TerminalTarget,
};
use icy_net::protocol::{Protocol, TransferProtocolType, XYModemVariant, XYmodem, Zmodem};

impl PcbBoardCommand {
    pub async fn upload_file(&mut self, action: &Command) -> Res<()> {
        self.state.set_activity(UserActivity::UploadFiles);
        let upload_location = self.state.resolve_path(&self.state.session.current_conference.pub_upload_location);
        if !upload_location.exists() {
            self.state
                .display_text(
                    IceText::NoDirectoriesAvailable,
                    display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE,
                )
                .await?;
            self.state.new_line().await?;
            self.state.press_enter().await?;
            return Ok(());
        }

        let file_name = self
            .state
            .input_field(
                IceText::FileNameToUpload,
                60,
                &MASK_ASCII,
                &action.help,
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;

        if file_name.is_empty() {
            self.state.new_line().await?;
            self.state.press_enter().await?;
            return Ok(());
        }
        let mut goodbye_after_upload = false;

        loop {
            let input = self
                .state
                .input_field(
                    IceText::GoodbyeAfterUpload,
                    1,
                    &"AGP",
                    &action.help,
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;

            match input.as_str() {
                "A" => {
                    self.state.new_line().await?;
                    self.state.press_enter().await?;
                    return Ok(());
                }
                "G" => {
                    goodbye_after_upload = true;
                    break;
                }
                "P" => {
                    self.set_transfer_protocol(action).await?;
                    continue;
                }
                "" => {
                    break;
                }
                _ => {}
            }
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
            let mut prot: Box<dyn Protocol> = match protocol {
                TransferProtocolType::ASCII => todo!(),
                TransferProtocolType::XModem => Box::new(XYmodem::new(XYModemVariant::XModem)),
                TransferProtocolType::XModemCRC => Box::new(XYmodem::new(XYModemVariant::XModemCRC)),
                TransferProtocolType::XModem1k => Box::new(XYmodem::new(XYModemVariant::XModem1k)),
                TransferProtocolType::XModem1kG => Box::new(XYmodem::new(XYModemVariant::XModem1kG)),
                TransferProtocolType::YModem => Box::new(XYmodem::new(XYModemVariant::YModem)),
                TransferProtocolType::YModemG => Box::new(XYmodem::new(XYModemVariant::YModemG)),
                TransferProtocolType::ZModem => Box::new(Zmodem::new(1024)),
                TransferProtocolType::ZModem8k => Box::new(Zmodem::new(8 * 1024)),
                TransferProtocolType::External(_) => todo!(),
            };

            match prot.initiate_recv(&mut *self.state.connection).await {
                Ok(mut state) => {
                    while !state.is_finished {
                        if let Err(e) = prot.update_transfer(&mut *self.state.connection, &mut state).await {
                            log::error!("Error while updating file transfer with {:?} : {}", protocol, e);
                            self.state.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                            break;
                        }
                    }
                    self.state
                        .display_text(IceText::TransferSuccessful, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    self.state
                        .display_text(IceText::ThanksForTheFiles, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;

                    let mut file_base = match FileBase::open(&self.state.session.current_conference.pub_upload_dir_file) {
                        Ok(file_base) => file_base,
                        Err(err) => {
                            log::error!("Error while opening file base: {}", err);
                            FileBase::create(&self.state.session.current_conference.pub_upload_dir_file)?
                        }
                    };

                    file_base.load_headers()?;
                    for (x, path) in state.recieve_state.finished_files {
                        let dest = upload_location.join(x);
                        std::fs::copy(&path, &dest)?;

                        // todo: scan
                        let info = FileInfo::new(&dest)
                            .with_uploader(self.state.session.get_username_or_alias())
                            .with_date(Utc::now().timestamp() as u64);

                        file_base.write_info(&info)?;

                        std::fs::remove_file(&path)?;
                    }
                }
                Err(e) => {
                    log::error!("Error while initiating file transfer with {:?} : {}", protocol, e);
                    self.state.println(TerminalTarget::Both, &format!("Error: {}", e)).await?;
                }
            }
        }
        if goodbye_after_upload {
            self.state.goodbye().await?;
        }
        self.state.new_line().await?;
        self.state.press_enter().await?;
        Ok(())
    }
}
