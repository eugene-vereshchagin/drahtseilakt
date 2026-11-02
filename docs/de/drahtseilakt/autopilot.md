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
der Spieler stand, nicht immer wieder gespielt wird. Die letzten vier Sekunden des Spielers vor einer verlorenen Kugel werden nicht verfolgt, und nach einem Hinweis wartet
die Linie des Spielers zehn Sekunden (der Hinweis kennt die Stelle besser). Festgefahren, wo kein Manöver
weiterkommt, innerhalb 30 Einheiten der Linie, rollt er zu einer früheren Stelle zurück, an der der Spieler stand,
und gibt von dort seine Tasten (ein Anlauf, eine Lücke mit Tempo), wenn das auf einer Kopie des Laufs die Kugel hält
und weiterkommt. `AUTOPILOT_DEMO=1` gibt jeden Treffer aus.

`--then REPLAY` (bis zu 8-mal) hängt einen Lauf an, der an einem Sektor beginnt (ein Replay, das mit `start N`
anfängt): ab diesem Sektor sind der Weg, seine Transformatoren und die Takte des Spielers die dieses Laufs. So wird
ein Level von seinem Anfang verfolgt, mit dem ganzen Lauf des Inhabers und einem späteren, der durch einen Sektor
kam, durch den der ganze Lauf nicht kam.

Versuche eines Spielers, in denen die Kugel verloren ging, fallen aus dem verfolgten Weg heraus: von jedem das Stück
vom Eintritt in den Sektor, in dem sie verloren ging, bis dorthin (die neue Kugel beginnt den Sektor neu). Ein Lauf,
der nicht ins Ziel kam, wird bis zum Ende seines Versuchs im letzten Sektor verfolgt, der am weitesten kam, und von
dort findet der Testspieler den Weg selbst (mit `--plan` durch die Objekte des Plans, an denen der Spieler noch nicht
vorbeikam).

## Die Wegsuche

`--plan FILE` nennt die Objekte des Levels, durch die es geht, der Reihe nach, eines je Zeile (Transformatoren,
Ventilatoren, Kontrollpunkte, `#`-Kommentare), wie ein Spieler, der das Level kennt; ohne Plan werden die
Transformatoren innerhalb 30 vom gefundenen Weg in seiner Reihenfolge genommen. Die nach einem Video geschriebenen
Pläne liegen als `hints/Level_NN.plan` bereit.

Der Weg wird über die Bodenpunkte der Böden, Schienen und Module des Levels gesucht. Außer von Punkt zu Punkt zu
rollen, ein wenig zu steigen und hinabzufallen (höchstens 30 Einheiten):

- **Schienen** halten die Kugel seitlich: auf eine Schiene und von ihr geht ein Weg nur entlang (nicht quer über eine
  kreuzende Schiene, nicht seitlich hinab), fällt nur dort von ihr, wo sie endet, und springt über keine Lücke auf eine.
- **Aufwinde** (P_Modul_18): vom Boden unter einem Ventilator die Säule hinauf bis zur Oberkante seines Volumens und
  hinüber auf einen Boden nahe dieser Höhe (höchstens 14 Einheiten um sie). Nur eine Papierkugel steigt; dafür sorgen
  die Transformatoren auf dem Weg. Eine Kugel, die ein Aufwind hält, wird bewertet, wo sie schwebt, nicht als fallend.
- **Sprünge**: wo kein Weg über die Böden das nächste Ziel erreicht, wird mit Lücken bis 20 Einheiten zu hohem Preis
  erneut gesucht, sodass der Weg die schmalste überquert (eine Zugbrücke zum Umstoßen, ein Teil, auf das man wartet)
  statt durch die Luft zu führen.
- Ein Transformator der Art, die die Kugel schon ist (mit der der Weg ankam), ändert nichts und gilt sofort als passiert.

Steckt er innerhalb 16 einer **Zugbrücke** (P_Modul_25, ein aufrecht stehendes Brett an einem Scharnier), über die
der Weg führt, versucht der Testspieler zuerst einen Anlauf entlang des Wegs (zurück für Schwung, solange sie steht),
wenn der Anlauf auf einer Kopie des Laufs die Kugel behält und weiterkommt: manche Brücken fallen beim Stoß, andere
nur durch einen Hebel.

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
einmal je Leben; danach findet der Testspieler seinen Ort auf dem Weg neu (hinter den Transformatoren, wenn der Hinweis die Kugel verwandelt hat). Schritte:

| Schritt | Was er tut |
|---|---|
| `goto X Y Z [T]` | zum Punkt rollen und dort halten (höchstens T Takte, 600) |
| `via X Y Z [T]` | durch den Punkt rollen ohne zu halten; kommt sie 1,5 s nicht näher, setzt sie zurück und nimmt Anlauf |
| `push DX DZ T` | T Takte in Weltrichtung (DX, DZ) schieben |
| `air DX DZ T` | in (DX, DZ) schieben, bis die Kugel geflogen und wieder gelandet ist (höchstens T) |
| `wait T` | T Takte ohne Pfeile |
| `turn N` | die Ansicht um N Viertel drehen (negativ: links) |
| `become KIND` | in den nächsten Transformator dieser Art rollen (höchstens 1200 Takte) |
| `lead X Y Z D T` | D Einheiten vor dem losen Körper nächst dem Punkt bleiben, während er rollt (hinter ihm bei negativem D), T Takte lang: eine Steinkugel schiebt die Papierkugel durch Ventilatoren, eine Kugel, auf die man wartet |
| `shove X Y Z TX TY TZ T` | den losen Körper (oder das bewegliche Teil eines Moduls) nächst X Y Z nach TX TY TZ schieben: um ihn herum hinter ihn, dann langsam hinein (höchstens T Takte) — ein Brett an seinen Platz geschoben, eine Kugel dorthin gerollt, wo man sie braucht |
| `replay FILE FROM TO` | die Eingaben eines Spielers in den Takten FROM..TO des Replays FILE (neben der Hinweisdatei), zur Ansicht des Testspielers gedreht; beginnt er, während ein Transformator die Kugel hält, startet er, wenn die Kugel losgelassen wird (FROM: der Verwandlungstakt des Spielers + 149), im Takt mit dem Spieler |

Richtungen sind Weltrichtungen (X nach Osten, Z nach Norden, wie das Level gebaut ist), ein Hinweis hängt also nicht
von der Drehung der Ansicht ab; die Pfeile werden in jedem Takt zur Ansicht gewählt. Eine Hinweisdatei ist nicht
vertrauenswürdig: ihre Größe (64 KiB), die Zahl der Hinweise (256) und Schritte (64 je Hinweis) und jede Zahl sind
begrenzt, ein Replay ist ein einfacher Dateiname daneben, und eine fehlerhafte Zeile weist die ganze Datei mit ihrer
Zeilennummer ab. `HINTS_TRACE=1` gibt die Kugel alle zehn Takte aus, solange ein Hinweis lenkt.

## Videos seiner Läufe

`--video FILE` mit `--replay` zeichnet den Lauf ohne Fenster, ein Bild je Takt (60 pro Sekunde) in `--size`, als
rohes RGB nach FILE, und mischt seinen Ton wie im Fenster (nach `DRAHTSEILAKT_SOUND_FILE`); nach dem Ziel geht es sechs
Sekunden weiter für den Endbildschirm. `scripts/video.sh LEVEL REPLAY OUT.mp4 [WxH]` fügt beides mit ffmpeg zu einem
H.264-Video zusammen (die Bilder gehen durch eine Pipe, nichts Großes bleibt liegen; das Spiel selbst startet nie ein
anderes Programm).

## Stand

Den Läufen des Inhabers folgend (`--autopilot --follow`), 2026-11-02:

- Level 1: vom Start an geschafft in 8:00.70 (Hinweise für die Rampe, die Kiste, die Schieber und die Schienen).
- Level 2: vom Start an geschafft in 5:47.25 (mit `--then Level_02-s5.replay`, dem Lauf des Inhabers durch Sektor 5).
- Level 3: vom Start an geschafft in 8:34.07.
- Level 4: ab Sektor 2 beendet in 7:55.15 (`--follow` der Lauf des Inhabers, `--then` der ab Sektor 3; ein Hinweis in den Papier-Transformator am Hang); vom Start bis Sektor 5 (Weg 2347 von 2953).
- Level 5: vom Start an geschafft in 12:03.58 (Hinweise für beide Kisten und den Schieber).
- Level 6: mit dem Lauf des Inhabers (er endet am gewölbten Stamm in Sektor 2) und dem Plan `hints/Level_06.plan`
  geht der Testspieler allein weiter, stößt die Zugbrücke um und kam einmal bis Sektor 3; meist rollt er an der Stelle
  vorbei, wo es vom Stamm hinabgeht (2026-11-03).
- Level 7: bis Sektor 5 (Weg 2635 von 3261): das Brett, auf dem die Papierkugel landet, muss erst als Stein an seinen Platz geschoben werden.
- Level 8–13 (2026-11-03): 8 allein mit dem Plan aus dem Video bis zum ersten Holz-Transformator (Weg 149 von 1118);
  9 dem Lauf des Inhabers folgend bis Sektor 3; 10 dem beendeten Lauf des Inhabers mit `hints/Level_10.txt` folgend
  bis Sektor 2; 11 in Sektor 1 (die Brücken und Schieber vor dem ersten Kontrollpunkt); 12 und 13 bis dorthin, wo die
  Läufe des Inhabers enden (Sektor 1; Sektor 2, wo die Steinkugel an einem Ventilator steht, den sie als Papier
  nehmen muss).
- Videos der beendeten Läufe: `scripts/video.sh`, außerhalb des Repositorys aufbewahrt.
