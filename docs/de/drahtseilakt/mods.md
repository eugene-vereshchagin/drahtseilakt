# Mods

Mods fügen dem Spiel Verhalten und Funktionen hinzu — eine klebende Kugel, einen Timer mit Zwischenzeiten, eine
gedrehte Ansicht —, und jeder Spieler schaltet jeden von ihnen ein oder aus. Sie sind in unserem eigenen Format
geschrieben: **nur Daten**. Ein Mod ist eine kleine Textdatei mit Regeln, gebaut aus einer festen Bibliothek von
Bausteinen der Engine; die Laufzeit des Spiels tut, was die Regeln sagen, so wie die Verhaltensgraphen des Originals
Daten sind, die seine Laufzeit ausführt. Nichts in einem Mod wird je als Code ausgeführt (decisions.md, „Unser eigenes
Mod-Format aus reinen Daten“).

Das Spiel liefert eigene Mods mit, die *Werks-Mods* (`assets/mods/`), geschrieben in genau diesem Format: Sie sind die
Funktionen und die Beispiele, von denen ein Modder ausgeht. Die Mods eines Spielers kommen in `mods/` des Datenordners.
Klangthemen, Textur- und Modellpakete und Level sind keine Mods: Sie haben ihre eigenen Menüs und Ordner
([sound-themes.md](sound-themes.md), [packs.md](packs.md), [levels.md](levels.md)).

## Mods ein- und ausschalten

*Options → Extended → Mods* (nur im erweiterten Modus; der klassische Modus führt keinen Mod aus und spielt wie das
Original). Der Bildschirm listet jeden gefundenen Mod: zuerst die Werks-Mods, in einem Debug-Lauf dann die
Entwicklungs-Mods (god-mode, unten), dann die des Spielers, jeder mit *Yes* oder *No*, oder *refused* für einen Mod,
der sich nicht lesen ließ (die Hilfe unter der Liste sagt, warum). Eine lange Liste scrollt, mit einem Balken daneben
und „▲ / ▼ N weitere“ (menu.md). Links und Rechts schalten den gewählten Mod; Enter öffnet seine Seite:
*Switched on?*, seine Optionen (eine Tastenoption zeigt ihre Taste), seine Beschreibung, Autor, Version und Lizenz und
eine Warnung, wenn er die Regeln ändert. Die Einstellungen halten `mod.<key> on|off` und
`mod.<key>.<option> <value>`. Die Ansichtsseite eines Mods wirkt sofort; einer, der den Lauf ändert, ab dem nächsten
Start des Levels.

## Die Werks-Mods

| Mod | Nach dem BML-Mod | Was er tut | Ändert den Lauf |
|---|---|---|---|
| `sticky-ball` | BallSticky, StickyAccess | was die Kugel berührt, hält sie fest; drückt sie gegen eine Wand, klettert sie hinauf; Optionen: Haftstärke, nur die Holzkugel | ja (gemoddet) |
| `physics-lab` | PhysicsEditor | Schwerkraft als Vektor: nach unten (0–60, im Original 20) und seitlich entlang x und z der Welt (je −20–20, im Original 0) — und der Zeitfaktor der Physik (0,5–4, im Original 2) | ja (gemoddet) |
| `dynamic-fov` | DynamicFov | die Ansicht wird mit der Geschwindigkeit der Kugel breiter; Option: Stärke | nein |
| `free-view` | FreeViewRotation, AdvancedTravelCam | die Maus dreht die Ansicht um die Kugel, eine Taste dreht sie zurück; Abstand der Kamera | nein |
| `segment-timer` | Segment, RealTimeMeter, WatchYourTime | die Zeit des Laufs, die Echtzeit, die Zwischenzeit jedes Sektors gegen den besten Lauf (grün voraus, rot zurück) | nein |
| `info-overlay` | PositionViewer, EnergyItemCounter, SysInfoDisplay | Position, Geschwindigkeit, im Sektor verbleibende Extrapunkte und -leben, Bilder je Sekunde, Uhr (UTC) | nein |
| `quick-restart` | SpeedrunShortcuts | eine Taste (Backspace, Entf, F4 oder Q) startet das Level sofort neu | nein |
| `screenshot` | ScreenCapturer | eine Taste (F12, F10, Druck oder P) speichert das Bild als PNG in `screenshots/` des Datenordners | nein |
| `spirit-trail` | SpiritTrail | der beste Lauf des Levels rollt als durchscheinende Kugel neben diesem; Option: Deckkraft | nein |
| `blackout` | Blackout | das Level wird dunkel, ein schwaches Licht über der Kugel; Optionen: Dunkelheit, Licht | nein |
| `smooth-camera` | AdvancedTravelCam (filmische Glättung) | die Kamera gleitet in Drehungen, Abstand und Sichtfeld, die freie Kamera schwebt; Option: Sanftheit | nein |
| `interference` | Interference | durchscheinende Täuschkugeln kreisen um die Kugel; Option: wie viele | nein |
| `deformed-ball` | DeformedWB | das Aussehen der Holzkugel wird zufällig gestaucht und gedehnt (die Physik behält die runde Kugel); Option: Stärke | nein |
| `back-to-2004` | (ein Scherz; macht unsere Bildverbesserungen rückgängig) | das Spiel wie auf einem PC von 2004: niedrige Auflösung (640x480, 800x600 oder 1024x768) in harten Pixeln in einem 4:3-Bild mit schwarzen Balken, Texturen ohne Mipmaps und Anisotropie (bilinear oder nächster Nachbar: das Flimmern in der Ferne), Bildratenbegrenzung (aus, 60, 30), wahlweise 16-Bit-Farbbänder; Brille für Blutstränen inklusive | nein |
| `play-time` | WatchYourTime | die Spielzeit je Level und gesamt, auf dem Bildschirm und in der `playtime.txt` des Datenordners; Option: anzeigen | nein |
| `sack-supports` | (unserer; im Original hängen die Säcke in der Luft) | die Halter der schwingenden Säcke hängen an etwas: einer Steinsäule, einem gestreiften Mast, einem Holzpfosten oder einem Tor aus zweien, oder einem Pfosten aus dem Schienenrohr der Bahn, der in seinen Balken umbiegt, neben der Bahn; an einem Balken zur nahen Wand oder Säule des Levels; und ein Sack ohne eigenen Platz an einem Querbalken zum Halter des Nachbarn, sodass eine Gruppe an einer oder zwei Stützen hängt. Halterung, Balken und Pfosten sind aus einem Material und mit einer gebauten Verbindung gefügt (Holz: Kopfklotz in Eisenbändern und verschraubter Balkenschuh; Eisenrohr: Flanschnabe und Schelle um Säule oder Mast; Schienenrohr: Klemmen und Muffe), jedes Glied liegt waagrecht oder steht senkrecht. Für jeden Sack beim Laden des Levels gewählt, nur wo die Stütze den Raum des Balls, alle beweglichen Teile, die Kulisse und den Blick der Kamera auf den Ball frei lässt; wo nichts frei ist, hängt der Sack wie eh und je. Nur gezeichnet, mit den eigenen Modellen des Spiels (die des Originals bleiben, wie sie sind). **An, bis man ihn ausschaltet** (er bessert das Aussehen und ändert keine Regel; der klassische Modus führt weiterhin keinen Mod aus). Option: verschiedene Bauarten | nein |
| `sector-jump` | SpeedrunShortcuts (Sektor setzen) | Tasten (F7 / PageUp, F6 / PageDown) starten das Level im nächsten oder vorigen Sektor neu | ja (gemoddet) |
| `spawn-point` | NewSpawn, SpeedrunShortcuts (Startpunkt setzen) | eine Taste (F8, H oder J) setzt einen Startpunkt, wo die Kugel ist, eine andere (G, F4 oder C) bringt die Kugel ruhend dorthin zurück, als die Kugel, die sie war; der Sektor und seine Objekte bleiben, wie sie sind | ja (gemoddet) |
| `free-steering` | FreeViewRotation (auch die Steuerung) | wie `free-view`, aber die Pfeile schieben entlang der gedrehten Sicht; die Drehung der Sicht steht im Replay | ja (gemoddet) |

## Die klebrige Kugelart

*2026-11-10, Entscheidung des Besitzers.* Nach BallSticky eine vierte, eigene Kugel, `sticky`, neben Holz, Stein und
Papier: kein Mod, sondern eine Kugelart, die jedes Level nutzen kann. Ihr Transformer ist `P_Trafo_Sticky` (ein
karminroter Ring; eine Karte, die einen hat, darunter BallStickys eigene Karten, nimmt ihn wie jeden Transformer), ihre
lose Kugel `P_Ball_Sticky`, ihr Name `sticky`, wo immer eine Art genannt wird (`--ball`, `start` eines Replays,
`sector_balls`, `kind` und `become` der Hinweise, `ball`, `trafo` und `loose` der Level-Sprache). Ihr Aussehen ist
unseres (`assets/own/make/make_parts.py`: karminroter Gummi in einer Wabe aus Zellen, vierzehn Gummibrocken für ihr
Zerspringen); mit der Balls.nmo der Kopie, die keine solche Kugel hat, wird sie mit dem Mesh der Holzkugel gezeichnet,
karminrot getönt. Ihre Geräusche sind die der Holzkugel, bis sie eigene hat.

Ihre Werte (gewählt, nicht gemessen: das Original hat keine solche Kugel, und BallStickys stehen in dessen eigenen
Dateien, die nicht gelesen werden): Masse 4 (Holz 1,9, Stein 10), Schub 0,6 (0,15 je Masseneinheit, zwischen 0,09 des
Steins und 0,23 des Holzes), Reibung 1,0 (die höchste der vier), Elastizität 0,05 (die niedrigste), Dämpfung 0,9 (wie
Holz); die lose Kugel ebenso. Sie haftet an dem, was sie berührt, wie `ball.stick 0.9` des Mods `sticky-ball`, ohne Mod:
bei dessen Standard 0,6 klammert sich eine so schwere Kugel nur an eine Wand (Auftrieb 1,09 gegen ihr Gewicht von 1,21),
bei 0,9 klettert sie (1,63). Ist der Mod auch an, zählt das stärkere Haften. An einem Hang, aus dem Stand geschoben,
steigt sie bis 43° und klammert sich steiler fest, ohne zurückzurollen (die Tabelle des Testspielers).

## Gottmodus

*2026-11-03, Entscheidung des Eigentümers* (decisions.md, „Der Gottmodus ist ein Entwicklungs-Mod“). Was die
Übungswerkzeuge und die endlosen Leben des erweiterten Modus waren, ist ein Mod, `god-mode`, geschrieben in genau diesem
Format aus den Bausteinen unten. Er ist ein **Entwicklungs-Mod**: er liegt in `assets/mods-dev/`, den das Spiel nur
liest, wenn es mit `--debug` läuft, und das Paket eines Releases lässt diesen Ordner weg, sodass das Spiel ohne ihn
gespielt wird. Seine Seite (Options → Extended → Mods → God mode) zeigt seine Schalter und Tasten; solange er an ist,
stehen seine Tasten unten links (Option *Keys on screen?*) und Sektor und Takt oben links.

| Option | Vorgabe | Was |
|---|---|---|
| Endless lives? | ja | die letzte Kugel geht nie verloren |
| Seconds back | 5 | wie weit *Back* zurückgeht (1–30) |
| Back | F3 (B, Z) | so viele Sekunden zurück; nochmals gedrückt, weiter (das Spiel behält die letzte halbe Minute, einen Moment je Sekunde) |
| Save / Load | F5 (K) / F9 (L) | den Moment behalten, zu ihm zurück; ein verlorener Lauf wartet auf Load |
| Restart | R (T) | das Level noch einmal |
| Slower / Faster | `[` (`-`) / `]` (`=`) | das Spiel mit halbem Tempo, bis zu einem Achtel, und zurück |
| Free camera | F2 (C) | die Kamera fliegt (Pfeile fliegen und drehen, A/D seitwärts, PageUp/PageDown, Home/End, Shift schneller); der Lauf wartet |
| Keys on screen? | ja | der Hinweis mit den Tasten |

Der Gottmodus ändert den Lauf: seine Läufe sind gemoddet (unten) — behalten, aber sie öffnen kein Level und sind nie
ein bester Lauf oder ein Highscore —, und sein Replay nennt ihn, sodass das Replay nur in einem Debug-Lauf mit
demselben god-mode abspielt. Die alten Einstellungen `extended.tools` und `extended.endless-lives` werden nicht mehr
gelesen.

## Gemoddete Läufe und Replays

Ein Mod, der den Lauf ändert (jede *Laufaktion*, unten), macht den Lauf **gemoddet**: Er wird behalten, ist aber nie
der beste Lauf des Levels, nie ein Highscore, wird nie vom Geist oder von den Zwischenzeiten verglichen und öffnet
nicht das nächste Level (`Level_NN-<ticks>-modded.replay`; „Alle Level offen“ eines Debug-Laufs öffnet alle). Sein Replay
nennt jeden solchen Mod in seinem Kopf —

    mod sticky-ball 9c41f0a2be1377d0 strength=60 wood_only=off

— seinen Schlüssel, den Hash seiner mod.txt und die Werte seiner Optionen, sodass das Replay denselben Lauf abspielt,
und nur mit genau diesem Mod: Ein Replay, dessen Mod fehlt oder sich geändert hat oder dessen Optionen fehlen, unbekannt oder außerhalb des Bereichs sind, wird mit Angabe des Grundes
abgelehnt. Das Replay eines klassischen Laufs hat keine solche Zeile und ist genau wie vorher. Ein Mod, der nur ändert,
was zu sehen ist (die Kamera, die Einblendungen, das Licht), lässt den Lauf klassisch.

## Einen eigenen Mod schreiben

Einen Ordner in `mods/` des Datenordners anlegen (Linux `~/.local/share/drahtseilakt/mods/`, Windows
`%APPDATA%\drahtseilakt\mods\`, macOS `~/Library/Application Support/drahtseilakt/mods/`) mit einem schlichten Namen
(Buchstaben, Ziffern, `-`, `_`) und eine `mod.txt` hineinlegen. Den erweiterten Modus einschalten und den Mod auf dem
Bildschirm Mods finden. Die Werks-Mods in `assets/mods/` sind die Beispiele; einen kopieren und ändern.

Der kleinste Mod:

    name.en Moon gravity
    on start
    do world.gravity 4

### Die Datei

Text in UTF-8, eine Anweisung je Zeile, `#` beginnt eine Kommentarzeile. In dieser Reihenfolge:

- **Kopf** — `name.<lang>` (mindestens einer; Sprachen `en ru de zh fr es it`), `description.<lang>`, `author`,
  `version`, `licence`, `requires <game version>` (z. B. `0.1`; eine neuere als die des Spiels lehnt den Mod ab). Ein Mod, der nur das Aussehen bessert, darf
  `default on` sagen: er ist an, bis der Spieler ihn ausschaltet (der Werks-Mod `sack-supports`).
- **Optionen**, die der Spieler auf der Seite des Mods setzt:
  - `option <name> switch on|off`
  - `option <name> number <min> <max> <step> <default>` (innerhalb von ±1 000 000, ein Schritt von mindestens 0,001, der
    Standardwert auf den Schritten ab `min`)
  - `option <name> key <Key1,Key2,...>` — Tasten mit den Namen von winit (`KeyQ`, `F12`, `Backspace`, `Home`...); die
    erste ist die Vorgabe. Die eigenen Tasten des Spiels werden abgelehnt: Escape, Enter, die Pfeile, Shift, Space,
    Tab, Alt, F11 und Backquote; eine Taste, die der Spieler auf der Seite Controls belegt hat, bleibt beim Spiel.
    (R, F2, F3, F5, F9, `[` `]` gehörten den Übungswerkzeugen; jetzt sind es die Tasten von god-mode und für jeden Mod
    frei.)
  - `option <name> choice <default> <name1> <name2> ...` — eine von 2 bis 12 benannten Auswahlmöglichkeiten (Kleinbuchstaben,
    Ziffern, `_`); in den Regeln ist die Option die Stelle der Auswahl (0 für die erste), also `if opt.quality == 1`;
    Einstellungen und Wiederholungen speichern den Namen (`mod.<mod>.<option> <name>`, `<option>=<name>`). Jede Auswahl
    braucht `label <option>.<name> <lang> <text>` (Englisch Pflicht): die Modseite zeigt diese Beschriftung als Wert;
    beide kurz halten, die Legende gehört in die Beschreibung.
- **Beschriftungen** — `label <name> <lang> <text>`: Die Beschriftung einer Option heißt wie die Option;
  Einblendungszeilen nennen ihre.
- **Variablen** — `var <name> <number>`: Zahlen, die die Regeln behalten, bei jedem Start ab ihrem ersten Wert.
- **Regeln** — `on <event>`, dann beliebige `if <condition>`-Zeilen (alle müssen gelten), dann `do <action>`-Zeilen,
  der Reihe nach ausgeführt.

Namen (Optionen, Beschriftungen, Variablen) bestehen aus Kleinbuchstaben, Ziffern und `_` und beginnen mit einem
Buchstaben.

### Ereignisse

Die Ereignisse des Laufs, in den Ticks des Laufs (deterministisch): `start` (vor dem ersten Tick), `tick` (jeder Tick,
vor der Physik), `sector` (ein Kontrollpunkt erreicht), `lost` (die Kugel verloren), `newball`, `transform`, `extra`
(ein Extrapunkt oder -leben genommen), `finish`.

Die Ereignisse der Ansicht: `frame` (jedes gezeichnete Bild), `key <option>` (die Taste einer der Tastenoptionen des
Mods gedrückt).

### Werte

| Wert | | Wert | |
|---|---|---|---|
| `ball.x` `ball.y` `ball.z` | wo die Kugel ist | `real.time` | echte Sekunden seit Beginn des Laufs * |
| `ball.speed` `ball.vy` | ihre Geschwindigkeit, ihre senkrechte Geschwindigkeit | `frame.dt` `frame.fps` | die Länge des Bildes, Bilder je Sekunde * |
| `ball.kind` | 0 Holz, 1 Stein, 2 Papier, 3 klebrig | `clock.hour` `clock.minute` `clock.second` | die Uhr, UTC * |
| `ball.touching` `ball.wall` | sie berührt etwas; etwas Steiles | `mouse.dx` `mouse.dy` | die Bewegung der Maus in diesem Bild * |
| `run.tick` `run.time` | die Ticks des Laufs, Sekunden | `extras.points` `extras.points_all` | im Sektor verbleibende Extrapunkte, und alle * |
| `practice.speed` | das Tempo des Spiels (1, oder bis 1/8) * | `practice.saved` | ein Moment ist gespeichert (1 oder 0) * |
| `run.sector` `run.lives` `run.points` | Sektor (1 der erste), Leben, Punkte | `extras.lives` `extras.lives_all` | verbleibende Extraleben, und alle * |
| `input.turn` | die von `view.turn` weitergegebene Drehung der Sicht, Grad | `playtime.level` `playtime.all` | Sekunden gespielt in diesem Level, in allen * |
| `input.forward` `input.backward` `input.left` `input.right` `input.rotation` `input.overview` | die gehaltenen Tasten des Spiels (1 oder 0) | `best.time` | die Bestzeit des Levels, Sekunden (-1: keine) * |

`opt.<name>` ist der Wert einer Option; eine Variable wird über ihren Namen gelesen. Mit * markierte Werte gehören der
Ansicht: Eine Regel auf einem Ereignis des Laufs kann sie nicht lesen (der Lauf darf nicht davon abhängen, was zu sehen
ist), ebenso wenig eine Variable, die eine Ansichtsregel setzt.

### Ausdrücke

Zahlen, Werte, `+ - * /`, Vergleiche `< <= > >= == !=`, `and or not`, `true false`, Klammern und die Funktionen
`min(a, b)`, `max(a, b)`, `abs(x)`, `clamp(x, lo, hi)`, `if(c, a, b)`, `floor(x)`, `sqrt(x)`. Eine Bedingung gilt,
wenn sie nicht 0 ist. Jeder Ausdruck ist total: Eine Division durch null ergibt 0, ebenso alles, was keine endliche
Zahl ist; es gibt keine Schleifen; Verschachtelung und Größe sind begrenzt.

### Aktionen

Laufaktionen (nur auf einem Ereignis des Laufs; sie machen den Mod *gemoddet*):

- `do ball.stick <0..1>` — die Kugel haftet mit dieser Stärke an dem, was sie berührt, gegen die Schwerkraft, und ein
  Druck gegen eine Wand lenkt sie an ihr hinauf.
- `do world.gravity <0..60>` — die Zugkraft der Schwerkraft (im Original 20).
- `do world.gravity_xz <x> <z>` — der seitliche Zug der Schwerkraft entlang x und z der Welt, je −60 bis 60 (im Original
  0, 0; zwei Ausdrücke ohne Leerzeichen). Mit `world.gravity` wird die Schwerkraft ein Vektor, wie PhysicsEditor sie setzt.
- `do world.time <0.5..4>` — der Zeitfaktor der Physik (im Original 2).
- `do lives.endless <0|1>` — die letzte Kugel geht nie verloren (god-mode).
- `do steer.yaw <Grad>` — der Schub der Pfeile um die Senkrechte um so viele Grad gedreht (die Kamera dreht sich um
  dieselbe Drehung, die Pfeile schieben also entlang einer gedrehten Sicht). Mit `input.turn` folgt er der Sicht (unten).

Übungshandlungen (nur auf `key`; sie greifen von außen in den Lauf ein, also ist ein Mod mit einer davon ebenfalls
gemoddet, und solange er an ist, behält das Spiel die Momente des Laufs, einen je Sekunde für die letzte halbe Minute):

- `do rewind <1..30>` — so viele Sekunden zurück; `do save`, `do load` — den Moment behalten, zu ihm zurück;
- `do slower`, `do faster` — das Tempo des Spiels halbiert (bis 1/8) oder verdoppelt (bis zum normalen);
- `do freecam` — die freie Kamera aufgenommen oder abgelegt.

Ein Sprung (nur bei `key`; macht den Mod gemoddet, ohne die Momente des Übens):

- `do sector.set <n>` — der Lauf beginnt erneut in Sektor n (1 ist der erste), mit der Kugel dieses Sektors, wie `--sector`;
  das Replay nennt den Start. Der nächste gewöhnliche Neustart ist wieder der Levelstart.

Der Startpunkt (nur auf `key`; der Mod wird gemoddet und gehört zur Seite des Laufs, sein Replay nennt ihn):

- `do spawn.set` — ein Startpunkt, wo die Kugel ist (ihr Ort, ihre Drehung, ihre Art);
- `do spawn.go` — die Kugel ruhend zum Startpunkt zurück, als die Kugel, die sie war; der Sektor und seine Objekte
  bleiben, wie sie sind (der schlichte Teleport von NewSpawn). Beides nicht, während die Kugel verloren, in einem
  Transformator oder im Wechsel ist.

  Beides tut der Lauf selbst: die Taste drückt für einen Tick ein Eingabebit (64 setzen, 128 zurück), das das Replay mit
  den Pfeilen behält, so spielt das Replay gleich ab. Ohne einen solchen Mod beachtet der Lauf die Bits nicht.

Ansichtsaktionen (nur auf `frame` oder `key`):

- `do camera.fov <degrees>` — die Ansicht um so viele Grad verbreitert (-30 bis 60).
- `do camera.distance <0.5..2>` — die Kamera näher oder weiter weg.
- `do camera.yaw <degrees>` — die Ansicht um die Kugel gedreht (die Pfeile schieben weiter entlang der eigenen
  Richtung der Kamera).
- `do render.scale <Breite> <Höhe>` — das Bild in dieser Größe gezeichnet (160x120 bis 4096x4096, nie über dem Fenster) und mit dem nächsten Pixel aufs Fenster gezogen. `do render.aspect <b>:<h>` — das Bild behält diese Form, schwarze Balken, wo das Fenster breiter oder höher ist.
- `do render.filter default|bilinear|nearest` — die Texturfilterung: unsere (Mipmaps, anisotrop), bilinear nur aus der größten Stufe (ohne Mipmaps flimmert die Ferne) oder der nächste Texel.
- `do render.fps_cap <n>` — höchstens n Bilder pro Sekunde (0: keine Grenze; die Takte des Laufs behalten ihre Uhr). `do render.colour_bits 16` — die Farbe auf 5-6-5 Bit ohne Dithering gekappt (jeder andere Wert: volle Farbe).
- `do render.supports <0|1|2>` — die Stützen unter den Haltern der schwingenden Säcke: keine, eine Bauart für alle (die erste freie: Säule, Pfosten, Mast, Schienenrohr, Tor, Wandplatte) oder verschiedene; ein Sack ohne Stütze bekommt einen waagrechten Querbalken zum nächsten Nachbarn mit Stütze (bis 34 entfernt, auf gleicher Höhe), und ein Halter, der schon an der Geometrie des Levels sitzt, bleibt, wie er ist. Die Teile bringt der eigene Sack des Spiels mit (seine Gruppe `Supports_P_Modul_26`); das Spiel setzt sie für jeden Sack dort zusammen, wo nichts davon näher als 4,5 an den Raum des Balls kommt (die Böden und Schienen und 9 darüber), in die Reichweite eines anderen Moduls (der Schwung eines Sacks, der Luftstrom eines Ventilators, 8 um jedes andere Teil), durch die Kulisse oder, bei einer Säule, in den üblichen Blick der Kamera auf den Ball. Eine Wandplatte darf auch an einer Säule sitzen, die zu den Böden gehört: eine steile Fläche, höchstens 7 breit, vom Raum des Balls nur 2,5 beiderseits der Platte ausgenommen. `--supports-check geometry|runs` misst jede gesetzte Stütze der Originallevel und der mitgelieferten Karten noch einmal, auch gegen die aufgezeichneten Läufe.
  Diese Aktionen betreffen nur die Ansicht: der Lauf bleibt klassisch, Wiedergaben laufen gleich. Eine Kantenglättung zum Rückgängigmachen gibt es im Spiel nicht.
- `do light <0..1>` — das Licht des Levels; `do lantern <0..1>` — ein Licht über der Kugel.
- `do show <corner> <label|-> <format> <value>[, <value>[, <value>]]` — eine Einblendungszeile in diesem Bild; Ecken
  `top-left top-right bottom-left bottom-right`; Formate `time` (mm:ss.hh aus Sekunden), `int`, `dec1`, `dec2`,
  `clock` (hh:mm), `ratio` (a/b).
- `do splits <corner>` — die Zwischenzeiten der Sektoren gegen den besten Lauf.
- `do keys <corner>` — die eigenen Tastenoptionen des Mods als Einblendungszeilen, „Beschriftung: Taste“ (der Hinweis
  von god-mode).
- `do ghost <0..1>` — der beste Lauf als durchscheinende Kugel, so deckend.
- `do screenshot`, `do restart` — nur bei `key` (nie in jedem Bild); höchstens ein Screenshot pro Sekunde.

Beide: `do set <variable> = <expression>`. Eine Variable, die eine Ansichtsregel setzt, gehört der Ansicht; die Regeln
des Laufs dürfen sie weder setzen noch lesen.

### Beispiele

Die klebende Kugel (`assets/mods/sticky-ball/mod.txt`, ihre Regel):

    option strength number 10 100 10 60
    option wood_only switch off
    on tick
    do ball.stick if(opt.wood_only and ball.kind != 0, 0, opt.strength / 100)

Die freie Ansicht: eine Ansichtsvariable, die die Maus dreht, und eine Taste, die sie zurückdreht:

    option sensitivity number 1 20 1 4
    option reset key KeyV,KeyB,KeyN,Home
    var yaw 0
    on frame
    do set yaw = yaw + mouse.dx * opt.sensitivity / 20
    do camera.yaw yaw
    on key reset
    do set yaw = 0

Eine Geschwindigkeitswarnung, nur über 30 gezeigt:

    name.en Speed warning
    label fast en Fast!
    on frame
    if ball.speed > 30
    do show bottom-right fast dec1 ball.speed

## Was ein Mod enthalten darf, und die Grenzen

Ein Mod kommt von einem Fremden, also liest das Spiel ihn als feindselig und lehnt im Zweifel ab:

- der Ordner des Mods enthält nur `mod.txt` und höchstens eine `readme.txt`/`README.md` und eine
  `licence.txt`/`license.txt` (je 64 KiB, vom Spiel nie gelesen). Eine Verknüpfung, ein Unterordner oder jede andere
  Datei — eine DLL, ein Skript, ein Bild — lehnt den ganzen Mod ab;
- mod.txt: UTF-8, höchstens 32 KiB und 512 Zeilen; jede Zeile muss eine sein, die das Format kennt (ein unbekannter
  Schlüssel, ein unbekanntes Ereignis, ein unbekannter Wert, eine unbekannte Aktion, Option, Ecke oder ein unbekanntes
  Format lehnt den Mod ab, und der Fehler nennt die Zeile); Texte höchstens 160 Zeichen, keine Steuerzeichen oder
  Richtungsüberschreibungen (bidirectional overrides);
- höchstens 16 Optionen, 32 Variablen, 128 Beschriftungen, 64 Regeln, 8 Bedingungen und 16 Aktionen je Regel, 256
  Aktionen; ein Ausdruck höchstens 64 Teile und 16 tief; eine Tastenoption höchstens 8 Tasten; höchstens 32 Mods je
  Ordner;
- ein Arbeitsbudget: die Ausdrucksknoten aller zusammen eingeschalteten Mods höchstens 20 000 (die jedes Mods werden
  beim Lesen gezählt); ein Mod, der es überschreiten würde, wird nicht eingeschaltet, und der Bildschirm Mods sagt warum;
- keine Pfade, keine Verknüpfungen, keine Dateinamen, kein Netz: Ein Mod kann nur die Bausteine oben nennen.

## Noch nicht (vorgemerkt)

- **Begleiter / zwei Kugeln** (DualBallControl). Seine Strings zeigen, was er tut: auf einer Karte mit einem zweiten
  Start (`PS_FourFlames_01_Dual`, mit eigenen `PC_TwoFlames_0*` / `PR_Resetpoint_0*`) lädt er `Balls.nmo` erneut,
  physikalisiert eine zweite Spielkugel und ein zweites `Cam_MF`, und die Wechseltaste schaltet die Steuerung der einen
  Kugel ab und macht die andere zur Kugel des BallManagers. Wir brauchen, alles im Sim: einen zweiten Kugelzustand und
  -körper (heute ist die Kugel die Entität 0 der Welt: `replace_ball(0, ..)`, die Kontakte der Entität 1, der Schub, die
  Klebekraft), die inaktive Kugel als passiven dynamischen Körper, auf welche Kugel Checkpoint, Rücksetzpunkt, Tod,
  Transformer und Extraleben eines Sektors wirken, ein zweites Kamera-Rig oder eines, das zur anderen Kugel springt,
  und den Wechsel als Eingabebit, das das Replay behält (wie beim Startpunkt). Von den vorhandenen Community-Karten ist
  keine mit Doppelstart bekannt (ihre Dateien sind komprimiert, eine einfache Suche beweist nichts); ohne eine zum
  Spielen wartet die Änderung auf eine solche Karte.
- Ein Fuzz-Ziel für mod.txt (der Parser ist streng, begrenzt und per Unit-Test abgedeckt; Fuzzing kommt mit der
  Release-Arbeit).

## Die durchgesehenen BML-Mods

Dreißig BML/BML+-Mods (`ref/Mods`, Windows-DLLs) wurden statisch gelesen — nur ihre Zeichenketten und Readmes; keiner
wurde ausgeführt. Was jeder tut, ob wir es haben und wie es auf unsere Engine abgebildet wird:

| BML-Mod | Was er tut | Bei uns |
|---|---|---|
| AdvancedTravelCam | eine Reisekamera, um überallhin zu fliegen, einen Punkt mit der Maus wählen, filmische Glättung | die freie Kamera von god-mode (F2); `free-view` (Abstand, Drehung); Werks-Mod `smooth-camera` (filmische Glättung) |
| BallanceOptiFine | erzwingt Schatten und eine Übergangssäule in alten Community-Karten; Texturformat | der Renderer zeichnet den Schatten der Kugel auf jeden Boden; kein Mod |
| BaseCmoCfg | bearbeitet die GameSettings von base.cmo (God-Modus, Debug) | `--debug` und sein god-mode (endlose Leben und der Rest) |
| Blackout | verdunkelt alles, ein schwaches Licht über der Kugel | Werks-Mod `blackout` |
| Coredump | schreibt den laufenden Virtools-Kontext in eine .nmo | nicht nötig (unsere Laufzeit ist unsere; `--export` schreibt Level) |
| CursorClipper | hält den Zeiger im Fenster | nicht nötig: Ein Lauf nimmt nichts vom Zeiger, `mouse.dx` liest die rohe Bewegung |
| DeformedWB | verformt das Aussehen der Holzkugel zufällig | Werks-Mod `deformed-ball` |
| DualBallControl | Karten mit zwei Kugeln, eine Taste wechselt die Kugel | vorgemerkt (braucht einen zweiten Spielerkörper; was genau, oben) |
| DynamicFov | das Sichtfeld folgt der Geschwindigkeit der Kugel | Werks-Mod `dynamic-fov`; das feste Sichtfeld ist in Graphics |
| EnergyItemCounter | im Sektor verbleibende Extrapunkte und -leben | Werks-Mod `info-overlay` |
| ExtraSector | lädt Karten mit mehr als acht Sektoren | die Engine: bis zu 64 Sektoren, kein Lader nötig |
| FontCraft | eine andere Schrift für die Oberfläche | unsere eigene Oberfläche und frei lizenzierte Schriften, jede Schrift(art); kein Mod |
| FreeViewRotation | die Maus dreht die Ansicht | Werks-Mod `free-view` (nur die Ansicht), `free-steering` (die Steuerung folgt; gemoddet) |
| Interference | durchscheinende Täuschkugeln um die Kugel | Werks-Mod `interference` |
| LanternFader | Alpha-Test auf dem Material der Laternen gegen Bildeinbrüche | nicht nötig (unser Renderer) |
| MapScripts | führt die Callback-Skripte von Karten aus | geplant: der Interpreter für Kartenskripte (scripts.md); kein Mod |
| NewSpawn | einen Startpunkt setzen, dorthin springen, eine Geisterkugel aufzeichnen | Werks-Mod `spawn-point` (gemoddet); god-mode (F5/F9), `--sector`, `--at`; `spirit-trail` |
| PhysicsEditor | Schwerkraft (ein Vektor), Zeitfaktor der Physik, Spieltempo | Werks-Mod `physics-lab` (gemoddet; Schwerkraft nach unten und seitlich); das Spieltempo ist `[` `]` von god-mode |
| PositionViewer | die Koordinaten der Kugel | Werks-Mod `info-overlay` |
| RealTimeMeter | echte Spielzeit, über Level hinweg | Werks-Mod `segment-timer` (Echtzeit) |
| ScreenCapturer | eine Taste speichert ein Bildschirmfoto (PNG/JPEG, Zwischenablage) | Werks-Mod `screenshot` (PNG; keine Zwischenablage) |
| Segment | die Zeit jedes Sektors gegen den Rekord | Werks-Mod `segment-timer` |
| SpeedrunShortcuts | Cheat-Modus, Startpunkt setzen, Neustart, Sektor setzen | Werks-Mod `quick-restart`; god-mode; `spawn-point` (Startpunkt; gemoddet); `sector-jump` (Sektor setzen; gemoddet) |
| SpiritTrail | der Rekordlauf als durchscheinende Kugel | Werks-Mod `spirit-trail` |
| StickyAccess | im Cheat-Modus zur klebenden Kugel wechseln | Werks-Mod `sticky-ball` |
| SysInfoDisplay | Systemzeit und Akku auf dem Bildschirm | Werks-Mod `info-overlay` (Uhr, UTC); kein Akku |
| TASSupport | Eingaben aufnehmen und abspielen, Zeichnen überspringen, am Ende beenden | die Engine: Replays sind deterministische Aufnahmen der Eingaben (`--record`, `--replay`, `--headless`), die Zeitlupe von god-mode |
| WatchYourTime | Spielzeit je Level | Werks-Mod `play-time` (`playtime.txt`; auch `segment-timer`) |
| BallSticky | ein klebender Kugeltyp, der Wände hinaufklettert | unsere eigene Kugelart `sticky` mit ihrem Transformer `P_Trafo_Sticky` (oben); Werks-Mod `sticky-ball` (gemoddet) macht jede Kugel klebrig |
| BMLModuls | neue Module (hölzerner Fall und drehende Bretter) | Levelteile, keine Mods: die Teile des offenen Formats ([open-format.md](open-format.md)) |

Die Grenzen des Originals, die diese Mods umgehen (Bildratengrenze, 4:3, niedrige Auflösungen, acht Sektoren, eine
Bitmap-Schrift, anfälliges Timing des Kugelwechsels), übernehmen wir nicht.

**Angezeigte Namen.** Das `name.<Sprache>` eines Mods wird in höchstens 40 Zeichen angezeigt: Ein längerer wird an einer Wortgrenze gekürzt und endet auf „…“, das Spiel schreibt eine Warnung ins Protokoll.
