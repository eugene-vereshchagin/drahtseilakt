# Fonts shipped with the game

Substitutes for the Windows system fonts (Arial and the like) that Virtools Building Blocks ask for by name.
They are kept here so that neither the build nor the game ever downloads a font (see "The game never goes
online" in `docs/drahtseilakt/decisions.md`). `SHA256SUMS` lists the expected checksums; check with
`sha256sum -c SHA256SUMS`.

| File | Font | Version | Licence |
|---|---|---|---|
| `NotoSans-Regular.ttf` | Noto Sans Regular, hinted | 2.008 | `OFL-NotoSans.txt` |
| `NotoSans-Bold.ttf` | Noto Sans Bold, hinted | 2.008 | `OFL-NotoSans.txt` |
| `NotoSansSC-Regular.otf` | Noto Sans SC Regular (Simplified Chinese subset, for community maps) | 2.004 | `OFL.txt` |

All are under the SIL Open Font License 1.1, which allows bundling and redistribution with the licence text.
File names follow Ballanced's manifest, so this folder works as its `VXMATH_FONT_DIR`. They are the same files, byte for byte, that Ballanced's font manifest (`Source/VxMath/fonts/sources.json`)
pins by sha256, taken on 2026-10-30 from:

- `https://raw.githubusercontent.com/notofonts/noto-fonts/main/hinted/ttf/NotoSans/` (Regular, Bold)
- `https://raw.githubusercontent.com/notofonts/noto-fonts/main/LICENSE` (`OFL-NotoSans.txt`)
- `https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf`
- `https://raw.githubusercontent.com/google/fonts/main/ofl/notosanssc/OFL.txt` (`OFL.txt`)

Updating a font is a deliberate change: replace the file, update `SHA256SUMS`, the version above and the date.
