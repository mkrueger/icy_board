use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::Res;
use async_recursion::async_recursion;
use icy_board_engine::{
    icy_board::{
        login_server::{SecureWebsocket, Telnet, Websocket, SSH},
        state::{IcyBoardState, NodeState},
        IcyBoard,
    },
    vm::TerminalTarget,
};
use icy_net::{
    telnet::TelnetConnection,
    //    websocket::{accept_websocket, accept_websocket_secure},
    Connection,
    ConnectionType,
};
use tokio::net::TcpListener;

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

pub async fn await_telnet_connections(telnet: Telnet, board: Arc<tokio::sync::Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>) -> Res<()> {
    let addr = if telnet.address.is_empty() {
        format!("0.0.0.0:{}", telnet.port)
    } else {
        format!("{}:{}", telnet.address, telnet.port)
    };
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _addr) = listener.accept().await?;
        let node = bbs.lock().unwrap().create_new_node(ConnectionType::Telnet);
        let node_list = bbs.lock().unwrap().get_open_connections().clone();
        let board = board.clone();
        let handle = std::thread::Builder::new()
            .name("Telnet handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    let orig_hook = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |panic_info| {
                        log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                        log::error!("full info: {:?}", panic_info);
                        orig_hook(panic_info);
                    }));

                    match TelnetConnection::accept(stream) {
                        Ok(connection) => {
                            // connection succeeded
                            if let Err(err) = handle_client(board, node_list, node, Box::new(connection), None).await {
                                log::error!("Error running backround client: {}", err);
                            }
                        }
                        Err(e) => {
                            log::error!("telnet connection failed {}", e);
                        }
                    }
                });
            })
            .unwrap();
        bbs.lock().unwrap().get_open_connections().lock().unwrap()[node].as_mut().unwrap().handle = Some(handle);
    }
}

pub fn await_ssh_connections(_ssh: SSH, _board: Arc<tokio::sync::Mutex<IcyBoard>>, _bbs: Arc<Mutex<BBS>>) -> Res<()> {
    /*
    let addr = if ssh.address.is_empty() {
        format!("127.0.0.1:{}", ssh.port)
    } else {
        format!("{}:{}", ssh.address, ssh.port)
    };
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(e) => panic!("could not read start TCP listener: {}", e),
    };
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let board = board.clone();
                let node = bbs.lock().unwrap().create_new_node(ConnectionType::SSH);

                let node_list = bbs.lock().unwrap().get_open_connections().clone();
                let handle = thread::spawn(move || {
                    let orig_hook = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |panic_info| {
                        log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                        log::error!("full info: {:?}", panic_info);
                        orig_hook(panic_info);
                    }));

                    match TelnetConnection::accept(stream) {
                        Ok(connection) => {
                            // connection succeeded
                            if let Err(err) = handle_client(board, node_list, node, Box::new(connection)) {
                                log::error!("Error running backround client: {}", err);
                            }
                        }
                        Err(e) => {
                            log::error!("ssh connection failed {}", e);
                        }
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
    drop(listener); */
    Ok(())
}

pub fn await_websocket_connections(_ssh: Websocket, _board: Arc<tokio::sync::Mutex<IcyBoard>>, _bbs: Arc<Mutex<BBS>>) -> Res<()> {
    /*
    let addr = if ssh.address.is_empty() {
        format!("127.0.0.1:{}", ssh.port)
    } else {
        format!("{}:{}", ssh.address, ssh.port)
    };
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(e) => panic!("could not read start TCP listener: {}", e),
    };
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let board = board.clone();
                let node = bbs.lock().unwrap().create_new_node(ConnectionType::Websocket);

                let node_list = bbs.lock().unwrap().get_open_connections().clone();
                let handle = thread::spawn(move || {
                    let orig_hook = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |panic_info| {
                        log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                        log::error!("full info: {:?}", panic_info);
                        orig_hook(panic_info);
                    }));

                    match accept_websocket(stream) {
                        Ok(connection) => {
                            // connection succeeded
                            if let Err(err) = handle_client(board, node_list, node, Box::new(connection)) {
                                log::error!("Error running backround client: {}", err);
                            }
                        }
                        Err(e) => {
                            log::error!("webserver connection failed {}", e);
                        }
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
    drop(listener); */
    Ok(())
}

pub fn await_securewebsocket_connections(_ssh: SecureWebsocket, _board: Arc<tokio::sync::Mutex<IcyBoard>>, _bbs: Arc<Mutex<BBS>>) -> Res<()> {
    /*
    let addr = if ssh.address.is_empty() {
        format!("127.0.0.1:{}", ssh.port)
    } else {
        format!("{}:{}", ssh.address, ssh.port)
    };
    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(e) => panic!("could not read start TCP listener: {}", e),
    };
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let board = board.clone();
                let node = bbs.lock().unwrap().create_new_node(ConnectionType::SecureWebsocket);

                let node_list = bbs.lock().unwrap().get_open_connections().clone();
                let cp = ssh.cert_pem.clone();
                let kp = ssh.key_pem.clone();
                let handle = thread::spawn(move || {
                    let orig_hook = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |panic_info| {
                        log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                        log::error!("full info: {:?}", panic_info);
                        orig_hook(panic_info);
                    }));

                    match accept_websocket_secure(stream, &cp, &kp) {
                        Ok(connection) => {
                            // connection succeeded
                            if let Err(err) = handle_client(board, node_list, node, Box::new(connection)) {
                                log::error!("Error running backround client: {}", err);
                            }
                        }
                        Err(e) => {
                            log::error!("secure webserver connection failed {}", e);
                        }
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
    drop(listener);*/
    Ok(())
}

#[async_recursion(?Send)]
pub async fn handle_client(
    board: Arc<tokio::sync::Mutex<IcyBoard>>,
    node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
    node: usize,
    connection: Box<dyn Connection>,
    login_options: Option<LoginOptions>,
) -> Res<()> {
    let mut state = IcyBoardState::new(board, node_state, node, connection).await;
    let mut logged_in = false;
    if let Some(login_options) = &login_options {
        if login_options.login_sysop {
            logged_in = true;
            state.session.is_sysop = true;
            state.set_current_user(0).await.unwrap();
        }
    }
    let mut cmd = PcbBoardCommand::new(state);

    if let Some(login_options) = &login_options {
        if let Some(ppe) = &login_options.ppe {
            let _ = cmd.state.run_ppe(&ppe, None);
            let _ = cmd.state.press_enter();
            let _ = cmd.state.hangup();
            return Ok(());
        }
    }
    if !logged_in {
        match cmd.login().await {
            Ok(true) => {}
            Ok(false) => {
                return Ok(());
            }
            Err(err) => {
                log::error!("error during login process {}", err);
                return Ok(());
            }
        }
    }

    loop {
        if let Err(err) = cmd.do_command().await {
            cmd.state.session.disp_options.reset_printout();
            // print error message to user, if possible
            if cmd.state.set_color(TerminalTarget::Both, 4.into()).await.is_ok() {
                cmd.state
                    .print(icy_board_engine::vm::TerminalTarget::Both, &format!("\r\nError: {}\r\n\r\n", err))
                    .await?;
                cmd.state.reset_color().await?;
            }
        }
        cmd.state.session.disp_options.reset_printout();
        if cmd.state.session.request_logoff {
            cmd.state.connection.shutdown().await?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub struct LoginOptions {
    pub login_sysop: bool,
    pub ppe: Option<PathBuf>,
}
