#!/bin/sh
# Renders a replay as an H.264 video with the game's sound: the game draws its frames (--video, raw RGB, 60 a
# second) into a pipe that ffmpeg reads, and mixes the sound into a WAV (DRAHTSEILAKT_SOUND_FILE); then the two are
# joined. The game itself never runs another program; this script does.
#
#   scripts/video.sh LEVEL REPLAY OUT.mp4 [WxH]      (the game's data from OPENBALLANCE_DATA, as for any run)
#
# LEVEL is an original level's number, or a level folder in the open format (one of ours in assets/levels).
#
# Nothing large is kept: the pipe and the WAV live in a temporary folder beside OUT and are removed.
#
# Beside the video goes OUT's name with .README.txt: the level, the engine, the textures, the sounds and the
# interface it was made with (read from the settings file the game used), and the run's mode (VIDEO_MODE: test,
# calm, speedrun or collector; test by default), as every published video carries.
set -eu
level=$1; replay=$2; out=$3; size=${4:-1920x1080}
# VIDEO_SS=N draws every frame N times larger and scales it down (supersampling: smooth edges, crisp far textures);
# VIDEO_CRF sets the encoder's quality (lower is better; 20 by default)
ss=${VIDEO_SS:-1}; crf=${VIDEO_CRF:-20}
# VIDEO_MODELS picks the model pack for the run (own, original or a pack's key), whatever the settings file says; a
# remaster video takes the level's own parts by default: Ballance's balls and parts as the toolbox remade them with the
# level (rounder, larger textures, relief), not the copy's 2004 meshes (original) and not ours (own)
models=${VIDEO_MODELS:-}; [ -z "$models" ] && [ "${VIDEO_MODE:-}" = remaster ] && models=level
w=${size%x*}; h=${size#*x}
draw="$((w * ss))x$((h * ss))"
here=$(dirname "$0")
game=${DRAHTSEILAKT_BIN:-$here/../target/release/drahtseilakt}
work=$(mktemp -d "$(dirname "$out")/.video.XXXXXX")
trap 'rm -rf "$work"' EXIT
mkfifo "$work/frames"
ffmpeg -loglevel error -y -f rawvideo -pix_fmt rgb24 -s "$draw" -r 60 -i "$work/frames" \
    -vf "scale=$size:flags=lanczos" -c:v libx264 -preset medium -crf "$crf" -pix_fmt yuv420p "$work/picture.mp4" &
encoder=$!
# a folder is an open-format level, a .nmo file a map (a community map or a modified original), else an original level
case "$level" in *.nmo|*.NMO) which=--map ;; *) if [ -d "$level" ]; then which=--open; else which=--level; fi ;; esac
# VIDEO_TEXTURES picks the texture pack (own, original, remastered or a pack's key), whatever the settings file says
textures=${VIDEO_TEXTURES:-}
# VIDEO_SOUNDS picks the sound theme (neuro, system, original or a theme's key), whatever the settings file says
sounds=${VIDEO_SOUNDS:-}
DRAHTSEILAKT_SOUND_FILE="$work/sound.wav" "$game" "$which" "$level" --replay "$replay" --size "$draw" \
    ${models:+--models "$models"} ${textures:+--textures "$textures"} ${sounds:+--sounds "$sounds"} --video "$work/frames"
wait "$encoder"
ffmpeg -loglevel error -y -i "$work/picture.mp4" -i "$work/sound.wav" -c:v copy -c:a aac -b:a 192k -shortest "$out"

VIDEO_MODELS=$models VIDEO_TEXTURES=$textures VIDEO_SOUNDS=$sounds sh "$here/video-readme.sh" "$level" "$replay" "$out" "$size"
echo "$out"
