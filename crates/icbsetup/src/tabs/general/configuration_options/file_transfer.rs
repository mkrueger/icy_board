use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::IcyBoard;
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    get_text,
    tab_page::Page,
    theme::get_tui_theme,
    BORDER_SET,
};
use ratatui::{
    layout::{Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{cfg_entry_bool, cfg_entry_u32};

pub struct FileTransfers {
    pub state: ConfigMenuState,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl FileTransfers {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let menu = {
            let lock = icy_board.lock().unwrap();
            let label_with = 50;
            let entry = vec![
                cfg_entry_bool!("disallow_batch_uploads", label_with, file_transfer, disallow_batch_uploads, lock),
                cfg_entry_bool!("promote_to_batch_transfers", label_with, file_transfer, promote_to_batch_transfers, lock),
                cfg_entry_u32!("upload_credit_time", label_with, 0, 10000, file_transfer, upload_credit_time, lock),
                cfg_entry_u32!("upload_credit_bytes", label_with, 0, 10000, file_transfer, upload_credit_bytes, lock),
                cfg_entry_bool!("verify_files_uploaded", label_with, file_transfer, verify_files_uploaded, lock),
                cfg_entry_bool!("disable_drive_size_check", label_with, file_transfer, disable_drive_size_check, lock),
                cfg_entry_u32!(
                    "stop_uploads_free_space",
                    label_with,
                    0,
                    1024 * 1024,
                    file_transfer,
                    stop_uploads_free_space,
                    lock
                ),
            ];
            ConfigMenu { obj: icy_board.clone(), entry }
        };

        Self {
            state: ConfigMenuState::default(),
            menu,
        }
    }
}

impl Page for FileTransfers {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(1),
        };

        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .border_style(get_tui_theme().content_box);
        block.render(area, frame.buffer_mut());

        let val = get_text("configuration_options_file_transfer");
        let width = val.len() as u16;
        Line::raw(val).style(get_tui_theme().menu_title).render(
            Rect {
                x: area.x + 1 + area.width.saturating_sub(width) / 2,
                y: area.y + 1,
                width,
                height: 1,
            },
            frame.buffer_mut(),
        );

        frame.buffer_mut().set_string(
            area.x + 1,
            area.y + 2,
            "─".repeat((area.width as usize).saturating_sub(2)),
            get_tui_theme().content_box,
        );

        let area = area.inner(Margin { vertical: 4, horizontal: 1 });
        self.menu.render(area, frame, &mut self.state);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: icy_board_tui::config_menu::EditMode::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }
}
