# Der Testspieler und seine Hinweise

`--autopilot` lässt einen Testspieler ein Level ohne Fenster spielen: Er findet einen Weg über die Böden und
Transformatoren des Levels und lenkt die Kugel darauf, wobei er jede Wahl der Pfeiltasten an einer Kopie des
Laufs ausprobiert. `--follow FILE` lässt ihn stattdessen dem Weg der Kugel im Replay eines Spielers folgen. Manche
Stellen schafft er allein nicht: Rätsel, Hebel und Schieber, die zuerst getroffen werden müssen, Aufwinde, Schienen
mit Tempo. Dafür liest er **Hinweise**.

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

- Level 1: die zwei Schieber in Sektor 3 (Hinweis: über die untere Bahn herum, sie nach Norden schieben); das
  Level wird geschafft.
- Level 2, Sektor 3: die Zugbrücke, die Aufwinde zum Stein-Transformator, der Schieber, die Schiene hinab;
  Sektor 4: die Schiene und die Spiralrampe. Die wellige Halfpipe danach noch nicht: Der Inhaber schwingt quer
  darüber, um das Tempo zu halten, was Wegpunkte nicht tun.
- Level 5, Sektor 1: Zum Hebel P_Modul_37 führt ein schmales Schienenpaar, das mit Tempo genommen werden muss; mit
  acht Pfeilrichtungen rollt die Kugel davon ab. Noch nicht geschafft.
- Level 3 (drehende Arme) und 4 (das Steinkugel-Rätsel): noch keine Hinweise.
