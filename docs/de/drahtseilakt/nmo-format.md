# Das Virtools-Dateiformat (.nmo / .cmo)

Wie `crates/nmo` die Level von Ballance und die Community-Karten liest. Erarbeitet aus den Dateien selbst — den
Dateien des Spiels und rund 600 Community-Karten, die alle vollständig gelesen werden — mit den
Bezeichner-Konstanten aus den Headern des Virtools SDK. Alle Zahlen sind Little-Endian; „Wort“ heißt 32 Bit.

## Container

| Offset | Größe | Feld |
|---|---|---|
| 0 | 8 | Signatur `Nemo Fi\0` |
| 8 | 4 | Prüfsumme |
| 12 | 4 | Virtools-Version (`0x13022002` bei Ballances Virtools 2.1, `0x26052005` bei 3.5) |
| 16 | 4 | Dateiversion (8 in allen gesehenen Dateien; Header 1 gibt es ab 5) |
| 20 | 4 | null |
| 24 | 4 | Schreibmodus (Bit `8`: ganze Datei komprimiert) |
| 28 | 4 | Header 1, gespeicherte Größe |
| 32 | 4 | Daten, gespeicherte Größe |
| 36 | 4 | Daten, entpackte Größe |
| 40 | 4 | Anzahl der Manager |
| 44 | 4 | Anzahl der Objekte |
| 48 | 4 | höchste Objekt-ID |
| 52 | 4 | Produktversion |
| 56 | 4 | Produkt-Build |
| 60 | 4 | Header 1, entpackte Größe |

Dann Header 1, dann die Daten. Ein Teil ist genau dann zlib-komprimiert, wenn gespeicherte und entpackte
Größe verschieden sind. Nach den Daten hängen Dateien von Virtools 3.5 bis zum Ende **eingebettete Dateien**
an: Name (Längenwort + Bytes) und Inhalt (Größenwort + Bytes); meist nur `vtthumbnail.jpg`, manche Karten
bringen auch Klänge und Bilder mit.

**Prüfsumme**: Adler-32, gestartet bei 0 statt 1, über den 64-Byte-Header mit genulltem Prüfsummenfeld, dann
über Header 1 und Daten in gespeicherter Form (ohne eingebettete Dateien). Sie stimmt in allen gesehenen
Dateien; der Leser meldet eine Abweichung, statt abzulehnen, denn ob das Originalspiel sie prüft, ist
unbekannt.

## Header 1: die Objekttabelle

Je Objekt: ID, Klassen-ID, Index in den Daten, Namenslänge, Namensbytes (in der Codepage des Schreibers — in
chinesischen Karten oft GBK). Es folgen Plugin-Abhängigkeiten, die nicht gebraucht werden.

## Daten: Manager und Objekte

Zuerst ein Eintrag je Manager (GUID: 2 Wörter, Größenwort, Chunk), dann einer je Objekt in Tabellenreihenfolge
(Größenwort, Chunk; Größe 0 heißt kein Chunk).

**State Chunk**: ein Versionswort (Datenversion, Klassen-ID, Chunk-Version und Optionsbits, je ein Byte), eine
Wortanzahl, die Wörter, dann optionale Listen je nach Optionsbit (`1` Objekt-IDs, `4` Sub-Chunks, `2`
Manager; jeweils Wortanzahl und Wörter). Option `8` heißt: Objektverweise sind Indizes in die
Objekttabelle; `0xFFFFFFFF` ist ein leerer Verweis.

Innerhalb der Wörter sind Felder unter **Bezeichnern** gruppiert, die eine verkettete Liste bilden:
`[Bezeichner, Index des nächsten Bezeichners (0 am Ende), Felder...]`. Ein String ist eine Bytelänge und die
auf ganze Wörter aufgefüllten Bytes; nach dem abschließenden NUL kann Datenmüll stehen.

## Klassen, aus denen ein Level besteht

| Klasse | ID | Bezeichner | Felder |
|---|---|---|---|
| jedes Objekt | | `0x4` / `0x18` | vorhanden: verborgen / hierarchisch verborgen |
| Gruppe | 23 | `0xFFFFF` | Anzahl, Verweise auf Mitglieder |
| Material | 30 | `0x1000` | Diffuse, Ambient, Specular, Emissive (ARGB-Wörter), Glanzstärke (Float), Texturverweis, Randfarbe, Moduswort, Flagwort |
| | | `0x10000` | Effekt: Parameterverweis, Effekt (1 TexGen, 2 TexGen mit Bezugssystem); der Wert des Parameters ist der TexGen-Typ (CK2_3D.dll: None 0, Transform 1, Reflect 2, Chrome 3, Planar 4, Cube-Maps 31..34) |
| Parameter | 46 | `0x40` | Typ-GUID (2 Wörter), ein Wort, Größe in Bytes, der Wert |
| Textur | 31 | `0x10000` | Anzahl der Slots, Dateiname je Slot |
| | | `0x20000` | eingebettete Pixel: Anzahl der Slots, dann Bits pro Pixel, Breite, Höhe, Masken A R G B, ein Wort, Pixelanzahl, Pixel; `[1, 0]`, wenn die Pixel in einer Datei liegen |
| | | `0x2FF000` | Ladeoptionen: ein Wort (`0x100` eine transparente Farbe, `0x200` ein Videoformat folgt), dann je nach Abschnittsgröße die transparente Farbe (0xRRGGBB), der aktuelle Slot, das Videoformat; Texel dieser Farbe verwirft ein Alphatest |
| Mesh | 32 | `0x100000` | Materialslots: Anzahl, dann (Verweis, Wort) je Slot |
| | | `0x20000` | Vertexanzahl, Speicherflags, Größe (Datenwörter + 1), dann Positionen, Farben, Speculars, Normalen, UVs je nach Flags |
| | | `0x10000` | Flächenanzahl, dann zwei Wörter je Fläche: Indizes 0 und 1 (je 16 Bit), Index 2 und Materialslot |
| | | `0x4000` | Materialkanäle (das Mesh noch einmal mit einem anderen Material über sich gezeichnet): Anzahl, dann je Kanal Materialverweis, Flags (`0x1` aktiv), Quell- und Zielmischung, UV-Anzahl (0 oder die Vertexanzahl) und die UV-Paare |
| 3D-Entity / -Objekt, Kamera, Licht | 33, 41, 34, 35, 38, 39 | `0x100000` | ein Wort, Flags, 3×3-Achsen (zeilenweise), Position — die Weltmatrix |
| 3D-Objekt | 41 | `0x4000` | Verweis auf das aktuelle Mesh, Anzahl, Mesh-Verweise |
| 3D-Sprite | 37 | `0x400000` | Modus (0 zum Blick, 1 dreht um x, 2 um y, 3 wie platziert), halbe Breite, halbe Höhe, Versatz x und y (-1..1 der halben Größe), Texturrechteck u0 v0 u1 v1, Materialverweis; ungeleuchtet in der Diffusfarbe des Materials gezeichnet |
| Datenarray | 52 | `0x1000` | Spalten: Anzahl, dann Name (String) und Typ (1 Int, 2 Float, 3 String, 4 Objekt, 5 Parameter + Typ-GUID) |
| | | `0x2000` | Zeilen: Anzahl, dann ein Wert je Spalte |

**Vertex-Speicherflags**: `0x1` eine Farbe für alle, `0x2` ein Specular für alle, `0x4` keine Normalen (der
Leser berechnet glatte), `0x8` ein UV für alle, `0x10` keine Positionen.

**Moduswort des Materials**, Nibbles vom niedrigsten: Textur-Blendmodus, Min-Filter, Mag-Filter,
Quellfaktor, Zielfaktor (Virtools-Faktoren: 1 zero, 2 one, 3 src colour, 4 inv src colour, 5 src alpha, 6 inv
src alpha, …), Schattierung, Füllmodus. **Flagwort**: Bit `0x1` zweiseitig, `0x2` Z-Schreiben, `0x8`
Alpha-Blending, `0x10` Alpha-Test; Byte 1 Tiefenvergleich, Byte 2 Alpha-Vergleich, Byte 3 Alpha-Referenz.

## Was das Spiel aus Leveln liest

Ein Level enthält Geometrie und **Gruppen**; die Spiellogik liegt in `base.cmo` und arbeitet mit
Gruppennamen: `PS_Levelstart` (Start; seine Z-Achse zeigt zurück zur Kamera), `PR_Resetpoints`
(Kugelmittelpunkte je Sektor), `PC_Checkpoints`, `PE_Levelende`, `Sector_NN`, `Phys_Floors` /
`Phys_FloorRails` / `Phys_FloorStopper` (Kollision), `P_*` (Module und Gegenstände, deren Modelle aus
`3D Entities/PH/*.nmo` kommen), Klanggruppen. Objekte dieser Gruppen, deren Mesh kein Material hat, sind
Platzhalter des Editors und werden nie gezeichnet. Himmel und Startkugel jedes Levels stehen im Datenarray
`AllLevel` von `3D Entities/Levelinit.nmo`.
