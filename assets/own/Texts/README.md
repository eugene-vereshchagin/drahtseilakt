# Interface texts (for translators)

Every menu, hint and map-check text of Drahtseilakt lives in one file per language here: `en.txt`, `ru.txt`,
`de.txt`, `zh.txt`, `fr.txt`, `es.txt`, `it.txt`. They are built into the game, and files found at start override them.

## Format

UTF-8, one text a line, `key = text` (the first `=` splits; the text may contain more). Blank lines and lines starting
with `#` are ignored. Keys are the same in every file; never translate a key, only the text after `=`.

    language name = Deutsch
    Menu_Main Button Options = Optionen
    more = {n} weitere

- `language name` is how the language is listed on the Language page.
- `{n}` is a number and `{name}` a level's name; keep them in the text, anywhere.
- Plural forms are separated by `||`, in the order of your language: English, German, French, Spanish, Italian have
  two (one, other), Russian three (1, 2-4, 5 and more), Chinese one. Example (ru):
  `report plain = {n} поверхность||{n} поверхности||{n} поверхностей`.
- A key missing from a file falls back to English, so a partial file works. `cargo test` lists keys a file lacks.

## Fixing a language or adding one

Drop a file named `<code>.txt` (the code is lowercase, e.g. `pt.txt`; the game picks the system language by the first
two letters of the locale) into a `Texts` folder:

- the player's data folder: `~/.local/share/drahtseilakt/Texts/` on Linux, `%APPDATA%\drahtseilakt\Texts\` on Windows,
  `~/Library/Application Support/drahtseilakt/Texts/` on macOS, or `data/Texts/` of a portable copy (the folder
  with `portable.txt`);
- or `assets/own/Texts/` of the game.

A file of a built-in language replaces the texts it has (key by key); a file of a new code adds a language to the
Language page. The profile's folder wins over the game's. Restart the game to see changes. Copy `en.txt` as a start.
