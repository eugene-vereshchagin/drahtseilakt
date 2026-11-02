#!/usr/bin/env python3
# Prepares the system sound theme (assets/sounds/system/Sounds and Music) from the openly licensed files the
# owner chose (CREDITS.md lists each source, author and licence). It is run by hand, not by the build: the sources are
# not in the repository. Each file is decoded with ffmpeg, cut, made mono or stereo, matched in loudness to the
# original's sound of the same role (the player's copy, given as ORIGINAL), looped without a seam where the game loops
# it, and encoded as Ogg Vorbis.
#
#   python3 -I prepare.py CANDIDATES ORIGINAL_SOUNDS_DIR
#
# Needs ffmpeg and numpy. Writes into the folders beside this file.

import json
import subprocess
import sys
from pathlib import Path

import numpy as np

RATE = 44100
HERE = Path(__file__).resolve().parent

# role, source (relative to CANDIDATES), kind (sound | loop | music), start s, length s (None: to the end),
# semitones of pitch shift, channels
SOUNDS = [
    ("Misc_StartLevel", "level_start/! 121800_1525789-hq.mp3", "sound", 0.0, None, 0, 2),
    ("Misc_Checkpoint", "checkpoint/169855_3126007-hq.mp3", "sound", 0.0, None, 0, 2),
    ("Misc_Fall", "ball_lost/159408_1656228-hq.mp3", "sound", 0.0, None, 0, 1),
    ("Misc_Lightning", "ball_appear/spark.wav", "sound", 0.0, None, 0, 1),
    ("Misc_Trafo", "transformer/qubodup-megaswosh1.wav", "sound", 0.0, None, 0, 1),
    ("Extra_Start", "extra_point/! glass_004.ogg", "sound", 0.0, None, 0, 1),
    ("Extra_Hit", "extra_point/glass_002.ogg", "sound", 0.0, None, 0, 1),
    ("Extra_Life_Blob", "extra_life/life_pickup.flac", "sound", 0.0, None, 0, 1),
    ("Misc_extraball", "extra_life/powerUp2.ogg", "sound", 0.0, None, 0, 1),
    ("Misc_RopeTears", "rope_tear/443833_9184212-hq.mp3", "sound", 0.0, None, 0, 1),
    ("Music_thunder", "thunder/446753_1790434-hq.mp3", "thunder", 0.0, 7.0, 0, 2),
    ("Misc_Ventilator", "fan_wind/242042_1900515-hq.mp3", "loop", 10.0, 6.0, 0, 1),
    ("Menu_click", "menu_ui/click1.ogg", "sound", 0.0, None, 0, 1),
    ("Menu_load", "menu_ui/select_001.ogg", "sound", 0.0, None, 0, 1),
    # rolling: the wooden ball's loops as recorded, the stone ball's a little lower and heavier
    ("Roll_Paper", "rolling_paper/! 545532_6233742-hq.mp3", "loop", 4.0, 5.0, 0, 1),
    ("Roll_Wood_Wood", "roll_wood/629001_13875907-hq.mp3", "loop", 8.0, 5.0, 0, 1),
    ("Roll_Wood_Stone", "roll_stone/182345_1864955-hq.mp3", "loop", 0.2, 4.2, 0, 1),
    ("Roll_Wood_Metal", "roll_metal/537939_11979494-hq.mp3", "loop", 2.0, 5.0, 0, 1),
    ("Roll_Stone_Wood", "roll_wood/629001_13875907-hq.mp3", "loop", 20.0, 5.0, -4, 1),
    ("Roll_Stone_Stone", "roll_stone/182345_1864955-hq.mp3", "loop", 0.2, 4.2, -4, 1),
    ("Roll_Stone_Metal", "roll_metal/537939_11979494-hq.mp3", "loop", 6.0, 5.0, -4, 1),
    # hits: the wooden ball's on wood, stone, metal and the domes; the stone ball's heavier
    ("Hit_Wood_Wood", "hit_wood/impactWood_medium_000.ogg", "sound", 0.0, None, 0, 1),
    ("Hit_Wood_Stone", "hit_stone/impactGeneric_light_000.ogg", "sound", 0.0, None, 0, 1),
    ("Hit_Wood_Metal", "hit_metal/impactMetal_light_000.ogg", "sound", 0.0, None, 0, 1),
    ("Hit_Wood_Dome", "hit_metal/impactPlate_light_000.ogg", "sound", 0.0, None, 0, 1),
    ("Hit_Stone_Stone", "hit_stone/impactMining_000.ogg", "sound", 0.0, None, 0, 1),
    ("Hit_Stone_Wood", "hit_wood/impactWood_heavy_000.ogg", "sound", 0.0, None, 0, 1),
    ("Hit_Stone_Metal", "hit_metal/impactMetal_medium_000.ogg", "sound", 0.0, None, 0, 1),
    ("Hit_Stone_Kuppel", "hit_metal/impactPlate_light_002.ogg", "sound", 0.0, None, -3, 1),
    ("Hit_Paper", "hit_paper/334200_4290188-hq.mp3", "sound", 0.0, None, 0, 1),
    # the finish's jingles and the loop near the level's end
    ("Music_Final", "level_start/108248_327532-hq.mp3", "music", 0.0, None, 0, 2),
    ("Music_LastFinal", "level_start/Orient Theme.ogg", "music", 0.0, None, 0, 2),
    ("Music_EndCheckpoint", "music_menu/moonlit_mirage_loop.flac", "musicloop", 0.0, 40.0, 0, 2),
]

# The level music: the owner's picks, each a whole track (named by itself; music.txt says which plays where).
TRACKS = [
    ("ripples", "music/! Ripples.mp3"),
    ("guzheng_city", "music/! Guzheng City.mp3"),
    ("shenyang", "music/! Shenyang.mp3"),
    ("liyan", "music/! liyan.mp3"),
    ("neolein_472707", "music/! 472707_4596312-hq.mp3"),
    ("neolein_573972", "music/! 573972_4596312-hq.mp3"),
    ("cold_to_lukewarm", "music/! cold to lukewarm mixia_0.ogg"),
    ("misty_mountains", "music/! RPG_-_Misty_Mountains.ogg"),
    ("ishikari_lore", "music/! Ishikari Lore.mp3"),
]

# The longest part of a track kept (the original's parts are 15 to 30 s; ours are whole pieces, cut and faded).
TRACK_SECONDS = 100.0

# Where each track plays: the original's five themes, three parts each (a part is played after a random pause).
THEMES = {
    1: ["ripples", "guzheng_city", "shenyang"],
    2: ["liyan", "neolein_472707", "neolein_573972"],
    3: ["cold_to_lukewarm", "misty_mountains", "ishikari_lore"],
    4: ["shenyang", "ripples", "liyan"],
    5: ["guzheng_city", "misty_mountains", "cold_to_lukewarm"],
}


def decode(path, start, length, channels, semitones):
    args = ["ffmpeg", "-v", "error", "-ss", str(start), "-i", str(path)]
    if length is not None:
        args += ["-t", str(length + (1.0 if length else 0))]
    filters = []
    if semitones:
        factor = 2 ** (semitones / 12)
        filters.append(f"asetrate={RATE}*{factor:.6f},aresample={RATE},atempo={1 / factor:.6f}")
    if filters:
        args += ["-af", ",".join(filters)]
    args += ["-ac", str(channels), "-ar", str(RATE), "-f", "f32le", "-"]
    raw = subprocess.run(args, check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype="<f4").reshape(-1, channels).copy()


def rms_db(x):
    r = float(np.sqrt(np.mean(np.square(x)))) if x.size else 0.0
    return 20 * np.log10(max(r, 1e-9))


def loudness_of_original(original_dir, role):
    path = Path(original_dir) / f"{role}.wav"
    if not path.is_file():
        return None
    x = decode(path, 0.0, None, 1, 0)
    # the loud part only: a sound's tail of silence would lower its level
    loud = x[np.abs(x[:, 0]) > 10 ** (-40 / 20)]
    return rms_db(loud if loud.size else x)


def seamless(x, length, fade):
    """A loop of `length` seconds whose end runs into its start: the part after `length` fades over the start."""
    n, f = int(length * RATE), int(fade * RATE)
    n = min(n, len(x) - f)
    out = x[:n].copy()
    ramp = np.linspace(0.0, 1.0, f)[:, None]
    out[:f] = x[:f] * ramp + x[n : n + f] * (1.0 - ramp)
    return out


def trim_silence(x):
    loud = np.where(np.max(np.abs(x), axis=1) > 10 ** (-50 / 20))[0]
    if not loud.size:
        return x
    return x[max(loud[0] - 64, 0) : min(loud[-1] + 2048, len(x))]


def encode(x, path, quality):
    path.parent.mkdir(parents=True, exist_ok=True)
    channels = x.shape[1]
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-f", "f32le", "-ar", str(RATE), "-ac", str(channels), "-i", "-",
         "-c:a", "libvorbis", "-q:a", str(quality), str(path)],
        input=np.ascontiguousarray(x, dtype="<f4").tobytes(), check=True)


def level_to(x, target_db, peak_db=-1.0):
    loud = x[np.max(np.abs(x), axis=1) > 10 ** (-40 / 20)]
    gain = 10 ** ((target_db - rms_db(loud if loud.size else x)) / 20)
    y = x * gain
    peak = float(np.max(np.abs(y))) if y.size else 0.0
    limit = 10 ** (peak_db / 20)
    return y * (limit / peak) if peak > limit else y


def main():
    candidates, original = Path(sys.argv[1]), Path(sys.argv[2])
    made = {}
    for role, src, kind, start, length, semi, ch in SOUNDS:
        x = decode(candidates / src, start, length, ch, semi)
        if kind == "loop":
            x = seamless(x, length, min(0.5, length / 4))
        elif kind == "musicloop":
            x = seamless(x, length, 2.0)
        elif kind == "thunder":
            # the strike: from just before the loudest moment, `length` seconds, faded out
            peak = int(np.argmax(np.max(np.abs(x), axis=1)))
            whole = decode(candidates / src, 0.0, None, ch, semi)
            peak = int(np.argmax(np.max(np.abs(whole), axis=1)))
            a = max(peak - int(0.4 * RATE), 0)
            x = whole[a : a + int(length * RATE)]
            x = x * np.linspace(1.0, 0.0, len(x))[:, None] ** 0.5
        else:
            x = trim_silence(x)
        target = loudness_of_original(original, role)
        if target is not None:
            x = level_to(x, target)
        folder = "Music" if kind.startswith("music") else "Sounds"
        out = HERE / folder / f"{role}.ogg"
        encode(x, out, 5 if ch == 2 else 4)
        made[role] = {"source": src, "seconds": round(len(x) / RATE, 2), "level_db": target}
    # the level music: matched to the original themes' average level
    themes = [loudness_of_original(original, f"Music_Theme_{n}_{k}") for n in range(1, 6) for k in range(1, 4)]
    themes = [t for t in themes if t is not None]
    target = sum(themes) / len(themes) if themes else -20.0
    for name, src in TRACKS:
        # a part of the level's music: at most TRACK_SECONDS, faded in and out (the game holds a part decoded whole)
        x = decode(candidates / src, 0.0, TRACK_SECONDS, 2, 0)[: int(TRACK_SECONDS * RATE)]
        x = x * np.minimum(1.0, np.minimum(np.arange(len(x)) / RATE / 1.0, (len(x) - np.arange(len(x))) / RATE / 5.0))[:, None]
        x = level_to(x, target)
        encode(x, HERE / "Music" / f"{name}.ogg", 4)
        made[name] = {"source": src, "seconds": round(len(x) / RATE, 2), "level_db": round(target, 1)}
    with open(HERE / "Music" / "music.txt", "w", encoding="utf-8") as f:
        f.write("# Which of our tracks plays as which part of the original's music (Music_Theme_<n>_<part>).\n")
        for n, parts in THEMES.items():
            for k, track in enumerate(parts, 1):
                f.write(f"Music_Theme_{n}_{k} {track}.ogg\n")
    print(json.dumps(made, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
