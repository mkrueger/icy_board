#!/bin/sh
cargo build 
export RUST_BACKTRACE=1 
target/debug/icy_board icb/icyboard.toml
