build:
  cargo build

# Local web administration UI, listens on 127.0.0.1:8787
admin board="": build
  target/debug/icbadmin {{board}}

build_ppe: build
  target/debug/pplc ppe/cnfn.pps
  target/debug/pplc ppe/area.pps
  target/debug/pplc ppe/dir.pps
  target/debug/pplc ppe/door.pps
  target/debug/pplc ppe/script2.pps
  
