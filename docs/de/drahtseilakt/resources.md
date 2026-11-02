# Ressourcen der Oberfläche

Das Spiel fragt nach seinen Dateien in der Anordnung des Originals; jede Datei spielt eine **Rolle** (die Schrift
des Menüs, der Klang des Falls, die Holzkugel...). Nach der Entscheidung „Eigenes als Standard“
([decisions.md](decisions.md)) können drei Quellen eine Rolle füllen:

- **eigen**: unsere mitgelieferten Dateien in `assets/own/`, angeordnet wie das Original (noch keine: Stufe 2 macht sie);
- **Original**: die Kopie des Spielers;
- **Remaster**: das Remaster der Kopie aus der Toolbox, in `levels/remastered/.interface/`.

Der Schalter **Interface** des erweiterten Modus (own / original / remaster, Einstellung `interface`) wählt, wer
zuerst kommt; er wirkt ab dem nächsten Start. Der klassische Modus spielt immer die Kopie. Eine Rolle, die keine
Quelle füllt, fehlt: das Spiel sagt es einmal und läuft ohne sie weiter. Der eigene Ordner eines Levels
(`<Level>/game/`, siehe [open-format.md](open-format.md)) kommt für seine Teile weiterhin zuerst.

Text, den die Schrift des Spiels nicht kann (sie hat nur Latin-1), wird mit den mitgelieferten Noto-Schriften
(`assets/fonts`, SIL OFL) gezeichnet: die chinesischen und kyrillischen Namen von Community-Karten.

| Rolle | Datei des Originals | Eigen |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | geplant |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | geplant |
| texts in the copy's languages | `3D Entities/Language.nmo` | geplant |
| menu texts, level table defaults | `base.cmo` | geplant |
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | geplant |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | geplant |
| transformer animation | `3D Entities/AnimTrafo.nmo` | geplant |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | geplant |
| game font texture | `Textures/Font_1.tga` | geplant |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | geplant |
| ball shadow | `Textures/HardShadow.bmp` | geplant |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | geplant |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | geplant |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | Klangthemen (assets/sounds) |
| menu sounds | `Sounds/Menu_click, Menu_load` | Klangthemen (assets/sounds) |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | Klangthemen (assets/sounds) |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | eigene: neun Stücke, Music/music.txt verteilt sie auf die fünf Themen; kein Atmosphäre-Teil |
| original levels | `3D Entities/Level/Level_NN.NMO` | geplant |

Eigenes kommt in Stufen: Oberfläche (Schriften, Knöpfe, Punktekasten, Lebenskugeln, Texte, Menüszene), dann Kugeln,
Himmel, Partikel und Klänge (aus offenen Bibliotheken), Musik im Stil eines Mahjong-Spiels mit chinesischen
Motiven, dann eigene Module und Level.

Klänge und Musik kommen als **Klangthemen**, jedes ein Ordner (`Sounds/` und `Music/` nach den Namen, nach denen
das Spiel fragt, Ogg Vorbis oder WAV, eine Tabelle `music.txt` für die Levelthemen, eine `theme.txt` mit dem Namen in
vier Sprachen, Autor, Lizenz und `fallback`). Das Spiel liefert zwei in `assets/sounds/`: *system* (frei lizenzierte
Aufnahmen und die Musik des Inhabers, gemacht von `assets/sounds/system/prepare.py`, CC0 oder CC-BY, Autoren und
Änderungen in ihrer `CREDITS.md` und auf dem Credits-Bildschirm) und *neuro* (für das Spiel synthetisierte Effekte,
GPL-3.0-only; Musik und fehlende Rollen aus *system*). Eigene Themen legt der Spieler in `sounds/<Name>/` des
Datenordners; darunter macht die Toolbox *original* aus der Kopie des Spielers, das nie ausgeliefert wird (solange es
fehlt, spielt seine Wahl die Klänge der Kopie). Die Zeile *Sounds* der Seite Extended wählt das Thema
(`sound-theme NAME`); der klassische Modus spielt *system*. Ein Klang, der dem Thema und seinem Ersatz fehlt, kommt aus
der Kopie (die eigenen Klänge eines Levels); Musik nie. Der Themenordner eines Spielers ist nicht vertrauenswürdig:
nur einfache Namen, keine Links, begrenzte Zahl von Themen, Klänge durch die Grenzen der Klangschicht dekodiert. Die
Musik des Levels wird vorab in einem eigenen Thread dekodiert.
