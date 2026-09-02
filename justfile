build:
  cargo build

# Keeps local test runs responsive; override either value for faster or quieter runs.
test jobs="4" threads="4":
  cargo test --workspace --no-fail-fast --jobs {{jobs}} -- --test-threads {{threads}}

# Installs the language server and sets up every editor found.
setup-editor target="all":
  tools/setup-editor.sh {{target}}

build_ppe: build
  target/debug/pplc ppe/cnfn.pps
  target/debug/pplc ppe/area.pps
  target/debug/pplc ppe/dir.pps
  target/debug/pplc ppe/door.pps
  target/debug/pplc ppe/script2.pps
  
