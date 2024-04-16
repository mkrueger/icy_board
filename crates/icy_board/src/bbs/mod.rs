use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
    vec,
};

use icy_board_engine::{
    icy_board::{
        state::{IcyBoardState, NodeState},
        IcyBoard,
    },
    vm::TerminalTarget,
};
use icy_net::{telnet::TelnetConnection, Connection, ConnectionType};
use icy_ppe::Res;

use crate::menu_runner::PcbBoardCommand;

pub struct BBS {
    pub open_connections: Vec<Option<Arc<Mutex<NodeState>>>>,
}

impl BBS {
    fn clear_closed_connections(&mut self) {
        for i in 0..self.open_connections.len() {
            if let Some(node) = &self.open_connections[i] {
                if node.lock().unwrap().handle.is_none() {
                    self.open_connections[i] = None;
                    continue;
                }
                if node.lock().unwrap().handle.as_mut().unwrap().is_finished() {
                    self.open_connections[i] = None;
                    continue;
                }
            }
        }
    }

    pub fn get_open_connections(&mut self) -> &[Option<Arc<Mutex<NodeState>>>] {
        self.clear_closed_connections();
        &self.open_connections
    }

    pub fn get_node(&self, node: usize) -> &Option<Arc<Mutex<NodeState>>> {
        &self.open_connections[node]
    }

    pub fn create_new_node(&mut self, connection_type: ConnectionType) -> Arc<Mutex<NodeState>> {
        self.clear_closed_connections();
        for i in 0..self.open_connections.len() {
            if self.open_connections[i].is_none() {
                let node_state = Arc::new(Mutex::new(NodeState::new(i + 1, connection_type)));
                self.open_connections[i] = Some(node_state.clone());
                return node_state;
            }
        }
        panic!("Could not create new connection");
    }

    pub fn new(nodes: usize) -> BBS {
        BBS {
            open_connections: vec![None; nodes],
        }
    }
}

pub fn await_telnet_connections(board: Arc<Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>) -> Res<()> {
    let addr = "127.0.0.1:1337".to_string();
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(e) => panic!("could not read start TCP listener: {}", e),
    };
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let board = board.clone();
                let node_state = bbs.lock().unwrap().create_new_node(ConnectionType::Telnet);

                let node = node_state.clone();
                let handle = thread::spawn(move || {
                    let orig_hook = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |panic_info| {
                        log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                        log::error!("full info: {:?}", panic_info);
                        orig_hook(panic_info);
                    }));

                    let connection = TelnetConnection::accept(stream).unwrap();
                    // connection succeeded
                    if let Err(err) = handle_client(board, node_state, Box::new(connection)) {
                        log::error!("Error running backround client: {}", err);
                    }
                    Ok(())
                });
                node.lock().unwrap().handle = Some(handle);
            }
            Err(e) => {
                log::error!("connection failed {}", e);
            }
        }
    }
    drop(listener);
    Ok(())
}

fn handle_client(board: Arc<Mutex<IcyBoard>>, node_state: Arc<Mutex<NodeState>>, connection: Box<dyn Connection>) -> Res<()> {
    let state = IcyBoardState::new(board, node_state, connection);
    let mut cmd = PcbBoardCommand::new(state);

    match cmd.login() {
        Ok(true) => {}
        Ok(false) => {
            return Ok(());
        }
        Err(err) => {
            log::error!("error during login process {}", err);
            return Ok(());
        }
    }

    loop {
        if let Err(err) = cmd.do_command() {
            cmd.state.session.disp_options.reset_printout();

            // print error message to user, if possible
            if cmd.state.set_color(TerminalTarget::Both, 4.into()).is_ok() {
                cmd.state
                    .print(icy_board_engine::vm::TerminalTarget::Both, &format!("\r\nError: {}\r\n\r\n", err))?;
                cmd.state.reset_color()?;
            }
        }
        cmd.state.session.disp_options.reset_printout();
        if cmd.state.session.request_logoff {
            cmd.state.connection.shutdown()?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
}
