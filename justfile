build:
  cargo build

# Keeps local test runs responsive; override either value for faster or quieter runs.
test jobs="4" threads="4":
  cargo test --workspace --no-fail-fast --jobs {{jobs}} -- --test-threads {{threads}}

# Installs the language server and sets up every editor found.
setup-editor target="all":
  tools/setup-editor.sh {{target}}

# Fuzzes a PPE binary or PPL source trust boundary. Needs nightly + cargo-fuzz;
# the corpus is temporary, so a run leaves nothing behind in the working tree.
# Binary targets: ppe_load, ppe_decompile, ppe_structured, ppe_roundtrip, ppe_disassemble.
# Source targets: ppl_parse, ppl_compile.
fuzz target="ppe_load" seconds="60":
  #!/usr/bin/env bash
  set -euo pipefail
  corpus="$(mktemp -d)"
  trap 'rm -rf "$corpus"' EXIT
  cargo +nightly fuzz run {{target}} "$corpus" \
    crates/ppld/test_data \
    crates/icy_board_engine/tests/test_ppe \
    crates/icy_board_engine/tests/test_data \
    ppe \
    -- -max_total_time={{seconds}} -max_len=262140 -timeout=15 -close_fd_mask=3

build_ppe: build
  target/debug/pplc ppe/cnfn.pps
  target/debug/pplc ppe/area.pps
  target/debug/pplc ppe/dir.pps
  target/debug/pplc ppe/door.pps
  target/debug/pplc ppe/script2.pps
  
