use std::path::PathBuf;

use async_recursion::async_recursion;
use humanize_bytes::humanize_bytes_decimal;

use crate::icy_board::icb_config::IcbColor;
use crate::icy_board::state::functions::MASK_NUM;
use crate::{icy_board::state::IcyBoardState, Res};
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn download(&mut self, ask_flagged_files: bool) -> Res<()> {
        if ask_flagged_files {
            if !self.session.flagged_files.is_empty() {
                let download_tagged = self
                    .input_field(
                        IceText::DownloadTagged,
                        1,
                        "",
                        &"",
                        Some(self.session.yes_char.to_string()),
                        display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE | display_flags::YESNO | display_flags::FIELDLEN,
                    )
                    .await?;

                if download_tagged == self.session.no_char.to_uppercase().to_string() {
                    return Ok(());
                }
            }

            self.flag_files_cmd(true).await?;

            if self.session.flagged_files.is_empty() {
                return Ok(());
            }
        } else {
            self.new_line().await?;
        }

        let mut protocol_str: String = self.session.current_user.as_ref().unwrap().protocol.clone();
        let mut protocol = None;
        let mut p_descr = "None".to_string();

        let mut goodbye_after_dl = false;
        let mut do_dl = true;
        loop {
            for p in self.get_board().await.protocols.iter() {
                if p.is_enabled && p.char_code == protocol_str {
                    p_descr = p.description.clone();
                    protocol = Some(p.send_command.clone());
                    break;
                }
            }

            let mut total_size = 0;
            for path in &self.session.flagged_files {
                if let Ok(data) = path.metadata() {
                    total_size += data.len();
                }
            }
            self.display_text(IceText::BatchDownloadSize, display_flags::DEFAULT).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;
            self.println(TerminalTarget::Both, &format!(" {}", humanize_bytes_decimal!(total_size).to_string()))
                .await?;

            self.display_text(IceText::BatchProtocol, display_flags::DEFAULT).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_cyan()).await?;
            self.println(TerminalTarget::Both, &p_descr).await?;
            self.display_text(IceText::ReadyToSendBatch, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;

            let input = self
                .input_field(
                    IceText::GoodbyeAfterDownload,
                    1,
                    &DL_LISTMASK,
                    &"",
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;

            match input.as_str() {
                "A" => {
                    do_dl = false;
                    break;
                }
                "E" => {
                    self.edit_dl_batch().await?;
                }
                "G" => {
                    goodbye_after_dl = true;
                    break;
                }
                "L" => {
                    self.list_dl_batch().await?;
                }
                "P" => {
                    let protocol = self.ask_protocols(&protocol_str).await?;

                    if !protocol.is_empty() {
                        protocol_str = protocol;
                    }
                }
                _ => {
                    break;
                }
            }
        }
        if do_dl {
            self.display_text(IceText::SendingFiles, display_flags::NEWLINE).await?;

            if let Some(protocol) = &protocol {
                let mut prot = protocol.create();
                let files: Vec<PathBuf> = self.session.flagged_files.drain(..).collect();
                for f in &files {
                    if !f.exists() {
                        log::error!("File not found: {:?}", f);
                        self.session.op_text = f.file_name().unwrap().to_string_lossy().to_string();
                        self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE).await?;
                        return Ok(());
                    }
                }
                match prot.initiate_send(&mut *self.connection, &files).await {
                    Ok(mut state) => {
                        while !state.is_finished {
                            if let Err(e) = prot.update_transfer(&mut *self.connection, &mut state).await {
                                log::error!("Error while updating file transfer with {:?} : {}", protocol, e);
                                self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                                break;
                            }
                        }
                        self.display_text(IceText::BatchTransferEnded, display_flags::LFBEFORE).await?;
                        self.transfer_statistics.downloaded_bytes = state.send_state.total_bytes_transfered as usize;
                        self.transfer_statistics.downloaded_files = state.send_state.finished_files.len();
                        self.transfer_statistics.downloaded_cps = state.send_state.get_bps() as usize;
                        self.display_text(IceText::BatchSend, display_flags::LFBEFORE).await?;

                        self.board.lock().await.statistics.add_download(&state);
                        self.board.lock().await.save_statistics()?;
                    }
                    Err(e) => {
                        log::error!("Error while initiating file transfer with {:?} : {}", protocol, e);
                        self.println(TerminalTarget::Both, &format!("Error: {}", e)).await?;
                    }
                }
            } else {
                self.println(TerminalTarget::Both, "Protocol not found.").await?;
            }

            if goodbye_after_dl {
                self.goodbye().await?;
            }
        }
        Ok(())
    }

    #[async_recursion(?Send)]
    async fn edit_dl_batch(&mut self) -> Res<()> {
        self.new_line().await?;

        loop {
            let input = self
                .input_field(
                    IceText::EditBatch,
                    1,
                    &DL_EDITMASK,
                    &"",
                    None,
                    display_flags::NEWLINE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?;

            match input.as_str() {
                "A" => {
                    self.new_line().await?;
                    self.flag_files_cmd(true).await?;
                }
                "R" => {
                    self.remove_dl_batch().await?;
                }
                "L" => {
                    self.list_dl_batch().await?;
                }
                _ => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn remove_dl_batch(&mut self) -> Res<()> {
        self.session.op_text = format!("1-{}", self.session.flagged_files.len());
        let input = self
            .input_field(
                IceText::RemoveFileNumber,
                16,
                &MASK_NUM,
                &"",
                None,
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::LFBEFORE,
            )
            .await?;
        self.session.push_tokens(&input);

        let mut remove = Vec::new();
        while let Some(token) = self.session.tokens.pop_front() {
            if let Ok(num) = token.parse::<usize>() {
                if num == 0 {
                    continue;
                }
                if let Some(path) = &self.session.flagged_files.get(num - 1) {
                    remove.push(num - 1);
                    self.session.op_text = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    self.display_text(IceText::RemovedFile, display_flags::NEWLINE).await?;
                }
            }
        }
        remove.sort_by(|a, b| b.cmp(a));
        for r in remove {
            self.session.flagged_files.remove(r);
        }
        self.new_line().await?;
        Ok(())
    }
    async fn list_dl_batch(&mut self) -> Res<()> {
        self.new_line().await?;
        for (i, path) in self.session.flagged_files.clone().iter().enumerate() {
            let size = if let Ok(data) = path.metadata() { data.len() } else { 0 };
            self.display_text(IceText::FileSelected, display_flags::DEFAULT).await?;
            self.set_color(TerminalTarget::Both, IcbColor::dos_light_green()).await?;

            let number = format!("({})", i + 1);
            self.print(TerminalTarget::Both, &format!("{number:<5}{:>8} ", humanize_bytes_decimal!(size).to_string()))
                .await?;
            self.println(TerminalTarget::Both, &format!("{}", path.file_name().unwrap_or_default().to_string_lossy()))
                .await?;
        }
        self.new_line().await?;

        Ok(())
    }
}

const DL_LISTMASK: &str = "AEGLP";
const DL_EDITMASK: &str = "ARL";
