# Troubleshooting

**The original levels are not there.** The game needs your own copy of Ballance for *Classic Ballance*. Import it
with the toolbox (see [The toolbox](toolbox.md)), using the folder of the installed game or its CD image.
Check that the toolbox saw the copy: its window shows the result, and the folder `levels/ballance-original/` in
your data folder gets filled.

**My map is not in the list.** A level must lie in its own folder with one `.nmo` (or an open-format level) and a
`level.txt` (see [Levels and folders](levels.md)). A loose file or a folder without `level.txt` is not shown. Put the
map into `levels/user-levels/` in your data folder, and open the list again. Starting the game from a terminal shows
a log that names each entry that was left out.

**Loading is slow.** Large 4K textures take time to load. Levels prepared by the toolbox carry textures in a form the
graphics card reads quickly; if you remastered with an older toolbox, run it again. Maps that the toolbox did not
prepare are rounded while they load, which takes a moment.

**The picture is jerky or tears.** In *Options, Graphics* try a smaller window, switch *Sync to Screen?* on, or choose
the *Drahtseilakt* Look.

**A level does not load or looks wrong.** Some community maps use features the game does not know yet. The toolbox's
report in the level list tells what it found. Such a map may still be playable.

## Where my files are

In your data folder (see [Levels and folders](levels.md)):

- `settings.txt`: your settings.
- `replays/<level>/`: replays of your runs (`.replay`) and the logs of your sessions (`session-....log`).
- `screenshots/`: pictures saved by the Screenshot key mod.

## Reporting a problem

Tell us on the project's page, github.com/eugene-vereshchagin/drahtseilakt (the issues). Say which level, what happened
and what you did, name your system, and attach the newest `session-....log` and the `.replay` of the run from the
`replays/<level>/` folder.
