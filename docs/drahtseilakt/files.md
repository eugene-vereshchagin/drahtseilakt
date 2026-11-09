# Where the game keeps its files

The game reads two kinds of files: the **shipped assets** (`assets/`: fonts, own parts, levels, factory mods, sound themes) and the **player's data** (settings, replays, highscores, imported and made levels, the player's packs, mods and sound themes). Both layouts below work with the same program.

**Shipped assets**, first found wins: `$DRAHTSEILAKT_ASSETS`; `assets/` beside the program; `../Resources/assets` (a macOS .app bundle); and only for developers, the source tree the program was built in (`assets/` three folders above `target/release`, or the path it was compiled in). At the start the game prints one line, `paths: assets ... | data ... (portable|per-user)`, and a session log repeats it.

**Portable** (copy the folder and play; nothing outside it is touched): `drahtseilakt(.exe)`, `assets/` and an empty file `portable.txt` side by side. The player's data is then in `data/` beside the program. The rule is just that file: no `portable.txt`, no portable mode.

**Installed** (the program and `assets/` in one folder, e.g. `/opt/drahtseilakt`, `C:\Program Files\Drahtseilakt`, an .app's Resources): the player's data is in the per-user folder, `$XDG_DATA_HOME/drahtseilakt` (default `~/.local/share/drahtseilakt`) on Linux, `%APPDATA%\drahtseilakt` on Windows, `~/Library/Application Support/drahtseilakt` on macOS. The player's packs, mods and themes there are found in addition to the shipped ones.
