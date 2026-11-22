# Das offene Levelformat (Version 1)

Das eigene Levelformat der Engine: offen, dokumentiert, mit gewöhnlichen Werkzeugen bearbeitbar. Jedes Level,
das das Spiel laden kann — die Originallevel aus der Kopie des Spielers, Community-Karten —, wird auf dem
Rechner des Spielers darin umgewandelt (`drahtseilakt --level N --export DIR`, `--map FILE --export DIR`), und
das Spiel spielt es (`drahtseilakt --open DIR`). Umgewandelte Level entstehen aus den eigenen Daten des
Spielers und werden nicht verbreitet. Mit `--textures own` trägt ein Export unsere freien Texturen (resources.md, „Eigene Texturen“) statt
deren der Kopie, wo wir eine haben: so lässt sich das Aussehen eines Levels nur mit freien Ressourcen teilen. Texturen mit demselben Bild werden einmal geschrieben
und von den Materialien geteilt; nicht gefundene Texturen stehen mit Namen in der Zusammenfassung. Mit `--textures
own` entsteht auch kein Ordner `game/` (nichts aus der Kopie: das Level nutzt die eigenen Teile und Klänge des
Spiels); `--export-parts keep|none` wählt es ausdrücklich.

## Dateien

Ein Ordner mit:

- `level.gltf` — [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) (JSON);
- `level.bin` — sein Binärpuffer (der einzige, unter diesem Namen);
- `textures/NNNN_name.png` — ein Bild je Textur; die `uri` eines Bildes muss eine einfache Datei direkt in `textures/`
  nennen (keine weiteren Ordner) oder `game:<name>` sein, sonst wird das Level abgelehnt. `game:<name>` nennt **die
  Textur des Spiels mit diesem Namen** statt einer Datei: keine Datei wird dafür gelesen; das Spiel zeichnet sie mit dem
  gewählten Texturpaket ([packs.md](packs.md)): zuerst das Bild dieses Namens im eigenen `game/Textures/` des Levels,
  dann das Paket und seine Rückfälle, dann unsere, dann die der Kopie (Namen verglichen wie Texturnamen: ohne Groß- und
  Kleinschreibung, `_`, `-`, Leerzeichen und Endung). Der Name hat 1 bis 128 Zeichen, keines davon `/`, `\`, `:` oder
  ein Steuerzeichen, und beginnt nicht mit einem Punkt; er wird nur mit den Namen der Texturen verglichen, nie zu einem
  Pfad. Die Level unseres Baukastens nennen so die Texturen, die sie teilen (die Seiten der Bahn, die Ziegel der Ränder, Holz, Schienen,
  Pfosten, Checkpoint-Platten), sodass ein Texturpaket auch sie ändert, und behalten ihre eigenen besonderen Texturen als
  Dateien. `--export` lässt ein `game:`-Bild wie es ist. Früher gemachte Level funktionieren weiter: ihre Dateien sind
  ihre;
- `level.txt` (freiwillig) — Titel, Autor und Version als Zeilen `Schlüssel = Wert`; ein Level in `levels/` ist ein
  Ordner mit einem Level, nach diesem Titel benannt ([levels.md](levels.md), „Ein Level ist ein Ordner“);
- `game/` — die Dateien des Spiels, die das Level benutzt, nach ihren Pfaden in der Kopie des Spielers, damit es
  ohne die Leveldateien der Kopie spielt: seine Teile im selben offenen Format, jedes ein Ordner mit dem Dateinamen
  ohne `.nmo` (`game/3D Entities/Balls/`, `game/3D Entities/AnimTrafo/`, `game/3D Entities/PH/P_Box/`...: die
  Kugeln, der Transformator und das Modul jedes danach benannten Objekts), und unverändert die Seiten seines Himmels
  (`game/Textures/Sky/`), die Texturen der Partikel (`game/Textures/Particles/`), der Schatten der Kugel und die
  Geräusche eines Laufs (`game/Sounds/`, nicht die des Menüs noch die Musik anderer Level). Das Spiel sucht zuerst
  dort, dann, für ein Level, das für unsere eigenen Teile gemacht ist (seine Einstellungen sagen `"parts": "own"`), in unseren
  Teilen (`assets/own/`, [resources.md](resources.md)) und danach in der Kopie, es spielt also mit und ohne Kopie
  gleich; für jedes andere Level in der Kopie und danach in unseren. Ein Pfad in `game/`, auf dem etwas ein Link ist, findet nichts. `--export` und die
  Toolbox schreiben ihn.

## Komprimierte Texturen

Neben einem großen Bild `textures/NNNN_name.png` kann ein Level `textures/NNNN_name.dtx` tragen: dasselbe Bild fertig
für die Grafikkarte — blockkomprimiert, mit der ganzen Mip-Kette im Voraus. Beim Laden dekodiert das Spiel dann weder
das PNG noch filtert es Mips, und die Textur belegt ein Achtel (BC1) oder ein Viertel (BC7) des Videospeichers. Das
`remaster` der Toolbox schreibt sie (Bilder ab 512 x 512, deren Seiten Vielfache von vier sind); die Leveldatei nennt
sie nicht. Die Teile eines Levels in `game/` tragen ihre ebenso.

Die Datei (Little-Endian), `crates/gamedata/src/gputex.rs`:

| Bytes | was |
|---|---|
| 0–3 | `DSTX` |
| 4–5 | Version, 1 |
| 6 | Format: 1 = BC1 (deckende Farbe, 4 Bit je Pixel), 2 = BC7 (mit Alpha oder eine Normal Map mit Höhe; 8 Bit je Pixel) |
| 7 | Mip-Stufen: immer die ganze Kette bis 1 x 1 |
| 8–11, 12–15 | Breite, Höhe: Vielfache von 4, höchstens 4096 |
| 16–23, 24–31 | die Bilddatei, aus der sie gemacht wurde: Länge und Hash |
| 32–39 | der Hash der folgenden Daten |
| 40– | die Blöcke der Mip-Stufen, die größte zuerst, danach nichts |

Der Hash ist FNV-1a über 8-Byte-Wörter in Little-Endian (das Ende mit Nullen aufgefüllt), dann die Länge. BC1-Blöcke
sind im Vier-Farben-Modus (die erste Farbe die größere) oder einfarbig; BC7-Blöcke sind vom Modus 6. Die Texturen sind
nicht sRGB (das Spiel beleuchtet im Gamma-Raum, wie das Original).

Die Datei ist fremd wie der Rest eines Levels: Der Kopf wird gegen die Grenzen geprüft, bevor etwas gelesen wird, die
Länge muss genau die sein, die der Kopf ergibt, die Daten müssen zu ihrem Hash passen und nur die genannten Blöcke
enthalten, und einem Link wird nie gefolgt. Eine Datei, die eine andere Bildlänge nennt als das PNG daneben, oder
deren Kopf nicht besteht, wird übergangen und das PNG genommen; eine mit beschädigten Daten, oder zwischen Ansehen und Lesen geändert, wird gemeldet und stattdessen ihr PNG gezeichnet, kleiner (das Level neu remastern). Eine Datei, zu der ein Link aus dem Levelordner hinaus führt, wird abgelehnt. Ein Level darf die `.dtx` auch ohne das PNG tragen.

- **Adapter.** BC gibt es auf jeder Desktop-Karte unter Vulkan und Direct3D 12 und auf jedem Mac unter Metal. Wo es
  fehlt (Chips mancher Telefone, manche Software-Adapter), dekodiert das Spiel die Blöcke selbst, ab der Mip-Stufe mit
  höchstens 1024 je Seite. Geprüft unter Linux (Vulkan, NVIDIA) auf beide Arten; Windows und macOS nicht gelaufen.
- **Speicher.** Die eigenen Spieldateien eines Levels (`game/`) teilen ein Budget von 1 GiB Texturspeicher; eine
  Textur nimmt höchstens ein Achtel des Rests — eine größere wird ohne ihre größten Mip-Stufen gezeichnet (ein PNG
  halbiert), nie blank. Ein Bild, das mehrere Teile tragen (dieselben Daten), wird einmal geladen und gezählt.
- **Laden.** Texturen werden zu acht gelesen, jede in ihrem eigenen Thread, und hochgeladen, wie sie kommen. Das Spiel
  gibt aus, wie viele es geladen hat, ihren Videospeicher und die Zeit.

## Inhalt

- **Knoten**: einer je Objekt des Levels, der Reihe nach, mit Name, Weltmatrix `matrix` und, wenn es gezeichnet
  wird, `mesh`. `extras.drahtseilakt.visible` sagt, ob das Objekt angezeigt wird; `extras.drahtseilakt.parent` den
  Index des Knotens, dem es folgt, wenn ein Skript jenen bewegt (Set Parent in einer Karte; die Matrix bleibt die der
  Welt, es ist also kein glTF-Kind; ein Index, der auf keinen anderen Knoten zeigt, lehnt das Level ab, ein Kreis
  wird aufgebrochen). Eine Matrix mit einem nicht endlichen Eintrag (Karten haben so etwas, und das Spiel spielt sie)
  steht vollständig in `extras.drahtseilakt.matrix`, diese Einträge als `"nan"`, `"inf"` oder `"-inf"`; die
  `matrix` von glTF hat dort 0.
- **Meshes**: eines je Mesh mit Vertices; ein Primitiv je Material, alle Primitive eines Meshes teilen sich
  dieselben Accessoren `POSITION`, `NORMAL` und `TEXCOORD_0` (Floats), `indices` als 32-Bit-Ganzzahlen. Ein Mesh
  aus Vertices ohne Flächen (die Punkte, aus denen die Partikel einer Karte kommen) ist ein Primitiv mit `mode` 0
  (Punkte), ohne `indices` und `material`. Das `mode` eines Primitivs ist 4 (Dreiecke) oder 0 (Punkte) oder fehlt (Dreiecke); jeder andere Wert lehnt das Level ab. Die
  Materialkanäle eines Meshes (Durchgänge über das ganze Mesh mit einem anderen Material) stehen in
  `extras.drahtseilakt.channels`: je Kanal `material`, `blend` (Namen von Quell- und Zielfaktor, wie unten) und
  `texcoord` k, der Satz `TEXCOORD_k` mit seinen Koordinaten; höchstens 16. Ein Mesh kann eine feinere Gestalt
  tragen, die an seiner Stelle gezeichnet wird (`extras.drahtseilakt.look`: der Index eines anderen Meshs; so hält
  die Toolbox rundere Formen): solche Meshes stehen nach allen eigenen Meshes des Levels, jedes die Gestalt genau
  eines Meshs, und kein Knoten setzt sie; Physik und Regeln benutzen das Mesh selbst.
- **Materialien**: `baseColorFactor` (Diffuse), `baseColorTexture`, `normalTexture` (eine Normal Map im
  Tangentenraum, Grün nach oben wie in glTF: das Relief, das die Toolbox Remaster-Leveln gibt; sie muss eine der
  Texturen des Levels nennen; mit `extras.drahtseilakt.parallax`, einer Tiefe in Texturkoordinaten über 0 und
  höchstens 0,25, ist ihr Alpha eine Höhe, 1 oben, und der Renderer zeichnet sie mit Parallax, sodass das Relief
  auch bei flachem Blick seine Tiefe behält), `emissiveFactor`, `doubleSided`;
  `alphaMode` `OPAQUE`, `MASK` (mit `alphaCutoff`) oder `BLEND`; unbeleuchtete Materialien über
  `KHR_materials_unlit`. In `extras.drahtseilakt` steht, wofür glTF kein Feld hat: `ambient`, `specular` und `power` (das Glanzlicht), `z_write`,
  `blend` (Namen von Quell- und Zielfaktor: `zero`, `one`, `src_color`, `inv_src_color`, `src_alpha`,
  `inv_src_alpha`, `dest_alpha`, `inv_dest_alpha`, `dest_color`, `inv_dest_color`), `alpha_cutoff`, wenn
  Blending und Alpha-Test zusammenkommen, und `texgen` (`reflect` oder `chrome`), wenn die
  Texturkoordinaten vom Blick kommen.
  **Ein Bild, ein Look (2026-11-16):** `original` sagt, wofür das Material im Original steht — `{"texture": "Floor_Side"}`, wahlweise mit `"texgen"` (die Schienen: `chrome`): mit einem Texturpaket, das nicht unseres ist (der Kopie, des Remasters, ein Paket des Spielers mit den Namen des Originals) und die Textur hat, zeichnet das Spiel das Material damit, wie es die Teile und Kugeln zeichnet, ohne die Normal-Map des Materials und mit dem angegebenen texgen; mit unserem oder einem Paket ohne sie behält das Material sein eigenes Bild. Der Name folgt den Regeln eines `game:`-Namens, sonst wird das Level abgelehnt. `own_only: true` erklärt, dass ein Bild für nichts im Original steht (die Schilder, die Texturen des Community-Teils, die Klebekugel) und in jedem Look unseres bleibt; das Spiel liest es nicht, die Prüfungen verlangen eines von beiden an jedem texturierten Material unserer Levels (levelkit.py `ORIGINAL_OF`, `mark_originals.py` für ein früher gemachtes Level). `models` ändert nie Texturen: allein die Wahl `textures` entscheidet jedes Bild.
- **Szenen-Extras** `extras.drahtseilakt`: `format` (1), `groups` (Name und Indizes der Mitgliedsknoten — die
  Spielregeln lesen sie, siehe [nmo-format.md](nmo-format.md)), `sky` (Buchstabe des Himmels), `start_ball` und
  `settings`: was das Original aus seiner Leveltabelle und seinen Regeln nach Levelnummer nimmt, damit das Level ohne
  die Dateien des Spiels gleich spielt — `music` (Thema 1 bis 1000), `light` (drei Zahlen 0..1), `sector_balls` (eine Liste, Sektor 1 zuerst, aus `wood`, `stone`, `paper`, `sticky` oder null: die Kugel eines mit `--sector` an diesem Sektor begonnenen Laufs; ohne Eintrag gilt die Startkugel des Levels), `level_bonus`
  (Punkte), `tutorial` und `last_level` (true oder false: erstes und letztes Level des Originals) und `source_hash`
  (16 Hexziffern: FNV-1a der `.nmo`, aus der es umgewandelt wurde) sowie `parts` (`"own"`: das Level ist für die
  eigenen Teile des Spiels gemacht und nimmt sie vor denen der Kopie; `"copy"` oder nichts: zuerst die der Kopie; unser
  Levelbaukasten schreibt `"own"`) und `fog` (nur gezeichnet: `color` und `color_down` je drei Zahlen 0..1, `density` 0..1 je Einheit Entfernung, `below` eine Höhe, `depth` eine Tiefe über 0: unsere Level haben ihn, die des Originals keinen; levels.md). Ein `source_hash` eines der Levels 1 bis 12 des Originals setzt auch die Startphase der Physik wie in jenem Level (decisions.md). Ein auf dieser Datei aufgenommenes Replay läuft auf
  dem umgewandelten Level (dasselbe Level, also derselbe Lauf); fehlerhafte Einstellungen lehnen das Level ab. Jedes
  Level kann jeden Hash nennen, ein darauf laufendes Replay beweist also keinen Lauf auf dem Original. Ein Level ohne
  Einstellungen (vor ihnen gemacht) läuft mit den Vorgaben und sagt es; ein Remaster `Level_NN` nimmt sie aus der
  Tabelle der Kopie. Die Dateien in `game/` ersetzen nur die eigenen Teile des Levels, Himmel, Partikel, Schatten und
  die Geräusche eines Laufs: die Dateien der Oberfläche (Menüs, Schrift, Leveltabelle, Menügeräusche) kommen nie aus
  einem Level: sie sind die der Oberfläche (unsere oder die der Kopie), und jede Datei wird mit einer Größengrenze gelesen.
- **Der Dunst** `settings.fog` (die Level des Baukastens), nur gezeichnet: was weit von der Kamera ist, verblasst in der Farbe der
  Luft, `color` zum Horizont hin und `color_down` senkrecht nach unten (drei Zahlen 0..1; ohne `color_down` eine Farbe
  ringsum), `density` davon je Entfernungseinheit (1 - e^(-density × Entfernung), 0..1), und was unter `below` liegt,
  verblasst ebenfalls darin, ganz `depth` darunter. Nur beleuchtete Flächen (Himmel, Sprites und das Wolkenmeer
  behalten ihre Farbe); die Level des Originals haben keinen. `color`, `density`, `below` und `depth` sind nötig; Werte
  außerhalb des Bereichs werden darauf begrenzt, ein fehlerhafter Wert weist das Level ab.
- **Das Wolkenmeer** `settings.clouds` (die Level des Baukastens; levels.md, „Das Wolkenmeer"), nur gezeichnet: `top` (die Höhe
  seiner Kuppen; nötig), `thickness`, `cover` (0..1), `scale` (die Größe einer Wolke), `sun` (die Richtung zur Sonne),
  `sun_color`, `sky` (das Licht von oben), `shadow` (im Schatten; Farben in linearem Licht, 1 weiß), `haze` (Dunst des
  Horizonts je Entfernungseinheit), `under` (0..1: wie viel der Welt darunter der Dunst dort verbirgt) und `wind`
  (x, z je Sekunde). Nur `top` ist nötig; Werte außerhalb des Bereichs werden darauf begrenzt, ein fehlerhafter Wert
  weist das Level ab.

- **Textur-Slots**: `extras.drahtseilakt.slots` eines Bildes nennt die Bilder, die seine weiteren Slots sind, der
  Reihe nach (eine Textur aus mehreren Bildern: die Frames einer Animation, die ein Skript der Karte mit Set Current Slot
  zeigt). Jedes ist ein anderes Bild des Levels; höchstens 63 je Bild und 4096 insgesamt; alles andere lehnt das Level ab.
- **Rahmen** `extras.drahtseilakt.frames` (die 2D-Rahmen einer konvertierten Karte: die Signatur des Autors in einer
  Ecke, ein Logo; nur gezeichnet): jeder mit `name`, `rect` (links, oben, rechts, unten in Pixeln eines Bildschirms von
  640 x 480), `uv` (der gezeigte Teil der Textur), `z_order` (niedriger zuerst gezeichnet), `material` (ein Index oder
  null) und `visible`. Höchstens 64; ein `name` hat höchstens 12288 Bytes (3 x die 4096 eines `.nmo`-Namens, da verlustbehaftetes UTF-8 ihn verdreifachen kann); ein fehlerhafter lehnt das Level ab.
- **Skripte** `extras.drahtseilakt.scripts` (die eigenen Verhaltensgraphen einer konvertierten Community-Karte; `script_json.rs`): `objects` (Name, Klasse und was jedes für die Engine ist: Entität, Gruppe, Mesh, Material, Textur oder Tabelle per Index), `nodes` (Blöcke, Links, IOs, Parameter und Operationen per Objektindex), `arrays` (die Tabellen der Karte), `scripts` (die Graphen der obersten Ebene mit dem Objekt, an dem jeder hängt) und, wenn die Karte sie hat, `groups` (die Mitglieder, die sie jeder Gruppe gibt), `attributes`, `priorities` (Objekt und Priorität: eine höhere führt ihre Skripte zuerst aus), `start_off` (die Skripte und Objekte, die die Szene deaktiviert beginnt) und `waves` (ihre Klänge: Objekt, Datei, Schleife, Lautstärke, Panorama, Tonhöhe). Ein Level, das geschrieben wurde, bevor die letzten drei bewahrt wurden, wird ohne sie gelesen. Nur Daten, von unserer Laufzeit gefahren ([scripts.md](scripts.md)); die Rückverweise werden beim Lesen neu abgeleitet, jeder Index wird geprüft, ein fehlerhafter Eintrag lehnt das Level ab. Ohne das Feld hat das Level keine Skripte (die Flammen der Vorlage werden in jedem Fall aus den Emitter-Frames gezeichnet). Unser Level-Baukasten schreibt sie auch (`assets/own/make/scriptkit.py`: Graphen Block für Block gebaut, Objekte, Materialien und Texturen des Levels beim Namen genannt), so die eigenen Graphen des Schau-Levels ([levels.md](levels.md)).

## Koordinaten

glTF ist rechtshändig, Virtools linkshändig, beide mit Y nach oben: Z wechselt das Vorzeichen (Positionen,
Normalen und Matrizen, deren Z-Zeile und -Spalte das Vorzeichen wechseln), und der Umlaufsinn jedes Dreiecks
wird umgekehrt. Das Negieren ist exakt, also ist ein umgewandeltes und zurückgelesenes Level Bit für Bit
dasselbe: Level 1 und seine offene Kopie ergeben mit derselben Eingabe identische Bahnen und identische
Bilder.

## Lesen

Geteilte Level kommen von Fremden, daher behandelt der Leser einen Ordner als feindlich: `level.gltf` bis
64 MiB, `level.bin` bis 1 GiB, jeder Accessor wird gegen seinen View und den Puffer geprüft, Indizes gegen die
Vertexanzahl, Bildpfade bleiben im Ordner, und alles außerhalb der beschriebenen Teilmenge wird mit einem
Fehler abgelehnt.
