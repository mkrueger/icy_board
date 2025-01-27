use crate::{
    icy_board::{
        icb_text::IceText,
        state::{
            functions::{self, display_flags},
            IcyBoardState,
        },
    },
    vm::TerminalTarget,
    Res,
};

impl IcyBoardState {
    pub async fn filebase_more(&mut self) -> Res<()> {
        loop {
            let input = self
                .input_field(
                    IceText::FilesMorePrompt,
                    40,
                    functions::MASK_COMMAND,
                    &self.session.disp_options.file_list_help.clone(),
                    None,
                    display_flags::UPCASE | display_flags::STACKED | display_flags::ERASELINE,
                )
                .await?;
            self.session.more_requested = false;
            self.session.num_lines_printed = 0;
            self.session.push_tokens(&input);
            match self.session.tokens.pop_front().unwrap_or_default().to_ascii_uppercase().as_str() {
                "F" | "FL" | "FLA" | "FLAG" => {
                    self.flag_files().await?;
                }
                "V" => {
                    self.view_file().await?;
                }
                "S" => {
                    // show: TODO
                    self.println(TerminalTarget::Both, "TODO").await?;
                }
                "G" => {
                    self.goodbye_cmd().await?;
                }
                "NS" => {
                    self.session.non_stop_on();
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
