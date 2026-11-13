# Playing

## Controls

| Key | What it does |
|---|---|
| Arrow keys | roll the ball (Up rolls it away from the camera) |
| Shift + Left / Right | turn the camera around the ball |
| Space | lift the camera for an overview of the level |
| Esc | pause menu; in other menus, go back |
| F11 | full screen on and off |
| F1 | open this manual, anywhere in the menus |

A gamepad works too: the left stick or the cross rolls the ball, the shoulder buttons turn the camera, the lower face
button gives the overview, Start pauses. The keys can be changed in **Options, Controls**: choose a line, press
Enter, then press the new key. *Defaults* brings the original keys back, and *Invert Rotation?* turns the camera the
other way. In the menus the mouse works as well.

## The ball and the transformers

You start with the ball a level gives you. There are three kinds, and a *transformer* (a ring on the track) changes
your ball into another kind:

- **Wood** is the all-rounder, and the ball most levels start with.
- **Stone** is heavy: slow to get going and hard to stop, but steady on the track.
- **Paper** is very light: it speeds up fast and is easily thrown around by the level's fans and winds.

Some levels use a fourth ball of our own, the **sticky** ball, which holds on to what it touches.
Each kind rolls differently, so look at the track before you choose.

## Checkpoints, lives and points

- A level is divided into sectors. When you reach the checkpoint at the end of a sector, it is saved.
- If you fall or lose your ball, you come back at the last checkpoint with a new ball. You start a level with
  **3 lives**; with none left you can restart the level or go back to the menu.
- You start with **1000 points** and they count down while you play. Pick up **extra points** and **extra lives** on
  the way.
- At the finish you get your points, a level bonus and 200 for each life you have left. Finish a level to open the
  next one, and your best scores go into the **Highscore** list.

## Pause and restart

**Esc** pauses the game. The pause menu offers *Back* (go on), *Restart Level*, *Highscore*, *Options* and *Exit Level*.
Restart and Exit ask once whether you are sure. Replays are off by default; see [Replays](#replays).

## The menus

- **Start** lists the sets of levels: *Featured maps*, *Classic Ballance* (needs your copy of the original),
  *Ballance Challenges* (community remakes of the originals, also needs the copy) and *Community maps* with *Included* and
  *User levels*. Next to each map you see its difficulty from 1 to 10.
- **Options** has *Graphics* (window size, sync, clouds, full screen and the *Look*, see [Looks](looks.md)),
  *Controls*, *Sound* (volumes and the sound theme, see [Sound and music](sound.md)), *Replays*, *Language*, *Extended* and
  *Help* (this manual).
- **Language** changes the language of the menus at once: English, Russian, German, Simplified Chinese, French,
  Spanish and Italian.
- **Extended** switches on the extended mode: best times beside the levels, more choices for looks and sounds and the
  [mods](mods.md). With it off, the game plays as the original did.

Every line of the menus has a short explanation in the panel on the screen.

## Replays

A replay is a recording of your key presses; it plays your run back exactly. They are off by default, as they take
space. In **Options > Replays** set *Save replays*:

- **Off**: nothing is saved.
- **Ask**: when a run ends (the level is finished, the game is over, or you restart or leave it) and when a ball is
  lost, a small question "Save the replay?" appears at the top. Press **Y** or **N**, or click Yes or No. The run goes
  on meanwhile; the question fades away after a few seconds when you do not answer. Runs shorter than five seconds are
  not offered unless finished.
- **Always**: every run is saved.

Replays are kept in the `user-replays` folder of your data folder, one file each, named by date and level:
`2026-11-12-21-40-05-Level_03.replay`. The same screen lists them, newest first, with the date, level, length and how
the run ended. **Enter** plays one in the window, **Esc** stops it and takes you back to the list. A replay plays only
on the very level file it was recorded on: if you do not have that level, or the replay comes from another version of
the game, the list says so. *Open folder...* opens a dialog to choose another folder (for example one with replays
somebody sent you); it is remembered and its replays are listed too. If a game has a bug, a replay of the run that
shows it is the best report: send the file.
