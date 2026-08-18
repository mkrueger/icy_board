#!/usr/bin/env bash
# Creates a throwaway board, so a screenshot or a recording starts from the same
# place every time instead of from whatever the last session left behind.
#
#   tools/demo_board.sh [target]
#
# Prints the sysop password it generated, because a recording usually wants to
# log in as somebody. Delete the directory when you are done with it.

set -euo pipefail

target=${1:-/tmp/icyboard-demo}
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

for dir in target/release target/debug; do
    if [[ -x $repo/$dir/icbsetup ]]; then
        bin=$repo/$dir
        break
    fi
done

if [[ -z ${bin:-} ]]; then
    echo "no icbsetup found - run: cargo build --release" >&2
    exit 1
fi

rm -rf "$target"
"$bin/icbsetup" create "$target"

cat <<EOF

The board is in $target. Start it with:

  cd $target && $bin/icboard --localon

EOF
