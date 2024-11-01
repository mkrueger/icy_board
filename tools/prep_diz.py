import sys
import os

if len(sys.argv) != 2:
    print("need 1 arguments")
    sys.exit(1)

version=""
cargo = open(os.path.join("Cargo.toml"), "r")
for line in cargo.readlines():
    if line.startswith("version"):
        m = line.index('"')
        version = line[m + 1:len(line) - 2]
        break
cargo.close()

file_id = open(os.path.join("data", "file_id.diz"), "r")
lines = file_id.readlines()
file_id.close()
new_lines = list(map(lambda line: line.replace("#VERSION", version), lines))

f = open(sys.argv[1], "w")
f.writelines(new_lines)
f.close()

print(version)
