#![no_main]

use icy_board_engine::executable::Executable;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > u16::MAX as usize * 4 {
        return;
    }
    let mut bytes = data.to_vec();
    let _ = Executable::from_buffer(&mut bytes, false);
});
