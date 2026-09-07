use crossterm::event::{KeyEvent, KeyEventKind};
use icy_board_tui::{
    config_menu::{ConfigMenu, ConfigMenuState, EditMessage, ListValue, ResultState},
    save_changes_dialog::{SaveChangesDialog, SaveChangesMessage},
    tab_page::PageMessage,
    theme::get_tui_theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Widget},
};

/// A parent-owned list (e.g. FTN) does not need this independent save boundary.
#[derive(Default)]
pub(crate) struct EditorSaveChanges {
    dialog: Option<SaveChangesDialog>,
}

impl EditorSaveChanges {
    pub fn is_open(&self) -> bool {
        self.dialog.is_some()
    }

    pub fn request_close(&mut self, changed: bool) -> PageMessage {
        if changed {
            self.dialog = Some(SaveChangesDialog::new());
            PageMessage::None
        } else {
            PageMessage::Close
        }
    }

    /// Consume modal input before forms or lists can act on it.
    pub fn handle_key(&mut self, key: KeyEvent, save: impl FnOnce() -> PageMessage) -> Option<PageMessage> {
        let dialog = self.dialog.as_mut()?;
        if key.kind == KeyEventKind::Release {
            return Some(PageMessage::None);
        }
        Some(match dialog.handle_key_press(key) {
            SaveChangesMessage::Cancel => {
                self.dialog = None;
                PageMessage::None
            }
            SaveChangesMessage::Close => {
                self.dialog = None;
                PageMessage::Close
            }
            SaveChangesMessage::Save => {
                // On failure return to the editable working copy, not a stale prompt.
                self.dialog = None;
                save()
            }
            SaveChangesMessage::None => PageMessage::None,
        })
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if let Some(dialog) = &self.dialog {
            dialog.render(frame, area);
        }
    }
}

/// Shared form lifecycle; the menu's model and validation belong to the editor.
pub(crate) struct EditorDialog<T> {
    pub menu: Option<ConfigMenu<T>>,
    pub state: ConfigMenuState,
}

impl<T> Default for EditorDialog<T> {
    fn default() -> Self {
        Self {
            menu: None,
            state: ConfigMenuState::default(),
        }
    }
}

impl<T> EditorDialog<T> {
    pub fn is_open(&self) -> bool {
        self.menu.is_some()
    }

    pub fn open(&mut self, menu: ConfigMenu<T>) {
        super::reset_config_state(&mut self.state);
        self.menu = Some(menu);
    }

    pub fn close(&mut self) {
        self.menu = None;
    }

    /// Validated editors inspect Close before committing and closing the draft.
    pub fn handle_input(&mut self, key: KeyEvent) -> Option<ResultState> {
        let menu = self.menu.as_mut()?;
        if key.kind == KeyEventKind::Release {
            return Some(ResultState::default());
        }
        // ConfigMenu gives browsers and expanded combos first refusal on Escape.
        Some(menu.handle_key_press(key, &mut self.state))
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<PageMessage> {
        let result = self.handle_input(key)?;
        Some(if result.edit_msg == EditMessage::Close {
            self.close();
            PageMessage::None
        } else {
            PageMessage::ResultState(result)
        })
    }

    pub fn status(&self) -> ResultState {
        self.menu
            .as_ref()
            .map_or_else(ResultState::default, |menu| ResultState::status_line(menu.current_status_line(&self.state)))
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, title: String, hint: String) {
        let Some(menu) = &mut self.menu else { return };
        let browse = super::path_browse_hint(menu, &self.state);
        let hint = if self.state.is_path_browser_open() {
            String::new()
        } else {
            [hint, browse].into_iter().filter(|text| !text.is_empty()).collect::<Vec<_>>().join("  ")
        };
        Clear.render(area, frame.buffer_mut());
        super::popup_frame(title)
            .title_bottom(Span::styled(hint, get_tui_theme().key_binding))
            .render(area, frame.buffer_mut());
        render_config_form(frame, area.inner(Margin { horizontal: 1, vertical: 1 }), menu, &mut self.state);
    }
}

pub(crate) fn render_config_form<T>(frame: &mut Frame, area: Rect, menu: &mut ConfigMenu<T>, state: &mut ConfigMenuState) {
    if area.is_empty() {
        return;
    }
    menu.render(area, frame, state);
    if !state.is_path_browser_open()
        && let Some(item) = menu.get_item(state.selected)
        && item.editable()
        && !matches!(item.value, ListValue::Bool(_) | ListValue::ComboBox(_) | ListValue::ValueList(..))
    {
        item.text_field_state.set_cursor_position(frame);
    }
}

/// Only the active editor advertises its commands; geometry stays unchanged.
pub(crate) fn list_editor_frame(title: String, hint: String, modal: bool) -> Block<'static> {
    editor_frame_hint(super::list_frame(title), hint, modal)
}

/// Full-page forms retain their background and heading layout.
pub(crate) fn standalone_editor_frame(hint: String, modal: bool) -> Block<'static> {
    editor_frame_hint(
        Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 5, 0))
            .borders(Borders::ALL)
            .border_set(icy_board_tui::BORDER_SET)
            .border_style(get_tui_theme().dialog_box)
            .title_alignment(Alignment::Center),
        hint,
        modal,
    )
}

fn editor_frame_hint(block: Block<'static>, hint: String, modal: bool) -> Block<'static> {
    if modal {
        block
    } else {
        block.title_bottom(Span::styled(hint, get_tui_theme().key_binding))
    }
}

/// Multi-line legends use the same visibility policy as border shortcuts.
pub(crate) fn render_editor_footer(frame: &mut Frame, area: Rect, lines: Vec<Line<'static>>, modal: bool) {
    if !modal {
        for (line, row) in lines.into_iter().zip(area.rows()) {
            line.centered().render(row, frame.buffer_mut());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;
    use icy_board_tui::config_menu::{ConfigEntry, ListItem, TextFlags};

    fn menu() -> ConfigMenu<()> {
        ConfigMenu {
            obj: (),
            entry: vec![ConfigEntry::Item(
                ListItem::new("Name".into(), ListValue::Text(20, TextFlags::None, "draft".into()))
                    .with_help("# Help\n\nField explanation")
                    .with_status("Field status"),
            )],
        }
    }

    #[test]
    fn reopening_resets_navigation_but_preserves_board_root() {
        let root = tempfile::tempdir().unwrap();
        let mut dialog = EditorDialog::default();
        dialog.state.path_base = Some(root.path().to_path_buf());
        dialog.state.selected = 8;
        dialog.open(menu());
        assert_eq!(dialog.state.selected, 0);
        assert_eq!(dialog.state.path_base.as_deref(), Some(root.path()));
        assert!(dialog.is_open());
        dialog.close();
        assert!(!dialog.is_open());
        dialog.open(menu());
        assert_eq!(dialog.state.selected, 0);
        assert_eq!(dialog.state.path_base.as_deref(), Some(root.path()));
    }

    #[test]
    fn form_forwards_help_and_status_without_closing() {
        let mut dialog = EditorDialog::default();
        assert!(dialog.handle_key(KeyEvent::from(KeyCode::F(1))).is_none());
        dialog.open(menu());
        assert_eq!(dialog.status().status_line, "Field status");
        assert!(matches!(
            dialog.handle_key(KeyEvent::from(KeyCode::F(1))),
            Some(PageMessage::ResultState(ResultState { edit_msg: EditMessage::DisplayHelp(text), .. }))
                if text.contains("Field explanation")
        ));
        assert!(dialog.is_open());
        assert!(matches!(dialog.handle_key(KeyEvent::from(KeyCode::Esc)), Some(PageMessage::None)));
        assert!(!dialog.is_open());
    }

    #[test]
    fn validated_form_keeps_draft_until_editor_accepts_close() {
        let mut dialog = EditorDialog::default();
        dialog.open(menu());
        assert!(dialog.handle_input(KeyEvent::from(KeyCode::Esc)).unwrap().edit_msg == EditMessage::Close);
        assert!(dialog.is_open());
        assert!(matches!(&dialog.menu.as_ref().unwrap().get_item(0).unwrap().value, ListValue::Text(_, _, text) if text == "draft"));
        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn save_callback_only_runs_after_explicit_yes_confirmation() {
        let mut save = EditorSaveChanges::default();
        assert!(matches!(save.request_close(false), PageMessage::Close));
        assert!(!save.is_open());
        assert!(save.handle_key(KeyEvent::from(KeyCode::Enter), || panic!("no prompt")).is_none());
        save.request_close(true);
        assert!(matches!(
            save.handle_key(KeyEvent::from(KeyCode::Enter), || panic!("default is No")),
            Some(PageMessage::Close)
        ));
        save.request_close(true);
        save.handle_key(KeyEvent::from(KeyCode::Right), || panic!("not confirmed"));
        assert!(save.is_open());
        let mut calls = 0;
        assert!(matches!(
            save.handle_key(KeyEvent::from(KeyCode::Enter), || {
                calls += 1;
                PageMessage::Close
            }),
            Some(PageMessage::Close)
        ));
        assert_eq!(calls, 1);
        assert!(!save.is_open());
    }
}
