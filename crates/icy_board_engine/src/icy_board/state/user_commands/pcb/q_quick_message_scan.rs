use crate::{
    Res,
    icy_board::{
        commands::CommandType,
        state::{IcyBoardState, functions::MASK_COMMAND},
    },
};

use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            NodeStatus,
            functions::display_flags,
            user_commands::mods::messagereader::read_command::{self, ReadLoop},
        },
    },
    vm::TerminalTarget,
};
use async_recursion::async_recursion;
use bstr::{BString, ByteSlice};
use jamjam::jam::JamMessageBase;

impl IcyBoardState {
    #[async_recursion(?Send)]
    pub async fn quick_message_scan(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::HandlingMail).await;

        let Some(message_base_file) = self.message_area_path(self.session.current_message_area) else {
            self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;
            return Ok(());
        };
        match JamMessageBase::open(&message_base_file) {
            Ok(message_base) => {
                self.show_quick_scans(self.session.current_message_area, message_base).await?;
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {}", err);
                log::error!("Creating new message index at {}", message_base_file.display());
                self.display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                if JamMessageBase::create(message_base_file).is_ok() {
                    log::error!("successfully created new message index.");
                    return self.quick_message_scan().await;
                }
                log::error!("failed to create message index.");

                self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                Ok(())
            }
        }
    }

    async fn show_quick_scans(&mut self, area: usize, message_base: JamMessageBase) -> Res<()> {
        let prompt = if self.session.expert_mode() {
            IceText::MessageScanCommandExpertmode
        } else {
            IceText::MessageScanCommand
        };
        let low = message_base.base_messagenumber();
        let high = low + message_base.active_messages();
        self.session.op_text = format!("{}-{}", low, high);

        let text = self
            .input_field(
                prompt,
                40,
                MASK_COMMAND,
                CommandType::QuickMessageScan.get_help(),
                None,
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::LFBEFORE,
            )
            .await?;
        if text.is_empty() {
            return Ok(());
        }

        // Q shares R's command language, and a bare number scans forward from
        // there rather than showing that one message.
        let tokens: Vec<String> = text.split_whitespace().map(|t| t.to_ascii_uppercase()).collect();
        let mut ctx = self.read_parse_context(0).await;
        ctx.quick_scan = true;
        let mut cmd = read_command::parse(&tokens, ReadLoop::Outside, &ctx);
        read_command::finalize(&mut cmd);

        let Some(range) = cmd.numbers.first() else {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
            return Ok(());
        };
        let number = range.first.clamp(low as i64, high as i64) as u32;

        if number < 1 || number > message_base.active_messages() {
            self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
            return Ok(());
        }
        self.display_text(IceText::Scanning, display_flags::DEFAULT).await?;
        let conf = format!(
            "{}/{}",
            self.session.current_conference.name,
            self.session.current_conference.areas.as_ref().unwrap()[area as usize].name
        );
        self.println(TerminalTarget::Both, &conf).await?;

        self.display_text(IceText::QuickScanHeader, display_flags::NEWLINE).await?;

        self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        for i in number..message_base.active_messages() {
            match message_base.read_header(i) {
                Ok(header) => {
                    let status = if header.needs_password() {
                        if header.is_read() { '^' } else { '%' }
                    } else if header.is_private() {
                        if header.get_to().unwrap().eq_ignore_ascii_case(b"SYSOP") {
                            if header.is_read() { '~' } else { '`' }
                        } else {
                            if header.is_read() { '+' } else { '*' }
                        }
                    } else if header.is_read() {
                        ' '
                    } else {
                        '-'
                    };

                    self.println(
                        TerminalTarget::Both,
                        &format!(
                            "{}{:<7} {:<7} {:<15} {:<15} {:<25}",
                            status,
                            header.message_number,
                            if header.reply_to > 0 { header.reply_to.to_string() } else { "-".to_string() },
                            get_str(header.get_to(), 15),
                            get_str(header.get_from(), 15),
                            get_str(header.get_subject(), 25)
                        ),
                    )
                    .await?;
                }
                _ => continue,
            }
        }

        self.read_msgs_from_base(message_base, false).await
    }
}

fn get_str(s: Option<&BString>, len: usize) -> String {
    match s {
        Some(s) => {
            if s.len() > len {
                s[..len].to_str_lossy().to_string()
            } else {
                s.to_str_lossy().to_string()
            }
        }
        None => "".to_string(),
    }
}
