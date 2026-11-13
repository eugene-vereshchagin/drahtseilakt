# Probleme lösen

**Die Originallevel fehlen.** Für *Klassisches Ballance* braucht das Spiel Ihre eigene Ballance-Kopie. Importieren Sie
sie mit der Toolbox (siehe [Die Toolbox](toolbox.md)), mit dem Ordner des installierten Spiels oder seinem CD-Abbild.
Prüfen Sie, ob die Toolbox die Kopie gesehen hat: Ihr Fenster zeigt das Ergebnis, und der Ordner
`levels/ballance-original/` in Ihrem Datenordner füllt sich.

**Meine Karte steht nicht in der Liste.** Ein Level muss in einem eigenen Ordner mit einer `.nmo` (oder einem Level im
offenen Format) und einer `level.txt` liegen (siehe [Level und Ordner](levels.md)). Eine lose Datei oder ein Ordner
ohne `level.txt` wird nicht angezeigt. Legen Sie die Karte in `levels/user-levels/` in Ihrem Datenordner und öffnen Sie
die Liste erneut. Startet man das Spiel aus einem Terminal, nennt das Protokoll jeden übergangenen Eintrag.

**Das Laden dauert lange.** Große 4K-Texturen brauchen Zeit. Von der Toolbox vorbereitete Level enthalten Texturen in
einer Form, die die Grafikkarte schnell liest; haben Sie mit einer älteren Toolbox remastert, lassen Sie sie noch
einmal laufen. Karten, die die Toolbox nicht vorbereitet hat, werden beim Laden abgerundet, das dauert einen Moment.

**Das Bild ruckelt oder reißt.** Probieren Sie unter *Optionen, Grafik* ein kleineres Fenster, schalten Sie
*Bildsynchronisation?* ein oder wählen Sie das Aussehen *Drahtseilakt*.

**Ein Level lädt nicht oder sieht falsch aus.** Manche Community-Karten nutzen Funktionen, die das Spiel noch nicht
kennt. Der Bericht der Toolbox in der Levelliste sagt, was sie gefunden hat. Eine solche Karte kann trotzdem spielbar sein.

## Wo meine Dateien liegen

In Ihrem Datenordner (siehe [Level und Ordner](levels.md)):

- `settings.txt`: Ihre Einstellungen.
- `user-replays/`: die Wiederholungen, die Sie speichern lassen (Optionen, Replays), siehe [Spielen](playing.md#wiederholungen).
- `screenshots/`: Bilder, die der Mod „Screenshot-Taste“ gespeichert hat.

## Ein Problem melden

Wohin Sie Probleme melden können, steht bald hier. Bis dahin schalten Sie *Replays speichern* ein (Optionen, Replays)
und heben die `.replay`-Datei des Laufs auf, der das Problem zeigt: sie zeigt genau, was geschah. Die Entwicklerprotokolle
in `replays/<Level>/` entstehen nur, wenn das Spiel mit `--debug`, mit der Umgebungsvariable `DRAHTSEILAKT_DEV_LOGS=1`
oder mit `dev-logs on` in `settings.txt` gestartet wird.
