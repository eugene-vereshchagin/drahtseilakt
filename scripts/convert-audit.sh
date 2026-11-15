#!/usr/bin/env bash
# Converter audit over a map collection: each .nmo is converted to the open format, loaded again, compared and
# played both ways (drahtseilakt --convert-audit, one process a map so that a crash or a hang is one map's verdict).
# Usage: scripts/convert-audit.sh BIN OUT.tsv DIR...
# env: JOBS (4), TICKS (3000), WORK (the temp folder's convert-audit/work: the exports, each removed after its map).
set -u
BIN=$(readlink -f "$1"); OUT=$2; shift 2
JOBS=${JOBS:-4}; TICKS=${TICKS:-3000}; WORK=${WORK:-${TMPDIR:-/tmp}/convert-audit/work}
mkdir -p "$WORK"
export BIN TICKS WORK
one() {
    f=$1
    d=$(mktemp -d "$WORK/m.XXXXXX")
    line=$(cd "$d" && timeout 900 nice -n 19 "$BIN" --convert-audit "$f" --convert-out "$d/x" --ticks "$TICKS" \
        2>"$d/err" </dev/null | grep '^convert-audit' | head -1)
    if [ -z "$line" ]; then
        why=$(grep -E "panicked|error|drahtseilakt:" "$d/err" | head -1 | tr '\t\n' '  ' | cut -c1-300)
        line=$(printf 'convert-audit\t%s\tno: crashed or timed out: %s\t-\t-\t-\t-' "$f" "$why")
    fi
    printf '%s\n' "$line"
    rm -rf "$d"
}
export -f one
for dir in "$@"; do find "$dir" -type f -iname '*.nmo'; done | sort | \
    xargs -d '\n' -P "$JOBS" -I{} bash -c 'one "$1"' _ {} > "$OUT"
echo "$(wc -l < "$OUT") maps -> $OUT"
