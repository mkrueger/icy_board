use std::collections::{HashMap, HashSet};

use crate::icy_board::state::user_commands::groupchat::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ChatParticipant {
    pub node: usize,
    pub handle: String,
    pub is_owner: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ChatRoom {
    pub topic: Option<String>,
    pub private: bool,
    owner: Option<usize>,
    pub participants: HashSet<usize>,
}

#[derive(Clone, Debug)]
pub struct GroupChatState {
    pub rooms: Vec<ChatRoom>,
    user_rooms: HashMap<usize, u8>,
    user_handles: HashMap<usize, String>,
    monitors: HashMap<usize, HashSet<u8>>,
}
/*
#[derive(Clone, Debug)]
pub struct GroupChatPreferences {
    pub current_room: Option<u8>,
    pub echo: bool,
    pub expert: bool,
    pub silent: bool,
    pub handle: String,
    pub ignore_handles: HashSet<String>,
    pub monitor_rooms: HashSet<u8>,
}*/

const MAX_CHAT_ROOMS: usize = 256;

impl GroupChatState {
    pub fn new() -> Self {
        Self {
            rooms: (0..MAX_CHAT_ROOMS).map(|_| ChatRoom::default()).collect(),
            user_rooms: HashMap::new(),
            user_handles: HashMap::new(),
            monitors: HashMap::new(),
        }
    }

    pub fn current_room(&self, node: usize) -> Option<u8> {
        self.user_rooms.get(&node).copied()
    }

    fn validate_room(room: u8) -> Result<usize, GroupChatError> {
        if room == 0 || room as usize >= MAX_CHAT_ROOMS {
            return Err(GroupChatError::InvalidRoom(room));
        }
        Ok(room as usize)
    }

    fn display_handle(&self, node: usize) -> String {
        self.user_handles.get(&node).cloned().unwrap_or_else(|| format!("Node {}", node))
    }

    fn find_node_by_handle(&self, handle: &str) -> Option<usize> {
        let needle = handle.to_ascii_uppercase();
        self.user_handles
            .iter()
            .find(|(_, value)| value.to_ascii_uppercase() == needle)
            .map(|(node, _)| *node)
    }

    fn broadcast_room_event(
        &self,
        room_idx: usize,
        exclude: Option<usize>,
        kind: GroupChatEventKind,
        from_node: Option<usize>,
        from_handle: Option<String>,
        text: String,
    ) -> Vec<GroupChatEvent> {
        let skip = exclude.unwrap_or(usize::MAX);
        let mut events = Vec::new();
        let room_id = room_idx as u8;

        if let Some(room) = self.rooms.get(room_idx) {
            for node in &room.participants {
                if *node == skip {
                    continue;
                }
                events.push(GroupChatEvent {
                    target_node: *node,
                    kind: kind.clone(),
                    room: Some(room_id),
                    from_node,
                    from_handle: from_handle.clone(),
                    text: text.clone(),
                });
            }
        }

        for (monitor_node, rooms) in &self.monitors {
            if rooms.contains(&room_id) && *monitor_node != skip && self.rooms.get(room_idx).map_or(false, |room| !room.participants.contains(monitor_node)) {
                events.push(GroupChatEvent {
                    target_node: *monitor_node,
                    kind: kind.clone(),
                    room: Some(room_id),
                    from_node,
                    from_handle: from_handle.clone(),
                    text: text.clone(),
                });
            }
        }

        events
    }

    pub fn join_room(&mut self, node: usize, handle: &str, room: u8) -> Result<(u8, Vec<GroupChatEvent>), GroupChatError> {
        let room_idx = Self::validate_room(room)?;
        let mut events = Vec::new();
        let current_room = self.current_room(node);

        if current_room == Some(room) {
            if self.user_handles.get(&node).map(|h| !h.eq_ignore_ascii_case(handle)).unwrap_or(true) {
                events.extend(self.update_handle(node, handle)?);
            }
            return Ok((room, events));
        }

        if current_room.is_some() {
            events.extend(self.leave_room(node));
        }

        self.user_handles.insert(node, handle.to_string());

        {
            let room_ref = self.rooms.get(room_idx).ok_or(GroupChatError::InvalidRoom(room))?;
            if room_ref.private && room_ref.owner != Some(node) && !room_ref.participants.contains(&node) && !room_ref.participants.is_empty() {
                return Err(GroupChatError::RoomIsPrivate(room));
            }
        }

        {
            let room_ref = self.rooms.get_mut(room_idx).ok_or(GroupChatError::InvalidRoom(room))?;
            room_ref.participants.insert(node);
            if room_ref.owner.is_none() {
                room_ref.owner = Some(node);
            }
        }

        self.user_rooms.insert(node, room);

        let join_message = format!("{} joined channel {}", handle, room);
        events.extend(self.broadcast_room_event(
            room_idx,
            Some(node),
            GroupChatEventKind::UserJoined,
            Some(node),
            Some(handle.to_string()),
            join_message,
        ));

        if let Some(topic) = self.rooms[room_idx].topic.clone() {
            events.push(GroupChatEvent {
                target_node: node,
                kind: GroupChatEventKind::TopicChanged,
                room: Some(room),
                from_node: self.rooms[room_idx].owner,
                from_handle: self.rooms[room_idx].owner.and_then(|owner| self.user_handles.get(&owner).cloned()),
                text: topic,
            });
        }

        events.push(GroupChatEvent {
            target_node: node,
            kind: GroupChatEventKind::SystemMessage,
            room: Some(room),
            from_node: None,
            from_handle: None,
            text: format!("Joined channel {}", room),
        });

        Ok((room, events))
    }

    pub fn leave_room(&mut self, node: usize) -> Vec<GroupChatEvent> {
        let Some(room) = self.user_rooms.remove(&node) else {
            return Vec::new();
        };
        let room_idx = room as usize;
        let handle = self.display_handle(node);
        let mut new_owner = None;

        if let Some(room_ref) = self.rooms.get_mut(room_idx) {
            room_ref.participants.remove(&node);
            if room_ref.owner == Some(node) {
                new_owner = room_ref.participants.iter().next().copied();
                room_ref.owner = new_owner;
            }
            if room_ref.participants.is_empty() {
                room_ref.topic = None;
                room_ref.private = false;
                room_ref.owner = None;
            }
        }

        let mut events = self.broadcast_room_event(
            room_idx,
            Some(node),
            GroupChatEventKind::UserLeft,
            Some(node),
            Some(handle.clone()),
            format!("{} left channel {}", handle, room),
        );

        if let Some(owner) = new_owner {
            let owner_handle = self.display_handle(owner);
            events.extend(self.broadcast_room_event(
                room_idx,
                None,
                GroupChatEventKind::SystemMessage,
                None,
                None,
                format!("{} is now the host of channel {}", owner_handle, room),
            ));
        }

        events
    }

    pub fn send_public_message(&self, node: usize, text: &str) -> Result<Vec<GroupChatEvent>, GroupChatError> {
        if text.trim().is_empty() {
            return Err(GroupChatError::EmptyMessage);
        }
        let room = self.current_room(node).ok_or(GroupChatError::NotInRoom)?;
        let room_idx = Self::validate_room(room)?;
        let handle = self.display_handle(node);

        Ok(self.broadcast_room_event(room_idx, None, GroupChatEventKind::UserMessage, Some(node), Some(handle), text.to_string()))
    }

    pub fn send_private_message(&self, node: usize, target_handle: &str, message: &str) -> Result<Vec<GroupChatEvent>, GroupChatError> {
        if message.trim().is_empty() {
            return Err(GroupChatError::EmptyMessage);
        }

        let room = self.current_room(node).ok_or(GroupChatError::NotInRoom)?;
        let target_node = self
            .find_node_by_handle(target_handle)
            .ok_or_else(|| GroupChatError::UserNotFound(target_handle.to_string()))?;

        if self.current_room(target_node) != Some(room) {
            return Err(GroupChatError::UserNotInRoom(target_handle.to_string()));
        }

        let mut events = Vec::new();
        let sender_handle = self.display_handle(node);
        let target_actual = self.display_handle(target_node);

        events.push(GroupChatEvent {
            target_node,
            kind: GroupChatEventKind::PrivateMessage,
            room: Some(room),
            from_node: Some(node),
            from_handle: Some(sender_handle.clone()),
            text: message.to_string(),
        });

        events.push(GroupChatEvent {
            target_node: node,
            kind: GroupChatEventKind::PrivateMessage,
            room: Some(room),
            from_node: Some(node),
            from_handle: Some(sender_handle),
            text: format!("(to {}) {}", target_actual, message),
        });

        Ok(events)
    }

    pub fn set_topic(&mut self, node: usize, topic: Option<String>) -> Result<Vec<GroupChatEvent>, GroupChatError> {
        let room = self.current_room(node).ok_or(GroupChatError::NotInRoom)?;
        let room_idx = Self::validate_room(room)?;

        {
            let room_ref = self.rooms.get_mut(room_idx).ok_or(GroupChatError::InvalidRoom(room))?;
            if room_ref.owner != Some(node) {
                return Err(GroupChatError::NotOwner);
            }
            room_ref.topic = topic.clone();
        }

        let text = topic.map(|t| format!("Topic is now: {}", t)).unwrap_or_else(|| "Topic cleared.".to_string());

        Ok(self.broadcast_room_event(
            room_idx,
            None,
            GroupChatEventKind::TopicChanged,
            Some(node),
            Some(self.display_handle(node)),
            text,
        ))
    }

    pub fn set_private(&mut self, node: usize, private: bool) -> Result<Vec<GroupChatEvent>, GroupChatError> {
        let room = self.current_room(node).ok_or(GroupChatError::NotInRoom)?;
        let room_idx = Self::validate_room(room)?;

        {
            let room_ref = self.rooms.get_mut(room_idx).ok_or(GroupChatError::InvalidRoom(room))?;
            if room_ref.owner != Some(node) {
                return Err(GroupChatError::NotOwner);
            }
            room_ref.private = private;
        }

        let text = if private {
            format!("Channel {} is now private.", room)
        } else {
            format!("Channel {} is now public.", room)
        };

        Ok(self.broadcast_room_event(
            room_idx,
            None,
            GroupChatEventKind::RoomModeChanged,
            Some(node),
            Some(self.display_handle(node)),
            text,
        ))
    }

    pub fn list_participants(&self, room: u8) -> Result<Vec<ChatParticipant>, GroupChatError> {
        let room_idx = Self::validate_room(room)?;
        let mut list: Vec<_> = self.rooms[room_idx]
            .participants
            .iter()
            .map(|node| ChatParticipant {
                node: *node,
                handle: self.display_handle(*node),
                is_owner: self.rooms[room_idx].owner == Some(*node),
            })
            .collect();
        list.sort_by_key(|entry| entry.node);
        Ok(list)
    }

    pub fn update_handle(&mut self, node: usize, new_handle: &str) -> Result<Vec<GroupChatEvent>, GroupChatError> {
        let previous = self.user_handles.insert(node, new_handle.to_string());
        if previous.as_ref().map(|old| old.eq_ignore_ascii_case(new_handle)).unwrap_or(false) {
            return Ok(Vec::new());
        }

        let room = match self.current_room(node) {
            Some(room) => room,
            None => return Ok(Vec::new()),
        };
        let room_idx = Self::validate_room(room)?;

        Ok(self.broadcast_room_event(
            room_idx,
            None,
            GroupChatEventKind::SystemMessage,
            None,
            None,
            format!("{} is now known as {}", previous.unwrap_or_else(|| format!("Node {}", node)), new_handle),
        ))
    }

    pub fn call_handle(&self, node: usize, target_handle: &str) -> Result<Vec<GroupChatEvent>, GroupChatError> {
        let room = self.current_room(node).ok_or(GroupChatError::NotInRoom)?;
        let target_node = self
            .find_node_by_handle(target_handle)
            .ok_or_else(|| GroupChatError::UserNotFound(target_handle.to_string()))?;

        let caller_handle = self.display_handle(node);
        let mut events = Vec::new();

        events.push(GroupChatEvent {
            target_node,
            kind: GroupChatEventKind::Call,
            room: Some(room),
            from_node: Some(node),
            from_handle: Some(caller_handle.clone()),
            text: format!("{} invites you to channel {}", caller_handle, room),
        });

        let ack = if self.current_room(target_node) == Some(room) {
            format!("{} is already in channel {}", self.display_handle(target_node), room)
        } else {
            format!("Calling {} (node {})", self.display_handle(target_node), target_node)
        };

        events.push(GroupChatEvent {
            target_node: node,
            kind: GroupChatEventKind::SystemMessage,
            room: Some(room),
            from_node: None,
            from_handle: None,
            text: ack,
        });

        Ok(events)
    }

    pub fn toggle_monitor(&mut self, node: usize, room: u8) -> Result<(bool, Vec<GroupChatEvent>), GroupChatError> {
        let room_idx = Self::validate_room(room)?;
        let entry = self.monitors.entry(node).or_default();
        let active = if entry.contains(&room) {
            entry.remove(&room);
            false
        } else {
            entry.insert(room);
            true
        };

        if entry.is_empty() {
            self.monitors.remove(&node);
        }

        let mut events = Vec::new();
        let note = if active {
            format!("Monitoring channel {}", room)
        } else {
            format!("Stopped monitoring channel {}", room)
        };

        events.push(GroupChatEvent {
            target_node: node,
            kind: GroupChatEventKind::SystemMessage,
            room: Some(room),
            from_node: None,
            from_handle: None,
            text: note,
        });

        if active {
            if let Some(topic) = self.rooms[room_idx].topic.clone() {
                events.push(GroupChatEvent {
                    target_node: node,
                    kind: GroupChatEventKind::TopicChanged,
                    room: Some(room),
                    from_node: self.rooms[room_idx].owner,
                    from_handle: self.rooms[room_idx].owner.and_then(|owner| self.user_handles.get(&owner).cloned()),
                    text: topic,
                });
            }
        }

        Ok((active, events))
    }

    pub fn clear_monitoring(&mut self, node: usize) {
        self.monitors.remove(&node);
    }
}

impl Default for GroupChatState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for GroupChatPreferences {
    fn default() -> Self {
        Self {
            current_room: None,
            echo: true,
            expert: false,
            silent: false,
            handle: String::new(),
            ignore_handles: HashSet::new(),
            monitor_rooms: HashSet::new(),
        }
    }
}
