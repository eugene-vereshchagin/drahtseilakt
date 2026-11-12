# Sound themes

Every sound and every piece of music in Drahtseilakt comes from a **sound theme**: a folder of files named by the
**roles** the game plays (the fall, a checkpoint, the wooden ball rolling on stone, a level's music...). The game
ships two themes and anyone can make another one: a different set of recordings, a different kind of music, a
remaster. This page is for theme authors: what goes in the folder, what each role is, how loud and long the
original's sound for it is, and how to share a theme so others can use it.

## Where themes live

- **Shipped** with the game, in `assets/sounds/`: *system* (openly licensed recordings and music, the theme every other
  theme falls back to) and *neuro* (sounds and music synthesized for the game, the default).
- **The player's own**, in the data folder's `sounds/<name>/`: `~/.local/share/drahtseilakt/sounds/` on Linux,
  `%APPDATA%\drahtseilakt\sounds\` on Windows, `~/Library/Application Support/drahtseilakt/sounds/` on macOS.
  To install a theme, copy its folder there.

The folder's name is the theme's key (what the settings save): letters, digits, `_`, `-` and `.`, not beginning with a
dot, at most 64 characters. A player's folder named like a shipped theme (`system`, `neuro`) is ignored; `original` is
the name the toolbox gives the theme it makes from the player's own copy of Ballance. At most 32 themes are taken.
The game looks for themes when it starts: restart it after adding one.

## The folder

```
sounds/
  calm/
    theme.txt          name, author, licence, fallback
    CREDITS.md         who made each file, under which licence (recommended)
    Sounds/            the effects: Misc_Fall.ogg, Hit_Wood_Wood.ogg, ...
      sounds.txt       optional: more files per role (pools)
    Music/             the music: Music_Atmo_1.ogg, Music_Theme_1_1.ogg, ...
      music.txt        optional: more files per role (pools), or files named as you like
```

A role's file is `Sounds/<role>.ogg` or `.wav` (or the same in `Music/`; the game looks in both). A theme does not have
to fill every role: what it lacks comes from its fallback theme (see `theme.txt`).

Only plain files are read: names of letters, digits, `_`, `-` and `.`; links (symbolic links, junctions) are never
followed; sub-folders other than `Sounds/` and `Music/` are ignored.

## theme.txt

Plain text, UTF-8, at most 4 KiB, one `key value` per line; lines beginning with `#` are comments.

| Key | Meaning |
|---|---|
| `name.en`, `name.ru`, `name.de`, `name.zh` | The theme's name in the settings, per language (at most 64 characters). A language without one shows the English name, else the folder's name. |
| `author` | Who made it (for people; the game does not show it yet). |
| `description.en`, `description.ru`, ... , `description` | A short text the notes panel of the Sound theme line shows when the theme is chosen (the menu's language, else `description.en`, else `description`; at most 220 characters). |
| `licence` | Its licence, e.g. `CC0 1.0` or `CC-BY 4.0, per file in CREDITS.md` (for people). |
| `fallback` | The theme a role this theme lacks comes from: a theme's key, or `none` for no other theme (the theme is complete). Without it, *system*. |

```
# Calm: soft recordings for long levels.
name.en Calm
name.ru Тихая
name.de Ruhig
name.zh 平静
author Jane Doe
licence CC-BY 4.0 (CREDITS.md)
fallback system
```

The order a role is looked for in: the chosen theme, its fallback, then *system* (`fallback none`: the chosen theme
only). A role's files all come from one theme: the first that has the role at all. A sound none of them has comes
from the level's own sounds or the player's copy (effects only); music never does.

## Roles

The original's figures below are measured on its files: length, and integrated loudness (EBU R 128, LUFS) where the
sound is long enough to measure; very short sounds are given as such. They are a guide, not a rule: a theme may be
louder or softer as a whole, but its sounds should keep about these proportions to one another, as the game mixes
them as the original did (the rolling sound's volume and speed follow the ball's speed, for instance).

**Loop** means the game plays the file over and over: it must loop without a seam (its end running into its start,
for example by a crossfade). Every other sound is a **one-shot**, played once.

### Effects (`Sounds/`)

| Role | When it plays | Kind | Original |
|---|---|---|---|
| `Misc_StartLevel` | A run begins | one-shot | 4.2 s, −18 LUFS |
| `Misc_Checkpoint` | A checkpoint is reached | one-shot | 3.5 s, −16 LUFS |
| `Misc_Fall` | The ball is lost | one-shot | 5.6 s, −18 LUFS |
| `Misc_Lightning` | A new ball appears | one-shot | 3.5 s, −25 LUFS |
| `Misc_Trafo` | A transformer changes the ball | one-shot | 3.1 s, −22 LUFS (the system theme's: 2.5 s, −16 LUFS) |
| `Extra_Life_Blob` | The extra life is touched | one-shot | 0.3 s, short |
| `Misc_extraball` | The extra life is gained (0.3 s after `Extra_Life_Blob`) | one-shot | 0.9 s, short |
| `Extra_Start` | Extra points are released | one-shot | 3.0 s, −28 LUFS |
| `Extra_Hit` | One extra point is collected | one-shot | 0.5 s, −36 LUFS |
| `Misc_RopeTears` | A rope breaks | one-shot | 0.2 s, short |
| `Misc_Ventilator` | A fan blows (volume follows the distance) | loop | 1.1 s, −23 LUFS |
| `Music_thunder` | Thunder after a lightning flash in stormy levels (an effect despite its name: the sound volume) | one-shot | 12 s, −18 LUFS |
| `Menu_click` | A menu choice | one-shot | 0.03 s, short |
| `Menu_load` | A level is loading | one-shot | 5.2 s, −19 LUFS |
| `Roll_<ball>_<floor>` | The ball rolls (volume and speed follow the ball's speed) | loop | 1–2 s, −27 to −41 LUFS |
| `Hit_<ball>_<floor>` | The ball hits the floor or a wall | one-shot | 0.1–0.3 s, short |

The balls: `Wood`, `Stone` and `Paper`; the floors: `Stone`, `Wood`, `Metal`. Rolling: `Roll_Wood_Stone`,
`Roll_Wood_Wood`, `Roll_Wood_Metal`, `Roll_Stone_Stone`, `Roll_Stone_Wood`, `Roll_Stone_Metal`, and `Roll_Paper` for the
paper ball on every floor. Hits: `Hit_Wood_Stone`, `Hit_Wood_Wood`, `Hit_Wood_Metal`, `Hit_Stone_Stone`,
`Hit_Stone_Wood`, `Hit_Stone_Metal`, the domes `Hit_Wood_Dome` (2.0 s, −18 LUFS) and `Hit_Stone_Kuppel` (1.2 s,
−21 LUFS), and `Hit_Paper` for the paper ball. The rolling loops of the original, from quiet to loud: paper and wood on
stone about −41, wood on metal −37, stone on stone −36, stone on wood −29, wood on wood −27, stone on metal −28 LUFS.

A level may bring its own effects (a community level's `Sounds/` folder); they come before any theme's.

### Music (`Music/`)

The music has its own volume setting. How a level plays it (as the original's music manager does):

| Role | When it plays | Kind | Original |
|---|---|---|---|
| `Music_Atmo_1`, `_2`, `_3` | The atmosphere: from the start of a run, one of them after a random pause of up to 15 s, again and again | one-shot | 26–28 s, −30 to −34 LUFS, one slow swell and fade, no rhythm or bass |
| `Music_Theme_<n>_1`, `_2`, `_3` | The level's theme (`n` from the level table, 1 to 5 in the original levels): from 7 s on, one of its parts after a random pause of up to 50 s, again and again; it stops after the last checkpoint | one-shot | 4–23 s, −13 to −19 LUFS |
| `Music_EndCheckpoint` | After the last checkpoint, within reach of the level's end, until the finish | loop | 14 s, −13 LUFS |
| `Music_Final` | The level is finished | one-shot | 26.5 s, −16 LUFS |
| `Music_LastFinal` | The last level is finished (instead of `Music_Final`) | one-shot | 7.7 s, −17 LUFS |

The atmosphere and the theme play at the same time, each with its own pauses, so the atmosphere should be very quiet
and calm. The original's theme parts are short phrases; longer pieces work too (*system*'s are whole tracks of up to
100 s at about −17 LUFS), but each piece is held decoded in memory while a level plays (about 35 MB for 100 s of
stereo), so keep them under two minutes or so. At most 384 MB of decoded music is held at once; a piece
that would go over it, or that cannot be decoded, is left out of its pool for the run and another one plays. `Music_Highscore` (the original's 2 s jingle) is accepted but not
played yet.

## Pools: several files for one role

A role can have several files, a **pool**: each time the game plays the role it picks one of them at random, never the
same one twice in a row. A pool is given by the tables `Music/music.txt` and `Sounds/sounds.txt`, both the same format:

```
# role file [file ...]
Music_Atmo_1 bowl.ogg flute.ogg
Music_Atmo_1 wind.ogg
Music_Theme_4_1 river.ogg
Music_Theme_4 garden.ogg dusk.ogg
Music_Final fanfare_a.ogg fanfare_b.ogg
```

- One line per role, the role first, then one or more file names separated by spaces or tabs. A role on several lines
  gathers all of them. The old one-file lines (`Music_Theme_1_1 piece.ogg`) are the same format.
- The file named like the role (`Music/Music_Atmo_1.ogg`), if there is one, is the first of its pool; the tables add
  to it. A table names files of its own folder only: `music.txt` those in `Music/`, `sounds.txt` those in `Sounds/`.
- The atmosphere and the level theme choose among **all** their parts' files together: the atmosphere among the pools
  of `Music_Atmo_1`, `_2` and `_3` and of the group `Music_Atmo`; theme `n` among those of `Music_Theme_<n>_1` to `_3`
  and of the group `Music_Theme_<n>`. So a theme can have any number of atmospheres and theme tracks, sorted into parts
  or not. A file in several of them counts once.
- `Music_EndCheckpoint`, `Music_Final` and `Music_LastFinal` pick one of their pool each level; every effect picks one
  of its pool each time it plays (several hit sounds, for example). A level's own effect is played instead of the pool.
- The music's picks follow a seed: a replay plays its music in the same order every time, a new run in a new one. The
  music never changes the game itself.
- Role names are matched without regard to case; file names exactly.

The tables are read as untrusted files (themes are shared by strangers) and checked strictly:

- at most 16 KiB and 512 lines, in UTF-8 (a byte order mark at the start is allowed); a table over these or not UTF-8 is ignored whole;
- file names are plain names ending in `.ogg` or `.wav` (letters, digits, `_`, `-`, `.`, not beginning with a dot): no
  folders, paths or drive letters. A line naming anything else is passed over whole;
- a role the game does not know is ignored; at most 16 files per role and 256 roles;
- a file named but not there (or a link) is left out.

Every line passed over is reported on the terminal (standard error), e.g.
`sound theme table .../Music/music.txt: line 3: My_Song is not a role the game plays: ignored`.

## Formats

- **Ogg Vorbis** (`.ogg`, preferred) or uncompressed PCM **WAV** (`.wav`, 8 or 16 bit); mono or stereo; any sample rate
  from 1 to 192 kHz (44.1 kHz recommended; the game resamples). Mono is enough for effects, stereo suits music.
- A file is at most 64 MB and decodes to at most ten minutes. A broken file is passed over for the next theme's.
- Ogg Vorbis quality 3–5 (`ffmpeg -c:a libvorbis -q:a 4`) is plenty.
- Match loudness by measuring, e.g. `ffmpeg -i file.ogg -af ebur128 -f null -` (the last `I:` is the integrated
  loudness). *system*'s `prepare.py` (in `assets/sounds/system/`) shows a complete preparation: cutting, fades,
  seamless loops, loudness matching and encoding.

## Testing a theme

1. Put the folder in the data folder's `sounds/` and start the game.
2. Choose the theme on the **Sound theme** line of the Sound page (in both modes; the setting `sound-theme NAME` in
   `settings.txt`). The click you hear on changing it already plays with the new theme. Without a copy of the original
   game the `original` theme is not offered, and a saved `original` choice plays our default instead.
3. Play a level; watch the terminal for table notes. A missing role is filled by the fallback theme, so a theme with
   `fallback none` should be checked for gaps: every role in the tables above.
4. To listen without playing, run a replay with the mix written to a file:
   `DRAHTSEILAKT_SOUND_FILE=mix.wav drahtseilakt --replay run.replay` (the mix goes to `mix.wav` instead of the speakers).

## Licensing and sharing

- Use your own recordings and music, or works under **CC0** or **CC-BY** (3.0 or 4.0), so that everyone can share
  the theme. Avoid NonCommercial (NC) and NoDerivatives (ND) licences: the game cuts and levels every file.
- Put a `CREDITS.md` in the theme: for each file, its role, the work's title, author, licence, source URL and what you
  changed (cut, faded, levelled, filtered). CC-BY requires this attribution; *system*'s `CREDITS.md` is an example.
- **Never include the original game's sounds or music** (or any other game's), not even edited. The original's sounds
  stay on each player's machine: the toolbox makes the *original* theme from the player's own copy, locally, and it is
  never shared.

## The shipped themes

- *system* (`assets/sounds/system/`): openly licensed effects (Kenney, freesound and OpenGameArt authors), eight
  atmospheres in three parts, eighteen level tracks (themes 1 to 3 three each, themes 4 and 5 nine more in pools) and
  the finish music; made by `prepare.py`; authors, licences and changes in its `CREDITS.md`.
- *neuro* (`assets/sounds/neuro/`): every role synthesized by code for the game, GPL-3.0-only, `fallback none`.

**Shown names.** The theme `original` is shown as **Ballance** in every language; *neuro* and *system* keep their names. A player's theme shows its `name.<lang>` (or its folder's name) in **at most 16 letters**: a longer name is cut at a word and ends in "…", and the game logs a warning. The folder key's limit (64 characters) stays.
