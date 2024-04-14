use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use icy_board_engine::icy_board::{state::Session, IcyBoard};
use icy_net::Connection;

pub struct ActiveConnection {
    pub connection: Box<dyn Connection>,
    pub session: Session,
}

pub struct BBS {
    pub board: Arc<std::sync::Mutex<IcyBoard>>,
    pub open_connections: Arc<Mutex<HashMap<u16, ActiveConnection>>>,
}

impl BBS {
    pub async fn get_board(&self) -> std::sync::MutexGuard<IcyBoard> {
        let a = self.board.lock().unwrap();
        a
    }

    pub fn new(board: IcyBoard) -> BBS {
        BBS {
            board: Arc::new(std::sync::Mutex::new(board)),
            open_connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    /*
    pub fn await_telnet_connections(&mut self) -> Res<()> {
        let addr = "127.0.0.1:23".to_string();
        let listener = match TcpListener::bind(addr) {
            Ok(listener) => listener,
            Err(e) => panic!("could not read start TCP listener: {}", e)
        };
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(move|| {
                        let orig_hook = std::panic::take_hook();
                        std::panic::set_hook(Box::new(move |panic_info| {
                            log::error!("IcyBoard thread crashed at {:?}", panic_info.location());
                            log::error!("full info: {:?}", panic_info);
                            orig_hook(panic_info);
                        }));

                        let cmd = PcbBoardCommand::new();

                        let connection = TelnetConnection::accept(stream).unwrap();
                        // connection succeeded
                        handle_client(stream, clients)
                    });
                }
                Err(e) => {
                    log::error!("connection failed {}", e);
                }
            }
        }
        drop(listener);
        Ok(())
    } */
}
