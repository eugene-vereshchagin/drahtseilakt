# Community-Karten: der Stand

Eine Übersicht über die 579 Community-Karten in `levels/community/` des Inhabers (2026-11-02; die Karten der
[CustomMapList](community-maps.md): Custom Maps 408, Pro Racing 97, Reverse Maps 35, 45-Degree Rotated Maps 13,
Invisible Maps 13, Original Maps 13). Jede wurde ohne Fenster geladen und 300 Ticks gespielt, ins offene Format
exportiert und wieder geladen, und ihre Virtools-Skripte aufgelistet (Bausteine nach Namen). Das ist der Plan für den
Skript-Interpreter und für Mods; der Interpreter ist nicht begonnen.

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
