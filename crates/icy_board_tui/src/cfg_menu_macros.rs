#[macro_export]
macro_rules! cfg_entry_text {
    ($i:expr, $width:expr, $field_width:expr, $property:ident, $conf:ident, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Text($field_width, $lock.config.$property.$conf.clone()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                board.lock().unwrap().config.$property.$conf = value;
            }),
        )
    };

    ($i:expr, $width:expr, $field_width:expr, $property:ident, $conf:ident, $lock:expr, $edit_with:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Text($field_width, $lock.config.$property.$conf.clone()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_edit_width($edit_with)
            .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value: String| {
                board.lock().unwrap().config.$property.$conf = value;
            }),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_bool {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Bool($lock.config.$property.$conf),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value: bool| {
                board.lock().unwrap().config.$property.$conf = value;
            }),
        )
    };
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr, $edit_width:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Bool($lock.config.$property.$conf),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_edit_width($edit_width)
            .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value: bool| {
                board.lock().unwrap().config.$property.$conf = value;
            }),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_u32 {
    ($i:expr, $width:expr, $min:expr, $max:expr, $property:ident, $conf:ident, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::U32($lock.config.$property.$conf, $min, $max),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::U32(val, _, _) = value else {
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
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::U32($lock.config.$property.$conf as u32, $min, $max),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::U32(val, _, _) = value else {
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
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::U32($lock.config.$property.$conf as u32, $min, $max),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::U32(val, _, _) = value else {
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
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Security($lock.config.$property.$conf.clone(), $lock.config.$property.$conf.to_string()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::Security(val, _) = value else {
                    return;
                };
                board.lock().unwrap().config.$property.$conf = val.clone();
            })),
        )
    };
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr, $edit_width:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Security($lock.config.$property.$conf.clone(), $lock.config.$property.$conf.to_string()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_edit_width($edit_width)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::Security(val, _) = value else {
                    return;
                };
                board.lock().unwrap().config.$property.$conf = val.clone();
            })),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_color {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Color($lock.config.$property.$conf.clone()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width), /*
                                       .with_update_color_value(&|board: &Arc<Mutex<IcyBoard>>, value: Color| {
                                           board.lock().unwrap().config.$property.$conf = value;
                                       })*/
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_path {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Path($lock.config.$property.$conf.clone()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::Path(val) = value else {
                    return;
                };
                board.lock().unwrap().config.$property.$conf = val.clone();
            })),
        )
    };
    ($i:expr, $width:expr, $property:ident, $conf:ident, $editor:expr, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Path($lock.config.$property.$conf.clone()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_path_editor($editor)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::Path(val) = value else {
                    return;
                };
                board.lock().unwrap().config.$property.$conf = val.clone();
            })),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_time {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::Time($lock.config.$property.$conf.clone(), $lock.config.$property.$conf.to_string()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::Time(val, _) = value else {
                    return;
                };
                board.lock().unwrap().config.$property.$conf = val.clone();
            })),
        )
    };
}

#[macro_export]
macro_rules! cfg_entry_dow {
    ($i:expr, $width:expr, $property:ident, $conf:ident, $lock:expr) => {
        icy_board_tui::config_menu::ConfigEntry::Item(
            icy_board_tui::config_menu::ListItem::new(
                icy_board_tui::get_text($i),
                icy_board_tui::config_menu::ListValue::DoW($lock.config.$property.$conf.clone(), $lock.config.$property.$conf.to_string()),
            )
            .with_status(&icy_board_tui::get_text(&format!("{}-status", $i)))
            .with_help(&icy_board_tui::get_text(&format!("{}-help", $i)))
            .with_label_width($width)
            .with_update_value(Box::new(|board: &Arc<Mutex<IcyBoard>>, value: &icy_board_tui::config_menu::ListValue| {
                let icy_board_tui::config_menu::ListValue::DoW(val, _) = value else {
                    return;
                };
                board.lock().unwrap().config.$property.$conf = val.clone();
            })),
        )
    };
}
