#!/bin/sh
cargo build 
RUST_BACKTRACE=1 target/debug/icy_board  ~/work/PCBoard/C/PCB/PCBOARD.DAT
