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
        if let Some(item) = self.menu.get_item(self.state.selected)
            && let ListValue::Path(path) = &item.value
        {
            let path = self.menu.obj.lock().unwrap().resolve_file(path);
            if !path.as_os_str().is_empty() && !path.is_dir() && item.editable() {
                bottom_text = if path.is_file() {
                    get_text("icb_setup_key_menu_edit_help")
                } else if can_create_file(&path) {
                    get_text("icb_setup_key_menu_create_help")
                } else {
                    bottom_text
                };
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
        if let Some(item) = self.menu.get_item(self.state.selected)
            && let ListValue::Path(path) = &item.value
            && item.editable()
            && matches!(key.code, crossterm::event::KeyCode::F(2) | crossterm::event::KeyCode::F(3))
        {
            let path = self.menu.obj.lock().unwrap().resolve_file(path);
            if path.as_os_str().is_empty() {
                return PageMessage::InfoBox(InfoState::Warning, get_text("no_file_name_given"));
            }
            if key.code == crossterm::event::KeyCode::F(3) && !path.exists() && can_create_file(&path) {
                return match create_empty_file(&path) {
                    Ok(()) => PageMessage::ResultState(self.request_status()),
                    Err(e) => {
                        log::error!("Error creating {}: {}", path.display(), e);
                        PageMessage::InfoBox(InfoState::Error, format!("{}\n\n{}", path.display(), e))
                    }
                };
            }
            if key.code != crossterm::event::KeyCode::F(2) || !path.is_file() {
                return PageMessage::ResultState(self.request_status());
            }
            if let Some(editor) = &item.path_editor {
                return editor(self.menu.obj.clone(), path);
            }

            let editor: &String = &self.menu.obj.lock().unwrap().config.sysop.external_editor;
            let started = crate::term::with_terminal(|| {
                std::process::Command::new(editor)
                    .arg(format!("{}", path.display()))
                    .spawn()
                    .and_then(|mut child| child.wait())
            });
            match started {
                Ok(_) => {
                    return PageMessage::ExternalProgramStarted;
                }
                Err(e) => {
                    log::error!("Error opening editor: {}", e);
                    return PageMessage::InfoBox(InfoState::Error, format!("{}\n\n{}", editor, e));
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

fn can_create_file(path: &std::path::Path) -> bool {
    !path.as_os_str().is_empty() && !path.exists() && path.parent().is_some_and(std::path::Path::is_dir)
}

fn create_empty_file(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::can_create_file;

    fn test_dir() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("icy-board-tui-file-create-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_file_with_an_existing_parent_can_be_created() {
        let dir = test_dir();
        assert!(can_create_file(&dir.join("new.txt")));
    }

    #[test]
    fn a_file_with_a_missing_parent_cannot_be_created() {
        let dir = test_dir();
        assert!(!can_create_file(&dir.join("missing/new.txt")));
    }

    #[test]
    fn an_existing_file_is_for_editing_not_creation() {
        let file = test_dir().join("existing.txt");
        std::fs::File::create(&file).unwrap();
        assert!(!can_create_file(&file));
    }
}
