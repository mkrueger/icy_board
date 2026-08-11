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
use dizbase::file_base::metadata::{MetadataHeader, MetadataType};
use dizbase::file_base_scanner::scan_file;
use icy_net::protocol::{Protocol, TransferProtocolType, XYModemVariant, XYmodem, Zmodem};

impl IcyBoardState {
    /// An upload throws the flag list
    /// away, so PCBoard asks first - and this is the very first prompt of the
    /// U command, ahead of the file name.
    pub async fn proceed_with_upload(&mut self) -> Res<bool> {
        if self.session.flagged_files.is_empty() {
            return Ok(true);
        }
        self.display_text(IceText::FilesAreFlagged, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::BELL)
            .await?;
        let answer = self
            .input_field(
                IceText::ContinueUpload,
                1,
                "",
                "",
                Some(self.session.no_char.to_uppercase().to_string()),
                display_flags::YESNO | display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
            )
            .await?;
        if answer != self.session.yes_char.to_uppercase().to_string() {
            return Ok(false);
        }
        self.session.flagged_files.clear();
        Ok(true)
    }

    pub async fn upload_file(&mut self) -> Res<()> {
        if let Some(window) = self.event_window().await {
            if window.uploads_blocked(&chrono::Local::now()) {
                self.display_text(IceText::UploadsDisabled, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
            }
        }
        if !self.proceed_with_upload().await? {
            return Ok(());
        }
        self.set_activity(NodeStatus::Transfer).await;
        let upload_location = self.session.current_conference.pub_upload_location.clone();
        let upload_metadata = self.session.current_conference.pub_upload_metadata.clone();
        if !upload_location.exists() {
            self.display_text(
                IceText::NoDirectoriesAvailable,
                display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE,
            )
            .await?;
            return Ok(());
        }

        // A name stacked on the command line skips the prompt.
        let file_name = if let Some(token) = self.session.tokens.pop_front() {
            token
        } else {
            self.input_field(
                IceText::FileNameToUpload,
                60,
                &MASK_ASCII,
                CommandType::UploadFile.get_help(),
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?
        };

        if file_name.is_empty() {
            return Ok(());
        }

        // PCBoard settles the protocol before it offers the goodbye question, and
        // asks for one rather than starting a transfer the caller has none for.
        let mut protocol_str: String = self.session.current_user.as_ref().unwrap().protocol.clone();
        loop {
            if self.get_protocol(protocol_str.clone()).await.is_some() {
                break;
            }
            let answer = self.ask_transfer_protocol(&protocol_str).await?;
            if answer.is_empty() || answer.eq_ignore_ascii_case("N") {
                return Ok(());
            }
            protocol_str = answer;
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
                    protocol_str = self.session.current_user.as_ref().unwrap().protocol.clone();
                    continue;
                }
                "" => {
                    break;
                }
                _ => {}
            }
        }

        let protocol = self.get_protocol(protocol_str).await;

        if let Some(protocol) = protocol {
            let Some(mut prot) = create_protocol(&protocol) else {
                self.display_text(IceText::TransferAborted, display_flags::NEWLINE | display_flags::LFBEFORE)
                    .await?;
                return Ok(());
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

                        let file_base = self.get_filebase(&upload_location, &upload_metadata).await?;
                        let mut metadata = scan_file(&dest)?;
                        metadata.push(MetadataHeader {
                            data: self.session.get_username_or_alias().as_bytes().to_vec(),
                            metadata_type: MetadataType::Uploader,
                        });
                        file_base.lock().await.add_file(&dest, metadata.clone())?;

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

    pub async fn get_protocol(&mut self, protocol_str: String) -> Option<TransferProtocolType> {
        let mut protocol = None;
        for p in self.get_board().await.protocols.iter() {
            if p.is_enabled && p.char_code == protocol_str {
                protocol = Some(p.send_command.clone());
                break;
            }
        }
        protocol
    }
}

pub fn create_protocol(protocol: &TransferProtocolType) -> Option<Box<dyn Protocol>> {
    match protocol {
        // No native handler runs a DOS external protocol, and ASCII/None have no
        // framing to drive, so the caller aborts rather than claim a transfer.
        TransferProtocolType::None | TransferProtocolType::ASCII | TransferProtocolType::External(_) => None,
        TransferProtocolType::XModem => Some(Box::new(XYmodem::new(XYModemVariant::XModem))),
        TransferProtocolType::XModemCRC => Some(Box::new(XYmodem::new(XYModemVariant::XModemCRC))),
        TransferProtocolType::XModem1k => Some(Box::new(XYmodem::new(XYModemVariant::XModem1k))),
        TransferProtocolType::XModem1kG => Some(Box::new(XYmodem::new(XYModemVariant::XModem1kG))),
        TransferProtocolType::YModem => Some(Box::new(XYmodem::new(XYModemVariant::YModem))),
        TransferProtocolType::YModemG => Some(Box::new(XYmodem::new(XYModemVariant::YModem))),
        TransferProtocolType::ZModem => Some(Box::new(Zmodem::new(1024))),
        TransferProtocolType::ZModem8k => Some(Box::new(Zmodem::new(8 * 1024))),
    }
}
