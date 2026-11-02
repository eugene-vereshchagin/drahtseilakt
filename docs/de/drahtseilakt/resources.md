# Ressourcen der Oberfläche

Das Spiel fragt nach seinen Dateien in der Anordnung des Originals; jede Datei spielt eine **Rolle** (die Schrift
des Menüs, der Klang des Falls, die Holzkugel...). Nach der Entscheidung „Eigenes als Standard“
([decisions.md](decisions.md)) können drei Quellen eine Rolle füllen:

- **eigen**: unsere mitgelieferten Dateien in `assets/own/`, angeordnet wie das Original (die Oberfläche seit Stufe 2: siehe „Die eigene Oberfläche“ unten);
- **Original**: die Kopie des Spielers;
- **Remaster**: das Remaster der Kopie aus der Toolbox, in `levels/remastered/.interface/`.

Der Schalter **Interface** des erweiterten Modus (own / original / remaster, Einstellung `interface`) wählt, wer
zuerst kommt; er wirkt ab dem nächsten Start. Vorgabe, und was der klassische Modus spielt, ist die Oberfläche der Kopie, wenn sie ihr Menü hat, sonst **Eigen**
(decisions.md, „Die Oberfläche der Kopie als Vorgabe, solange unsere neu entsteht“). Eine Rolle, die keine
Quelle füllt, fehlt: das Spiel sagt es einmal und läuft ohne sie weiter. Der eigene Ordner eines Levels
(`<Level>/game/`, siehe [open-format.md](open-format.md)) kommt für seine Teile weiterhin zuerst.

Text, den die Schrift der Kopie nicht kann (sie hat nur Latin-1), wird mit den mitgelieferten Schriften
(`assets/fonts`, SIL OFL) gezeichnet, wie die eigene Oberfläche all ihren Text zeichnet: russische und chinesische
Menüs bei gewählter Oberfläche des Originals, die chinesischen und kyrillischen Namen von Community-Karten.

| Rolle | Datei des Originals | Eigen |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | Tektur (`assets/fonts`), vom Spiel gezeichnet |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | unsere im offenen Format: `3D Entities/MenuLevel/` |
| texts in the copy's languages | `3D Entities/Language.nmo` | `Texts/menu.txt`, sieben Sprachen |
| menu texts, level table defaults | `base.cmo` | geplant |
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | ein Level von uns bringt seine eigenen Einstellungen mit (open-format.md) |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | unsere im offenen Format: `3D Entities/Balls/` |
| transformer animation | `3D Entities/AnimTrafo.nmo` | unsere: `3D Entities/AnimTrafo/` |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | unsere: `3D Entities/PH/<module>/`, jedes Modul des Originals |
| game font texture | `Textures/Font_1.tga` | Tektur, vom Spiel gerastert |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | unsere, gleiche Namen |
| ball shadow | `Textures/HardShadow.bmp` | unserer: `Textures/HardShadow.png` |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | unsere für die Buchstaben A, B, C: `Textures/Sky/Sky_<letter>_<face>.png` |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | unsere, dieselben Namen als `.png` |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | Klangthemen (assets/sounds) |
| menu sounds | `Sounds/Menu_click, Menu_load` | Klangthemen (assets/sounds) |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | Klangthemen (assets/sounds) |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | eigene: in *system* achtzehn Stücke und acht Atmosphären, Music/music.txt gibt jedem Teil einen Pool (Themen 1–3 die ersten neun Stücke, Themen 4 und 5 neun weitere); in *neuro* fünfzehn synthetisierte Phrasen, eine je Teil, eigene Atmosphären und Zielmusik |
| original levels | `3D Entities/Level/Level_NN.NMO` | unsere eigenen Level, `assets/levels/` (levels.md) |

## Die eigene Oberfläche

Stufe 2 (2026-11-03): die Menüs, die Oberfläche im Level (Punktekasten, Lebenskugeln, „Ball verloren“), der
Endbildschirm und der Ladebildschirm sind unsere eigenen. Ein Spieler ohne die Oberflächendateien des Originals sieht
sie alle, und solange die eigene Oberfläche gewählt ist, werden die Oberflächendateien der Kopie gar nicht gelesen.

- **Texturen** (`assets/own/Textures/`): `Button01_select.tga`, `Button01_deselect.tga` und `Button01_special.tga`, jedes
  Teil dort, wo der Code des Spiels es entnimmt (die UV-Rechtecke in `menu.rs`, `hud.rs`, `endscreen.rs`), in der Art
  des Originals, auf eigene Weise gemacht (Version 2, 2026-11-03): durchsichtiges Glas, neutral grau oder (gewählt)
  dunkelrot; Stahlrohre, als beleuchtete Zylinder schattiert, mit gebürsteter Maserung; Rinnen aus Rohren mit getönter
  Füllung (fliedergrau, golden beim gewählten Knopf) um eine Chromkugel, in der sich ein Himmel spiegelt; die Plätze
  der Highscores mit einer Perle, die Lebenskugel und ihr Rahmen, der Punktekasten mit seiner Schiene, der leuchtende
  Balken des Endbildschirms, das dunkle Band.
- **Die Menüszene** (`assets/own/3D Entities/MenuLevel/`, das offene Levelformat): worüber die Menüs und der
  Ladebildschirm stehen; das Spiel zeichnet sie wie MenuLevel.nmo des Originals, ihre Kamera zieht denselben Kreis mit
  demselben Objektiv. Eine Insel aus Bahnteilen in einem Dämmerungshimmel: ein Mittelblock mit dunkler Schale, Bahnarme
  mit erhöhten Rändern, der Start mit vier Schalen und einer Steinkugel, drei Transformatoren, zwei Laternentürme auf
  hohen Masten, Stahlschienen und eine Spirale, eine Holzrampe, Säulen hinab in die Wolken und ferne Böden. Kurve und
  Ziel der Kamera sind die Knoten `I_MenuLevel_Curve_0..3` und `Cam_MenuLevel_Target`, die Flammen brennen an
  `FourFlames_Flame_A..D`, ihr eigener Himmel sind die `Sky_*`-Objekte, um die Kamera gezeichnet.
- **Wie sie entstehen**: durch `assets/own/make/make_interface.py` (die Texturen) und `make_menu_scene.py` (die Szene,
  ihre Texturen und ihr Himmel), Python mit numpy und Pillow, nur aus den Skripten und der mitgelieferten Schrift
  Tektur; die Ergebnisse liegen im Repository, der Build braucht kein Python. Wie bei den erzeugten Klängen: das
  Original wird vermessen (Palette, Linienstärken, Proportionen und Anordnung; bei der Szene Höhen und Ausdehnung der
  Böden, Himmelsfarben nach Höhenwinkel), unseres wird nach diesen Zahlen gezeichnet, und `metric.py` bewertet, wie nah
  jedes Element und jeder Bildschirm kommt (Überlappung der Lab-Farbhistogramme, SSIM der unscharfen Helligkeit und der
  Kantenbilder); `scores.md` hält die Werte fest. Nichts vom Original ist kopiert, nachgezeichnet oder neu abgetastet,
  seine Bilder kommen nie ins Repository.
- **Schrift**: Tektur (SIL OFL, `assets/fonts/Tektur-Variable.ttf`, Version und Prüfsumme in `assets/fonts/README.md`),
  eine eckige technische Schrift, im Charakter nahe an der breiten Menüschrift des Originals; Kleinbuchstaben als
  Kapitälchen, verbreitert und gesperrt nach den gemessenen Proportionen des Originals (Breite der Beschriftungen
  innerhalb von 2 %). Was ihr fehlt, kommt aus Noto Sans, Chinesisch aus Noto
  Sans SC. Die Glyphen werden zur Laufzeit in einen Atlas gerastert und mit denselben Rechtecken, Ausrichtungen,
  Umbrüchen und Schatten gesetzt wie die Schrift des Spiels, die Bildschirme behalten also das Layout des Originals
  (`crates/game/src/fallback.rs`).
- **Texte** (`assets/own/Texts/menu.txt`, ins Spiel eingebaut): alle Texte der Menüs, des Levels und des
  Endbildschirms auf Englisch, Russisch, Deutsch, vereinfachtem Chinesisch, Französisch, Spanisch und Italienisch,
  unter den Schlüsseln von `Language.nmo` des Originals, wo es sie gibt. Mit der Oberfläche des Originals kommen
  zuerst die Texte der Kopie in ihren fünf Sprachen; für Russisch und Chinesisch (die die Kopie nicht hat) unsere, in
  den mitgelieferten Schriften gezeichnet.

Die Standard-Highscore-Listen der eigenen Oberfläche sind ebenfalls unsere (zehn Namen, 3000 bis 750 Punkte).
Weiterhin aus der Kopie, wenn es eine gibt: die Standardtasten (DB_Options aus base.cmo; ohne sie Pfeile, Shift und
Leertaste).

## Unsere eigenen Teile

Stufe 3 (2026-11-03, decisions.md „Das Spiel läuft ohne Weiteres“): jedes Teil, das ein Level nennen kann, hat ein
eigenes von uns, im offenen Format unter `assets/own/3D Entities/` (die Module von `PH/`, `Balls/`, `AnimTrafo/`) und
`assets/own/Textures/` (Himmel, Partikel, der Schatten der Kugel; ein als `.bmp` oder `.tga` verlangtes Bild kann
unseres als `.png` sein), gemacht von `assets/own/make/make_parts.py` aus nichts als dem Skript. Sie sind nach den
gemessenen Größen des Originals gemacht: jedes Objekt, das der Code des Spiels nennt (Teile, Gelenkrahmen,
Flammenframes, die Meshes der Kugeln, deren Boxen die Physik benutzt), hat die gemessene Lage, jedes Kollisionsmesh
die gemessenen Maße als eigene Form (ein Quader, ein Pyramidenstumpf, ein Prisma, ein Ellipsoid), und die physikalischen
Werte (Massen, Reibungen, Gelenke, Kräfte) sind die des Spiels, unverändert (crates/game/src/modules.rs); nur das
Aussehen ist neu. `crates/game/src/partcheck.rs` fährt jedes Modul der Level 1 bis 12 mit dem Teil der Kopie und mit
unserem vom selben Start mit derselben Eingabe und vergleicht die Wege.

Ein Level nimmt zuerst unsere Teile, wenn es neuer Inhalt ist (ein Level im offenen Format, nicht aus einer `.nmo`
umgewandelt: kein `source_hash` in seinen Einstellungen), es spielt also mit und ohne Kopie gleich; ein Level der
Kopie, eine Community-Karte oder ein umgewandeltes Level oder Remaster nimmt zuerst die der Kopie, wie es gemacht
wurde, und unsere für das, was der Kopie fehlt (ohne Kopie unsere). Der eigene `game/`-Ordner eines Levels kommt vor
beiden.

Eigenes kommt in Stufen: Oberfläche (Schriften, Knöpfe, Punktekasten, Lebenskugeln, Texte, Menüszene), dann Kugeln,
Himmel, Partikel und Klänge (aus offenen Bibliotheken), Musik im Stil eines Mahjong-Spiels mit chinesischen
Motiven, dann eigene Module und Level.

Klänge und Musik kommen als **Klangthemen**, jedes ein Ordner (`Sounds/` und `Music/` nach den Namen, nach denen
das Spiel fragt, Ogg Vorbis oder WAV, Tabellen `Music/music.txt` und `Sounds/sounds.txt`, die einer Rolle mehrere Dateien geben, einen Pool, aus dem das Spiel zufällig wählt, eine `theme.txt` mit dem Namen in
vier Sprachen, Autor, Lizenz und `fallback`, `none` für kein anderes Thema). Das Spiel liefert zwei in `assets/sounds/`: *system* (frei lizenzierte
Aufnahmen und die Musik des Inhabers, gemacht von `assets/sounds/system/prepare.py`, CC0 oder CC-BY, Autoren und
Änderungen in ihrer `CREDITS.md` und auf dem Credits-Bildschirm) und *neuro* (für das Spiel synthetisierte Effekte
und Levelmusik, GPL-3.0-only; die Levelmusik sind kurze gezupfte Phrasen im Stil der Guzheng, eine für jeden der drei
Teile der fünf Themen; es enthält jede Rolle, nach der das Spiel fragt, und nimmt nichts aus einem anderen Thema,
`fallback none`). Eigene Themen legt der Spieler in `sounds/<Name>/` des
Datenordners; darunter macht die Toolbox *original* aus der Kopie des Spielers, das nie ausgeliefert wird (solange es
fehlt, spielt seine Wahl die Klänge der Kopie). Die Zeile *Sounds* der Seite Extended wählt das Thema
(`sound-theme NAME`); bis der Spieler ein anderes wählt, und im klassischen Modus immer, spielt *neuro*
(decisions.md, „Das generierte Klangthema ist der Standard“); eine getroffene Wahl bleibt. Ein Klang, der dem Thema und seinem Ersatz fehlt, kommt aus
der Kopie (die eigenen Klänge eines Levels); Musik nie. Der Themenordner eines Spielers ist nicht vertrauenswürdig:
nur einfache Namen, keine Links, begrenzte Zahl von Themen, Klänge durch die Grenzen der Klangschicht dekodiert. Die
Musik, die als Nächste spielen kann, wird vorab in einem eigenen Thread dekodiert. Wie man ein Thema macht, alle
Rollen und das Format der Tabellen: [Klangthemen](sound-themes.md).
