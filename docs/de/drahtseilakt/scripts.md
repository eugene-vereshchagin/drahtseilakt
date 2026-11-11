# Skripte: die eigenen Verhaltensgraphen der Karten

Community-Karten können eigene Virtools-Verhaltensgraphen tragen („Skripte"): die Flammenpartikel der Vorlage
der Kartenbauer, Portale, die die Kugel versetzen, Objekte, die sich verstecken, bewegen oder drehen, Texte und
Klänge. Das Spiel spielt jede Karte nach den eingebauten Regeln (Start, Kontrollpunkte, Ziel, Module); die
Skripte kommen dazu. Diese Seite ist die Kurzfassung der englischen [scripts.md](../../drahtseilakt/scripts.md),
die die volle Bausteintabelle und die Tabelle je Karte ([scripts-maps.csv](../../drahtseilakt/scripts-maps.csv))
hält.

Phase 1 (2026-11-09): der Kern des Interpreters, die Bausteine der Vorlage und die meistgenutzten übrigen; an
echten Karten ohne Fenster geprüft. Code: `crates/nmo/src/behavior.rs` (die Dateiseite), `crates/script`
(Graphmodell, Laufzeit, Bausteine), `crates/game/src/scripting.rs` (die Seite der Engine). Formatreferenz:
[ck-runtime.md](../../ck-runtime.md).

Phase 2 (2026-11-11): „This“ – das eigene Objekt eines Skripts (ein lokaler Parameter ohne Wert, nur der leere Abschnitt `0x200`) wird als `Owner` gelesen (im offenen Format `["this", 0]`) und beim Start an das Objekt des Skripts gebunden; vorher ging ein solcher Parameter verloren. **Object Copy** macht eine Kopie: ein Objekt des Laufs mit Ort, Mesh und Box des Originals, einem eigenen Namen (der des Originals und eine dreistellige Nummer; die genaue Form im Original ist nicht bekannt) und einer Kopie jedes Skripts des Originals, an die Kopie gebunden; höchstens 256 Kopien je Lauf (auch abgelehnte Versuche zählen) und 16384 Knoten je kopiertem Skript; die Knoten einer Kopie werden aus dem Arbeitsbudget des Ticks bezahlt, eine Kopie, die dort oder in die Grenze des Graphen nicht passt, wird ganz abgelehnt; ab der ersten Kopie hat der Lauf seinen eigenen Graphen, der des Levels bleibt unberührt. Die Kopien werden noch **nicht gezeichnet und haben keinen Körper**. **Box Box Intersection** mit einem Objekt ohne Mesh prüft dessen Ort als leere Box. Damit funktionieren die Teleporte der **Portal**-Karten. Gruppen ändern sich im Lauf (Add To Group, Remove From Group, Group Clear). Verläufe: Linear Progression, Bezier Progression (ihre 2D-Kurve wird nicht gelesen: eine Gerade), Interpolator, Get / Set Component, TT_Timer. Random und Random Switch: ein Generator mit festem Startwert im Zustand des Laufs, so dass ein Lauf und seine Aufzeichnung übereinstimmen. Dazu: Set Euler Orientation, Scale, Get Nearest In Group, Row Search, Remove Row, Object Rename. Nicht getan: Physik per Skript für Objekte des Levels (eine vom Skript bewegte Plattform wird am neuen Ort gezeichnet und stößt am alten), das Zeichnen der Kopien, Attach Script, Object Create, Klänge über die Klang-Engine; der Testspieler (`--autopilot`) kennt die Portale nicht. Eine vor dieser Änderung aufgezeichnete Wiederholung einer Karte mit Skripten kann anders ablaufen.

## Was die Datei hält

Ein Graph ist eine Menge Objekte weniger Klassen, jedes mit seinem Zustandschunk: 8 Behavior (Skript, Graph oder
Baustein mit Prototyp-GUID; Flags, Priorität, Zielparameter, die Listen der Unterverhalten, Links, Operationen,
Ein-/Aus-/Lokalparameter, Ein- und Ausgänge), 6 BehaviorLink (Verzögerung in Frames, Quell- und Ziel-IO),
9 BehaviorIO, 2 ParameterIn (Typ und Quelle), 3 ParameterOut und 45 ParameterLocal (Typ, gespeicherter Wert, bei
Ausgängen die Ziele der Kopie), 4 ParameterOperation (Operations-GUID, zwei Eingänge, Ausgang), 52 DataArray
(Tabellen), und bei jedem CKBeObject seine Skripte (Kennung `0x800`). Pin-Namen sind die Namen der
Parameterobjekte, also lesen Bausteine ihre Pins nach Namen. Was die Datei nicht sagt: welche Skripte die Szene
aktiv startet (alle starten aktiv und zurückgesetzt) und die Objekte des Spiels selbst (`Ball_Pos_Frame`, die
Tabelle `CurrentLevel`), die die Engine beantwortet.

## Das Modell des Interpreters

`script::Graph` sind die Graphen der Datei als Daten; `script::Runtime` ist der Zustand (aktive Verhalten und
IOs, Zähler der Links, Parameterwerte, Tabellenzeilen, Nachrichten) und ein Tick wie ein Frame des
Verhaltensmanagers des Originals: jedes aktive Skript läuft; ein Baustein liest und löscht seine aktiven Eingänge,
zieht seine Pins (ein Operationswert wird zuerst berechnet), wirkt über den `Host`, schreibt seine Ausgänge und
aktiviert seine Ausgangs-IOs; ein Graph feuert die Links von aktiven IOs (ohne Verzögerung sofort, mit
Verzögerung in die Warteschlange), arbeitet den Stapel aktiver Unterverhalten nach Priorität ab und zählt am
Ende die verzögerten Links herunter. Nachrichten erreichen ihre Warter im nächsten Tick. Skripte laufen einmal je
Tick vor der Physik, deterministisch; ein Replay eines Formats vor 8 läuft ohne Skripte, wie es aufgenommen wurde.
Level ohne Skripte bleiben unberührt.

Die Seite der Engine: Matrizen und Sichtbarkeit der Objekte (Überlagerungen, die der Renderer liest), die
Position der Kugel (Set Position auf der Kugel setzt eine neue Kugel in Ruhe dorthin), Hüllquader, Gruppen, die
Tabellen des Spiels (`CurrentLevel`), ein Protokoll der Taten (von `--headless` als `script:`-Zeilen gedruckt).
Was Phase 1 nicht tut: Physik für Objekte per Skript (außer der Kugel), Kopien von Objekten, Texte und Klänge
durch die Engine (bisher Protokoll), Tasten außer denen des Spiels, die 2D-Oberfläche des Originals.

## Feindliche Eingabe

Jede Liste ist gedeckelt, jeder Index geprüft, Rekursion begrenzt; ein unbekannter Baustein wird einmal gemeldet
und seine Eingänge gelöscht, damit keine Karte an ihm hängt. Der Leser gerät bei Dateidaten nie in Panik.

## Wo die Karten stehen

184 Karten mit Skripten (584 Karten mit `community-unsorted/`): 148 nur die Vorlage, 36 eigene Skripte. Bei 173
kennt die Laufzeit jeden Baustein; von den 36 eigenen laufen 8 mit der vollen Wirkung jedes Bausteins (AAAAAA
Class Islands Scenery, Be Besieged on All Sides 2.0, Painstaking Journey, Queen Ball (Easy Ver.), Take the leap,
Traveling on Rubber Walls, Trial of the Ballance King, 反向Level_11), 17 mit einem eingeschränkten Baustein (die
neun Portal-Karten: die Objektkopie fehlt noch, das Versetzen der Kugel läuft; Physik per Skript; Texturen;
Tasten). 11 Karten haben unbekannte Bausteine; die Reihenfolge der Arbeit (Gruppen und Progressionen, Physik per
Skript, Kopien, Klang und Text durch die Engine, Aussehen, übrige Logik, „nie": Maus, Schriften, 2D) steht auf der
englischen Seite.

## Stufe 0 (2026-11-11): Konformität des Kerns

**Das Register der Prototypen** (`crates/script/src/blocks/`): eine Deklaration je Prototyp — GUID, Name,
Bibliothek des Players, Pin-Schema, Flags, Stand, ältere Pin-Layouts als Daten, Laufzeitfunktion und optionale
Ereignisfunktion. Die Module spiegeln die Bibliotheken des Players (`logics.rs` = Logics.dll, `narratives.rs`, ...).

**Nicht im Player.** Acht Bausteinarten aus Karten (Explode, Midi Player, Run VSL, Add/Remove/Select Mesh, Get
Nearest Object, Object Rename `6f074ca7:45427431`) stehen in keiner Bibliothek des Players: im Original tun sie
nichts. Sie sind als **benannte No-ops** deklariert (Entscheidung des Eigentümers 2026-11-11): bekannt, Eingänge
gelöscht, keine Ausgabe, eine Logzeile. Der Object-Rename-Code aus Phase 2 bleibt hinter dem Cargo-Feature
`absent-as-authored` (aus), für einen späteren optionalen Mod „wie der Autor es meinte".
