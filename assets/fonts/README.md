# Fonts shipped with the game

The own interface's font (Tektur), and substitutes for the Windows system fonts (Arial and the like) that Virtools
Building Blocks ask for by name. They are kept here so that neither the build nor the game ever downloads a font (see "The game never goes
online" in `docs/drahtseilakt/decisions.md`). `SHA256SUMS` lists the expected checksums; check with
`sha256sum -c SHA256SUMS`.

| File | Font | Version | Licence |
|---|---|---|---|
| `NotoSans-Regular.ttf` | Noto Sans Regular, hinted | 2.008 | `OFL-NotoSans.txt` |
| `NotoSans-Bold.ttf` | Noto Sans Bold, hinted | 2.008 | `OFL-NotoSans.txt` |
| `NotoSansSC-Regular.otf` | Noto Sans SC Regular (Simplified Chinese subset: the Chinese menus, community maps) | 2.004 | `OFL.txt` |
| `Tektur-Variable.ttf` | Tektur, variable (width, weight): the own interface's font, Latin, Cyrillic, Greek | 1.005 | `OFL-Tektur.txt` |

All are under the SIL Open Font License 1.1, which allows bundling and redistribution with the licence text.
The Noto files' names follow Ballanced's manifest, so this folder works as its `VXMATH_FONT_DIR`. They are the same
files, byte for byte, that Ballanced's font manifest (`Source/VxMath/fonts/sources.json`) pins by sha256,
taken on 2026-10-30 from:

- `https://raw.githubusercontent.com/notofonts/noto-fonts/main/hinted/ttf/NotoSans/` (Regular, Bold)
- `https://raw.githubusercontent.com/notofonts/noto-fonts/main/LICENSE` (`OFL-NotoSans.txt`)
- `https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf`
- `https://raw.githubusercontent.com/google/fonts/main/ofl/notosanssc/OFL.txt` (`OFL.txt`)

Tektur (Adam Jagosz, https://github.com/hyvyys/Tektur) was chosen on 2026-11-03 for the own interface: a squared,
technical sans close in feel to the original's wide menu font, with Latin, Cyrillic and Greek in one design. It is
the Google Fonts build, taken on 2026-11-03 from google/fonts at commit `8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5`:

- `https://raw.githubusercontent.com/google/fonts/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/tektur/Tektur%5Bwdth,wght%5D.ttf`
  (`Tektur-Variable.ttf`)
- `https://raw.githubusercontent.com/google/fonts/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/tektur/OFL.txt`
  (`OFL-Tektur.txt`)

The game uses it unmodified (its weight is chosen at run time through the font's variation axes), so the OFL's
Reserved Font Name clause does not come into play.

Updating a font is a deliberate change: replace the file, update `SHA256SUMS`, the version above and the date.
