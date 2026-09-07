use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{IcyBoard, IcyBoardSerializer, accounting_cfg::AccountingConfig};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    get_text,
    tab_page::{InfoState, Page, PageMessage},
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
    pub fn new(path: PathBuf) -> icy_board_engine::Res<Self> {
        let orig = if path.exists() {
            AccountingConfig::load(&path)?
        } else {
            AccountingConfig::default()
        };
        let menu = {
            let config = Arc::new(Mutex::new(orig.clone()));
            let label_width = 33;
            let lock = config.lock().unwrap();
            let mut entry = vec![
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
            // ListItem does not automatically load Fluent help. Keep the keys
            // in the same order as the editable rate fields above.
            let mut keys = [
                "accounting_start_balance",
                "accounting_warning_level",
                "accounting_per_logon",
                "accounting_per_minute",
                "accounting_per_minute_peak",
                "accounting_per_minute_grpChat",
                "accounting_per_message_read",
                "accounting_per_message_captured",
                "accounting_per_message_written",
                "accounting_per_message_written_echoed",
                "accounting_per_message_written_private",
                "accounting_per_file_downloaded",
                "accounting_per_file_bytes_downloaded",
                "accounting_payback_per_file",
                "accounting_payback_per_file_bytes",
            ]
            .into_iter();
            for item in &mut entry {
                if let ConfigEntry::Item(item) = item {
                    let key = keys.next().expect("one help key per accounting rate");
                    item.help = get_text(&format!("{key}-help"));
                    item.status = get_text(&format!("{key}-status"));
                }
            }
            ConfigMenu { obj: config.clone(), entry }.with_aligned_labels()
        };

        Ok(Self {
            menu,
            orig,
            state: ConfigMenuState::default(),
            path,
            save_changes: super::EditorSaveChanges::default(),
        })
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
        if let Some(message) = self.save_changes.handle_key(key, || {
            super::save_file(&self.path, || {
                let config = self.menu.obj.lock().unwrap();
                config.validate()?;
                config.save(&self.path)
            })
        }) {
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
    match AccountingRatesEditor::new(path) {
        Ok(editor) => PageMessage::OpenSubPage(Box::new(editor)),
        Err(error) => PageMessage::InfoBox(InfoState::Error, error.to_string()),
    }
}

pub(super) fn validate_activity_rates(per_use: f64, per_minute: f64) -> icy_board_engine::Res<()> {
    if [per_use, per_minute].iter().any(|rate| !rate.is_finite() || *rate < 0.0) {
        return Err(get_text("accounting_activity_invalid").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    #[test]
    fn accounting_rates_have_help_and_malformed_files_are_not_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rates.toml");
        let editor = AccountingRatesEditor::new(path.clone()).unwrap();
        let mut count = 0;
        for entry in &editor.menu.entry {
            if let ConfigEntry::Item(item) = entry {
                assert!(!item.help.is_empty());
                assert!(!item.status.is_empty());
                count += 1;
            }
        }
        assert_eq!(count, 15);
        std::fs::write(&path, "new_user_balance = 100.0\n").unwrap();
        assert!(AccountingRatesEditor::new(path.clone()).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "new_user_balance = 100.0\n");
    }

    #[test]
    fn activity_rates_require_finite_nonnegative_values() {
        assert!(validate_activity_rates(0.0, 0.25).is_ok());
        for invalid in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(validate_activity_rates(invalid, 0.0).is_err());
            assert!(validate_activity_rates(0.0, invalid).is_err());
        }
    }

    #[test]
    fn nonfinite_rate_save_keeps_the_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rates.toml");
        AccountingConfig::default().save(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut editor = AccountingRatesEditor::new(path.clone()).unwrap();
        editor.menu.obj.lock().unwrap().charge_per_time = f64::INFINITY;
        editor.handle_key_press(KeyEvent::from(KeyCode::Esc));
        editor.handle_key_press(KeyEvent::from(KeyCode::Right));
        assert!(matches!(
            editor.handle_key_press(KeyEvent::from(KeyCode::Enter)),
            PageMessage::InfoBox(InfoState::Error, _)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!editor.save_changes.is_open());
    }

    #[test]
    fn save_hides_form_hint_and_failure_allows_correction_and_retry() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("blocked");
        std::fs::write(&parent, b"not a directory").unwrap();
        let path = parent.join("accounting.toml");
        let mut editor = AccountingRatesEditor::new(path.clone()).unwrap();
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
