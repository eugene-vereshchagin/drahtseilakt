# Textur- und Modellpakete

Wie die Level aussehen, kommt aus zwei Arten von **Paketen**, getrennt gewählt, so wie die Klänge aus
[Klangthemen](sound-themes.md) kommen:

- ein **Texturpaket** liefert die Bilder, mit denen die Level, ihre Teile und die Kugeln gezeichnet werden;
- ein **Modellpaket** liefert das Aussehen der Teile und der Kugeln: die Netze, die für eine Kiste, einen Ventilator, die
  Holzkugel gezeichnet werden.

Ein Modellpaket ändert nur, was **gezeichnet** wird. Die Physik nutzt immer die Formen, die das Spiel vermessen hat (die
des Originals für die Level der Kopie, unsere für unsere eigenen Level: [resources.md](resources.md), „Unsere eigenen
Teile“), also ändert ein Paket nie, wie ein Level spielt, und jede Aufzeichnung (Replay) spielt mit jedem Paket gleich.
Diese Seite ist für Paketautoren und Spieler.

## Ein Paket wählen

Options → Graphics (in beiden Modi) hat die Zeilen **Texturen** und **Modelle**, und Options → Extended wiederholt sie; die Pfeile gehen durch alle
gefundenen Pakete, jedes mit seinem Namen in der Sprache des Menüs. Die Wahl gilt ab dem nächsten geladenen Level und
wird in den Einstellungen gespeichert (`textures KEY`, `models KEY` in `settings.txt`). Der klassische Modus hat dieselben Zeilen auf seiner Seite Graphics; Vorgabe sind unsere Texturen und die Teile, die jedes Level nimmt. Für einen Lauf: `--textures KEY` und `--models KEY`.

| Art | Eingebaute Pakete | Voreinstellung |
|---|---|---|
| Texturen | `own` („eigene“: unsere, frei), `original` („Original“: die Kopie des Spielers), `remastered` („Remaster“: die eigenen hochskalierten Bilder eines Remaster-Levels, sonst die der Kopie) | `own` |
| Modelle | `level` („wie im Level“: jedes Level mit den Teilen, die es nimmt), `own` („eigene“: das Aussehen unserer Teile), `original` („Original“: das Aussehen der Teile der Kopie) | `level` |

Mit den Paketen geändert (2026-11-03): `original` zeichnet in einem remasterten Level nun die Bilder der Kopie (mit
ihren transparenten Farben); das Aussehen, das ein remastertes Level vorher mit `original` hatte, seine eigenen
hochskalierten Bilder, ist nun die Wahl `remastered`.

## Wo Pakete liegen

Im Datenordner, neben `sounds/`: `textures/<paket>/` und `models/<paket>/` (`~/.local/share/drahtseilakt/` unter Linux,
`%APPDATA%\drahtseilakt\` unter Windows, `~/Library/Application Support/drahtseilakt/` unter macOS). Ein Paket wird
installiert, indem man seinen Ordner dorthin kopiert; das Spiel sucht Pakete beim Start, also nach dem Hinzufügen neu
starten.

Der Name des Ordners ist der Schlüssel des Pakets (den die Einstellungen speichern): Buchstaben, Ziffern, `_`, `-` und
`.`, nicht mit einem Punkt am Anfang, höchstens 64 Zeichen. Ein Ordner mit dem Namen eines eingebauten Pakets (`own`,
`original`, `remastered`, `level`) oder `none` wird übergangen. Höchstens 32 Pakete jeder Art werden genommen (die
eingebauten zählen mit).

## pack.txt

Dieselbe Form wie die `theme.txt` eines Klangthemas: reiner Text in UTF-8, höchstens 4 KiB, ein `Schlüssel Wert` je Zeile,
`#` beginnt einen Kommentar.

| Schlüssel | Bedeutung |
|---|---|
| `name.en`, `name.ru`, `name.de`, `name.zh` | Der Name des Pakets im Menü je Sprache (höchstens 64 Zeichen); ohne eigenen der englische, sonst der Ordnername. |
| `author`, `licence` | Wer es gemacht hat und unter welcher Lizenz (für Menschen). |
| `fallback` | Das Paket, aus dem kommt, was diesem fehlt: ein Paketschlüssel oder `none`. Ohne Angabe: Texturen `own`, Modelle `level`. |

Die Reihenfolge, in der eine Textur oder ein Teil gesucht wird: das gewählte Paket, sein Ersatzpaket, dessen Ersatzpaket
(höchstens acht, jedes einmal), dann der Standard (Texturen: unsere, dann das eigene Bild des Levels; Modelle: das eigene
Teil des Levels). Ein Paket ersetzt also nur, was es hat.

## Texturpakete

```
textures/
  stone/
    pack.txt
    CREDITS.md             who made each picture, under which licence (recommended)
    Floor_Top_Border.png   a picture for the original's texture Floor_Top_Border
    Wood_Plain.jpg
    Sky/Sky_A_Front.png    the sky's faces (also accepted at the top)
    Particles/Particle_Flames.png
```

- Ein Bild wird nach dem **Namen der Textur** benannt: die Texturnamen des Originals (`assets/own/TEXTURES.md` listet
  alle auf, mit denen die Level zeichnen, was sie sind und ihre Größe), die der Strecke (`Track_Side`,
  `Track_floor_border`, ...) und jede andere Textur, die ein Level nennt. Namen werden ohne Rücksicht auf Groß- und
  Kleinschreibung, `_`, `-` und Leerzeichen und ohne Endung verglichen: `ball_wood.png` ist `Ball_Wood.bmp`.
- PNG, JPEG, BMP oder TGA; höchstens 32 MB je Datei und 8192 Pixel je Seite; höchstens 4096 Bilder je Paket. Alpha zählt
  (die „transparente Farbe“ des Originals wird auf das Bild eines Pakets nicht angewandt).
- Gelesen werden nur gewöhnliche Dateien direkt im Ordner des Pakets, in `Sky/` und in `Particles/`; Verknüpfungen werden
  nie verfolgt. Ein Bild, das sich nicht lesen oder dekodieren lässt, wird zugunsten des nächsten Pakets übergangen. Die
  Bilder eines Levels aus einem Levelordner und aus Paketen teilen sich ein Budget von 256 Millionen Pixeln; ein Bild, das für mehrere Texturen steht, zählt für jede.

Welche Texturen ein Paket ersetzt (dieselbe Regel wie für unsere eigenen Texturen, [resources.md](resources.md), „Eigene
Texturen“): jede Textur der Level, Teile und Kugeln der Kopie und ihrer Remasters; in einer fremden Karte nur die
eigenen Texturen der Kopie, die sie nutzt (nie ein Bild, das die Karte mitbringt); in einem Level im offenen Format nur
die Texturen, die es als die des Spiels nennt (`game:`-Bilder, [open-format.md](open-format.md)), nie die Dateien seines
eigenen Ordners. Ein Bild eines Namens aus dem `game/Textures/` eines Levels kommt vor jedem Paket. Himmel, Partikel und
der Schatten der Kugel werden aus dem Paket eines Spielers genommen, wenn es sie hat, vor den eingebauten Paketen.

### Ein Beispiel: ein Texturpaket

1. Den Ordner `textures/myfloors/` im Datenordner anlegen.
2. `pack.txt` schreiben:
   ```
   name.en My floors
   name.ru Мои полы
   name.de Meine Böden
   author Jane Doe
   licence CC0 1.0
   fallback own
   ```
3. `Floor_Top_Border.png` und `Floor_Side.png` zeichnen (die des Originals sind 256 × 256 und kachelbar; unsere, in
   `assets/own/Textures/`, sind ein guter Anfang: eine kopieren und übermalen).
4. Das Spiel starten, Options → Extended → Texturen: *Meine Böden*, und ein Level laden. Jeder Boden mit Rand ist nun
   deiner; alles andere ist unseres (das Ersatzpaket).

## Modellpakete

```
models/
  round/
    pack.txt
    PH/P_Box/level.gltf     the box's look, a part in the open format
    PH/P_Box/level.bin
    PH/P_Box/textures/...   its own pictures (kept as they are by any texture pack)
    Balls/level.gltf        the balls, their pieces and shadows
    AnimTrafo/level.gltf    the transformer
```

- Aufgebaut wie die eigenen Teile des Spiels (`assets/own/3D Entities/`): ein Teil ist ein Ordner im
  [offenen Levelformat](open-format.md), benannt wie die Datei des Originals ohne `.nmo` (`PH/P_Modul_18/`, `Balls/`).
  Am einfachsten beginnt man, indem man das Teil aus `assets/own/3D Entities/` kopiert und seine Netze ändert.
- Das Netz eines Teils wird statt des Netzes des **Objekts gleichen Namens** in dem Teil gezeichnet, mit dem das Spiel
  spielt (die Namen, die der Code des Spiels nutzt: `P_Box_MF`, `Ball_Wood`, die Stücke `Ball_Wood_Piece_01`...), an den
  Platz dieses Objekts verschoben, sonst statt des Netzes gleichen Namens. Ein Netz des Spielteils, das das Paket nicht
  trifft, wird unverändert gezeichnet. Ein Netz, das sich mehrere Objekte teilen, bekommt ein Aussehen nur, wenn alle
  dasselbe Netz an derselben Stelle treffen.
- Die Materialien und Texturen des Pakets sind seine eigenen: seine `textures/`-Dateien werden nie durch ein Texturpaket
  ersetzt; seine `game:`-Bilder folgen dem Texturpaket.
- Als nicht vertrauenswürdig geprüft (Pakete werden geteilt): nur echte Ordner und Dateien, `level.gltf` höchstens 8 MB
  und `level.bin` höchstens 64 MB, gelesen vom Leser des offenen Formats (jeder Accessor geprüft), höchstens 1.048.576
  Vertices und 2.097.152 Dreiecke je Teil, jede Zahl endlich. Ein Teil, das eine Prüfung nicht besteht, wird ausgelassen
  (einmal im Terminal gemeldet), und das nächste Paket in der Kette liefert es; das Spiel scheitert nie an einem Paket.
- Nur das Aussehen: Kollisionsformen, Massen, Gelenke und jeder andere Wert bleiben die des Spiels.

### Ein Beispiel: ein Modellpaket

1. `assets/own/3D Entities/PH/P_Box/` (aus dem Ordner des Spiels) nach `models/bigbox/PH/P_Box/` im Datenordner kopieren.
2. `models/bigbox/pack.txt` mit `name.en Big box` schreiben.
3. Das Netz der Kiste bearbeiten (Blender importiert und exportiert glTF; die Objektnamen behalten, als glTF Separate
   exportieren, mit den Texturen in `textures/`, dann das `extras.drahtseilakt` der Szene so zurücksetzen, wie die
   Originaldatei es hat).
4. Options → Extended → Modelle: *Big box*. Die Kiste wird als deine gezeichnet; sie rollt, fällt und stößt weiterhin wie
   die des Spiels.

## Lizenzen und Weitergabe

- Eigene Bilder und Netze verwenden oder Werke unter CC0 oder CC-BY; sie in `CREDITS.md` aufführen.
- **Niemals Texturen oder Modelle des Originalspiels** (oder eines anderen Spiels) aufnehmen, auch nicht bearbeitet. Die
  eingebauten `original`-Pakete nehmen sie aus der eigenen Kopie jedes Spielers, auf dessen Rechner.

## Referenz

- Code: `crates/gamedata/src/packs.rs` (Pakete finden und wählen, pack.txt), `crates/gamedata/src/assets.rs`
  (`put_pack_textures`, `game_texture`, `put_pack_looks`, `load_pack_part`).
- Gefuzzt: `pack_txt` ([fuzzing.md](fuzzing.md)); das Teil eines Modellpakets läuft durch den Leser des offenen Formats
  (`open_level`).
