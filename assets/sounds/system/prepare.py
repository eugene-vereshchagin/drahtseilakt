#!/usr/bin/env python3
# Prepares the system sound theme (assets/sounds/system/Sounds and Music) from the openly licensed files the
# owner chose (CREDITS.md lists each source, author, licence and the changes made). It is run by hand, not by the
# build: the sources are not in the repository. Each file is decoded with ffmpeg, cut, made mono or stereo, matched in
# loudness to the original's sound of the same role (the player's copy, given as ORIGINAL) or to our other tracks,
# looped without a seam where the game loops it, and encoded as Ogg Vorbis. Music/music.txt, the table of which track
# plays in which role (several per role: a pool the game picks from), is written from POOLS below.
#
#   python3 -I prepare.py ORIGINAL_SOUNDS_DIR [--earlier CANDIDATES] [--round3 CANDIDATES_3] [--fixes BASE_SOUNDS --rails RAILS]
#
# --earlier: the first rounds' candidates (effects, the finish music and the first nine level tracks);
# --round3: the third round's (2026-11-03: its music/ and music_atmo/ folders, the atmospheres and nine more tracks).
# --fixes (2026-11-12, after the owner's listening notes): reworks four roles from BASE_SOUNDS, a folder holding the
# first round's Hit_Wood_Metal.ogg and Hit_Stone_Metal.ogg (the commit before the fixes has them: git show
# f662ce4:assets/sounds/system/Sounds/<file>), the rail hits being longer and more iron (a modal ring added), and writes
# the fall sound, synthesized (no source: gentle falling glide and a soft thud); --rails: the folder of the synthesized
# rail loops (audio-candidates/rails-2026-11-09: !Roll_Wood_Metal_C.ogg and !Roll_Stone_Metal_A.ogg; or a folder
# freshly made by tools/audio-synth/rails.py, whose files are the same without the "!").
# A round not given is left as it is. Needs ffmpeg and numpy. Writes into the folders beside this file.

import argparse
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
    # Misc_Trafo was changed by hand afterwards (2026-11-12; the owner could not hear the 0.5 s original cut): slowed to
    # 2.5 s (the original's runs 3.1 s, the animation 2.55 s) with ffmpeg atempo=0.5,0.5,0.8, +6 dB (-16 LUFS, as the
    # checkpoint), limited at 0.84, faded out over the last 0.3 s
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

# The third round (2026-11-03): nine more tracks, matched in loudness to the nine above (their mean), cut and faded
# like them; name, source (relative to CANDIDATES_3).
TRACKS_3 = [
    ("gentle_guzheng", "music/512943_6125364-hq.mp3"),
    ("guzheng_loop", "music/520729_197130-hq.mp3"),
    ("blossom_valley", "music/521367_4596312-hq.mp3"),
    ("guzheng_fantasie", "music/541306_4596312-hq.mp3"),
    ("crane_dance", "music/582209_5462375-hq.mp3"),
    ("asian_duet", "music/asian_duet.mp3"),
    ("finding_movement", "music/Finding Movement.mp3"),
    ("orient_peace_valley", "music/orien1_0.ogg"),
    ("senbazuru", "music/Senbazuru.mp3"),
]

# The atmospheres (Music_Atmo): beds made like the original's (26 to 28 s, one slow swell and fade, very quiet).
# name, source (relative to CANDIDATES_3), start s, length s (None: to the end), where the swell peaks (0..1 of the
# length), ffmpeg filter (None: none).
ATMOS = [
    ("atmo_quartz_bowl", "music_atmo/129219_649468-hq.mp3", 60.0, 27.0, 0.45, None),
    ("atmo_tibetan_bowl", "music_atmo/196365_1847371-hq.mp3", 4.0, 27.0, 0.5, None),
    ("atmo_mystic_forest", "music_atmo/559389_2282212-hq.mp3", 42.0, 27.0, 0.4, None),
    ("atmo_reverb_flute", "music_atmo/815787_2520418-hq.mp3", 0.0, None, 0.35, None),
    # the shakuhachi's cicadas are above 3 kHz: cut away
    ("atmo_shakuhachi", "music_atmo/361508_313780-hq.mp3", 1.0, 27.0, 0.4,
     "lowpass=f=2600,lowpass=f=2600,lowpass=f=2600,lowpass=f=2600"),
    # the tonal wind's infrasonic rumble: cut away
    ("atmo_tonal_wind", "music_atmo/397092_3287894-hq.mp3", 26.0, 27.0, 0.4,
     "highpass=f=90,highpass=f=90,highpass=f=90"),
    ("atmo_winter_temple", "music_atmo/612645_2282212-hq.mp3", 20.0, 27.0, 0.4, None),
    ("atmo_wind_synth", "music_atmo/151783_1457270-hq.mp3", 0.0, None, 0.45, None),
]

# The atmospheres' loudness when the original's cannot be measured (its three measure -33, -34 and -30 LUFS).
ATMO_LUFS = -32.5

# Which tracks play in which role: a pool each, the game picks one at random each time (never the same twice in a
# row). The atmospheres: tonal pads (1), airy flutes and wind (2), darker drones (3), like the original's three.
# Themes 1 to 3 keep the first nine tracks; themes 4 and 5 have the third round's.
POOLS = {
    "Music_Atmo_1": ["atmo_quartz_bowl", "atmo_tibetan_bowl", "atmo_mystic_forest"],
    "Music_Atmo_2": ["atmo_reverb_flute", "atmo_shakuhachi", "atmo_tonal_wind"],
    "Music_Atmo_3": ["atmo_winter_temple", "atmo_wind_synth"],
    "Music_Theme_1_1": ["ripples"],
    "Music_Theme_1_2": ["guzheng_city"],
    "Music_Theme_1_3": ["shenyang"],
    "Music_Theme_2_1": ["liyan"],
    "Music_Theme_2_2": ["neolein_472707"],
    "Music_Theme_2_3": ["neolein_573972"],
    "Music_Theme_3_1": ["cold_to_lukewarm"],
    "Music_Theme_3_2": ["misty_mountains"],
    "Music_Theme_3_3": ["ishikari_lore"],
    "Music_Theme_4_1": ["gentle_guzheng", "guzheng_loop"],
    "Music_Theme_4_2": ["senbazuru"],
    "Music_Theme_4_3": ["finding_movement", "orient_peace_valley"],
    "Music_Theme_5_1": ["blossom_valley"],
    "Music_Theme_5_2": ["guzheng_fantasie", "asian_duet"],
    "Music_Theme_5_3": ["crane_dance"],
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


def lufs(x):
    """The integrated loudness (EBU R 128) of a signal, by ffmpeg's meter."""
    out = subprocess.run(
        ["ffmpeg", "-nostats", "-f", "f32le", "-ar", str(RATE), "-ac", str(x.shape[1]), "-i", "-",
         "-af", "ebur128", "-f", "null", "-"],
        input=np.ascontiguousarray(x, dtype="<f4").tobytes(), capture_output=True, check=True).stderr.decode()
    values = [line.split()[1] for line in out.splitlines() if line.strip().startswith("I:")]
    return float(values[-1]) if values else -70.0


def lufs_of_file(path, channels=2):
    return lufs(decode(path, 0.0, None, channels, 0))


def to_lufs(x, target, peak_db=-1.0):
    y = x * 10 ** ((target - lufs(x)) / 20)
    peak = float(np.max(np.abs(y))) if y.size else 0.0
    limit = 10 ** (peak_db / 20)
    return y * (limit / peak) if peak > limit else y


def track(x):
    """A part of the level's music: at most TRACK_SECONDS, faded in over 1 s and out over 5 s."""
    x = x[: int(TRACK_SECONDS * RATE)]
    t = np.arange(len(x)) / RATE
    return x * np.minimum(1.0, np.minimum(t / 1.0, (len(x) / RATE - t) / 5.0))[:, None]


def swell(x, peak):
    """One slow swell and fade over the whole bed, as the original's atmospheres: up to `peak` (0..1), then down."""
    t = np.linspace(0.0, 1.0, len(x))
    up = np.sin(np.pi / 2 * np.clip(t / peak, 0.0, 1.0)) ** 2
    down = np.cos(np.pi / 2 * np.clip((t - peak) / (1.0 - peak), 0.0, 1.0)) ** 2
    return x * np.where(t < peak, up, down)[:, None]


def decode_filtered(path, start, length, flt):
    args = ["ffmpeg", "-v", "error", "-ss", str(start), "-i", str(path)]
    if length is not None:
        args += ["-t", str(length)]
    if flt:
        args += ["-af", flt]
    args += ["-ac", "2", "-ar", str(RATE), "-f", "f32le", "-"]
    raw = subprocess.run(args, check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype="<f4").reshape(-1, 2).copy()


def earlier(candidates, original, made):
    for role, src, kind, start, length, semi, ch in SOUNDS:
        x = decode(candidates / src, start, length, ch, semi)
        if kind == "loop":
            x = seamless(x, length, min(0.5, length / 4))
        elif kind == "musicloop":
            x = seamless(x, length, 2.0)
        elif kind == "thunder":
            # the strike: from just before the loudest moment, `length` seconds, faded out
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
        x = track(decode(candidates / src, 0.0, TRACK_SECONDS, 2, 0))
        x = level_to(x, target)
        encode(x, HERE / "Music" / f"{name}.ogg", 4)
        made[name] = {"source": src, "seconds": round(len(x) / RATE, 2), "level_db": round(target, 1)}


def round3(candidates, original, made):
    # the tracks: as loud as the first nine are on average (measured in LUFS, their files as made)
    first = [lufs_of_file(HERE / "Music" / f"{name}.ogg") for name, _ in TRACKS]
    target = sum(first) / len(first)
    for name, src in TRACKS_3:
        x = track(decode(candidates / src, 0.0, TRACK_SECONDS, 2, 0))
        x = to_lufs(x, target)
        encode(x, HERE / "Music" / f"{name}.ogg", 4)
        made[name] = {"source": src, "seconds": round(len(x) / RATE, 2), "lufs": round(lufs(x), 1)}
    # the atmospheres: as loud as the original's on average
    measured = [lufs_of_file(Path(original) / f"Music_Atmo_{k}.wav") for k in range(1, 4)
                if (Path(original) / f"Music_Atmo_{k}.wav").is_file()]
    target = sum(measured) / len(measured) if measured else ATMO_LUFS
    for name, src, start, length, peak, flt in ATMOS:
        x = decode_filtered(candidates / src, start, length, flt)
        x = swell(x, peak)
        x = to_lufs(x, target)
        encode(x, HERE / "Music" / f"{name}.ogg", 3)
        made[name] = {"source": src, "seconds": round(len(x) / RATE, 2), "lufs": round(lufs(x), 1)}


# --- the fixes of 2026-11-12 -------------------------------------------------------------------------------------
# Iron ring: the partials (Hz) of a struck iron bar, their decay times (s) and levels. Inharmonic on purpose: a bell-like
# harmonic series sounds like a glass, these like a rail or a pipe. Kept below 2 kHz so the ring is not harsh.
IRON = [(310.0, 0.95, 1.0), (548.0, 0.75, 0.7), (905.0, 0.6, 0.5), (1380.0, 0.45, 0.3), (1960.0, 0.3, 0.15)]
RING_SECONDS = 0.9


def iron_hit(x, semitones, seconds=RING_SECONDS):
    """The hit `x` (mono) as if it struck iron: its own knock, a ring of IRON's partials excited by it (the
    partials as damped sines), a soft low pass at 5 kHz, the whole `seconds` long, faded out, peak -3 dB."""
    n = int(seconds * RATE)
    t = np.arange(n) / RATE
    k = 2 ** (semitones / 12)
    # the ring: the partials struck at the moment of the hit (a sine each, the same phases every run), their sum a
    # little under the knock's own peak, so that it rings audibly for most of the 0.9 s
    ir = sum(a * np.exp(-6.9 * t / (1.4 * d)) * np.sin(2 * np.pi * f * k * t + 0.7 * i) for i, (f, d, a) in enumerate(IRON))
    ir *= np.minimum(1.0, t / 0.002)
    src = x[:, 0]
    ring = ir * 0.5 * float(np.max(np.abs(src))) / float(np.max(np.abs(ir)))
    body = np.zeros(n)
    body[: min(len(src), n)] = src[:n] * 0.6
    y = body + ring
    y = np.convolve(y, np.hanning(9) / np.hanning(9).sum(), mode="same")  # about -3 dB at 5 kHz: no needle
    y *= np.minimum(1.0, (n - np.arange(n)) / (0.2 * RATE))
    y *= 10 ** (-3.0 / 20) / np.max(np.abs(y))
    return y[:, None].astype("<f4")


def fall_sound():
    """The ball is lost: a gentle falling glide (490 down to 90 Hz, a sine with a soft octave) that grows quieter, then a
    low soft thud at its end. 1.9 s, nothing above 1 kHz but the glide's own octaves, peak -3 dB, -22 LUFS."""
    n = int(1.9 * RATE)
    t = np.arange(n) / RATE
    f = 90 + 400 * np.exp(-t / 0.5)
    phase = 2 * np.pi * np.cumsum(f) / RATE
    env = np.minimum(1.0, t / 0.04) * np.exp(-t / 0.8) * np.minimum(1.0, (1.9 - t) / 0.3)
    glide = (np.sin(phase) + 0.6 * np.sin(2 * phase) + 0.2 * np.sin(3 * phase)) * env
    td = t - 0.95
    thud = np.where(td > 0, np.exp(-td / 0.16) * (np.sin(2 * np.pi * (60 + 30 * np.exp(-td / 0.05)) * td) + 0.3 * np.sin(2 * np.pi * 180 * td)), 0.0)
    y = glide * 0.55 + thud * 0.7
    y *= np.minimum(1.0, (1.9 - t) / 0.4)
    return to_lufs(y[:, None].astype("<f4"), -22.0, peak_db=-3.0)


def sack_sound():
    """The swinging sack hits the ball: a soft, low thud of cloth and sand, as the sound of a body in a sack: an 85 Hz
    knock falling from 130 to 85 Hz, a short low-passed (600 Hz) rustle, 0.4 s, nothing above 700 Hz to speak of, peak -3 dB."""
    n = int(0.4 * RATE)
    t = np.arange(n) / RATE
    knock = np.sin(2 * np.pi * (85 + 45 * np.exp(-t / 0.04)) * t) * np.exp(-t / 0.09)
    noise = np.random.default_rng(26).standard_normal(n)
    kernel = np.hanning(int(RATE / 600 * 2))
    rustle = np.convolve(noise, kernel / kernel.sum(), mode="same") * np.exp(-t / 0.05)
    y = knock + 0.5 * rustle / np.max(np.abs(rustle))
    y *= np.minimum(1.0, t / 0.003) * np.minimum(1.0, (0.4 - t) / 0.1)
    y *= 10 ** (-3.0 / 20) / np.max(np.abs(y))
    return y[:, None].astype("<f4")


def fixes(base, rails, made):
    y = sack_sound()
    encode(y, HERE / "Sounds" / "Hit_Sack.ogg", 4)
    made["Hit_Sack"] = {"source": "synthesized", "seconds": round(len(y) / RATE, 2)}
    for role, semitones in (("Hit_Wood_Metal", 0), ("Hit_Stone_Metal", -3)):
        x = decode(Path(base) / f"{role}.ogg", 0.0, None, 1, 0)
        y = iron_hit(x, semitones)
        encode(y, HERE / "Sounds" / f"{role}.ogg", 4)
        made[role] = {"source": f"{role}.ogg (first round) + iron ring", "seconds": round(len(y) / RATE, 2)}
    y = fall_sound()
    encode(y, HERE / "Sounds" / "Misc_Fall.ogg", 4)
    made["Misc_Fall"] = {"source": "synthesized", "seconds": round(len(y) / RATE, 2), "lufs": round(lufs(y), 1)}
    if rails:
        for role, src in (("Roll_Wood_Metal", "!Roll_Wood_Metal_C.ogg"), ("Roll_Stone_Metal", "!Roll_Stone_Metal_A.ogg")):
            # the owner marked his picks with "!"; a fresh tools/audio-synth/rails.py folder has them unmarked
            path = Path(rails) / src
            y = decode(path if path.exists() else Path(rails) / src.lstrip("!"), 0.0, None, 1, 0)
            encode(y, HERE / "Sounds" / f"{role}.ogg", 4)
            made[role] = {"source": f"rails-2026-11-09/{src} (synthesized)", "seconds": round(len(y) / RATE, 2),
                          "lufs": round(lufs(y), 1)}


def write_table():
    with open(HERE / "Music" / "music.txt", "w", encoding="utf-8") as f:
        f.write("# Which of our tracks plays in which role of the original's music: a pool per role, the game picks one\n")
        f.write("# at random each time it plays the role (docs/drahtseilakt/sound-themes.md). Written by prepare.py.\n")
        for role, names in POOLS.items():
            f.write(f"{role} {' '.join(n + '.ogg' for n in names)}\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("original")
    ap.add_argument("--earlier", type=Path)
    ap.add_argument("--round3", type=Path)
    ap.add_argument("--fixes", type=Path)
    ap.add_argument("--rails", type=Path)
    args = ap.parse_args()
    made = {}
    if args.earlier:
        earlier(args.earlier, args.original, made)
    if args.round3:
        round3(args.round3, args.original, made)
    if args.fixes:
        fixes(args.fixes, args.rails, made)
    write_table()
    print(json.dumps(made, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
