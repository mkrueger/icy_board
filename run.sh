#!/bin/sh
cargo build 
export RUST_BACKTRACE=1 
target/debug/icboard icb
