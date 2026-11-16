# Level bauen und teilen

Was der Autor eines Levels und ein Spieler wissen müssen, um ein Level dorthin zu legen, wo das Spiel es findet, es so
zu benennen, dass das Menü es gut zeigt, und es weiterzugeben. Die Einzelheiten hinter jeder Regel stehen in
[levels.md](levels.md) (die Ordner), im [offenen Format](open-format.md) und in [menu.md](menu.md) (die Liste); die
Entscheidungen des Eigentümers dahinter in [decisions.md](decisions.md) („Ein Level ist ein Ordner“, „Eigenes als
Standard, das Original auf Wunsch des Spielers“).

## Wohin ein Level gehört

Alle Level liegen in `levels/` im Datenordner des Spiels (`~/.local/share/drahtseilakt/` unter Linux,
`%APPDATA%\drahtseilakt\` unter Windows, `~/Library/Application Support/drahtseilakt/` unter macOS). Ein eigenes Level
oder das eines anderen kommt nach `levels/user-levels/`, in zwei Arten:

- Community-Karten (Virtools-`.nmo`-Dateien, das Format des Originals);
- Level im [offenen Format](open-format.md) (gewöhnliche glTF- und PNG-Dateien, leicht von Hand
  fertigzustellen oder nachzubessern).

**Ein Level, ein Ordner, direkt in diesem Ordner:**

*Seit 2026-11-12 ist `levels/user-levels/` der eine empfohlene Ordner (die früheren `downloaded/`, `community/` und `open/` wurden beim ersten Start hineinverschoben, nichts gelöscht).* Im Menü: *Start, Community-Karten, Eigene Level*.

```
levels/user-levels/
  Green Dam/                 der Ordner des Levels: ein schlichter Name, höchstens 40 Zeichen
    Green Dam.NMO            das Level: genau eine .nmo ...
    level.txt                sein Name, Autor, Schwierigkeit (unten; freiwillig, aber ohne Titel steht es zuletzt)
    README.txt               freiwillig: was es ist, wie man es spielt, seine Lizenz
    preview.png              freiwillig: ein Bild davon (auch preview.jpg)
levels/user-levels/
  Sky Road/
    level.gltf level.bin     ... oder ein Level im offenen Format
    textures/                seine eigenen Texturen (PNG), direkt in textures/
    game/                    die Dateien des Spiels, die es mitbringt (seine Teile, Himmel, Klänge: open-format.md)
    level.txt  README.md  preview.jpg
```

**Die Regel:** ein Level zählt nur dann als Level, wenn es in einem eigenen benannten Ordner liegt, der entweder eine `.nmo`-Datei oder ein Level im offenen Format (`level.gltf` ...) enthält, dazu die Beschreibungsdatei `level.txt`. Unterordner zum Gruppieren sind erlaubt (`levels/user-levels/Pro Racing/Season 1/Green Dam/`). Ein Level-Ordner ohne `level.txt` und eine lose `.nmo` direkt in `user-levels/` werden nicht gelistet; das Log des Spiels nennt jeden Fall und sagt, was zu tun ist. (Früher war eine lose `.nmo` ein Level und level.txt optional.)

## level.txt

Einfacher UTF-8-Text, ein `Schlüssel = Wert` je Zeile, `#` beginnt einen Kommentar; unbekannte Schlüssel werden
übergangen. Gelesen werden höchstens 4 KiB und 64 Zeilen; eine größere Datei oder ein Link wird übergangen, als gäbe es
keine.

| Schlüssel | Was | Regeln |
|---|---|---|
| `title` | der Name des Levels, **auf Englisch** | nötig, um unter den benannten Leveln gelistet zu werden; höchstens 40 Zeichen (an einer Wortgrenze gekürzt, „…“) |
| `title.<Code>` | sein Name in einer anderen Sprache: `title.ru`, `title.de`, `title.zh`, `title.fr`, `title.es`, `title.it` | gezeigt, wenn die Menüs diese Sprache sprechen, sonst der englische Titel; ein Code ist 1–8 Kleinbuchstaben; höchstens 16 davon |
| `author` | wer es gemacht hat | höchstens 40 Zeichen |
| `version` | seine Version | höchstens 40 Zeichen |
| `difficulty` | wie schwer es ist | eine ganze Zahl, **1** (leicht) bis **10** (am schwersten); alles andere (0, 11, 3.5, „hard“) gilt als unbekannt |
| `order` | sein Platz in einer Liste der Level eines Autors | eine ganze Zahl; Level gleicher Schwierigkeit mit einer kommen zuerst, in ihrer Reihenfolge |
| `sky` | der Himmel, unter dem die Karte spielt | ein Name der Himmelsboxen des Spiels: `A`, `B`, `C` oder einer unserer (`Mountains`, `Forest`, `Sea`, `Storm`, `Sunset`, `Night`, `Desert`, `Mist`, `Clouds`); er geht dem Himmel der Leveldatei und der Tabelle des Originals vor, die nur mit Kopie und bei schweigender Karte gelesen wird |
| `music` | das Musikthema | eine ganze Zahl von 1 bis 5 |
| `light` | die Farbe des Lichts | drei Zahlen von 0 bis 1, `1 0.9 0.8` |
| `hidden` | `yes`: aus der Liste genommen (ein Level für Tests) | öffnet weiter mit `--open` |
| `source` | wo die Toolbox es gefunden hat | von `import-maps` geschrieben; nicht gezeigt |

Jeder Wert wird bereinigt: Steuerzeichen, Richtungsumschalter und andere unsichtbare Formatzeichen werden entfernt,
mehrere Leerzeichen werden zu einem. Eine level.txt zum Beispiel:

```
title = Paper Winds
title.ru = Бумажные ветра
title.de = Papierwinde
title.zh = 纸之风
author = Drahtseilakt
version = 1
licence = GPL-3.0-only
order = 3
difficulty = 3
```

## Namen

- **Der Name des Ordners** ist schlicht: Buchstaben, Ziffern, Leerzeichen und `-_.()` funktionieren überall; er darf
  keiner der Namen sein, die Windows reserviert (`CON`, `NUL`, `COM1`...), nicht auf einen Punkt oder ein Leerzeichen
  enden und nicht mit einem Punkt beginnen (versteckt). Höchstens 40 Zeichen. Die Toolbox macht jeden Namen, den sie
  schreibt, für Linux, Windows und macOS sicher.
- **Der Titel ist englisch.** Ein Level aus einer anderen Community behält seinen eigenen Namen als `title.<Code>` (eine
  chinesische Karte als `title.zh`), damit jeder Spieler die Liste lesen kann und der Originalname nicht verloren geht.
- Die Version gehört nach Möglichkeit in `version`, nicht in den Titel („Lost City“, `version = 1.1`).

## Was das Menü zeigt, in welcher Reihenfolge

Unter *Start → Community maps* (oder *Open format*) im erweiterten Modus eine flache Liste, eine Zeile je Level: ein
Stern und seine Schwierigkeit (★7; ★?, wenn unbekannt) und sein Titel (in der Sprache der Menüs, wenn level.txt ihn
hat). Die Liste ist sortiert:

1. nach Schwierigkeit, die leichtesten zuerst; Level unbekannter Schwierigkeit nach allen mit einer;
2. bei gleicher Schwierigkeit nach `order`, wo angegeben, dann nach Titel;
3. Level ohne `title` in level.txt (nur nach ihrem Ordner benannt) zuletzt.

Eine lange Liste scrollt, mit einem Balken und „▼ N weitere“. Ein Titel, der für seine Zeile zu lang ist, wird kleiner
gezeichnet oder verliert seine Mitte („...“); er verlässt nie seine Zeile.

## Was übergangen, was beiseitegelegt wird

Das Menü sieht nur die Ordner direkt in `community/` und `open/` an und nur das Level in jedem. Es übergeht, ohne
Schaden: jede andere Datei im Ordner eines Levels, jeden Ordner darin (keine Verschachtelung: eine Sammlung ist ein
Ordner je Level, „45-Degree Rotated - Level 07“), versteckte Namen, Links (nie verfolgt) und einen Ordner ohne Level
oder mit mehreren (mehrere `.nmo` oder eine `.nmo` neben `level.gltf`): ein solcher Ordner ist kein Level und wird auf
der Konsole genannt.

`levels/community-unsorted/` hält, was die Toolbox zum Ansehen von Hand beiseitegelegt hat: gepackte Archive
(`.zip`, `.rar`, `.7z`, `.tar.gz`...), Ordner, die nicht genau ein Level sind, und Karten, die weder im Katalog der
Community stehen noch mit einem Readme kommen. Seine `README.txt` nennt jedes und warum. Das Spiel listet ihn nie;
was man behalten will: entpacken, sortieren und benennen, eine level.txt dazulegen und nach `community/` verschieben.

## Eine Sammlung mit der Toolbox importieren

Die Toolbox (`drahtseilakt-toolbox`, ein eigenes Programm; das Spiel selbst führt sie nie aus) sortiert eine
heruntergeladene Sammlung in diese Form:

```
drahtseilakt-toolbox import-maps ~/Downloads/Maps --catalogue CustomMapList.md
```

- jede `.nmo` unter dem Ordner (Ordner beliebiger Tiefe) wird ein Level-Ordner in `levels/user-levels/` (`--to DIR` für
  einen anderen), mit der Karte, ihrem Bild als Vorschau und einer level.txt, die sagt, woher sie kommt;
- mit `--catalogue` (der Kartenliste der Community, `CustomMapList.md`) bekommt eine Karte, die darin steht, ihren
  englischen Titel, ihren Originalnamen (`title.zh` für eine chinesische), ihren Autor und ihre Schwierigkeit (der
  Katalog bewertet 1–5 in halben Schritten; das wird zu unserem 1–10 verdoppelt);
- eine Karte, die weder im Katalog steht noch ein Readme neben sich hat, gepackte Archive und Ordner, die nicht genau
  ein Level sind, werden nach `levels/community-unsorted/` kopiert (`--unsorted DIR` für einen anderen), mit einer
  README.txt, die sagt, warum; die Quelle wird nur gelesen;
- `--refresh levels/community --catalogue CustomMapList.md [--source DIR]` bringt einen früher importierten Ordner auf
  den neuesten Stand: es schreibt nur die level.txt-Dateien neu (und behält eigene Zeilen, wo der Katalog nichts hat)
  und verschiebt die Ordner, die nicht passen, in den Ordner der unsortierten. Selbst bearbeitete level.txt-Dateien
  vorher sichern.

## Ein Level weitergeben

- **Eigene Ressourcen, eine freie Lizenz.** Ein mit seinen Dateien weitergegebenes Level bringt nur mit, was man selbst
  gemacht hat oder was unter einer freien Lizenz steht (CC0, CC-BY, GPL...), mit der Lizenz in seinem Ordner
  (`LICENSE` oder eine Zeile in level.txt und README) und der Nennung fremder Arbeit.
- **Nie die Dateien des Originals.** Keine Texturen, Klänge, Meshes oder `.nmo`-Teile von Ballance kommen in ein
  weitergegebenes Level. Ein Level, das die Teile oder Texturen des Originals nutzt, nennt sie stattdessen: ein Level im
  offenen Format mit `"parts": "copy"` (die Vorgabe) nimmt seine Teile aus der eigenen Kopie jedes Spielers, eines mit
  `"parts": "own"` aus den eigenen Teilen des Spiels (open-format.md, resources.md); eine `game:`-Textur wird aus dem
  Texturpaket oder der Kopie des Spielers gezeichnet. Die Toolbox macht, was die Kopie eines Spielers liefert; was
  Spieler selbst in ihre Ordner kopieren, ist ihre Sache.
- **Mit dem Spiel ausgelieferte Level** brauchen außerdem die Erlaubnis ihres Autors, ein Replay, das sie schafft, und
  nichts vom Original darin ([levels.md](levels.md), „Was mit dem Spiel kommt“).

## Grenzen

Ein Level kommt von einem Fremden, also liest das Spiel es als feindselig und lehnt ab, was eine Grenze überschreitet,
statt abzustürzen:

| Was | Grenze |
|---|---|
| level.txt | 4 KiB, 64 Zeilen, ein Wert 40 Zeichen, 16 Titel in anderen Sprachen |
| ein Ordner | 10 000 angesehene Einträge in `community/` oder `open/`, 4096 im Ordner eines Levels |
| eine `.nmo` | 256 MiB; jede Struktur darin wird auf ihre Grenzen geprüft und begrenzt (nmo-format.md) |
| ein Level im offenen Format | `level.gltf` 64 MiB, `level.bin` 1 GiB, Bilder nur direkt in `textures/`, Pfade bleiben im Ordner (open-format.md, „Lesen“) |

Ein Level, das das Spiel nicht laden kann, sagt im Menü, warum, und wird nicht gestartet.

## Unser Level-Baukasten: was er prüft

Level des Baukastens werden als Text geschrieben (`level.lvl`, [levels.md](levels.md), „Der Levelbaukasten“) und von
`assets/own/make/leveldsl.py` mit `levelkit.py` gebaut; heute sind das Schaufensterlevel und `Parts_Test` mit ihm gemacht
(die zehn eigenen Level, für die er geschrieben wurde, wurden am 2026-11-16 entfernt). Ein mit dem Baukasten gemachtes Level wird
abgelehnt, mit Angabe der Stellen, wenn es bricht, was ein Level spielbar macht (die Prüfungen auf Platz und ebenen
Boden und der Nahttest kommen mit der Arbeit an Level 1, Zweig `own-level-1`, wo levels.md jede beschreibt):

- **keine Nähte**: ein Stück Boden ist ein Mesh vom Start oder einem Kontrollpunkt bis zum nächsten, sodass die Kugel nie
  auf eine Kante zwischen zwei Stücken trifft;
- **Platz für die Kugel**: nichts von Böden oder Schienen innerhalb von 12 über einem Transformator, einem Ventilator
  oder dem Ballon; ein Weg unter einem anderen Boden oder Schienen hat dessen Unterseite höchstens 3,6 darüber (eine
  Wand, auf die die Kugel trifft) oder 6 und mehr (Platz zum Durchrollen), nie dazwischen, wo eine Kugel sich verkeilen
  würde;
- **ebener Boden unter Teilen**: das Polster eines Transformators, eine Kiste oder eine Kuppel steht auf einem flachen,
  waagerechten Stück Boden an ihrem Fuß;
- **Ränder, die halten**: Geländer, Ziegelränder und Rinnen halten eine gegen sie geworfene Kugel auf der Bahn (ein Test
  wirft jede Kugel mit 12, 25 und 40 Einheiten je Sekunde);
- **ein Start und ein Ziel, ein Titel**, Kontrollpunkte in einer Richtung, die die Kamera einnehmen kann.

Ein Level kann auch von einem eigenen Skript auf dem Baukasten gebaut werden: `assets/own/make/make_synthetic_level.py`
baut das mitgelieferte Schau-Level ([levels.md](levels.md), „Das Schau-Level") aus einer Beschreibung im Skript, mit
eigenen Befehlen (Schilder, ein Tor, Ausstellungsstücke), Materialien, die die Böden des Baukastens nicht nutzen, und
Verhaltensgraphen, die `scriptkit.py` in die `scripts` des Levels schreibt; es gelten dieselben Prüfungen.
