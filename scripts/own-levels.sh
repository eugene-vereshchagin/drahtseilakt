#!/bin/sh
# Builds our levels from their descriptions and records the test player's runs of each in every style
# (docs/drahtseilakt/levels.md, "Our levels"): for each level folder given (default: every assets/levels/*/level.lvl),
# leveldsl.py writes the level and its hint files (pilot/<style>.txt), and the game's test player plays it with each,
# its run kept as pilot/<style>.replay when it finishes (the speed style is tried with its way's points set further
# and further apart, and the fastest run kept with its hints). crates/game/src/ownlevels.rs checks the runs.
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
# The test player's run of one style: its result line; the replay in $XDG_DATA_HOME/run.replay.
run() {
    "$game" --open "$1" --autopilot --hints "$1/pilot/$2.txt" --ticks 72000 --give-up 40 \
        --record "$XDG_DATA_HOME/run.replay" --report "$XDG_DATA_HOME/stuck" 2>&1 | grep '^result:' || true
}
status=0
for dir in "$@"; do
    # the speed style: the way's points this far apart, each tried; the fastest finished run is kept
    best=""
    for spacing in 14 20 28 40; do
        LEVELDSL_SPEED_SPACING=$spacing python3 -I "$here/assets/own/make/leveldsl.py" "$dir/level.lvl" >/dev/null
        result=$(run "$dir" speed)
        ticks=$(echo "$result" | sed -n 's/^result: finished after \([0-9]*\).*/\1/p')
        if [ -n "$ticks" ] && { [ -z "$best" ] || [ "$ticks" -lt "$best" ]; }; then
            best=$ticks
            cp "$XDG_DATA_HOME/run.replay" "$XDG_DATA_HOME/speed.replay"
            cp "$dir/pilot/speed.txt" "$XDG_DATA_HOME/speed.txt"
            speed_result=$result
        fi
    done
    python3 -I "$here/assets/own/make/leveldsl.py" "$dir/level.lvl"
    rm -f "$dir/pilot/speed.replay"
    if [ -n "$best" ]; then
        cp "$XDG_DATA_HOME/speed.replay" "$dir/pilot/speed.replay"
        cp "$XDG_DATA_HOME/speed.txt" "$dir/pilot/speed.txt"
    else
        status=1
        speed_result="not finished"
    fi
    echo "$(basename "$dir") speed: ${speed_result#result: }"
    for style in safe collector; do
        out="$dir/pilot/$style.replay"
        rm -f "$out"
        result=$(run "$dir" $style)
        case "$result" in
            "result: finished"*) cp "$XDG_DATA_HOME/run.replay" "$out" ;;
            *) status=1 ;;
        esac
        echo "$(basename "$dir") $style: ${result#result: }"
    done
done
exit $status
