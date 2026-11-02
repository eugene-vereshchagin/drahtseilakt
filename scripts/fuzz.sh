#!/usr/bin/env bash
# Runs the stable fuzz targets (crates/fuzzkit; docs/drahtseilakt/fuzzing.md) for a time each, several at once.
#
#   scripts/fuzz.sh [SECONDS_PER_TARGET] [PARALLEL] [TARGET...]
#
# Defaults: 600 seconds, 4 at once, every target. FUZZ_SEED picks the run's inputs (default: a new one from the
# clock, so runs one after another try new inputs); the other FUZZ_* settings pass through. Findings land in
# fuzz/artifacts/<target>/ (crash-*, hang-*, oom-*, abort-*, each with a .txt and a minimised .min.bin), a log of each
# target in fuzz/artifacts/logs/, and every run's input count in fuzz/artifacts/<target>/stats.txt.
set -euo pipefail

secs=${1:-600}
parallel=${2:-4}
shift $(( $# > 2 ? 2 : $# )) || true

cd "$(dirname "$0")/.."
export VXMATH_FONT_DIR=${VXMATH_FONT_DIR:-$PWD/assets/fonts}
export FUZZ_SEED=${FUZZ_SEED:-$(date +%s)}
export FUZZ_SECS=$secs

# target -> the test binary (crate, test target) and the test in it
declare -A where=(
  [nmo_file]="level/fuzz fuzz_nmo_file"
  [nmo_chunk]="level/fuzz fuzz_nmo_chunk"
  [open_level]="level/fuzz fuzz_open_level"
  [image]="gamedata/fuzz fuzz_image"
  [level_txt]="gamedata/fuzz fuzz_level_txt"
  [theme_txt]="gamedata/fuzz fuzz_theme_txt"
  [sound_table]="gamedata/fuzz fuzz_sound_table"
  [disc_image]="cdimage/fuzz fuzz_disc_image"
  [cue_sheet]="cdimage/fuzz fuzz_cue_sheet"
  [iscab]="cdimage/fuzz fuzz_iscab"
  [replay]="game/drahtseilakt fuzz_tests::fuzz_replay"
  [hints]="game/drahtseilakt fuzz_tests::fuzz_hints"
  [wav]="game/drahtseilakt fuzz_tests::fuzz_wav"
  [ogg]="game/drahtseilakt fuzz_tests::fuzz_ogg"
  [highscore]="game/drahtseilakt fuzz_tests::fuzz_highscore"
  [settings]="game/drahtseilakt fuzz_tests::fuzz_settings"
  [level_sim]="game/drahtseilakt fuzz_tests::fuzz_level_sim"
)
order=(nmo_file nmo_chunk open_level level_sim image ogg disc_image cue_sheet iscab wav replay hints settings highscore
  level_txt theme_txt sound_table)
targets=("$@")
[ ${#targets[@]} -eq 0 ] && targets=("${order[@]}")
for t in "${targets[@]}"; do
  [ -n "${where[$t]:-}" ] || { echo "unknown target $t (known: ${order[*]})" >&2; exit 2; }
done

# build the test binaries once and run copies of them: sources edited meanwhile never rebuild under a run. Optimised,
# but with overflow checks and debug assertions on (as cargo-fuzz builds), in a target folder of their own: an
# arithmetic overflow then panics where a release build would wrap silently.
bin=fuzz/artifacts/bin/$FUZZ_SEED
mkdir -p "$bin" fuzz/artifacts/logs
export CARGO_TARGET_DIR=${FUZZ_TARGET_DIR:-$PWD/target/fuzz}
export RUSTFLAGS="${RUSTFLAGS:-} -C overflow-checks=on -C debug-assertions=on"
cargo test --release --offline -p level -p gamedata -p cdimage -p drahtseilakt --no-run --message-format=json 2>/dev/null |
  grep '"executable":"[^"]*/deps/' | grep '"test":true' |
  sed -E 's@.*crates/([a-z]+)#[^"]*".*"name":"([a-z_]+)".*"executable":"([^"]+)".*@\1 \2 \3@' |
  while read -r crate name exe; do mkdir -p "$bin/$crate" && cp "$exe" "$bin/$crate/$name"; done

echo "fuzzing ${#targets[@]} targets, $secs s each, $parallel at once, FUZZ_SEED=$FUZZ_SEED"
run_target() {
  local t=$1 exe=$2 test=$3 log=fuzz/artifacts/logs/$1.log
  if "$bin/$exe" "$test" --ignored --exact --nocapture --test-threads=1 >"$log" 2>&1; then
    echo "$t: $(grep -h "inputs in" "$log" | tail -1)"
  else
    echo "$t: FINDINGS (see $log and fuzz/artifacts/$t/)"
  fi
}
export -f run_target
export bin
for t in "${targets[@]}"; do echo "$t ${where[$t]}"; done | xargs -P "$parallel" -L1 bash -c 'run_target "$@"' _
rm -rf "$bin"
