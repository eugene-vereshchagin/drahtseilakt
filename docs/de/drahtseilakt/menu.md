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
  - *Fullscreen?* (`fullscreen off|borderless|exclusive`): randlos nimmt den Modus des Desktops, exklusiv ändert ihn,
    wo die Plattform es kann (Wayland nicht: randlos). F11 wechselt zwischen Fenster und randlos.
- **Controls**: Enter auf der Zeile einer Taste wartet auf eine Taste („Press a key"); die gedrückte wird belegt, Esc
  bricht ab. Eine Taste, die eine andere Aktion hat, wird getauscht, so tut keine Taste zwei Dinge. Esc und F11
  gehören dem Spiel und lassen sich nicht belegen; eine belegte Taste geht den Übungstasten des erweiterten Modus vor.
  Invert Rotation? dreht die Kamera andersherum. Die Tasten werden als `key.forward ArrowUp` … `key.overview Space`
  und `invert-rotation on|off` gespeichert (die Tastennamen von winit); ohne sie gelten die Tasten aus `DB_Options`
  der Kopie. Jede Shift-Taste tut, was die belegte tut, solange die andere nicht selbst belegt ist. Die Seite zeigt
  eine Taste unter dem Namen der Kopie, sonst unter unserem.
- **Gamepads** (gilrs) drücken dieselben Tasten wie die Tastatur, so ist ein Lauf und sein Replay gleich, womit auch
  immer gespielt wurde: der linke Stick oder das Steuerkreuz rollt die Kugel, die linke und rechte Schultertaste drehen
  die Kamera (Rotation mit links oder rechts), die untere Aktionstaste (A / ✕) ist Overview, Start pausiert. In den
  Menüs bewegt das Steuerkreuz, die untere Taste ist Enter, die rechte (B / ○) Esc. Ihre Tasten lassen sich noch
  nicht umbelegen.
- Jeder beendete Lauf wird weiterhin als Replay behalten (auf dem Bildschirm ändert das nichts); der schnellste
  ist die Bestzeit, die der erweiterte Modus zeigt.

## Unser erweiterter Modus

Eingeschaltet in Options oder für einen Start mit `--debug`. Die Menüs behalten das Aussehen des Originals; es
ändert sich:

- **Hauptmenü**: Start, Highscore, Options, Credits, Exit wie im Original; *Start* fragt zuerst, welche Level.
- **Options** (die Debug-Schalter, Übungswerkzeuge und endlose Leben, liegen auf ihrem Unterbildschirm *Debug mode*):
  die Seite *Extended* hat den Schalter des Modus und darunter je einen Schalter für jede seiner
  Ergänzungen (mit dem Modus alle an; die Einstellungen halten die ausgeschalteten als `extended.<key> off`):
  - alle Level offen, ohne Freischalten;
  - Bestzeiten neben den Leveln;
  - Level nach Art unter Start;
  - Interface: own / original / remaster, woher die Dateien der Oberfläche kommen ([resources.md](resources.md));
  - Sounds: das Klangthema — System, generiert, original (von der Toolbox gemacht) oder ein eigenes des Spielers
    ([resources.md](resources.md)); das generierte, bis der Spieler ein anderes wählt, und im klassischen Modus
    immer;
  - Übungswerkzeuge: freie Kamera (F2), langsamer und schneller (`[` `]`), den Moment sichern und dorthin zurück
    (F5 / F9), fünf Sekunden zurück (F3, nochmals weiter, bis eine halbe Minute), Sektor und Tick auf dem Bildschirm;
    ein so unterstützter Lauf wird behalten, ist aber nie bester Lauf;
  - endlose Leben: die letzte Kugel geht nie verloren (ein so geretteter Lauf ist nie bester Lauf). Ein Zeitlimit zum
    Aufheben gibt es nicht: die Punkte zählen nur herunter und bleiben bei null stehen, wie im Original.
  Weitere Schalter kommen mit den Funktionen des Fahrplans (Timer, Geist des besten Laufs).
- **Graphics** fügt *Field of View* hinzu: die Breite der Spielkamera über 4:3, 40° bis 100° in Schritten von 2
  (`fov 58`; das Original hat 58°, die 1,0123 Radiant von InGameCam). Der klassische Modus zeichnet immer den des
  Originals.
- **Controls** fügt *Defaults* hinzu: zurück zu den Tasten des Originals (Pfeile, Shift, Leertaste), ohne gedrehte
  Kamera.
- **Level nach Art**: Start bietet *Original* (die klassische Levelliste), *Remastered*, *Community maps* und
  *Open format*, die Ordner von `levels/` ([levels.md](levels.md)). In jedem eine flache Liste seiner Level nach Titel (ein
  Level ist ein Ordner mit einem Level oder eine einzelne `.nmo`: levels.md, „Ein Level ist ein Ordner“; keine Ordner
  zum Öffnen, nichts anderes gezeigt), gespielt wie ein Original-Level. Ein Titel hat höchstens 40 Zeichen und wird
  kleiner oder umbrochen, um auf seiner Schaltfläche zu bleiben. Ein Level, das nicht
  geladen werden kann, sagt warum und startet nicht; seine Läufe und Bestzeiten bleiben getrennt von den
  Original-Leveln. Später: Titel, Autor und Schwierigkeit aus dem Community-Katalog.
- Texturpakete und unsere eigenen Mods kommen ebenfalls hierher, wenn es sie gibt, jeweils hinter einem eigenen
  Schalter und mit dem Hinweis, dass Inhalte Dritter auf eigenes Risiko gespielt werden.

## Profile

Das Original hat keine (siehe oben). Bei uns würde ein Profil Name, Einstellungen, freigeschaltete Level,
Highscores und Replays bündeln, damit mehrere Personen einen Rechner teilen können. Der klassische Modus braucht
es nicht; er behält wie das Original einen Namen beim Highscore-Eintrag. Offene Frage an den Eigentümer: ob
Profile später als Funktion des erweiterten Modus kommen.

## Reihenfolge der Arbeit

1. Erledigt (2026-11-02): klassisch als Vorgabe; Hauptmenü und Options wie in der Tabelle; die Seite *Extended*.
2. Erledigt: das Pausenmenü mit seinen Yes-/No-Fragen; das Tot-Menü; das Levelende-Menü.
3. Erledigt: Highscores mit Namenseingabe; Freischalten der Level.
4. Erledigt: die Liste Custom levels (levels.md) und die Schalter der Seite Extended.
5. Erledigt: Graphics und Controls; Gamepads.

`DRAHTSEILAKT_SHOT_MENU=base:keys` mit `--screenshot` zeichnet ein Menü über das Bild ohne Fenster (base `main`,
`pause`, `dead` oder `end`; Tasten `u d l r` für die Pfeile, `e` Enter, `b` Esc, `n` die Namenseingabe, `w` die
Taste W, während eine erwartet wird), um jeden
Bildschirm anzusehen.
