#!/bin/sh
# Builds our levels from their descriptions and records the test player's runs of each in every style
# (docs/drahtseilakt/levels.md, "Our levels"): for each level folder given (default: every assets/levels/*/level.lvl),
# leveldsl.py writes the level and its hint files (pilot/<style>.txt), and the game's test player plays it with each,
# its run kept as pilot/<style>.replay when it finishes. crates/game/src/ownlevels.rs checks the runs.
#
#   scripts/own-levels.sh [assets/levels/NN_Name ...]
#
# The runs are played in a data folder of their own (target/own-levels-data), so no player's settings, copy or
# runs come into them.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
game=${DRAHTSEILAKT_BIN:-$here/target/release/drahtseilakt}
export XDG_DATA_HOME="$here/target/own-levels-data"
mkdir -p "$XDG_DATA_HOME"
if [ $# -eq 0 ]; then
    set -- $(ls -d "$here"/assets/levels/*/ | while read -r d; do [ -f "$d/level.lvl" ] && echo "${d%/}"; done)
fi
status=0
for dir in "$@"; do
    python3 -I "$here/assets/own/make/leveldsl.py" "$dir/level.lvl"
    for style in safe speed collector; do
        out="$dir/pilot/$style.replay"
        rm -f "$out"
        result=$("$game" --open "$dir" --autopilot --hints "$dir/pilot/$style.txt" --ticks 72000 --give-up 40 \
            --record "$XDG_DATA_HOME/run.replay" --report "$XDG_DATA_HOME/stuck" 2>&1 | grep '^result:' || true)
        case "$result" in
            "result: finished"*) cp "$XDG_DATA_HOME/run.replay" "$out" ;;
            *) status=1 ;;
        esac
        echo "$(basename "$dir") $style: ${result#result: }"
    done
done
exit $status
