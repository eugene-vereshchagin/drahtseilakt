# audio-synth: the sound synthesizer

The code that made the *neuro* sound theme (`assets/sounds/neuro/`, every effect and piece of music) and the
synthesized rail loops of the *system* theme. Everything is synthesized: oscillators, noise, filters, modal synthesis
(banks of decaying resonators). The original's sounds are only **measured** (length, loudness envelope, spectral
centroid, peaks, decay) to set the parameters; no sample of the original is resampled, cut, filtered or otherwise
copied into what is made here. This project's own code, GPL-3.0-only, as the game.

`NOTES.ru.md` is the working log in Russian (how each role was modelled, the similarity scores, the owner's listening
rounds); its paths are those of the folder it was written in, outside the repository.

## Files

| File | What it does |
|---|---|
| `dsp.py` | primitives: oscillators, envelopes, filters, resonators, modal banks |
| `synth.py` | one generator per effect role |
| `build.py` | renders every effect role, scores it against the original, optional parameter search (`--search N`) |
| `params.json`, `scores.json`, `baseline.json` | the chosen parameters, the scores, the scores against unrelated originals |
| `metric.py` | the similarity score (log-mel spectrum, envelope, MFCC + DTW, duration) |
| `music.py`, `music_end.py` | the level music (`Music_Theme_<n>_<part>`) and the end music |
| `rails.py` | rolling on iron rails (three variants per ball) |
| `to_theme.py` | encodes the rendered files into a theme's layout under the game's names |
| `analyse.py`, `peaks.py`, `track.py`, `diag.py` | measuring the originals and comparing |

## Rebuilding the themes

Needs Python 3 with numpy and scipy, and ffmpeg. The renders are deterministic (seeded): a rebuild gives the shipped
sounds again (checked 2026-11-16: decoded, they correlate 1.0000 with the files in `assets/sounds/neuro/`).

The level of each effect is matched to the original's, so the player's own copy is needed: point `ORIGINAL_SOUNDS` at
its `Sounds` folder (by default the copy the game imported, `~/.local/share/drahtseilakt/levels/original/Sounds`).
The output folders `generated/` and `compare/` stay out of git (`compare/` holds the original's sounds, for listening
at home only: never share it).

```sh
cd tools/audio-synth
export ORIGINAL_SOUNDS=/path/to/Ballance/Sounds
python3 build.py                   # every effect -> generated/<role>.wav (and compare/)
python3 music.py                   # the level and end music -> generated/music/<name>.wav
python3 rails.py /tmp/rails        # the rail loops, variants A-C -> /tmp/rails/Roll_<Ball>_Metal_<V>.ogg
python3 to_theme.py /tmp/neuro --rails /tmp/rails   # -> /tmp/neuro/Sounds/*.ogg, /tmp/neuro/Music/*.ogg
```

Listen, then copy `/tmp/neuro/Sounds` and `/tmp/neuro/Music` over `assets/sounds/neuro/` and update its
`SHA256SUMS` (`sha256sum Sounds/* Music/* > SHA256SUMS` there). The rail loops used are the owner's choices by ear:
wood variant C, stone variant A. `Hit_Sack` is not made here: it is the system theme's (`prepare.py`, `sack_sound`).

The *system* theme takes the same two rail loops: `python3 -I assets/sounds/system/prepare.py ORIGINAL_SOUNDS_DIR
--fixes BASE_SOUNDS --rails /tmp/rails` (see the head of `prepare.py`; it also needs the sources it names).

`python3 build.py ROLE --search 80` searches the parameters of one role again (80 random tries) and saves the better
ones into `params.json`.
