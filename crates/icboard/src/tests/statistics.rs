use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use icy_board_engine::icy_board::{IcyBoardSerializer, statistics::Statistics, user_base::FSEMode};

use crate::tests::{setup_conference, test_output};

/// Runs a session and hands back the statistics the board left on disk.
fn stats_after(cmd: &str) -> (Statistics, String) {
    let file = Arc::new(Mutex::new(PathBuf::new()));
    let recorder = file.clone();
    let output = test_output(cmd.to_string(), move |board| {
        setup_conference(board);
        // The line editor is the one that can be driven from a plain input script.
        for user in board.users.iter_mut() {
            user.flags.fse_mode = FSEMode::No;
        }
        *recorder.lock().unwrap() = board.config.paths.statistics_file.clone();
    });
    let path = file.lock().unwrap().clone();
    let stats = Statistics::load(&path).unwrap_or_else(|err| panic!("no statistics at {}: {err}\n{output}", path.display()));
    (stats, output)
}

#[test]
fn test_logging_on_counts_the_call() {
    let (stats, _) = stats_after("\n");
    assert_eq!(stats.total.calls, 1);
    assert_eq!(stats.today.calls, 1);
}

#[test]
fn test_posting_a_message_is_counted() {
    let (stats, output) = stats_after("E\nSYSOP\nA subject\nN\nHello there\n\nS\n\n");
    assert_eq!(stats.total.messages, 1, "the message was not counted\n{output}");
    assert_eq!(stats.today.messages, 1);
}
