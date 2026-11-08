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
| `physics-lab` | PhysicsEditor | Schwerkraft (0–60, im Original 20) und der Zeitfaktor der Physik (0,5–4, im Original 2) | ja (gemoddet) |
| `dynamic-fov` | DynamicFov | die Ansicht wird mit der Geschwindigkeit der Kugel breiter; Option: Stärke | nein |
| `free-view` | FreeViewRotation, AdvancedTravelCam | die Maus dreht die Ansicht um die Kugel, eine Taste dreht sie zurück; Abstand der Kamera | nein |
| `segment-timer` | Segment, RealTimeMeter, WatchYourTime | die Zeit des Laufs, die Echtzeit, die Zwischenzeit jedes Sektors gegen den besten Lauf (grün voraus, rot zurück) | nein |
| `info-overlay` | PositionViewer, EnergyItemCounter, SysInfoDisplay | Position, Geschwindigkeit, im Sektor verbleibende Extrapunkte und -leben, Bilder je Sekunde, Uhr (UTC) | nein |
| `quick-restart` | SpeedrunShortcuts | eine Taste (Backspace, Entf, F4 oder Q) startet das Level sofort neu | nein |
| `screenshot` | ScreenCapturer | eine Taste (F12, F10, Druck oder P) speichert das Bild als PNG in `screenshots/` des Datenordners | nein |
| `spirit-trail` | SpiritTrail | der beste Lauf des Levels rollt als durchscheinende Kugel neben diesem; Option: Deckkraft | nein |
| `blackout` | Blackout | das Level wird dunkel, ein schwaches Licht über der Kugel; Optionen: Dunkelheit, Licht | nein |

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
und nur mit genau diesem Mod: Ein Replay, dessen Mod fehlt oder sich geändert hat, wird mit Angabe des Grundes
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
  `version`, `licence`, `requires <game version>` (z. B. `0.1`; eine neuere als die des Spiels lehnt den Mod ab).
- **Optionen**, die der Spieler auf der Seite des Mods setzt:
  - `option <name> switch on|off`
  - `option <name> number <min> <max> <step> <default>`
  - `option <name> key <Key1,Key2,...>` — Tasten mit den Namen von winit (`KeyQ`, `F12`, `Backspace`, `Home`...); die
    erste ist die Vorgabe. Die eigenen Tasten des Spiels werden abgelehnt: Escape, Enter, die Pfeile, Shift, Space,
    Tab, Alt, F11 und Backquote; eine Taste, die der Spieler auf der Seite Controls belegt hat, bleibt beim Spiel.
    (R, F2, F3, F5, F9, `[` `]` gehörten den Übungswerkzeugen; jetzt sind es die Tasten von god-mode und für jeden Mod
    frei.)
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
| `ball.kind` | 0 Holz, 1 Stein, 2 Papier | `clock.hour` `clock.minute` `clock.second` | die Uhr, UTC * |
| `ball.touching` `ball.wall` | sie berührt etwas; etwas Steiles | `mouse.dx` `mouse.dy` | die Bewegung der Maus in diesem Bild * |
| `run.tick` `run.time` | die Ticks des Laufs, Sekunden | `extras.points` `extras.points_all` | im Sektor verbleibende Extrapunkte, und alle * |
| `practice.speed` | das Tempo des Spiels (1, oder bis 1/8) * | `practice.saved` | ein Moment ist gespeichert (1 oder 0) * |
| `run.sector` `run.lives` `run.points` | Sektor (1 der erste), Leben, Punkte | `extras.lives` `extras.lives_all` | verbleibende Extraleben, und alle * |
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
- `do world.time <0.5..4>` — der Zeitfaktor der Physik (im Original 2).
- `do lives.endless <0|1>` — die letzte Kugel geht nie verloren (god-mode).

Übungshandlungen (nur auf `key`; sie greifen von außen in den Lauf ein, also ist ein Mod mit einer davon ebenfalls
gemoddet, und solange er an ist, behält das Spiel die Momente des Laufs, einen je Sekunde für die letzte halbe Minute):

- `do rewind <1..30>` — so viele Sekunden zurück; `do save`, `do load` — den Moment behalten, zu ihm zurück;
- `do slower`, `do faster` — das Tempo des Spiels halbiert (bis 1/8) oder verdoppelt (bis zum normalen);
- `do freecam` — die freie Kamera aufgenommen oder abgelegt.

Ansichtsaktionen (nur auf `frame` oder `key`):

- `do camera.fov <degrees>` — die Ansicht um so viele Grad verbreitert (-30 bis 60).
- `do camera.distance <0.5..2>` — die Kamera näher oder weiter weg.
- `do camera.yaw <degrees>` — die Ansicht um die Kugel gedreht (die Pfeile schieben weiter entlang der eigenen
  Richtung der Kamera).
- `do light <0..1>` — das Licht des Levels; `do lantern <0..1>` — ein Licht über der Kugel.
- `do show <corner> <label|-> <format> <value>[, <value>[, <value>]]` — eine Einblendungszeile in diesem Bild; Ecken
  `top-left top-right bottom-left bottom-right`; Formate `time` (mm:ss.hh aus Sekunden), `int`, `dec1`, `dec2`,
  `clock` (hh:mm), `ratio` (a/b).
- `do splits <corner>` — die Zwischenzeiten der Sektoren gegen den besten Lauf.
- `do keys <corner>` — die eigenen Tastenoptionen des Mods als Einblendungszeilen, „Beschriftung: Taste“ (der Hinweis
  von god-mode).
- `do ghost <0..1>` — der beste Lauf als durchscheinende Kugel, so deckend.
- `do screenshot`, `do restart`.

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
- keine Pfade, keine Verknüpfungen, keine Dateinamen, kein Netz: Ein Mod kann nur die Bausteine oben nennen.

## Noch nicht (vorgemerkt)

- **Begleiter / zwei Kugeln** (DualBallControl): eine zweite Kugel, zu der die Tasten wechseln, braucht einen zweiten
  Spielerkörper im Lauf — ein eigener Baustein, mit dem Wechsel im Replay aufgezeichnet.
- **Eine eigene klebende Kugelart** mit ihrem Transformator (`P_Trafo_Sticky` von BallSticky): Heute kann jede Kugel
  klebend gemacht werden.
- **Steuern mit der gedrehten Ansicht** (FreeViewRotation dreht auch die Steuerung): Dafür muss die Drehung der Maus im
  Replay aufgezeichnet werden; heute dreht `camera.yaw` nur die Ansicht.
- **Täuschkugeln** (Interference), **eine verformte Kugel** (DeformedWB): Zeichenbausteine für zusätzliche Kugeln und
  Rauschen im Netz.
- **Zu einem Startpunkt springen, den Sektor im Lauf setzen** (NewSpawn, SpeedrunShortcuts): Save und Load von
  god-mode und `--sector` decken das Üben ab; ein Baustein, der den Sektor setzt, würde daraus einen Mod machen.
- **Spielzeit je Level** (die Statistik von WatchYourTime): ein Baustein für einen behaltenen Zähler.
- Ein Fuzz-Ziel für mod.txt (der Parser ist streng, begrenzt und mit Unit-Tests geprüft; das Fuzzing kommt mit der
  Arbeit am Release).

## Die durchgesehenen BML-Mods

Dreißig BML/BML+-Mods (`ref/Mods`, Windows-DLLs) wurden statisch gelesen — nur ihre Zeichenketten und Readmes; keiner
wurde ausgeführt. Was jeder tut, ob wir es haben und wie es auf unsere Engine abgebildet wird:

| BML-Mod | Was er tut | Bei uns |
|---|---|---|
| AdvancedTravelCam | eine Reisekamera, um überallhin zu fliegen, einen Punkt mit der Maus wählen, filmische Glättung | die freie Kamera von god-mode (F2); `free-view` (Abstand, Drehung); Glättung vorgemerkt |
| BallanceOptiFine | erzwingt Schatten und eine Übergangssäule in alten Community-Karten; Texturformat | der Renderer zeichnet den Schatten der Kugel auf jeden Boden; kein Mod |
| BaseCmoCfg | bearbeitet die GameSettings von base.cmo (God-Modus, Debug) | `--debug` und sein god-mode (endlose Leben und der Rest) |
| Blackout | verdunkelt alles, ein schwaches Licht über der Kugel | Werks-Mod `blackout` |
| Coredump | schreibt den laufenden Virtools-Kontext in eine .nmo | nicht nötig (unsere Laufzeit ist unsere; `--export` schreibt Level) |
| CursorClipper | hält den Zeiger im Fenster | nicht nötig: Ein Lauf nimmt nichts vom Zeiger, `mouse.dx` liest die rohe Bewegung |
| DeformedWB | verformt das Aussehen der Holzkugel zufällig | vorgemerkt |
| DualBallControl | Karten mit zwei Kugeln, eine Taste wechselt die Kugel | vorgemerkt (Begleiter-Baustein) |
| DynamicFov | das Sichtfeld folgt der Geschwindigkeit der Kugel | Werks-Mod `dynamic-fov`; das feste Sichtfeld ist in Graphics |
| EnergyItemCounter | im Sektor verbleibende Extrapunkte und -leben | Werks-Mod `info-overlay` |
| ExtraSector | lädt Karten mit mehr als acht Sektoren | die Engine: bis zu 64 Sektoren, kein Lader nötig |
| FontCraft | eine andere Schrift für die Oberfläche | unsere eigene Oberfläche und frei lizenzierte Schriften, jede Schrift(art); kein Mod |
| FreeViewRotation | die Maus dreht die Ansicht | Werks-Mod `free-view` (nur die Ansicht; Steuerung vorgemerkt) |
| Interference | durchscheinende Täuschkugeln um die Kugel | vorgemerkt |
| LanternFader | Alpha-Test auf dem Material der Laternen gegen Bildeinbrüche | nicht nötig (unser Renderer) |
| MapScripts | führt die Callback-Skripte von Karten aus | die Engine führt die Verhaltensgraphen von Karten als Daten aus; kein Mod |
| NewSpawn | einen Startpunkt setzen, dorthin springen, eine Geisterkugel aufzeichnen | god-mode (F5/F9), `--sector`, `--at`; `spirit-trail` |
| PhysicsEditor | Schwerkraft, Zeitfaktor der Physik, Spieltempo | Werks-Mod `physics-lab` (gemoddet); das Spieltempo ist `[` `]` von god-mode |
| PositionViewer | die Koordinaten der Kugel | Werks-Mod `info-overlay` |
| RealTimeMeter | echte Spielzeit, über Level hinweg | Werks-Mod `segment-timer` (Echtzeit) |
| ScreenCapturer | eine Taste speichert ein Bildschirmfoto (PNG/JPEG, Zwischenablage) | Werks-Mod `screenshot` (PNG; keine Zwischenablage) |
| Segment | die Zeit jedes Sektors gegen den Rekord | Werks-Mod `segment-timer` |
| SpeedrunShortcuts | Cheat-Modus, Startpunkt setzen, Neustart, Sektor setzen | Werks-Mod `quick-restart`; god-mode; Sektor setzen vorgemerkt |
| SpiritTrail | der Rekordlauf als durchscheinende Kugel | Werks-Mod `spirit-trail` |
| StickyAccess | im Cheat-Modus zur klebenden Kugel wechseln | Werks-Mod `sticky-ball` |
| SysInfoDisplay | Systemzeit und Akku auf dem Bildschirm | Werks-Mod `info-overlay` (Uhr, UTC); kein Akku |
| TASSupport | Eingaben aufnehmen und abspielen, Zeichnen überspringen, am Ende beenden | die Engine: Replays sind deterministische Aufnahmen der Eingaben (`--record`, `--replay`, `--headless`), die Zeitlupe von god-mode |
| WatchYourTime | Spielzeit je Level | vorgemerkt (Statistik) |
| BallSticky | ein klebender Kugeltyp, der Wände hinaufklettert | Werks-Mod `sticky-ball` (gemoddet) |
| BMLModuls | neue Module (hölzerner Fall und drehende Bretter) | Levelteile, keine Mods: die Teile des offenen Formats ([open-format.md](open-format.md)) |

Die Grenzen des Originals, die diese Mods umgehen (Bildratengrenze, 4:3, niedrige Auflösungen, acht Sektoren, eine
Bitmap-Schrift, anfälliges Timing des Kugelwechsels), übernehmen wir nicht.


## Präzisierungen nach dem Review

- Zahlenoption: Grenzen innerhalb von ±1 000 000, Schritt mindestens 0,001, der Standardwert liegt auf den Schritten ab `min`.
- Eine Tastenoption darf keine Tasten des Spiels belegen: Escape, Enter, die Pfeiltasten, Shift, Space, Tab, Alt, F11
  und Backquote; eine vom Spieler auf der Seite Controls belegte Taste bleibt beim Spiel (R, F2, F3, F5, F9, `[` `]`
  sind jetzt die Tasten von god-mode und für jeden Mod frei).
- `do screenshot` und `do restart` nur in `on key`-Regeln (nie in jedem Bild); höchstens ein Screenshot pro Sekunde.
- Ein Replay, dessen Mod-Optionen fehlen, unbekannt oder außerhalb des Bereichs sind, wird mit Begründung abgelehnt.
- Arbeitsbudget: die Ausdrucksknoten aller eingeschalteten Mods zusammen höchstens 20 000 (beim Lesen gezählt); ein Mod, der es überschreiten würde, wird nicht eingeschaltet, und der Bildschirm Mods sagt warum.
