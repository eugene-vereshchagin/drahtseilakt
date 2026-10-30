# Original levels (private backup)

The working set for the engine and the converter: if the original levels play, the rest follows. Kept here
only while the repository is private, to move them between the owner's machines; they are Ballance's
copyrighted data and never go into the public repository (`docs/drahtseilakt/decisions.md`, "The private
repository is a backup"). Code and tests still read the game from `OPENBALLANCE_DATA` and never depend on
this folder.

- `original/` — `Level_01.NMO` … `Level_12.NMO` from `3D Entities/Level/` of the owner's GOG copy.
- `level13/` — the level 13 add-on ("Speed", Cyparade 2004) in the layout of `3D Entities/`: besides
  `Level/Level_13.NMO` it replaces `Gameplay.nmo`, `Levelinit.nmo`, `Menu.nmo`, `MenuLevel.nmo` and
  `Sound.nmo` and adds `PH/PE_Balloon.nmo`. Level 13 needs all of them.

`SHA256SUMS` lists every file; check with `sha256sum -c SHA256SUMS` from this folder.
