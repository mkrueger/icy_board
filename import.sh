#!/bin/sh
cargo build 
rm -rf icb
RUST_BACKTRACE=1 target/debug/icbsetup import ~/dos/PCB/PCBOARD.DAT icb
