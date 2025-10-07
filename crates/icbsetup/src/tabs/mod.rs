pub mod about;
pub use about::*;

pub mod general;
pub use general::*;

#[macro_export]
macro_rules! cfg_entry_password {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Text(12, icy_board_tui::config_menu::TextFlags::None, $lock.config.$property.$conf.to_string()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                board.lock().unwrap().config.$property.$conf = icy_board_engine::icy_board::user_base::Password::PlainText(value);
            }),
        )
    };
}
