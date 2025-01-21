use std::path::PathBuf;

use crate::{icy_board::state::IcyBoardState, Res};
use crate::{
    icy_board::{icb_text::IceText, state::functions::display_flags},
    vm::TerminalTarget,
};

impl IcyBoardState {
    pub async fn download(&mut self) -> Res<()> {
        if self.session.flagged_files.is_empty() {
            self.println(TerminalTarget::Both, "No files flagged for download.").await?;
            self.new_line().await?;
            self.press_enter().await?;
            self.display_current_menu = true;
        }

        let download_tagged = self
            .input_field(
                IceText::DownloadTagged,
                1,
                "",
                &"",
                Some(self.session.yes_char.to_string()),
                display_flags::NEWLINE | display_flags::UPCASE | display_flags::YESNO | display_flags::FIELDLEN,
            )
            .await?;

        if download_tagged == self.session.no_char.to_uppercase().to_string() {
            self.new_line().await?;
            self.press_enter().await?;
            self.display_current_menu = true;
            return Ok(());
        }

        let protocol_str: String = self.session.current_user.as_ref().unwrap().protocol.clone();
        let mut protocol = None;
        for p in &self.get_board().await.protocols.protocols {
            if p.is_enabled && p.char_code == protocol_str {
                protocol = Some(p.send_command.clone());
                break;
            }
        }

        if let Some(protocol) = protocol {
            let mut prot = protocol.create();
            let files: Vec<PathBuf> = self.session.flagged_files.drain().collect();
            for f in &files {
                if !f.exists() {
                    log::error!("File not found: {:?}", f);
                    self.session.op_text = f.file_name().unwrap().to_string_lossy().to_string();
                    self.display_text(IceText::NotFoundOnDisk, display_flags::NEWLINE).await?;
                    self.new_line().await?;
                    self.press_enter().await?;
                    return Ok(());
                }
            }
            match prot.initiate_send(&mut *self.connection, &files).await {
                Ok(mut state) => {
                    /*     let mut c = BlockingConnection {
                        conn: &mut self.connection,
                    };*/
                    while !state.is_finished {
                        if let Err(e) = prot.update_transfer(&mut *self.connection, &mut state).await {
                            log::error!("Error while updating file transfer with {:?} : {}", protocol, e);
                            self.display_text(IceText::TransferAborted, display_flags::NEWLINE).await?;
                            break;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error while initiating file transfer with {:?} : {}", protocol, e);
                    self.println(TerminalTarget::Both, &format!("Error: {}", e)).await?;
                }
            }
        } else {
            self.println(TerminalTarget::Both, "Protocol not found.").await?;
        }
        self.new_line().await?;
        self.press_enter().await?;
        Ok(())
    }
}
