# The game's own interface files

Our own files for the interface roles the original's copy fills (docs/drahtseilakt/resources.md), laid out as the
original (`3D Entities/...`, `Textures/...`). Nothing here comes from the original. The own interface is the default
(and always the classic mode's); with the extended mode's Interface switch on "original" or "remaster" the game takes
those first and these for what they lack. Sounds and music are not here: they are sound themes, in
`assets/sounds/<theme>/`.

- `Textures/Button01_select.tga`, `Button01_deselect.tga`, `Button01_special.tga`: the buttons, fields, highscore
  places, life ball, points box and end screen's bar, each where the game's code takes it from the texture.
- `3D Entities/MenuLevel/`: the 3D scene behind the menus and the loading screen, in the open level format
  (`level.gltf`, `level.bin`, `textures/`), standing for the original's `MenuLevel.nmo`.
- `Texts/menu.txt`: the interface's words in English, Russian, German, Simplified Chinese, French, Spanish and
  Italian (built into the game).
- `make/make_interface.py`: draws the textures (`python3 -I assets/own/make/make_interface.py`, needs numpy and
  Pillow); `make/make_menu_scene.py` builds the menu scene the same way. Their outputs are committed so the build
  needs no Python. Change a script, run it, commit both. The menu scene's concrete, brick, wood and iron come from
  CC0 photo textures (`make/free_textures.py`: pinned, checksummed; `CREDITS.md`; `SHA256SUMS` for the shipped files).
- `make/metric.py`: scores how close each element and screen comes to the original's (measured from the player's
  copy, never stored); `make/scores.md` is its latest report (how to make the comparison screenshots: its docstring
  and docs/drahtseilakt/menu.md, `DRAHTSEILAKT_SHOT_MENU`).

The interface's font is Tektur, in `assets/fonts/` with the other shipped fonts.
