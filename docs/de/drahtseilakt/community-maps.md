# Community-Karten: wo sie liegen und wie man den Katalog nutzt

Nachschlagewerk für Phase 4 des [Fahrplans](roadmap.md) und für alle (Menschen und Agenten), die
Community-Karten oder Daten darüber brauchen. Stand des Katalogs: 25.09.2026.

## Quellen

| Was | Wo | Hinweise |
|---|---|---|
| Katalog (die Liste) | [pseudoacetyl/BallanceCustomMaps — `CustomMapList.md`](https://github.com/pseudoacetyl/BallanceCustomMaps/blob/main/CustomMapList.md) | Repository unter MIT; maschineller Zugriff unten |
| Alle Karten (Dateien) | [Mega: Custom Maps](https://mega.nz/folder/ARR3RRBL#kvoVNZVOxbUmpDBUqLUccg) | vom Autor des Katalogs |
| Ausgewähltes Paket | [MediaFire: Ballance Map Packs](http://www.mediafire.com/folder/vyzo7893i0oln/Ballance_Map_Packs) | „hochwertige“ Karten, Version 7.6; wird nur alle paar Jahre aktualisiert, neue Karten fehlen |
| Player, Mods | [Mega: Players](https://mega.nz/folder/EIYiyASK#GGanOfhFVcSWjnjxOzJjIQ), [Mega: Mods](https://mega.nz/folder/wEB11B5D#FhU0JpzqomuFCAkMuBePWA) | |
| Neue Karten | internationaler Ballance-Discord, Kanal `#new` | Anleitung: [Discord-FAQ](https://yyc12345.github.io/ballance-discord-rules/en/faq/) |
| Chinesische Community | Baidu Tieba (Ballance-Forum), chinesisches Ballance-Forum; Videos auf Bilibili | das größte Publikum; die IDs im Katalog verweisen dorthin |

**Zugriff für einen Agenten.** Der Katalog ist einfaches Markdown auf GitHub:
`git clone --depth 1 https://github.com/pseudoacetyl/BallanceCustomMaps` oder
`https://raw.githubusercontent.com/pseudoacetyl/BallanceCustomMaps/main/CustomMapList.md`.
Mega und MediaFire sind aus Cloud-Sandboxen meist nicht erreichbar; Kartendateien lädt der Inhaber auf seinem
Rechner herunter.

## Das Format des Katalogs

Eine Markdown-Tabelle, eine Zeile pro Karte, die ältesten zuerst:

| Spalte | Inhalt |
|---|---|
| Posted | Veröffentlichungsdatum `JJJJ-MM-TT`; in Klammern, wenn ungefähr, `(~…)`, wenn geschätzt |
| Baidu Tieba ID & Chinese Ballance Forum ID | Beitrags-IDs; Forum-IDs wie `C126`; kann leer oder `N/A` sein |
| Original Name | meist chinesisch |
| English Name | übersetzt |
| Author | Autor |
| Difficulty | 1–5, manchmal Halbe (`2.5`) oder Spannen (`2.5–3`) |
| Type | Comprehensive, Technical, Logical, Casual, Skillogic, Racing, Puzzle, Landscape |
| Videos | YouTube-Links, mehrere getrennt durch `<br />`; kann leer sein |

Vorsichtig parsen: Einige Zeilen enthalten ein `|` im Namen, wodurch sich die Spalten verschieben (im Stand vom
25.09.2026 steht bei einer Handvoll Zeilen ein Autor oder Name, wo die Schwierigkeit stehen sollte). Jede Zeile
prüfen und fehlerhafte melden, statt zu raten.

## Die Sammlung in Zahlen (25.09.2026)

- Rund 427 Karten, veröffentlicht von 2009 bis 2026; 20–60 neue Karten pro Jahr.
- Schwierigkeit: etwa 75 bei 1, 75 bei 2, 100 bei 3, 100 bei 4, 65 bei 5.
- Typen: etwa die Hälfte Comprehensive; dann Technical (~85), Logical (~55), Casual (~30), Skillogic, Racing,
  Puzzle, Landscape.
- Videos: 366 Karten haben mindestens eines, **99 haben zwei oder mehr**. Karten, die mehrere Leute gespielt und
  veröffentlicht haben, sind ein natürliches „Top 100“ — die erste Auswahl zum Testen.

## Konventionen der Community zum Installieren von Karten

- **Ein Level ersetzen**: die Karte über eine von `3D Entities/Level/Level_01.NMO` … `Level_13.NMO` kopieren.
  Keine Mods nötig.
- **BML**: Karten in `ModLoader/Maps` legen; BallanceModLoader(Plus) fügt ein Kartenmenü hinzu.
- Unsere Engine nutzt keins von beiden: Karten liegen in ihrem eigenen Kartenordner und werden nie in die
  Dateien des Spiels geschrieben (Phase 4).

Wie Karten gebaut werden: Blender 3.6 mit [BallanceBlenderHelper](https://github.com/yyc12345/BallanceBlenderHelper),
exportiert über Virtools 3.5; ältere Karten über 3ds-Max-Plugins.

## Regeln für dieses Projekt

- **Karten sind nicht vertrauenswürdige Eingaben**: Dateien von fremden Laufwerken. Sie gehen nur durch die
  gehärteten, gefuzzten Parser (siehe `CLAUDE.md`).
- **Kartendateien kommen nie ins Repository** — sie sind das Werk ihrer Autoren, und wir dürfen sie nicht unter
  der GPL weiterverbreiten. Eine Auswahl wird nur als Daten geführt: Name, Autor, Datum, Katalogzeile, Link.
- Wie eine Karte aussehen und sich spielen soll, kommt aus den Videos im Katalog, nicht aus dem, was unsere
  Engine gerade zeigt.
