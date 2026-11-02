#!/bin/sh
# Renders a replay as an H.264 video with the game's sound: the game draws its frames (--video, raw RGB, 60 a
# second) into a pipe that ffmpeg reads, and mixes the sound into a WAV (DRAHTSEILAKT_SOUND_FILE); then the two are
# joined. The game itself never runs another program; this script does.
#
#   scripts/video.sh LEVEL REPLAY OUT.mp4 [WxH]      (the game's data from OPENBALLANCE_DATA, as for any run)
#
# Nothing large is kept: the pipe and the WAV live in a temporary folder beside OUT and are removed.
set -eu
level=$1; replay=$2; out=$3; size=${4:-1920x1080}
here=$(dirname "$0")
game=${DRAHTSEILAKT_BIN:-$here/../target/release/drahtseilakt}
work=$(mktemp -d "$(dirname "$out")/.video.XXXXXX")
trap 'rm -rf "$work"' EXIT
mkfifo "$work/frames"
ffmpeg -loglevel error -y -f rawvideo -pix_fmt rgb24 -s "$size" -r 60 -i "$work/frames" \
    -c:v libx264 -preset medium -crf 20 -pix_fmt yuv420p "$work/picture.mp4" &
encoder=$!
DRAHTSEILAKT_SOUND_FILE="$work/sound.wav" "$game" --level "$level" --replay "$replay" --size "$size" \
    --video "$work/frames"
wait "$encoder"
ffmpeg -loglevel error -y -i "$work/picture.mp4" -i "$work/sound.wav" -c:v copy -c:a aac -b:a 192k -shortest "$out"
echo "$out"
