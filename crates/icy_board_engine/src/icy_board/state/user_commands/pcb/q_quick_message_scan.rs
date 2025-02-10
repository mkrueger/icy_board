use crate::{
    icy_board::{
        commands::CommandType,
        state::{functions::MASK_COMMAND, IcyBoardState},
    },
    Res,
};

use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{functions::display_flags, NodeStatus},
    },
    vm::TerminalTarget,
};
use async_recursion::async_recursion;
use bstr::{BString, ByteSlice};
use jamjam::jam::JamMessageBase;

impl IcyBoardState {
    pub async fn quick_message_scan(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::HandlingMail).await;

        let Ok(Some(area)) = self.show_message_areas(self.session.current_conference_number).await else {
            self.press_enter().await?;
            self.display_current_menu = true;
            return Ok(());
        };
        self.quick_message_scan_in_area(area).await
    }

    #[async_recursion(?Send)]
    async fn quick_message_scan_in_area(&mut self, area: usize) -> Res<()> {
        let message_base_file = &self.session.current_conference.areas[area].filename;
        let msgbase_file_resolved = self.get_board().await.resolve_file(message_base_file);
        match JamMessageBase::open(&msgbase_file_resolved) {
            Ok(message_base) => {
                self.show_quick_scans(area, message_base).await?;
                Ok(())
            }
            Err(err) => {
                log::error!("Message index load error {}", err);
                log::error!("Creating new message index at {}", msgbase_file_resolved.display());
                self.display_text(IceText::CreatingNewMessageIndex, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;
                if JamMessageBase::create(msgbase_file_resolved).is_ok() {
                    log::error!("successfully created new message index.");
                    return self.quick_message_scan_in_area(area).await;
                }
                log::error!("failed to create message index.");

                self.display_text(IceText::PathErrorInSystemConfiguration, display_flags::NEWLINE | display_flags::LFAFTER)
                    .await?;

                self.press_enter().await?;
                self.display_current_menu = true;
                Ok(())
            }
        }
    }

    async fn show_quick_scans(&mut self, area: usize, message_base: JamMessageBase) -> Res<()> {
        let prompt = if self.session.expert_mode {
            IceText::MessageScanCommandExpertmode
        } else {
            IceText::MessageScanCommand
        };
        self.session.op_text = format!("{}-{}", message_base.base_messagenumber(), message_base.active_messages());

        let text = self
            .input_field(
                prompt,
                40,
                MASK_COMMAND,
                CommandType::QuickMessageScan.get_help(),
                None,
                display_flags::UPCASE | display_flags::NEWLINE | display_flags::NEWLINE,
            )
            .await?;
        if text.is_empty() {
            return Ok(());
        }

        let number = if let Ok(n) = text.parse::<u32>() {
            n
        } else {
            self.display_text(IceText::InvalidEntry, display_flags::NEWLINE).await?;
            return Ok(());
        };

        if number < 1 || number > message_base.active_messages() {
            self.display_text(IceText::NoMailFound, display_flags::NEWLINE).await?;
            return Ok(());
        }
        self.display_text(IceText::Scanning, display_flags::DEFAULT).await?;
        let conf = format!(
            "{}/{}",
            self.session.current_conference.name, self.session.current_conference.areas[area as usize].name
        );
        self.println(TerminalTarget::Both, &conf).await?;

        self.display_text(IceText::QuickScanHeader, display_flags::NEWLINE).await?;

        self.set_color(TerminalTarget::Both, IcbColor::dos_cyan()).await?;
        for i in number..message_base.active_messages() {
            match message_base.read_header(i) {
                Ok(header) => {
                    let status = if header.needs_password() {
                        if header.is_read() {
                            '^'
                        } else {
                            '%'
                        }
                    } else if header.is_private() {
                        if header.get_to().unwrap().eq_ignore_ascii_case(b"SYSOP") {
                            if header.is_read() {
                                '~'
                            } else {
                                '`'
                            }
                        } else {
                            if header.is_read() {
                                '+'
                            } else {
                                '*'
                            }
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
