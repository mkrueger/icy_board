#![no_main]

use icy_board_engine::{
    decompiler::decompile,
    executable::{Executable, PPEScript, SUPPORTED_PPL_LANGUAGE_VERSIONS},
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

    // Reconstruction reads the language version, so let the file pick one.
    let lang_version = SUPPORTED_PPL_LANGUAGE_VERSIONS[bytes[0] as usize % SUPPORTED_PPL_LANGUAGE_VERSIONS.len()];
    let _ = decompile(executable.clone(), true, lang_version);
    let _ = decompile(executable, false, lang_version);
});
