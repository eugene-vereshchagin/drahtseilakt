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
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | geplant |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | geplant |
| transformer animation | `3D Entities/AnimTrafo.nmo` | geplant |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | geplant |
| game font texture | `Textures/Font_1.tga` | Tektur, vom Spiel gerastert |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | unsere, gleiche Namen |
| ball shadow | `Textures/HardShadow.bmp` | geplant |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | geplant |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | geplant |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | Klangthemen (assets/sounds) |
| menu sounds | `Sounds/Menu_click, Menu_load` | Klangthemen (assets/sounds) |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | Klangthemen (assets/sounds) |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | eigene: in *system* achtzehn Stücke und acht Atmosphären, Music/music.txt gibt jedem Teil einen Pool (Themen 1–3 die ersten neun Stücke, Themen 4 und 5 neun weitere); in *neuro* fünfzehn synthetisierte Phrasen, eine je Teil, eigene Atmosphären und Zielmusik |
| original levels | `3D Entities/Level/Level_NN.NMO` | geplant |

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
- **Fototexturen** (Version 3 der Menüszene, 2026-11-03): ihr Beton (Böden, Block, Säulen), die Ziegelränder, die
  hölzerne Rampe und Kiste und das Eisen (Manschetten, Sockel, Laternenstützen) entstehen aus frei lizenzierten
  Fototexturen (CC0, ambientCG), von `make_menu_scene.py` auf die gemessene Palette und Körnung der Originaltexturen in
  denselben Rollen abgestimmt, mit ihren Normal Maps. `assets/own/make/free_textures.py` legt jedes Quellarchiv mit
  seinem SHA-256 fest (`fetch` lädt fehlende herunter, nichts wird entpackt oder ausgeführt); die Quellen werden nicht
  mitgeliefert, die daraus gemachten Texturen schon. Quellen, Autoren und Lizenzen: `assets/own/CREDITS.md`;
  Prüfsummen der mitgelieferten Dateien: `assets/own/SHA256SUMS`.
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
Weiterhin aus der Kopie, bei jeder Oberfläche: die Standardtasten (DB_Options aus base.cmo; ohne sie Pfeile, Shift und
Leertaste) und die Teile eines Levels (Kugeln, Partikel, der Schatten der Kugel, Himmel, Module), die spätere Stufen
ersetzen.

## Eigene Texturen

Stufe 3 (2026-11-03): die Level, ihre Teile und die Kugeln werden standardmäßig mit unseren eigenen Texturen
gezeichnet (decisions.md, „Eigene Texturen als Vorgabe“): `assets/own/Textures/<Name>.png` tritt an die Stelle der
Originaltextur dieses Namens, welches Level sie auch anfordert — ein Originallevel, eine Community-Karte, ein Modul,
die Kugeln —, verglichen ohne Groß- und Kleinschreibung, `_`, `-` und Leerzeichen (`BallWood.bmp` ist `Ball_Wood`).
45 der 63 Texturen, mit denen die dreizehn Level zeichnen, sind unsere (die Böden und ihre Ziegelstreifen, der
Checkpoint, die Säulen, Ziegel, alles Holz, das Metall, die Dächer, die Markierungen, die Spiegelungen, die drei
Kugeln); Himmel, Partikel, Leuchten, Schatten und einige Bilder kommen weiter aus der Kopie. `assets/own/TEXTURES.md`
listet jede Textur mit dem, was sie ist, ihrer Größe, wie oft sie benutzt wird und ob unsere eintritt. Sie entstehen
durch `assets/own/make/make_textures.py` aus CC0-Fototexturen (`free_textures.py`, festgelegt und mit Prüfsummen;
`assets/own/CREDITS.md`) und vom Skript gezeichneten Formen, nach Palette, Körnung und Anordnung, gemessen an der
Originaltextur gleichen Namens (ihre Ziegelstreifen, Bänder, Ringe und Schattierungen als Anteile ihrer Seite), in
doppelter Auflösung, kachelnd, wo sie kachelt, und die Kugeln so umhüllend wie sie. Der Schalter *Textures* der
erweiterten Seite (`textures own|original`) zeichnet die der Kopie; der klassische Modus zeichnet immer unsere. Die
eigenen Texturen eines Levels im offenen Format (Dateien in seinem Ordner) bleiben die seines Autors.
`--textures original|own` wählt für einen Lauf; `--export` behält die der Kopie, außer mit `--textures own`, das
unsere in das exportierte Level schreibt. Unsere Dateien werden mit dem Spiel geliefert und mit denselben
Größengrenzen gelesen wie jede Textur; ein Name aus einem Level wählt nur unter ihnen, er wird nie zu einem Pfad.

Welche Texturen unsere ersetzen, beim Laden eines Levels (so stimmen Zeichnen und Export überein): in den Leveln und
Teilen der Kopie und in ihren Remastern (`levels/remastered/`) jede, für die wir eine haben, nach Namen; in einer
fremden Karte (einer Community-`.nmo`) eine Textur, die sie per Datei nennt, wenn die Datei die der Kopie ist (eine
Datei aus dem eigenen Ordner der Karte bleibt ihre), und mitgebrachte Pixel nur, wenn sie Pixel für Pixel die Textur
der Kopie dieses Namens sind, sodass die eigenen Bilder einer Karte nie überdeckt werden; die eigenen Dateien eines
Levels im offenen Format nie. Der Dateiname einer Textur aus einer `.nmo` ist immer nur sein letzter Teil, gesucht in
den Texturordnern des Spiels; nur der eigene Ordner eines Levels im offenen Format wird per Pfad gelesen. Die
Menüszene folgt der Oberfläche: unsere mit der eigenen Oberfläche, das MenuLevel der Kopie mit seinen eigenen
Texturen mit der originalen.

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
