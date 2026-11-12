# Level und Ordner

## Was mitgeliefert wird

Zwei Kartensätze stecken im Spiel, Sie können also sofort losspielen:

- **Ausgewählte Karten**: 13 von uns gewählte Karten von Community-Autoren, mit abgerundeten Formen und großen Texturen.
- **Community-Karten, Beste**: 13 weitere Community-Karten in den Dateien, wie ihre Autoren sie gemacht haben, mit
  Namensnennung.

Alle gehören ihren Autoren und werden mit Dank gezeigt. Bittet ein Autor darum, entfernen wir seine Karte.

## Ihr Datenordner

Einstellungen, Ergebnisse, Wiederholungen und Ihre eigenen Level liegen in einem Datenordner:

| System | Datenordner |
|---|---|
| Linux | `~/.local/share/drahtseilakt/` |
| Windows | `%APPDATA%\drahtseilakt\` |
| macOS | `~/Library/Application Support/drahtseilakt/` |
| Portable Kopie | der Ordner `data/` neben dem Programm (dort liegt eine Datei `portable.txt`, und nichts außerhalb des Ordners wird benutzt) |

Darin enthält der Ordner `levels/` diese Ordner:

| Ordner | Inhalt |
|---|---|
| `ballance-original/` | Ihre Kopie des Originalspiels, von der Toolbox dort abgelegt |
| `ballance-remaster/` | die Originallevel in 4K neu gemacht, erzeugt von `drahtseilakt-toolbox` |
| `user-levels/` | Ihre eigenen und heruntergeladene Karten |
| `modified/` | Umbauten der Originallevel durch die Community (gedreht, rückwärts, unsichtbar); sie brauchen Ihre Kopie |

## Die Regel: Ein Level ist ein Ordner

Ein Level ist **nur dann** ein Level, wenn es in einem eigenen, benannten Ordner liegt, der **eine `.nmo`-Datei** (oder
ein Level im offenen Format) **und eine Beschreibungsdatei `level.txt`** enthält. Eine lose `.nmo`-Datei im Ordner oder
ein Ordner ohne `level.txt` wird in der Liste nicht angezeigt.

```
user-levels/
  Meine Karte/
    Meine Karte.nmo
    level.txt
    preview.png        (Bild, freiwillig)
  Staffel 1/           (ein Sammelordner: er enthält selbst kein Level)
    Andere Karte/
      Andere Karte.nmo
      level.txt
```

Ein Ordner ohne eigenes Level gilt als Sammelordner, das Spiel schaut hinein. Legen Sie nicht zwei Level in einen Ordner.

### level.txt

Eine einfache Textdatei mit Zeilen der Form `Schlüssel = Wert`. Eine Zeile, die mit `#` beginnt, ist ein Kommentar.

```
# Meine erste Karte
title = My Map
title.de = Meine Karte
author = Ihr Name
version = 1.0
difficulty = 4
```

| Schlüssel | Bedeutung |
|---|---|
| `title` | der Name in der Liste (englisch; höchstens 40 Zeichen werden gezeigt) |
| `title.de`, `title.ru`, `title.zh` ... | der Name in einer anderen Sprache, gezeigt, wenn die Menüs sie sprechen |
| `author` | wer die Karte gemacht hat |
| `version` | die Version der Karte |
| `difficulty` | Schwierigkeit, eine ganze Zahl von 1 bis 10; danach wird die Liste sortiert |
| `order` | eine Zahl, die das Level weiter nach vorn setzt |

Wirklich nötig ist nur `title`; die Datei darf bis zu 4 KB groß sein.

### „Einträge im Levelordner sind fehlerhaft“

In der Liste Ihrer Level kann das Spiel melden, dass einige Einträge fehlerhaft sind. Dann hat es einen Ordner oder
eine Datei gefunden, die der Regel oben nicht folgt: eine lose `.nmo`, einen Ordner ohne `level.txt` oder einen Ordner
mit mehreren Leveln. Zur Behebung: Jede Karte in einen eigenen Ordner legen, eine `level.txt` mit mindestens einer
`title`-Zeile hinzufügen und die Liste erneut öffnen. Das Protokoll des Spiels nennt jeden übergangenen Eintrag.

Am einfachsten gelingt das mit [der Toolbox](toolbox.md): Ihr *Karten-Import* sortiert eine ganze heruntergeladene
Sammlung in diese Form.
