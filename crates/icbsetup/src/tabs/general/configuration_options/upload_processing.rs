use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use icy_board_engine::icy_board::{
    IcyBoard,
    icb_config::{UploadProcessingConfig, UploadPublishPolicy},
};
use icy_board_tui::{
    config_menu::{ComboBox, ComboBoxValue, ConfigEntry, ConfigMenu, ListItem, ListValue, ResultState, TextFlags},
    get_text,
    icbconfigmenu::ICBConfigMenuUI,
    tab_page::{Page, PageMessage},
};

pub struct UploadProcessing {
    menu: ICBConfigMenuUI,
    validation_error: Arc<Mutex<Option<String>>>,
}

fn policy_name(policy: UploadPublishPolicy) -> String {
    match policy {
        UploadPublishPolicy::Immediate => get_text("upload_processing_policy_immediate"),
        UploadPublishPolicy::AfterProcessing => get_text("upload_processing_policy_after_processing"),
        UploadPublishPolicy::ManualApproval => get_text("upload_processing_policy_manual_approval"),
    }
}

fn bool_entry(key: &'static str, label_width: u16, value: bool, update: fn(&mut UploadProcessingConfig, bool)) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::Bool(value))
            .with_status(get_text(&format!("{key}-status")))
            .with_help(get_text(&format!("{key}-help")))
            .with_label_width(label_width)
            .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                let ListValue::Bool(value) = value else {
                    return;
                };
                update(&mut board.lock().unwrap().config.upload_processing, *value);
            })),
    )
}

fn number_entry(
    key: &'static str,
    label_width: u16,
    value: u64,
    minimum: u32,
    maximum: u32,
    update: fn(&mut UploadProcessingConfig, u64),
) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::U32(value.min(u64::from(u32::MAX)) as u32, minimum, maximum))
            .with_status(get_text(&format!("{key}-status")))
            .with_help(get_text(&format!("{key}-help")))
            .with_label_width(label_width)
            .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                let ListValue::U32(value, _, _) = value else {
                    return;
                };
                update(&mut board.lock().unwrap().config.upload_processing, u64::from(*value));
            })),
    )
}

fn path_entry(key: &'static str, label_width: u16, value: PathBuf, update: fn(&mut UploadProcessingConfig, PathBuf)) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::Path(value))
            .with_status(get_text(&format!("{key}-status")))
            .with_help(get_text(&format!("{key}-help")))
            .with_label_width(label_width)
            .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                let ListValue::Path(value) = value else {
                    return;
                };
                update(&mut board.lock().unwrap().config.upload_processing, value.clone());
            })),
    )
}

fn text_entry(
    key: &'static str,
    label_width: u16,
    maximum: u16,
    value: String,
    update: fn(&mut UploadProcessingConfig, String),
) -> ConfigEntry<Arc<Mutex<IcyBoard>>> {
    ConfigEntry::Item(
        ListItem::new(get_text(key), ListValue::Text(maximum, TextFlags::None, value))
            .with_status(get_text(&format!("{key}-status")))
            .with_help(get_text(&format!("{key}-help")))
            .with_label_width(label_width)
            .with_edit_width(70)
            .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                let ListValue::Text(_, _, value) = value else {
                    return;
                };
                update(&mut board.lock().unwrap().config.upload_processing, value.clone());
            })),
    )
}

fn split_list(value: &str) -> Vec<String> {
    value.split(';').map(str::trim).filter(|entry| !entry.is_empty()).map(str::to_string).collect()
}

impl UploadProcessing {
    pub fn new(icy_board: Arc<Mutex<IcyBoard>>) -> Self {
        let validation_error = Arc::new(Mutex::new(None));
        let menu = Self::build_menu(icy_board, validation_error.clone());
        Self {
            menu: ICBConfigMenuUI::new(get_text("upload_processing_title"), menu),
            validation_error,
        }
    }

    fn build_menu(icy_board: Arc<Mutex<IcyBoard>>, validation_error: Arc<Mutex<Option<String>>>) -> ConfigMenu<Arc<Mutex<IcyBoard>>> {
        {
            let lock = icy_board.lock().unwrap();
            let options = &lock.config.upload_processing;
            let label_width = 35;
            let mut entries = vec![
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("upload_processing_group_publication")),
                ConfigEntry::Item(
                    ListItem::new(
                        get_text("upload_processing_publish_policy"),
                        ListValue::ComboBox(ComboBox {
                            cur_value: ComboBoxValue::new(policy_name(options.publish_policy), format!("{:?}", options.publish_policy)),
                            selected_item: 0,
                            is_edit_open: false,
                            first_item: 0,
                            values: vec![
                                ComboBoxValue::new(policy_name(UploadPublishPolicy::Immediate), "Immediate"),
                                ComboBoxValue::new(policy_name(UploadPublishPolicy::AfterProcessing), "AfterProcessing"),
                                ComboBoxValue::new(policy_name(UploadPublishPolicy::ManualApproval), "ManualApproval"),
                            ],
                        }),
                    )
                    .with_status(get_text("upload_processing_publish_policy-status"))
                    .with_help(get_text("upload_processing_publish_policy-help"))
                    .with_label_width(label_width)
                    .with_update_combobox_value(&|board: &Arc<Mutex<IcyBoard>>, combo: &ComboBox| {
                        board.lock().unwrap().config.upload_processing.publish_policy = match combo.cur_value.value.as_str() {
                            "AfterProcessing" => UploadPublishPolicy::AfterProcessing,
                            "ManualApproval" => UploadPublishPolicy::ManualApproval,
                            _ => UploadPublishPolicy::Immediate,
                        };
                    }),
                ),
                bool_entry("upload_processing_notify_sysop", label_width, options.notify_sysop, |options, value| {
                    options.notify_sysop = value;
                }),
                path_entry(
                    "upload_processing_quarantine",
                    label_width,
                    options.quarantine_path.clone(),
                    |options, value| {
                        options.quarantine_path = value;
                    },
                ),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("upload_processing_group_advertising")),
                bool_entry(
                    "upload_processing_remove_advertisements",
                    label_width,
                    options.remove_advertisements,
                    |options, value| {
                        options.remove_advertisements = value;
                    },
                ),
                path_entry("upload_processing_rules", label_width, options.advertisement_rules.clone(), |options, value| {
                    options.advertisement_rules = value;
                }),
                path_entry(
                    "upload_processing_advertisement_file",
                    label_width,
                    options.advertisement_file.clone(),
                    |options, value| options.advertisement_file = value,
                ),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("upload_processing_group_zip")),
                bool_entry("upload_processing_repack_zip", label_width, options.repack_to_zip, |options, value| {
                    options.repack_to_zip = value;
                }),
                number_entry(
                    "upload_processing_compression",
                    label_width,
                    options.compression_level.max(0) as u64,
                    0,
                    9,
                    |options, value| {
                        options.compression_level = value as i64;
                    },
                ),
                text_entry(
                    "upload_processing_replacement_comment",
                    label_width,
                    255,
                    options.replacement_archive_comment.clone(),
                    |options, value| options.replacement_archive_comment = value,
                ),
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("upload_processing_group_scanner")),
            ];
            entries.push(ConfigEntry::Item(
                ListItem::new(get_text("upload_processing_scanner_enabled"), ListValue::Bool(options.scanner.enabled))
                    .with_status(get_text("upload_processing_scanner_enabled-status"))
                    .with_help(get_text("upload_processing_scanner_enabled-help"))
                    .with_label_width(label_width)
                    .with_update_bool_value(&|board: &Arc<Mutex<IcyBoard>>, value| {
                        board.lock().unwrap().config.upload_processing.scanner.enabled = value;
                    }),
            ));
            entries.push(ConfigEntry::Item(
                ListItem::new(
                    get_text("upload_processing_scanner_executable"),
                    ListValue::Text(255, TextFlags::None, options.scanner.executable.to_string_lossy().to_string()),
                )
                .with_status(get_text("upload_processing_scanner_executable-status"))
                .with_help(get_text("upload_processing_scanner_executable-help"))
                .with_label_width(label_width)
                .with_edit_width(70)
                .with_update_text_value(&|board: &Arc<Mutex<IcyBoard>>, value| {
                    board.lock().unwrap().config.upload_processing.scanner.executable = PathBuf::from(value);
                }),
            ));
            let scanner_validation = validation_error.clone();
            entries.push(ConfigEntry::Item(
                ListItem::new(
                    get_text("upload_processing_scanner_arguments"),
                    ListValue::Text(1024, TextFlags::None, options.scanner.arguments.join("; ")),
                )
                .with_status(get_text("upload_processing_scanner_arguments-status"))
                .with_help(get_text("upload_processing_scanner_arguments-help"))
                .with_label_width(label_width)
                .with_edit_width(70)
                .with_update_value(Box::new(move |board: &Arc<Mutex<IcyBoard>>, value: &ListValue| {
                    let ListValue::Text(_, _, value) = value else {
                        return;
                    };
                    let arguments = split_list(value);
                    if arguments.iter().filter(|argument| argument.as_str() == "{file}").count() != 1 {
                        *scanner_validation.lock().unwrap() = Some(get_text("upload_processing_scanner_arguments-invalid"));
                        return;
                    }
                    board.lock().unwrap().config.upload_processing.scanner.arguments = arguments;
                    *scanner_validation.lock().unwrap() = None;
                })),
            ));
            entries.push(number_entry(
                "upload_processing_scanner_timeout",
                label_width,
                options.scanner.timeout_seconds,
                1,
                86_400,
                |options, value| options.scanner.timeout_seconds = value,
            ));
            entries.push(number_entry(
                "upload_processing_scanner_clean_code",
                label_width,
                options.scanner.clean_exit_code.max(0) as u64,
                0,
                255,
                |options, value| options.scanner.clean_exit_code = value as i32,
            ));
            entries.push(number_entry(
                "upload_processing_scanner_infected_code",
                label_width,
                options.scanner.infected_exit_code.max(0) as u64,
                0,
                255,
                |options, value| options.scanner.infected_exit_code = value as i32,
            ));
            entries.extend([
                ConfigEntry::Separator,
                ConfigEntry::Label(get_text("upload_processing_group_limits")),
                number_entry(
                    "upload_processing_max_members",
                    label_width,
                    options.max_members as u64,
                    1,
                    1_000_000,
                    |options, value| {
                        options.max_members = value as usize;
                    },
                ),
                number_entry(
                    "upload_processing_max_member_size",
                    label_width,
                    options.max_member_size,
                    1,
                    u32::MAX,
                    |options, value| {
                        options.max_member_size = value;
                    },
                ),
                number_entry(
                    "upload_processing_max_expanded_size",
                    label_width,
                    options.max_expanded_size,
                    1,
                    u32::MAX,
                    |options, value| {
                        options.max_expanded_size = value;
                    },
                ),
                number_entry(
                    "upload_processing_max_ratio",
                    label_width,
                    options.max_compression_ratio,
                    1,
                    1_000_000,
                    |options, value| options.max_compression_ratio = value,
                ),
            ]);
            ConfigMenu {
                obj: icy_board.clone(),
                entry: entries,
            }
        }
    }
}

impl Page for UploadProcessing {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        self.menu.render(frame, area);
    }

    fn request_status(&self) -> ResultState {
        match self.validation_error.lock().unwrap().clone() {
            Some(error) => ResultState::status_line(error),
            None => self.menu.request_status(),
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> PageMessage {
        self.menu.handle_key_press(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_builds_from_default_upload_processing_settings() {
        let board = Arc::new(Mutex::new(IcyBoard::default()));
        let page = UploadProcessing::new(board);
        assert!(page.validation_error.lock().unwrap().is_none());
    }

    #[test]
    fn menu_has_only_the_expected_entries_in_logical_groups() {
        let menu = UploadProcessing::build_menu(Arc::new(Mutex::new(IcyBoard::default())), Arc::new(Mutex::new(None)));
        let mut groups: Vec<(String, Vec<String>)> = Vec::new();
        for entry in &menu.entry {
            match entry {
                ConfigEntry::Label(title) => groups.push((title.clone(), Vec::new())),
                ConfigEntry::Item(item) => groups.last_mut().expect("entry must have a heading").1.push(item.status.clone()),
                ConfigEntry::Separator => {}
                _ => panic!("unexpected upload menu structure"),
            }
        }
        let expected: [(&str, &[&str]); 5] = [
            ("publication", &["publish_policy", "notify_sysop", "quarantine"]),
            ("advertising", &["remove_advertisements", "rules", "advertisement_file"]),
            ("zip", &["repack_zip", "compression", "replacement_comment"]),
            (
                "scanner",
                &[
                    "scanner_enabled",
                    "scanner_executable",
                    "scanner_arguments",
                    "scanner_timeout",
                    "scanner_clean_code",
                    "scanner_infected_code",
                ],
            ),
            ("limits", &["max_members", "max_member_size", "max_expanded_size", "max_ratio"]),
        ];
        let expected: Vec<_> = expected
            .iter()
            .map(|(group, keys)| {
                (
                    get_text(&format!("upload_processing_group_{group}")),
                    keys.iter().map(|key| get_text(&format!("upload_processing_{key}-status"))).collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(groups, expected);
    }

    #[test]
    fn single_advertisement_checkbox_updates_the_unified_setting() {
        let board = Arc::new(Mutex::new(IcyBoard::default()));
        let menu = UploadProcessing::build_menu(board.clone(), Arc::new(Mutex::new(None)));
        let items: Vec<_> = (0..menu.count()).map(|index| menu.get_item(index).unwrap()).collect();
        // Notification, unified ad removal, ZIP repacking and scanner activation only.
        assert_eq!(items.iter().filter(|item| matches!(&item.value, ListValue::Bool(_))).count(), 4);
        let ad_items: Vec<_> = items
            .iter()
            .filter(|item| item.status == get_text("upload_processing_remove_advertisements-status"))
            .collect();
        assert_eq!(ad_items.len(), 1);
        let item = ad_items[0];
        assert!(matches!(&item.value, ListValue::Bool(_)));
        for enabled in [false, true] {
            item.update_value.as_ref().unwrap()(&board, &ListValue::Bool(enabled));
            assert_eq!(board.lock().unwrap().config.upload_processing.remove_advertisements, enabled);
        }
    }

    #[test]
    fn single_advertisement_path_preserves_spaces_and_semicolons_literally() {
        let board = Arc::new(Mutex::new(IcyBoard::default()));
        for path in ["", " ads/own board; notice.txt ", "ppe/own board; generator.PpE"] {
            let menu = UploadProcessing::build_menu(board.clone(), Arc::new(Mutex::new(None)));
            let items: Vec<_> = (0..menu.count())
                .map(|index| menu.get_item(index).unwrap())
                .filter(|item| item.status == get_text("upload_processing_advertisement_file-status"))
                .collect();
            assert_eq!(items.len(), 1);
            assert!(matches!(&items[0].value, ListValue::Path(_)));
            let value = PathBuf::from(path);
            items[0].update_value.as_ref().unwrap()(&board, &ListValue::Path(value.clone()));
            assert_eq!(board.lock().unwrap().config.upload_processing.advertisement_file, value);

            let rebuilt = UploadProcessing::build_menu(board.clone(), Arc::new(Mutex::new(None)));
            let item = (0..rebuilt.count())
                .map(|index| rebuilt.get_item(index).unwrap())
                .find(|item| item.status == get_text("upload_processing_advertisement_file-status"))
                .unwrap();
            assert!(matches!(&item.value, ListValue::Path(path) if path == &value));
        }
    }

    #[test]
    fn compression_immediately_follows_repack_without_a_heading_or_separator() {
        let menu = UploadProcessing::build_menu(Arc::new(Mutex::new(IcyBoard::default())), Arc::new(Mutex::new(None)));
        let repack = menu
            .entry
            .iter()
            .position(|entry| matches!(entry, ConfigEntry::Item(item) if item.status == get_text("upload_processing_repack_zip-status")))
            .unwrap();
        let ConfigEntry::Item(compression) = &menu.entry[repack + 1] else {
            panic!("compression must immediately follow ZIP repacking");
        };
        assert_eq!(compression.status, get_text("upload_processing_compression-status"));
        assert!(matches!(&compression.value, ListValue::U32(_, 0, 9)));
    }

    #[test]
    fn scanner_arguments_reject_invalid_placeholders_without_changing_config() {
        let board = Arc::new(Mutex::new(IcyBoard::default()));
        let validation_error = Arc::new(Mutex::new(None));
        let menu = UploadProcessing::build_menu(board.clone(), validation_error.clone());
        let item = (0..menu.count())
            .map(|index| menu.get_item(index).unwrap())
            .find(|item| item.status == get_text("upload_processing_scanner_arguments-status"))
            .unwrap();
        let original = board.lock().unwrap().config.upload_processing.scanner.arguments.clone();
        for invalid in ["", "--no-summary", "--file={file}", "{file}; {file}"] {
            item.update_value.as_ref().unwrap()(&board, &ListValue::Text(1024, TextFlags::None, invalid.to_string()));
            assert_eq!(board.lock().unwrap().config.upload_processing.scanner.arguments, original);
            assert_eq!(*validation_error.lock().unwrap(), Some(get_text("upload_processing_scanner_arguments-invalid")));
        }
        item.update_value.as_ref().unwrap()(&board, &ListValue::Text(1024, TextFlags::None, " --no-summary ; {file} ".to_string()));
        assert_eq!(board.lock().unwrap().config.upload_processing.scanner.arguments, vec!["--no-summary", "{file}"]);
        assert!(validation_error.lock().unwrap().is_none());
    }

    #[test]
    fn catalogs_have_no_obsolete_cleanup_keys() {
        for catalog in [
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../icy_board_tui/i18n/en/icy_board_tui.ftl")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../icy_board_tui/i18n/de/icy_board_tui.ftl")),
        ] {
            for retired in ["clean_descriptions", "clean_comments", "clean_passes", "additions"] {
                assert!(!catalog.lines().any(|line| line.starts_with(&format!("upload_processing_{retired}"))));
            }
            for suffix in ["", "-status", "-help"] {
                assert_eq!(
                    catalog
                        .lines()
                        .filter(|line| line.starts_with(&format!("upload_processing_advertisement_file{suffix}=")))
                        .count(),
                    1
                );
            }
            for group in ["publication", "advertising", "zip", "scanner", "limits"] {
                assert_eq!(
                    catalog
                        .lines()
                        .filter(|line| line.starts_with(&format!("upload_processing_group_{group}=")))
                        .count(),
                    1
                );
            }
        }
    }
}
