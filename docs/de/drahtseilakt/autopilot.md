# Der Testspieler und seine Hinweise

`--autopilot` lässt einen Testspieler ein Level ohne Fenster spielen: Er findet einen Weg über die Böden und
Transformatoren des Levels und lenkt die Kugel darauf, wobei er jede Wahl der Pfeiltasten an einer Kopie des
Laufs ausprobiert. `--follow FILE` lässt ihn stattdessen dem Weg der Kugel im Replay eines Spielers folgen. Manche
Stellen schafft er allein nicht: Rätsel, Hebel und Schieber, die zuerst getroffen werden müssen, Aufwinde, Schienen
mit Tempo. Dafür liest er **Hinweise**.

## Dem Lauf eines Spielers folgen

Mit `--follow REPLAY` kennt der Testspieler den Lauf des Spielers Tick für Tick: wo die Kugel war, wie schnell,
welche Kugel, in welchem Sektor und welche Tasten gedrückt waren. Wo seine Kugel ist, wo die des Spielers war
(innerhalb 2 Einheiten, die Geschwindigkeit innerhalb 4, dieselbe Kugel, derselbe Sektor, nicht verloren), sind die
Tasten des Spielers bei jeder Entscheidung eine weitere Wahl (in der Ansicht des Spielers genau seine Tasten, auch
Drehungen der Ansicht; in einer anderen dieselben Stöße als ihre Pfeile). Sie wird auf einer Kopie des Laufs bewertet
wie jede andere und immer genommen, wenn sie die Kugel nicht verliert: Der Spieler wusste, was die Wegsuche nicht
weiß (erst einen Hebel treffen, zu einem Transformator zurück, auf einem Aufzug warten), und eine eigene Abkürzung
führt für immer von seiner Linie weg. Wo die Linie des Spielers nicht nah ist (er fiel dort, oder die Kugel ist
abgekommen), lenkt die Wegsuche, und festgefahren probiert sie auch längere Stücke seiner Tasten von nahen Stellen.
Zuerst wird der Tick gesucht, bis zu dem der Lauf seit dem letzten Treffer gekommen ist, damit eine Stelle, an der
der Spieler stand, nicht immer wieder gespielt wird. `AUTOPILOT_DEMO=1` gibt jeden Treffer aus.

## Berichte, wo er steckt

Kommt der Testspieler 20 Sekunden auf seinem Weg nicht weiter, schreibt er einen Bericht nach `--report DIR`
(Standard: `stuck/` im Datenordner des Spiels), ein neuer Ordner je Stelle:

- `report.txt`: Level, Takt, Sektor, Leben, Art, Ort und Tempo der Kugel und das Viertel der Ansicht, der Weg
  voraus, die Transformatoren, losen Körper und Levelobjekte in der Nähe (mit ihren Gruppen);
- `view.png`: was der Spieler sieht; `top.png` und `wide.png`: die Stelle von oben, nah und weit.

Aus einem Bericht, den Replays des Inhabers (`AUTOPILOT_TRACE_FOLLOW=1` gibt den Ort der Kugel im verfolgten Replay
jede halbe Sekunde aus) und Lösungsvideos wird ein Hinweis geschrieben.

## Hinweisdateien

Eine Datei je Level: `hints/<Level>.txt` im Datenordner des Spiels (`hints/Level_01.txt` für Level 1) oder die
Datei, die `--hints FILE` nennt. Die bisher geschriebenen Hinweise liegen im Repository unter `hints/`, mit den
Replays, die sie benutzen; sie enthalten keine Spieldaten. Ein Hinweis je Zeile, `#` beginnt einen Kommentar:

```
at X Y Z r R [sector N] [kind wood|stone|paper] : SCHRITT ; SCHRITT ; ...
```

Kommt die Kugel dem Punkt näher als R (in diesem Sektor, als diese Kugel), laufen die Schritte der Reihe nach,
einmal je Leben; danach findet der Testspieler seinen Ort auf dem Weg neu. Schritte:

| Schritt | Was er tut |
|---|---|
| `goto X Y Z [T]` | zum Punkt rollen und dort halten (höchstens T Takte, 600) |
| `via X Y Z [T]` | durch den Punkt rollen ohne zu halten; kommt sie 1,5 s nicht näher, setzt sie zurück und nimmt Anlauf |
| `push DX DZ T` | T Takte in Weltrichtung (DX, DZ) schieben |
| `air DX DZ T` | in (DX, DZ) schieben, bis die Kugel geflogen und wieder gelandet ist (höchstens T) |
| `wait T` | T Takte ohne Pfeile |
| `turn N` | die Ansicht um N Viertel drehen (negativ: links) |
| `become KIND` | in den nächsten Transformator dieser Art rollen (höchstens 1200 Takte) |
| `replay FILE FROM TO` | die Eingaben eines Spielers in den Takten FROM..TO des Replays FILE (neben der Hinweisdatei), zur Ansicht des Testspielers gedreht |

Richtungen sind Weltrichtungen (X nach Osten, Z nach Norden, wie das Level gebaut ist), ein Hinweis hängt also nicht
von der Drehung der Ansicht ab; die Pfeile werden in jedem Takt zur Ansicht gewählt. Eine Hinweisdatei ist nicht
vertrauenswürdig: ihre Größe (64 KiB), die Zahl der Hinweise (256) und Schritte (64 je Hinweis) und jede Zahl sind
begrenzt, ein Replay ist ein einfacher Dateiname daneben, und eine fehlerhafte Zeile weist die ganze Datei mit ihrer
Zeilennummer ab. `HINTS_TRACE=1` gibt die Kugel alle zehn Takte aus, solange ein Hinweis lenkt.

## Stand

Den Läufen des Inhabers folgend (`--autopilot --follow`), 2026-11-02:

- Level 1: beendet (mit dem Hinweis für die Schieber).
- Level 2: bis Sektor 4 (Hinweise für Sektor 3; die wellige Halfpipe noch nicht).
- Level 3: bis Sektor 4 (Weg 1773 von 2065).
- Level 4: ab Sektor 3 beendet in 5:26.00 mit 4 Leben (dort begann der Lauf des Inhabers).
- Level 5: bis Sektor 3 (Weg 2224 von 3514).
- Level 6: der Lauf des Inhabers endet in Sektor 2 am gewölbten Stamm, der des Testspielers auch.
- Level 7: bis Sektor 3 (Weg 1002 von 3261).
