use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::Res;
use icy_board_engine::{
    icy_board::{
        state::{IcyBoardState, NodeState},
        IcyBoard,
    },
    vm::TerminalTarget,
};
use icy_net::{telnet::TelnetConnection, Connection, ConnectionType};

use crate::menu_runner::PcbBoardCommand;

pub struct BBS {
    pub open_connections: Arc<Mutex<Vec<Option<NodeState>>>>,
}

impl BBS {
    fn clear_closed_connections(&mut self) {
        if let Ok(list) = &mut self.open_connections.lock() {
            for i in 0..list.len() {
                let is_finished = if let Some(state) = &list[i] {
                    if let Some(handle) = &state.handle {
                        handle.is_finished()
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                if is_finished {
                    list[i] = None;
                }
            }
        }
    }

    pub fn get_open_connections(&mut self) -> &Arc<Mutex<Vec<Option<NodeState>>>> {
        self.clear_closed_connections();
        &self.open_connections
    }

    pub fn create_new_node(&mut self, connection_type: ConnectionType) -> usize {
        self.clear_closed_connections();
        if let Ok(list) = &mut self.open_connections.lock() {
            for i in 0..list.len() {
                if list[i].is_none() {
                    let node_state = NodeState::new(i + 1, connection_type);
                    list[i] = Some(node_state);
                    return i;
                }
            }
        }
        panic!("Could not create new connection");
    }

    pub fn new(nodes: usize) -> BBS {
        let mut vec = Vec::new();
        for _ in 0..nodes {
            vec.push(None);
        }
        BBS {
            open_connections: Arc::new(Mutex::new(vec)),
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
                let node = bbs.lock().unwrap().create_new_node(ConnectionType::Telnet);

                let node_list = bbs.lock().unwrap().get_open_connections().clone();
                let handle = thread::spawn(move || {
                    let orig_hook = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |panic_info| {
                        log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                        log::error!("full info: {:?}", panic_info);
                        orig_hook(panic_info);
                    }));

                    let connection = TelnetConnection::accept(stream).unwrap();
                    // connection succeeded
                    if let Err(err) = handle_client(board, node_list, node, Box::new(connection)) {
                        log::error!("Error running backround client: {}", err);
                    }
                    Ok(())
                });
                bbs.lock().unwrap().get_open_connections().lock().unwrap()[node].as_mut().unwrap().handle = Some(handle);
            }
            Err(e) => {
                log::error!("connection failed {}", e);
            }
        }
    }
    drop(listener);
    Ok(())
}

fn handle_client(board: Arc<Mutex<IcyBoard>>, node_state: Arc<Mutex<Vec<Option<NodeState>>>>, node: usize, connection: Box<dyn Connection>) -> Res<()> {
    let state = IcyBoardState::new(board, node_state, node, connection);
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
