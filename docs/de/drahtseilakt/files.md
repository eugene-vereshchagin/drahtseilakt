# Wo das Spiel seine Dateien ablegt

Das Spiel liest zweierlei Dateien: die **mitgelieferten Assets** (`assets/`: Schriften, eigene Teile, Level, Fabrik-Mods, Klangthemen) und die **Daten des Spielers** (Einstellungen, Replays, Bestenlisten, importierte und eigene Level, eigene Pakete, Mods und Klangthemen). Beide Anordnungen unten funktionieren mit demselben Programm.

**Mitgelieferte Assets**, der erste Treffer gilt: `$DRAHTSEILAKT_ASSETS`; `assets/` neben dem Programm; `../Resources/assets` (ein macOS-.app-Bundle); und nur für Entwickler der Quellbaum, in dem das Programm gebaut wurde (`assets/` drei Ordner über `target/release` oder der Pfad der Kompilierung). Beim Start gibt das Spiel eine Zeile `paths: assets ... | data ... (portable|per-user)` aus; das Sitzungsprotokoll wiederholt sie.

**Portabel** (Ordner kopieren und spielen; außerhalb wird nichts berührt): `drahtseilakt(.exe)`, `assets/` und eine leere Datei `portable.txt` liegen nebeneinander. Die Daten des Spielers liegen dann in `data/` neben dem Programm. Die Regel ist allein diese Datei: ohne `portable.txt` kein portabler Modus.

**Installiert** (Programm und `assets/` in einem Ordner, z. B. `/opt/drahtseilakt`, `C:\Program Files\Drahtseilakt`, Resources einer .app): die Daten des Spielers liegen im Benutzerordner: `$XDG_DATA_HOME/drahtseilakt` (Standard `~/.local/share/drahtseilakt`) unter Linux, `%APPDATA%\drahtseilakt` unter Windows, `~/Library/Application Support/drahtseilakt` unter macOS. Eigene Pakete, Mods und Themen dort werden zusätzlich zu den mitgelieferten gefunden.
