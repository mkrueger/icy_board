pub mod about;
pub use about::*;

pub mod general;
pub use general::*;

#[macro_export]
macro_rules! cfg_entry_text {
    ($i:expr, $width:expr, $field_width:expr, $property:ident, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(icy_board_tui::get_text($i), ListValue::Text($field_width, $lock.config.$property.$conf.clone()))
                .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
                .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                    board.lock().unwrap().config.$property.$conf = value;
                }),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_bool {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(icy_board_tui::get_text($i), ListValue::Bool($lock.config.$property.$conf))
                .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
                .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value: bool| {
                    board.lock().unwrap().config.$property.$conf = value;
                }),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_u32 {
    ($i:expr, $width:expr, $min:expr, $max:expr, $property:ident, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(icy_board_tui::get_text($i), ListValue::U32($lock.config.$property.$conf, $min, $max))
                .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
                .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                    let ListValue::U32(val, _, _) = value else {
                        return;
                    };
                    board.lock().unwrap().config.$property.$conf = *val;
                })),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_u16 {
    ($i:expr, $width:expr, $min:expr, $max:expr, $property:ident, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(icy_board_tui::get_text($i), ListValue::U32($lock.config.$property.$conf as u32, $min, $max))
                .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
                .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                    let ListValue::U32(val, _, _) = value else {
                        return;
                    };
                    board.lock().unwrap().config.$property.$conf = *val as u16;
                })),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_u8 {
    ($i:expr, $width:expr, $min:expr, $max:expr, $property:ident, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(icy_board_tui::get_text($i), ListValue::U32($lock.config.$property.$conf as u32, $min, $max))
                .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
                .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                    let ListValue::U32(val, _, _) = value else {
                        return;
                    };
                    board.lock().unwrap().config.$property.$conf = *val as u8;
                })),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_sec_level {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(icy_board_tui::get_text($i), ListValue::U32($lock.config.$property.$conf as u32, 0, 255))
                .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
                .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
                .with_label_width($width)
                .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                    let ListValue::U32(val, _, _) = value else {
                        return;
                    };
                    board.lock().unwrap().config.$property.$conf = *val as u8;
                })),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_password {
    ($i:expr, $field_width:expr, $property:ident, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(
                icy_board_tui::get_text($i),
                ListValue::Text($field_width, $lock.config.$property.$conf.to_string()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width(12)
            .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                board.lock().unwrap().config.$property.$conf = Password::PlainText(value);
            }),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_color {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        ConfigEntry::Item(
            ListItem::new(icy_board_tui::get_text($i), ListValue::Color($lock.config.$property.$conf.clone()))
                .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
                .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
                .with_label_width($width), /*
                                           .with_update_color_value(&|board: &Arc<Mutex<IcyBoard>>, value: Color| {
                                               board.lock().unwrap().config.$property.$conf = value;
                                           })*/
        )
    };
}
