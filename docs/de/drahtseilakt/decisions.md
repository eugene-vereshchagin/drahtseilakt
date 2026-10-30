# Entscheidungen

Warum der Fork so gebaut ist, wie er gebaut ist. Jeder Eintrag: die Entscheidung, dann die Gründe. Neue
Entscheidungen kommen ans Ende; eine ersetzte Entscheidung wird als ersetzt markiert, nicht gelöscht.

## OpenBallance forken, nicht neu schreiben

Der Upstream-Code ist sauberes C11 (~27 000 Zeilen) mit einer ehrlichen Trennung zwischen portablem Kern und
Host: Der Kern braucht nur `src/platform.h` und `src/render/gpu.h`, die ganze gasm-Anbindung ist eine Datei.
Er führt bereits die originalen Virtools-Graphen aus, setzt die ~180 Building Blocks um, die das Spiel nutzt,
und portiert die Ipion-Physik. Das Schwierige ist erledigt; was fehlt (nativer Host, Auflösungen, Level 13,
Karten), setzt obendrauf auf.

## Die Engine bleibt in C

Funktionierenden Code in einer anderen Sprache neu zu schreiben bringt dem Spieler nichts und kostet Monate.
C ist außerdem die portabelste Wahl für unsere Zielplattformen.

## SDL3 für den Host, wgpu-native für die Grafik

SDL3 ist der De-facto-Standard für Fenster, Eingabe, Gamepads und Ton unter Linux, Windows und macOS und in
den Distributionen paketiert. wgpu-native ist WebGPU als native Bibliothek mit C-Schnittstelle, die auf
Vulkan, Metal und Direct3D 12 läuft; der Renderer spricht bereits WebGPU und WGSL und zieht daher fast
unverändert um.

## Zwei Levelformate, nebeneinander und dauerhaft

Die Engine liest die originalen `.nmo` / `.cmo` und später ein eigenes offenes Format. Die Unterstützung für
`.nmo` ist keine Übergangslösung: Rund 425 Community-Karten gibt es nur darin, viele mit eigenen Skripten.
Das offene Format dient zum Bearbeiten und Bauen von Leveln. Original und konvertierte Kopie desselben Levels
sind zugleich ein Test des Konverters.

## Keine Spieldaten im Repository, weder original noch hochskaliert

Die Rechte an Ballance sind nicht mit Cyparade gestorben: Im Januar 2024 erschien das Spiel neu auf Steam
(Ziggurat Interactive), es wird bei GOG verkauft. Originale Assets und alles, was daraus gemacht ist
(hochskaliert, retuschiert), kommen nicht ins Repository. Neben dem Recht gibt es die Praxis: Eine
DMCA-Meldung bei GitHub sperrt das ganze Repository, nicht einen Ordner. Texturen in höherer Auflösung
entstehen durch eine reproduzierbare Pipeline, die auf der eigenen Kopie des Spielers läuft.

Das Repository ist vorerst privat (30.10.2026), was das Sperrrisiko während der Entwicklung beseitigt; die
Regel bleibt, weil der Code zur Veröffentlichung gedacht ist.

## Eine öffentliche Veröffentlichung hat eigene Inhalte

Eine Veröffentlichung für alle liefert die Engine mit neu gemachten Texturen und Leveln — im Geist des
Originals gezeichnet, nicht aus seinen Pixeln — und nutzt die Daten des Originalspiels, wenn der Spieler eine
Kopie hat (das Freedoom-Modell). Originaltexturen zu retuschieren macht sie nicht zu unseren.

## Simulation mit festem 60-Hz-Takt, interpolierte Darstellung

Die Physik und jede Community-Karte sind auf den originalen 60-Hz-Schritt abgestimmt. Die Simulation taktet
immer mit 60 Hz; nur das Zeichnen folgt dem Bildschirm und interpoliert zwischen den Takten. Ein variabler
Schritt würde das Verhalten der Kugel ändern und Karten kaputt machen.

## Die Kugel wird analytisch gezeichnet

Eine Kugel, die pro Pixel über den Schnitt mit einem Strahl gezeichnet wird, ist bei jeder Auflösung rund,
was ein Mesh nie ist. Welche Objekte als Kugeln gezeichnet werden, sagen die Daten, also ist das eine Fähigkeit
des Renderers und kein Sonderfall.

## Sicherheitsregeln gelten für alles

Der gesamte Code folgt den secure-codegen-Regeln des Inhabers (https://github.com/eugene-vereshchagin/secure-codegen-rules).
Das Hauptgewicht liegt auf den Parsern: Community-Karten sind nicht vertrauenswürdige Dateien von Fremden,
daher wird der `.nmo`-Leser gefuzzt und lehnt fehlerhafte Daten ab, statt abzustürzen.

## Arbeitsablauf

Die Entwicklung läuft mit einem lokalen Agenten auf dem Rechner des Inhabers (dort liegen die Spieldaten);
Änderungen kommen ins Git, nachdem sie dort getestet wurden. CI läuft auf Abruf und bei Pull Requests. Nichts
wird ohne ausdrückliche Anweisung des Inhabers veröffentlicht, getaggt oder publiziert.

## Der Name

*Drahtseilakt* — die Nummer des Seiltänzers und im übertragenen Sinn jeder Balanceakt, bei dem ein Schritt
zur Seite den Absturz bedeutet. Genau das ist eine Kugel auf einer Schiene über dem Abgrund. Deutsch als
Verbeugung vor Cyparade, dem deutschen Studio, das Ballance gemacht hat.

## Dokumentation in mehreren Sprachen

Die Dokumentation wird auf Englisch, Russisch und Deutsch geschrieben. Englisch ist die Hauptversion (die
Sprache von Code, Kommentaren und Commits); die anderen spiegeln deren Struktur unter `docs/<sprache>/` und in
`README.<sprache>.md`. Alle Versionen ändern sich in derselben Änderung. Die Website wird aus `docs/` mit einer
Locale pro Sprache gebaut. (Ersetzt den früheren Eintrag zu zwei Sprachen.)

## Offen: OpenBallance oder Ballanced als Basis

Gefunden, nachdem der Fork angelegt war: [Ballanced](https://github.com/doyaGu/Ballanced) baut die komplette
Virtools-Laufzeitumgebung nativ auf SDL3 nach und erledigt womöglich schon einen Großteil von Phase 1. Die Wahl
zwischen Bleiben, Umziehen und Übernehmen fällt, nachdem Phase 0 es tatsächlich unter Linux gestartet hat, nicht
anhand seines README. Bis dahin bleibt dieser Eintrag offen.

## Optisches Glätten berührt nie die Physik

Gekrümmte Geometrie rund aussehen zu lassen (Phase 3) ist allein Sache des Renderings. Die Kollisionsgeometrie
bleibt exakt die des Originals, weil das Verhalten der Kugel und jede Community-Karte darauf abgestimmt sind.
Welche Glättungstechniken eingesetzt werden, ist offen, bis sie ausprobiert sind; diese Einschränkung nicht.

## Das Repository bleibt privat, bis das Spiel spielbar ist

Nichts wird öffentlich gezeigt — weder Repository noch Website noch Videos —, bis alle Originallevel (die zwölf
und Level 13) nativ spielbar sind. Roher Code wird nicht gezeigt. Veröffentlicht wird auf Anweisung des
Inhabers, wie jedes Release.

## Das Spiel geht nie online

Das fertige Spiel baut keine Netzwerkverbindungen auf: Weder beim Start noch während des Spiels wird etwas
heruntergeladen — keine Schriften, Shader oder andere Ressourcen bei Bedarf, keine Update-Prüfung, keine
Telemetrie. Alles Nötige wird mit dem Spiel ausgeliefert (Schriften samt Lizenzen) oder stammt aus der eigenen
Ballance-Kopie des Spielers. Community-Karten und Mods legt der Spieler selbst ab; das Spiel lädt sie nicht
herunter. Build-Abhängigkeiten sind eine andere Frage: Der Build darf sie herunterladen, in festen Versionen und
mit Prüfsummenabgleich, muss aber auch offline aus lokalen Kopien funktionieren.
