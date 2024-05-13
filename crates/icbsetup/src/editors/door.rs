use crossterm::event::{KeyCode, KeyEvent};
use icy_board_engine::{
    icy_board::{
        doors::{BBSLink, Door, DoorList, DoorServerAccount, DoorType},
        security::RequiredSecurity,
        IcyBoardSerializer,
    },
    Res,
};
use icy_board_tui::{
    config_menu::{ConfigEntry, ConfigMenu, ConfigMenuState, ListItem, ListValue},
    tab_page::Editor,
};
use ratatui::{layout::Rect, Frame};

pub struct DoorEditor {
    path: std::path::PathBuf,
    door_list: DoorList,
    menu: ConfigMenu,
    menu_state: ConfigMenuState,
}
impl DoorEditor {
    pub(crate) fn new(path: &std::path::PathBuf) -> Res<Self> {
        let mut door_list = if path.exists() {
            DoorList::load(&path)?
        } else {
            let mut door_list = DoorList::default();
            door_list.accounts.push(DoorServerAccount::BBSLink(BBSLink::default()));
            door_list
        };

        if door_list.accounts.is_empty() {
            door_list.accounts.push(DoorServerAccount::BBSLink(BBSLink::default()));
        }

        let DoorServerAccount::BBSLink(bbs_link) = &door_list.accounts[0];
        let items = vec![
            ConfigEntry::Item(ListItem::new(
                "system_code",
                "System Code".to_string(),
                ListValue::Text(25, bbs_link.system_code.clone()),
            )),
            ConfigEntry::Item(ListItem::new(
                "auth_code",
                "Auth Code".to_string(),
                ListValue::Text(25, bbs_link.auth_code.clone()),
            )),
            ConfigEntry::Item(ListItem::new(
                "sheme_code",
                "Scheme Code".to_string(),
                ListValue::Text(25, bbs_link.sheme_code.clone()),
            )),
        ];

        let menu = ConfigMenu { entry: items };

        Ok(Self {
            path: path.clone(),
            door_list,
            menu,
            menu_state: ConfigMenuState::default(),
        })
    }
}

impl Editor for DoorEditor {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.menu.render(area, frame, &mut self.menu_state);
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                for item in self.menu.iter() {
                    if let ListValue::Text(_, value) = &item.value {
                        match item.id.as_str() {
                            "system_code" => {
                                let DoorServerAccount::BBSLink(bbs_link) = &mut self.door_list.accounts[0];
                                bbs_link.system_code = value.clone();
                            }
                            "auth_code" => {
                                let DoorServerAccount::BBSLink(bbs_link) = &mut self.door_list.accounts[0];
                                bbs_link.auth_code = value.clone();
                            }
                            "sheme_code" => {
                                let DoorServerAccount::BBSLink(bbs_link) = &mut self.door_list.accounts[0];
                                bbs_link.sheme_code = value.clone();
                            }
                            _ => {}
                        }
                    }
                }
                self.door_list.save(&self.path).unwrap();
                return false;
            }
            KeyCode::Insert => {
                self.door_list.doors.push(Door {
                    name: format!("door{}", self.door_list.len() + 1),
                    description: "".to_string(),
                    password: "".to_string(),
                    securiy_level: RequiredSecurity::new(0),
                    use_shell_execute: false,
                    door_type: DoorType::BBSlink,
                    path: "".to_string(),
                    drop_file: Default::default(),
                });
            }
            _ => {
                self.menu.handle_key_press(key, &mut self.menu_state);
            }
        }
        true
    }
}
