use std::{
    sync::mpsc::{RecvTimeoutError, channel},
    time::Duration,
};

use icy_board_engine::{
    decompiler::decompile,
    executable::{Executable, LAST_PPL_LANGUAGE_VERSION, SUPPORTED_PPL_LANGUAGE_VERSIONS},
};

#[test]
fn excessive_array_allocations_are_rejected() {
    let mut bytes = include_bytes!("test_data/malformed_array_allocation.ppe").to_vec();
    let Err(error) = Executable::from_buffer(&mut bytes, false) else {
        panic!("the malformed PPE loaded");
    };

    assert!(error.to_string().contains("loading is limited"), "{error}");
}

#[test]
fn invalid_routine_metadata_is_rejected() {
    let mut bytes = include_bytes!("test_data/malformed_routine_metadata.ppe").to_vec();
    let Err(error) = Executable::from_buffer(&mut bytes, false) else {
        panic!("the malformed PPE loaded");
    };

    assert!(error.to_string().contains("Invalid index in variable table"), "{error}");
}

#[test]
fn an_assignment_target_that_is_not_a_variable_is_decompiled_as_a_comment() {
    let mut bytes = include_bytes!("test_ppe/test_pplc_100.ppe").to_vec();
    bytes[650] = 102;
    let executable = Executable::from_buffer(&mut bytes, false).unwrap();

    let (ast, _) = decompile(executable, false, LAST_PPL_LANGUAGE_VERSION).unwrap();

    assert!(ast.to_string().contains("Invalid assignment target"), "{ast}");
}

/// A routine body whose table entry is gone leaves the walk with nothing to move it
/// on, so decompiling has to keep making progress on its own.
#[test]
fn a_routine_offset_without_an_entry_still_finishes() {
    for lang_version in SUPPORTED_PPL_LANGUAGE_VERSIONS.iter().copied() {
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let mut bytes = include_bytes!("test_data/malformed_routine_offset.ppe").to_vec();
            if let Ok(executable) = Executable::from_buffer(&mut bytes, false) {
                let _ = decompile(executable, false, lang_version);
            }
            let _ = sender.send(());
        });

        match receiver.recv_timeout(Duration::from_secs(30)) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => panic!("decompiling at {lang_version} does not finish"),
            Err(RecvTimeoutError::Disconnected) => panic!("decompiling at {lang_version} panicked"),
        }
    }
}
