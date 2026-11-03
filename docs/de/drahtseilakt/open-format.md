# Das offene Levelformat (Version 1)

Das eigene Levelformat der Engine: offen, dokumentiert, mit gewöhnlichen Werkzeugen bearbeitbar. Jedes Level,
das das Spiel laden kann — die Originallevel aus der Kopie des Spielers, Community-Karten —, wird auf dem
Rechner des Spielers darin umgewandelt (`drahtseilakt --level N --export DIR`, `--map FILE --export DIR`), und
das Spiel spielt es (`drahtseilakt --open DIR`). Umgewandelte Level entstehen aus den eigenen Daten des
Spielers und werden nicht verbreitet. Mit `--textures own` trägt ein Export unsere freien Texturen (resources.md, „Eigene Texturen“) statt
deren der Kopie, wo wir eine haben: so lässt sich das Aussehen eines Levels nur mit freien Ressourcen teilen.

## Dateien

Ein Ordner mit:

- `level.gltf` — [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) (JSON);
- `level.bin` — sein Binärpuffer (der einzige, unter diesem Namen);
- `textures/NNNN_name.png` — ein Bild je Textur; die `uri` eines Bildes muss eine einfache Datei direkt in `textures/`
  nennen (keine weiteren Ordner), sonst wird das Level abgelehnt;
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

## Inhalt

- **Knoten**: einer je Objekt des Levels, der Reihe nach, mit Name, Weltmatrix `matrix` und, wenn es gezeichnet
  wird, `mesh`. `extras.drahtseilakt.visible` sagt, ob das Objekt angezeigt wird.
- **Meshes**: eines je Mesh mit Geometrie; ein Primitiv je Material, alle Primitive eines Meshes teilen sich
  dieselben Accessoren `POSITION`, `NORMAL` und `TEXCOORD_0` (Floats), `indices` als 32-Bit-Ganzzahlen. Die
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
- **Szenen-Extras** `extras.drahtseilakt`: `format` (1), `groups` (Name und Indizes der Mitgliedsknoten — die
  Spielregeln lesen sie, siehe [nmo-format.md](nmo-format.md)), `sky` (Buchstabe des Himmels), `start_ball` und
  `settings`: was das Original aus seiner Leveltabelle und seinen Regeln nach Levelnummer nimmt, damit das Level ohne
  die Dateien des Spiels gleich spielt — `music` (Thema 1 bis 1000), `light` (drei Zahlen 0..1), `level_bonus`
  (Punkte), `tutorial` und `last_level` (true oder false: erstes und letztes Level des Originals) und `source_hash`
  (16 Hexziffern: FNV-1a der `.nmo`, aus der es umgewandelt wurde) sowie `parts` (`"own"`: das Level ist für die
  eigenen Teile des Spiels gemacht und nimmt sie vor denen der Kopie; `"copy"` oder nichts: zuerst die der Kopie; unser
  Levelbaukasten schreibt `"own"`) und `fog` (nur gezeichnet: `color` und `color_down` je drei Zahlen 0..1, `density` 0..1 je Einheit Entfernung, `below` eine Höhe, `depth` eine Tiefe über 0: unsere Level haben ihn, die des Originals keinen; levels.md). Ein auf dieser Datei aufgenommenes Replay läuft auf
  dem umgewandelten Level (dasselbe Level, also derselbe Lauf); fehlerhafte Einstellungen lehnen das Level ab. Jedes
  Level kann jeden Hash nennen, ein darauf laufendes Replay beweist also keinen Lauf auf dem Original. Ein Level ohne
  Einstellungen (vor ihnen gemacht) läuft mit den Vorgaben und sagt es; ein Remaster `Level_NN` nimmt sie aus der
  Tabelle der Kopie. Die Dateien in `game/` ersetzen nur die eigenen Teile des Levels, Himmel, Partikel, Schatten und
  die Geräusche eines Laufs: die Dateien der Oberfläche (Menüs, Schrift, Leveltabelle, Menügeräusche) kommen nie aus
  einem Level: sie sind die der Oberfläche (unsere oder die der Kopie), und jede Datei wird mit einer Größengrenze gelesen.

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
