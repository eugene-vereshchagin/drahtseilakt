# Menus: the original's scheme and ours

The plan for the game's menus, agreed with the owner on 2026-11-02. Until the player turns the extended mode on,
the game is the original as closely as we can make it; the only visible difference is one more option that
turns the extended mode on. The extended mode then has its own settings: what to show, which of our additions
to use, and community maps.

## The original's menus

Read from the copy's `3D Entities/Menu.nmo` (its scripts `Menu_*`) and `Language.nmo` (the strings, in German,
English, Spanish, Italian and French; `GameSettings.Language` chooses the column), and checked in the C game.

| Screen | Lines, top to bottom | Notes |
|---|---|---|
| Main (`Menu_Main`) | Start, Highscore, Options, Credits, Exit | Exit asks "Do you want to quit the game?" (Yes / No) |
| Start (`Menu_Start`) | Level 1 … Level 12, Back | a level is playable once the one before it is finished (`DB_Levelfreischaltung`); the others are grey. The level 13 add-on replaces Menu.nmo and adds its line |
| Highscore (`Menu_Highscore`) | "Highscore Level N", ten names with points, Back; left and right change the level | the copy's defaults are the arrays `DB_Highscore_LvNN` in base.cmo ("Mr. Default") |
| Options (`Menu_Options`) | Graphics, Controls, Sound, Back | |
| Graphics (`Menu_Opt_Graphics`) | Screen Resolution, Synch to Screen? (Yes/No), Clouds? (Yes/No), OK | |
| Controls (`Menu_Opt_Keys`) | Forward, Backward, Left, Right, Rotation, Overview: one key each; Invert Rotation? (Yes/No); OK | defaults: arrows, Shift, Space |
| Sound (`Menu_Opt_Sound`) | Music Volume, OK | one volume |
| Credits (`Menu_Credits`) | the original's credits | |
| Pause, Esc in a level (`Menu_Pause`) | Back, Restart Level, Highscore, Options, Exit Level | Restart and Exit ask Yes / No first; the points stay on screen |
| Dead, no lives left (`Menu_Dead`) | Restart Level, Home | |
| Score at the finish (`Menu_Score`) | Level Bonus, Time Points, Extra Lives, Score | counted up line by line |
| New highscore (`Menu_HighscoreEntry`) | "New highscore entry!", the points, a name to type, OK | the last name typed is offered (`DB_Options.LastPlayer`) |
| End of a level (`Menu_End`) | Next level, Restart Level, Highscore, Options, Home | |

Settings and highscores live in `Database.tdb` in the game folder. There are no player profiles: the only
per-player thing is the name in the highscore list.

## Ours: the classic mode (the default)

- The same screens, lines, order and strings as the table above, drawn with the copy's own textures and fonts.
  The strings come from the copy's `Language.nmo`, so the menus speak the copy's language.
- **One addition**: Options has a fourth button, *Extended*; its page has the switch *Extended mode?* (Yes / No).
  That is the only difference a player of the original levels sees.
- Levels unlock as in the original. Highscores are the original's: ten per level, points, a name. They start
  from the copy's defaults and are saved in our own data folder, never in the game's files. Reading the player's
  `Database.tdb` to take over their old scores is a later option (a parser: reviewed like every file reader).
- Settings that the original has and we don't yet (screen resolution, synch, clouds, keys) are added as
  their behaviour is built; until then their lines show the current value and cannot be changed.
- Every finished run is still kept as a replay (it costs nothing on screen); the fastest counts as the best time
  shown in the extended mode.

## Ours: the extended mode

Turned on in Options, or for one run with `--debug`. The menus keep the original's look; what changes:

- **Main menu**: Start, Highscore, Options, Credits, Exit, as the original; *Start* first asks which levels.
- **Options**: the *Extended* page has the mode's switch and, under it, one switch for each of its additions
  (all on with the mode; the settings keep those turned off as `extended.<key> off`); the debug ones (practice tools,
  endless lives) are on its *Debug mode* screen, the others on the page itself:
  - all levels open, without unlocking;
  - best times beside the levels;
  - levels by kind under Start;
  - Interface: own / original / remaster, where the interface's files come from ([resources.md](resources.md));
  - practice tools: free camera (F2), slower and faster (`[` `]`), save the moment and go back to it (F5 / F9),
    five seconds back (F3, again for further, up to half a minute), the sector and tick on screen; a run helped this
    way is kept but never a best run;
  - the sound volume beside the music volume;
  - endless lives: the last ball is never lost (a run it saves is never a best run). There is no time limit to lift:
    the points only count down and stop at zero, as in the original.
  More switches come with the features in the roadmap (field of view, timers, the ghost of the best run).
- **Levels by kind**: Start offers *Original* (the classic list of levels), *Remastered*, *Community maps* and
  *Open format*, the folders of `levels/` ([levels.md](levels.md)). In each, its levels by file name, then the folders
  holding more (Enter opens one, Esc goes back up), played like an original level. A level that cannot be loaded says
  why and is not started; its runs and best times are kept apart from the original levels. Later: title, author and
  difficulty from the community catalogue.
- Texture packs and our own mods come here too when they exist, each behind its own switch and a warning that
  third-party content is played at the player's own risk.

## Profiles

The original has none (above). For us, a profile would bundle a name, settings, unlocked levels, highscores
and replays, so that several people can share one computer. It is not needed for the classic mode, which
keeps the original's single name in the highscore entry. Open question for the owner: whether profiles come
later as an extended-mode feature.

## Order of work

1. Done (2026-11-02): classic by default; the main menu and Options as in the table; the *Extended* page.
2. Done: the pause menu with its Yes / No questions; the dead menu; the end menu.
3. Done: highscores with the name entry; levels unlocking.
4. Done: the custom levels list (levels.md) and the extended page's switches.
5. Graphics and Controls settings as their behaviour is built (now shown, not changeable).

`DRAHTSEILAKT_SHOT_MENU=base:keys` with `--screenshot` draws a menu over the frame without a window (base `main`,
`pause`, `dead` or `end`; keys `u d l r` for the arrows, `e` Enter, `b` Esc, `n` the name entry), to look at
every screen.
