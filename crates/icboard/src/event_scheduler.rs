use std::sync::Arc;

use chrono::Local;
use icy_board_engine::icy_board::{
    IcyBoard,
    bbs::{BBS, BBSMessage},
    events::{EventMode, EventWindow, next_window},
    icb_text::IceText,
};
use tokio::sync::Mutex;

/// How often the clock is looked at. Events are scheduled to the minute.
const TICK: std::time::Duration = std::time::Duration::from_secs(10);

/// Watches the clock, clears the board before a timed event and runs it.
pub async fn run_event_scheduler(board: Arc<Mutex<IcyBoard>>, bbs: Arc<Mutex<BBS>>) {
    let mut last_run = None;
    loop {
        tokio::time::sleep(TICK).await;
        let now = Local::now();
        let window = {
            let board = board.lock().await;
            next_window(&board.config.event, &board.events, &now)
        };
        let Some(window) = window else {
            continue;
        };
        if !window.is_suspended(&now) {
            continue;
        }
        if now < window.run_at {
            clear_the_board(&board, &bbs, &window).await;
            continue;
        }
        if last_run == Some(window.run_at) {
            continue;
        }
        if window.event.mode == EventMode::Idle && callers_online(&bbs).await > 0 {
            log::info!("Skipping event '{}' - callers are still online.", window.event.description);
            last_run = Some(window.run_at);
            continue;
        }
        if window.event.mode == EventMode::Slide {
            if callers_online(&bbs).await > 0 {
                continue;
            }
        } else {
            clear_the_board(&board, &bbs, &window).await;
            if callers_online(&bbs).await > 0 {
                continue;
            }
        }
        last_run = Some(window.run_at);
        run_event(&board, &window).await;
    }
}

async fn callers_online(bbs: &Arc<Mutex<BBS>>) -> usize {
    let mut bbs = bbs.lock().await;
    bbs.clear_closed_connections().await;
    let list = bbs.open_connections.lock().await;
    list.iter().filter(|node| node.is_some()).count()
}

async fn clear_the_board(board: &Arc<Mutex<IcyBoard>>, bbs: &Arc<Mutex<BBS>>, window: &EventWindow) {
    let message = board_text(board, IceText::WaitingForEvent).await;
    let bbs = bbs.lock().await;
    for channel in bbs.bbs_channels.iter().flatten() {
        let _ = channel.send(BBSMessage::Shutdown(message.clone())).await;
    }
    log::info!("Board suspended for event '{}' at {}.", window.event.description, window.run_at);
}

async fn board_text(board: &Arc<Mutex<IcyBoard>>, text: IceText) -> String {
    let board = board.lock().await;
    match board.default_display_text.get_display_text(text) {
        Ok(entry) => entry.text.replace('~', " "),
        Err(_) => String::new(),
    }
}

async fn run_event(board: &Arc<Mutex<IcyBoard>>, window: &EventWindow) {
    let command = window.event.command.trim().to_string();
    if command.is_empty() {
        log::info!("Event '{}' has no command to run.", window.event.description);
        return;
    }
    let root = board.lock().await.root_path.clone();
    log::info!("Running event '{}': {}", window.event.description, command);
    let mut shell = if cfg!(windows) {
        let mut shell = tokio::process::Command::new("cmd");
        shell.arg("/C");
        shell
    } else {
        let mut shell = tokio::process::Command::new("sh");
        shell.arg("-c");
        shell
    };
    match shell.arg(&command).current_dir(&root).status().await {
        Ok(status) => log::info!("Event '{}' ran at {} ({}).", window.event.description, Local::now(), status),
        Err(err) => log::error!("Event '{}' failed to start: {}", window.event.description, err),
    }
}
