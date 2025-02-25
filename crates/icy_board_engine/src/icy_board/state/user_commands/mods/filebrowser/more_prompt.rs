use crate::{
    Res,
    icy_board::{
        icb_text::IceText,
        state::{
            IcyBoardState,
            functions::{self, display_flags},
        },
    },
};

impl IcyBoardState {
    pub async fn filebase_more(&mut self) -> Res<()> {
        log::info!("filebase_more");
        loop {
            let input = self
                .input_field(
                    IceText::FilesMorePrompt,
                    40,
                    functions::MASK_COMMAND,
                    "HLPXFRMORE",
                    None,
                    display_flags::UPCASE | display_flags::STACKED | display_flags::ERASELINE,
                )
                .await?;
            self.session.more_requested = false;
            self.session.disp_options.no_change();
            self.session.push_tokens(&input);
            match self.session.tokens.pop_front().unwrap_or_default().to_ascii_uppercase().as_str() {
                "F" | "FL" | "FLA" | "FLAG" => {
                    self.session.disp_options.no_change();
                    self.flag_files_cmd(false).await?;
                }
                "V" | "S" => {
                    self.view_file().await?;
                }

                "G" => {
                    self.goodbye_cmd().await?;
                    return Ok(());
                }
                "NS" => {
                    self.session.disp_options.force_non_stop();
                    return Ok(());
                }
                "N" => {
                    self.session.disp_options.abort_printout = true;
                    return Ok(());
                }
                "Y" | "" => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}
