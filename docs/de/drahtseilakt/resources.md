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
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | eigene (assets/own, CREDITS.md) |
| menu sounds | `Sounds/Menu_click, Menu_load` | eigene (assets/own, CREDITS.md) |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | eigene (assets/own, CREDITS.md) |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | eigene: neun Stücke, Music/music.txt verteilt sie auf die fünf Themen; kein Atmosphäre-Teil |
| original levels | `3D Entities/Level/Level_NN.NMO` | geplant |

Eigenes kommt in Stufen: Oberfläche (Schriften, Knöpfe, Punktekasten, Lebenskugeln, Texte, Menüszene), dann Kugeln,
Himmel, Partikel und Klänge (aus offenen Bibliotheken), Musik im Stil eines Mahjong-Spiels mit chinesischen
Motiven, dann eigene Module und Level.

Klänge und Musik: der klassische Modus und die Oberfläche *own* des erweiterten Modus spielen unsere (assets/own: Ogg
Vorbis, von `assets/own/prepare.py` aus frei lizenzierten Aufnahmen gemacht, CC0 oder CC-BY; Autoren und Änderungen
in `assets/own/CREDITS.md`, auch auf dem Credits-Bildschirm); die Oberfläche *original* oder *remaster* spielt die der
Kopie. Unsere Musik wird nie durch die der Kopie ersetzt, ein fehlender Klang schon. Die Musik des Levels wird vorab
in einem eigenen Thread dekodiert.
