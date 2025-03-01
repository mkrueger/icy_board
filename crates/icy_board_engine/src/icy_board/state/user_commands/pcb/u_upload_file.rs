use crate::icy_board::commands::CommandType;
use crate::{Res, icy_board::state::IcyBoardState};
use crate::{
    icy_board::{
        icb_text::IceText,
        state::{
            NodeStatus,
            functions::{MASK_ASCII, display_flags},
        },
    },
    vm::TerminalTarget,
};
use icy_net::protocol::{Protocol, TransferProtocolType, XYModemVariant, XYmodem, Zmodem};

impl IcyBoardState {
    pub async fn upload_file(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::Transfer).await;
        let upload_location = self.session.current_conference.pub_upload_location.clone();
        if !upload_location.exists() {
            self.display_text(
                IceText::NoDirectoriesAvailable,
                display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE,
            )
            .await?;
            return Ok(());
        }

        let file_name = self
            .input_field(
                IceText::FileNameToUpload,
                60,
                &MASK_ASCII,
                CommandType::UploadFile.get_help(),
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;

        if file_name.is_empty() {
            return Ok(());
        }
        let mut goodbye_after_upload = false;

        loop {
            let input = self
                .input_field(
                    IceText::GoodbyeAfterUpload,
                    1,
                    &"AGP",
                    "",
                    None,
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;

            match input.as_str() {
                "A" => {
                    return Ok(());
                }
                "G" => {
                    goodbye_after_upload = true;
                    break;
                }
                "P" => {
                    self.set_transfer_protocol().await?;
                    continue;
                }
                "" => {
                    break;
                }
                _ => {}
            }
        }

        let protocol_str: String = self.session.current_user.as_ref().unwrap().protocol.clone();
        let mut protocol = None;
        for p in self.get_board().await.protocols.iter() {
            if p.is_enabled && p.char_code == protocol_str {
                protocol = Some(p.send_command.clone());
                break;
            }
        }

        if let Some(protocol) = protocol {
            let mut prot: Box<dyn Protocol> = match protocol {
                TransferProtocolType::None => todo!(),
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

            match prot.initiate_recv(&mut *self.connection).await {
                Ok(mut state) => {
                    while !state.is_finished {
                        if let Err(e) = prot.update_transfer(&mut *self.connection, &mut state).await {
                            log::error!("Error while updating file transfer with {:?} : {}", protocol, e);
                            self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                            break;
                        }
                    }
                    self.display_text(IceText::TransferSuccessful, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;
                    self.display_text(IceText::ThanksForTheFiles, display_flags::NEWLINE | display_flags::LFBEFORE)
                        .await?;

                    for (x, path) in state.recieve_state.finished_files {
                        let dest = upload_location.join(x);
                        std::fs::copy(&path, &dest)?;
                        // todo: scan
                        std::fs::remove_file(&path)?;
                    }
                }
                Err(e) => {
                    log::error!("Error while initiating file transfer with {:?} : {}", protocol, e);
                    self.println(TerminalTarget::Both, &format!("Error: {}", e)).await?;
                }
            }
        }
        if goodbye_after_upload {
            self.goodbye().await?;
        }
        Ok(())
    }
}
