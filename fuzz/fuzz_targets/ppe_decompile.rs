#![no_main]

use icy_board_engine::{
    decompiler::decompile,
    executable::{Executable, LAST_PPL_LANGUAGE_VERSION, PPEScript},
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > u16::MAX as usize * 4 {
        return;
    }
    let mut bytes = data.to_vec();
    let Ok(executable) = Executable::from_buffer(&mut bytes, false) else {
        return;
    };
    let _ = PPEScript::from_ppe_file(&executable);
    let _ = decompile(executable.clone(), true, LAST_PPL_LANGUAGE_VERSION);
    let _ = decompile(executable, false, LAST_PPL_LANGUAGE_VERSION);
});
