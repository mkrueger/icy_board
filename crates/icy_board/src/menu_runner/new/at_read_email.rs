use std::fs;

use icy_board_engine::icy_board::{commands::Command, icb_text::IceText, state::functions::display_flags, user_base::UserBase, IcyBoardError};
use jamjam::jam::JamMessageBase;

use crate::{menu_runner::PcbBoardCommand, Res};

impl PcbBoardCommand {
    pub fn read_email(&mut self, action: &Command) -> Res<()> {
        let name = self.state.session.user_name.to_string();
        let msg_base = self.get_email_msgbase(&name)?;
        self.read_msgs_from_base(msg_base, action)?;
        Ok(())
    }

    pub fn get_email_msgbase(&mut self, user_name: &str) -> Res<JamMessageBase> {
        let home_dir = if let Ok(board) = self.state.board.lock() {
            let name = if user_name == self.state.session.sysop_name {
                &board.users[0].get_name()
            } else {
                user_name
            };
            let home_dir = UserBase::get_user_home_dir(&self.state.resolve_path(&board.config.paths.home_dir), name);
            home_dir
        } else {
            return Err(IcyBoardError::ErrorLockingBoard.into());
        };

        if !home_dir.exists() {
            log::error!("Homedir for user {} does not exist", user_name);
            self.state.display_text(IceText::MessageBaseError, display_flags::NEWLINE)?;
            return Err(IcyBoardError::HomeDirMissing(user_name.to_string()).into());
        }

        let msg_dir = home_dir.join("msg");
        fs::create_dir_all(&msg_dir)?;
        let msg_base = msg_dir.join("email");
        Ok(if msg_base.with_extension("jhr").exists() {
            JamMessageBase::open(msg_base)?
        } else {
            log::info!("Creating new email message base for user {}", user_name);
            JamMessageBase::create(msg_base)?
        })
    }
}
