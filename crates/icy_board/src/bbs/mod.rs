use std::{path::PathBuf, sync::Arc, thread, time::Duration};

use crate::Res;
use async_recursion::async_recursion;
use icy_board_engine::{
    icy_board::{
        bbs::BBS,
        login_server::{SecureWebsocket, Telnet, Websocket, SSH},
        state::{IcyBoardState, NodeState},
        IcyBoard,
    },
    vm::TerminalTarget,
};
use icy_net::{telnet::TelnetConnection, termcap_detect::TerminalCaps, Connection, ConnectionType};
use tokio::{net::TcpListener, sync::Mutex};

use crate::menu_runner::PcbBoardCommand;

pub async fn await_telnet_connections(telnet: Telnet, board: Arc<tokio::sync::Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>) -> Res<()> {
    let addr = if telnet.address.is_empty() {
        format!("0.0.0.0:{}", telnet.port)
    } else {
        format!("{}:{}", telnet.address, telnet.port)
    };
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _addr) = listener.accept().await?;
        let bbs2 = bbs.clone();
        let node = bbs.lock().await.create_new_node(ConnectionType::Telnet).await;
        let node_list = bbs.lock().await.get_open_connections().await.clone();
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
                            if let Err(err) = handle_client(bbs2, board, node_list, node, Box::new(connection), None).await {
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
        bbs.lock().await.get_open_connections().await.lock().await[node].as_mut().unwrap().handle = Some(handle);
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
    bbs: Arc<tokio::sync::Mutex<BBS>>,
    board: Arc<tokio::sync::Mutex<IcyBoard>>,
    node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
    node: usize,
    connection: Box<dyn Connection>,
    login_options: Option<LoginOptions>,
) -> Res<()> {
    let mut state = IcyBoardState::new(bbs, board, node_state, node, connection).await;
    let mut logged_in = false;
    let mut local = false;
    if let Some(login_options) = &login_options {
        if login_options.login_sysop {
            logged_in = true;
            state.session.is_sysop = true;
            state.set_current_user(0).await.unwrap();
            local = login_options.local;
        }
    }
    let mut cmd = PcbBoardCommand::new(state);
    let caps = if local {
        TerminalCaps::LOCAL
    } else {
        TerminalCaps::detect(&mut *cmd.state.connection).await?
    };
    log::warn!("terminal caps: {:?}", caps);

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
                cmd.state.reset_color(TerminalTarget::Both).await?;
            }
        }
        cmd.state.session.disp_options.reset_printout();
        if cmd.state.session.request_logoff {
            cmd.state.connection.shutdown().await?;
            log::info!("logoff");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub struct LoginOptions {
    pub login_sysop: bool,
    pub ppe: Option<PathBuf>,
    pub local: bool,
}
