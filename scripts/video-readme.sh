#!/bin/sh
# Writes the README that goes beside a published video (OUT's name with .README.txt, plain text to paste into a
# video description): the level, the engine, the
# textures, the sounds and the interface it was made with, read from the settings file the game used
# (XDG_DATA_HOME, as for the run). scripts/video.sh calls it; run it alone for a video made before.
#
#   scripts/video-readme.sh LEVEL REPLAY OUT.mp4 [WxH]
#
# VIDEO_MODE names the run's style: test (the default: test runs and early tries), calm, speedrun or collector, or remaster (a remastered original level).
set -eu
level=$1; replay=$2; out=$3; size=${4:-1920x1080}
here=$(dirname "$0")
mode=${VIDEO_MODE:-test}
case $mode in
    test) mode_line="test run (an early try, not a polished run)" ;;
    calm) mode_line="calm: steady and safe, every checkpoint" ;;
    speedrun) mode_line="speedrun: the fastest way found, risky tricks allowed" ;;
    collector) mode_line="collector: every extra point and life" ;;
    remaster) mode_line="remaster: the full beauty pass of the original level (see below), not an early try" ;;
    *) echo "VIDEO_MODE: test, calm, speedrun, collector or remaster" >&2; exit 1 ;;
esac
# The video's README: what it shows and what it was made with (the same choices the game made from its settings).
settings=${XDG_DATA_HOME:-$HOME/.local/share}/drahtseilakt/settings.txt
setting() { [ -f "$settings" ] && sed -n "s/^$1 //p" "$settings" | head -n 1; }
extended=$( [ "$(setting profile)" = extended ] && echo yes || echo no )
remastered=${XDG_DATA_HOME:-$HOME/.local/share}/drahtseilakt/levels/ballance-remaster
case $(cd "$level" 2>/dev/null && pwd -P) in
    "$(cd "$remastered" 2>/dev/null && pwd -P)"/Level_[0-9][0-9]) is_remaster=yes ;;
    *) is_remaster=no ;;
esac
if [ "$is_remaster" = yes ]; then
    n=$(basename "$level" | sed 's/^Level_0*//')
    level_line="Ballance level $n (Cyparade / Atari, 2004), remastered by drahtseilakt-toolbox from the player's own copy"
    textures="the toolbox's remaster of Ballance's original textures (Real-ESRGAN, up to 4096 px, relief maps), made from the player's own copy"
elif [ -f "$level" ]; then
    set_name=$(basename "$(dirname "$(dirname "$level")")"); n=$(basename "$level" | sed 's/[^0-9]//g; s/^0*//')
    level_line="Ballance level $n, $set_name (a community remake of the original level), from the player's own files"
    textures="Drahtseilakt's own (CC0 photo textures and generated ones, see assets/own/CREDITS.md)"
elif [ -d "$level" ]; then
    title=$(sed -n 's/^title *= *//p' "$level/level.txt" 2>/dev/null | head -n 1)
    level_line="${title:-$(basename "$level")} - a Drahtseilakt level (our own, freely licensed)"
    textures="Drahtseilakt's own (CC0 photo textures and generated ones, see assets/own/CREDITS.md)"
else
    level_line="Ballance level $level (Cyparade / Atari, 2004), from the player's own copy of the game"
    t=own; [ "$extended" = yes ] && t=$(setting textures); t=${t:-own}
    if [ "$t" = original ]; then textures="Ballance's original textures, from the player's own copy"
    else textures="Drahtseilakt's own (CC0 photo textures and generated ones, see assets/own/CREDITS.md)"; fi
fi
case ${VIDEO_TEXTURES:-} in
    original) textures="Ballance's original textures, from the player's own copy" ;;
    remastered) textures="the toolbox's remaster of Ballance's original textures, made from the player's own copy" ;;
esac
theme=neuro; [ "$extended" = yes ] && theme=$(setting sound-theme); theme=${theme:-neuro}; theme=${VIDEO_SOUNDS:-$theme}
case $theme in
    original) sounds="Ballance's original sounds and music, from the player's own copy" ;;
    neuro) sounds="Drahtseilakt's generated theme (synthesized by code, no sample of the original)" ;;
    system) sounds="Drahtseilakt's system theme (CC0 / CC-BY recordings and music, see assets/sounds/system/CREDITS.md)" ;;
    *) sounds="the player's sound theme \"$theme\"" ;;
esac
ui=$(setting interface); [ "$extended" = yes ] || ui=""
case $ui in
    own) interface="Drahtseilakt's own" ;;
    remastered) interface="the toolbox's remaster of the player's copy" ;;
    *) interface="Ballance's original, from the player's own copy (Drahtseilakt's own where the copy has none)" ;;
esac
mo=$(setting models); [ "$extended" = yes ] || mo=""
[ -n "${VIDEO_MODELS:-}" ] && mo=$VIDEO_MODELS
case $mo in
    own) models="Drahtseilakt's own (balls, transformers and every part drawn by us)" ;;
    original) models="Ballance's original, from the player's own copy" ;;
    ""|level) models="as the level takes them"
        [ "$is_remaster" = yes ] && models="Ballance's original balls and parts, remade by the toolbox with the level (rounder shapes, larger textures, relief)" ;;
    *) models="the player's model pack \"$mo\"" ;;
esac
about=""
if [ "$is_remaster" = yes ]; then
    about="
What the remaster is: the same level, the same physics and the same run, drawn anew. drahtseilakt-toolbox takes the
level from the player's own copy of Ballance and rebuilds its look on the player's machine: every texture is enlarged
by Real-ESRGAN up to 4096 pixels a side and stored compressed for the graphics card, the level's shapes are rounded
where the 2004 meshes were faceted, and the lit surfaces get relief (normal maps, raised bricks and sunk mortar). The
balls and parts are the game's own models, remade the same way. Nothing of Ballance is shipped: whoever owns the game makes the remaster
from their own copy in a couple of clicks.
"
fi
engine="Drahtseilakt $(git -C "$here/.." describe --always --dirty 2>/dev/null || echo unknown), a new engine for Ballance written in Rust"
cat > "${out%.*}.README.txt" <<EOF2
Level: $level_line
Engine: $engine
Textures: $textures
Sounds: $sounds
Interface: $interface
Models: $models
Mode: $mode_line
Played by: the Drahtseilakt test player (autopilot), replay $(basename "$replay")
Video: $size, 60 fps, H.264 with AAC sound$( [ "${VIDEO_SS:-1}" -gt 1 ] && echo ", every frame drawn at ${VIDEO_SS}x the size and scaled down (supersampling)"), made $(date -u +%Y-%m-%d)
$about
Drahtseilakt needs the player's own copy of Ballance for the original levels and never ships Ballance's data.
Ballance is (c) 2004 Cyparade / Atari; this project is not affiliated with them.
EOF2

