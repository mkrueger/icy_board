use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{IcyBoard, IcyBoardSerializer, accounting_cfg::AccountingConfig};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    get_text,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{layout::Rect, text::Span, widgets::Widget};

pub struct AccountingRatesEditor {
    state: ConfigMenuState,
    orig: AccountingConfig,
    menu: ConfigMenu<Arc<Mutex<AccountingConfig>>>,

    path: PathBuf,
    save_changes: super::EditorSaveChanges,
}

impl AccountingRatesEditor {
    pub fn new(path: PathBuf) -> Self {
        let orig = AccountingConfig::load(&path).unwrap_or_else(|_| AccountingConfig::default());
        let menu = {
            let config = Arc::new(Mutex::new(orig.clone()));
            let label_width = 33;
            let lock = config.lock().unwrap();
            let entry = vec![
                ConfigEntry::Separator,
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_start_balance"),
                        ListValue::Float(lock.new_user_balance, lock.new_user_balance.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.new_user_balance = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_warning_level"),
                        ListValue::Float(lock.warn_level, lock.warn_level.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.warn_level = value;
                    }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("accounting_charges_label")),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_logon"),
                        ListValue::Float(lock.charge_per_logon, lock.charge_per_logon.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_logon = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_minute"),
                        ListValue::Float(lock.charge_per_time, lock.charge_per_time.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_time = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_minute_peak"),
                        ListValue::Float(lock.charge_per_peak_time, lock.charge_per_peak_time.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_peak_time = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_minute_grpChat"),
                        ListValue::Float(lock.charge_per_group_chat_time, lock.charge_per_group_chat_time.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_group_chat_time = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_message_read"),
                        ListValue::Float(lock.charge_per_msg_read, lock.charge_per_msg_read.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_msg_read = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_message_captured"),
                        ListValue::Float(lock.charge_per_msg_read_captured, lock.charge_per_msg_read_captured.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_msg_read_captured = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_message_written"),
                        ListValue::Float(lock.charge_per_msg_written, lock.charge_per_msg_written.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_msg_written = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_message_written_echoed"),
                        ListValue::Float(lock.charge_per_msg_write_echoed, lock.charge_per_msg_write_echoed.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_msg_write_echoed = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_message_written_private"),
                        ListValue::Float(lock.charge_per_msg_write_private, lock.charge_per_msg_write_private.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_msg_write_private = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_file_downloaded"),
                        ListValue::Float(lock.charge_per_download_file, lock.charge_per_download_file.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_download_file = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_per_file_bytes_downloaded"),
                        ListValue::Float(lock.charge_per_download_bytes, lock.charge_per_download_bytes.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.charge_per_download_bytes = value;
                    }),
                ),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("accounting_payback_label")),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_payback_per_file"),
                        ListValue::Float(lock.pay_back_for_upload_file, lock.pay_back_for_upload_file.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.pay_back_for_upload_file = value;
                    }),
                ),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("accounting_payback_per_file_bytes"),
                        ListValue::Float(lock.pay_back_for_upload_bytes, lock.pay_back_for_upload_bytes.to_string()),
                    )
                    .with_label_width(label_width)
                    .with_update_float_value(&|acc_cfg: &Arc<Mutex<AccountingConfig>>, value: f64| {
                        let mut ib = acc_cfg.lock().unwrap();
                        ib.pay_back_for_upload_bytes = value;
                    }),
                ),
            ];
            ConfigMenu { obj: config.clone(), entry }
        };

        Self {
            menu,
            orig,
            state: ConfigMenuState::default(),
            path,
            save_changes: super::EditorSaveChanges::default(),
        }
    }
}

impl Page for AccountingRatesEditor {
    fn render(&mut self, frame: &mut ratatui::Frame, disp_area: ratatui::prelude::Rect) {
        let area = Rect {
            x: disp_area.x + 1,
            y: disp_area.y,
            width: disp_area.width.saturating_sub(2),
            height: disp_area.height,
        };

        let block = super::standalone_editor_frame(
            get_text("icb_setup_key_menu_help"),
            self.save_changes.is_open() || self.state.is_path_browser_open(),
        )
        .title_top(Span::styled(get_text("accounting_title"), get_tui_theme().menu_title));
        block.render(area, frame.buffer_mut());

        let area = Rect {
            x: disp_area.x + 3,
            y: area.y + 1,
            width: disp_area.width - 3,
            height: area.height - 2,
        };
        super::render_config_form(frame, area, &mut self.menu, &mut self.state);
        self.save_changes.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_msg: icy_board_tui::config_menu::EditMessage::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if let Some(message) = self
            .save_changes
            .handle_key(key, || super::save_file(&self.path, || self.menu.obj.lock().unwrap().save(&self.path)))
        {
            return message;
        }

        let res = self.menu.handle_key_press(key, &mut self.state);
        if res.edit_msg == icy_board_tui::config_menu::EditMessage::Close {
            return self.save_changes.request_close(self.orig != *self.menu.obj.lock().unwrap());
        }
        PageMessage::ResultState(res)
    }
}

pub fn edit_account_config(_board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(AccountingRatesEditor::new(path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    #[test]
    fn save_hides_form_hint_and_failure_allows_correction_and_retry() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("blocked");
        std::fs::write(&parent, b"not a directory").unwrap();
        let path = parent.join("accounting.toml");
        let mut editor = AccountingRatesEditor::new(path.clone());
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 25)).unwrap();
        let mut assert_hint = |editor: &mut AccountingRatesEditor, visible: bool| {
            terminal.draw(|frame| editor.render(frame, Rect::new(0, 1, 80, 23))).unwrap();
            let border: String = (1..79).map(|x| terminal.backend().buffer()[(x, 23)].symbol()).collect();
            assert_eq!(border.contains(&get_text("icb_setup_key_menu_help")), visible);
        };

        assert!(matches!(editor.handle_key_press(KeyEvent::from(KeyCode::Esc)), PageMessage::Close));
        editor.menu.obj.lock().unwrap().new_user_balance += 1.0;
        assert_hint(&mut editor, true);
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        assert!(editor.save_changes.is_open());
        assert_hint(&mut editor, false);
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        assert!(!editor.save_changes.is_open());
        assert_hint(&mut editor, true);
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        editor.handle_key_press(KeyEvent::from(KeyCode::Right));
        assert!(matches!(
            editor.handle_key_press(KeyEvent::from(KeyCode::Enter)),
            PageMessage::InfoBox(icy_board_tui::tab_page::InfoState::Error, _)
        ));
        assert!(!editor.save_changes.is_open());
        assert_hint(&mut editor, true);
        let selected = editor.state.selected;
        editor.handle_key_press(KeyEvent::from(KeyCode::Down));
        assert_ne!(editor.state.selected, selected);
        std::fs::remove_file(parent).unwrap();
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        editor.handle_key_press(KeyEvent::from(KeyCode::Right));
        assert!(matches!(editor.handle_key_press(KeyEvent::from(KeyCode::Enter)), PageMessage::Close));
        assert!(AccountingConfig::load(&path).unwrap() == *editor.menu.obj.lock().unwrap());
    }
}
