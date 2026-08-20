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
        self.message_scan(false).await
    }

    /// Sysop command 5. `PCBoard` runs the quick scan with its header scan flag on,
    /// which adds the active/inactive column and lists killed messages too.
    #[async_recursion(?Send)]
    pub async fn header_message_scan(&mut self) -> Res<()> {
        self.message_scan(true).await
    }

    #[async_recursion(?Send)]
    async fn message_scan(&mut self, header_scan: bool) -> Res<()> {
        self.set_activity(NodeStatus::HandlingMail).await;

        let Some(message_base_file) = self.message_area_path(self.session.current_message_area) else {
            self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                .await?;
            return Ok(());
        };
        match JamMessageBase::open(&message_base_file) {
            Ok(message_base) => {
                self.show_quick_scans(self.session.current_message_area, message_base, header_scan).await?;
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {err}");
                log::error!("Creating new message index at {}", message_base_file.display());
                self.display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                if JamMessageBase::create(message_base_file).is_ok() {
                    log::error!("successfully created new message index.");
                    return self.message_scan(header_scan).await;
                }
                log::error!("failed to create message index.");

                self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                Ok(())
            }
        }
    }

    async fn show_quick_scans(&mut self, area: usize, message_base: JamMessageBase, header_scan: bool) -> Res<()> {
        let prompt = if self.session.expert_mode() {
            IceText::MessageScanCommandExpertmode
        } else {
            IceText::MessageScanCommand
        };
        let low = message_base.lowest_message_number();
        let high = message_base.highest_message_number();
        self.session.op_text = format!("{low}-{high}");

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
        let tokens: Vec<String> = text.split_whitespace().map(str::to_ascii_uppercase).collect();
        let mut ctx = self.read_parse_context(0).await;
        ctx.quick_scan = true;
        let mut cmd = read_command::parse(&tokens, ReadLoop::Outside, &ctx);
        read_command::finalize(&mut cmd);

        let Some(range) = cmd.numbers.first() else {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
            return Ok(());
        };
        let number = range.first.clamp(low as i64, high as i64) as u32;

        if number < low || high < low {
            self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
            return Ok(());
        }
        self.display_text(IceText::Scanning, display_flags::DEFAULT).await?;
        let conf = format!(
            "{}/{}",
            self.session.current_conference.name,
            self.session.current_conference.areas.as_ref().unwrap()[area].name
        );
        self.println(TerminalTarget::Both, &conf).await?;

        self.display_text(
            if header_scan { IceText::FiveScanHeader } else { IceText::QuickScanHeader },
            display_flags::NEWLINE,
        )
        .await?;

        self.set_color(TerminalTarget::Both, IcbColor::dos_light_cyan()).await?;
        for i in number..=high {
            if let Ok(header) = message_base.read_header(i) {
                // Only the header scan lists what has been killed.
                if header.is_deleted() && !header_scan {
                    continue;
                }
                let status = if header.needs_password() {
                    if header.is_read() { '^' } else { '%' }
                } else if header.is_private() {
                    if header.to().unwrap().eq_ignore_ascii_case(b"SYSOP") {
                        if header.is_read() { '~' } else { '`' }
                    } else {
                        if header.is_read() { '+' } else { '*' }
                    }
                } else if header.is_read() {
                    ' '
                } else {
                    '-'
                };
                let active = if !header_scan {
                    String::new()
                } else if header.is_deleted() {
                    "I".to_string()
                } else {
                    "A".to_string()
                };

                self.println(
                    TerminalTarget::Both,
                    &format!(
                        "{}{}{:<7} {:<7} {:<15} {:<15} {:<25}",
                        active,
                        status,
                        header.message_number,
                        if header.reply_to > 0 { header.reply_to.to_string() } else { "-".to_string() },
                        get_str(header.to(), 15),
                        get_str(header.from(), 15),
                        get_str(header.subject(), 25)
                    ),
                )
                .await?;
            }
        }

        if header_scan {
            // A header scan lists and stops; it does not walk into the messages.
            return Ok(());
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
        None => String::new(),
    }
}
