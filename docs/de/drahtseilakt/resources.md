# Ressourcen der Oberfläche

Das Spiel fragt nach seinen Dateien in der Anordnung des Originals; jede Datei spielt eine **Rolle** (die Schrift
des Menüs, der Klang des Falls, die Holzkugel...). Nach der Entscheidung „Eigenes als Standard“
([decisions.md](decisions.md)) können drei Quellen eine Rolle füllen:

- **eigen**: unsere mitgelieferten Dateien in `assets/own/`, angeordnet wie das Original (die Oberfläche seit Stufe 2: siehe „Die eigene Oberfläche“ unten);
- **Original**: die Kopie des Spielers;
- **Remaster**: das Remaster der Kopie aus der Toolbox, in `levels/remastered/.interface/`.

Der Schalter **Interface** des erweiterten Modus (own / original / remaster, Einstellung `interface`) wählt, wer
zuerst kommt; er wirkt ab dem nächsten Start. **Eigen** ist die Vorgabe, und der klassische Modus spielt immer sie (decisions.md,
„Die eigene Oberfläche ist die Vorgabe“). Eine Rolle, die keine
Quelle füllt, fehlt: das Spiel sagt es einmal und läuft ohne sie weiter. Der eigene Ordner eines Levels
(`<Level>/game/`, siehe [open-format.md](open-format.md)) kommt für seine Teile weiterhin zuerst.

Text, den die Schrift der Kopie nicht kann (sie hat nur Latin-1), wird mit den mitgelieferten Schriften
(`assets/fonts`, SIL OFL) gezeichnet, wie die eigene Oberfläche all ihren Text zeichnet: russische und chinesische
Menüs bei gewählter Oberfläche des Originals, die chinesischen und kyrillischen Namen von Community-Karten.

| Rolle | Datei des Originals | Eigen |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | Tektur (`assets/fonts`), vom Spiel gezeichnet |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | ein Bild: `Textures/Menu_Background.png` |
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
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | eigene: in *system* neun Stücke, Music/music.txt verteilt sie auf die fünf Themen; in *neuro* fünfzehn synthetisierte Phrasen, eine je Teil, und eigene Zielmusik; kein Atmosphäre-Teil |
| original levels | `3D Entities/Level/Level_NN.NMO` | geplant |

## Die eigene Oberfläche

Stufe 2 (2026-11-03): die Menüs, die Oberfläche im Level (Punktekasten, Lebenskugeln, „Ball verloren“), der
Endbildschirm und der Ladebildschirm sind unsere eigenen. Ein Spieler ohne die Oberflächendateien des Originals sieht
sie alle, und solange die eigene Oberfläche gewählt ist, werden die Oberflächendateien der Kopie gar nicht gelesen.

- **Texturen** (`assets/own/Textures/`): `Button01_select.tga`, `Button01_deselect.tga` und `Button01_special.tga`, jedes
  Teil dort, wo der Code des Spiels es entnimmt (die UV-Rechtecke in `menu.rs`, `hud.rs`, `endscreen.rs`): violette
  Glasflächen in einem Stahlrohr mit polierter Kugel, rosa mit goldener Kugel beim gewählten Knopf, grau bei einem
  noch nicht offenen Level; die Platznummern der Highscores, die Lebenskugel und ihre Klammer, der Punktekasten, der
  Balken des Endbildschirms, das dunkle Band. Dazu `Menu_Background.png`, das Bild hinter den Menüs und dem
  Ladebildschirm (Dämmerung über einem Wolkenmeer, links eine Holzbahn mit Kugel, rechts eine Säule mit Kugel; die
  Mitte liegt unter dem Band der Menüs). Es steht für die 3D-Menüszene des Originals, die nur mit dessen Oberfläche
  gezeichnet wird.
- **Wie sie entstehen**: durch `assets/own/make/make_interface.py` (Python mit numpy und Pillow), nur aus dem Skript
  und der mitgelieferten Schrift Tektur; die Ergebnisse liegen im Repository, der Build braucht kein Python. Der Stil
  folgt dem Original nur in Farben und Proportionen: nichts vom Original ist kopiert, nachgezeichnet oder neu abgetastet.
- **Schrift**: Tektur (SIL OFL, `assets/fonts/Tektur-Variable.ttf`, Version und Prüfsumme in `assets/fonts/README.md`),
  eine eckige technische Schrift, im Charakter nahe an der breiten Menüschrift des Originals; Kleinbuchstaben als
  Kapitälchen, etwas breiter und gesperrt, wie beim Original. Was ihr fehlt, kommt aus Noto Sans, Chinesisch aus Noto
  Sans SC. Die Glyphen werden zur Laufzeit in einen Atlas gerastert und mit denselben Rechtecken, Ausrichtungen,
  Umbrüchen und Schatten gesetzt wie die Schrift des Spiels, die Bildschirme behalten also das Layout des Originals
  (`crates/game/src/fallback.rs`).
- **Texte** (`assets/own/Texts/menu.txt`, ins Spiel eingebaut): alle Texte der Menüs, des Levels und des
  Endbildschirms auf Englisch, Russisch, Deutsch, vereinfachtem Chinesisch, Französisch, Spanisch und Italienisch,
  unter den Schlüsseln von `Language.nmo` des Originals, wo es sie gibt. Mit der Oberfläche des Originals kommen
  zuerst die Texte der Kopie in ihren fünf Sprachen; für Russisch und Chinesisch (die die Kopie nicht hat) unsere, in
  den mitgelieferten Schriften gezeichnet.

Weiterhin aus der Kopie, bei jeder Oberfläche: die Standardtasten und die Standard-Highscore-Listen (base.cmo) und die
Teile eines Levels (Kugeln, Partikel, der Schatten der Kugel, Himmel, Module), die spätere Stufen ersetzen.

Eigenes kommt in Stufen: Oberfläche (Schriften, Knöpfe, Punktekasten, Lebenskugeln, Texte, Menüszene), dann Kugeln,
Himmel, Partikel und Klänge (aus offenen Bibliotheken), Musik im Stil eines Mahjong-Spiels mit chinesischen
Motiven, dann eigene Module und Level.

Klänge und Musik kommen als **Klangthemen**, jedes ein Ordner (`Sounds/` und `Music/` nach den Namen, nach denen
das Spiel fragt, Ogg Vorbis oder WAV, eine Tabelle `music.txt` für die Levelthemen, eine `theme.txt` mit dem Namen in
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
Musik des Levels wird vorab in einem eigenen Thread dekodiert.
