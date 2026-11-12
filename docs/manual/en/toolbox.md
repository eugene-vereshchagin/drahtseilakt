# The toolbox

**drahtseilakt-toolbox** is a separate program that prepares levels for the game on your own computer. The game itself
plays what is in its level folders; everything heavy lives in the toolbox. It never goes online and never changes your
copy of Ballance. It runs on Linux, Windows and macOS. Start it without arguments and it opens its window; its texts
are in English, Russian, German and Simplified Chinese.

## What it does

- **Imports your copy of Ballance.** Choose the folder of the installed game, or a CD image (`.cue`, `.bin`, `.iso`),
  with the **Browse** and **Image** buttons beside the path. The original levels then appear in the game under
  *Start, Classic Ballance*. The same step makes the **original** sound theme from your copy's sounds and music, which
  stay on your machine.
- **Remasters to 4K.** The *Make* button remakes the original levels with rounder shapes and textures 4 times larger
  (a scale of your choice), using the Real-ESRGAN upscaler when it is installed next to the toolbox, else a plain
  smooth resize. The result lands in `levels/ballance-remaster/` and is used by the *Remaster* Look
  (see [Looks](looks.md)). Your originals stay untouched.
- **Imports downloaded maps.** Give it a folder with an unpacked collection of community maps, and press *Import*:
  each map gets a folder of its own in `levels/user-levels/` with a `level.txt` and a picture, ready for the game
  (see [Levels and folders](levels.md)). Running it again adds only what is new. Packed archives (`.zip`) are not
  opened: unpack them first, outside `user-levels/`.
- **Checks the maps.** After an import it asks the game to try each map and keeps a short report beside it, which
  you see in the list of your levels: plays fully, plays with differences, may not be passable, or does not load.

## Typical steps

1. Install the toolbox, start it and point it at your Ballance copy. Press *Make* and wait (large textures take a
   while; the progress is shown per level).
2. Start the game: *Classic Ballance* now lists the 13 originals. In *Options, Graphics* choose the Look you like.
3. For downloaded maps: unpack the download, choose its folder in the import field and press *Import*.
