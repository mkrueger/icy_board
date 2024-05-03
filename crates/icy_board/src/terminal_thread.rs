use codepages::tables::CP437_TO_UNICODE;
use icy_board_engine::Res;
use icy_engine::{ansi, BufferParser, Caret};
use icy_net::Connection;
use std::{
    mem,
    sync::{Arc, Mutex},
    thread,
};
use tokio::sync::mpsc;

use crate::icy_engine_output::Screen;

pub struct ConnectionThreadData {
    pub rx: mpsc::Receiver<SendData>,
    pub com: Box<dyn Connection>,
    pub thread_is_running: bool,
    pub is_connected: bool,
}

pub fn start_update_thread(com: Box<dyn Connection>, screen: Arc<Mutex<Screen>>) -> (thread::JoinHandle<()>, mpsc::Sender<SendData>) {
    let (tx, rx) = mpsc::channel(32);
    (
        std::thread::Builder::new()
            .name("Terminal update".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut buffer_parser = ansi::Parser::default();
                    buffer_parser.bs_is_ctrl_char = true;
                    let mut connection = ConnectionThreadData {
                        is_connected: false,
                        com,
                        thread_is_running: true,
                        rx,
                    };
                    let mut data = [0; 1024 * 64];
                    loop {
                        tokio::select! {
                            read_data = connection.com.read(&mut data) => {
                                match read_data {
                                    Err(err) => {
                                        log::error!("run_update_thread::read_data: {err}");
//                                        update_thread.lock().is_connected = false;
                                        break;
                                    }
                                    Ok(size) => {
                                        if size > 0 {
                                            let mut s = screen.lock().unwrap();
                                            let mut caret = Caret::default();
                                            mem::swap(&mut caret, &mut s.caret);
                                            for ch in &data[0..size] {
                                                let _ = buffer_parser.print_char( &mut s.buffer, 0, &mut caret, CP437_TO_UNICODE[*ch as usize]);
                                            }
                                            mem::swap(&mut s.caret, &mut caret);
                                        } else {
                                            std::thread::sleep(std::time::Duration::from_millis(20));
                                        }
                                    }
                                }
                            }
                            Some(data) = connection.rx.recv() => {
                                let _ = handle_receive(&mut connection, data).await;
                            }
                        };
                    }
                });
            })
            .unwrap(),
        tx,
    )
}

async fn handle_receive(c: &mut ConnectionThreadData, data: SendData) -> Res<()> {
    match data {
        SendData::Data(buf) => {
            c.com.send(&buf).await?;
        }

        SendData::_Disconnect => {
            c.com.shutdown().await?;
        }
    }
    Ok(())
}

/// Data that is sent to the connection thread
#[derive(Debug)]
pub enum SendData {
    Data(Vec<u8>),
    _Disconnect,
}
