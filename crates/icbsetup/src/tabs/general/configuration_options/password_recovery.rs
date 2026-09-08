use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::icy_board::{IcyBoard, password_recovery::PasswordRecoveryConfig};
use icy_board_tui::{
    cfg_entry_bool,
    config_menu::{ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{InfoState, Page, PageMessage},
};
use std::sync::{Arc, Mutex};

pub struct PasswordRecovery {
    menu: ICBConfigMenuUI,
    board: Arc<Mutex<IcyBoard>>,
}

fn text(key: &'static str, value: &str, update: fn(&mut PasswordRecoveryConfig, String)) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::Text(254, TextFlags::None, value.to_string()))
            .with_label_width(34)
            .with_edit_width(40)
            .with_help(get_text("recovery_smtp-help"))
            .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                if let ListValue::Text(_, _, value) = value {
                    update(&mut board.lock().unwrap().config.password_recovery, value.clone());
                }
            })),
    )
}

fn number(key: &'static str, value: u32, max: u32, update: fn(&mut PasswordRecoveryConfig, u32)) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::U32(value, 1, max))
            .with_label_width(34)
            .with_help(get_text("recovery_limits-help"))
            .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                if let ListValue::U32(value, _, _) = value {
                    update(&mut board.lock().unwrap().config.password_recovery, *value);
                }
            })),
    )
}

impl PasswordRecovery {
    pub fn new(board: Arc<Mutex<IcyBoard>>) -> Self {
        let entries = {
            let lock = board.lock().unwrap();
            let c = &lock.config.password_recovery;
            vec![
                cfg_entry_bool!("recovery_enabled", 34, password_recovery, enabled, lock),
                text("recovery_smtp_host", &c.smtp_host, |c, v| c.smtp_host = v),
                number("recovery_smtp_port", c.smtp_port.into(), 65535, |c, v| c.smtp_port = v as u16),
                cfg_entry_bool!("recovery_implicit_tls", 34, password_recovery, implicit_tls, lock),
                text("recovery_sender", &c.sender, |c, v| c.sender = v),
                text("recovery_smtp_username", &c.smtp_username, |c, v| c.smtp_username = v),
                text("recovery_smtp_password_env", &c.smtp_password_env, |c, v| c.smtp_password_env = v),
                number("recovery_ttl", c.ttl_minutes, 60, |c, v| c.ttl_minutes = v),
                number("recovery_cooldown", c.cooldown_minutes, 60, |c, v| c.cooldown_minutes = v),
                number("recovery_account_limit", c.account_per_hour, 10, |c, v| c.account_per_hour = v),
                number("recovery_board_limit", c.board_per_hour, 500, |c, v| c.board_per_hour = v),
                number("recovery_attempts", c.max_attempts, 10, |c, v| c.max_attempts = v),
                number("recovery_timeout", c.timeout_seconds, 30, |c, v| c.timeout_seconds = v),
            ]
        };
        Self {
            menu: ICBConfigMenuUI::new(
                get_text("recovery_title"),
                ConfigMenu {
                    obj: board.clone(),
                    entry: entries,
                },
            ),
            board,
        }
    }
}

impl Page for PasswordRecovery {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        self.menu.render(frame, area);
    }
    fn request_status(&self) -> ResultState {
        self.menu.request_status()
    }
    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        if key.code == KeyCode::Esc {
            let board = self.board.lock().unwrap();
            if board
                .config
                .password_recovery
                .validate(board.config.system_control.password_storage_method)
                .is_err()
            {
                return PageMessage::InfoBox(InfoState::Error, get_text("recovery_invalid"));
            }
            if !board.config.password_recovery.enabled {
                board.password_recovery_service.revoke_runtime_challenges();
                // Leave durable revocation to the board settings transaction.
            }
        }
        self.menu.handle_key_press(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_settings_fit_80_by_25() {
        if let Ok(expected) = std::env::var("RECOVERY_TEST_LOCALE") {
            assert_eq!(
                get_text("recovery_title"),
                match expected.as_str() {
                    "en" => "Email password recovery",
                    "de" => "Passwort per E-Mail zurücksetzen",
                    _ => panic!("unsupported recovery test locale"),
                }
            );
        }
        let mut page = PasswordRecovery::new(Arc::new(Mutex::new(IcyBoard::new())));
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 25)).unwrap();
        terminal.draw(|f| page.render(f, f.area())).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..25).map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect()).collect();
        for key in [
            "recovery_enabled",
            "recovery_smtp_host",
            "recovery_smtp_port",
            "recovery_implicit_tls",
            "recovery_sender",
            "recovery_smtp_username",
            "recovery_smtp_password_env",
            "recovery_ttl",
            "recovery_cooldown",
            "recovery_account_limit",
            "recovery_board_limit",
            "recovery_attempts",
            "recovery_timeout",
        ] {
            assert_ne!(get_text(key), key, "untranslated {key}");
            assert!(rows.iter().any(|row| row.contains(&get_text(key))), "clipped {key}");
        }
    }
}
