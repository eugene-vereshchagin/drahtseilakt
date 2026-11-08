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
flache Liste der Level, nach Schwierigkeit und Titel sortiert, ohne Ordner zum Öffnen. Alles, was der Autor eines
Levels und ein Spieler wissen müssen, steht gesammelt auf [making-levels.md](making-levels.md).

- **Ein Level ist ein Ordner direkt im Ordner seiner Art** mit genau einem Level: einer `.nmo`-Datei oder einem Level im
  offenen Format (`level.gltf`, `level.bin`, `textures/`; open-format.md). Das offene Format ist das klare, leicht von
  Hand fertigzustellen oder nachzubessern: gewöhnliche glTF- und PNG-Dateien.
- Daneben, alles freiwillig: `level.txt`, einfache Zeilen `Schlüssel = Wert` (`title`, `author`, `version`;
  `title.<Code>`, sein Titel in einer anderen Sprache, z. B. `title.ru`, gezeigt, wenn die Menüs sie sprechen; `order`,
  eine Zahl, die es vor die Level ohne eine stellt, in ihrer Reihenfolge; `difficulty`, wie schwer es ist, eine ganze Zahl von 1
  (leicht) bis 10, alles andere gilt als unbekannt; `hidden = yes`, nicht in der Liste der mit dem Spiel gelieferten
  Level, wie das Testlevel; `#` beginnt einen Kommentar, andere Schlüssel werden übergangen), ein README (`README.txt` oder `README.md`), ein Vorschaubild (`preview.png` oder
  `preview.jpg`) und die Ordner der Ressourcen, die das Level nutzt (`textures/`, `game/`).
- **Der Titel** ist `title` aus level.txt (auf Englisch; andere Sprachen als `title.<Code>`), sonst der Ordnername. Er wird bereinigt (Steuerzeichen,
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
  Zeichen, ein zweites „Hello Mars“ mit „(2)“); die Quelle wird nur gelesen. Ein zweiter Lauf fügt nur Neues hinzu. Mit `--catalogue CustomMapList.md` (der
  Kartenliste der Community) füllt es level.txt aus dem Katalog: den englischen Titel, den Originalnamen als
  `title.zh`, den Autor und die Schwierigkeit (die 1–5 des Katalogs verdoppelt); eine Karte, die weder im Katalog
  steht noch ein Readme hat, wird nicht importiert, sondern mit gepackten Archiven und Ordnern, die nicht genau ein
  Level sind, in `levels/community-unsorted/` beiseitegelegt, mit einer README.txt, die sagt, warum
  (`--unsorted DIR` wählt einen anderen Ordner); `--refresh` tut dasselbe mit einem früher importierten Ordner und
  schreibt nur seine level.txt-Dateien neu. Das Menü listet nach Schwierigkeit, dann Titel; Level unbekannter
  Schwierigkeit danach, Level ohne `title` (nach ihrem Ordner benannt) zuletzt.

## Level, die mit dem Spiel kommen

Unsere eigenen Level kommen mit dem Spiel in seinem `assets/levels/`, ein Ordner je Level im offenen Format, nur aus
unseren eigenen Teilen gemacht (assets/own, [resources.md](resources.md)), sie spielen also ganz ohne Kopie des
Originals. Das Menü zeigt sie mit den Leveln im offenen Format aus `levels/open/`: unter Start, wenn es keine Kopie
gibt (die Liste „Our levels“), unter „Open format“ des erweiterten Modus immer. Sie entstehen durch Skripte in
`assets/own/make/` (`levelkit.py` baut ein Level aus Bahnstücken und den Platzhaltern der Teile; `make_test_level.py`
das erste, `Parts_Test`, einige hundert Einheiten Bahn mit jedem Teil darauf oder daneben und einem Stück Bahn jeder
Sicherungsart, für die Tests; es ist durch `hidden = yes` in seiner level.txt aus der Liste der Spieler genommen und
öffnet weiter mit `--open`), deren Ergebnisse im Repository liegen. Ihre Lizenz ist die des Spiels (GPL-3.0-only).

## Unsere Level und wie man sie bearbeitet

*2026-11-03.* Zehn eigene Level, vom ersten Rollen bis zum Spießrutenlauf, kommen mit dem Spiel (das
Release-Kriterium aus roadmap.md). Jedes ist neu: sein Verlauf ist unserer, nur die Mechanik ist die des Originals.
Das Menü listet sie in dieser Reihenfolge (`order` in level.txt; ihre Schwierigkeit, `difficulty`, läuft mit ihr von 1 bis 10), unter ihren Namen in der Sprache der Menüs:

| # | Level | Sektoren | Idee | Himmel, Musik, Bonus |
|---|---|---|---|---|
| 1 | First Roll (Erster Lauf) | 3 | breite Böden mit Geländern, sanfte Kurven und Hänge, ein erstes Stück Schienen, ein kleiner Sprung | A, 1, 100 |
| 2 | Wood and Stone (Holz und Stein) | 4 | die Holzkugel erklimmt die Rampen, die die Steinkugel nicht schafft; die Steinkugel schiebt Kisten und lose Kugeln über Kanten ohne Rand | B, 2, 200 |
| 3 | Paper Winds (Papierwinde) | 4 | die Papierkugel fährt auf Ventilatoren zu den Böden darüber; Plattenbrücken tragen sie | C, 3, 300 |
| 4 | Swing Time (Schaukelzeit) | 4 | was sich bewegt: schwingende Säcke, drehende Arme, ein Klappenrad, Schaukeln, die eine Lücke überbrücken, während sie vorbeischwingen | A, 4, 400 |
| 5 | Bridges and Boards (Brücken und Bretter) | 5 | Wippen unter jeder Kugel; Plattenbrücken, die unter der Steinkugel brechen | B, 5, 500 |
| 6 | Stone Work (Steinarbeit) | 5 | Kisten, lose Kugeln und Kuppeln im Weg: von der Steinkugel weggestoßen, von der Holzkugel umfahren | C, 1, 600 |
| 7 | Rail Runner (Schienenläufer) | 5 | lange Schienen durch Kurven, hinauf und hinab; schmale Böden ohne Rand | A, 2, 700 |
| 8 | Shifting Shapes (Wechselnde Gestalten) | 5 | jeder Sektor will eine andere Kugel: Papier für Ventilatoren und Plattenbrücken, Stein für Kisten, Holz für Rampen und Wippen | B, 3, 800 |
| 9 | Leap of Faith (Sprung ins Ungewisse) | 5 | Tempo und Sprünge: lange Abfahrten, Rampen, die die Kugel über Lücken werfen, schnell genommene Schienen | C, 4, 900 |
| 10 | The Gauntlet (Spießrutenlauf) | 6 | schmale Wege, jede Kugel und jedes Teil nacheinander | A, 5, 1000 |

Jedes Level beginnt mit der Holzkugel und hat Kontrollpunkte mit ihren Flammen, Extrapunkte und ein Extraleben.

**Ein Level wird als Text geschrieben.** Jeder Ordner enthält seine Beschreibung, `level.lvl`, in einer kleinen
Sprache, die `assets/own/make/leveldsl.py` liest; es schreibt das Level daneben (`level.gltf`, `level.bin`,
`textures/`, `level.txt`) mit `levelkit.py`. Nur die eigenen Texturen des Levels (sein Satz Platten, Randsteine,
Schatten und Hänge) liegen als Dateien in seinem `textures/`; die Texturen, die alle Level teilen, sind als die des
Spiels genannt (`game:`-Bilder, open-format.md) und werden mit dem gewählten Texturpaket gezeichnet
([packs.md](packs.md)). Die Beschreibung legt die Bahn wie eine Schildkröte: ein Cursor (ein Punkt
auf der Oberseite des Bodens und eine Richtung, am Start entlang +X, links ist +Z) rückt mit jedem Stück weiter, und
Teile werden dorthin gesetzt, wo er ist. Eine Zeile ist ein Befehl; `#` beginnt einen Kommentar:

```
title Paper Winds            # der Name des Levels; title.ru, title.de, title.zh: in anderen Sprachen
order 3                      # sein Platz in der Liste
difficulty 3                 # wie schwer, 1 bis 10 (difficulty in level.txt: das Menü sortiert danach)
sky C                        # A, B, C oder ein Himmel mit Namen: Mountains, Sea, ... (unten)
music 3                      # das Thema, 1 bis 5
bonus 300                    # der Levelbonus
ball wood                    # die Startkugel
columns 45                   # Säulen unter den Böden alle 45 Einheiten Weg, für das Aussehen

start                        # die Plattform des Starts; der Weg geht von ihrer Vorderkante weiter
safety railings              # wie die Bahn den Ball hält: railings, gutter, border oder none (unten)
width 12                     # die Breite des Bodens (borders off|on, surface stone|wood)
floor 30                     # ein gerader Boden, 30 lang (floor 30 up 4, floor 30 down 6: eine Rampe)
right 90 26                  # ein Boden, der sich um 90 Grad nach rechts dreht, Radius 26 (left A R)
point                        # ein Extrapunkt hier (life: ein Extraleben; side S, ahead A: daneben, weiter vorn)
trafo paper                  # ein Transformator
fan ahead -5                 # ein Ventilator 5 zurück: eine Papierkugel steigt auf ihm bis etwa 17 über seinen Boden
gap 4 up 12                  # 4 vor, 12 hoch: nichts gebaut (ein Sprung oder der Boden, zu dem ein Ventilator trägt)
checkpoint                   # ein Kontrollpunkt (die Richtung ein Vielfaches von 90): der nächste Sektor beginnt
finish                       # der Ballon, 22 voraus
```

Außerdem: `rails L`, `rails-left A R`, `rails-right A R` (Schienen, auch hinauf oder hinab); `box`, `dome`,
`loose wood|stone|paper`; `seesaw` (über einer Lücke von der Länge ihres Bretts), `platebridge` (eine Lücke von 14,8),
`swingbridge` (eine Schaukel in einer Lücke von 5), `sack`, `arms`, `flaps`; `module P_Modul_NN` mit `ahead`, `side`,
`up`, `turn`, `tilt` für jedes andere Teil; `move`, `turn`, `mark NAME` und `back NAME`, um den Cursor zu bewegen,
ohne zu bauen. Die ganze Liste steht oben in leveldsl.py.

**Himmel.** `sky` wählt die Himmelsbox um das Level: `A`, `B`, `C` (unsere, über einem Wolkenmeer, wie die Buchstaben
des Originals) oder einen der fotografischen Himmel mit Namen: `Mountains` (ein Abendhimmel über dunstigen Bergen mit
Schnee auf den Gipfeln), `Forest` (Sommerwolken über einem Blätterdach mit Lichtungen und Seen), `Sea` (ein ruhiges
Meer im Sonnenuntergang, das die Wolken spiegelt), `Storm` (schwere Wolken über dunklem Meer mit Schaumkronen),
`Sunset` (ein goldener Sonnenuntergang über einem Wolkenmeer), `Clouds` (ein Sommertag über einem
Wolkenmeer), `Night` (die Milchstraße über einem dunklen See zwischen
Hügeln), `Desert` (später Nachmittag über roten Canyons), `Mist` (Abendlicht über Hügeln mit Nebel in den Tälern).
Ihr Himmel sind CC0-Fotos von Poly Haven; was unter dem Horizont liegt, zeichnet `assets/own/make/make_skies.py`, wie
aus großer Höhe gesehen, sodass die Bahn hoch darüber schwebt (`assets/own/CREDITS.md`). Jeder andere Name ist ein
Satz Dateien `Textures/Sky/Sky_<Name>_<Seite>` (Front, Right, Back, Left, Down und Top, falls vorhanden) aus dem
Levelordner oder den Texturen; ein Name besteht aus Buchstaben, Ziffern, `_` und `-`. `--sky NAME` auf der
Kommandozeile spielt jedes Level unter einem anderen Himmel.

**Das Wolkenmeer.** Unter `Sunset` und `Clouds` schweben unsere Level über einem Meer aus Haufenwolken, das das Spiel
als Volumen zeichnet (`crates/game/src/clouds.wgsl`): Wolkenballen aus kachelndem Rauschen, beleuchtet von der Sonne
des Himmels (hell, wo sie sie trifft, blauviolett im Schatten) und vom Himmel darüber, in der Ferne im Dunst des
Himmels am Horizont verschwindend; Säulen und Stützen tauchen in sie ein. Unter `Mountains`, `Forest`, `Mist` und
`Desert` sind es aufgerissene Bänke mit Lücken, durch die tief unten das Land zu sehen ist. Der Levelbaukasten
schreibt es in die Einstellungen des Levels (`clouds`, [open-format.md](open-format.md)), aus dem Licht des Himmels
(`assets/own/make/sky_light.json`, geschrieben von `make_skies.py`) und seiner Tabelle (`levelkit.py`, `CLOUD_SEA`);
ein Level ändert es mit `clouds cover C below B thickness T size S` (beliebige davon: wie viel davon Wolke ist, 0..1;
die Kuppen B unter dem tiefsten Boden, sonst 35; T dick, 110; eine Wolke S breit, 85) oder hat mit `clouds off` keins.
Die Level des Originals haben keins. `cloud-quality` in der Einstellungsdatei (`low`, `medium` (Vorgabe), `high`;
`off` zeichnet keins) legt fest, wie fein es gezeichnet wird, und *Clouds?* aus blendet es ebenfalls aus
([menu.md](menu.md)). Auf einer RTX 3060 bei 2560×1440 kostet es etwa 3,5, 4,5 und 6 ms je Bild (low, medium, high).
`make_skies.py` macht `Sunset` und `Clouds` mit nur Dunst unter dem Horizont: ihr Wolkenmeer zeichnet das Spiel,
und der Levelbaukasten schreibt es in ein Level unter ihnen; ein Level ohne es (`clouds off` oder anders gemacht)
zeigt unten den Dunst.

**Wie die Bahn den Ball hält** (`safety`): Im Original nimmt die Hilfe von Level zu Level ab, und unsere Level folgen
dem. Gemessen an den dreizehn Leveln des Originals (die Kanten seiner Böden, nur Zahlen): in den Leveln 1, 3 und 5
sind etwa zwei Drittel der Bodenkanten profiliert (der Boden steigt zur Kante um 0,7 auf 2,5 Breite an: eine Rinne,
die einen abdriftenden Ball zur Mitte zurückrollt; ein Drittel ist blank), ab Level 6 nur ein Viertel bis ein Drittel
(der Rest blank); erhöhte Wände sind selten (nur die hölzernen Halfpipes von Level 4 und einige Stücke in 12 und 13);
Stahlgeländer (ein Rohr etwa 2,6 über dem Boden, auf Pfosten) laufen an etwa 490 und 420 Einheiten der Bodenkanten in
den Leveln 1 und 2, 130 bis 540 in 3 bis 8, unter 90 in 9 bis 11 und keine in 12. Unsere Level
gehen denselben Weg, eine Stufe deutlicher: Level 1 und 2 haben Geländer, 3 und 4 Ziegelränder (3 einen Sektor mit
Geländer), 5 bis 8 Rinnen (5 einen Sektor mit Rändern, 8 den letzten blank), 9 und 10 blanke Böden (9 den ersten
Sektor als Rinne). `safety ART` setzt sie von dort an (die erste in einer Datei ist die des Levels); `borders off`
macht die Böden von dort an blank, `borders on` gibt ihnen die Art des Levels zurück. Was jede ist:

- `railings`: Stahlgeländer an beiden Kanten, Holme 1,3 und 2,6 über dem Boden auf Pfosten alle 6 Einheiten, in der
  Physikgruppe der Schienen wie die des Originals;
- `gutter`: der profilierte Boden des Originals, die Kanten 0,7 über einer flachen Mitte, ansteigend auf 2,5 in zwei
  Flächen;
- `border`: Ziegelränder 1 breit und 2,2 hoch an beiden Kanten, die Ziegel der Menüszene. Höher als die Mitte des
  Balls (sein Radius ist 2), so trifft ein Ball die Fläche gerade und prallt zurück: über einen niedrigeren Rand (1,4)
  rollte ein mit 25 Einheiten in der Sekunde dagegen geworfener Ball hinweg;
- `none`: der blanke Boden, seine Kanten nur durch den Ziegelstreifen markiert.

Jeder Rand und jedes Geländer gehört zur Physik der Böden: der Test `borders_and_railings_keep_the_ball_on_the_track`
(ownlevels.rs) wirft jeden Ball mit 12, 25 und 40 Einheiten in der Sekunde seitlich gegen die Geländer und Ränder von
Parts_Test und schiebt ihn weiter dagegen, und er bleibt auf der Bahn; ein in die Rinne treibender Ball rollt zurück;
vom blanken Boden geworfen fällt er.

**Das Aussehen der Bahn** folgt, wie die Teile, dem, was die Böden des Originals lesbar macht (gemessen an seinen
Böden und Texturen, nur Zahlen): eine Textur deckt 5 Einheiten Boden; Platten von 5 mal 5 mit Fugen dazwischen, jede
mit eigenem Ton, Schmutz und Abrieb, vier entlang einer Textur (Wiederholung nach 20 Einheiten), einer von drei
Plattensätzen je Level; an jeder Kante ein Bordstein mit dem Streifen kleiner Ziegel des Originals, seiner Rille und
seinem Schatten; die Hänge der Rinne zu ihrem Grund hin dunkler; der Schatten, den ein Rand neben sich wirft; die
Seiten dunklerer Beton mit eigenem Ziegelstreifen. Die Texturen liegen in `assets/own/Track/` (make_textures.py,
Schritt `track`); die Ränder nehmen die Ziegel der Menüszene. Sie werden nach der Strecke entlang der ganzen Bahn
aufgetragen, und nacheinander gelegte Stücke (gerade, gebogen, ein Stück mit wechselnder Breite) sind ein Netz, so
laufen die Texturen ohne Naht von Stück zu Stück.

**Start und Checkpoints** sind Schalen wie die des Originals (gemessen, nur Zahlen): der Start eine runde Plattform mit
Radius 10 und einer Schale von Radius 7,5 und 1,25 Tiefe in der Mitte (der Ball kommt darin zur Ruhe), vier
Flammenschalen an ihrem Rand; ein Checkpoint ein Bodenstück von 10 Länge mit einer 1,26 tief eingelassenen Schale von
Radius 5 (bis zu einer Lippe) und zwei Flammenschalen quer zum Weg, ±7 zur Seite, wo die Flammen des Teils brennen.
Die Böden vor und nach einem Checkpoint enden an seinem Stück. Ihre Rahmen und Rücksetzpunkte stehen in den Schalen
(die Rücksetzpunkte 3,65 und 3,33 über dem Grund, wie beim Original).

**Was der Testspieler über ein Level weiß**, steht in derselben Datei. Aus dem gelegten Weg schreibt leveldsl.py die
Hinweisdateien des Testspielers für drei Stile in den Ordner `pilot/` des Levels (autopilot.md, „Hinweisdateien“):
`safe` (rollt von Punkt zu Punkt und hält an, nimmt jeden Kontrollpunkt, kein Risiko), `speed` (rollt ohne Halt durch
die Punkte) und `collector` (wie safe, und holt jeden Extrapunkt und jedes Extraleben abseits des Wegs). Lücken,
Wippen und drehende Arme werden in jedem Stil mit Tempo genommen (von den vorsichtigen mit Anlauf); eine Stelle, die
mehr braucht, sagt es mit `pilot STYLES : STEPS` (die Schritte des Hinweises, ihre Punkte als `ahead side up` um den
Cursor; z. B. ein auf eine Schaukel abgestimmtes Warten) und `nopilot STYLES` / `autopilot STYLES` um ein Stück, das
die Stile von selbst nehmen.

**Um ein Level zu ändern**: seine `level.lvl` bearbeiten, dann

```
scripts/own-levels.sh assets/levels/03_Paper_Winds
```

baut es und lässt den Testspieler es in jedem Stil spielen; jeder Lauf, der ins Ziel kommt, wird als
`pilot/<style>.replay` behalten (die Läufe werden in einem eigenen Datenordner unter `target/` gespielt). Der Test
`crates/game/src/ownlevels.rs` spielt die drei Läufe jedes Levels ohne Kopie des Originals und schlägt fehl, wenn einer
nicht ins Ziel kommt; so wird ein Level nie ohne einen Lauf ausgeliefert, der beweist, dass es zu schaffen ist.
`scripts/video.sh assets/levels/03_Paper_Winds assets/levels/03_Paper_Winds/pilot/speed.replay out.mp4` macht aus
einem Lauf ein Video. Ein Level wiegt 3,3 bis 3,8 MB, 2,6 MB davon die Texturen, die es mitbringt (das Format hält die Texturen eines Levels in seinem eigenen Ordner).

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
