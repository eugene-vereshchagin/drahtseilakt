# Wo die Level liegen

Wie das Spiel seine Level aufbewahrt, mit dem Eigentümer am 2026-11-02 abgestimmt (decisions.md, „Level liegen in
einem Ordner, vier Arten“). Die Menüs, die sie zeigen, stehen in [menu.md](menu.md).

## Die Kartenprüfung

Nach `drahtseilakt-toolbox import-maps` prüft das Spiel jede erzeugte Karte (`drahtseilakt --audit-report DIR|FILE`, das Audit von `--audit`) und legt daneben `report.txt` ab: die Arten von Auffälligkeiten des Audits mit ihrer Anzahl. Die Liste der heruntergeladenen Karten formuliert sie in der Sprache des Spielers unter der gewählten Karte: ein Urteil (läuft vollständig, mit Abweichungen, evtl. nicht schaffbar, lässt sich nicht laden) und bis zu drei Zeilen, etwa unbekannte Skripte, still stehende skriptgesteuerte Teile, fehlende Texturen in Einheitsfarbe, nicht gespielte Skript-Sounds. Die Zeilen sind eine Tabelle über die Audit-Arten (crates/game/src/report.rs), nie über Karten; eine neue Art zählt als "weitere", ein bekannt gewordener Skript-Block verschwindet bei der nächsten Prüfung.

## Die Ordner

Alle Level liegen in einem Ordner, `levels/`, im Datenordner des Spiels (`~/.local/share/drahtseilakt/` unter
Linux, `%APPDATA%\drahtseilakt\` unter Windows, `~/Library/Application Support/drahtseilakt/` unter macOS),
geteilt nach Herkunft und Format:

| Ordner | Was | Woher | Modus |
|---|---|---|---|
| `levels/ballance-original/` | das Originalspiel: `base.cmo`, `3D Entities/`, `Textures/`, `Sounds/` und der Rest, mit dem darin installierten Add-on für Level 13 | vom Spiel aus dem installierten Spiel des Spielers oder von der CD kopiert | klassisch und erweitert |
| `levels/ballance-remaster/` | die Original-Level im offenen Format, mit auf 4K vergrößerten Texturen und was ein modernes Aussehen sonst braucht | auf dem Rechner des Spielers von `drahtseilakt-toolbox remaster` aus `ballance-original/` erzeugt; nie weitergegeben | erweitert |
| `levels/user-levels/` | die eigenen Level des Spielers: Community-`.nmo`-Karten und Level im offenen Format, jedes in einem eigenen benannten Ordner mit einer `level.txt` (Abschnitt „Die Regel“ unten); beim ersten Start leer angelegt, mit einer README.txt, die sagt, wie man ein Level hinzufügt | der Spieler legt sie dort ab; `drahtseilakt-toolbox import-maps` sortiert einen Download hinein | klassisch und erweitert |
| `levels/modified/` | modifizierte Originale: Community-Umbauten der 13 Original-Level, ein Ordner je Satz | eigene Dateien des Spielers; `drahtseilakt-toolbox import-maps` sortiert einen Download ein; nie mitgeliefert | klassisch und erweitert, mit Kopie des Originals |

Start listet sie nach Bedeutung, in beiden Modi (menu.md, „Start nach Bedeutung“): die mitgelieferten Sätze, **Drahtseilakt** (`assets/levels/drahtseilakt/`: die 13 Auswahl-Karten des Eigentümers, offenes Format, gerundet, 4x-Texturen) und **Community** (`assets/levels/community/`: 13 Community-Karten als Original-`.nmo`-Dateien der Autoren mit `level.txt`, Originaltexturen), **Ballance** (`ballance-original/` und `ballance-remaster/`, wenn importiert), `modified/` und **User levels** (`levels/user-levels/`).

**Die Regel (Entscheidung des Eigentümers 2026-11-12).** Ein Level zählt nur dann als Level, wenn es in einem eigenen benannten Ordner liegt, der entweder eine `.nmo`-Datei oder ein Level im offenen Format (`level.gltf` ...) enthält, dazu die Beschreibungsdatei `level.txt`. In `user-levels/` sind Unterordner zum Gruppieren erlaubt (`user-levels/Pro Racing/Season 1/Meine Karte/`): ein Ordner ohne Level ist ein Gruppenordner, das Spiel schaut hinein. Was das Spiel mit dem Rest tut: ein Level-Ordner ohne `level.txt` wird nicht gelistet (früher schon, unter dem Ordnernamen); eine lose `.nmo` direkt im Ordner auch nicht (früher ein eigenes Level); nichts fällt stillschweigend weg: das Konsolen-Log nennt jeden Fall und sagt, was zu tun ist (eigener Ordner, level.txt). Ein Ordner mit mehreren Leveln wird wie bisher übergangen. Die mitgelieferten Ordner und die früheren `community/` und `open/` (solange sie existieren) werden wie bisher gelesen: level.txt ist optional.

**Frühere Ordnernamen (Umzug, 2026-11-12).** Beim ersten Start verschiebt das Spiel `original/` des Spielers nach `ballance-original/`, `remastered/` nach `ballance-remaster/`, `downloaded/` nach `user-levels/` und die Einträge von `community/` und `open/` einzeln nach `user-levels/` (ein Eintrag, dessen Name dort belegt ist, bleibt liegen und wird weiter gelesen); jeder Umzug ist eine Log-Zeile, nichts wird gelöscht oder überschrieben, ein von den Einstellungen anderswohin gelegter Ordner bleibt unberührt (die Einstellung `levels.original` gilt weiter als `levels.ballance-original`). Frühere Läufe werden wiedergefunden.

## Modifizierte Originale

Community-Umbauten der Original-Level (der Satz 45-Degree Rotated, die Reverse Maps, die Invisible Maps und ähnliche)
behalten Geometrie und Texturen des Originals. Darum liefert das Spiel sie nie mit und zeigt sie nur, wenn der Spieler
eine Kopie des Originals hat (wie bei `original/`); gespielt werden sie aus den eigenen Dateien des Spielers. Aufbau:
`levels/modified/<Satz>/<Levelordner>/<Datei>.nmo`, ein Ordner je Satz, darin wie überall ein Ordner je Level (Regeln
von `community/` oben). Das Menü (Start, "Modifizierte Originale"; im klassischen Modus eine Zeile neben dem Original)
zeigt die Sätze nacheinander: 45 Grad gedreht, Rückwärts, Unsichtbar, dann andere Ordner nach Namen; im Satz nach
Nummer des Original-Levels, Variante und Autor.

Der Titel kommt aus dem Datei- oder Ordnernamen, in der Sprache des Menüs, sofern das Level keine eigene `level.txt` hat.
Ein Name wie `反向Level_11(普通-fiype).NMO` wird so gelesen: das Präfix `反向` (rückwärts) entfällt, `Level_11` ist Level 11
des Originals, die Klammer ist `(Variante-Autor)`: Variante 新 ist Neu, 困难 Schwer, 简单 Leicht, 普通 Normal, der Autor
(hier fiype) kommt in die Namenszeile; eine Klammer ohne bekannte Variante ist nur der Autor. Eine Karte ohne Levelnummer
(`反向蓝星序曲.NMO`) zeigt ihren Namen. Läufe werden je Satz gespeichert, wie bei den anderen Arten.

Was die Sätze sind, im Vergleich mit den Originalen: **Invisible** sind die Original-Level mit gelöschtem
Sichtbarkeits-Flag jedes Objekts (Physik unverändert, die Kugel sieht man vor dem Himmel), **Reverse** vertauscht Start
und Ziel und kehrt die Kontrollpunkte um (teils mit zusätzlichen Schienen oder ohne Kisten), **45-Degree Rotated** sind
die um 45 Grad gedrehten Originale. Die Pro-Racing-Saisons sind keine solche Art: neue Geometrie, sie gehören nach
`community/`.

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

Zwei Sätze von Leveln kommen mit dem Spiel, beide Community-Karten, vom Eigentümer ausgewählt (decisions.md, „Keine
eigenen Level“): der **Drahtseilakt-Satz** (`assets/levels/drahtseilakt/01-…13-…`, ins offene Format konvertiert) und
der **Community-Satz** (`assets/levels/community/`, die `.nmo`-Dateien der Karten), jeder in seiner `level.txt` und in
CREDITS.md genannt. Neben ihnen enthält `assets/levels/drahtseilakt/` zwei Level von uns, nur aus unseren eigenen Teilen
gemacht (assets/own, [resources.md](resources.md)) durch Skripte in `assets/own/make/` (`levelkit.py` baut ein Level
aus Bahnstücken und den Platzhaltern der Teile): das Schaufensterlevel `00_Синтетический_уровень` (unten) und
`Parts_Test` (`make_test_level.py`, einige hundert Einheiten Bahn mit jedem Teil darauf oder daneben und einem Stück
Bahn jeder Sicherungsart, für die Tests; durch `hidden = yes` in seiner level.txt aus der Liste der Spieler genommen
und weiter mit `--open` geöffnet). Ihre Textquellen liegen im Repository, die gebauten Dateien nicht. Ihre Lizenz ist
die des Spiels (GPL-3.0-only).

### Das Schau-Level

`00_Синтетический_уровень` („Synthetischer Level", das erste der mitgelieferten: `order = 0`, `difficulty = 1`)
enthält jedes Element des Spiels auf einer Bahn von etwa 1900 Einheiten, jedes mit einem Schild daneben: in ein paar
Minuten durchrollen und in jedem Look mit dem Auge prüfen, ob alles gezeichnet wird, mit den richtigen Texturen,
klingt und sich verhält, wie es soll. Gebaut wird es von `assets/own/make/make_synthetic_level.py` (der klassische
Teil, als Levelbeschreibung mit zwei eigenen Befehlen, `sign` und `gate`) und `synthetic_community.py` (der
Community-Teil), nur aus eigenen Inhalten; das gebaute Level liegt nicht in git.

- **Klassisch**, fünf Sektoren: Böden und Oberflächen (Rinne, Ziegelrand, Geländer, blanker Boden, Holz, Breiten,
  Rampe, runde Kurven, harte Ecken); Schienen (gerade, in Kurven, steigend, fallend) und Kulisse (Holzdeck auf
  Säulen, Lampenturm, Mast, Sockel, Pfeiler, Mauerblock, Girlande, Leuchtturm); die Teile (Extrapunkt und -leben,
  Kisten, Kuppel, die losen Holz-, Stein- und Papierbälle); die Transformer und die Mechanik auf dem Weg
  (Papier und der Ventilator, Wippe, Plattenbrücke, Schaukel, Stein schiebt Kisten, Holz); Stein
  und die Rätselmodule auf dem Weg (Klappenrad, Brettbrücke, Zugbrücke, Schieber, Kiste unter ihrem Stein,
  Aufzug, eine Reihe Säcke, Drehkreuz).
- **Das Tor** „COMMUNITY", hinter dem Himmel, Nebel und Licht wechseln (ein Skript).
- **Community**, zwei Sektoren: die Materialien der Community-Karten (Alpha-Blending, Alpha-Test, additives
  Leuchten, Chrom- und Spiegel-Texturkoordinaten, ein doppelseitiges Blatt, unbeleuchtete Farbe, Normalmap mit
  Parallaxe, ein zweiter Durchgang über ein Mesh, ein 3D-Sprite, ein Glasboden auf dem Weg), dann unsere eigenen
  Verhaltensgraphen (`assets/own/make/scriptkit.py` schreibt sie in die `scripts` des Levels,
  [open-format.md](open-format.md)): Rotate, ein Pendelwagen und ein Lift (Linear Progression, Interpolator, Set
  Position), Text Display, solange der Ball nahe ist, 3D Text, ein Partikelbrunnen, ein Bildschirm mit wechselnden
  Texturen (Set Texture), Show/Hide und Set Diffuse, ein Portal (Box Box Intersection, Set Position auf dem Ball),
  eine Kiste, die Physicalize zum Körper macht.

Was das offene Format noch nicht trägt (beim Bauen gefunden): die Slots einer Textur (Movie Player, Set Current
Slot), Wave-Klänge (Wave Player), eine eigene Textur des Levels für Skript-Partikel, ein zur Sicht gedrehtes
3D-Sprite des Levels. Seit den Anmerkungen 10 des Besitzers (2026-11-16) steht jedes klassische Element so, wie die Level des Originals
es setzen (gemessen an den Leveln 1-13), der Weg führt durch jedes, aus unseren eigenen Teilen: zwei Ventilatorstufen
(Level 3: ein Ventilator auf einer 15 x 15 Plattform hebt den Papierball auf Schienen 11 höher, die über eine Lücke von
10 zur nächsten Plattform führen, deren Ventilator ihn auf einen Boden 14 höher hebt, der auf einer Wand an ihrem Rand
steht), sechs Mulden über der Leere (Level 7), die Wippe über einer Lücke von 12,9 (Level 2-9), die Plattenbrücke über
15,4 mit ihrem Rahmen 6,9 vom nahen Rand (Level 2-10), die Schaukel über einer Lücke von 15 über einem 6,2 tiefen
Graben zwischen zwei Wänden (Level 8), das Klappenrad zwischen zwei Bahnen im Abstand 15, gekippt von einem Steinball,
der rollend auf seine stehende Klappe trifft (Level 9-11), die Brettbrücke über 9,1 (Level 2, 3, 5, 8), die
Zugbrücke, die in eine Lücke von 7,6 einer Querbahn fällt (Level 5), zwei Schieber, geschoben von einem Boden 2,5
tiefer (Level 1), die Kiste unter ihrem Stein in einem Schlitz von 5,3 über einem breiten Boden 5 tiefer (hier 34 x 28,
in Level 1 34 x 42), vom unter dem Stein durchrollenden Steinball herausgestoßen und auf diesem Boden bleibend, zurück
nach oben über eine Rampe auf der fernen Seite (Level 1); der Aufzug (P_Modul_03; Level 7, 9, 10, 12): ein Holzball von
einem Transformer 23 davor kommt auf einem Boden 11,3 unter dem Rahmen ins Tor des Käfigs, rammt die sieben Wände ab
und fährt auf der Scheibe zum weiteren Weg 8,9 höher (Level 9; die Scheibe steigt mit unserem Teil 16,5, mit dem der
Kopie 13,8); drei Säcke im Abstand 10 über einer Bahn, 3 darüber gehängt (Level 9, 10, 12, 13): ein Holzball rollt
durch, ein unter einem wartender Papierball wird hinuntergestoßen; das Drehkreuz auf einer Nabe in einer quadratischen
Schleife von Bahnen 10 von ihr entfernt, der Weg um drei ihrer Seiten, die Arme in Ruhe quer über den Bahnen (Level
9); und eine Schienenspirale mit zwei Windungen. Im Community-Teil liegen der Glasboden, unser Skript-Aufzug (der Weg
endet an seiner unteren Stellung und geht 7 höher weiter) und das Portal (am Ende einer Bahn, sein zweiter Bogen hinter
einer Lücke von 18) auf dem Weg. Von den 13 konvertierten Community-Karten stehen nur ein Glasboden (Karte 10), das
Gitter des Ventilators, Portale (Karte 05: Trigger-Boxen auf dem Boden des Wegs) und P_Box-Kisten auf ihren Wegen;
keine nutzt texgen, unbeleuchtete oder additive Materialien, einen zweiten Durchgang, Sprites, Rotate,
Progressionen, Skript-Aufzüge, Text Display, Set Texture, Blinken oder die Himmel-, Nebel- und Lichtblöcke, daher
stehen diese Exponate neben dem Weg, ohne Community-Umgebung zum Nachbilden. `crates/game/src/showcase.rs` prüft,
dass jedes unserer Teile im Level steht und jedes Element in der Physik des Spiels seine Aufgabe erfüllt: die
`checks.txt` des Levels (vom Generator aus seinen `check`-Zeilen geschrieben, 29 Prüfungen) rollt eine Kugel der
richtigen und der falschen Art mit den Pfeiltasten durch jedes Element und prüft, wohin sie kommt, wie weit sich die
Teile des Moduls drehen oder bewegen und wo ein herausgestoßenes Teil liegt; jede Prüfung ist Pflicht.
`SHOWCASE_COPY_PARTS=<Kopie>` führt dieselben Prüfungen mit den Teilen der Kopie statt unserer aus,
`SHOWCASE_ONLY=<Präfix>` nur die so benannten, `SHOWCASE_TRACE=<Prüfung>` gibt den Weg der Kugel aus. Der Lauf des
Testspielers wird für das Level nicht mehr aufbewahrt.
Ohne Kopie des Originals steht hinter dem Menü der erste mitgelieferte Level nach dem Schaulevel (order 0), damit das
Menü nicht seine Schilder zeigt.
Seit den Notizen 11 des Eigentümers (2026-11-16): die Bahn der Zugbrücke hat zu beiden Seiten ihrer Lücke eine Stufe, 1 lang und 0,5 tief, wie in Level 5, so dass das gefallene Deck bündig mit der Bahn liegt (die Prüfung misst die Höhe der Kugel darauf); die Mulden aller Checkpoints und des Starts werden ganz gezeichnet; keine zwei sichtbaren Flächen liegen in einer Ebene übereinander (der Baukasten lehnt sie ab, `check_coplanar`, und `kit_levels_have_no_faces_flickering_in_one_plane` prüft jedes von ihm gebaute Level); Schilder stehen, wo kein Weg vorbeiführt; das Gitter des Ventilators ist ein Raster aus Stäben mit dunkler Mitte wie das des Originals; wo die Steinkugel auf blankem Boden fährt, hat jede scharfe Ecke ein Geländer an ihrer Außenseite (die schmalen Steinbahnen des Originals sind stattdessen Rinnen, 0,68 tief); die klebrige Kugel, die kein Level des Originals und keine konvertierte Community-Karte hat (sie stammt aus dem Mod BallSticky), steht im Community-Teil, mit einem Hang von 40 Grad, den sie aus dem Stand hinauffährt und die Holzkugel nicht.

## Der Levelbaukasten

*2026-11-16.* Der Baukasten unten (`assets/own/make/leveldsl.py`, `levelkit.py`, `levelparts.py`, `levelcheck.py`) wurde
für zehn eigene Level geschrieben (2026-11-03). Der Eigentümer hat sie am 2026-11-11 zurückgestellt und am 2026-11-16
entfernen lassen (decisions.md, „Keine eigenen Level“); der Baukasten bleibt, weil das Schaufensterlevel und
`Parts_Test` mit ihm gemacht sind und seine Prüfungen an ihnen laufen (`crates/game/src/testlevels.rs`). Was er über
„unsere Level“ sagt, gilt für jedes mit ihm gemachte Level.

**Ein Level wird als Text geschrieben**: seine Beschreibung, `level.lvl`, in einer kleinen
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
`loose wood|stone|paper|sticky`; `seesaw` (über einer Lücke von der Länge ihres Bretts), `platebridge` (eine Lücke von 14,8),
`swingbridge` (eine Schaukel in einer Lücke von 5), `sack`, `arms`, `flaps` (das Klappenrad über einer Lücke von 6,4 in
einem etwa 6 breiten Weg, wie es die Level des Originals setzen: eine Klappe liegt über der Lücke, die andere steht neben
dem Weg); `module P_Modul_NN` mit `ahead`, `side`,
`up`, `turn`, `tilt` für jedes andere Teil; `move`, `turn`, `mark NAME` und `back NAME`, um den Cursor zu bewegen,
ohne zu bauen. Die ganze Liste steht oben in leveldsl.py.

`railing left|right|both|off` setzt Stahlgeländer an die Ränder der Böden von dort an, auf jedem Profil (Level 1:
Rinnen, und Geländer, wo man fallen kann). Zwei Rätsel des Originals sind eigene Befehle: `crateblock` (der Stein auf
der Kiste, P_Modul_34: ein Schlitz von 6 quer über den Weg, darin der Stein auf seiner Kiste, die auf einem 5 tieferen
Boden steht, den die Beschreibung unter den Schlitz legt; eine Kugel auf diesem Boden schiebt die Kiste unter dem
Stein hervor, und der Stein rutscht bündig mit dem Weg herab) und `pusher left|right` (P_Modul_01: ein Schlitz von 1,6
quer über einen höchstens 6 breiten Weg, der Block des Schiebers füllt den Weg, seine Zielscheibe steht auf dieser
Seite 5 über den Rand hinaus über einem 2,5 tieferen Boden, von dem aus eine Kugel den Block aus dem Weg schiebt). Die
Böden darunter werden mit `mark`, `move`, `turn` und `back` gelegt.

**Ecken, Plätze und Kulissen** (2026-11, Neubau von Level 1; `assets/own/make/levelparts.py`). `corner left|right [rail]
[gap G]` ist eine harte rechtwinklige Ecke: das Ende des letzten Bodens ist ein Gehrungsring (der Querschnitt steht auf
der Winkelhalbierenden, 1/cos 45 breiter), gemeinsam mit dem folgenden Boden; die Außenecke ist gefüllt, das Profil läuft
ohne Bruch weiter; `rail` setzt außen ein Geländer 8 beiderseits, `gap` lässt es G lang geradeaus offen. `plaza W L
[brick|kerb] [stone|wood] [gap SIDE C W]... [fence SIDE...]` ist eine ebene Fläche ab der Mitte ihrer Hinterkante mit
Ziegelrand oder niedriger Kante außer an den Lücken und Geländern an den Seiten in `fence`; `disc R [rim] [ring [gap W]]`
eine runde Fläche; `pit L [depth D] [wall LEN] [side S]` eine Grube im Weg mit Prellwand; `mark`/`back` beginnen und
beenden einen Seitenast. Freistehende Kulissen (nur gezeichnet): `lamptower`, `mast`, `plinth`, `garland`, `wallblock`,
`wlamp`, `lighthouse`; `scatter D` verteilt Türme, Masten und Sockel entlang des Wegs abseits der Böden. `loose KIND lead`
und `plug` sind vom Platzcheck ausgenommen. Plätze, Scheiben und Gruben liegen 0,03 unter den anschließenden Böden
(zwei ebene Teile auf gleicher Höhe wären ein Riss für den Nahttest).

**Himmel.** `sky` wählt die Himmelsbox um das Level: `A`, `B`, `C` (unsere, über einem Wolkenmeer, wie die Buchstaben
des Originals) oder einen der fotografischen Himmel mit Namen: `Mountains` (ein Abendhimmel über bewaldeten Hügeln und
Bergen mit Flüssen, Seen, Felsgraten und Schnee auf den höchsten Gipfeln), `Forest` (Sommerwolken über einem Blätterdach mit Lichtungen und Seen), `Sea` (ein ruhiges
Meer im Sonnenuntergang, das die Wolken spiegelt), `Storm` (schwere Wolken über dunklem Meer mit Schaumkronen),
`Sunset` (ein goldener Sonnenuntergang über einem Wolkenmeer), `Clouds` (ein Sommertag über einem
Wolkenmeer), `Night` (die Milchstraße über einem dunklen See zwischen
Hügeln), `Desert` (später Nachmittag über roten Canyons), `Mist` (Abendlicht über Hügeln mit Nebel in den Tälern).
Ihr Himmel sind CC0-Fotos von Poly Haven; was unter dem Horizont liegt, zeichnet `assets/own/make/make_skies.py`, wie
aus großer Höhe gesehen, sodass die Bahn hoch darüber schwebt (`assets/own/CREDITS.md`); unter `Mountains`, `Forest` und `Mist` wird
das grüne Land Punkt für Punkt im Maßstab jedes Pixels gezeichnet (`assets/own/make/skyland.py`: senkrecht unten
die Baumkronen, in der Ferne ein kühler bläulich-weißer Dunst) als Gitter unter dem Level. **Das Land unter der
Bahn** (2026-11-16, der Eigentümer nach dem Vorbild von Ballex: das Land soll nah sein, keine Karte aus dem
Flugzeugfenster): `make_skies.py land` (auch Teil von `make`) schreibt `Sky_<Name>_LandHeight.png` (16-Bit-Höhen, 641
je Seite), `Sky_<Name>_LandColor.jpg` (4096, von der Sonne des eigenen Himmels beleuchtet) und `Sky_<Name>_Land.txt`;
das Spiel (`crates/game/src/terrain.rs`, `terrain.wgsl`) legt es so, dass das Level über dem tiefsten Gelände hängt,
senkt es weich ab, wo es näher als 290 unter dem tiefsten Boden an etwas vom Level käme (weiter weg darf es um 0,85 je
Einheit steigen: seitlich reichen die Berge etwa bis zur Höhe der Bahn), gibt ihm nah eine Körnung aus Steinen und
Kronen und lässt es mit der Entfernung im Dunst unter dem Horizont des Himmels verschwinden; die Seiten zeigen unter
dem Horizont nur noch Dunst (wieder 1024). Es bewegt sich mit der Kamera wie das Level (Parallaxe), die Wolken liegen
zwischen ihm und der Bahn, die Physik sieht es nicht; die Himmel des Originals haben keins. Etwa 110 MB Videospeicher
und 0,2 bis 0,6 ms je Bild (RTX 3060, 2560x1440); das Standard-Wolkenmeer umgewandelter Karten unter diesen Himmeln
ist nun aufgelockert (cover 0,3). Jeder andere Name ist ein
Satz Dateien `Textures/Sky/Sky_<Name>_<Seite>` (Front, Right, Back, Left, Down und Top, falls vorhanden) aus dem
Levelordner oder den Texturen; ein Name besteht aus Buchstaben, Ziffern, `_` und `-`. `--sky NAME` auf der
Kommandozeile spielt jedes Level unter einem anderen Himmel.

**Das Wolkenmeer.** Unter `Sunset` und `Clouds` schweben die Level des Baukastens über einem Meer aus Haufenwolken, das das Spiel
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
den Leveln 1 und 2, 130 bis 540 in 3 bis 8, unter 90 in 9 bis 11 und keine in 12. `safety ART` setzt sie von dort an (die erste in einer Datei ist die des Levels); `borders off`
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
(testlevels.rs) wirft jeden Ball mit 12, 25 und 40 Einheiten in der Sekunde seitlich gegen die Geländer und Ränder von
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

**Keine Nähte** (2026-11-03): ein Stück Boden ist ein Netz vom Start oder einem Checkpoint bis zum nächsten, sein
Querschnitt wird aufrecht quer zur eigenen Richtung des Cursors (in einer Kurve genau) mitgeführt, so haben eine
Gerade, die in eine Kurve übergeht, oder eine Rampe, die in eine Ebene übergeht, dort, wo sie sich treffen, denselben
Querschnitt: kein Keil aus Nichts dazwischen; jeder Ring von Punkten gehört den Vierecken davor und danach (kein Riss,
kein T-Stoß), und die Texturen liegen nach der Strecke entlang des Wegs. Die Stücke des Starts und der Checkpoints
sind ebenso angeschlossen: an ihren Kanten, wo ein Boden weiterläuft, liegen ihre Ecken auf den Querschnittspunkten
des Bodens selbst, und ihre Oberseite ist wie die Bahn texturiert. Der Test `kit_floors_have_no_seams` (testlevels.rs)
prüft die Böden jedes Levels, das der Baukasten gemacht hat (ab Version 2, in `asset.generator` von level.gltf): wo
die Oberseite eines Bodens endet, liegt daneben in ihrer Höhe keine andere (`the_seam_finder_sees_a_crack` prüft die
Suche selbst). Ein Checkpoint auf einem schmalen Weg bekommt eine kleinere Schale und mit ihr weniger Tiefe, so
bleiben ihre Hänge die gemessenen (eine Steinkugel, die in einer steileren Schale lag, rollte nicht heraus).

**Platz, Schienen und Wolken** (2026-11-03, nachdem der Besitzer Level 1 gespielt hat): ein Weg muss der Kugel (Radius
2) auf der Höhe ihrer Mitte mindestens 5,5 Breite lassen, zwischen seinen Rändern oder den Holmen seiner Geländer
(levelkit.MIN_FREE); der Baukasten schreibt kein Level mit einer engeren Stelle und nennt jede (ein 6 breiter Weg mit
Ziegelrändern ließ 4, und die Kugel klemmte). Eine Kiste oder eine lose Kugel muss so viel Platz neben sich lassen,
mit dem, was nahe bei ihr quer über dem Weg liegt, außer sie ist mit `shove` markiert und die Kugel, die der Weg dort
hat (die des Starts, dann die jedes Transformators), ist nicht leichter (Holz 1,9, Stein 10, Papier 0,2, eine Kiste
1); keine lose Kugel liegt näher als 14 am Ende von Schienen (sie rollt in ihren Trichter). Schienen haben die Breite
des Originals, gemessen: Rohre 0,6 dick, ihre Mitten 3,9 auseinander, so sitzt die Kugel tief zwischen ihnen; unsere
liegen auf dem Boden, den sie verlassen (die Kugel sinkt 0,48 auf sie), und wo von einem Ende ein Boden weiterläuft,
biegt sich jedes Rohr flach über die ebene Fläche des Bodens nach außen, von oben ein Trichter (keiner auf einer
Rinne, die die Kugel selbst zur Mitte führt); Pfosten unter den Rohren halten sie, Schwellen quer zwischen ihnen nur
mit `rails ... ties`, für spätere Level. Ein Transformator steht gleich vor dem, wofür seine Kugel da ist. Die flache
Oberseite jedes Profils ist an denselben Stellen quer geschnitten (1, 1,25 und 2,5 von den Rändern, die letzte
mindestens 1,5 von der Mitte, wo der Fuß einer schmalen Rinne liegt), so teilen Stücke verschiedener Profile, die Ende
an Ende aufeinandertreffen, ihre Ecken; jede Fläche zeigt dahin, wohin ihre Normale zeigt (die Flächen der Bahn waren
vorher verkehrt gewickelt, ihre Normalen zeigten nach innen). Der Beton der Bahn ist dunkler und grauer als der
sandige Ton des Originals, die Fugen der Platten schwächer (Wunsch des Besitzers). Unter jedem Level liegt eine
Wolkenschicht wie die des Originals (`SkyLayer`, in der Datei verborgen, vom Spiel gezeigt, wenn die Optionen Wolken
haben, additiv über den Himmel gezeichnet): eine Ebene 165 unter dem tiefsten Boden, 500 über das Level hinaus nach
allen Seiten, unsere eigenen grauen Wolken auf Schwarz, alle 675 wiederholt.

**Stützen, Kurven und Aussehen** (2026-11-03, dritter Blick des Besitzers): Schienen halten ein paar Sättel unter
beiden Rohren (unter der Kugel, die zwischen ihnen rollt) auf Säulen hinab in den Dunst, etwa einer alle 40 entlang,
keiner nahe ihren Enden (dort halten sie die Böden) und keiner über einem anderen Boden; `railing turns` setzt von
dort an ein Geländer an die Außenseite jeder Kurve (Level 1 hat es), und ein Geländer nur an der Innenseite einer
Kurve wird abgelehnt. Die Wolkenschicht liegt nur unter den Himmeln, die ein Wolkenmeer sind (A, B, C, Sunset); Level
1 liegt unter `Mountains`. Die Seiten der Bahn und die Säulen haben Regenspuren von ihrer Oberkante, die Säulen sind
in Ringen gegossen (`Track_Column`). Das Netz eines Teils kann ein feineres Aussehen tragen (ownkit.py, `Mesh.look`:
was das Spiel zeichnet, das Netz selbst bleibt der Physik): die Transformatoren sind wie die Platte des Originals
gezeichnet (ein Chromrahmen um eine Scheibe aus dem Material der Kugel, farbige Knöpfe auf einer genieteten Platte),
die Papierkugel als zerknülltes Blatt, die Seile der Ballons am Korb festgebunden.

**Tore für die Kugeln** (2026-11-03, in der Simulation auf Rampen von 20 Länge gemessen): die Holz- und die
Papierkugel steigen aus dem Stand eine Rampe von 33 Grad hinauf; die Steinkugel bleibt aus dem Stand bei 19 Grad
stehen und mit 30 Anlauf bei 24, so ist eine Rampe, die auf 20 um 10 steigt (27 Grad), ein Tor, das nur Holz (oder
Papier) passiert. Level 1 setzt eines gleich nach jeden Holz-Transformator und einen Stein-Transformator gleich vor
das, wofür die Steinkugel gebraucht wird (eine lose Steinkugel, die Kiste, eine Kiste auf einem kahlen Stück, die
Schieber); seine langen leeren Geraden sind gekürzt und zu Bögen geworden. Eine Kuppel muss auf beiden Seiten den
Platz eines Wegs lassen (die DSL lehnt ein Level ab, wo sie es nicht tut, bei der Breite, die der Boden dort hat);
Kisten und lose Kugeln werden bei derselben Breite geprüft. Säulen unter den Böden und unter Schienen halten Abstand
zu jedem Boden unter ihnen, nach dessen eigener Breite.

**Luft und Tore zum Herausfinden** (2026-11-03): ein Level im offenen Format kann in seinen Einstellungen Nebel tragen
(`fog`: `color` zum Horizont, `color_down` gerade hinab, `density` je Einheit Entfernung, und `below`/`depth`: unter
dieser Höhe vergeht alles innerhalb dieser Tiefe ganz in ihm). Der Renderer zeichnet ihn auf allem Beleuchteten (der
Himmel, Sprites und die Wolkenschicht behalten ihre Farbe); die Physik sieht ihn nie, so bleiben Wiederholungen
gleich, und die Level des Originals haben keinen. Unser Baukasten nimmt ihn aus den Farben des Himmels selbst (der
Dunst gleich unter seinem Horizont, die Welt darunter) und lässt die Säulen und die Stützen der Schienen 240 tief in
ihn reichen, so verblassen sie, statt abzubrechen; jeder Himmel außer Night bekommt die Wolkenschicht, als zwei Decks
verstreuter Wolkenbänke über einer eigenen Welt. Nicht jeder Wechsel der Kugel liegt auf dem Weg: in Level 1 kommt die
Steinkugel an einen Anstieg, den sie nicht schafft, der Holz-Transformator steht seitlich auf dem Platz davor; die
Holzkugel kommt an eine Kiste, die sie nicht verschiebt, der Stein-Transformator steht seitlich auf dem Boden darunter
(eine schnelle Holzkugel verschiebt sie manchmal doch); jeder Sektor, der eine Kugel braucht, gibt sie weiterhin.

**Nach den Konzeptbildern des Besitzers** (2026-11-03, seine Bilder nur als Vorbild): Schienen aus abgenutztem,
gebürstetem verzinktem Stahl mit leichtem Rostton und Relief (`Track_Rail`, eine Normal-Map), eine Schelle um jedes
Rohr alle 9 entlang, ihr Bügel unter dem Weg der Kugel zum anderen Rohr hinab gebogen, und an den Stützen ein dünner
Stahlstab vom Bügel hinab in den Dunst (die Beschläge nur gezeichnet); die Kugeln glatt gezeichnet (ihre groben Netze
bleiben der Physik); der Start auf einem Steinturm hinab in den Dunst; Laternen auf Holzpfosten am Start und neben
jedem Checkpoint.

**Teile stehen auf ebenem Boden** (2026-11-03): der Baukasten lehnt ein Level ab, in dem die Platte eines
Transformators, eine Kiste oder eine Kuppel nicht auf einem ebenen, waagerechten Stück an ihrem Fuß steht (ihre
Grundfläche wird abgetastet: jede Bodenoberseite darunter innerhalb von 0,05 ihres Fußes, nichts fehlt), und nennt
jede; wo eine steht, wird die Rinne verbreitert oder das Teil versetzt. Eine Kugel, die wechselt, wirft ihre Haut ab:
ihre Stücke sind gebogene Bruchstücke ihrer eigenen Schale, außen ihre Textur, innen eine dünne Seite.

**Lose Körper liegen auf dem Boden, nicht darin** (2026-11-09): der Platz eines Teils (`ahead`, `side`) ist ein
Punkt auf der Geraden des Cursors in seiner Höhe, hinter einem Gefälle, neben einer Kurve oder an der Schräge einer
Rinne also nicht auf dem Boden: Level 1 hatte lose Kugeln bis 2,4 tief darin, und die Physik stieß jede zu Beginn
ihres Sektors heraus und schleuderte den Ball des Spielers, der sie berührte, mit Hunderten Einheiten pro Sekunde
davon. Der Baukasten legt jetzt jede lose Kugel auf das, was unter ihr ist (aus einem Boden gehoben, auf einen
gesenkt; die Papierkugel als Kugel von 2,04, der Reichweite ihrer Hülle), und lehnt ein Level ab, in dem eine lose
Kugel oder eine Kiste mehr als 0,05 in Böden, Schienen oder Stoppern beginnt oder eine lose Kugel auf nichts liegt,
und nennt jede (`levelkit.py --check ORDNER` prüft ein fertiges Level). Eine auf ein Gefälle gelegte Kugel wird
gemeldet (sie rollt zu Beginn ihres Sektors davon). Das Spiel selbst hebt einen losen Körper, der mehr als 0,1 in der
statischen Geometrie beginnt, vor dem Erzeugen auf sie hinauf; die Level des Originals haben keinen tiefer als 0,031
(die Papierkugeln von Level 10), es bewegt also keinen der ihren.

**Raum über der Kugel** (2026-11-03): über einem Transformator (sein Käfig hebt und hält die Kugel), einem Ventilator
und dem Ballon darf nichts von Böden oder Schienen innerhalb von 12 hängen; und ein Weg darf unter einem anderen Boden
oder Schienen nur verlaufen, wenn deren Unterseite höchstens 3,6 über ihm liegt (eine Wand, auf die die Kugel trifft)
oder 6 und mehr (Platz zum Durchrollen), nie dazwischen, wo eine Kugel klemmt. Der Baukasten lehnt ein Level ab, das
eines davon verletzt, und nennt die Stellen. Eine Kiste kann auf einer anderen stehen (`box ... up 5`).

**Geometrieprüfungen** (2026-11-09, nach dem Spiel des Eigentümers in Level 1): das Kit schreibt kein Level, das eine von drei Prüfungen (`levelcheck.py`) nicht besteht, und nennt, was und wo. *Nähte:* kein Riss und keine Stufe von 0,005 bis 0,05, wo zwei Bodenstücke zusammentreffen (Plätze, Scheiben, Abzweige, Ecken liegen genau auf der Höhe des Bodens; ein flaches Stück trifft ein Profil nur an dessen flacher Mitte). *Kulissen:* kein Lampenturm, Mast, Sockel, keine Girlande, kein Wandblock und keine Säule innerhalb von 5 über der Oberseite eines Bodens (4 der Kugel und 1 Rand; seitlich 0,3, bei Schienen 2), außer die Beschreibung markiert es mit `obstacle`. *Abkürzungen:* kein Boden, der auf der Strecke 30 oder mehr weiter liegt, darf von einem früheren durch Fall oder Flug von einer Kante erreichbar sein (Kugel mit 20 Einheiten/s, Schwerkraft 20; gemessene Höchstgeschwindigkeiten: Papier 13,9, Holz 15,9, Stein 17,9; ein geneigtes Stück gibt Anstieg; eine Kante mit Geländer oder Rand ist keine Absprungkante) oder von der Spitze eines Gebläses (17 über seinem Boden), außer die Landung ist in der Beschreibung mit `drop [R]` markiert. Lose Körper im Boden (`check_clear`) bleiben verboten.

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

**Um ein Level zu bauen** aus seiner Beschreibung: `python3 -I assets/own/make/leveldsl.py DIR/level.lvl` (das
Schaufensterlevel: `make_synthetic_level.py`). `scripts/video.sh LEVEL RUN out.mp4` macht aus einem Lauf davon ein Video.
Ein Level wiegt 3,3 bis 3,8 MB, 2,6 MB davon die Texturen, die es mitbringt (das Format hält die Texturen eines Levels in seinem eigenen Ordner).

## Das Originalspiel: standardmäßig kopiert

Das Spiel kopiert einmal, was es braucht, aus der eigenen Kopie des Spielers nach `levels/ballance-original/`: aus einem
installierten Spiel (`drahtseilakt --import <Ballance-Ordner>`) oder direkt von der CD des Spiels oder ihrem
Abbild (`drahtseilakt --import Ballance.cue`, auch `.bin`, `.iso`: das InstallShield-6-Archiv in `SETUP/`, gelesen
von `crates/cdimage`). Die Kopie ist freiwillig (decisions.md, „Das Spiel läuft ohne
Weiteres“): ohne sie startet das Spiel mit den mitgelieferten Sätzen, und die Options-Seite sagt, dass die Originallevel mit `drahtseilakt-toolbox` importiert werden (ein eigenes Programm; das Spiel hat kein Importfenster und startet es nie). Die Kopie sind die eigenen Daten des Spielers auf seinem Rechner; nichts wird
heruntergeladen, nichts kommt ins Repository. `--data DIR` spielt weiterhin aus jedem Ordner ohne Kopie.

Das README des Ordners nennt, was von Hand zu kopieren ist: `base.cmo` und die Ordner `3D Entities`, `Textures`
und `Sounds` eines installierten Ballance (mehr liest das Spiel nicht) und für Level 13 das SuDu-Add-on, in diese Kopie installiert, wie
sein Readme es sagt.

## Jeder Ordner lässt sich verlegen

Jeder der vier kann in den Einstellungen woandershin zeigen (`levels.original`, `levels.community`,
`levels.remastered`, `levels.open`), für eine große Platte oder einen geteilten Ordner. Der Standardordner behält
dann nur ein README, das sagt, wo die Level sind, damit ein Spieler, der dort sucht, sie findet.

## Was mit dem Spiel kommt

Das Spiel wird mit einigen Leveln in `levels/open/` installiert (unser Schau- und unser Testlevel) und mit umgewandelten
Community-Karten, deren Autoren sie mit dem Spiel ausliefern möchten. Jedes solche Level braucht:

- die Erlaubnis seines Autors und eine freie Lizenz, daneben festgehalten (`LICENSE` in seinem Ordner);
- nichts vom Originalspiel darin: keine Texturen, Klänge oder Meshes von Ballance. Eine umgewandelte Karte, die
  Texturen des Originals nutzt, nennt sie beim Namen, und das Spiel nimmt sie aus `levels/ballance-original/` (der Kopie
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

*Seit 2026-11-12:* der Kind-Name im Schlüssel folgt dem Ordner (`user-levels`, `ballance-original`, `ballance-remaster`); unter den früheren Namen (`downloaded`, `community`, `open`, `original`, `remastered`) gespeicherte Läufe werden beim ersten Spielen des Levels unter den neuen Schlüssel verschoben.

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

**Angezeigte Titel.** Der Titel eines Levels (und die Namen der Community-, Open-Format-, heruntergeladenen und Challenge-Level) wird in höchstens 40 Zeichen angezeigt: Ein längerer wird an einer Wortgrenze gekürzt und endet auf „…“, und das Spiel schreibt eine Warnung ins Protokoll.
