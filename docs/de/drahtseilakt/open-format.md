# Das offene Levelformat (Version 1)

Das eigene Levelformat der Engine: offen, dokumentiert, mit gewöhnlichen Werkzeugen bearbeitbar. Jedes Level,
das das Spiel laden kann — die Originallevel aus der Kopie des Spielers, Community-Karten —, wird auf dem
Rechner des Spielers darin umgewandelt (`drahtseilakt --level N --export DIR`, `--map FILE --export DIR`), und
das Spiel spielt es (`drahtseilakt --open DIR`). Umgewandelte Level entstehen aus den eigenen Daten des
Spielers und werden nicht verbreitet.

## Dateien

Ein Ordner mit:

- `level.gltf` — [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) (JSON);
- `level.bin` — sein Binärpuffer (der einzige, unter diesem Namen);
- `textures/NNNN_name.png` — ein Bild je Textur.

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
- **Materialien**: `baseColorFactor` (Diffuse), `baseColorTexture`, `emissiveFactor`, `doubleSided`;
  `alphaMode` `OPAQUE`, `MASK` (mit `alphaCutoff`) oder `BLEND`; unbeleuchtete Materialien über
  `KHR_materials_unlit`. In `extras.drahtseilakt` steht, wofür glTF kein Feld hat: `ambient`, `specular` und `power` (das Glanzlicht), `z_write`,
  `blend` (Namen von Quell- und Zielfaktor: `zero`, `one`, `src_color`, `inv_src_color`, `src_alpha`,
  `inv_src_alpha`, `dest_alpha`, `inv_dest_alpha`, `dest_color`, `inv_dest_color`), `alpha_cutoff`, wenn
  Blending und Alpha-Test zusammenkommen, und `texgen` (`reflect` oder `chrome`), wenn die
  Texturkoordinaten vom Blick kommen.
- **Szenen-Extras** `extras.drahtseilakt`: `format` (1), `groups` (Name und Indizes der Mitgliedsknoten — die
  Spielregeln lesen sie, siehe [nmo-format.md](nmo-format.md)), `sky` (Buchstabe des Himmels) und `start_ball`.

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
