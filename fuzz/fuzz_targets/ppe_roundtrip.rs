#![no_main]

use icy_board_engine::executable::Executable;
use libfuzzer_sys::fuzz_target;

// Loading is lenient, so the first pass is what normalizes a file. From there on
// writing and reading it again has to keep answering the same program.
fuzz_target!(|data: &[u8]| {
    if data.len() > u16::MAX as usize * 4 {
        return;
    }
    let mut bytes = data.to_vec();
    let Ok(executable) = Executable::from_buffer(&mut bytes, false) else {
        return;
    };

    let Ok(mut written) = executable.to_buffer() else {
        return;
    };
    let normalized = match Executable::from_buffer(&mut written, false) {
        Ok(normalized) => normalized,
        Err(error) => panic!("a written PPE does not load again: {error}"),
    };

    let Ok(mut rewritten) = normalized.to_buffer() else {
        panic!("a PPE that was just loaded cannot be written again");
    };
    let reloaded = match Executable::from_buffer(&mut rewritten, false) {
        Ok(reloaded) => reloaded,
        Err(error) => panic!("a rewritten PPE does not load again: {error}"),
    };

    assert_eq!(normalized.runtime, reloaded.runtime, "the runtime changed");
    assert_eq!(normalized.user_types, reloaded.user_types, "the type table changed");
    assert_eq!(normalized.variable_table.len(), reloaded.variable_table.len(), "the variable count changed");
    assert_eq!(normalized.script_buffer, reloaded.script_buffer, "the script changed");
});
