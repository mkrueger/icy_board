build:
  cargo build

build_ppe: build
  target/debug/pplc ppe/cnfn.pps
  target/debug/pplc ppe/area.pps
  target/debug/pplc ppe/dir.pps
  target/debug/pplc ppe/door.pps
  target/debug/pplc ppe/script2.pps
  
