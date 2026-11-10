# Community-Karten: der Stand

Eine Übersicht über die 579 Community-Karten in `levels/community/` des Inhabers (2026-11-02; die Karten der
[CustomMapList](community-maps.md): Custom Maps 408, Pro Racing 97, Reverse Maps 35, 45-Degree Rotated Maps 13,
Invisible Maps 13, Original Maps 13). Jede wurde ohne Fenster geladen und 300 Ticks gespielt, ins offene Format
exportiert und wieder geladen, und ihre Virtools-Skripte aufgelistet (Bausteine nach Namen). Das ist der Plan für den
Skript-Interpreter und für Mods; **2026-11-09:** Phase 1 des Interpreters ist da ([scripts.md](scripts.md)): der Kern, die Bausteine der
Vorlage und die meistgenutzten übrigen; 173 der 184 Karten mit Skripten (584 Karten mit `community-unsorted/`) haben nur bekannte Bausteine,
8 der 36 mit eigenen Skripten laufen mit der vollen Wirkung jedes Bausteins.

| | Karten |
|---|---|
| laden und spielen nach den eingebauten Regeln | 579 |
| haben die Standardgruppen (Start, Ziel, Checkpoints, Rücksetzpunkte) | 579 |
| exportieren ins offene Format | 579 |
| laden wieder aus dem offenen Format | 578 |
| bringen eigene Virtools-Skripte mit | 188 |

**Die eine, die nicht wieder lädt:** `Custom Maps/Beautiful.NMO` hat ein Objekt, dessen Matrix eine nicht endliche
Zahl enthält; das Spiel spielt die `.nmo`, aber das offene Format lehnt eine solche Matrix ab. Zu tun: der Export
schreibt ein solches Objekt so, wie das Spiel es nutzt (oder lässt es weg), mit einer Warnung.

**Skripte.** 188 Karten bringen eigene Verhaltensgraphen mit, alle aus Custom Maps außer 4 Reverse Maps. 142 davon
bringen dieselben neun Bausteine (Activate Script, Binary Switch, Change Texture Video Format, Deactivate Script,
Get Cell, Object Delete, Op, Point Particle System, TT Scaleable Proximity), weitere 7 dieselben mit Wait Message:
die Skripte, die die Vorlage der Kartenbauer für die Partikel der Flammen mitbringt, die das Spiel selbst zeichnet. Ob
diese Karten ohne sie etwas verlieren, ist durch Spielen zu prüfen; die eingebauten Regeln führen schon jede Karte
durch Start, Checkpoints und Ziel. Die übrigen 39 Karten nutzen eigene Skripte: 107 Bausteine insgesamt, die meisten
in ein oder zwei Karten.

**Bausteine nach Zahl der Karten** (die Reihenfolge für den Interpreter): Op 184, Get Cell 183, Object Delete 183, Activate Script 182, Binary Switch 182, Deactivate Script 182, Point Particle System 182, Change Texture Video Format 180, TT Scaleable Proximity 179, Wait Message 35, Group Iterator 22, Hide 20, Physicalize 19, Box Box Intersection 19, Set Position 18, Iterator 17, Show 16, Switch On Parameter 16, Add Row 15, Delayer 15, PlanarParticleSystem 14, Get Key Row 13, Object Copy 13, Send Message To Group 12, Test 11, Text Display 11, Set Cell 9, Wave Player 9, Nop 8, Restore IC 8; dann 77
Bausteine in jeweils weniger als 8 Karten (darunter physikalische Kräfte und Impulse, Text und Schrift, Tasten und
Maus, Timer und Zähler, Licht, Texturen und Klänge).

**Was das heißt.** Die eingebauten Regeln spielen jede Karte; ein Interpreter für die neun Bausteine der Vorlage und
die nächsten zwanzig deckte ab, was die Karten mit Skripten hinzufügen (bewegte und versteckte Objekte,
Physikänderungen, Kopien, Nachrichten, Text, Klänge). Karten laufen nie halb: eine Karte, die einen nicht umgesetzten
Baustein braucht, wird mit dessen Namen gemeldet.

## Die Prüfung: was eine Karte verlangt und was sie bekommt (2026-11-10)

`drahtseilakt --audit FILE|DIR [--audit-csv FILE]` geht ohne Fenster durch eine Karte oder einen Ordner von Karten,
mit der Kopie des Spielers und ohne sie, und listet für jede Karte: jeden Körper, den die Physik aus einem
Levelobjekt macht und der nicht gezeichnet wird (ein unsichtbares Hindernis, mit Namen, Ort und Grund), die
Platzhalter, für die kein Modul gebaut wird, die Module und Texturen, die niemand hat, die Objekte von Klassen, die
der Lader auslässt, die Alphatests, denen der Renderer nicht folgt, und die unbekannten Skriptblöcke. Die CSV hat
eine Zeile je Karte, Kopie und Art des Problems mit den Namen; die Übersicht ordnet die Arten nach der Zahl der
betroffenen Karten. `--show-colliders` zeichnet die von einer Karte versteckten Böden und Schienen (rot) und ihre
versteckten Stopper (gelb) durchscheinend, im Spiel und in Bildschirmfotos.

Der erste Lauf über 605 Karten (die 26 mitgelieferten und die 579 der Sammlung) fand die Ursachen der
„unsichtbaren Objekte“, und drei Regeln wurden für alle Karten geändert:

| Ursache | Karten | jetzt |
|---|---|---|
| ein Platzhalter wurde an seinem Namen erkannt (`P_Modul_26_01`) und nur als sichtbares Mesh ohne Material; Karten aus Blender nennen ihn `P_Modul_26_01_.001`, geben ihm Materialien oder verstecken ihn, also wurde kein Modul gebaut und der Platzhalter blieb als unsichtbarer oder weißer fester Körper | 71 + 29 + 22 | ein Mitglied der Gruppe eines Moduls ist ein Platzhalter, wie ihn die Levelinitialisierung des Originals nimmt |
| ein Boden, eine Schiene oder ein Stopper ohne Material kollidierte, wurde aber nicht gezeichnet | 42 | einfarbig gezeichnet |
| eine Gruppe, die kein Modul des Spiels ist (`P_Modul_Portal` eines Mods), machte ihre Mitglieder fest | 13 | nie ein Körper; gezeichnet als das Objekt, das es ist |

Was bleibt, nach betroffenen Karten (mit der Kopie): in der Datei versteckte Böden und Schienen, die kollidieren,
204 Karten, und versteckte Stopper, 160 (die eigenen unsichtbaren Kollisionskörper der Karten, wie im Original;
`--show-colliders` zeigt sie); Objekte nicht gelesener Klassen, 58 (2D-Objekte 34, Kurven 16, Klänge 9,
Patch-Meshes 2); Texturen ohne Dateinamen und ohne Pixel, 35; Meshes mit Flächen ohne Material, 30 (grau gezeichnet);
Module von Mods, 17; unbekannte Skriptblöcke, 11; völlig durchsichtige Böden, 6.

### Mit dem Original abgeglichen: versteckte Kollisionskörper und Objektanimationen (2026-11-10)

**Versteckte Böden und Schienen bleiben fest.** Die Levelinitialisierung des Originals physikalisiert jedes Mitglied
von `Phys_Floors` und `Phys_FloorRails`, und Physicalize sieht nicht nach, ob das Objekt gezeigt wird (die
Portierung des Bausteins in der Referenz, `src/bb/bb_physics.c`). Die eigenen Level des Originals bauen darauf: 7
seiner 12 Leveldateien halten dort 19 versteckte Mitglieder (`A02_inv_Coll_00`..`04` in Level 6,
`A07_Floor_Invisible_Col` in Level 11, `A7_invisibleCollCube` in Level 12). Keine Regel wurde geändert. In Go Ahead
stehen die versteckten Wände paarweise längs der Strecke zu beiden Seiten eines Transformers (`Box04` / `Box05`):
ein Gang, der verhindert, dass die Kugel ihn umfährt, keine Wand quer über den Weg; die Karte hat keine Skripte,
also nimmt nichts sie weg.

**Objektanimationen: nichts spielt sie ab.** 88 Karten enthalten 9871 Animationsobjekte (Klassen 15 und 18). Keine
der 605 Karten hat einen Baustein, der eine abspielt (Play Animation 3D Entity, Play Global Animation), oder einen
Parameter, der eine hält, und Virtools spielt von sich aus keine Animation ab. 9287 davon sind das Einbacken eines
Exporters: je 35 Schlüssel für Position, Drehung, Skalierung und Skalierungsachse, jeder gleich dem ersten. 356
haben keine eigenen Schlüssel, 8 sind die Arme des Ballons des Spiels. Im Original bewegt sich nichts, also bewegt
auch das Spiel nichts. Die Prüfung zählt sie getrennt (`animation-unplayed`), nennt eine, deren Schlüssel das
Objekt bewegen würden, und meldet `animation-played` für eine Karte, die etwas hat, womit eine abgespielt wird.
`nmo-dump --anims FILE...` gibt jede Animation mit ihrem Objekt, ihren Controllern und der Reichweite ihrer
Schlüssel aus. Was sich in Community-Karten wirklich bewegt, bewegen ihre Skripte (Set Position, Rotate,
Physicalize, Set Physics Slider; etwa zwanzig Karten): die nächste Phase der Skriptlaufzeit
([scripts.md](scripts.md)).
