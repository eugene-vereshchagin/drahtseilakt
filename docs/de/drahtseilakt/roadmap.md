# Fahrplan

Was dieser Fork vorhat, in welcher Reihenfolge und woran man erkennt, dass ein Schritt fertig ist. Die Gründe
für die Entscheidungen stehen in [decisions.md](decisions.md). Wer die Arbeit übernimmt, fängt hier an.

Stand 01.11.2026: Phase 0 ist abgeschlossen (decisions.md, „Alles von Grund auf, in Rust“). Eine erste Nacht
der neuen Engine hat Level 1 spielbar gemacht, der Phasenreihenfolge voraus:

- **Phase 1**: Der Leser liest jede Datei des Spiels und alle ~600 Karten, jedes Szenenobjekt wird dekodiert.
  Ein Mutationstest beschädigt die entpackten Teile der Dateien des Spielers und lässt jeden Decoder darauf laufen
  (crates/level/tests/mutations.rs, mit `cargo test`). Fehlt: cargo-fuzz-Ziele (brauchen eine Nightly-Toolchain und
  libfuzzer, noch nicht installiert), die Bestandsaufnahme der Building Blocks, das Entfernen des C-Codes des
  Upstreams.
- **Phase 2**: Level auf dem Bildschirm mit ihrem Himmel, jede Größe und jedes Seitenverhältnis, Licht im
  Gamma-Raum wie im Original (seine Fixed-Function-Formel: Emissive + Ambient + Diffuse, mal Textur, plus Glanzlicht; die
  Level haben keine eigenen Lichter: das Licht ist Light_Ingame aus Gameplay.nmo, gerichtet, in der Farbe der Leveltabelle, Ambient 0x0F aus base.cmo), 3D-Sprites (die silbernen Kugeln der Extrapunkte), Texturen mit Farbschlüssel, Spiegelungen (TexGen) und
  Materialkanäle (das Blau der Kuppeln), der Schattenfleck der Kugel auf den Böden darunter (TT Simple Shadow). Ein Hauptmenü und ein Levelmenü
  in der eigenen Schrift des Prototyps; eine freie Kamera im Debug-Modus. Die Anzeige im
  Spiel ist die des Originals: Punktekasten und Lebenskugeln, die Punkte in der Schrift des Spiels. Die Menüs sehen
  aus wie im Original (seine Knöpfe, das dunkle Band und die Schriften). Der Bildschirm am Levelende zählt
  die Punkte wie im Original. Dahinter dreht sich die 3D-Szene des Menüs
  (MenuLevel), ihre Kamera fährt ihre Kurve ab. Die Menüs sind die Bildschirme des
  Originals in den Worten der Kopie, klassisch als Vorgabe mit einem erweiterten Modus ([menu.md](menu.md)): Pause-,
  Tot- und Levelende-Menü, Highscores mit Namenseingabe, Freischalten der Level, eigene Level aus einem
  Kartenordner. Die Steinkugel rollt in der Menüszene ihren
  aufgezeichneten Weg. Die Spielkamera folgt wie im Original (zwei Federn, eine Drehung in
  250 ms, das Anheben; am C-Spiel gemessen). Die kurzen Lichter sind die der Skripte: die Blitze des letzten
  Levels mit ihrem Donner und das blaue Licht der neuen Kugel.
- **Phase 3**: Die Kugelphysik ist IVP, aus dem C-Code des Upstreams portiert (crates/ivp) und gegen das C-Spiel
  geprüft: ein gerader Stoß der Holz- und der Steinkugel und eine 17-Sekunden-Route in Level 1 stimmen auf 0,1
  bzw. 0,8 Einheiten überein. Alle Böden, Schienen und Stopper haben das eine Material, das das Original ihnen
  gibt (Reibung 0,7, Elastizität 0,3; Physicalize_Floors aus Levelinit, gleich in der Erweiterung von Level 13);
  sie unterscheiden sich nur in ihrer Kollisionsgruppe.
- **Phase 4**: Regeln aus den Gruppen des Levels (Start, Rücksetzpunkte, Checkpoints, Bonuspunkte und -leben,
  Kugelwandler, Absturz, Ziel); Replays werden aufgezeichnet, das beste je Level behalten. Sektoren, Kuppeln und der
  Levelstart wie im C-Spiel, ebenso die Animation des Kugelwandlers (Ring, Stäbe, Wechsel der Kugel). Die Module der Level 1 bis 12
  bestehen aus ihren physikalischen Teilen und Gelenken wie im C-Spiel, und der Ballon des Ziels trägt die Kugel davon
  wie im C-Spiel. Der Aufwind der
  Ventilatoren hebt die Papierkugel; Schaukeln und Säcke schwingen wie im C-Spiel; eine Steinkugel bricht die
  Plattenbrücke. Ton wie bei Sound_Manager und Music_Manager
  des Originals: Ereignisklänge, Stöße und Rollen der Kugel nach Bodenmaterial, die Musik des Levels. Ventilatoren brummen in ihrer Nähe, das Seil der
  Plattenbrücke reißt hörbar. Das Menü stellt die Lautstärke von Ton und Musik
  ein. Partikel, wie die Blöcke des Originals sie erzeugen: die Flammen am Start und an den Checkpoints, die
  Funken und Spuren der Extrapunkte. der Wind der Ventilatoren, die Funken des Extralebens. Level 13 (die SuDu-Erweiterung von 2004,
  nach ihrem Readme in die Kopie des Spielers installiert) lädt und ist spielbar; das vollständige Grundspiel hat
  dreizehn Level. Im Transformator zerspringt die alte Kugel in ihre Stücke (die
  Explosionsskripte aus Balls.nmo: Körper in der Gruppe der Kugel, nach oben gestoßen, liegen 20 s (Fadeout
  Manager aus Gameplay.nmo) oder bis zum Verlust der Kugel, dann ausgeblendet); vor den Stücken aufgenommene Replays
  (Format 1) laufen ohne sie, vor den 20 s aufgenommene (Format 2) mit Stücken, die bis zum Verlust der Kugel liegen. Die Checkpoints öffnen die Sektoren in der Reihenfolge ihrer Namen
  (PC_TwoFlames_01, _02, ...), wie auch immer die Gruppe des Levels sie aufzählt (Level 11 und 12 zählen sie
  durcheinander auf); früher aufgenommene Replays (Format 3) behalten die Reihenfolge der Gruppe. Die losen
  Objekte (Kisten, Papierkugeln, Kuppeln) entstehen aus dem Objekt ihrer Moduldatei (`P_Dome_MF` aus PH/P_Dome.nmo),
  wie das Original sie macht, nicht aus dem gröberen Platzhalter des Levels; früher aufgenommene Replays (Format 4)
  behalten die Platzhalter. Der Rauch der Kugel pufft, wenn sie erscheint und
  wenn sie zerspringt (BallParticle_Frame script).
- **Phase 6**: offenes Format v1 mit Export und Import; Level 1 übersteht den Hin- und Rückweg unverändert. Ein
  exportiertes Level ist vollständig: es trägt seine Einstellungen (Musik, Licht, Bonus, Tutorial, letztes Level) und
  in seinem Ordner `game/` seine Module, die Kugeln, den Transformator, Himmel, Partikel, Schatten und Klänge, sodass
  es mit nichts aus der Kopie außer der Oberfläche spielt. Die beendeten Läufe des Inhabers in Level 1, 3, 5, 7 und 9
  (tests/replays/) ergeben auf der `.nmo` und auf ihrem Export dieselben Ereignisse (crates/game/tests/open_format.rs),
  auch mit einer Kopie, die nur die Dateien der Oberfläche hat.


## Phase 0 — Ballanced prüfen, bevor ein Backend geschrieben wird

[Ballanced](https://github.com/doyaGu/Ballanced) (von doyaGu, dem Autor von BallanceModLoaderPlus) baut die ganze
Virtools-Laufzeitumgebung aus dem Quellcode nach, Modul für Modul wie das Original: `CK2`, `VxMath`, eine
`RenderEngine` auf SDL_GPU, vollständige Building-Block-Bibliotheken einschließlich `physics_RT`, Plugins,
Eingabe- und Ton-Manager auf SDL3 und den `Player`. MIT (das Repository) und Apache-2.0 (die Building Blocks), also
darf sein Code in diesem GPL-Projekt verwendet werden. Es hat CMake-Presets für Linux, Windows und macOS (x64 und
ARM64) und eine CI, die seine Tests unter Linux ausführt, aber das einzige Release (0.1.0) ist nur für Windows.
Wenn sein Linux-Build funktioniert, ist Phase 1 dort womöglich schon erledigt, und seine vollständigen
Building-Block-Bibliotheken passen besser zu Community-Karten als unser auf das Spiel beschränkter Satz. Unter
Linux hat es für uns noch niemand gestartet.

Auf dem Rechner des Inhabers, mit seiner Kopie des Spiels:

1. Den Standardzweig `sdl` mit `cmake --preset linux-x64-runtime` bauen (Submodule rekursiv; SDL3 aus der
   Distribution oder aus einem festgelegten Release gebaut).
2. Starten und prüfen: Startet es; lässt sich ein Level beenden; fühlt sich die Kugel wie im Original an; was es
   mit Breitbild und hohen Auflösungen macht; ob sich einige Community-Karten laden lassen.
3. In dieser Datei festhalten, was ausgeführt wurde und was zu sehen war.

Dann entscheiden und die Entscheidung in decisions.md festhalten:

- **bleiben** bei OpenBallance und mit Phase 1 wie beschrieben weitermachen;
- **umziehen** mit dem Fork auf Ballanced — der Plan bleibt, Phase 1 schrumpft auf das, was dort fehlt;
- **übernehmen**: bei OpenBallance bleiben und Teile von Ballanced nehmen (Building Blocks für Karten, den
  SDL3-Host).

### Bisherige Ergebnisse (30.10.2026)

Rechner des Inhabers: Debian 13, GCC 14.2, CMake 3.31, Ninja, NVIDIA RTX 3060 (Treiber 550), KDE unter X11.
Ballanced: Branch `sdl`, Commit `ececf94`, Preset `linux-x64-runtime`.

- **SDL3.** Debian 13 liefert 3.2.10; Ballanced verlangt 3.4.8 oder neuer. 3.4.8 aus dem Release-Archiv gebaut,
  GPG-Signatur (Sam Lantinga) geprüft, sha256 `e9fff7467fb60f037e6708da18b25560649e4c63edc2a69bb871b960d9cbfbba`.
- **Build.** Konfiguration und Build sind sauber; eine GCC-Warnung (`-Wfree-nonheap-object` in `XClassArray.h`,
  in `CKSkin.cpp` eingebettet), vermutlich ein Fehlalarm. Das Stage-Verzeichnis hat 25 MB, mit den Spieldaten
  über `BALLANCE_ASSETS_ROOT` 162 MB.
- **Netzwerk.** Die Laufzeit enthält keinen Netzwerkcode. Der Build lädt die Noto-Schriften aus den
  `main`-Branches der notofonts-Repositories: per sha256 geprüft, aber nicht auf einen Commit festgelegt, sodass
  ein Schrift-Update upstream den Build bricht. `VXMATH_FONT_DIR` baut offline aus lokalen Kopien.
- **Start.** SDL_GPU auf Vulkan. Hauptmenü, Level 1 geschafft; die Kugel fühlt sich wie im Original an (Urteil
  des Inhabers). Das Fenster wurde im Betrieb von 1280×720 auf 3840×2069 vergrößert, ohne Probleme; das Bild
  bleibt 4:3 mit Balken. Zwei Sitzungen, 1,5 und 7 Minuten, endeten sauber, ohne Einträge im Log.
- **Breitbild.** `--unlock-widescreen --unlock-high-resolution` fügen nur breite Modi zur Auflösungsliste der
  Spielskripte hinzu; 1920×1080 wird angenommen, aber die Kameras der Level bleiben 4:3, und der Renderer setzt
  sie mit Balken ein. Das HUD wird dagegen über das ganze Fenster verteilt und steht im breiten Fenster neben dem
  Bild, teils auf den schwarzen Balken. Echtes Breitbild gibt es nicht; im Original brauchte es dafür einen BML-Mod.
- **Community-Karten.** Ein Teststand (außerhalb der Repositories) legt eine Karte an die Stelle von Level 1 in
  einer Wegwerfkopie des Spielverzeichnisses, startet Player auf einem eigenen Xvfb-Display mit Software-Vulkan
  (lavapipe), geht per xdotool durchs Menü und nimmt das Spielfenster auf. Fünf Probekarten, 74 KB bis 4 MB,
  luden und liefen ohne Einträge im Log. Der ganze Satz (579 `.nmo` und 19 Archive) steht noch aus.
- **Fehler.** Die Klammer des Lebenszählers im HUD wird weit links gezeichnet und wandert während des Spiels
  (gesehen im maximierten Fenster 3840×2069). Bei 640×480 stimmt das HUD genau mit dem Original überein,
  einschließlich der kleinen linken Klammer; der Fehler gehört also zu Fenstern, die nicht 4:3 sind, und zur
  HUD-Anordnung oben. Upstream nicht gemeldet.
- **Textumbruch.** Der Tutorial-Text bricht anders um als im Original (eine Zeile, die das Original ganz
  druckt, ist zweigeteilt): Glyphenmetriken oder die Breite des Textrahmens weichen ab.
- **Referenz.** Das Originalspiel des Inhabers läuft unter Wine (Wine 10, eigenes Prefix, eigene Kopie des
  Spiels, eingebautes `ddraw`/`d3dimm` statt des mitgelieferten dgVoodoo) auf einem eigenen Xvfb-Display, per
  Skript gesteuert wie oben. Zwei Fallen: Xvfb hat genau einen Videomodus, also muss der Bildschirm dem Modus
  des Spiels entsprechen, sonst fragt das Spiel per Dialog; und Software-OpenGL braucht
  `WINE_LARGE_ADDRESS_AWARE=1`, sonst geht dem 32-Bit-Prozess der Adressraum aus und `base.cmo` lädt nicht.
- `Player --help` ist keine Option: Es wird ignoriert, und das Spiel startet.
- **Tests** (`linux-x64-tests`, 967 Tests, ausgeführt auf Xvfb mit lavapipe). GCC: alle bestehen, aber 36
  CK2-Tests schlagen bei paralleler Ausführung fehl und bestehen nacheinander — sie teilen sich eine Ressource;
  ein Problem der Testisolation, kein Engine-Fehler. Clang 19: baut nur mit `lld` (RenderEngine nutzt LTO, und
  GNU ld kann Clangs Bitcode nicht lesen); zwei Fälle von `RectTransformReferenceTest` scheitern an
  Gleitkomma-Ausnahmeflags (FE_INVALID in ungenutzten SIMD-Lanes), unter GCC bestehen sie. Die explizit
  einzuschaltenden GPU-Tests bestehen bis auf einen Pixeltest mit exaktem Tiefenvergleich auf lavapipe; der
  muss noch auf echter Hardware laufen. GoogleTest wird per Tag geladen, nicht per Commit.
- **Offline-Build.** Mit `VXMATH_FONT_DIR` auf unsere mitgelieferten Schriften (`assets/fonts`) und
  `FETCHCONTENT_FULLY_DISCONNECTED` konfiguriert und baut Ballanced ohne Netzzugang.
- **Originallevel.** Jedes der zwölf, an die Stelle von Level 1 gesetzt, lädt und läuft ohne Einträge im Log
  (nur Laden: durchgespielt hat sie niemand).
- **Noch nicht geprüft:** `--unlock-framerate`, der ganze Kartensatz.

## Phase 1 — Grundlage und Leser für Virtools-Dateien

Ziel: eine saubere Rust-Codebasis, die jede `.nmo` / `.cmo` des Spiels und der Community sicher liest.

- Ein Cargo-Workspace in diesem Repository (`Cargo.toml` im Wurzelverzeichnis, Crates in `crates/`).
  Abhängigkeiten sind in `Cargo.lock` festgelegt und vendort, sodass der Build offline funktioniert; jedes Crate
  wird vor der Aufnahme geprüft (existiert, wird gepflegt, Lizenz mit GPL-3.0 verträglich).
- Der Leser: Dateikopf, komprimierte Teile, Objekttabelle, State Chunks, Managerdaten, eingebettete Dateien.
  Die Eingabe ist feindlich: Jeder Lesezugriff wird auf Grenzen geprüft, Größen, Anzahlen und Verschachtelung
  sind begrenzt, und eine fehlerhafte Datei endet mit einem Fehler samt Offset — nie mit einer Panik. Kein
  `unsafe` im Leser.
- `nmo-dump`, ein Kommandozeilenwerkzeug, das zeigt, was eine Datei enthält: Objekte, Gruppen, Meshes,
  Materialien, Texturen, Verhaltensgraphen.
- Tests: synthetische Dateien, die die Tests selbst bauen; Fuzz-Ziele (cargo-fuzz); lokale Tests auf der Kopie
  des Spielers (`OPENBALLANCE_DATA`), den dreizehn Leveln in `levels/` und der Kartensammlung
  (`OPENBALLANCE_MAPS`).
- **Bestandsaufnahme**: welche Objektklassen, Gruppennamen, Building Blocks und Parametertypen in allen Karten
  vorkommen und wie oft. Sie bestimmt die Reihenfolge der Phasen 4 und 5.
- Der C-Code des Upstreams verlässt den Build (die Historie bewahrt ihn); `docs/` bleibt als Nachschlagewerk.

Fertig, wenn: jede Datei des Spiels und alle ~600 Karten entweder vollständig gelesen werden oder mit einem
klaren Fehler abbrechen; das Fuzzing lief ausreichend lange ohne Befund; die Bestandsaufnahme liegt vor.

## Phase 2 — das erste Level auf dem Bildschirm

- Host für Fenster, Eingabe und Ton; Renderer auf wgpu (Vulkan, Metal, Direct3D 12). Ob das Fenster aus einer
  SDL3-Anbindung oder aus reinen Rust-Crates (winit, gilrs, cpal) kommt, wird zu Beginn entschieden und
  festgehalten.
- Level 1 aus seiner `.nmo` mit den Texturen des Spielers: Meshes, Materialien, Transparenz, Himmel, Licht.
- Jede Auflösung und jedes Seitenverhältnis ab dem ersten Bild, kein 4:3: Die 3D-Ansicht nutzt das echte
  Seitenverhältnis; das 2D-Layout hält 640×480 in 4:3 in der Mitte und verankert Randelemente an den Rändern;
  HiDPI; bis 4K.
- Der feste Simulationstakt von 60 Hz mit interpoliertem Rendern von Anfang an.
- Eine freie Kamera, um durch jedes Level zu fliegen.

Fertig, wenn: sich alle dreizehn Level durchfliegen lassen und wie das Original aussehen (Urteil des
Inhabers, Bilder verglichen mit dem Original unter Wine).

## Phase 3 — die Kugel

- **Referenz**: das Originalspiel unter Wine als Messinstrument — skriptgesteuerte Eingabe, Position und
  Geschwindigkeit der Kugel je Takt aus dem laufenden Spiel gelesen, ohne seine Dateien zu ändern (die Methode
  wird in dieser Phase erarbeitet). Aufgezeichnete Szenarien werden zu Referenzbahnen.
- **Eigene Physik**, von Grund auf: die drei Kugeln auf statischer Geometrie mit Reibung, Dämpfung und
  Schwerkraft wie gemessen; Kisten und bewegte Objekte folgen in Phase 4. Die Kollisionsgeometrie ist die des
  Originals, Polygon für Polygon.
- Kugelsteuerung und Spielkamera wie im Original.

Fertig, wenn: die Referenzszenarien innerhalb der Toleranz bleiben und der Inhaber durch Level 1 rollt und
sagt, dass sich die Kugel richtig anfühlt.

## Phase 4 — das Spiel

- Die Regeln nativ, gesteuert von den Daten des Levels (seinen Gruppenkonventionen): Start, Sektoren,
  Checkpoints und Rücksetzpunkte, Leben, Punkte, Bonuspunkte und -leben, Kugelwandler, physikalische und
  bewegte Objekte, Ziel. Menü, HUD (für jedes Seitenverhältnis angeordnet), Ton, Musik.
- **Replays**: ein Eingabeprotokoll je Takt in einem dokumentierten, versionierten Textformat, das Level per
  Hash bezeichnet. Die Wiedergabe ohne Fenster prüft Checkpoints, Ziel und Punkte. Replays enthalten nur
  unsere Eingaben, liegen daher im Repository (`tests/replays/`) und laufen in der CI.
- Level 13: die SuDu-Erweiterung von 2004, Teil des Grundspiels. Der Spieler installiert sie nach ihrem Readme
  in seine Kopie (sie ersetzt Gameplay, Menu, MenuLevel, Levelinit, Sound und PE_Balloon und fügt Level_13 hinzu).

Fertig, wenn: sich alle dreizehn Level beenden lassen und jedes ein bestehendes Replay hat.

## Phase 5 — Community-Karten

- Ein Kartenordner im Benutzerdatenverzeichnis und eine Liste im Spiel (Titel, Autor, Schwierigkeit aus dem
  Katalog).
- Karten nach den Standardkonventionen laufen über die nativen Regeln aus Phase 4.
- Karten mit eigenen Virtools-Skripten: ein Interpreter für Verhaltensgraphen, Building Blocks in der
  Reihenfolge der Bestandsaufnahme. Eine Karte, die einen fehlenden Block braucht, wird nicht halb gestartet:
  Sie wird mit Name und GUID des Blocks gemeldet und als nicht unterstützt markiert.
- **Spielprofile**: *Klassisch* — nur das Originalspiel (zwölf Level und Level 13, Originalregeln; Replays und
  Rekorde zählen nur hier); *Erweitert* — Inhalte Dritter auf eigenes Risiko mit deutlicher Warnung:
  Community-Karten, Texturpakete, Mods. Mods sind unser eigenes Format aus reinen Daten: Regeln aus einer festen
  Bibliothek von Bausteinen der Engine, von unserer Laufzeit interpretiert, nie Code ([mods.md](mods.md)); BML-Mods
  sind Windows-DLLs und werden nie geladen.
  Die Menüs beider Modi: [menu.md](menu.md).

Fertig, wenn: die meisten Karten spielbar sind; der Bericht führt die übrigen auf, jede mit Grund.

## Phase 6 — offenes Levelformat und Konverter

- Ein offenes, dokumentiertes Levelformat: Geometrie glTF 2.0, Texturen PNG, Klänge Opus, Platzierung und
  Eigenschaften in einer Textdatei. Die Spezifikation liegt in `docs/`.
- Ein Konverter aus `.nmo`, den der Spieler auf seinen eigenen Dateien ausführt. Die Engine spielt beide
  Formate dauerhaft (decisions.md).
- Test: Jedes Originallevel und seine umgewandelte Kopie ergeben mit demselben Replay dieselben Bahnen.

## Phase 7 — Aussehen und Extras

- **Rundes sieht rund aus**: die Kugel analytisch gezeichnet; Pro-Pixel-Licht mit glatten Normalen;
  normalengeführte Tessellierung für Schienen und Rohre. Nur optisch: Die Physik bleibt auf den
  Originalpolygonen. Welche Techniken und wie weit, wird mit dem Inhaber nach dem Ausprobieren entschieden.
  Erster Schritt erledigt: Die Toolbox gibt den gekrümmten Meshes der Remaster-Level eine Gestalt aus
  gekrümmten Punkt-Normalen-Dreiecken und schreibt sie ins Level (smooth.rs der Toolbox; die Level sind flach schattiert, daher gelten Flächen unter 40 Grad zueinander zuerst als eine
  glatte Fläche) und biegen nur Kanten, deren Normalen sie wie einen Bogen biegen, damit Schienen gerade bleiben;
  ein Remaster Level_NN spielt mit den Einstellungen von Level NN (Licht, Musik, Blitze des letzten Levels).
  Zweiter Schritt erledigt: Relief. Die Toolbox macht aus dem Bild jeder beleuchteten Textur eine Normal Map (feine
  Helligkeitsunterschiede als Höhe: Nieten, Fugen zwischen Brettern, Seile), und der Renderer beugt das Licht damit;
  Level ohne sie werden genau wie vorher gezeichnet. Standardmäßig aus (`--relief S`): aus dem Bild geraten, vertiefte
  es jeden Riss und jede Fuge (der Inhaber: „von Säure zerfressen“); echtes Relief braucht Höhe aus mehr als dem Bild.
  Mauerwerk erledigt: wo eine Textur Ziegel in Beton zeigt (die Ziegelbänder der Böden), erkennt die Toolbox die
  Ziegel an der Farbe, nicht an der Helligkeit, hebt sie an und senkt die Fugen, und der Renderer zeichnet diese Höhe
  mit Parallax Occlusion; immer an, Holz, Stein und Metall bleiben unberührt.
- **Lokale Texturverbesserung**: Skripte, die der Spieler auf seiner Kopie ausführt, um Texturen
  hochzuskalieren (bis 4K); das Repository enthält die Pipeline und ihre Parameter, nie die Ergebnisse.
- **Replays als Video**: offline, Bild für Bild, in jeder Auflösung bis 4K, mit Ton. Hochgeladen wird nichts
  ohne Anweisung des Inhabers.

## Kriterien für die Veröffentlichung

*Am 2026-11-16 auf die heutigen Entscheidungen des Eigentümers umgeschrieben* (decisions.md, „Keine eigenen Level“,
„Das Spiel läuft ohne Weiteres“; die Fassung vom 2026-11-03 verlangte zehn eigene Level). Das Spiel erscheint, wenn
alles Folgende gilt. Stand 2026-11-16: **erledigt**, **teilweise**, **offen**.

- **Keine eigenen Level.** Das Spiel liefert die vom Eigentümer ausgewählten Community-Karten aus, ihren Autoren
  zugeschrieben: den Drahtseilakt-Satz (`assets/levels/drahtseilakt/01-…13-…`, ins offene Format konvertiert) und den
  Community-Satz (`assets/levels/community/`, die `.nmo` der Karten), dazu unser synthetisches Schaufensterlevel
  `00-Showcase` und das versteckte Testlevel `Parts_Test`. **Erledigt** (Ordner seit 2026-11-12, levels.md
  „Level, die mit dem Spiel kommen“; die zehn eigenen Level am 2026-11-16 entfernt).
- **Eigene Teile**: Jedes Teil, das ein Level verwenden kann (die drei Kugeln und ihre Bruchstücke, Transformatoren,
  Ventilatoren, Schieber, Zugbrücken, die Kiste, das Steinkugel-Rätsel, Schaukeln und Seilbrücken, Start und Ziel,
  Kontrollpunkte, Extraleben und Extrapunkte, Kuppeln), hat ein eigenes Modell, von einem Skript in
  `assets/own/make` nach den gemessenen Maßen und der Physik des Originals gemacht. **Erledigt**; ihr Aussehen wird
  weiter verbessert (resources.md).
- **Keine Kopie nötig**: Das Spiel startet und spielt ganz ohne Kopie des Originals; der Import der Originallevel
  bleibt über die Toolbox angeboten. **Erledigt.**
- **Läufe des Autopiloten und Videos** sind von den Originalleveln und von den veränderten Sätzen (45°-Kamera,
  unsichtbare Bahn), die Videos für YouTube (4K, jedes mit seinem README). Läufe des Autopiloten auf den mitgelieferten
  Community-Karten sind **keine** Bedingung für die Veröffentlichung (Eigentümer, 2026-11-14). Läufe: **erledigt** für
  die 13 Originallevel (`hints/runs/`), den 45°-Satz und den unsichtbaren Satz (`hints/runs/modified/`); Videos:
  **teilweise** (einzelne Level aufgenommen, die ganze Reihe und der Upload offen).
- **Builds für jede Desktop-Plattform**: Linux, Windows und macOS (BSD willkommen), als Installer und als portabler
  Ordner. **Teilweise**: der portable Linux-Build existiert und wird aktuell gehalten; die Builds für Windows und macOS
  sind in der CI vorbereitet (`.github/workflows/drahtseilakt-release.yml`, Stand 2026-11-18: Windows-Installer und portables Zip, portables Linux-Archiv und `.deb`/`.rpm`, macOS-`.dmg` und portables Archiv, FreeBSD experimentell; Release-Entwurf bei einem Versions-Tag; was man herunterlädt: [distribution.md](distribution.md)) und warten auf den Befehl des Eigentümers, zu taggen und zu veröffentlichen.
- **Die Toolbox wird mit dem Spiel ausgeliefert** (`drahtseilakt-toolbox`, ein eigenes Programm). **Teilweise**: sie steckt in den portablen
  Archiven und den Installern (die CI baut sie neben dem Spiel, ihr Repository hat eigene Releases); die Linux-`.deb`/`.rpm` enthalten nur das Spiel.
- **Ein öffentliches, sauberes Repository**: die Veröffentlichung ist ein neues Repository ohne Historie und Daten des
  privaten (decisions.md, „Das private Repository ist ein Backup“). **Offen.**
- **Eine Lizenzübersicht**: die Lizenz des Spiels und die jedes ausgelieferten Assets (Schriften, Karten, Klänge) an
  einem Ort. **Teilweise**: LICENSE und CREDITS.md gibt es; die Übersicht ist offen.
- **Debug standardmäßig aus**: das Spiel eines Spielers startet ohne den Debug-Lauf; Sitzungsprotokolle werden nur in
  einem Debug-Lauf geschrieben; Gottmodus und die Debug-Schalter gibt es nur in Debug-Builds. **Teilweise**: Protokolle
  folgen dem Debug-Schalter, und der Gottmodus ist ein Entwicklungs-Mod, den ein Release-Paket nicht ausliefert
  (`mods-dev/`); ein Release-Paket wird mit `DRAHTSEILAKT_RELEASE` gebaut (`scripts/make-portable.sh` ohne `--dev`) und startet ohne
  Debug-Lauf; DEV-Builds gibt es nur für Linux, mit Debug-Lauf und Gottmodus. `--debug` funktioniert weiter in einem Release-Binary.
- **Fuzzing des Mod-Formats** (`mod.txt`), wie bei jedem anderen Parser. **Offen** (vom Eigentümer zurückgestellt).
- **Problemmeldungen**: *Problem melden* in den Optionen und in der Anleitung öffnet das Meldeformular im Browser.
  **Erledigt** 2026-11-16 (menu.md, „Die Anleitung“).

## Was Spieler mit Mods ergänzt haben

Eine statische Durchsicht von dreißig BML/BML+-Mods (nur Zeichenketten und Readmes; Mods werden nie
ausgeführt) zeigt, was Spielern fehlte; die vollständige Tabelle, Mod für Mod, und was wir aus jedem gemacht haben,
steht in [mods.md](mods.md). Erledigt (2026-11-03): unser eigenes Mod-Format aus reinen Daten und seine Laufzeit, der
Bildschirm Mods und zehn Werks-Mods darin (klebende Kugel, Physiklabor, dynamisches Sichtfeld, freie Ansicht,
Segment-Timer mit Zwischenzeiten, Info-Einblendung, schneller Neustart, Bildschirmfoto, Geisterspur, Verdunkelung);
von einem Mod geänderte Läufe werden in ihren Replays als gemoddet markiert. Vorgemerkt: der Begleiter (zwei Kugeln),
Steuern mit der gedrehten Ansicht, Täuschkugeln, Statistik der Spielzeit. Ins Grundspiel: Übungswerkzeuge (Start an Punkt und Sektor nach Wahl,
schneller Neustart, ein Geist des besten Laufs je Sektor, Zwischenzeiten und Echtzeituhr); deterministische
Aufnahme und Wiedergabe der Eingaben mit Vorspulen (TAS); eine Info-Einblendung (Position der Kugel, im Sektor
verbleibende Extraleben und -punkte); die freie Kamera und eine Einstellung des Sichtfelds; keine feste Grenze
der Sektoren und Ereignisse für Kartenskripte; Schwerkraft, Zeitfaktor und Spieltempo als Einstellungen für
Tests; Bildschirmfotos. Später oder wahlweise: dynamisches Sichtfeld, Kameradrehung mit der Maus, zwei Kugeln,
neue Kugeltypen und Bauteile, Herausforderungsmodi (Verdunkelung, Täuschkugeln). Die Grenzen des Originals,
die die Mods umgehen (Bildratengrenze, 4:3, niedrige Auflösungen, acht Sektoren, eine Bitmap-Schrift,
anfälliger Kugelwechsel), übernehmen wir nicht; seine verborgenen Konstanten (Schwerkraft -20, Zeitfaktor der
Physik 2) schon.

## Durchgehend

- Die Sicherheitsregeln (`CLAUDE.md`): feindliche Eingaben, festgelegte und vendorte Abhängigkeiten,
  CI-Actions per Commit-SHA festgelegt mit minimalen `permissions:` (die Workflows des Upstreams legen per Tag
  fest und müssen korrigiert oder ersetzt werden).
- Datenfreie Tests laufen überall (Unit, synthetische Dateien, Fuzzing); Tests mit Spieldaten lokal.
- Dokumentation in vier Sprachen. Die nur englische Dokumentation des Upstreams ist Nachschlagematerial und
  wird nicht übersetzt, solange sie nicht unsere wird.
