import sys
import os

version=""
cargo = open(os.path.join("Cargo.toml"), "r")
for line in cargo.readlines():
    if line.startswith("version"):
        m = line.index('"')
        version = line[m + 1:len(line) - 2]
        break
cargo.close()

print(version)
