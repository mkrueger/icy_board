use crate::{menu_runner::PcbBoardCommand, Res};
use icy_board_engine::icy_board::{
    commands::Command,
    doors::Door,
    icb_text::IceText,
    state::{
        functions::{display_flags, MASK_ALNUM},
        UserActivity,
    },
};

impl PcbBoardCommand {
    pub fn open_door(&mut self, action: &Command) -> Res<()> {
        self.state.set_activity(UserActivity::RunningDoor);
        let doors = self.state.session.current_conference.doors.clone();
        if doors.is_empty() {
            self.state.display_text(
                IceText::NoDOORSAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER | display_flags::BELL,
            )?;
            return Ok(());
        }
        let display_menu = self.state.session.tokens.is_empty();
        if display_menu {
            let file = self.state.session.current_conference.doors_menu.clone();
            self.state.display_menu(&file)?;
        }
        let text = if let Some(token) = self.state.session.tokens.pop_front() {
            token
        } else {
            self.state.input_field(
                if self.state.session.expert_mode {
                    IceText::DOORNumberCommandExpertmode
                } else {
                    IceText::DOORNumber
                },
                20,
                &MASK_ALNUM,
                &action.help,
                None,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE,
            )?
        };
        let mut found_door = false;

        if let Ok(number) = text.parse::<usize>() {
            if number > 0 {
                if let Some(b) = doors.get(number - 1) {
                    found_door = true;
                    self.run_door(b)?;
                }
            }
        } else {
            for d in &doors.doors {
                if d.name.to_uppercase().starts_with(&text.to_uppercase()) {
                    found_door = true;
                    self.run_door(d)?;
                }
            }
        }

        if !found_door {
            self.state
                .display_text(IceText::InvalidDOOR, display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER)?;
        }

        self.state.press_enter()?;
        self.display_menu = true;
        Ok(())
    }

    pub fn run_door(&mut self, b: &Door) -> Res<()> {
        if !b.securiy_level.user_can_access(&self.state.session) {
            self.state.display_text(
                IceText::DOORNotAvailable,
                display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::LFAFTER,
            )?;
            return Ok(());
        }

        // TODO: run door!!!!
        Ok(())
    }
}
