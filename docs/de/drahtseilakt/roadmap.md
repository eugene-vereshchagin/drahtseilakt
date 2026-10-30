# Fahrplan

Was dieser Fork vorhat, in welcher Reihenfolge und woran man erkennt, dass ein Schritt fertig ist. Die Gründe
für die Entscheidungen stehen in [decisions.md](decisions.md). Wer die Arbeit übernimmt, fängt hier an.

Stand 30.10.2026: Der Fork ist das OpenBallance 0.1.3 des Upstreams plus Projektregeln (`CLAUDE.md`) und diese
Dokumente. Am Engine-Code wurde noch nichts geändert; Phase 0 läuft.

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

## Phase 1b — aufgezeichnete Durchläufe (Replays)

Ziel: Niemand muss Level von Hand nachspielen, um zu prüfen, dass eine Änderung nichts kaputt gemacht hat.

- Mit dem festen 60-Hz-Takt ist das Spiel deterministisch: Dieselbe Eingabe ab demselben Takt ergibt dieselbe
  Bahn. Ein **Replay** ist ein Protokoll der Eingabe pro Takt, aus echtem Spielen aufgezeichnet, in einem
  dokumentierten Textformat (versioniert; das Level ist darin per Hash seiner Datei benannt, sodass ein Replay
  nie mit falschen Daten läuft).
- Wiedergabe ohne Fenster in voller Geschwindigkeit (`run_test` aus dem Upstream steuert das Spiel bereits ohne
  Fenster mit geskripteter Eingabe). Ein Replay prüft: Checkpoints in den erwarteten Takten erreicht, das Ziel,
  die Punkte.
- Gleitkommaergebnisse können sich zwischen CPU-Architekturen und Compilern unterscheiden, und kleine
  Abweichungen wachsen über Hunderte Takte. Replays werden an Schlüsselpunkten mit Toleranz geprüft; für exakte
  Übereinstimmung hat jede Plattform eine eigene Referenz.
- Woher Replays kommen: Der Inhaber spielt jedes Level einmal; das Originalspiel unter Wine (Phase 5);
  Speedrun-Aufzeichnungen, falls sich die Formate der Community umwandeln lassen.
- Replays enthalten nur unsere Eingabe und keine Spieldaten, daher liegen sie im Repository (`tests/replays/`)
  und laufen in der CI.

Fertig, wenn: ein Replay jedes der zwölf Level aufgezeichnet ist, ohne Fenster abgespielt wird und fehlschlägt,
wenn die Physik absichtlich verändert wird.

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

## Phase 3 — Rundes sieht rund aus

Die gekrümmte Geometrie des Originals — die Kugel, Schienen, Rohre, abgerundete Plattformkanten, gebogene
Rinnen — besteht aus groben Polygonen, und bei heutigen Auflösungen sieht man jede Facette. Diese Phase macht
sie glatt. **Welche Techniken eingesetzt werden und wie weit jede geht, wird nach dem Ausprobieren mit dem
Inhaber entschieden**; die Kandidaten:

- **Die Kugel, analytisch.** Ein Render-Primitiv „analytische Kugel“: pro Pixel über den Schnitt von Strahl
  und Kugel gezeichnet, mit Beleuchtung pro Pixel und exakter Tiefe, daher rund bei jeder Auflösung und
  Entfernung. Welche Meshes Kugeln sind, sagen die Daten (eine Tabelle der drei Kugel-Meshes nach Namen; im
  offenen Format eine Eigenschaft des Objekts), nie ein Sonderfall im Renderer. Die Textur behält die
  Originalabwicklung.
- **Beleuchtung pro Pixel mit geglätteten Normalen** für die gesamte Geometrie. Ein guter Teil des facettierten
  Aussehens ist die Vertex-Beleuchtung des Originals, die über die Flächen verschmiert wird; mit Beleuchtung
  pro Pixel wirken gekrümmte Flächen gekrümmt, ohne dass sich ein Vertex bewegt. Am billigsten, größter Gewinn.
- **Phong-Tessellation** (oder eine ähnliche normalengeführte Verfeinerung): Dreiecke werden unterteilt und
  entlang der Vertex-Normalen gewölbt. Wo benachbarte Flächen Normalen teilen (Schienen, Rohre), wird die
  Fläche rund; wo eine Kante getrennte Normalen hat (Kistenecken, Plattformkanten), bleibt sie scharf. Eine
  Regel für die ganze Geometrie, Community-Karten eingeschlossen — der Umriss einer Schiene wird ein Kreis.
- **Erkannte Grundformen neu aufbauen** (eine Schiene als entlang eines Pfads gezogener Kreis, in beliebiger
  Detailstufe neu erzeugt): das sauberste Ergebnis, aber schwieriger und anfälliger; gehört in den Konverter
  (Phase 6), nicht in die Laufzeit.

Fest, egal wie gewählt wird: **die Physik bleibt auf der Originalgeometrie.** Das Glätten ist rein optisch; die
Kugel rollt auf denselben Polygonen wie 2004, sonst ändern sich ihr Verhalten und jede Community-Karte.
Prüfen, dass der kleine Abstand zwischen sichtbarer und physikalischer Oberfläche (die Kugel wirkt einen
Hauch eingesunken oder angehoben) nicht auffällt.

Fertig, wenn: die gewählten Techniken umgesetzt sind; die drei Kugeln in 8K und aus nächster Nähe vollkommen
rund sind und ansonsten wie die Originale aussehen; Schienen und andere gekrümmte Teile bei den Auflösungen
aus Phase 2 keine Facetten mehr zeigen; die Replays (Phase 1b) dieselben Bahnen wie zuvor ergeben (nichts
Physikalisches hat sich bewegt).

## Phase 3b — Replays als Video gerendert

Ziel: ein 8K-Video eines perfekten Durchlaufs des neu aussehenden Ballance für einen YouTube-Kanal.

- Ein Replay offline Bild für Bild rendern, in beliebiger Auflösung (bis 7680×4320) und mit fester Bildrate,
  unabhängig von der Echtzeit — auch ein langsamer Rechner liefert ein vollkommen flüssiges 8K-Video. Der Ton
  wird aus demselben deterministischen Lauf gerendert. Die Bilder gehen als Strom an einen Encoder (z. B.
  ffmpeg), nicht als Dateien auf die Platte.
- Kameras: zuerst die Spielkamera; eine filmische Kamera (weiche Verfolgung, Totalen eines Levels) später, als
  Daten im Replay oder daneben, nicht als Code pro Level.
- „Perfekte“ Durchläufe: zuerst die besten menschlichen (Inhaber, Speedrunner). Eine automatische Suche nach
  schneller Eingabe — ein TAS-artiger Optimierer, der Eingabefolgen durchprobiert, bis die Kugel den nächsten
  Checkpoint früher erreicht — ist ein späteres Zusatzziel: Er braucht keinen Menschen, ist aber ein eigenes
  Forschungsprojekt.
- Hochladen ist Veröffentlichen: Nichts geht ohne Anweisung des Inhabers auf YouTube. Material aus den
  Original-Assets ist Inhalt des Rechteinhabers; Videos einer Veröffentlichung mit eigenen Texturen (siehe
  decisions.md) werfen diese Frage nicht auf.

Fertig, wenn: das Replay eines Levels zu einem 8K-Video mit Ton gerendert wird, das der Inhaber veröffentlichen
möchte.

## Phase 4 — Level 13 und Community-Karten

- **Level 13** („Speed“): eine kostenlose Erweiterung, die Cyparade 2004 veröffentlicht hat; auf der
  Original-CD ist sie nicht. Das Laden ist fest auf 12 Level verdrahtet (`src/app.c`, `Level_%02d.NMO`; der
  Viewer nimmt 1–12). Level 13 unterstützen, wenn die Datei vorhanden ist, auf demselben Weg wie die anderen.
- **Community-Karten**: Rund 425 Karten (von 2009 bis heute) sind im Katalog
  [pseudoacetyl/BallanceCustomMaps](https://github.com/pseudoacetyl/BallanceCustomMaps) verzeichnet
  (`CustomMapList.md`: Autor, Schwierigkeit, Typ, Videos). Heruntergeladen werden sie vom Mega-Laufwerk der
  Community, ein ausgewähltes Paket liegt auf MediaFire, neue Karten erscheinen im Kanal `#new` des
  Ballance-Discords ([FAQ](https://yyc12345.github.io/ballance-discord-rules/en/faq/)). Eine Karte ist eine
  `.NMO` im selben Format wie `Level_XX.NMO`. Quellen, Format des Katalogs und seine Nutzung:
    [community-maps.md](community-maps.md).
  - Ein Kartenordner im Benutzerdatenverzeichnis und ein Menü im Spiel zur Auswahl. Originaldateien werden
    nie angefasst (die anderen Konventionen der Community sind das Ersetzen von `Level_XX.NMO` oder der
    Ordner `ModLoader/Maps` des Mod-Loaders BML).
  - Eine Karte, die einen Building Block nutzt, den wir nicht umsetzen, stürzt nicht ab: Das Spiel meldet,
    welcher Block fehlt, mit Namen und GUID. Ein Lauf ohne Fenster über die ganze Sammlung ergibt eine
    Häufigkeitsliste der fehlenden Blöcke; umgesetzt wird in dieser Reihenfolge.
  - **Karten sind nicht vertrauenswürdige Eingaben** (Dateiablagen von Fremden). Bevor es das Kartenmenü
    gibt, hat jeder Parser, der `.nmo` / `.cmo` liest, Fuzzing-Ziele, läuft unter ASan + UBSan und lehnt
    fehlerhafte Daten ab, statt abzustürzen.

- **Spielprofile**: eine Einstellung mit zwei Werten.
  - *Klassisch* — das Originalspiel und sonst nichts: die zwölf Level und Level 13, die Originalregeln, keine
    Inhalte Dritter. Nur hier haben Replays, Rekorde und Vergleiche mit dem Original einen Sinn.
  - *Erweitert* — Inhalte Dritter erlaubt, auf eigene Verantwortung des Spielers, mit einer klaren Warnung beim
    Umschalten: Community-Karten, Texturpakete, später Mods. **Mods heißt hier unser eigenes Mod-System** —
    Daten und Skripte, die in einer Sandbox laufen (ohne Datei- oder Netzwerkzugriff), auf jeder Plattform
    gleich. Die BML-Mods der Community sind Windows-DLLs für die Original-Laufzeitumgebung und können von
    dieser Engine überhaupt nicht geladen werden.
  - Grafikverbesserungen (Auflösung, runde Geometrie, Filterung) sind eigene Einstellungen und in beiden
    Profilen verfügbar: Sie ändern, wie das Spiel aussieht, nicht was es ist.

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
