use icy_net::ConnectionType;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use super::state::NodeState;

#[derive(Clone, Debug, PartialEq)]
pub enum BBSMessage {
    SysopLogin,
    SysopLogout,
    Broadcast(String),
}

pub struct BBS {
    pub open_connections: Arc<Mutex<Vec<Option<NodeState>>>>,
    pub bbs_channels: Vec<Option<tokio::sync::mpsc::Sender<BBSMessage>>>,
}

impl BBS {
    pub async fn clear_closed_connections(&mut self) {
        let list = &mut self.open_connections.lock().await;
        for i in 0..list.len() {
            let is_finished = if let Some(state) = &list[i] {
                if let Some(handle) = &state.handle {
                    handle.is_finished()
                } else {
                    true
                }
            } else {
                continue;
            };
            if is_finished {
                list[i] = None;
            }
        }
    }

    pub async fn get_open_connections(&mut self) -> &Arc<Mutex<Vec<Option<NodeState>>>> {
        &self.open_connections
    }

    pub async fn create_new_node(&mut self, connection_type: ConnectionType) -> usize {
        self.clear_closed_connections().await;
        let mut list = self.open_connections.lock().await;
        for i in 0..list.len() {
            if list[i].is_none() {
                let (tx, rx) = mpsc::channel(32);
                let node_state = NodeState::new(i + 1, connection_type, rx);
                list[i] = Some(node_state);
                self.bbs_channels[i] = Some(tx);
                return i;
            }
        }
        0
    }

    pub fn new(nodes: usize) -> BBS {
        let mut vec = Vec::new();
        let mut vec2 = Vec::new();
        for _ in 0..nodes {
            vec.push(None);
            vec2.push(None);
        }
        BBS {
            open_connections: Arc::new(Mutex::new(vec)),
            bbs_channels: vec2,
        }
    }
}
