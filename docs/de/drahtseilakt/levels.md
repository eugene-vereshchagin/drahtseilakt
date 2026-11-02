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
| `levels/community/` | Community-Karten: ein Ordner je Level, darin eine `.nmo` oder ein Level im offenen Format (unten) | der Spieler lädt sie selbst, `drahtseilakt-toolbox import-maps` sortiert einen Download in Ordner; das README des Ordners sagt, wo | erweitert |
| `levels/remastered/` | die Original-Level im offenen Format, mit auf 4K–8K vergrößerten Texturen und was ein modernes Aussehen sonst braucht | auf dem Rechner des Spielers von `drahtseilakt-toolbox remaster` aus `original/` erzeugt; nie weitergegeben | erweitert |
| `levels/open/` | Level im offenen Format: darin gebaut oder umgewandelte Community-Karten | von Spielern gemacht; einige kommen mit dem Spiel | erweitert |

Der klassische Modus sieht nur `original/`, der erweiterte alle vier (menu.md).

## Ein Level ist ein Ordner

*Entscheidung des Eigentümers vom 2026-11-03* (decisions.md, „Ein Level ist ein Ordner“). In `community/` und `open/`
(und `remastered/`) liegen Level, nicht alles, was ein Archiv entpackt hat: das Menü der eigenen Level zeigt eine
flache Liste der Level, nach Titel sortiert, ohne Ordner zum Öffnen.

- **Ein Level ist ein Ordner direkt im Ordner seiner Art** mit genau einem Level: einer `.nmo`-Datei oder einem Level im
  offenen Format (`level.gltf`, `level.bin`, `textures/`; open-format.md). Das offene Format ist das klare, leicht von
  Hand fertigzustellen oder nachzubessern: gewöhnliche glTF- und PNG-Dateien.
- Daneben, alles freiwillig: `level.txt`, einfache Zeilen `Schlüssel = Wert` (`title`, `author`, `version`; `#` beginnt
  einen Kommentar, andere Schlüssel werden übergangen), ein README (`README.txt` oder `README.md`), ein Vorschaubild
  (`preview.png` oder `preview.jpg`) und die Ordner der Ressourcen, die das Level nutzt (`textures/`, `game/`).
- **Der Titel** ist `title` aus level.txt, sonst der Ordnername. Er wird bereinigt (Steuerzeichen,
  Richtungsumschalter und andere unsichtbare Formatzeichen entfernt, Leerzeichen zu einem) und auf **40 Zeichen**
  (Zeichen, nicht Bytes) an einer Wortgrenze gekürzt, mit „…“ am Ende, das die Schrift des Spiels als "..." zeichnet.
  Überall, wo das Spiel den Namen eines Levels zeigt (Menü, Ladebildschirm, Fenstertitel), zeigt es diesen Titel.
  level.txt wird nur gelesen, wenn es eine gewöhnliche Datei von höchstens 4 KiB ist; eine größere oder ein Link
  wird übergangen.
- **Eine einzelne `.nmo` direkt im Ordner der Art ist auch ein Level**, nach ihrer Datei benannt: der einfachste Weg,
  eine Karte auszuprobieren.
- Das Menü übergeht: andere Dateien im Ordner eines Levels, alle Ordner darin (keine Verschachtelung), versteckte
  Namen (mit Punkt vorn), Links (nie verfolgt) und Ordner ohne Level oder mit mehreren (mehrere `.nmo` oder eine `.nmo`
  neben `level.gltf`): das sind keine Level, sie werden auf der Konsole genannt.
- Sammlungen sind kein Begriff: die 13 gedrehten Original-Level sind 13 Level-Ordner („45-Degree Rotated - Level 07“).
- `drahtseilakt-toolbox import-maps <Ordner>` sortiert eine geladene Sammlung so ein: ein Ordner je Karte mit ihrer
  `.nmo`, ihrem Bild als Vorschau und einer level.txt, die sagt, woher sie kommt (und den ganzen Titel, wenn der
  Ordnername Zeichen verlieren musste). Ein nichtssagender Dateiname (`Level_07.NMO`) wird nach seiner Sammlung
  benannt; dieselbe Datei zweimal gefunden ist ein Level; gepackte Archive werden genannt, nicht geöffnet; Namen
  taugen für Linux, Windows und macOS (keine reservierten Namen, keine Punkte oder Leerzeichen am Ende, höchstens 40
  Zeichen, ein zweites „Hello Mars“ mit „(2)“); die Quelle wird nur gelesen. Ein zweiter Lauf fügt nur Neues hinzu.

## Level, die mit dem Spiel kommen

Unsere eigenen Level kommen mit dem Spiel in seinem `assets/levels/`, ein Ordner je Level im offenen Format, nur aus
unseren eigenen Teilen gemacht (assets/own, [resources.md](resources.md)), sie spielen also ganz ohne Kopie des
Originals. Das Menü zeigt sie mit den Leveln im offenen Format aus `levels/open/`: unter Start, wenn es keine Kopie
gibt (die Liste „Our levels“), unter „Open format“ des erweiterten Modus immer. Sie entstehen durch Skripte in
`assets/own/make/` (`levelkit.py` baut ein Level aus Bahnstücken und den Platzhaltern der Teile; `make_test_level.py`
das erste, `Parts_Test`, einige hundert Einheiten Bahn mit jedem Teil darauf oder daneben), deren Ergebnisse im
Repository liegen. Ihre Lizenz ist die des Spiels (GPL-3.0-only).

## Das Originalspiel: standardmäßig kopiert

Das Spiel kopiert einmal, was es braucht, aus der eigenen Kopie des Spielers nach `levels/original/`: aus einem
installierten Spiel (`drahtseilakt --import <Ballance-Ordner>`) oder direkt von der CD des Spiels oder ihrem
Abbild (`drahtseilakt --import Ballance.cue`, auch `.bin`, `.iso`: das InstallShield-6-Archiv in `SETUP/`, gelesen
von `crates/cdimage`). Die Kopie ist freiwillig (decisions.md, „Das Spiel läuft ohne
Weiteres“): ohne sie startet das Spiel mit seinen eigenen Leveln, und **Import** im Hauptmenü öffnet ein Fenster, das
dasselbe tut: es zeigt, was es auf dem Rechner findet (installierte Spiele in den üblichen Ordnern von Windows, Wine, GOG, Heroic und Lutris; Discs in
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
man es bekommt und wie Level abgelegt werden, und für `remastered/`, welches Programm es erzeugt (die Toolbox). Ein
README einer früheren Version wird, unverändert, aktualisiert; eines, das der Spieler geändert hat, bleibt. Die Texte kommen mit dem Spiel (die Doku in
jeder Sprache); die Links im README sind Text, das Spiel öffnet sie nie.

## Läufe und Highscores nach Art

Replays, Bestzeiten und Highscores werden nach Art getrennt, damit eine Community-Karte mit dem Namen eines
Original-Levels nie als dieses zählt: `replays/<Level>/` für die Original-Level, `replays/maps/<Art>/<Karte>/` für die
anderen (`community`, `open`, `remastered`), die eines Level-Ordners unter `replays/levels/<Art>/<Level-Ordner>/<Karte>/`,
damit `Level_07.NMO` aus zwei Ordnern ihre Läufe getrennt halten. Auch eine einzelne `Foo.nmo` und ein Ordner `Foo/`
teilen keine Läufe. Läufe, die eine frühere Version in `replays/maps/<Level-Ordner>/` hielt, werden beim ersten Spielen
des Levels dorthin verschoben (nur durch echte Ordner, nie über schon vorhandene).

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
4. Das Menü der eigenen Level liest `community/` und `open/`; `remastered/` macht `drahtseilakt-toolbox remaster`:
   der Umwandler (Phase 6), rundere Formen und N-mal größere Texturen; die Teile jedes Levels (Kugeln,
   Module, Transformator) werden ebenso in seinen eigenen Ordner `game/` mit Himmel und Klängen umgearbeitet, sodass es
   ohne die Leveldateien der Kopie spielt (open-format.md) (Real-ESRGAN, wenn installiert, sonst Lanczos; Phase 7). Ein Originallevel,
   das der Kopie fehlt (eine Kopie ohne Level 13), wird im Levelmenü aus seinem Export in `remastered/` gespielt.
5. Level, die mit dem Spiel kommen, sobald Autoren sie anbieten.
