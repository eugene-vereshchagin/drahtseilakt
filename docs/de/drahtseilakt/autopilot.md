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

**An der Linie des Spielers bleiben.** Sobald seine Kugel dort ist, wo die des Spielers war, ist der Lauf des Spielers
eine Linie in der Zeit: bei jeder Entscheidung (jede Zehntelsekunde) wird die Eingabe für das nächste Zehntel so
gewählt, dass seine Kugel die nächste halbe Sekunde der des Spielers am nächsten bleibt (dem Ort, ein wenig auch dem
Tempo): die Tasten des Spielers selbst oder jede Pfeilwahl für ein Zehntel und danach die Tasten des Spielers, alle
auf Kopien des Laufs probiert. Ein Ausrutscher wird sofort korrigiert, und die Stöße eines Rätsels (Blöcke in einem
Labyrinth aus Bahnen, ein Hebel, ein Brett) geschehen, wie der Spieler sie machte. Wo seine Kugel mehr als 4
Einheiten von der des Spielers entfernt ist oder der Spieler dort seine Kugel verliert, wird die Linie weiter vorn
wieder aufgenommen, wo die Kugel des Spielers nahe vorbeikommt (nach einer verlorenen Kugel: der Weg der neuen zurück
an dieser Stelle vorbei), sonst steuert die Wegsuche. Ein Hinweis für die Stelle kommt trotzdem zuerst (er wurde
dort geschrieben, wo die Linie des Spielers nicht reichte). Die Linie wird auch in den letzten Sekunden des Spielers
vor einer verlorenen Kugel gehalten, bis auf die letzte: was der Spieler dann tat, kann das sein, was das Level
braucht (eine Brücke, die mit der fallenden Kugel umgestoßen wurde). Aus dem verfolgten Weg fallen auch die letzten vier Sekunden des Spielers vor
einer verlorenen Kugel heraus (die Kugel rollte schon ab), ein nicht geschaffter Lauf wird also bis dorthin verfolgt,
wo der Spieler die Kugel noch hatte.

`--then REPLAY` (bis zu 8-mal) hängt einen Lauf an, der an einem Sektor beginnt (ein Replay, das mit `start N`
anfängt): ab diesem Sektor sind der Weg, seine Transformatoren und die Takte des Spielers die dieses Laufs. So wird
ein Level von seinem Anfang verfolgt, mit dem ganzen Lauf des Inhabers und einem späteren, der durch einen Sektor
kam, durch den der ganze Lauf nicht kam.

Versuche eines Spielers, in denen die Kugel verloren ging, fallen aus dem verfolgten Weg heraus: von jedem das Stück
vom Eintritt in den Sektor, in dem sie verloren ging, bis dorthin (die neue Kugel beginnt den Sektor neu). Ein Lauf,
der nicht ins Ziel kam, wird bis zum Ende seines Versuchs im letzten Sektor verfolgt, der am weitesten kam, aber nicht
weiter als kurz hinter seinen letzten Transformator dort (danach suchte der Spieler den Weg und kann dorthin gerollt
sein, wo eine Kugel festsitzt: eine Steinkugel in die Grube eines Ventilators; dort endet auch die Linie des
Spielers), und von dort findet der Testspieler den Weg selbst (mit `--plan` durch die Objekte des Plans, an denen der Spieler noch nicht
vorbeikam).

## Die Wegsuche

`--plan FILE` nennt die Objekte des Levels, durch die es geht, der Reihe nach, eines je Zeile (Transformatoren,
Ventilatoren, Kontrollpunkte, `#`-Kommentare), wie ein Spieler, der das Level kennt; ohne Plan werden die
Transformatoren innerhalb 30 vom gefundenen Weg in seiner Reihenfolge genommen. Die nach einem Video geschriebenen
Pläne liegen als `hints/Level_NN.plan` bereit.

Der Weg wird über die Bodenpunkte der Böden, Schienen und Module des Levels gesucht. Außer von Punkt zu Punkt zu
rollen, ein wenig zu steigen (eine Stufe von 1,2; wo der Testspieler den Weg selbst findet, auch ein glatter Hang bis 47
Grad, ein welliger Stamm: ein Anstieg über eine Stufe hinaus braucht einen Bodenpunkt auf halber Höhe, den eine
Kante nicht hat) und
hinabzufallen (höchstens 30 Einheiten):

- **Schienen** halten die Kugel seitlich: auf eine Schiene und von ihr geht ein Weg nur entlang (nicht quer über eine
  kreuzende Schiene, nicht seitlich hinab), fällt nur dort von ihr, wo sie endet, und springt über keine Lücke auf eine.
- **Aufwinde** (P_Modul_18): vom Boden unter einem Ventilator die Säule hinauf bis zur Oberkante seines Volumens und
  hinüber auf einen Boden nahe dieser Höhe (höchstens 14 Einheiten um sie). Nur eine Papierkugel steigt; dafür sorgen
  die Transformatoren auf dem Weg. Eine Kugel, die ein Aufwind hält, wird bewertet, wo sie schwebt, nicht als fallend.
- **Sprünge**: wo kein Weg über die Böden das nächste Ziel erreicht, wird mit Lücken bis 20 Einheiten zu hohem Preis
  erneut gesucht, sodass der Weg die schmalste überquert (eine Zugbrücke zum Umstoßen, ein Teil, auf das man wartet)
  statt durch die Luft zu führen.
- Die Transformatoren, die nicht auf dem Weg liegen (nicht im Plan oder nicht nahe dem gefundenen Weg), umgeht der
  Weg, wo der Umweg nicht viel länger ist: über einen zu rollen verwandelt die Kugel.
- Ein Transformator der Art, die die Kugel schon ist (mit der der Weg ankam), ändert nichts und gilt sofort als passiert.

Steckt er innerhalb 16 einer **Zugbrücke** (P_Modul_25, ein aufrecht stehendes Brett an einem Scharnier), über die
der Weg führt, versucht der Testspieler zuerst einen Anlauf entlang des Wegs (zurück für Schwung, solange sie steht),
wenn der Anlauf auf einer Kopie des Laufs die Kugel behält und weiterkommt: manche Brücken fallen beim Stoß, andere
nur durch einen Hebel. Zuerst geradewegs hindurch; dann Rammstöße aus einem Anlauf von 8 oder 14 entlang des Wegs,
gleich nach dem Stoß gebremst (um einem Brett, das in die Lücke fällt, nicht zu folgen), ein Warten, bis es liegt,
und hinüber.

## Zurück und noch einmal

Die Simulation ist deterministisch und lässt sich kopieren, also speichert der Testspieler seinen Lauf alle zwei
Sekunden (die Stände werden mit dem Alter ausgedünnt und reichen einige Minuten zurück) und geht,
wenn die Kugel verloren ist, ein anderer Transformator als der nächste des Wegs sie verwandelt hat,
versehentlich überrollt (abseits der Linie des Spielers; eine Wahl, die auf einer Kopie die Kugel so verwandelt,
gilt als Verlust der Kugel), oder sein Weg 20 Sekunden nicht weiterkommt, zu einem Stand zurück und versucht es
abgewandelt noch einmal: ein kurzer eigener erster Zug (ein Warten oder ein Stoß zufälliger Länge, vor allem anderen,
auch einem laufenden Hinweis, sodass bewegte Teile in einer anderen Phase getroffen werden) und für einige Sekunden
zufällige Zuschläge auf die Bewertungen seiner Wahlen; ab dem dritten Versuch verlässt jeder dritte die Linie des
Spielers. Drei Versuche gehen vom letzten Stand vor dem Fehlschlag aus, dann je drei von einem doppelt so weit
zurückliegenden (die Ursache kann früher liegen: ein zu langsamer Anlauf, eine schlechte Linie in eine Kurve). Nach 30
Versuchen an einer Stelle (eine Stelle: 6 Wegpunkte weiter als die letzte) wird der Fehlschlag hingenommen: die Kugel
geht verloren, oder der Lauf wird mit seinem Bericht aufgegeben (ein so hingenommener fremder Transformator: die Kugel
fährt in einen, der die Art des Wegs macht, wie eine neue Kugel). Die Hinweise können nach dem Zurückgehen wieder
auslösen, wie für eine neue Kugel. Eine Kugel, die verloren geht, wie der Spieler dort seine verlor (innerhalb vier
Sekunden, nachdem er sich an die letzten Sekunden des Spielers vor einem Verlust hielt), führt nicht zurück, wo der Lauf des Spielers danach über diesen Sektor
hinauskam: was die
Kugel des Spielers im Gehen tat, kann das sein, was das Level braucht (eine Zugbrücke, mit der Kugel umgestoßen). `--rewinds N` begrenzt, wie oft er in einem Lauf zurückgeht (600; 0: nie). Die
Abwandlung hat einen festen Startwert, ein Lauf ist also reproduzierbar; behalten wird der Lauf (und seine
Aufzeichnung), der durchkam, ohne die fehlgeschlagenen Versuche.

## Die Suche, wenn er feststeckt

Vier Sekunden festgefahren, probiert der Testspieler auf Kopien des Laufs Rammstöße, Schaukeln (hin und her entlang
einer Wahl, zwei- oder viermal: aus einer Mulde zwischen Wellen, zu steil, um aus dem Stand hinaufzukommen) und
zufällige Manöver. Festgefahren, wo weder Rammen noch Manöver, Brückensturm oder Sprung weiterkommen, sucht der Testspieler tiefer: eine
Strahlsuche über Folgen gehaltener Pfeile (jeder eine Drittel-, Dreiviertel- oder anderthalb Sekunden lang, oder eine
halbe Sekunde ohne Pfeile), vier tief, mit den sechs besten Zuständen je Tiefe, einer je Ort. Jeder wird nach
zwei Dritteln einer Sekunde Ausrollen danach bewertet, wie weit die Kugel auf dem Weg ist, abzüglich dessen, was vorn
im Weg steht: lose Kisten und Kugeln sowie bewegte Teile der Module über dem Weg (der Block eines Schiebers in der
Bahn, ein aufgestelltes Brett). Eine verlorene Kugel beendet eine Folge. Die beste, die weiterkommt, wird gehalten.
Mit `AUTOPILOT_PARALLEL=1` laufen die Kopien jeder Entscheidung und der Suche nebeneinander in Threads; die
Reihenfolge der Ergebnisse ist fest, der Lauf also derselbe.

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

Den Läufen des Inhabers folgend (`--autopilot --follow`), mit Zurückgehen und Halten der Linie des Spielers,
2026-11-03 (die geschafften Läufe liegen in `hints/runs/`, ihre Videos außerhalb des Repositorys, gemacht mit
`scripts/video.sh`):

- Level 1: vom Start an geschafft in 6:19.50 (Hinweise für Rampe, Kiste, Schieber und Schienen).
- Level 2: vom Start an geschafft in 5:44.53 (mit `--then Level_02-s5.replay`, dem Lauf des Inhabers ab Sektor 5).
- Level 3: vom Start an geschafft in 7:32.08 (Hebel und Brücke in Sektor 4 wie beim Inhaber).
- Level 4: vom Start an geschafft in 9:16.25 (`--follow` der Lauf des Inhabers, `--then` der ab Sektor 3).
- Level 5: vom Start an geschafft in 13:06.93 (Hinweise für beide Kisten und den Schieber).
- Level 6: mit dem Lauf des Inhabers (er endet am gewölbten Baumstamm in Sektor 2) und dem Plan
  `hints/Level_06.plan` kommt der Testspieler allein durch Sektor 2 und 3 und über den welligen Stamm von Sektor 4
  bis zu seinem Stein-Transformator; als Nächstes sind die Kisten als Stein vom Absatz zu stoßen.
- Level 7: vom Start an geschafft in 11:47.53, dem geschafften Lauf des Inhabers folgend.
- Level 8: allein mit dem Plan aus dem Video bis zum ersten Holz-Transformator (Weg 156 von 1072).
- Level 9: vom Start an geschafft in 12:30.10, dem geschafften Lauf des Inhabers folgend (das Labyrinth der Schieber
  in Sektor 3 wird gelöst, wie der Inhaber es löste).
- Level 10: vom Start an geschafft in 14:42.52, dem geschafften Lauf des Inhabers folgend, mit `hints/Level_10.txt`.
- Level 11–13: 11 folgt dem Lauf des Inhabers durch Sektor 1–3 (verliert an der Zugbrücke eine Kugel wie der
  Inhaber, was sie umstößt) und fährt in Sektor 4 allein weiter; 12 in Sektor 2
  (seine dünnen Schienen); 13 in Sektor 2, am Papier-Transformator vorbei und den Ventilator hinauf, am
  Balken, an dem die Ventilatoren 18_10–12 entlangblasen.
