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
| Main without a copy (ours) | Start, Import, Highscore, Options, Credits, Exit | Start lists "Our levels" (the shipped ones and `levels/open/`); Import opens the window that brings in the player's copy, then the game starts again with it (decisions.md, "The game plays out of the box") |
| Start (`Menu_Start`) | Level 1 … Level 12, Back | a level is playable once the one before it is finished (`DB_Levelfreischaltung`); the others are grey. The level 13 add-on replaces Menu.nmo and adds its line |
| Highscore (`Menu_Highscore`) | "Highscore Level N", ten names with points, Back; left and right change the level | the copy's defaults are the arrays `DB_Highscore_LvNN` in base.cmo ("Mr. Default") |
| Options (`Menu_Options`) | Graphics, Controls, Sound, Back | ours adds Language and Extended before Back |
| Graphics (`Menu_Opt_Graphics`) | Screen Resolution, Synch to Screen? (Yes/No), Clouds? (Yes/No), OK | the copy's defaults (`DB_Options`): no synch, clouds |
| Controls (`Menu_Opt_Keys`) | Forward, Backward, Left, Right, Rotation, Overview: one key each; Invert Rotation? (Yes/No); OK | defaults: arrows, Shift, Space; `DB_Options` keeps a key as its row in `Language.nmo`'s `all_keys` (DirectInput's scan codes in order) |
| Sound (`Menu_Opt_Sound`) | Music Volume, Sound Volume, OK | the music's and the effects' volume (ours shows both in every mode, as the owner knows the original) |
| Credits (`Menu_Credits`) | the original's credits | |
| Pause, Esc in a level (`Menu_Pause`) | Back, Restart Level, Highscore, Options, Exit Level | Restart and Exit ask Yes / No first; the points stay on screen |
| Dead, no lives left (`Menu_Dead`) | Restart Level, Home | |
| Score at the finish (`Menu_Score`) | Level Bonus, Time Points, Extra Lives, Score | counted up line by line |
| New highscore (`Menu_HighscoreEntry`) | "New highscore entry!", the points, a name to type, OK | the last name typed is offered (`DB_Options.LastPlayer`) |
| End of a level (`Menu_End`) | Next level, Restart Level, Highscore, Options, Home | |

Settings and highscores live in `Database.tdb` in the game folder. There are no player profiles: the only
per-player thing is the name in the highscore list.

## Ours: the classic mode (the default)

- The same screens, lines, order and strings as the table above, drawn with our own interface (decisions.md, "The
  own interface is the default"; [resources.md](resources.md), "The own interface"): our textures, our font, our
  texts. The extended mode's Interface switch can choose the original's, drawn with the copy's textures and font and
  speaking its `Language.nmo`.
- **Two additions** in Options: *Language* and *Extended*. *Language* opens a page with the languages, each in its own
  name (English, Русский, Deutsch, 简体中文, Français, Español, Italiano), the one spoken in gold; Enter speaks the
  chosen one at once and saves it (`language ru`). Until the player chooses, the menus speak the system's language
  (the locale variables `LC_ALL`, `LC_MESSAGES`, `LANG`; Windows and macOS set none for a program started from the
  desktop) if it is one of these, else English. *Extended* has the switch *Extended mode?* (Yes / No).
  That is all a player of the original levels sees changed.
- Levels unlock as in the original. Highscores are the original's: ten per level, points, a name. They start
  from our own default lists (the copy's with the original's interface) and are saved in our own data folder, never in the game's files. Reading the player's
  `Database.tdb` to take over their old scores is a later option (a parser: reviewed like every file reader).
- **Graphics**: the original's three lines and one more, *Fullscreen?* (No / Yes / Exclusive), as a window can
  be one. Left and right (or Enter) change a line; it takes effect at once and is saved in `settings.txt`:
  - *Screen Resolution* goes through the monitor's modes and the usual window sizes (`resolution 1600x900`); the
    window takes that size, an exclusive full screen that mode;
  - *Synch to Screen?* lets the frames wait for the monitor (`vsync on`; off by default, as in the copy, and
    because waiting stalled on the test machine; `DRAHTSEILAKT_PRESENT` still comes first);
  - *Clouds?* shows the levels' cloud layer (`clouds on|off`), their `SkyLayer` object, which `Gameplay.nmo` shows
    when the option is on. Its texture is grey clouds on black and its material does not blend, so we draw it added
    to the sky behind it (our reading; the copy's material does not say). The video tracking draws without it;
  - `cloud-quality low|medium|high|off` (the settings file; `medium` if not given): how finely our levels' sea of
    cloud is drawn (levels.md, "The sea of cloud"): steps through it, steps towards the sun, small billows on its
    edges; `off` draws none, and *Clouds?* off hides it too;
  - *Fullscreen?* (`fullscreen off|borderless|exclusive`): borderless takes the desktop's mode, exclusive changes
    it where the platform can (Wayland cannot: borderless). F11 switches between a window and borderless.
- **Controls**: Enter on a key's line waits for a key ("Press a key"); the key pressed is bound, Esc gives up.
  A key another action has is swapped onto it, so no key does two things. Esc and F11 are the game's and cannot be
  bound; a bound key comes before the extended mode's practice keys. Invert Rotation? turns the camera the other way.
  The keys are saved as `key.forward ArrowUp` … `key.overview Space` and `invert-rotation on|off` (winit's key
  names); without them the copy's `DB_Options` keys count. Either Shift does what the bound one does unless the
  other is bound itself. The page shows a key by the copy's name for it, else ours.
- **Gamepads** (gilrs) press the same keys as the keyboard, so a run and its replay are the same whatever played
  it: the left stick or the cross rolls the ball, the left and right shoulder buttons turn the camera (Rotation
  with left or right), the lower face button (A / ✕) is Overview, Start pauses. In the menus the cross moves, the
  lower face button is Enter, the right one (B / ○) Esc. Their buttons cannot be rebound yet.
- **The mouse** works every menu, as in the original: the button under the cursor is chosen (only when the cursor
  moves, so a resting cursor never fights the keys), a left click presses it as Enter does, the right button and the
  mouse's back button are Esc. A line with a value (a volume, the resolution, the field of view, a Yes/No switch)
  goes down on a click on its left half and up on its right half, as Left and Right change it; the highscores'
  arrows change the level. The wheel scrolls the community levels' list (choosing the line under the cursor) and
  goes through the lines elsewhere, through the levels on the highscores. A click on a key's line on the Controls
  page waits for a key as Enter does; mouse buttons cannot be bound. The cursor is shown over the menus and hidden
  in a run, never held in the window; the run takes nothing from the mouse, so a click that closes a menu does
  nothing in the level. The buttons are hit where they are drawn, in any window and either interface.
- Every finished run is still kept as a replay (it costs nothing on screen); the fastest counts as the best time
  shown in the extended mode.

## Ours: the extended mode

Turned on in Options, or for one run with `--debug`. The menus keep the original's look; what changes:

- **Main menu**: Start, Highscore, Options, Credits, Exit, as the original; *Start* first asks which levels.
- **Options**: the *Extended* page has the mode's switch and, under it, one switch for each of its additions
  (all on with the mode; the settings keep those turned off as `extended.<key> off`); the practice tools and endless
  lives are on its *Mods* screen with the mods ([mods.md](mods.md)), the others on the page itself:
  - all levels open, without unlocking;
  - best times beside the levels;
  - levels by kind under Start;
  - Interface: own / original / remaster, where the interface's files come from ([resources.md](resources.md));
  - Textures: the texture pack the levels, their parts and the balls are drawn with — own (the default, and always
    in the classic mode), original, remaster or a player's pack, by its name; the next level loads with the choice
    ([packs.md](packs.md));
  - Models: the model pack the parts and balls are drawn with (their look only, never their physics) — the level's
    (each level with the parts it takes: the default, and always in the classic mode), own, original or a player's
    pack ([packs.md](packs.md));
  - Sounds: the sound theme — system, generated, original (made by the toolbox) or a player's own
    ([resources.md](resources.md)); the generated theme until the player chooses another, and always in the
    classic mode;
  - practice tools: free camera (F2), slower and faster (`[` `]`), save the moment and go back to it (F5 / F9),
    five seconds back (F3, again for further, up to half a minute), the sector and tick on screen; a run helped this
    way is kept but never a best run;
  - endless lives: the last ball is never lost (a run it saves is never a best run). There is no time limit to lift:
    the points only count down and stop at zero, as in the original.
  - Mods: the screen of the practice tools, endless lives and every mod found (factory and the player's), each
    switched with left and right, Enter opening its page with its options ([mods.md](mods.md)); off in the classic
    mode.
- **Graphics** adds *Field of View*: the in-game camera's width across 4:3, 40° to 100° in steps of 2 (`fov 58`; the
  original's is 58°, InGameCam's 1.0123 radians). The classic mode always draws the original's.
- **Controls** adds *Defaults*, back to the original's keys (arrows, Shift, Space) and no inverted turn.
- **Levels by kind**: Start offers *Original* (the classic list of levels), *Remastered*, *Community maps* and
  *Open format*, the folders of `levels/` ([levels.md](levels.md)). In each, a flat list of its levels by title (a level is a
  folder holding one level, or a loose `.nmo`: levels.md, "A level is a folder"; no folders to open, nothing else
  shown), played like an original level. A title is at most 40 characters and shrinks or wraps to stay on its button. A level that cannot be loaded says
  why and is not started; its runs and best times are kept apart from the original levels. Later: title, author and
  difficulty from the community catalogue.
- Texture packs and mods are here: the packs on the Textures and Models lines ([packs.md](packs.md)), the mods on the
  Mods screen ([mods.md](mods.md)), each behind its own switch; third-party content is played at the player's own
  risk.

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
5. Done: Graphics and Controls; gamepads.

`DRAHTSEILAKT_SHOT_MENU=base:keys` with `--screenshot` draws a menu over the frame without a window (base `main`,
`pause`, `dead` or `end`; keys `u d l r` for the arrows, `e` Enter, `b` Esc, `n` the name entry, `w` the key W
pressed while one is awaited), to look at every screen. A menu not over a level stands over the interface's menu
scene, `DRAHTSEILAKT_SHOT_MENU_MS` milliseconds (default 0) into its camera's round.
