# Fahrplan

Was dieser Fork vorhat, in welcher Reihenfolge und woran man erkennt, dass ein Schritt fertig ist. Die Gründe
für die Entscheidungen stehen in [decisions.md](decisions.md). Wer die Arbeit übernimmt, fängt hier an.

Stand 30.10.2026: Der Fork ist das OpenBallance 0.1.3 des Upstreams plus Projektregeln (`CLAUDE.md`) und diese
Dokumente. Am Engine-Code wurde noch nichts geändert.

## Phase 1 — nativer Backend

Ziel: eine ganz normale native ausführbare Datei (Linux, Windows, macOS), die ein Fenster öffnet und das Spiel
spielt, ohne gasm.

- Ein neuer Backend, der `src/platform.h` mit **SDL3** umsetzt: Fenster, Tastatur (der Kern erwartet
  DirectInput-Tastencodes, `plat_dik_keys`), Maus, Gamepads, Tonausgabe, Benutzerdateien (Spielstände,
  Einstellungen) im üblichen Benutzerdatenverzeichnis der Plattform, Startparameter von der Kommandozeile.
- Eine neue Umsetzung von `src/render/gpu.h` auf **wgpu-native**. `gpu.h` ist bereits wie WebGPU aufgebaut,
  mit WGSL-Shadern und JSON-Deskriptoren, also ist das eine Übersetzungsschicht und kein neuer Renderer.
  Den genauen Vertrag zeigen `gpu.h` und die `gpu_*`-Funktionen in `platform_gasm.c`.
- **Simulation mit festem 60-Hz-Takt.** `src/app.c` ruft `ck_process(&ctx, 1000.0f / 60)` einmal pro
  gezeichnetem Bild auf. Das stimmt nur, weil gasm das Spiel genau 60-mal pro Sekunde aufruft. Nativ muss die
  Simulation aus einem Zeitakkumulator mit festen 60 Hz takten, egal was der Bildschirm macht; der Renderer
  zeichnet mit der Bildwiederholrate des Bildschirms und interpoliert die Lage der Objekte zwischen den beiden
  letzten Takten. Logik und Physik bekommen nie einen variablen Schritt: Die Physik und jede Community-Karte
  sind auf 60 Hz abgestimmt.
- CMake: ein Plattformziel `native`, das die ausführbare Datei des Spiels baut, neben dem vorhandenen Ziel
  `gasm` und der datenfreien Bibliothek `ob_core`. Abhängigkeiten in exakten, festgelegten Versionen; alles
  Heruntergeladene wird gegen eine Prüfsumme geprüft. In Release-Builds die Härtungs-Flags aus den
  secure-codegen-Regeln für C, in Debug-Builds ASan + UBSan.
- Der gasm-Backend bleibt und wird weiter gebaut: Der Browser-Build ist ebenfalls ein Ziel.
- Grundlagen der Paketierung: `.desktop`-Datei und Icon unter Linux.

Fertig, wenn: der native Build alle zwölf Level unter Linux, Windows und macOS in Originalgeschwindigkeit
spielt, auf einem 60-Hz- wie auf einem 144-Hz-Bildschirm, und der gasm-Build weiterhin baut und läuft.

## Phase 2 — jede Auflösung und jedes Seitenverhältnis, bis 8K

- 3D: Die Projektion ist in `src/render/render.c` fest auf 4:3 verdrahtet (die drei Stellen mit
  `4.0f / 3.0f`: Spielkamera, Standardkamera, Himmel), das Bild hat schwarze Balken. Das echte
  Seitenverhältnis der Zeichenfläche verwenden.
- 2D: Menüs und HUD leben in einem 640×480-Koordinatenraum (`src/app.c` rechnet die Maus darauf um,
  `render.c` zeichnet den Vordergrund darin). Nach einer einzigen Regel skalieren: Das 640×480-Layout bleibt
  4:3 in der Mitte, an einem Rand verankerte Elemente bleiben an diesem Rand. Die Mausumrechnung folgt
  derselben Regel.
- HiDPI: in den echten Pixeln des Fensters zeichnen (Pixeldichte von SDL3), nicht in logischen Punkten.
- 8K (7680×4320) passt in das Standardlimit von WebGPU von 8192 pro Textur und Bild; prüfen, nicht annehmen.
- Bildqualität: anisotrope Filterung der Welttexturen (fast jedes Ballance-Level sieht man unter flachem
  Winkel), MSAA. Beides einstellbar.

Fertig, wenn: Fenster in 16:9, 21:9, 16:10 und 4:3 ein unverzerrtes Bild mit brauchbarem HUD und Menü zeigen,
von 1280×720 bis 7680×4320, und das Fenster sich während des Spiels in der Größe ändern lässt.

## Phase 3 — die Kugel ist wirklich rund

- Ein Render-Primitiv „analytische Kugel“: pro Pixel über den Schnitt von Strahl und Kugel gezeichnet,
  mit Beleuchtung pro Pixel und exakter Tiefe, daher rund bei jeder Auflösung und Entfernung.
- Welche Meshes Kugeln sind, sagen die Daten (eine Tabelle der drei Kugel-Meshes nach ihren Namen in den
  Spieldateien; im offenen Format eine Eigenschaft des Objekts), nie ein Sonderfall im Renderer.
- Die Textur behält die Originalabwicklung, sodass jede Kugel aussieht wie sie selbst, nur ohne Facetten.
- Die Physik bleibt unberührt: Dort ist die Kugel bereits eine Kugel.

Fertig, wenn: alle drei Kugeln in 8K und aus nächster Nähe vollkommen rund sind und ansonsten wie die
Originale aussehen.

## Phase 4 — Level 13 und Community-Karten

- **Level 13** („Speed“): eine kostenlose Erweiterung, die Cyparade 2004 veröffentlicht hat; auf der
  Original-CD ist sie nicht. Das Laden ist fest auf 12 Level verdrahtet (`src/app.c`, `Level_%02d.NMO`; der
  Viewer nimmt 1–12). Level 13 unterstützen, wenn die Datei vorhanden ist, auf demselben Weg wie die anderen.
- **Community-Karten**: Rund 425 Karten (von 2009 bis heute) sind im Katalog
  [pseudoacetyl/BallanceCustomMaps](https://github.com/pseudoacetyl/BallanceCustomMaps) verzeichnet
  (`CustomMapList.md`: Autor, Schwierigkeit, Typ, Videos). Heruntergeladen werden sie vom Mega-Laufwerk der
  Community, ein ausgewähltes Paket liegt auf MediaFire, neue Karten erscheinen im Kanal `#new` des
  Ballance-Discords ([FAQ](https://yyc12345.github.io/ballance-discord-rules/en/faq/)). Eine Karte ist eine
  `.NMO` im selben Format wie `Level_XX.NMO`.
  - Ein Kartenordner im Benutzerdatenverzeichnis und ein Menü im Spiel zur Auswahl. Originaldateien werden
    nie angefasst (die anderen Konventionen der Community sind das Ersetzen von `Level_XX.NMO` oder der
    Ordner `ModLoader/Maps` des Mod-Loaders BML).
  - Eine Karte, die einen Building Block nutzt, den wir nicht umsetzen, stürzt nicht ab: Das Spiel meldet,
    welcher Block fehlt, mit Namen und GUID. Ein Lauf ohne Fenster über die ganze Sammlung ergibt eine
    Häufigkeitsliste der fehlenden Blöcke; umgesetzt wird in dieser Reihenfolge.
  - **Karten sind nicht vertrauenswürdige Eingaben** (Dateiablagen von Fremden). Bevor es das Kartenmenü
    gibt, hat jeder Parser, der `.nmo` / `.cmo` liest, Fuzzing-Ziele, läuft unter ASan + UBSan und lehnt
    fehlerhafte Daten ab, statt abzustürzen.

Fertig, wenn: Level 13 spielbar ist; ein Ordner mit Community-Karten im Menü erscheint und die Karten
spielbar sind; der Bericht über fehlende Blöcke existiert; die Parser ein Fuzzing nennenswerter Dauer ohne
Befund überstanden haben.

## Phase 5 — Physik am Original geprüft

Der Port des Upstreams wurde nie zahlenmäßig mit dem Originalspiel verglichen (`docs/guide/status.md`).

- Referenzbahnen aus dem Originalspiel aufzeichnen (einmal unter Wine gestartet, nur als Messinstrument):
  Kugeltyp, Start, Eingabefolge → Position und Geschwindigkeit pro Takt.
- Dieselben Szenarien ohne Fenster in unserer Engine abspielen und automatisch vergleichen. Eine Abweichung
  zeigt auf einen Takt und einen Kontakt, nicht auf ein Gefühl.
- Daraus werden Regressionstests.

## Phase 6 — offenes Levelformat und Konverter

- Ein offenes, dokumentiertes Levelformat: Geometrie in glTF 2.0, Texturen in PNG, Klänge in Opus, Platzierung
  und Eigenschaften der Objekte in einer Textdatei. Die Spezifikation liegt in `docs/`.
- Ein Konverter von `.nmo` dorthin, als eigenes Werkzeug.
- Die Engine versteht beide Formate nebeneinander und dauerhaft: `.nmo` bleibt, weil die Community-Karten
  nur darin existieren.
- Damit konvertierte Level ohne die Virtools-Graphen spielbar sind, muss die Spiellogik aus den Graphen in
  typisierte Entitäten der Engine wandern — eine Architekturänderung, die nach den Phasen 1–5 kommt.
- Test: Jedes Originallevel und seine konvertierte Kopie müssen bei gleicher Eingabe dieselben Bahnen ergeben.
- Das Hochskalieren der Texturen ist ein Schritt dieser Pipeline: Im Repository liegen die Pipeline und ihre
  Parameter (ein Modell mit offener Lizenz, Einstellungen und Masken pro Textur), niemals hochskalierte
  Originaltexturen. Siehe decisions.md.

## Durchgehend

- Die Befunde aus der Anwendung der secure-codegen-Regeln auf den Upstream-Code beheben, sobald der betroffene
  Code an die Reihe kommt: CI-Actions sind per Tag (`actions/checkout@v7`) statt per Commit-SHA festgelegt und
  haben kein `permissions:`; keine Härtungs-Flags in CMake; zwei `strcpy` von Konstanten in `src/app.c`.
- Datenfreie Tests laufen überall (Unit-Tests, synthetische `.nmo`, Fuzzing, Sanitizer); Tests mit Spieldaten
  laufen lokal mit `OPENBALLANCE_DATA`.
- Dokumentation in drei Sprachen (siehe `CLAUDE.md`). Die Dokumentation des Upstreams (Benutzerhandbuch,
  Interna) gibt es bisher nur auf Englisch; bevor die offizielle Website online geht, braucht sie eine
  russische und eine deutsche Übersetzung, und die Website muss von OpenBallance in Drahtseilakt umbenannt
  werden.
