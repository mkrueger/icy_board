use std::sync::{Arc, Mutex};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::IcyBoard;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{
    BORDER_SET,
    config_menu::{ConfigMenu, ConfigMenuState, EditMessage, ListValue, ResultState},
    get_text,
    tab_page::{InfoState, PageMessage},
    theme::get_tui_theme,
};

pub struct ICBConfigMenuUI {
    state: ConfigMenuState,
    title: String,
    menu: ConfigMenu<Arc<Mutex<IcyBoard>>>,
}

impl ICBConfigMenuUI {
    pub fn new(title: String, menu: ConfigMenu<Arc<Mutex<IcyBoard>>>) -> Self {
        Self {
            state: ConfigMenuState::default(),
            title,
            menu,
        }
    }

    pub fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        let area = Rect {
            x: disp_area.x + 1,
            y: disp_area.y + 1,
            width: disp_area.width.saturating_sub(2),
            height: disp_area.height.saturating_sub(1),
        };
        let mut bottom_text = get_text("icb_setup_key_menu_help");
        if let Some(item) = self.menu.get_item(self.state.selected) {
            if let ListValue::Path(path) = &item.value {
                let path = self.menu.obj.lock().unwrap().resolve_file(path);
                if !path.as_os_str().is_empty() && !path.is_dir() && item.editable() {
                    bottom_text = get_text("icb_setup_key_menu_edit_help");
                }
            }
        }

        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_bottom(Span::styled(bottom_text, get_tui_theme().key_binding))
            .border_style(get_tui_theme().menu_box);
        block.render(area, frame.buffer_mut());

        let width = self.title.len() as u16;
        Line::raw(&self.title).style(get_tui_theme().menu_title).render(
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
            get_tui_theme().menu_box,
        );

        let area = Rect {
            x: disp_area.x + 3,
            y: area.y + 3,
            width: disp_area.width - 3,
            height: area.height - 4,
        };
        self.menu.render(area, frame, &mut self.state);
    }

    pub fn request_status(&self) -> ResultState {
        ResultState {
            edit_msg: EditMessage::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    pub fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(item) = self.menu.get_item(self.state.selected) {
            if let ListValue::Path(path) = &item.value {
                if key.code == crossterm::event::KeyCode::F(2) && item.editable() {
                    let path = self.menu.obj.lock().unwrap().resolve_file(path);
                    if path.as_os_str().is_empty() {
                        return PageMessage::InfoBox(InfoState::Warning, get_text("no_file_name_given"));
                    }
                    if let Some(editor) = &item.path_editor {
                        return editor(self.menu.obj.clone(), path);
                    }
                    // A file that is only named in the configuration is created on demand.
                    if !path.exists() {
                        if let Err(e) = create_empty_file(&path) {
                            log::error!("Error creating {}: {}", path.display(), e);
                            return PageMessage::InfoBox(InfoState::Error, format!("{}\n\n{}", path.display(), e));
                        }
                    }

                    let editor: &String = &self.menu.obj.lock().unwrap().config.sysop.external_editor;
                    match std::process::Command::new(editor).arg(format!("{}", path.display())).spawn() {
                        Ok(mut child) => match child.wait() {
                            Ok(_) => {
                                return PageMessage::ExternalProgramStarted;
                            }
                            Err(e) => {
                                log::error!("Error opening editor: {}", e);
                                return PageMessage::InfoBox(InfoState::Error, format!("{}\n\n{}", editor, e));
                            }
                        },
                        Err(e) => {
                            log::error!("Error opening editor: {}", e);
                            ratatui::init();
                            return PageMessage::InfoBox(InfoState::Error, format!("{}\n\n{}", editor, e));
                        }
                    }
                }
            }
        }

        let res = self.menu.handle_key_press(key, &mut self.state);
        if res.edit_msg == EditMessage::Close {
            return PageMessage::Close;
        }
        PageMessage::ResultState(res)
    }
}

fn create_empty_file(path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::File::create(path)?;
    Ok(())
}
