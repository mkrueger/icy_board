use crate::{
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{functions::display_flags, UserActivity},
    },
    vm::TerminalTarget,
};

use crate::{icy_board::state::IcyBoardState, Res};

impl IcyBoardState {
    pub async fn who_display_nodes(&mut self) -> Res<()> {
        if self.displaycmdfile("who").await? {
            return Ok(());
        }

        self.display_text(IceText::UserNetHeader, display_flags::NEWLINE).await?;
        self.display_text(IceText::UsernetUnderline, display_flags::NEWLINE).await?;
        let mut lines = Vec::new();
        for (i, connection) in self.node_state.lock().await.iter().enumerate() {
            if let Some(connection) = connection {
                if let Some(name) = self.get_board().await.users.get(connection.cur_user as usize) {
                    let name = name.get_name().to_string();
                    let text = match connection.user_activity {
                        UserActivity::LoggingIn => IceText::LogIntoSystem,
                        UserActivity::BrowseMenu => {
                            if connection.enabled_chat {
                                IceText::Available
                            } else {
                                IceText::Unavailable
                            }
                        }
                        UserActivity::EnterMessage => IceText::EnterMessage,

                        UserActivity::BrowseFiles => IceText::Transfer,
                        UserActivity::UploadFiles => IceText::Transfer,
                        UserActivity::DownloadFiles => IceText::Transfer,

                        UserActivity::ReadMessages => IceText::HandlingMail,
                        UserActivity::TakeSurvey => IceText::AnswerSurvey,

                        UserActivity::ReadBulletins => IceText::HandlingMail,
                        UserActivity::CommentToSysop => IceText::EnterMessage,

                        UserActivity::RunningDoor => IceText::InADOOR,
                        UserActivity::ChatWithSysop => IceText::ChatWithSysop,
                        UserActivity::GroupChat => IceText::GroupChat,
                        UserActivity::PagingSysop => IceText::PagingSysop,
                        UserActivity::Goodbye => IceText::LogoffPending,
                        UserActivity::ReadBroadcast => IceText::ReceivedMessage,
                    };

                    let txt = self.display_text.get_display_text(text).unwrap();
                    lines.push(format!("{:>4}   {:23} {}", i + 1, txt.text, name));
                }
            }
        }
        self.set_color(TerminalTarget::Both, IcbColor::Dos(11)).await?;
        self.println(TerminalTarget::Both, &lines.join("\r\n")).await?;
        self.new_line().await?;
        Ok(())
    }
}
