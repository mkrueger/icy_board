#!/bin/sh
cargo build 
RUST_BACKTRACE=1 target/debug/icy_board run icb/icyboard.toml
