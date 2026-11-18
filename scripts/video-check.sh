#!/bin/sh
# Checks the remastered original levels before their videos are rendered, with the very settings scripts/video.sh
# gives the game (VIDEO_MODE=remaster), so that hours of 4K rendering are not spent on a wrong picture:
#   - the run finishes on its replay;
#   - every texture is found (the audit names nothing missing or drawn plain), none over 4096 px, GPU-compressed ones
#     present;
#   - the balls and parts are the level's remade ones (rounder), not the copy's 2004 meshes nor ours: a frame drawn
#     with them must differ from one drawn with the copy's (`original`) and from ours (`own`) where the ball is;
#   - the game's log has no warning;
#   - frames from the start, the middle and the end of each run go on one sheet per level for a person to look at.
#
#   scripts/video-check.sh OUTDIR [NN ...]      (levels 01..13 by default)
#
# Exits non-zero when any level fails; OUTDIR/summary.txt lists every check, OUTDIR/Level_NN.png the sheets.
set -u
out=$1; shift
[ $# -gt 0 ] || set -- 01 02 03 04 05 06 07 08 09 10 11 12 13
here=$(cd "$(dirname "$0")/.." && pwd)
game=${DRAHTSEILAKT_BIN:-$here/target/release/drahtseilakt}
remastered=${XDG_DATA_HOME:-$HOME/.local/share}/drahtseilakt/levels/ballance-remaster
models=level # what scripts/video.sh takes for VIDEO_MODE=remaster
# the recorded runs live in the autopilot store, not in the repository
pilot=${DRAHTSEILAKT_AUTOPILOT:-$here/../drahtseilakt-tools/autopilot}
[ -d "$pilot" ] || { echo "no autopilot store at $pilot (set DRAHTSEILAKT_AUTOPILOT)" >&2; exit 0; }
mkdir -p "$out"
sum=$out/summary.txt
: > "$sum"
bad=0
say() { echo "$*" | tee -a "$sum"; }
for n in "$@"; do
    lv=$remastered/Level_$n
    rp=$pilot/hints/runs/Level_$n-clean.replay; [ -f "$rp" ] || rp=$pilot/hints/runs/Level_$n.replay
    w=$out/work-$n; mkdir -p "$w"
    fail=""
    [ -d "$lv" ] || { say "Level_$n: FAIL no remastered level at $lv"; bad=1; continue; }
    # the run, and its length for the frames
    (cd "$here" && nice -n 19 "$game" --open "$lv" --replay "$rp" --models $models --headless) > "$w/run.log" 2>&1
    res=$(grep '^result' "$w/run.log")
    case $res in *finished*) ;; *) fail="$fail run-not-finished" ;; esac
    ticks=$(echo "$res" | sed -n 's/.*after \([0-9]*\) ticks.*/\1/p'); ticks=${ticks:-3000}
    # textures
    (cd "$here" && nice -n 19 "$game" --audit "$lv") > "$w/audit.log" 2>&1
    # counted only, or hidden in the file and so invisible in the original too (body-hidden, stopper-hidden)
    issues=$(grep -E '^ +[0-9]+ maps' "$w/audit.log" | awk '{print $5}' | tr -d : |
        grep -vxE 'texture-found|texture-by-name|body-hidden|stopper-hidden' | sort -u)
    [ -z "$issues" ] || fail="$fail audit:$(echo "$issues" | tr '\n' ',')"
    side=$(python3 -I -c "
import glob,struct
m=0
for f in glob.glob('$lv/**/*.png',recursive=True):
    with open(f,'rb') as h: d=h.read(24)
    if d[:8]==b'\x89PNG\r\n\x1a\n': m=max(m,*struct.unpack('>II',d[16:24]))
print(m)")
    [ "$side" -le 4096 ] || fail="$fail png-side-$side"
    dtx=$(find "$lv" -name '*.dtx' | wc -l)
    [ "$dtx" -gt 0 ] || fail="$fail no-dtx"
    # frames: start, middle, end, drawn as the video draws them; at the start also with the copy's parts and ours
    t1=300; t2=$((ticks / 2)); t3=$((ticks - 200))
    for t in $t1 $t2 $t3; do
        (cd "$here" && nice -n 19 "$game" --open "$lv" --replay "$rp" --models $models --screenshot "$w/f$t.png" \
            --ticks "$t" --size 1920x1080) >> "$w/shots.log" 2>&1
    done
    for m in original own; do
        (cd "$here" && nice -n 19 "$game" --open "$lv" --replay "$rp" --models $m --screenshot "$w/m-$m.png" \
            --ticks $t1 --size 1920x1080) >> "$w/shots.log" 2>&1
    done
    warn=$(cat "$w/run.log" "$w/shots.log" | grep -ciE 'warn|error|missing|plain|fallback|not found|failed')
    [ "$warn" = 0 ] || fail="$fail log-warnings-$warn"
    parts=$(python3 -I - "$w" $t1 "$out/Level_$n.png" "Level_$n" <<'EOF'
import sys
from PIL import Image, ImageChops, ImageDraw
w, t1, sheet, name = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
def img(p): return Image.open(p).convert('RGB')
lvl = img(f'{w}/f{t1}.png')
box = (760, 340, 1160, 740)  # the middle of the picture, where the camera keeps the ball at a run's start
def differs(p):
    d = ImageChops.difference(lvl.crop(box), img(p).crop(box)).convert('L')
    return sum(1 for v in d.getdata() if v > 24)
o, s = differs(f'{w}/m-original.png'), differs(f'{w}/m-own.png')
print(f'{o} {s}')
frames = sorted([p for p in __import__('glob').glob(f'{w}/f*.png')], key=lambda p: int(p.split('/f')[-1][:-4]))
sh = Image.new('RGB', (4 * 640, 360 + 24), 'black')
for i, p in enumerate(frames):
    sh.paste(img(p).resize((640, 360)), (i * 640, 24))
sh.paste(lvl.crop((640, 270, 1280, 630)), (3 * 640, 24))  # the ball at full size
ImageDraw.Draw(sh).text((8, 4), f'{name}: start / middle / end / ball at full size', fill='white')
sh.save(sheet)
EOF
)
    po=${parts% *}; ps=${parts#* }
    [ "${po:-0}" -gt 200 ] || fail="$fail parts-same-as-copy($po)"
    [ "${ps:-0}" -gt 200 ] || fail="$fail parts-same-as-ours($ps)"
    if [ -z "$fail" ]; then st=OK; else st="FAIL$fail"; bad=1; fi
    say "Level_$n: $st | $res | max png $side | dtx $dtx | ball area differs from copy's parts by $po px, ours by $ps px"
done
[ $bad = 0 ] && say "all levels pass" || say "SOME LEVELS FAIL"
exit $bad
