#![no_main]

use icy_board_engine::executable::{Executable, PPEScript};
use libfuzzer_sys::fuzz_target;

// What `ppld -d` and `pplc -d` walk. It steps past broken statements instead of
// stopping at the first one, so it reaches commands the loader never hands out.
fuzz_target!(|data: &[u8]| {
    if data.len() > u16::MAX as usize * 4 {
        return;
    }
    let mut bytes = data.to_vec();
    let Ok(mut executable) = Executable::from_buffer(&mut bytes, false) else {
        return;
    };

    if let Ok(script) = PPEScript::from_ppe_file(&executable) {
        executable.variable_table.analyze_usage(&script);
        executable.variable_table.generate_names();
    }

    executable.print_script_buffer_dump();
    executable.print_variable_table();
    executable.print_disassembler();
});
