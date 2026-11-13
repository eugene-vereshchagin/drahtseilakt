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
| Main without a copy (ours) | Start, Highscore, Options, Credits, Exit | Start lists "Our levels" (the shipped ones and `levels/open/`); Options shows a note that the original levels are imported with `drahtseilakt-toolbox` (a separate program; the game has no import window and never runs the toolbox) (decisions.md, "The game plays out of the box") |
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
- **The look choices** (owner's decision 2026-11-09), in the classic mode too, with the same settings as the
  extended mode's lines: *Textures* and *Models* under the Graphics page's lines, *Sound theme* under the Sound
  page's two volumes (the original's lines first, in their order; ours below them, like *Fullscreen?*). The arrows
  go round every pack found: the built-in `own`, `original` (only when the player's copy is present), `remastered`,
  and any pack in the data folder, so resources a player converts himself just show up ([packs.md](packs.md),
  [sound-themes.md](sound-themes.md)). The defaults are unchanged (own textures, the level's parts, the default
  theme). Only the look changes: the rules and the gameplay stay the original's, and replays do not depend on the
  look. The rendering fixes (any resolution, widescreen, filtering, smoothing) are always on and not switchable.
- **Controls**: Enter on a key's line waits for a key ("Press a key"); the key pressed is bound, Esc gives up.
  A key another action has is swapped onto it, so no key does two things. Esc and F11 are the game's and cannot be
  bound; a bound key comes before the mods' keys. Invert Rotation? turns the camera the other way.
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

Turned on in Options, or for one run with `--debug` (a debug run, which also offers *All levels open* and the
development mod god-mode, [mods.md](mods.md), "God mode"). The menus keep the original's look; what changes:

- **Main menu**: Start, Highscore, Options, Credits, Exit, as the original; *Start* first asks which levels.
- **Options**: the *Extended* page has the mode's switch and, under it, one switch for each of its additions
  (all on with the mode; the settings keep those turned off as `extended.<key> off`):
  - all levels open, without unlocking: only in a debug run (`--debug`); a player's game opens the levels by playing
    them;
  - best times beside the levels;
  - (Start is the same in both modes since 2026-11-09; the switch no longer exists);
  - Interface: Drahtseilakt / Ballance / Remaster, where the interface's files come from ([resources.md](resources.md));
  - Look: one choice, the Graphics page's *Look* line (Original / Remaster / Drahtseilakt / Player's pack; see "The Look" below), in place of the texture pack line; the next level loads with it ([packs.md](packs.md));
  - Models: the model pack the parts and balls are drawn with (their look only, never their physics) — the level's
    (each level with the parts it takes: the default), Drahtseilakt, Ballance or a player's
    pack ([packs.md](packs.md));
  - Sounds: the sound theme — system, generated, Ballance (made by the toolbox) or a player's own
    ([resources.md](resources.md)); the generated theme until the player chooses another;
  - Mods: the screen of every mod found (factory and the player's; in a debug run god-mode too, which holds what were
    the practice tools and endless lives), each switched with left and right, Enter opening its page with its options
    and keys ([mods.md](mods.md)); off in the classic mode. There is no time limit to lift: the points only count down
    and stop at zero, as in the original.
- **Graphics** adds *Field of View*: the in-game camera's width across 4:3, 40° to 100° in steps of 2 (`fov 58`; the
  original's is 58°, InGameCam's 1.0123 radians). The classic mode always draws the original's.
- **Controls** adds *Defaults*, back to the original's keys (arrows, Shift, Space) and no inverted turn.
- **Lists of levels** (a list opens from Start, below): In each, a flat list of its levels (a level is a
  folder holding one level, or a loose `.nmo`: levels.md, "A level is a folder"; no folders to open, nothing else
  shown), played like an original level. Each row shows the level's **difficulty**, a star and 1 to 10 (level.txt's
  `difficulty`; "★?" when unknown), and its **title** in English (level.txt's `title`), or in the menus' language when
  level.txt gives one (`title.ru`, ...). The list is sorted by difficulty, then title; levels of unknown difficulty
  after those of a known one, and a level that names itself only by its folder (no `title`) last. A title is at most 40
  characters and is drawn smaller, or shortened in its middle, to stay on its row. A level that cannot be loaded says
  why and is not started; its runs and best times are kept apart from the original levels. How to make and place a
  level: [making-levels.md](making-levels.md).
- Texture packs and mods are here: the packs on the Textures and Models lines ([packs.md](packs.md)), the mods on the
  Mods screen ([mods.md](mods.md)), each behind its own switch; third-party content is played at the player's own
  risk.
- **Long lists** (Start > My maps folder, mods, modified originals): Esc or Backspace (the pad's back button) leaves from any row; Back stays pinned under the list, with an "Esc: back" hint beside it. PageUp / PageDown move a page, Home / End jump to the first / last row, a held Up / Down repeats (after 0.35 s, 12 rows a second, 30 after a second more), the wheel scrolls, and a typed letter jumps to the next title starting with it. *Community picks* are the 13 maps shipped with the game (credited); *My maps folder* lists the maps you put in `levels/community/` of the game's data folder (its path is shown under the list).


## Start, by meaning (owner's decision 2026-11-09)

Start is the same in the classic and the extended mode; an item that is empty or unavailable is left out. Each item
and each submenu item says on the notes panel what it is and where it comes from.

| Item | Opens | Needs |
|---|---|---|
| *Featured maps* (ru «Избранные карты») | the owner's 13 picks of players' maps, brought into our open format, credited to their authors | |
| *Classic Ballance* (ru «Классический Ballance») | the 13 original levels of the player's copy, unlocking as in the original | a copy |
| *Ballance Challenges* (ru «Испытания Ballance») | the original levels remade by the community: the camera turned 45 degrees, the way backwards, the road invisible (`levels/modified/`) | a copy and levels in the folder |
| *Community maps* (ru «Карты сообщества») | a submenu | |
| &nbsp;&nbsp;*Best* (ru «Лучшие») | the 13 community maps shipped with the game, with their authors and the thanks notice | |
| &nbsp;&nbsp;*Downloaded* (ru «Скачанные») | the player's folder `levels/user-levels/` (its path is shown): community `.nmo` maps and open-format levels in one list, with those of the earlier folders `levels/community/` and `levels/open/`, which are still read | |

The strings are in all seven languages. Start's order since 2026-11-11: the featured maps first, *Classic Ballance* second.

**The Look** (owner's decision 2026-11-11, decisions.md "One Look, and beauty comes through the toolbox") is one line,
*Look*, on the Graphics page (setting `look original|remaster|drahtseilakt|player`, with `textures` kept as the pack it
draws with), replacing the separate Textures line and the Options line of 2026-11-09. *Original*: the copy's textures on
the 2004 shapes. *Remaster*: the toolbox's enlarged textures on rounded shapes. *Drahtseilakt* (the default): our
textures on rounded shapes. *Player's pack*: a texture pack of the player's on rounded shapes. A look that cannot be shown
here is not offered (no copy: no Original or Remaster; no export: no Remaster; no pack of his: no Player's pack), and an
old settings file keeps its meaning (`look remastered` is Remaster; `textures` decide otherwise). The rounded shapes of an
original level are its export `levels/ballance-remaster/Level_NN` (the rounding cache), used by every look but Original; a level
without one plays as it is. The Look's help and the notes panel of a downloaded map say that the beautiful looks come after
the level is imported with `drahtseilakt-toolbox` (the game never goes online and does not run it itself).

## Words that fit, lists that say they scroll

*2026-11-03, after the owner's notes ("the text does not fit everywhere", "two lines in a low button", "it is unclear
that only half the mods are shown").*

- **A label always stays on its button**, in every language and every window (the menu is laid out in the original's
  640×480 frame and scaled whole, so a size or aspect never changes what fits). A label too wide is drawn smaller,
  down to three quarters of its size; then, only where the button is tall enough for two lines at that size or more
  (the big buttons, the fields), on two lines broken at the space nearest the middle; else smaller still, down to 60 %,
  and what still does not fit loses its middle to "..". A low button never holds two lines.
- **Our own screens use a size smaller** than the original's small font: the Extended page, the Mods screen, a mod's
  page, the custom levels and the Controls page's rows, so the longest German, Russian and French labels fit and more
  rows show (13 levels, 10 mods). The help under the Mods screen and a mod's page is a paragraph: it wraps and, if it
  must, gets smaller to fit its box. The rows of these screens are drawn without the key bar's middle mark (the "≡"
  that parts a key's name from the key on the Controls page; under a label it read as stripes).
- **Every list that scrolls says so**: a slim bar beside its rows, its thumb as long as the part shown and where it
  is, and "▲ 3 more" above the first row and "▼ 12 more" under the last. The wheel scrolls the list (the custom
  levels, the mods, a mod's page), a click on the bar scrolls there.
- The test `every_label_fits_its_button_in_every_language_and_window` (menu.rs) draws every screen (each mod's page,
  the Mods screen with each mod chosen) in all seven languages at 640×480, 1280×720, 1920×1080, 3440×1440 and
  1080×1920, with the own interface's face, and fails when a label leaves its frame or its button, is drawn below 60 %,
  or stands on two lines below three quarters of its size.

## Profiles

The original has none (above). For us, a profile would bundle a name, settings, unlocked levels, highscores
and replays, so that several people can share one computer. It is not needed for the classic mode, which
keeps the original's single name in the highscore entry. Open question for the owner: whether profiles come
later as an extended-mode feature.

## Help on every line

Every selectable line on every screen says on the notes panel what it is, how it works and what it gives: the main menu, Start and its submenus, Options and all its pages (Graphics, Controls, Sound, Language, Extended), the pause, dead and end menus and the Yes/No questions. A line with a value (a switch, a choice: *Look*, *Textures*, *Models*, *Sound theme*, *Interface*, *Fullscreen?*, *Clouds?*, *Field of View*, *Extended mode?*, *All levels open?*, *Best times?*...) also explains the value chosen now, and the text changes as Left and Right change it. A player's own texture pack, model pack or sound theme shows the `description` (`description.<lang>`, else `description.en`) of its `pack.txt` or `theme.txt` when it has one; a mod shows its own description. The words are in `assets/own/Texts/menu.txt` (`help <line>`, `help <line> <value>`), in all seven languages; a test fails when a line or a value has no text in a language, and the all-screens fit test walks every line and every value. The panel is as tall as the longest help of its screen, so the rows above it do not move with the cursor; to make room, the pages with a panel (Graphics, Controls, Sound, Extended) show their lines as rows, and the main menu, Options and Start draw their buttons closer together.

## The manual

Options has a *Help* line, and F1 anywhere in the menu does the same: it opens the player's manual, local HTML pages in `assets/manual/<lang>/index.html` (English, Russian, German, Chinese; French, Spanish and Italian open English), with the system's opener (`xdg-open`, `open`, `cmd /c start`), detached. On the player's levels list F1 opens the levels page. Only that local file is opened, only when the player asks, never during a run, and never over the network; if the file is missing the help panel says where it looked. `tools/make_manual.py` builds the pages from the player-facing docs (`python3 -I tools/make_manual.py`, needs the `markdown` module); `assets/manual/` is generated and not in git, so a build that ships `assets/` runs the generator first.

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

**Names of the look choices** (owner's decision 2026-11-09): wherever a choice picks between the original game's things, ours and the remaster, the values are named **Ballance** (the player's copy), **Drahtseilakt** (ours) and **Remaster** (the toolbox's remaster), in every language (proper names, not translated). Only the shown names changed: the settings keys and the folder names stay `own`, `original`, `remastered`, so saved settings keep working. A player's pack or theme keeps its own name, shown in at most 16 letters ([packs.md](packs.md)).

**Start without a level** (2026-11-12): started with no `--level`, `--map` or `--open`, the game opens the main menu at once, over the menu scene only; no level is read or uploaded, and the window's title is `Drahtseilakt`. The level is loaded (behind the loading screen) when the player picks one, and the title then shows it. Internally the game holds a placeholder with no level (`Game::placeholder`), so the mods, the settings preview and the rest of the menu code work unchanged. Returning to the main menu from a level keeps that level loaded.

**Stall log**: in the window, every frame whose own work (the run's ticks, building and drawing the picture, presenting; not the wait that holds the frame rate) takes over 100 ms is logged on stderr and, with a level loaded, in the session log (`# stall: ...`), with the stages (ticks with the slowest tick, acquire, instances, draw with the material sync, overlay, present) and what happened in it: a level load, a death or respawn, a sector change, pipelines created, music and sound pools started, the swapchain set up again. Two start-up lines (`startup: event loop begins`, `startup: first frame drawn`) give the milliseconds since launch.

**Loading bar and splash** (2026-11-13): a level's reading and building (files, parsing, the simulation, the rounding when present) and its textures' decoding and upload to the card run on a loading thread while the window goes on drawing; the main thread only builds the models (materials, buffers, pipelines) in slices of about 26 ms a frame. A load that takes over half a second shows the loading screen: the splash's dusk, "Loading {name}", a bar with a gold ball rolling at its head, and the stage under it ("Reading the level...", "Building the world...", "Smoothing the shapes...", "Textures 57/174...", "Preparing the scene..."; texts `loading reading`, `loading building`, `loading rounding`, `loading textures`, `loading finishing`, all seven languages). Quicker loads stay as they were. Esc during a load drops the thread's result and returns to the main menu; an error is shown as before. Code that takes time while a level loads reports its stage with `loading::stage(...)`. At the start of the game a splash (the name, a rope with a ball walking it, the dusk of the menu scene's colours, our own art) stands for at least 1.4 s, or as long as the menu scene and the first pipelines take to get ready, and gives way to the menu in a 0.45 s fade; any key or click asks for the menu at once. The loading log line (`loading: ... s in all, N frames drawn meanwhile (M over 50 ms), the longest ... ms`) gives the numbers; `DRAHTSEILAKT_AUTOLOAD=<level folder>` loads a level once the menu is up (for screenshots and timing) and `DRAHTSEILAKT_NO_SPLASH=1` skips the splash. Headless runs, replays and videos load synchronously as before.

**The look of the title and loading screens** (2026-11-13, replaces what the paragraph above says is seen; the loading machinery is unchanged): both screens are one picture traced per pixel by a shader of ours (`crates/game/src/titlescene.rs`, `titlescene.wgsl`): a varnished wooden ball on a rope over a sea of clouds under an evening sky, all procedural, no model or texture loaded, so it is there on the first frame and costs the load nothing; edges are covered analytically (smooth at any size without multisampling), the rope blurs away from the focus, the result is dithered. The title screen is a close shot with a slowly drifting camera and the name "Drahtseilakt" large over it; it fades into the menu, which is drawn under it. The loading screen shows the level's name large, its author under it (level.txt's `author`), a thin wire with the ball rolling along it as the load goes on (the done part glows, a glint runs along it) and the stage in small letters under the wire; the sky takes the mood of the level's own sky (level.txt's `sky`: Sunset, Night, Storm, Desert, Mist, the daylight ones; an original level goes by its number), not that sky's pictures, which are not loaded yet. The shown progress follows the real one by the clock, so the ball rolls evenly however the frames fall. The words are not scaled from the interface's glyph atlas: each line is rasterised at the very pixel size it has in the window (`fallback::line`, Tektur with Noto for what it lacks) into a texture of its own, with a blurred copy for its glow and shadow, and made again only when the text or the window's size changes: sharp at 720p and at 4K alike. The stage line is set from one texture holding its characters and the digits, so its running count makes no texture; a screen coming up draws one new line a frame, the picture's pipeline is compiled on a thread at the start, and the interface, the picture and the words go to the card as one buffer and one batch: every object made during a load waits behind the loading thread's uploads, and the screens must not cost the start or the load time (measured: the same as before within the noise). The text `loading` ("Loading {name}") is no longer shown. `DRAHTSEILAKT_SPLASH_HOLD=<seconds>` keeps the title screen up that long (for pictures).

**Rail and still picture** (2026-11-13, changes the two paragraphs above): the ball now rolls on a rail, not a rope: one polished steel tube (as on the single-rail sections of the later levels; rounded, mirroring the sky and the clouds, with the sun's glint) with the ball resting on its top, held by small posts with a cradle under the tube at intervals; on the loading screen the tube is the progress bar: the done part glows gold and its posts light up. The ball is drawn from our own texture (`own/3D Entities/Balls/textures/Ball_Wood.png`, read and cut to mips when the picture's pipeline is made on its thread; the shader's own grain stands in if the file is missing). The first frames show a still instead of waiting for the shaders: `own/Splash/splash.jpg`, a frame of the title scene taken with the game itself (`DRAHTSEILAKT_SPLASH_STILL=1 DRAHTSEILAKT_SPLASH_HOLD=30`, then a window screenshot), is decoded and drawn as one textured triangle (`splashpic.rs`) before the renderer, the fonts and the menu are made; the live scene, starting at its time zero, fades in over it in half a second, then the menu as before. The `startup:` lines give `first pixel (the still picture) N ms after launch` (about 270 ms on the test machine, nearly all of it the window and the card; before, the first pixel came with the first frame, about 890 ms). With no `splash.jpg` the game starts as before.
