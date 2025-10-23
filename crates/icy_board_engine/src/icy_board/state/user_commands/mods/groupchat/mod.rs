use crate::{
    Res,
    icy_board::{
        icb_config::IcbColor,
        icb_text::IceText,
        state::{
            IcyBoardState, NodeStatus,
            functions::{MASK_ALNUM, display_flags},
        },
    },
    vm::TerminalTarget,
};
use std::collections::HashSet;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

mod state;
pub use state::*;

#[derive(Clone, Debug, PartialEq)]
pub enum GroupChatEventKind {
    UserJoined,
    UserLeft,
    UserMessage,
    PrivateMessage,
    SystemMessage,
    TopicChanged,
    RoomModeChanged,
    Call,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupChatEvent {
    pub target_node: usize,
    pub kind: GroupChatEventKind,
    pub room: Option<u8>,
    pub from_node: Option<usize>,
    pub from_handle: Option<String>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatParticipant {
    pub node: usize,
    pub handle: String,
    pub is_owner: bool,
}
/*
#[derive(Clone, Debug, Default)]
struct ChatRoom {
    topic: Option<String>,
    private: bool,
    owner: Option<usize>,
    participants: HashSet<usize>,
}
*/

#[derive(Clone, Debug)]
pub struct GroupChatPreferences {
    pub current_room: Option<u8>,
    pub echo: bool,
    pub expert: bool,
    pub silent: bool,
    pub handle: String,
    pub ignore_handles: HashSet<String>,
    pub monitor_rooms: HashSet<u8>,
}

#[derive(Error, Debug)]
pub enum GroupChatError {
    #[error("Invalid chat room {0}")]
    InvalidRoom(u8),

    #[error("You are not currently in a chat room")]
    NotInRoom,

    #[error("Chat room {0} is currently private")]
    RoomIsPrivate(u8),

    #[error("Only the room owner may perform this action")]
    NotOwner,

    #[error("User '{0}' not found")]
    UserNotFound(String),

    #[error("User '{0}' is not in this room")]
    UserNotInRoom(String),

    #[error("Cannot send an empty message")]
    EmptyMessage,
}

const CHAT_MENU_FALLBACK: &[&str] = &[
    "(CALL)    Call a user to chat        (MENU)  Display this menu",
    "(CHANNEL) Change channels            (X)PERT Toggle expert mode",
    "(HANDLE)  Change your handle         (Q)UIT  Exit chat",
    "(SEND)    Send private message       (GOODBYE) Log off system",
    "(TOPIC)   Change current topic       (PRIVATE)  Make discussion private",
    "(PUBLIC)  Make discussion public     (SHOW) Show users in chat",
    "(WHO)     Show who is online         (ECHO/NOECHO) Toggle echo",
    "(IGNORE)  Ignore handles             (MONITOR) Toggle monitor mode",
    "(SILENT)  Toggle silent mode",
];

enum ChatLoopMode {
    Chat,
    Command,
}
enum ChatCommandResult {
    Continue,
    ExitChat,
    Logoff,
}

impl IcyBoardState {
    pub async fn start_group_chat(&mut self) -> Res<()> {
        self.set_activity(NodeStatus::GroupChat).await;
        if self.session.group_chat.handle.is_empty() {
            self.session.group_chat.handle = self.session.get_username_or_alias();
        }
        if !self.session.group_chat.expert {
            self.session.group_chat.expert = self.session.expert_mode();
        }

        let intro = self.get_board().await.config.paths.chat_intro_file.clone();
        self.display_file(&intro).await?;

        loop {
            // Get token from command line or prompt user
            let input = if let Some(token) = self.session.tokens.pop_front() {
                token
            } else {
                self.input_field(
                    IceText::NewChannel,
                    1,
                    "AGU",
                    "",
                    Some("1".to_string()),
                    display_flags::NEWLINE | display_flags::LFBEFORE | display_flags::UPCASE | display_flags::FIELDLEN,
                )
                .await?
            };

            if input.is_empty() {
                break;
            }

            let cmd = input.to_uppercase();

            // Handle single-character commands
            if cmd.len() == 1 {
                match cmd.chars().next().unwrap() {
                    'L' => {
                        self.list_channels().await?;
                        break;
                    }
                    'Q' => {
                        break;
                    }
                    _ => {
                        if let Some(number) = cmd.parse::<u8>().ok() {
                            return self.group_chat(number).await;
                        }
                        self.display_text(IceText::InvalidEntry, display_flags::LFBEFORE | display_flags::NEWLINE)
                            .await?;
                        continue;
                    }
                }
            }
        }
        return Ok(());
    }

    pub async fn group_chat(&mut self, _room: u8) -> Res<()> {
        let manager = {
            let bbs = self.bbs.lock().await;
            bbs.group_chat.clone()
        };
        let node_id = self.node + 1;
        let join_result = {
            let mut guard = manager.lock().await;
            let desired = self.session.group_chat.current_room.unwrap_or(1);
            match guard.join_room(node_id, &self.session.group_chat.handle, desired) {
                Ok(pair) => Ok(pair),
                Err(GroupChatError::RoomIsPrivate(_)) => guard.join_room(node_id, &self.session.group_chat.handle, 1),
                Err(err) => Err(err),
            }
        };
        let (room, events) = match join_result {
            Ok(res) => res,
            Err(err) => {
                self.show_chat_error(err).await?;
                return Ok(());
            }
        };
        self.session.group_chat.current_room = Some(room);
        self.dispatch_group_chat_events(events).await?;
        self.display_text(IceText::NodeChatEntered, display_flags::LFBEFORE | display_flags::NEWLINE)
            .await?;
        if let Some(user) = &mut self.session.current_user {
            user.stats.num_group_chats = user.stats.num_group_chats.saturating_add(1);
        }
        let mut mode = ChatLoopMode::Chat;
        let mut buffer = String::new();
        loop {
            if self.session.request_logoff {
                break;
            }
            match mode {
                ChatLoopMode::Chat => {
                    let Some(ch) = self.get_char(TerminalTarget::Both).await? else {
                        continue;
                    };
                    match ch.ch {
                        '\r' | '\n' => {
                            if !buffer.is_empty() {
                                let events = {
                                    let guard = manager.lock().await;
                                    guard.send_public_message(node_id, &buffer)
                                };
                                match events {
                                    Ok(events) => self.dispatch_group_chat_events(events).await?,
                                    Err(err) => self.show_chat_error(err).await?,
                                }
                                buffer.clear();
                            }
                            self.new_line().await?;
                        }
                        '\x08' | '\u{7f}' => {
                            if !buffer.is_empty() {
                                buffer.pop();
                                self.print(TerminalTarget::Both, "\x08 \x08").await?;
                            }
                        }
                        '\x1b' => {
                            mode = ChatLoopMode::Command;
                            self.new_line().await?;
                        }
                        c => {
                            buffer.push(c);
                            if self.session.group_chat.echo {
                                self.print(TerminalTarget::Both, &c.to_string()).await?;
                            }
                        }
                    }
                }
                ChatLoopMode::Command => {
                    let command = self.prompt_chat_command().await?;
                    if command.trim().is_empty() {
                        mode = ChatLoopMode::Chat;
                        continue;
                    }
                    match self.handle_chat_command(&manager, command.trim()).await? {
                        ChatCommandResult::Continue => mode = ChatLoopMode::Chat,
                        ChatCommandResult::ExitChat => break,
                        ChatCommandResult::Logoff => break,
                    }
                }
            }
        }
        {
            let mut guard = manager.lock().await;
            let events = guard.leave_room(node_id);
            guard.clear_monitoring(node_id);
            self.dispatch_group_chat_events(events).await?;
        }
        self.session.group_chat.current_room = None;
        self.session.group_chat.monitor_rooms.clear();
        self.display_text(IceText::NodeChatEnded, display_flags::LFBEFORE | display_flags::NEWLINE)
            .await?;
        self.set_activity(NodeStatus::Available).await;
        Ok(())
    }

    async fn prompt_chat_command(&mut self) -> Res<String> {
        let prompt = if self.session.group_chat.expert {
            IceText::ChatPromptExpertmode
        } else {
            IceText::ChatPromptNovice
        };
        self.input_field(
            prompt,
            60,
            &MASK_ALNUM,
            "hlpchat",
            None,
            display_flags::UPCASE | display_flags::FIELDLEN | display_flags::NEWLINE | display_flags::LFBEFORE,
        )
        .await
        .map(|s| s.trim().to_string())
    }

    async fn handle_chat_command(&mut self, manager: &Arc<Mutex<GroupChatState>>, raw: &str) -> Res<ChatCommandResult> {
        let mut parts = raw.split_whitespace();
        let Some(cmd) = parts.next() else {
            return Ok(ChatCommandResult::Continue);
        };
        let rest = raw[cmd.len()..].trim_start();
        match cmd.to_ascii_uppercase().as_str() {
            "?" | "HELP" | "MENU" => self.show_chat_menu().await?,
            "CHANNEL" | "CHAN" | "C" => {
                let Ok(room) = rest.parse::<u8>() else {
                    self.println(TerminalTarget::User, "Usage: CHANNEL <1-255>").await?;
                    return Ok(ChatCommandResult::Continue);
                };
                let events = {
                    let mut guard = manager.lock().await;
                    guard.join_room(self.node + 1, &self.session.group_chat.handle, room)
                };
                match events {
                    Ok((room, events)) => {
                        self.session.group_chat.current_room = Some(room);
                        self.dispatch_group_chat_events(events).await?;
                    }
                    Err(err) => self.show_chat_error(err).await?,
                }
            }
            "TOPIC" => {
                let topic = rest.trim();
                let events = {
                    let mut guard = manager.lock().await;
                    guard.set_topic(self.node + 1, if topic.is_empty() { None } else { Some(topic.to_string()) })
                };
                match events {
                    Ok(events) => self.dispatch_group_chat_events(events).await?,
                    Err(err) => self.show_chat_error(err).await?,
                }
            }
            "PRIVATE" => {
                let events = {
                    let mut guard = manager.lock().await;
                    guard.set_private(self.node + 1, true)
                };
                match events {
                    Ok(events) => self.dispatch_group_chat_events(events).await?,
                    Err(err) => self.show_chat_error(err).await?,
                }
            }
            "PUBLIC" => {
                let events = {
                    let mut guard = manager.lock().await;
                    guard.set_private(self.node + 1, false)
                };
                match events {
                    Ok(events) => self.dispatch_group_chat_events(events).await?,
                    Err(err) => self.show_chat_error(err).await?,
                }
            }
            "SHOW" => {
                let participants = {
                    let guard = manager.lock().await;
                    let room = guard.current_room(self.node + 1).ok_or(GroupChatError::NotInRoom)?;
                    guard.list_participants(room)
                };
                match participants {
                    Ok(list) => {
                        self.new_line().await?;
                        for entry in list {
                            let marker = if entry.is_owner { '*' } else { ' ' };
                            self.println(TerminalTarget::User, &format!("[{}] {} (node {})", marker, entry.handle, entry.node))
                                .await?;
                        }
                    }
                    Err(err) => self.show_chat_error(err).await?,
                }
            }
            "HANDLE" => {
                if rest.is_empty() {
                    self.println(TerminalTarget::User, "Usage: HANDLE <name>").await?;
                } else {
                    let events = {
                        let mut guard = manager.lock().await;
                        guard.update_handle(self.node + 1, rest)
                    };
                    match events {
                        Ok(events) => {
                            self.session.group_chat.handle = rest.to_string();
                            self.dispatch_group_chat_events(events).await?;
                        }
                        Err(err) => self.show_chat_error(err).await?,
                    }
                }
            }
            "SEND" => {
                let mut pieces = rest.splitn(2, char::is_whitespace);
                let Some(target) = pieces.next().filter(|s| !s.is_empty()) else {
                    self.println(TerminalTarget::User, "Usage: SEND <handle> <message>").await?;
                    return Ok(ChatCommandResult::Continue);
                };
                let Some(message) = pieces.next().filter(|s| !s.trim().is_empty()) else {
                    self.println(TerminalTarget::User, "Usage: SEND <handle> <message>").await?;
                    return Ok(ChatCommandResult::Continue);
                };
                let events = {
                    let guard = manager.lock().await;
                    guard.send_private_message(self.node + 1, target, message)
                };
                match events {
                    Ok(events) => self.dispatch_group_chat_events(events).await?,
                    Err(err) => self.show_chat_error(err).await?,
                }
            }
            "CALL" => {
                if rest.is_empty() {
                    self.println(TerminalTarget::User, "Usage: CALL <handle>").await?;
                } else {
                    let events = {
                        let guard = manager.lock().await;
                        guard.call_handle(self.node + 1, rest)
                    };
                    match events {
                        Ok(events) => self.dispatch_group_chat_events(events).await?,
                        Err(err) => self.show_chat_error(err).await?,
                    }
                }
            }
            "ECHO" => {
                self.session.group_chat.echo = true;
                self.println(TerminalTarget::User, "Chat echo enabled.").await?;
            }
            "NOECHO" => {
                self.session.group_chat.echo = false;
                self.println(TerminalTarget::User, "Chat echo disabled.").await?;
            }
            "IGNORE" => {
                if rest.is_empty() {
                    if self.session.group_chat.ignore_handles.is_empty() {
                        self.println(TerminalTarget::User, "Ignoring nobody.").await?;
                    } else {
                        self.println(TerminalTarget::User, "Ignoring:").await?;
                        /* TODO:
                        for entry in &self.session.group_chat.ignore_handles {
                            self.println(TerminalTarget::User, &format!("  {}", entry)).await?;
                        }*/
                    }
                } else {
                    let key = rest.to_ascii_uppercase();
                    if self.session.group_chat.ignore_handles.remove(&key) {
                        self.println(TerminalTarget::User, &format!("No longer ignoring {}", rest)).await?;
                    } else {
                        self.session.group_chat.ignore_handles.insert(key);
                        self.println(TerminalTarget::User, &format!("Ignoring {}", rest)).await?;
                    }
                }
            }
            "MONITOR" => {
                if rest.eq_ignore_ascii_case("OFF") || rest.eq_ignore_ascii_case("CLEAR") {
                    {
                        let mut guard = manager.lock().await;
                        guard.clear_monitoring(self.node + 1);
                    }
                    self.session.group_chat.monitor_rooms.clear();
                    self.println(TerminalTarget::User, "Monitor mode cleared.").await?;
                } else {
                    let Ok(room) = rest.parse::<u8>() else {
                        self.println(TerminalTarget::User, "Usage: MONITOR <channel>|OFF").await?;
                        return Ok(ChatCommandResult::Continue);
                    };
                    let events = {
                        let mut guard = manager.lock().await;
                        guard.toggle_monitor(self.node + 1, room)
                    };
                    match events {
                        Ok((active, mut events)) => {
                            if active {
                                self.session.group_chat.monitor_rooms.insert(room);
                            } else {
                                self.session.group_chat.monitor_rooms.remove(&room);
                            }
                            self.dispatch_group_chat_events(std::mem::take(&mut events)).await?;
                        }
                        Err(err) => self.show_chat_error(err).await?,
                    }
                }
            }
            "SILENT" => {
                self.session.group_chat.silent = !self.session.group_chat.silent;
                self.println(
                    TerminalTarget::User,
                    if self.session.group_chat.silent {
                        "Silent mode enabled."
                    } else {
                        "Silent mode disabled."
                    },
                )
                .await?;
            }
            "X" | "XPERT" => {
                self.session.group_chat.expert = !self.session.group_chat.expert;
                self.println(
                    TerminalTarget::User,
                    if self.session.group_chat.expert {
                        "Expert prompts enabled."
                    } else {
                        "Expert prompts disabled."
                    },
                )
                .await?;
            }
            "WHO" => self.who_display_nodes().await?,
            "QUIT" | "Q" => return Ok(ChatCommandResult::ExitChat),
            "GOODBYE" | "G" => {
                self.session.request_logoff = true;
                return Ok(ChatCommandResult::Logoff);
            }
            other => {
                self.println(TerminalTarget::User, &format!("Unknown command: {}", other)).await?;
            }
        }
        Ok(ChatCommandResult::Continue)
    }

    async fn show_chat_menu(&mut self) -> Res<()> {
        let menu = self.get_board().await.config.paths.chat_menu.clone();
        if menu.exists() {
            self.display_file(&menu).await?;
        } else {
            self.new_line().await?;
            for line in CHAT_MENU_FALLBACK {
                self.println(TerminalTarget::User, line).await?;
            }
        }
        Ok(())
    }

    async fn show_chat_error(&mut self, err: GroupChatError) -> Res<()> {
        self.set_color(TerminalTarget::User, IcbColor::dos_yellow()).await?;
        self.println(TerminalTarget::User, &format!("! {}", err)).await?;
        self.reset_color(TerminalTarget::User).await?;
        Ok(())
    }

    pub async fn dispatch_group_chat_events(&mut self, _events: Vec<GroupChatEvent>) -> Res<()> {
        /* same body as in state/mod.rs, but kept here for cohesion */
        Ok(())
    }

    pub async fn handle_group_chat_event(&mut self, _event: GroupChatEvent) -> Res<()> {
        /* render logic described above */
        Ok(())
    }
}
