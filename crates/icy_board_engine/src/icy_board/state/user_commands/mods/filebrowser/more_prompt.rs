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

            log::info!("input: {}", input);

            match input.as_str() {
                "F" => {
                    self.flag_files().await?;
                }
                "V" => {
                    // view: TODO
                    self.println(TerminalTarget::Both, "TODO").await?;
                }
                "S" => {
                    // show: TODO
                    self.println(TerminalTarget::Both, "TODO").await?;
                }
                "G" => {
                    self.goodbye_cmd().await?;
                }
                _ => {
                    if input.to_ascii_uppercase() == self.session.no_char.to_string() {
                        self.session.disp_options.abort_printout = true;
                    }
                    return Ok(());
                }
            }
        }
    }
}
