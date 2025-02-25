use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{IcyBoard, IcyBoardSerializer, accounting_cfg::AccountingConfig};
use icy_board_tui::{
    BORDER_SET,
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue, ResultState},
    get_text,
    save_changes_dialog::SaveChangesDialog,
    tab_page::{Page, PageMessage},
    theme::get_tui_theme,
};
use ratatui::{
    layout::Rect,
    text::Span,
    widgets::{Block, Borders, Padding, Widget},
};

pub struct AccountingRatesEditor {
    state: ConfigMenuState,
    orig: AccountingConfig,
    menu: ConfigMenu<Arc<Mutex<AccountingConfig>>>,

    path: PathBuf,
    save_dialog: Option<SaveChangesDialog>,
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
            save_dialog: None,
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

        let bottom_text = get_text("icb_setup_key_menu_help");
        let block: Block<'_> = Block::new()
            .style(get_tui_theme().background)
            .padding(Padding::new(2, 2, 1 + 4, 0))
            .borders(Borders::ALL)
            .border_set(BORDER_SET)
            .title_alignment(ratatui::layout::Alignment::Center)
            .title_top(Span::styled(get_text("accounting_title"), get_tui_theme().menu_title))
            .title_bottom(Span::styled(bottom_text, get_tui_theme().key_binding))
            .border_style(get_tui_theme().dialog_box);
        block.render(area, frame.buffer_mut());

        let area = Rect {
            x: disp_area.x + 3,
            y: area.y + 1,
            width: disp_area.width - 3,
            height: area.height - 2,
        };
        self.menu.render(area, frame, &mut self.state);
        if let Some(save_changes) = &self.save_dialog {
            save_changes.render(frame, area);
        }
    }

    fn request_status(&self) -> ResultState {
        ResultState {
            edit_mode: icy_board_tui::config_menu::EditMode::None,
            status_line: self.menu.current_status_line(&self.state),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if self.save_dialog.is_some() {
            let res = self.save_dialog.as_mut().unwrap().handle_key_press(key);
            return match res {
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Cancel => {
                    self.save_dialog = None;
                    PageMessage::None
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Close => PageMessage::Close,
                icy_board_tui::save_changes_dialog::SaveChangesMessage::Save => {
                    self.menu.obj.lock().unwrap().save(&self.path).unwrap();
                    PageMessage::Close
                }
                icy_board_tui::save_changes_dialog::SaveChangesMessage::None => PageMessage::None,
            };
        }

        if key.code == crossterm::event::KeyCode::Esc {
            if *self.menu.obj.lock().unwrap() == self.orig {
                return PageMessage::Close;
            }
            self.save_dialog = Some(SaveChangesDialog::new());
            return PageMessage::None;
        }
        let res = self.menu.handle_key_press(key, &mut self.state);
        PageMessage::ResultState(res)
    }
}

pub fn edit_account_config(_board: Arc<Mutex<IcyBoard>>, path: PathBuf) -> PageMessage {
    PageMessage::OpenSubPage(Box::new(AccountingRatesEditor::new(path)))
}
