use icy_net::ConnectionType;
use std::{
    collections::{HashSet, VecDeque},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use tokio::sync::{Mutex, mpsc};

use crate::icy_board::state::user_commands::groupchat::{GroupChatEvent, GroupChatState};

use super::state::NodeState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SysopCommand {
    TogglePrivileges,
    LockCaller,
    TogglePageBell,
    ToggleAlarm,
    DisconnectCaller,
    DecreaseTime,
    IncreaseTime,
    DecreaseSecurity,
    IncreaseSecurity,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BBSMessage {
    SysopLogin,
    SysopLogout,
    StartSysopChat,
    RunSysopFunctionKey(usize),
    RunSysopCommand(SysopCommand),
    Broadcast(String),
    /// Show the text and drop the caller - sent when an event is about to run.
    Shutdown(String),
    GroupChat(GroupChatEvent),
    InvalidateFileBase(PathBuf),
}

/// Read-only UI snapshot, published by the event scheduler and main's listener restart helper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventMaintenanceStatus {
    pub description: String,
    pub phase: EventMaintenancePhase,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventMaintenancePhase {
    Waiting(String),
    Draining(usize),
    Stopping,
    Running,
    Reloading,
    ReloadFailed(String),
    /// Listener preparation retries with admission closed and BoardLock retained;
    /// neither the event command nor the configuration reload is repeated.
    ListenerFailed(String),
    Restarting,
}

/// Online work never owns the admission gate or the maintenance restart handshake.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnlineEventStatus {
    pub event_id: String,
    pub description: String,
    pub started: chrono::DateTime<chrono::Utc>,
    pub log_file: Option<String>,
    pub running_long: bool,
}

/// Read-only listener diagnostics, never an admission gate or restart handshake.
/// The address is the actual bound endpoint, including assigned port/fallback.
/// Stopped entries retain their last bound address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListenerStatus {
    pub name: String,
    pub address: SocketAddr,
    pub running: bool,
    /// UTC time of the actual bound/running or stopped transition, not a UI refresh.
    pub changed_at: chrono::DateTime<chrono::Utc>,
}

pub struct BBS {
    /// Runtime uptime origin, preserved across listener/configuration restarts.
    pub started_at: Instant,
    /// Read-only diagnostics: never gates admission or acknowledges maintenance.
    /// Replaced atomically only after every enabled listener is prepared.
    pub runtime_listeners: Vec<ListenerStatus>,
    pub open_connections: Arc<Mutex<Vec<Option<NodeState>>>>,
    pub bbs_channels: Vec<Option<tokio::sync::mpsc::Sender<BBSMessage>>>,
    pub group_chat: Arc<Mutex<GroupChatState>>,
    /// Scheduler-owned occurrence: sessions must prefer this to next_window(now).
    pub event_window: Option<super::events::EventWindow>,
    /// Admission is closed from fixed suspension / slide due time until restart completes.
    pub event_maintenance: bool,
    /// Progress only; never use this snapshot to decide admission or release the gate.
    pub event_maintenance_status: Option<EventMaintenanceStatus>,
    /// Call-wait/main must stop and join listeners, restart them, then clear this flag.
    pub event_restart_requested: bool,
    /// Main acknowledges listener termination here before the command may start.
    pub event_listeners_stopped: bool,
    /// UI enqueues stable IDs after confirmation. Prefer request_event_run().
    pub event_run_requests: VecDeque<String>,
    /// Scheduler-owned IDs of accepted manual requests and running commands.
    pub event_active_ids: HashSet<String>,
    pub event_online_status: Option<OnlineEventStatus>,
    /// Sticky journal/runtime error. Restart/repair required; admission stays shut.
    pub event_scheduler_error: Option<String>,
    /// Last rejected manual request (unknown ID, duplicate or invalid record).
    pub event_request_error: Option<String>,
    /// Offline board tools use the same admission interlock, without owning the scheduler gate.
    pub operator_maintenance: bool,
}

impl BBS {
    /// Does not validate existence: scheduler revalidates against the latest board.
    /// Disabled events and global scheduling-off are allowed for explicit manual
    /// requests. Day/start/end restrictions are overridden, mode/execution are not.
    pub fn request_event_run(&mut self, id: String) -> Result<(), String> {
        if id.is_empty() || self.event_scheduler_error.is_some() {
            return Err("Invalid event ID or event scheduler requires repair".into());
        }
        if self.event_active_ids.contains(&id) || self.event_run_requests.contains(&id) {
            return Err("Event is already queued or active".into());
        }
        if self.event_run_requests.len() >= 128 {
            return Err("Event request queue is full".into());
        }
        self.event_run_requests.push_back(id);
        Ok(())
    }

    pub async fn invalidate_file_base(&self, path: PathBuf) {
        for channel in self.bbs_channels.iter().flatten() {
            let _ = channel.send(BBSMessage::InvalidateFileBase(path.clone())).await;
        }
    }

    pub async fn clear_closed_connections(&mut self) {
        let list = &mut self.open_connections.lock().await;
        for i in 0..list.len() {
            let is_finished = if let Some(state) = &list[i] {
                // A reserved node without its handle installed is NOT an empty node.
                match state.handle.as_ref() {
                    Some(handle) => handle.is_finished(),
                    // Legacy owners may take/join a thread themselves. A node
                    // with its receiver still present is an unstarted reservation.
                    None => state.bbs_channel.is_none() && self.bbs_channels[i].as_ref().is_some_and(mpsc::Sender::is_closed),
                }
            } else {
                continue;
            };
            if is_finished {
                if let Some(mut state) = list[i].take() {
                    if let Some(handle) = state.handle.take() {
                        // is_finished includes thread-local destruction and session saves.
                        if handle.join().is_err() {
                            log::error!("Node {} panicked", i + 1);
                        }
                    }
                }
                self.bbs_channels[i] = None;
            }
        }
    }

    pub fn get_open_connections(&mut self) -> &Arc<Mutex<Vec<Option<NodeState>>>> {
        &self.open_connections
    }

    pub async fn create_new_node(&mut self, connection_type: ConnectionType) -> usize {
        self.try_create_new_node(connection_type).await.expect("No free node or board in maintenance")
    }

    pub fn admissions_closed(&self) -> bool {
        self.event_maintenance || self.operator_maintenance
    }

    pub async fn try_create_new_node(&mut self, connection_type: ConnectionType) -> Option<usize> {
        if self.admissions_closed() {
            return None;
        }
        self.clear_closed_connections().await;
        let mut list = self.open_connections.lock().await;
        for i in 0..list.len() {
            if list[i].is_none() {
                let (tx, rx) = mpsc::channel(32);
                let node_state = NodeState::new(i + 1, connection_type, rx);
                list[i] = Some(node_state);
                self.bbs_channels[i] = Some(tx);
                return Some(i);
            }
        }
        None
    }

    /// Allocate and install a thread atomically under the BBS/node locks. No await
    /// after reservation: listener cancellation cannot strand a half-created node.
    pub async fn spawn_node<F>(&mut self, connection_type: ConnectionType, spawn: F) -> std::io::Result<Option<usize>>
    where
        F: FnOnce(usize, &mut NodeState) -> std::io::Result<std::thread::JoinHandle<crate::Res<()>>>,
    {
        if self.admissions_closed() {
            return Ok(None);
        }
        self.clear_closed_connections().await;
        let mut list = self.open_connections.lock().await;
        let Some(node) = list.iter().position(Option::is_none) else {
            return Ok(None);
        };
        let (tx, rx) = mpsc::channel(32);
        let mut state = NodeState::new(node + 1, connection_type, rx);
        state.handle = Some(spawn(node, &mut state)?);
        list[node] = Some(state);
        self.bbs_channels[node] = Some(tx);
        Ok(Some(node))
    }

    /// Caller must hold the maintenance gate. Never discard live thread handles.
    pub async fn resize_idle_nodes(&mut self, nodes: usize) -> bool {
        self.clear_closed_connections().await;
        let mut list = self.open_connections.lock().await;
        if !self.admissions_closed() || list.iter().any(Option::is_some) {
            return false;
        }
        list.resize_with(nodes, || None);
        self.bbs_channels.resize_with(nodes, || None);
        true
    }

    pub fn new(nodes: usize) -> BBS {
        let mut vec = Vec::new();
        let mut vec2 = Vec::new();
        for _ in 0..nodes {
            vec.push(None);
            vec2.push(None);
        }
        BBS {
            started_at: Instant::now(),
            runtime_listeners: Vec::new(),
            open_connections: Arc::new(Mutex::new(vec)),
            bbs_channels: vec2,
            group_chat: Arc::new(Mutex::new(GroupChatState::default())),
            event_window: None,
            event_maintenance: false,
            event_maintenance_status: None,
            event_restart_requested: false,
            event_listeners_stopped: false,
            event_run_requests: VecDeque::new(),
            event_active_ids: HashSet::new(),
            event_online_status: None,
            event_scheduler_error: None,
            event_request_error: None,
            operator_maintenance: false,
        }
    }
}

#[cfg(test)]
mod event_lifecycle_tests {
    use super::*;

    #[test]
    fn listener_diagnostics_never_gate_admission() {
        let mut bbs = BBS::new(1);
        assert!(bbs.runtime_listeners.is_empty());
        assert!(!bbs.admissions_closed());
        bbs.runtime_listeners.push(ListenerStatus {
            name: "Telnet".into(),
            address: "127.0.0.1:12345".parse().unwrap(),
            running: false,
            changed_at: chrono::Utc::now(),
        });
        assert!(!bbs.admissions_closed());
        bbs.event_maintenance = true;
        bbs.runtime_listeners[0].running = true;
        assert!(bbs.admissions_closed());
        bbs.event_maintenance = false;
        bbs.operator_maintenance = true;
        bbs.runtime_listeners.clear();
        assert!(bbs.admissions_closed());
    }

    #[test]
    fn manual_event_queue_refuses_duplicates_active_requests_and_failed_scheduler() {
        let mut bbs = BBS::new(1);
        bbs.request_event_run("stable-id".into()).unwrap();
        assert!(bbs.request_event_run("stable-id".into()).is_err());
        assert_eq!(bbs.event_run_requests.pop_front().as_deref(), Some("stable-id"));
        bbs.event_active_ids.insert("stable-id".into());
        assert!(bbs.request_event_run("stable-id".into()).is_err());
        bbs.event_scheduler_error = Some("journal failure".into());
        assert!(bbs.request_event_run("other-id".into()).is_err());
        assert!(bbs.request_event_run(String::new()).is_err());
    }

    #[tokio::test]
    async fn reservations_are_live_and_full_board_never_overwrites_node_zero() {
        let mut bbs = BBS::new(1);
        assert_eq!(bbs.try_create_new_node(ConnectionType::Channel).await, Some(0));
        bbs.clear_closed_connections().await;
        assert!(bbs.open_connections.lock().await[0].is_some());
        assert_eq!(bbs.try_create_new_node(ConnectionType::Channel).await, None);
        bbs.event_maintenance = true;
        assert!(!bbs.resize_idle_nodes(2).await);
    }

    #[tokio::test]
    async fn maintenance_blocks_every_admission_and_idle_resize_preserves_shared_nodes() {
        let mut bbs = BBS::new(1);
        let nodes = bbs.open_connections.clone();
        bbs.event_maintenance = true;
        for connection in [ConnectionType::Telnet, ConnectionType::SSH, ConnectionType::Channel] {
            assert!(bbs.spawn_node(connection, |_, _| panic!("maintenance must not spawn")).await.unwrap().is_none());
        }
        assert!(bbs.resize_idle_nodes(3).await);
        assert_eq!(nodes.lock().await.len(), 3);
        assert!(Arc::ptr_eq(&nodes, &bbs.open_connections));
        bbs.event_maintenance = false;
        bbs.operator_maintenance = true;
        assert!(bbs.try_create_new_node(ConnectionType::Channel).await.is_none());
        bbs.operator_maintenance = false;
        assert_eq!(bbs.try_create_new_node(ConnectionType::Channel).await, Some(0));
    }

    #[tokio::test]
    async fn failed_spawn_releases_slot_and_running_thread_must_finish_before_resize() {
        let mut bbs = BBS::new(1);
        assert!(
            bbs.spawn_node(ConnectionType::Channel, |_, _| Err(std::io::Error::other("spawn failed")))
                .await
                .is_err()
        );
        assert!(bbs.open_connections.lock().await[0].is_none());
        let (finish, wait) = std::sync::mpsc::channel();
        bbs.spawn_node(ConnectionType::Channel, |_, _| {
            std::thread::Builder::new().spawn(move || {
                wait.recv().unwrap();
                Ok(())
            })
        })
        .await
        .unwrap()
        .unwrap();
        bbs.event_maintenance = true;
        assert!(!bbs.resize_idle_nodes(2).await);
        finish.send(()).unwrap();
        while !bbs.open_connections.lock().await[0].as_ref().unwrap().handle.as_ref().unwrap().is_finished() {
            tokio::task::yield_now().await;
        }
        assert!(bbs.resize_idle_nodes(2).await);
        assert!(bbs.open_connections.lock().await.iter().all(Option::is_none));
        assert!(bbs.bbs_channels.iter().all(Option::is_none));
    }
}
