#!/bin/sh
# Contact sheets of a stretch of a walkthrough video, to read step by step what the player does at a spot.
#   scripts/video-sheets.sh VIDEO START SECONDS FPS OUTDIR [COLSxROWS]
# e.g. scripts/video-sheets.sh "ref/video/Ballance - Level 13 ....mp4" 384 16 12 ref/video/frames/dense/level13/s4-arms
# Each sheet holds COLSxROWS frames (default 4x3) left to right, top to bottom; sheet N starts at
# START + (N-1) * COLS*ROWS / FPS seconds. Frame times are written to OUTDIR/times.txt.
set -eu
[ $# -ge 5 ] || { sed -n 2,6p "$0"; exit 2; }
video=$1 start=$2 secs=$3 fps=$4 out=$5 tile=${6:-4x3}
mkdir -p "$out"
ffmpeg -loglevel error -y -ss "$start" -t "$secs" -i "$video" \
    -vf "fps=$fps,scale=480:-1,tile=$tile" "$out/sheet%03d.jpg"
cols=${tile%x*} rows=${tile#*x}
per=$((cols * rows))
n=$(ls "$out"/sheet*.jpg | wc -l)
i=1
: > "$out/times.txt"
while [ "$i" -le "$n" ]; do
    printf 'sheet%03d.jpg starts at %s s, %s frames at %s fps\n' "$i" \
        "$(echo "$start + ($i - 1) * $per / $fps" | bc -l | cut -c1-7)" "$per" "$fps" >> "$out/times.txt"
    i=$((i + 1))
done
echo "$n sheets in $out (times.txt)"
