#!/usr/bin/env bash
# Records a board session as an asciicast and turns it into a GIF, at the 80x25
# a PCBoard screen is drawn for. Anything wider makes the art sit in a corner.
#
#   tools/record_demo.sh logon          # writes assets/logon.cast and .gif
#
# Needs asciinema and agg:
#   https://asciinema.org  -  https://github.com/asciinema/agg
#
# The board is a full screen program, so it has to be driven by hand while the
# recording runs. tools/demo_board.sh gives it the same starting point each
# time, which is what makes two takes comparable.

set -euo pipefail

name=${1:-demo}
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
board=${BOARD_DIR:-/tmp/icyboard-demo}
out=$repo/assets

for tool in asciinema agg; do
    command -v "$tool" >/dev/null || {
        echo "$tool is not installed" >&2
        exit 1
    }
done

if [[ ! -f $board/icboard.toml ]]; then
    echo "no board in $board - run tools/demo_board.sh first" >&2
    exit 1
fi

for dir in target/release target/debug; do
    [[ -x $repo/$dir/icboard ]] && bin=$repo/$dir && break
done
: "${bin:?no icboard found - run: cargo build --release}"

mkdir -p "$out"
rm -f "$out/$name.cast"

# The size is forced because a recording inherits the window it was made in.
asciinema rec "$out/$name.cast" \
    --cols 80 --rows 25 \
    --idle-time-limit 2 \
    --command "cd '$board' && '$bin/icboard' --localon"

agg --cols 80 --rows 25 "$out/$name.cast" "$out/$name.gif"

echo "wrote $out/$name.cast and $out/$name.gif"
