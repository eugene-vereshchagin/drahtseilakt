#!/bin/sh
# Writes the README that goes beside a published video (OUT's name with .README.md): the level, the engine, the
# textures, the sounds and the interface it was made with, read from the settings file the game used
# (XDG_DATA_HOME, as for the run). scripts/video.sh calls it; run it alone for a video made before.
#
#   scripts/video-readme.sh LEVEL REPLAY OUT.mp4 [WxH]
set -eu
level=$1; replay=$2; out=$3; size=${4:-1920x1080}
here=$(dirname "$0")
# The video's README: what it shows and what it was made with (the same choices the game made from its settings).
settings=${XDG_DATA_HOME:-$HOME/.local/share}/drahtseilakt/settings.txt
setting() { [ -f "$settings" ] && sed -n "s/^$1 //p" "$settings" | head -n 1; }
extended=$( [ "$(setting profile)" = extended ] && echo yes || echo no )
if [ -d "$level" ]; then
    title=$(sed -n 's/^title *= *//p' "$level/level.txt" 2>/dev/null | head -n 1)
    level_line="${title:-$(basename "$level")} - a Drahtseilakt level (our own, freely licensed)"
    textures="Drahtseilakt's own (CC0 photo textures and generated ones, see assets/own/CREDITS.md)"
else
    level_line="Ballance level $level (Cyparade / Atari, 2004), from the player's own copy of the game"
    t=own; [ "$extended" = yes ] && t=$(setting textures); t=${t:-own}
    if [ "$t" = original ]; then textures="Ballance's original textures, from the player's own copy"
    else textures="Drahtseilakt's own (CC0 photo textures and generated ones, see assets/own/CREDITS.md)"; fi
fi
theme=neuro; [ "$extended" = yes ] && theme=$(setting sound-theme); theme=${theme:-neuro}
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
engine="Drahtseilakt $(git -C "$here/.." describe --always --dirty 2>/dev/null || echo unknown), a new engine for Ballance written in Rust"
cat > "${out%.*}.README.md" <<EOF2
# $(basename "$out")

- **Level:** $level_line
- **Engine:** $engine
- **Textures:** $textures
- **Sounds:** $sounds
- **Interface:** $interface
- **Played by:** the Drahtseilakt test player (autopilot), replay \`$(basename "$replay")\`
- **Video:** $size, 60 fps, H.264 with AAC sound, made $(date -u +%Y-%m-%d)

Drahtseilakt needs the player's own copy of Ballance for the original levels and never ships Ballance's data.
Ballance is (c) 2004 Cyparade / Atari; this project is not affiliated with them.
EOF2

