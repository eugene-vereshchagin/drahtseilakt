# Wo die Level liegen

Wie das Spiel seine Level aufbewahrt, mit dem Eigentümer am 2026-11-02 abgestimmt (decisions.md, „Level liegen in
einem Ordner, vier Arten“). Die Menüs, die sie zeigen, stehen in [menu.md](menu.md).

## Die Ordner

Alle Level liegen in einem Ordner, `levels/`, im Datenordner des Spiels (`~/.local/share/drahtseilakt/` unter
Linux, `%APPDATA%\drahtseilakt\` unter Windows, `~/Library/Application Support/drahtseilakt/` unter macOS),
geteilt nach Herkunft und Format:

| Ordner | Was | Woher | Modus |
|---|---|---|---|
| `levels/original/` | das Originalspiel: `base.cmo`, `3D Entities/`, `Textures/`, `Sounds/` und der Rest, mit dem darin installierten Add-on für Level 13 | vom Spiel aus dem installierten Spiel des Spielers oder von der CD kopiert | klassisch und erweitert |
| `levels/community/` | Community-Karten, `.nmo` | der Spieler lädt sie selbst; das README des Ordners sagt, wo | erweitert |
| `levels/remastered/` | die Original-Level im offenen Format, mit auf 4K–8K vergrößerten Texturen und was ein modernes Aussehen sonst braucht | auf dem Rechner des Spielers von unseren Skripten aus `original/` erzeugt; nie weitergegeben | erweitert |
| `levels/open/` | Level im offenen Format: darin gebaut oder umgewandelte Community-Karten | von Spielern gemacht; einige kommen mit dem Spiel | erweitert |

Der klassische Modus sieht nur `original/`, der erweiterte alle vier (menu.md).

## Das Originalspiel: standardmäßig kopiert

Das Spiel kopiert einmal, was es braucht, aus der eigenen Kopie des Spielers nach `levels/original/`: aus einem
installierten Spiel (`drahtseilakt --import <Ballance-Ordner>`) oder direkt von der CD des Spiels oder ihrem
Abbild (`drahtseilakt --import Ballance.cue`, auch `.bin`, `.iso`: das InstallShield-6-Archiv in `SETUP/`, gelesen
von `crates/cdimage`). Ein Start ohne das Spiel öffnet ein Fenster, das dasselbe tut: es zeigt, was es auf dem
Rechner findet (installierte Spiele in den üblichen Ordnern von Windows, Wine, GOG, Heroic und Lutris; Discs in
ihren Laufwerken; CD-Abbilder im Home-, Desktop- und Download-Ordner), nimmt einen daraufgezogenen Ordner, ein
Abbild oder einen eingetippten Pfad und startet das Spiel mit der Kopie. Die Kopie sind die eigenen Daten des Spielers auf seinem Rechner; nichts wird
heruntergeladen, nichts kommt ins Repository. `--data DIR` spielt weiterhin aus jedem Ordner ohne Kopie.

Das README des Ordners nennt, was von Hand zu kopieren ist: `base.cmo` und die Ordner `3D Entities`, `Textures`
und `Sounds` eines installierten Ballance (mehr liest das Spiel nicht) und für Level 13 das SuDu-Add-on, in diese Kopie installiert, wie
sein Readme es sagt.

## Jeder Ordner lässt sich verlegen

Jeder der vier kann in den Einstellungen woandershin zeigen (`levels.original`, `levels.community`,
`levels.remastered`, `levels.open`), für eine große Platte oder einen geteilten Ordner. Der Standardordner behält
dann nur ein README, das sagt, wo die Level sind, damit ein Spieler, der dort sucht, sie findet.

## Was mit dem Spiel kommt

Das Spiel wird mit einigen Leveln in `levels/open/` installiert (unseren eigenen) und mit umgewandelten
Community-Karten, deren Autoren sie mit dem Spiel ausliefern möchten. Jedes solche Level braucht:

- die Erlaubnis seines Autors und eine freie Lizenz, daneben festgehalten (`LICENSE` in seinem Ordner);
- nichts vom Originalspiel darin: keine Texturen, Klänge oder Meshes von Ballance. Eine umgewandelte Karte, die
  Texturen des Originals nutzt, nennt sie beim Namen, und das Spiel nimmt sie aus `levels/original/` (der Kopie
  des Spielers), wie es die Karte im Original tat;
- ein Replay, das es schafft, geprüft wie die Replays der Original-Level.

Das Repository enthält solche Level nur, wenn ihre Lizenz es erlaubt; Dateien des Originalspiels nie.

## Jeder Ordner erklärt sich selbst

Beim ersten Start schreibt das Spiel in jeden Ordner ein README in der Sprache des Spielers: was hineingehört, wo
man es bekommt und für `remastered/`, welches Skript es erzeugt. Die Texte kommen mit dem Spiel (die Doku in
jeder Sprache); die Links im README sind Text, das Spiel öffnet sie nie.

## Läufe und Highscores nach Art

Replays, Bestzeiten und Highscores werden nach Art getrennt, damit eine Community-Karte mit dem Namen eines
Original-Levels nie als dieses zählt: `replays/<Level>/` für die Original-Level, `replays/maps/<Ordner>/<Karte>/` für die
anderen (`community`, `open`, `remastered`).

## Sprachen

Die größte Community ist in China (Baidu Tieba, das Chinese Ballance Forum, Bilibili; community-maps.md), daher
gibt es die Doku und die READMEs der Ordner auch in vereinfachtem Chinesisch, neben Englisch, Russisch und
Deutsch.

## Reihenfolge der Arbeit

1. Erledigt (2026-11-02): diese Seite und die Entscheidung; die chinesische Doku.
2. Erledigt: die Ordner `levels/` mit ihren READMEs; die Einstellungen, die sie verlegen, mit dem zurückgelassenen
   README.
3. Erledigt: `--import` aus einem installierten Spiel oder vom CD-Abbild (das Spiel findet die importierte
   Kopie auch ohne `--data`). Die 318 Spieldateien der europäischen CD gleichen einer installierten Kopie aufs Byte.
4. Das Menü der eigenen Level liest `community/` und `open/`; `remastered/` macht `--remaster N`: der Umwandler
   (Phase 6) und N-mal größere Texturen (Real-ESRGAN, wenn installiert, sonst Lanczos; Phase 7). Ein Originallevel,
   das der Kopie fehlt (eine Kopie ohne Level 13), wird im Levelmenü aus seinem Export in `remastered/` gespielt.
5. Level, die mit dem Spiel kommen, sobald Autoren sie anbieten.
