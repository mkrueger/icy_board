use icy_board_engine::icy_board::lock::{BoardLock, LOCK_FILE_NAME};
use std::fs::OpenOptions;

/// The board server holds the lock for its whole run while its embedded web
/// admin takes one per write, so a second handle inside one process has to work.
#[test]
fn a_second_handle_in_the_same_process_is_granted() {
    let dir = tempfile::tempdir().unwrap();

    let server = BoardLock::acquire(dir.path()).unwrap();
    let web_admin = BoardLock::acquire(dir.path()).expect("the same process must be able to lock again");

    drop(web_admin);
    drop(server);
}

/// Another process still has to stay out while any handle is alive.
#[test]
fn the_file_lock_is_held_until_the_last_handle_goes() {
    let dir = tempfile::tempdir().unwrap();
    let probe = || {
        OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(dir.path().join(LOCK_FILE_NAME))
            .unwrap()
    };

    let outer = BoardLock::acquire(dir.path()).unwrap();
    let inner = BoardLock::acquire(dir.path()).unwrap();
    assert!(probe().try_lock().is_err(), "another process must be refused");

    drop(inner);
    assert!(probe().try_lock().is_err(), "one handle is still holding the board");

    drop(outer);
    assert!(probe().try_lock().is_ok(), "the last handle releases the board");
}

/// The refusal is what a user gets to see, so it has to say which board is busy.
#[test]
fn a_busy_board_is_named_in_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let held = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(dir.path().join(LOCK_FILE_NAME))
        .unwrap();
    held.try_lock().expect("the board starts out free");

    let Err(refused) = BoardLock::acquire(dir.path()) else {
        panic!("a locked board has to be refused");
    };

    let message = refused.to_string();
    assert!(
        message.contains(&dir.path().display().to_string()),
        "the refusal does not say which board it is about: {message}"
    );
}
