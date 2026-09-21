#![no_main]

use icy_board_fuzz::ppe400::fuzz_container;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| fuzz_container(data));
