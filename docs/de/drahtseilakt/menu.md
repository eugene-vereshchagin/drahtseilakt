# Menüs: das Schema des Originals und unseres

Der Plan für die Menüs des Spiels, mit dem Eigentümer am 2026-11-02 abgestimmt. Bis der Spieler den erweiterten
Modus einschaltet, ist das Spiel so genau wie möglich das Original; der einzige sichtbare Unterschied ist eine
weitere Option, die den erweiterten Modus einschaltet. Der erweiterte Modus hat dann eigene Einstellungen: was
gezeigt wird, welche unserer Ergänzungen genutzt werden, und Community-Karten.

## Die Menüs des Originals

Gelesen aus `3D Entities/Menu.nmo` der Kopie (die Skripte `Menu_*`) und `Language.nmo` (die Texte auf Deutsch,
Englisch, Spanisch, Italienisch und Französisch; `GameSettings.Language` wählt die Spalte), geprüft im C-Spiel.

| Bildschirm | Zeilen von oben nach unten | Hinweise |
|---|---|---|
| Hauptmenü (`Menu_Main`) | Start, Highscore, Options, Credits, Exit | Exit fragt „Do you want to quit the game?“ (Yes / No) |
| Hauptmenü ohne Kopie (unseres) | Start, Highscore, Options, Credits, Exit | Start listet „Our levels“ (die mitgelieferten und `levels/open/`); Options zeigt eine Notiz, dass die Originallevel mit `drahtseilakt-toolbox` importiert werden (ein eigenes Programm; das Spiel hat kein Importfenster und startet die Toolbox nie) (decisions.md, „Das Spiel läuft ohne Weiteres“) |
| Start (`Menu_Start`) | Level 1 … Level 12, Back | ein Level ist spielbar, sobald das vorige geschafft ist (`DB_Levelfreischaltung`); die anderen sind grau. Das Add-on mit Level 13 ersetzt Menu.nmo und fügt seine Zeile hinzu |
| Highscore (`Menu_Highscore`) | „Highscore Level N“, zehn Namen mit Punkten, Back; links und rechts wechseln das Level | die Vorgaben der Kopie sind die Arrays `DB_Highscore_LvNN` in base.cmo („Mr. Default“) |
| Options (`Menu_Options`) | Graphics, Controls, Sound, Back | bei uns vor Back noch Language und Extended |
| Graphics (`Menu_Opt_Graphics`) | Screen Resolution, Synch to Screen? (Yes/No), Clouds? (Yes/No), OK | Vorgaben der Kopie (`DB_Options`): keine Synchronisation, Wolken |
| Controls (`Menu_Opt_Keys`) | Forward, Backward, Left, Right, Rotation, Overview: je eine Taste; Invert Rotation? (Yes/No); OK | Vorgaben: Pfeile, Shift, Leertaste; `DB_Options` speichert eine Taste als ihre Zeile in `all_keys` aus `Language.nmo` (die Scancodes von DirectInput der Reihe nach) |
| Sound (`Menu_Opt_Sound`) | Music Volume, Sound Volume, OK | Lautstärke der Musik und der Effekte (bei uns beide in jedem Modus, wie der Inhaber das Original kennt) |
| Credits (`Menu_Credits`) | der Abspann des Originals | |
| Pause, Esc im Level (`Menu_Pause`) | Back, Restart Level, Highscore, Options, Exit Level | Restart und Exit fragen zuerst Yes / No; die Punkte bleiben sichtbar |
| Tot, keine Leben mehr (`Menu_Dead`) | Restart Level, Home | |
| Punkte im Ziel (`Menu_Score`) | Level Bonus, Time Points, Extra Lives, Score | Zeile für Zeile hochgezählt |
| Neuer Highscore (`Menu_HighscoreEntry`) | „New highscore entry!“, die Punkte, ein Name zum Eintippen, OK | der zuletzt eingegebene Name wird angeboten (`DB_Options.LastPlayer`) |
| Levelende (`Menu_End`) | Next level, Restart Level, Highscore, Options, Home | |

Einstellungen und Highscores liegen in `Database.tdb` im Spielordner. Spielerprofile gibt es nicht: das einzig
Persönliche ist der Name in der Highscore-Liste.

## Unser klassischer Modus (Vorgabe)

- Dieselben Bildschirme, Zeilen, Reihenfolge und Texte wie in der Tabelle, gezeichnet mit unserer eigenen Oberfläche
  (decisions.md, „Die eigene Oberfläche ist die Vorgabe“; [resources.md](resources.md), „Die eigene Oberfläche“):
  unsere Texturen, unsere Schrift, unsere Texte. Der Schalter Interface des erweiterten Modus kann die des Originals
  wählen, mit den Texturen und der Schrift der Kopie und in der Sprache ihrer `Language.nmo`.
- **Zwei Ergänzungen** in Options: *Language* und *Extended*. *Language* öffnet eine Seite mit den Sprachen, jede in
  ihrem eigenen Namen (English, Русский, Deutsch, 简体中文, Français, Español, Italiano), die gesprochene in Gold; Enter
  spricht die gewählte sofort und speichert sie (`language de`). Bis der Spieler wählt, sprechen die Menüs die Sprache
  des Systems (die Locale-Variablen `LC_ALL`, `LC_MESSAGES`, `LANG`; Windows und macOS setzen sie für ein vom Desktop
  gestartetes Programm nicht), wenn sie eine davon ist, sonst Englisch. *Extended* hat den Schalter *Extended mode?*
  (Yes / No). Das ist alles, was ein Spieler der Original-Level verändert sieht.
- Level werden wie im Original freigeschaltet. Highscores wie im Original: zehn pro Level, Punkte, ein Name. Sie
  beginnen mit unseren eigenen Vorgabelisten (mit der Oberfläche des Originals mit denen der Kopie) und werden in unserem eigenen Datenordner gespeichert, nie in den Dateien
  des Spiels. Die `Database.tdb` des Spielers zu lesen, um alte Punkte zu übernehmen, ist eine spätere Option
  (ein Parser: geprüft wie jeder Dateileser).
- **Graphics**: die drei Zeilen des Originals und eine weitere, *Fullscreen?* (No / Yes / Exclusive), da ein Fenster
  den ganzen Bildschirm füllen kann. Links und rechts (oder Enter) ändern eine Zeile; die Änderung wirkt sofort und
  wird in `settings.txt` gespeichert:
  - *Screen Resolution* geht durch die Modi des Monitors und die üblichen Fenstergrößen (`resolution 1600x900`); das
    Fenster nimmt diese Größe an, ein exklusiver Vollbildmodus diesen Modus;
  - *Synch to Screen?* lässt die Bilder auf den Monitor warten (`vsync on`; aus als Vorgabe, wie in der Kopie, und weil
    das Warten auf dem Testrechner hängen blieb; `DRAHTSEILAKT_PRESENT` geht weiter vor);
  - *Clouds?* zeigt die Wolkenschicht der Level (`clouds on|off`), ihr Objekt `SkyLayer`, das `Gameplay.nmo` zeigt,
    wenn die Option an ist. Ihre Textur sind graue Wolken auf Schwarz und ihr Material mischt nicht, also zeichnen wir
    sie zum Himmel dahinter addiert (unsere Lesart; das Material der Kopie sagt es nicht). Die Videoverfolgung zeichnet
    ohne sie;
  - `cloud-quality low|medium|high|off` (die Einstellungsdatei; sonst `medium`): wie fein das Wolkenmeer unserer Level
    gezeichnet wird (levels.md, „Das Wolkenmeer"): Schritte hindurch, Schritte zur Sonne, kleine Ballen an den Rändern;
    `off` zeichnet keins, und *Clouds?* aus blendet es ebenfalls aus;
  - *Fullscreen?* (`fullscreen off|borderless|exclusive`): randlos nimmt den Modus des Desktops, exklusiv ändert ihn,
    wo die Plattform es kann (Wayland nicht: randlos). F11 wechselt zwischen Fenster und randlos.
- **Die Wahl des Aussehens** (Entscheidung des Eigentümers 2026-11-09) gibt es auch im klassischen Modus, mit
  denselben Einstellungsschlüsseln wie die Zeilen des erweiterten: *Textures* und *Models* unter den Zeilen der Seite
  Graphics, *Sound theme* unter den zwei Lautstärken der Seite Sound (zuerst die Zeilen des Originals in ihrer
  Reihenfolge, unsere darunter, wie *Fullscreen?*). Die Pfeile gehen durch alle gefundenen Pakete: das eingebaute
  `own`, `original` (nur wenn die Kopie des Spielers da ist), `remastered` und jedes Paket im Datenordner, sodass vom
  Spieler selbst umgewandelte Ressourcen einfach zur Wahl erscheinen ([packs.md](packs.md),
  [sound-themes.md](sound-themes.md)). Die Vorgaben bleiben (unsere Texturen, die Teile des Levels, das
  Standardthema). Nur das Aussehen ändert sich: Regeln und Spielablauf bleiben die des Originals, Replays hängen
  nicht vom Aussehen ab. Die Darstellungskorrekturen (jede Auflösung, Breitbild, Filterung, Glättung) sind immer an
  und nicht umschaltbar.
- **Controls**: Enter auf der Zeile einer Taste wartet auf eine Taste („Press a key"); die gedrückte wird belegt, Esc
  bricht ab. Eine Taste, die eine andere Aktion hat, wird getauscht, so tut keine Taste zwei Dinge. Esc und F11
  gehören dem Spiel und lassen sich nicht belegen; eine belegte Taste geht den Tasten der Mods vor.
  Invert Rotation? dreht die Kamera andersherum. Die Tasten werden als `key.forward ArrowUp` … `key.overview Space`
  und `invert-rotation on|off` gespeichert (die Tastennamen von winit); ohne sie gelten die Tasten aus `DB_Options`
  der Kopie. Jede Shift-Taste tut, was die belegte tut, solange die andere nicht selbst belegt ist. Die Seite zeigt
  eine Taste unter dem Namen der Kopie, sonst unter unserem.
- **Gamepads** (gilrs) drücken dieselben Tasten wie die Tastatur, so ist ein Lauf und sein Replay gleich, womit auch
  immer gespielt wurde: der linke Stick oder das Steuerkreuz rollt die Kugel, die linke und rechte Schultertaste drehen
  die Kamera (Rotation mit links oder rechts), die untere Aktionstaste (A / ✕) ist Overview, Start pausiert. In den
  Menüs bewegt das Steuerkreuz, die untere Taste ist Enter, die rechte (B / ○) Esc. Ihre Tasten lassen sich noch
  nicht umbelegen.
- **Die Maus** bedient jedes Menü, wie im Original: die Schaltfläche unter dem Zeiger wird gewählt (nur wenn er sich
  bewegt, so streitet ein ruhender Zeiger nie mit den Tasten), ein Linksklick drückt sie wie Enter, die rechte Taste
  und die Zurück-Taste der Maus sind Esc. Eine Zeile mit einem Wert (Lautstärke, Auflösung, Sichtfeld, ein
  Ja/Nein-Schalter) geht mit einem Klick auf ihre linke Hälfte herunter und auf ihre rechte hinauf, wie Links und
  Rechts; die Pfeile der Bestenliste wechseln das Level. Das Mausrad scrollt die Liste der Community-Level (und wählt
  die Zeile unter dem Zeiger), sonst geht es durch die Zeilen, in der Bestenliste durch die Level. Ein Klick auf die
  Zeile einer Taste auf der Seite Controls wartet wie Enter auf eine Taste; Maustasten lassen sich nicht belegen. Der
  Zeiger ist über den Menüs sichtbar und im Lauf verborgen, nie im Fenster gefangen; der Lauf nimmt nichts von der
  Maus, so tut ein Klick, der ein Menü schließt, im Level nichts. Die Schaltflächen werden dort getroffen, wo sie
  gezeichnet sind, in jedem Fenster und mit beiden Oberflächen.
- Jeder beendete Lauf wird weiterhin als Replay behalten (auf dem Bildschirm ändert das nichts); der schnellste
  ist die Bestzeit, die der erweiterte Modus zeigt.

## Unser erweiterter Modus

Eingeschaltet in Options oder für einen Start mit `--debug` (ein Debug-Lauf, der außerdem *Alle Level offen* und den
Entwicklungs-Mod god-mode bietet, [mods.md](mods.md), „Gottmodus“). Die Menüs behalten das Aussehen des Originals; es
ändert sich:

- **Hauptmenü**: Start, Highscore, Options, Credits, Exit wie im Original; *Start* fragt zuerst, welche Level.
- **Options**: die Seite *Extended* hat den Schalter des Modus und darunter je einen Schalter für jede seiner
  Ergänzungen (mit dem Modus alle an; die Einstellungen halten die ausgeschalteten als `extended.<key> off`):
  - alle Level offen, ohne Freischalten: nur in einem Debug-Lauf (`--debug`); im Spiel eines Spielers öffnen sich die
    Level durch Spielen;
  - Bestzeiten neben den Leveln;
  - (Start ist seit 2026-11-09 in beiden Modi gleich; den Schalter gibt es nicht mehr);
  - Interface: Drahtseilakt / Ballance / Remaster, woher die Dateien der Oberfläche kommen ([resources.md](resources.md));
  - Aussehen: eine Wahl, die Zeile *Aussehen* der Graphics-Seite (Original / Remaster / Drahtseilakt / Eigenes Paket; siehe „Das Aussehen" unten) statt der Texturpaket-Zeile; das nächste Level lädt damit ([packs.md](packs.md));
  - Models: das Modellpaket, womit die Teile und Kugeln gezeichnet werden (nur ihr Aussehen, nie ihre Physik) — wie im
    Level (jedes Level mit den Teilen, die es nimmt: Vorgabe), Drahtseilakt, Ballance oder ein Paket
    des Spielers ([packs.md](packs.md));
  - Sounds: das Klangthema — System, generiert, Ballance (von der Toolbox gemacht) oder ein eigenes des Spielers
    ([resources.md](resources.md)); das generierte, bis der Spieler ein anderes wählt;
  - Mods: der Bildschirm jedes gefundenen Mods (Werks-Mods und die des Spielers; in einem Debug-Lauf auch god-mode,
    der enthält, was die Übungswerkzeuge und die endlosen Leben waren), jeder mit Links und Rechts geschaltet, Enter
    öffnet seine Seite mit seinen Optionen und Tasten ([mods.md](mods.md)); im klassischen Modus aus. Ein Zeitlimit
    zum Aufheben gibt es nicht: die Punkte zählen nur herunter und bleiben bei null stehen, wie im Original.
- **Graphics** fügt *Field of View* hinzu: die Breite der Spielkamera über 4:3, 40° bis 100° in Schritten von 2
  (`fov 58`; das Original hat 58°, die 1,0123 Radiant von InGameCam). Der klassische Modus zeichnet immer den des
  Originals.
- **Controls** fügt *Defaults* hinzu: zurück zu den Tasten des Originals (Pfeile, Shift, Leertaste), ohne gedrehte
  Kamera.
- **Level-Listen** (eine Liste öffnet sich aus Start, unten): In jedem eine flache Liste seiner Level (ein
  Level ist ein Ordner mit einem Level oder eine einzelne `.nmo`: levels.md, „Ein Level ist ein Ordner“; keine Ordner
  zum Öffnen, nichts anderes gezeigt), gespielt wie ein Original-Level. Jede Zeile zeigt die
  **Schwierigkeit** des Levels, einen Stern und 1 bis 10 (`difficulty` aus level.txt; „★?“, wenn unbekannt), und
  seinen **Titel** auf Englisch (`title` aus level.txt) oder in der Sprache der Menüs, wenn level.txt einen gibt
  (`title.ru`, ...). Die Liste ist nach Schwierigkeit, dann Titel sortiert; Level unbekannter Schwierigkeit nach denen
  bekannter, und ein Level, das sich nur nach seinem Ordner nennt (kein `title`), zuletzt. Ein Titel hat höchstens 40
  Zeichen und wird kleiner gezeichnet oder in der Mitte gekürzt, um in seiner Zeile zu bleiben. Ein Level, das nicht
  geladen werden kann, sagt warum und startet nicht; seine Läufe und Bestzeiten bleiben getrennt von den
  Original-Leveln. Wie man ein Level baut und ablegt: [making-levels.md](making-levels.md).
- Texturpakete und Mods sind hier: die Pakete in den Zeilen Textures und Models ([packs.md](packs.md)), die Mods auf
  dem Bildschirm Mods ([mods.md](mods.md)), jeweils hinter einem eigenen Schalter; Inhalte Dritter werden auf eigenes
  Risiko gespielt.
- **Lange Listen** (Start > Mein Kartenordner, Mods, modifizierte Originale): Esc oder Backspace (die Zurück-Taste des Pads) verlässt jede Zeile; Back bleibt unter der Liste fest, daneben der Hinweis "Esc: zurück". PageUp / PageDown blättern eine Seite, Home / End springen zur ersten / letzten Zeile, gehaltenes Hoch / Runter wiederholt sich (nach 0,35 s, 12 Zeilen pro Sekunde, 30 nach einer weiteren Sekunde), das Rad scrollt, ein getippter Buchstabe springt zum nächsten Titel mit diesem Anfang. *Community-Auswahl* sind die 13 mitgelieferten Karten (mit Autoren); *Mein Kartenordner* zeigt die Karten aus `levels/community/` im Datenordner des Spiels (der Pfad steht unter der Liste).


## Start nach Bedeutung (Entscheidung des Besitzers 2026-11-09)

Start ist im klassischen und im erweiterten Modus gleich; ein leerer oder nicht verfügbarer Punkt entfällt. Jeder Punkt
und Unterpunkt sagt im Hinweisfeld, was er ist und woher er kommt.

| Punkt | Öffnet | Braucht |
|---|---|---|
| *Ausgewählte Karten* (en *Featured maps*) | die 13 vom Eigentümer gewählten Spielerkarten, in unser offenes Format übertragen, mit ihren Autoren | |
| *Klassisches Ballance* (en *Classic Ballance*) | die 13 Original-Level der Kopie des Spielers, freigeschaltet wie im Original | eine Kopie |
| *Ballance-Prüfungen* (en *Ballance Challenges*) | die Original-Level, von der Community umgebaut: Kamera um 45° gedreht, der Weg zurück, die Bahn unsichtbar (`levels/modified/`) | eine Kopie und Level im Ordner |
| *Community-Karten* (en *Community maps*) | ein Untermenü | |
| &nbsp;&nbsp;*Beste* (en *Best*) | die 13 mit dem Spiel gelieferten Community-Karten, mit Autoren und Dank | |
| &nbsp;&nbsp;*Heruntergeladen* (en *Downloaded*) | der Ordner des Spielers `levels/user-levels/` (sein Pfad wird gezeigt): Community-`.nmo`-Karten und Level im offenen Format in einer Liste, samt den früheren `levels/community/` und `levels/open/`, die weiter gelesen werden | |

Die Texte gibt es in allen sieben Sprachen. Reihenfolge von Start seit 2026-11-11: zuerst die ausgewählten Karten, an zweiter Stelle *Klassisches Ballance*.

**Das Aussehen (Look)** (Entscheidung des Eigentümers 2026-11-11, decisions.md, „One Look, and beauty comes through the
toolbox") ist eine Zeile, *Aussehen*, auf der Graphics-Seite (Einstellung `look original|remaster|drahtseilakt|player`,
`textures` hält das Paket, mit dem gezeichnet wird); sie ersetzt die getrennte Textures-Zeile und die Options-Zeile vom
2026-11-09. *Original*: die Texturen der Kopie auf den Formen von 2004. *Remaster*: die von der Toolbox vergrößerten Texturen
auf runden Formen. *Drahtseilakt* (Voreinstellung): unsere Texturen auf runden Formen. *Eigenes Paket*: ein Texturpaket des
Spielers auf runden Formen. Ein Look, der hier nicht gezeigt werden kann, wird nicht angeboten (keine Kopie: kein Original
und kein Remaster; kein Export: kein Remaster; kein eigenes Paket: kein „Eigenes Paket"); eine alte Einstellungsdatei behält
ihre Bedeutung (`look remastered` ist Remaster, sonst entscheidet `textures`). Die runden Formen eines Originallevels sind
sein Export `levels/ballance-remaster/Level_NN` (der Rundungs-Cache), den jeder Look außer Original nutzt; ein Level ohne ihn läuft
wie es ist. Die Hilfe der Zeile und die Notiz einer heruntergeladenen Karte sagen, dass die schönen Looks nach dem Import des
Levels mit `drahtseilakt-toolbox` kommen (das Spiel geht nie online und startet es nicht selbst).

## Wörter, die passen, Listen, die sagen, dass sie scrollen

*2026-11-03, nach den Notizen des Eigentümers („der Text passt nicht überall“, „zwei Zeilen in einer niedrigen
Schaltfläche“, „es ist unklar, dass nur die Hälfte der Mods gezeigt wird“).*

- **Eine Beschriftung bleibt immer auf ihrer Schaltfläche**, in jeder Sprache und jedem Fenster (das Menü ist im
  640×480-Rahmen des Originals angelegt und wird als Ganzes skaliert, so ändern Größe oder Seitenverhältnis nie, was
  passt). Eine zu breite Beschriftung wird kleiner gezeichnet, bis auf drei Viertel ihrer Größe; dann, nur wo die
  Schaltfläche hoch genug für zwei Zeilen in dieser Größe oder mehr ist (die großen Schaltflächen, die Felder), auf
  zwei Zeilen, umbrochen am Leerzeichen nächst der Mitte; sonst noch kleiner, bis 60 %, und was dann noch nicht passt,
  verliert seine Mitte an „..“. Eine niedrige Schaltfläche hält nie zwei Zeilen.
- **Unsere eigenen Bildschirme nutzen eine Größe kleiner** als die kleine Schrift des Originals: die Seite Extended,
  der Bildschirm Mods, die Seite eines Mods, die eigenen Level und die Zeilen der Seite Controls, damit die längsten
  deutschen, russischen und französischen Beschriftungen passen und mehr Zeilen zu sehen sind (13 Level, 10 Mods). Die
  Hilfe unter dem Bildschirm Mods und der Seite eines Mods ist ein Absatz: sie bricht um und wird, wenn sie muss,
  kleiner, um in ihren Kasten zu passen. Die Zeilen dieser Bildschirme werden ohne die Mittelmarke der Tastenleiste
  gezeichnet (das „≡“, das auf der Seite Controls den Namen einer Taste von der Taste trennt; unter einer Beschriftung
  las es sich als Streifen).
- **Jede Liste, die scrollt, sagt es**: ein schmaler Balken neben ihren Zeilen, sein Griff so lang wie der gezeigte
  Teil und dort, wo er ist, und „▲ 3 weitere“ über der ersten Zeile und „▼ 12 weitere“ unter der letzten. Das Mausrad
  scrollt die Liste (die eigenen Level, die Mods, die Seite eines Mods), ein Klick auf den Balken scrollt dorthin.
- Der Test `every_label_fits_its_button_in_every_language_and_window` (menu.rs) zeichnet jeden Bildschirm (die Seite
  jedes Mods, den Bildschirm Mods mit jedem gewählten Mod) in allen sieben Sprachen bei 640×480, 1280×720, 1920×1080,
  3440×1440 und 1080×1920, mit dem Schriftbild der eigenen Oberfläche, und schlägt fehl, wenn eine Beschriftung ihren
  Rahmen oder ihre Schaltfläche verlässt, unter 60 % gezeichnet wird oder unter drei Vierteln ihrer Größe auf zwei
  Zeilen steht.

## Profile

Das Original hat keine (siehe oben). Bei uns würde ein Profil Name, Einstellungen, freigeschaltete Level,
Highscores und Replays bündeln, damit mehrere Personen einen Rechner teilen können. Der klassische Modus braucht
es nicht; er behält wie das Original einen Namen beim Highscore-Eintrag. Offene Frage an den Eigentümer: ob
Profile später als Funktion des erweiterten Modus kommen.

## Hilfe zu jeder Zeile

Jede wählbare Zeile jedes Bildschirms sagt auf dem Notizfeld, was sie ist, wie sie funktioniert und was sie bringt: Hauptmenü, Start mit Untermenüs, Optionen mit allen Seiten (Grafik, Steuerung, Sound, Sprache, Erweitert), Pause-, Tod- und Endmenü sowie die Ja/Nein-Fragen. Eine Zeile mit Wert (Schalter, Auswahl: *Aussehen*, *Texturen*, *Modelle*, *Klänge*, *Oberfläche*, *Vollbild?*, *Wolken?*, *Blickwinkel*, *Erweiterter Modus?*, *Alle Level offen?*, *Bestzeiten?* ...) erklärt auch den jetzt gewählten Wert; der Text ändert sich mit Links und Rechts. Für ein eigenes Textur- oder Modellpaket oder Klangthema des Spielers erscheint die `description` (`description.<Sprache>`, sonst `description.en`) aus seiner `pack.txt` oder `theme.txt`, falls vorhanden; ein Mod zeigt seine eigene Beschreibung. Die Texte stehen in `assets/own/Texts/menu.txt` (`help <Zeile>`, `help <Zeile> <Wert>`) in allen sieben Sprachen; ein Test schlägt fehl, wenn für eine Zeile oder einen Wert in einer Sprache der Text fehlt, und der Alle-Bildschirme-Test geht jede Zeile und jeden Wert durch. Das Feld ist so hoch wie die längste Hilfe seines Bildschirms, damit die Zeilen darüber dem Cursor nicht folgen; für den Platz zeigen die Seiten mit Feld (Grafik, Steuerung, Sound, Erweitert) ihre Zeilen als Reihen, und Hauptmenü, Optionen und Start rücken ihre Knöpfe enger zusammen.

## Reihenfolge der Arbeit

1. Erledigt (2026-11-02): klassisch als Vorgabe; Hauptmenü und Options wie in der Tabelle; die Seite *Extended*.
2. Erledigt: das Pausenmenü mit seinen Yes-/No-Fragen; das Tot-Menü; das Levelende-Menü.
3. Erledigt: Highscores mit Namenseingabe; Freischalten der Level.
4. Erledigt: die Liste Custom levels (levels.md) und die Schalter der Seite Extended.
5. Erledigt: Graphics und Controls; Gamepads.

`DRAHTSEILAKT_SHOT_MENU=base:keys` mit `--screenshot` zeichnet ein Menü über das Bild ohne Fenster (base `main`,
`pause`, `dead` oder `end`; Tasten `u d l r` für die Pfeile, `e` Enter, `b` Esc, `n` die Namenseingabe, `w` die
Taste W, während eine erwartet wird), um jeden
Bildschirm anzusehen. Ein Menü, das nicht über einem Level liegt, steht über der 3D-Menüszene der Oberfläche,
`DRAHTSEILAKT_SHOT_MENU_MS` Millisekunden (Vorgabe 0) in die Runde ihrer Kamera.

**Namen der Aussehen-Werte** (Entscheidung des Eigentümers 2026-11-09): Wo eine Wahl zwischen den Dingen des Originalspiels, unseren und dem Remaster entscheidet, heißen die Werte **Ballance** (die Kopie des Spielers), **Drahtseilakt** (unsere) und **Remaster** (das Remaster der Toolbox), in jeder Sprache (Eigennamen, nicht übersetzt). Nur die angezeigten Namen haben sich geändert: Die Einstellungsschlüssel und Ordnernamen bleiben `own`, `original`, `remastered`, gespeicherte Einstellungen funktionieren also weiter. Ein Paket oder Thema des Spielers behält seinen Namen, angezeigt mit höchstens 16 Buchstaben ([packs.md](packs.md)).
