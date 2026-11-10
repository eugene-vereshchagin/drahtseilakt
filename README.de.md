# Drahtseilakt

[English](README.md) · [Русский](README.ru.md) · **Deutsch** · [简体中文](README.zh.md)

*Drahtseilakt* — die Nummer des Seiltänzers und jeder Balanceakt, bei dem ein Schritt zur Seite den Absturz
bedeutet. Ballance, 2004 vom kleinen deutschen Studio Cyparade gemacht, ist genau das: eine Kugel auf einer
Schiene über dem Abgrund. Der Name ist unsere Verbeugung vor dem Spiel und den Menschen, die es gemacht haben.

Drahtseilakt ist eine neue Engine für **Ballance** (Cyparade / Atari, 2004), von Grund auf in Rust geschrieben.
Sie spielt die Daten deiner eigenen Kopie des Spiels — die Originallevel genau so, wie sie sind, und
Community-Karten — unter Linux, Windows und macOS, in jeder Auflösung und jedem Seitenverhältnis bis 4K, mit
Level 13 und einem Menü für Community-Karten, später mit einem offenen, dokumentierten Levelformat und einem
Konverter dafür.

Das Projekt begann als Fork von [OpenBallance](https://github.com/emdzej/openballance) von emdzej, das die
originalen Virtools-Skripte auf einer eigenen Laufzeit ausführt; seine Historie und Reverse-Engineering-Notizen
bleiben erhalten, sein Code wird ersetzt (warum, steht in den [Entscheidungen](docs/de/drahtseilakt/decisions.md)).

Stand: in Arbeit — Phase 1 des [Fahrplans](docs/de/drahtseilakt/roadmap.md), der Leser für Virtools-Dateien.
Warum das Spiel das überhaupt brauchte: [Geschichte](docs/de/drahtseilakt/history.md).

## Bauen und spielen

Rust (stable, 1.87 oder neuer) und cargo; unter Linux ein Vulkan-Treiber und die udev-Header für Gamepads
(`libudev-dev` unter Debian/Ubuntu, `systemd-devel` unter Fedora). Du brauchst deine eigene Kopie von
Ballance.

```sh
cargo build --release
target/release/drahtseilakt --data /pfad/zu/Ballance --level 1
```

Unter Windows öffnet der Release-Build kein Konsolenfenster: aus dem Explorer gestartet schreibt das Spiel seine
Meldungen nach `drahtseilakt.log` in seinem Datenordner (`%APPDATA%\drahtseilakt\`; der des vorigen Starts ist `drahtseilakt.old.log`), ebenso aus einer Konsole
ohne Umleitung; mit umgeleiteter Ausgabe
(`2> log.txt`) dorthin. Der Actions-Workflow *Drahtseilakt* (auf Abruf) baut und testet es unter Windows, Linux und
macOS und lädt jeden Build so hoch, wie er installiert aussieht: das Programm neben `assets/`.

`drahtseilakt --import DIR` kopiert das Originalspiel aus Ihrem installierten Ballance in DIR (oder vom Abbild Ihrer
Spiel-CD: `--import Ballance.cue`, `.bin` oder `.iso`; ohne das Spiel gestartet bietet das Fenster dasselbe an:
was es auf dem Rechner findet, einen daraufgezogenen Ordner oder ein Abbild, einen eingetippten Pfad) nach `levels/original/`
des Spiels; alle Level liegen in `levels/` in seinem Datenordner (`~/.local/share/drahtseilakt/` unter Linux),
jeder Ordner mit einem README, was hineingehört ([wo die Level liegen](docs/de/drahtseilakt/levels.md)).
`--data` (oder `OPENBALLANCE_DATA`) spielt stattdessen aus einem anderen Ballance-Ordner. Das eigene
Programm `drahtseilakt-toolbox remaster --scale 8` wandelt die Original-Level nach `levels/remastered/` (offenes
Format) mit runderen Formen und achtmal größeren Texturen um: mit Real-ESRGAN (es ergänzt Details), wenn es in
`realesrgan/` neben der Toolbox liegt (festgelegt in seinem Repository drahtseilakt-toolbox) oder im PATH ist, sonst mit Lanczos; sie laufen unter Start → Remastered. Das Spiel selbst braucht
beides nicht. `--level N` spielt Originallevel N, `--map
FILE.nmo` jede Leveldatei oder Community-Karte, `--open DIR` ein Level im
[offenen Format](docs/de/drahtseilakt/open-format.md); `--export DIR` wandelt das gewählte Level darin um.
Pfeiltasten rollen die Kugel relativ zur Kamera, Shift+Links/Rechts drehen die Kamera, Leertaste hebt sie, F11
ist Vollbild, Esc pausiert; andere Tasten belegt Options → Controls. Ein Gamepad spielt auch: der linke Stick
oder das Steuerkreuz rollt, die Schultertasten drehen die Kamera, die untere Aktionstaste hebt sie, Start
pausiert. Ohne genanntes Level beginnt das Spiel mit dem Hauptmenü. Die Menüs sind die des
Originals ([Menüs](docs/de/drahtseilakt/menu.md)), in den Worten der Kopie: Start (ein Level öffnet sich, sobald
das vorige geschafft ist), Highscore (zehn Plätze pro Level, in unserem Datenordner), Options, Credits, Exit; Esc
im Level öffnet das Pausenmenü, ein verlorener Lauf das Tot-Menü, ein geschaffter die Punkte, die Namenseingabe
für einen neuen Highscore und das Levelende-Menü.

Jeder beendete Lauf wird als Replay gespeichert (unter Linux in
`~/.local/share/drahtseilakt/replays/<Level>/`, der schnellste als `best.replay`); `--replay FILE` spielt eines
ab, `--headless` prüft es ohne Fenster, `--screenshot FILE.png` rendert ein Bild. Um eine Stelle zu testen,
beginnt `--sector N` an einem Checkpoint und `--at X,Y,Z` (oder der Name eines Level-Objekts) an beliebiger
Stelle, mit der Kugel, mit der der Sektor beginnt (`--ball wood|stone|paper|sticky` überschreibt sie); ein solcher Lauf behält seinen Start im Replay und ist nie ein bester Lauf.
Bis der erweiterte Modus eingeschaltet ist (Options, Extended, oder `--debug` für einen Start), ist das Spiel so
genau wie möglich das Original. Der *erweiterte Modus* zeigt Bestzeiten. Start listet die Level in beiden Modi nach Bedeutung (Klassisches Ballance aus Ihrer Kopie, Drahtseilakt,
Ballance-Prüfungen, Community-Karten: die mitgelieferten besten und Ihr `levels/downloaded/`, jedes mit seiner Schwierigkeit; wie man eines
baut und teilt: [Level bauen und teilen](docs/de/drahtseilakt/making-levels.md)) und die Mods
([Mods](docs/de/drahtseilakt/mods.md)). Ein Debug-Lauf (`--debug`) öffnet zudem alle Level und bietet den
Entwicklungs-Mod *Gottmodus*: endlose Leben, F5 speichert den Moment und F9 kehrt dorthin zurück, F3 geht Sekunden
zurück, R startet neu, `[` und `]` verlangsamen den Lauf und beschleunigen ihn wieder, F2 nimmt eine freie Kamera auf,
und Sektor und Takt werden angezeigt; seine Läufe werden als gemoddet behalten, nie als beste. Ein Release liefert ihn
nicht mit. Zum Testen lässt `--autopilot` einen Testspieler das Level ohne
Fenster versuchen (mit `--record FILE` bleibt sein Lauf erhalten); `--follow FILE` lässt ihn dem Weg der Kugel im Replay eines
Spielers folgen; wo er steckt, schreibt er einen Bericht, und er folgt Hinweisen ([der Testspieler](docs/de/drahtseilakt/autopilot.md)). `--locate DIR --record TRACK` findet den Weg der Kugel in den Bildern eines Videos des Originals (DIR/s0001.jpg usw., vier pro Sekunde, z. B. `ffmpeg -vf fps=4,scale=640:480`): Das Level wird von den möglichen Orten der Kugel aus gezeichnet und mit dem Bild verglichen; einer solchen Spur folgt der Testspieler mit `--follow TRACK`. `--frames DIR --replay FILE` zeichnet ein Replay als solche Bilder, mit den wahren Orten der Kugel, zum Prüfen. `--level-stats DIR` vermisst die Originallevel für den Entwurf eigener (worüber die Kugel rollt, Sektor für Sektor, entlang der fertigen Läufe und Pläne in `hints/`; `scripts/level-stats.py` macht daraus einen Bericht); mit `--open LEVEL --replay RUN` vermisst es eines unserer Level entlang eines Laufs, und `scripts/level-stats.py --own` stellt es seinem Ziel gegenüber (mit `hints/level-stats-manual-own.csv`). `drahtseilakt --help` zeigt alles.

## Wer es gemacht hat

Code, Dokumentation und die erzeugten Klänge und Musik schreibt **Claude** (das KI-Modell von Anthropic, in Claude
Code). **Veresk** führt das Projekt: die Idee, die Entscheidungen, der Plan, das Spielen und das Ohr, das sagt, was
stimmt. Die Commits tragen beide Namen.

## Rechtliches

Dateien, die aus OpenBallance übernommen wurden (GPL-3.0, emdzej und Mitwirkende), sind in ihren Kopfzeilen gekennzeichnet; das Crate `ivp` ist der Physik-Port.

Drahtseilakt steht unter der GPL-3.0, wie OpenBallance, mit dem es begann. Es braucht deine eigene Kopie des
Spiels und liefert nie Daten von Ballance aus. (Solange dieses Repository privat ist, hält `levels/` eine
Sicherung der Originallevel für die Entwicklung; das öffentliche Repository enthält sie nicht.) Ballance ist
© 2004 Cyparade / Atari. Dieses Projekt steht in keiner Verbindung zu ihnen.

Wo das Spiel seine Dateien ablegt (portabler Ordner mit `portable.txt` oder Installation mit Daten im Benutzerordner): [files.md](docs/de/drahtseilakt/files.md).
