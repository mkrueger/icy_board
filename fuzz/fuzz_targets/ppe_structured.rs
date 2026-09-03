#![no_main]

use icy_board_engine::{
    decompiler::decompile,
    executable::{Executable, LAST_PPL_LANGUAGE_VERSION, PPEScript},
};
use libfuzzer_sys::fuzz_target;

const SEEDS: [&[u8]; 4] = [
    include_bytes!("../../crates/icy_board_engine/tests/test_data/test_dim1.ppe"),
    include_bytes!("../../crates/icy_board_engine/tests/test_data/local_variables.ppe"),
    include_bytes!("../../crates/icy_board_engine/tests/test_ppe/test_pplc_100.ppe"),
    include_bytes!("../../crates/icy_board_engine/tests/test_ppe/test_pplc_340.ppe"),
];

fuzz_target!(|data: &[u8]| {
    let Some((&selector, mutations)) = data.split_first() else {
        return;
    };
    let mut bytes = SEEDS[selector as usize % SEEDS.len()].to_vec();
    for mutation in mutations.chunks_exact(4).take(4_096) {
        let index = u16::from_le_bytes([mutation[0], mutation[1]]) as usize;
        let value = mutation[3];
        match mutation[2] % 4 {
            0 if !bytes.is_empty() => {
                let position = index % bytes.len();
                bytes[position] = value;
            }
            1 if !bytes.is_empty() => {
                let position = index % bytes.len();
                bytes[position] ^= value;
            }
            2 if bytes.len() < u16::MAX as usize * 4 => bytes.insert(index % (bytes.len() + 1), value),
            3 if !bytes.is_empty() => {
                bytes.remove(index % bytes.len());
            }
            _ => {}
        }
    }

    let Ok(executable) = Executable::from_buffer(&mut bytes, false) else {
        return;
    };
    let _ = PPEScript::from_ppe_file(&executable);
    let _ = decompile(executable.clone(), true, LAST_PPL_LANGUAGE_VERSION);
    let _ = decompile(executable, false, LAST_PPL_LANGUAGE_VERSION);
});
