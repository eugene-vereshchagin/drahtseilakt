# Die Toolbox

**drahtseilakt-toolbox** ist ein eigenes Programm, das auf Ihrem Rechner Level für das Spiel vorbereitet. Das Spiel
selbst spielt, was in seinen Levelordnern liegt; alles Schwere übernimmt die Toolbox. Sie geht nie online und verändert
Ihre Ballance-Kopie nie. Sie läuft unter Linux, Windows und macOS. Ohne Argumente gestartet, öffnet sie ihr Fenster; die
Texte gibt es auf Englisch, Russisch, Deutsch und in vereinfachtem Chinesisch.

## Was sie kann

- **Ihre Ballance-Kopie importieren.** Wählen Sie den Ordner des installierten Spiels oder ein CD-Abbild (`.cue`,
  `.bin`, `.iso`) mit den Knöpfen **Durchsuchen** und **Abbild** neben dem Pfad. Die Originallevel erscheinen dann im
  Spiel unter *Start, Klassisches Ballance*. Derselbe Schritt erzeugt aus Klängen und Musik Ihrer Kopie das Klangthema
  **original**; sie bleiben auf Ihrem Rechner.
- **In 4K remastern.** Der Knopf *Make* baut die Originallevel mit runderen Formen und 4-fach größeren Texturen neu
  (der Maßstab ist wählbar). Dabei dient der Hochskalierer Real-ESRGAN, wenn er neben der Toolbox liegt, sonst eine
  einfache glatte Vergrößerung. Das Ergebnis landet in `levels/ballance-remaster/` und wird vom Aussehen *Remaster*
  genutzt (siehe [Aussehen](looks.md)). Ihre Originale bleiben unberührt.
- **Heruntergeladene Karten importieren.** Geben Sie einen Ordner mit einer entpackten Sammlung von Community-Karten an
  und drücken Sie *Import*: Jede Karte bekommt in `levels/user-levels/` einen eigenen Ordner mit `level.txt` und Bild,
  fertig fürs Spiel (siehe [Level und Ordner](levels.md)). Ein erneuter Lauf fügt nur Neues hinzu. Archive (`.zip`)
  öffnet sie nicht: Entpacken Sie sie zuerst, außerhalb von `user-levels/`.
- **Die Karten prüfen.** Nach dem Import bittet sie das Spiel, jede Karte auszuprobieren, und legt einen kurzen
  Bericht daneben, den Sie in der Liste Ihrer Level sehen: spielt vollständig, spielt mit Abweichungen, womöglich nicht
  schaffbar oder lädt nicht.

## Der übliche Ablauf

1. Toolbox installieren, starten und auf Ihre Ballance-Kopie zeigen. *Make* drücken und warten (große Texturen dauern;
   der Fortschritt wird je Level angezeigt).
2. Das Spiel starten: *Klassisches Ballance* listet nun die 13 Originale. Unter *Optionen, Grafik* das Aussehen wählen.
3. Für heruntergeladene Karten: Download entpacken, seinen Ordner im Importfeld wählen und *Import* drücken.
