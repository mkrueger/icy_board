use std::{path::PathBuf, sync::Arc, thread, time::Duration};

use crate::Res;
use async_recursion::async_recursion;
use icy_board_engine::{
    icy_board::{
        bbs::BBS,
        icb_text::IceText,
        login_server::{SecureWebsocket, Telnet, Websocket},
        state::{
            functions::{display_flags, MASK_COMMAND},
            IcyBoardState, NodeState, NodeStatus,
        },
        IcyBoard,
    },
    vm::TerminalTarget,
};
use icy_net::{
    telnet::TelnetConnection,
    termcap_detect::TerminalCaps,
    websocket::{accept_sec_websocket, accept_websocket},
    Connection, ConnectionType,
};
use tokio::{net::TcpListener, sync::Mutex};

use crate::menu_runner::PcbBoardCommand;

pub mod ssh;

pub async fn await_telnet_connections(con: Telnet, board: Arc<tokio::sync::Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>) -> Res<()> {
    let addr = if con.address.is_empty() {
        format!("0.0.0.0:{}", con.port)
    } else {
        format!("{}:{}", con.address, con.port)
    };
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _addr) = listener.accept().await?;
        let bbs2 = bbs.clone();
        let node = bbs.lock().await.create_new_node(ConnectionType::Telnet).await;
        let node_list: Arc<Mutex<Vec<Option<NodeState>>>> = bbs.lock().await.get_open_connections().await.clone();
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
                            if let Err(err) = handle_client(bbs2, board, node_list, node, Box::new(connection), None, "").await {
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

pub async fn await_websocket_connections(con: Websocket, board: Arc<tokio::sync::Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>) -> Res<()> {
    let addr = if con.address.is_empty() {
        format!("0.0.0.0:{}", con.port)
    } else {
        format!("{}:{}", con.address, con.port)
    };
    let listener = TcpListener::bind(&addr).await?;
    loop {
        let (stream, _addr) = listener.accept().await?;
        let bbs2 = bbs.clone();
        let node = bbs.lock().await.create_new_node(ConnectionType::Telnet).await;
        let node_list = bbs.lock().await.get_open_connections().await.clone();
        let board = board.clone();
        let handle = std::thread::Builder::new()
            .name("Websocket handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    let orig_hook: Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync> = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |panic_info| {
                        log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                        log::error!("full info: {:?}", panic_info);
                        orig_hook(panic_info);
                    }));

                    match accept_websocket(stream).await {
                        Ok(connection) => {
                            // connection succeeded
                            if let Err(err) = handle_client(bbs2, board, node_list, node, Box::new(connection), None, "").await {
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

pub async fn await_securewebsocket_connections(con: SecureWebsocket, board: Arc<tokio::sync::Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>) -> Res<()> {
    let addr = if con.address.is_empty() {
        format!("0.0.0.0:{}", con.port)
    } else {
        format!("{}:{}", con.address, con.port)
    };
    let listener = TcpListener::bind(&addr).await?;
    loop {
        let (stream, _addr) = listener.accept().await?;
        let bbs2 = bbs.clone();
        let node: usize = bbs.lock().await.create_new_node(ConnectionType::Telnet).await;
        let node_list = bbs.lock().await.get_open_connections().await.clone();
        let board = board.clone();
        let handle = std::thread::Builder::new()
            .name("Secure Websocket handle".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                    let orig_hook = std::panic::take_hook();
                    std::panic::set_hook(Box::new(move |panic_info| {
                        log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                        log::error!("full info: {:?}", panic_info);
                        orig_hook(panic_info);
                    }));

                    match accept_sec_websocket(stream).await {
                        Ok(connection) => {
                            // connection succeeded
                            if let Err(err) = handle_client(bbs2, board, node_list, node, Box::new(connection), None, "").await {
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

pub async fn handle_client(
    bbs: Arc<tokio::sync::Mutex<BBS>>,
    board: Arc<tokio::sync::Mutex<IcyBoard>>,
    node_state: Arc<Mutex<Vec<Option<NodeState>>>>,
    node: usize,
    connection: Box<dyn Connection>,
    login_options: Option<LoginOptions>,
    stuffed_chars: &str,
) -> Res<()> {
    let state = IcyBoardState::new(bbs, board, node_state, node, connection).await;
    internal_handle_client(state, login_options, stuffed_chars).await
}

#[async_recursion(?Send)]
pub async fn internal_handle_client(mut state: IcyBoardState, login_options: Option<LoginOptions>, stuffed_chars: &str) -> Res<()> {
    let mut logged_in = false;
    let mut local = false;

    let mut num_tries = 0;
    if !stuffed_chars.is_empty() {
        state.stuff_keyboard_buffer(stuffed_chars, true)?;
    }

    if let Some(login_options) = &login_options {
        if login_options.login_sysop {
            logged_in = true;
            state.session.is_sysop = true;
            state.set_current_user(0).await.unwrap();
        }
        local = login_options.local;
    }

    let mut cmd = PcbBoardCommand::new(state);

    cmd.state.session.disp_options.force_count_lines();
    cmd.state.session.term_caps = if local {
        TerminalCaps::LOCAL
    } else {
        TerminalCaps::detect(&mut *cmd.state.connection).await?
    };

    if let Some(login_options) = &login_options {
        if let Some(ppe) = &login_options.ppe {
            if let Err(err) = cmd.state.run_ppe(&ppe, None).await {
                log::error!("error running PPE: {}", err);
            };
            let _ = cmd.state.press_enter().await;
            return Ok(());
        }
    }
    if !logged_in {
        match cmd.login(local).await {
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

    let mut press_enter = cmd.state.session.disp_options.num_lines_printed > 3;
    cmd.state.session.disp_options.count_lines = false;
    loop {
        if num_tries > 15 {
            cmd.state
                .display_text(
                    IceText::ExcessiveErrors,
                    display_flags::NEWLINE | display_flags::BELL | display_flags::LFBEFORE | display_flags::LOGIT,
                )
                .await?;
            cmd.state.logoff_user(true).await?;
            return Ok(());
        }

        if cmd.state.session.disp_options.abort_printout {
            cmd.state.session.disp_options.check_display_status();
        }
        if num_tries == 0 && !cmd.state.session.expert_mode {
            if press_enter && cmd.state.session.disp_options.num_lines_printed > 0 {
                cmd.state.new_line().await?;
                cmd.state.press_enter().await?;
                cmd.state.session.disp_options.check_display_status();
            }
            cmd.state.display_current_menu().await?;
            num_tries = 1;
        }
        cmd.state.session.disp_options.num_lines_printed = 0;
        cmd.state.fresh_line().await?;
        if num_tries == 0 {
            cmd.state.new_line().await?;
        }

        /* TODO: Check for mail.
                if let Some(user) = cmd.state.session.current_user {
                    if cmd.state.board.lock().await.config.message.prompt_to_read_mail && cmd.state.session.has_mail {

                    }
                }
        */
        cmd.state.set_activity(NodeStatus::Available).await;
        let command = cmd
            .state
            .input_field(
                IceText::CommandPrompt,
                40,
                MASK_COMMAND,
                "",
                None,
                display_flags::NEWLINE | display_flags::STACKED,
            )
            .await?;

        if command.starts_with('!') {
            if command.len() == 1 && !cmd.state.saved_cmd.is_empty() {
                let str = cmd.state.saved_cmd.clone();
                cmd.state.stuff_keyboard_buffer(&str, true)?;
            }
            continue;
        }
        if command.len() >= 5 {
            cmd.state.saved_cmd = command.clone();
        }

        let num_tokens = cmd.state.session.push_tokens(&command);
        if num_tokens == 0 {
            press_enter = false;
            num_tries += 1;
            continue;
        }
        press_enter = true;

        match cmd.state.run_single_command(true).await {
            Ok(cmd_run) => {
                if cmd_run {
                    num_tries = 0;
                } else {
                    num_tries += 1;
                }
            }
            Err(err) => {
                cmd.state.session.disp_options.force_count_lines();
                // print error message to user, if possible
                if cmd.state.set_color(TerminalTarget::Both, 4.into()).await.is_ok() {
                    cmd.state
                        .print(icy_board_engine::vm::TerminalTarget::Both, &format!("\r\nError: {}\r\n\r\n", err))
                        .await?;
                    cmd.state.reset_color(TerminalTarget::Both).await?;
                }
            }
        }

        if cmd.state.session.request_logoff {
            cmd.state.connection.shutdown().await?;
            cmd.state.save_current_user().await?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

pub struct LoginOptions {
    pub login_sysop: bool,
    pub ppe: Option<PathBuf>,
    pub local: bool,
}
