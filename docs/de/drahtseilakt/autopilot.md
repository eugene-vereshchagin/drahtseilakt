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
Ventilatoren, Kontrollpunkte, `#`-Kommentare) oder Orte auf dem Weg als `X Y Z` (welche Schiene zu nehmen ist, ein Punkt, an dem der Weg vorbeimuss), wie ein Spieler, der das Level kennt; ohne Plan werden die
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

## In der Mitte bleiben

Ein Spieler hält sich in der Mitte einer Bahn, nicht an ihrem Rand; der Testspieler auch. Die Böden des Levels
werden einmal je Level auf ihre **Ränder** gelesen: wo es hinter dem Rand eines Bodens nicht gleich weitergeht (ein
Absturz) oder eine Wand auf ihm steht (ihr Fuß); eine Naht zwischen den Meshes zweier Böden ist kein Rand. Jeder
Bodenpunkt weiß, wie weit er vom nächsten Rand auf gleicher Höhe ist und wo die Mitte seines Bodens liegt (der
freieste Punkt in der Nähe: die halbe Breite des Bodens).

- **Die Wegsuche** zahlt für einen Punkt umso mehr, je näher er einem Rand ist, so folgt der Weg der Mitte einer
  Bahn; auf einem schmalen Boden ist seine Mitte der einzige Weg. Eine Schiene ist kein Boden mit Rändern: ihre
  Linie ist der Weg.
- **Das Steuern** bewertet jede auf einer Kopie des Laufs probierte Wahl auch danach, wie weit die Kugel von der
  Mitte ihres Bodens abkommt (und stärker, wo sie einem Rand näher als 3 kommt): weniger, als das Halten der Kugel
  oder Vorankommen wert ist, mehr als ein kleiner Fortschritt. Jeder seit der letzten Entscheidung gedrückte oder
  losgelassene Pfeil kostet auch etwas, so wird die Linie glatt, kein Zickzack. Und sie rollt mit der Geschwindigkeit
  eines sicheren Spielers für ihren Boden: auf einer Schiene oder schmalen Bahn mäßig (6), auf breitem Boden schneller
  (bis 10, mit der Breite des Bodens); schneller kostet, und langsamer als die Hälfte davon auch, wo nichts ein Warten
  verlangt (ein Warten, das das Level verlangt, wird trotzdem genommen: eine Wahl, die die Kugel verliert, ist nie etwas wert). Kurz aufgehalten, wo der Weg von einer Schiene auf einen
  Boden geht oder eine kleine Stufe hinauf (eine Schwelle), setzt sie etwas zurück und nimmt sie mit Anlauf. Dieser
  ruhige Stil gilt für einen eigenen Weg; folgt sie dem Lauf eines Spielers, ist dessen Tempo das Maß.
- **Beim Halten der Linie eines Spielers** darf seine Kugel dort, wo die Kugel des Spielers einem Rand näher als 3
  kam, bis zu 1,5 von dieser Linie zur Mitte hin bleiben, wenn es ebenso schnell ist; sonst ist die Linie die des
  Spielers.

Die Zusammenfassung jedes Laufs endet damit, wie nah den Rändern seine Kugel rollte, über seine Ticks auf einem
Boden (nicht auf Schienen): `edges: mean M, min N, within 1 for P% of T ticks on floors`. Dieselbe Zeile beendet
`--replay FILE --headless`, so lässt sich jeder Lauf, auch der eines Spielers, vergleichen.

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

Zwei Sekunden festgefahren (an einer Lücke, über die der Weg geht, oder am Rand des nächsten Transformators: eine Dreiviertelsekunde, und dann springt er sofort oder rollt hinein), probiert der Testspieler auf Kopien des Laufs Rammstöße, Schaukeln (hin und her entlang
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
einmal je Leben; danach findet der Testspieler seinen Ort auf dem Weg neu (hinter den Transformatoren, wenn der Hinweis die Kugel verwandelt hat: die Transformatoren
des Plans, in die der Hinweis die Kugel rollen ließ, gelten der Reihe nach als durchfahren, auch einer, in dem er begann). Schritte:

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
| `press X Y Z SX SZ V T` | entlang der Geraden vom Ort, an dem der Schritt beginnt, zum Punkt rollen, seitlich an ihr gehalten (SX SZ: ein Versatz quer dazu, höchstens 3: auf welcher Seite eines Schienenpaars), solange langsamer als V; fertig am Punkt, dahinter oder nach T Takten — Schienen, auf der oberen lehnend gefahren, ein gleichmäßiger Anstieg |
| `round X Z R V T [Y]` | um die senkrechte Achse durch (X, Z) im Radius R mit der Geschwindigkeit V rollen (in eine Richtung, bei negativem V in die andere), T Takte lang oder bis die Kugel über Y ist — eine Schienenspirale hinauf |
| `along V T` | in die Richtung schieben, in die die Kugel rollt, nicht schneller als V, T Takte lang — Schienen, die sie führen |
| `pass X Y Z V [T]` | durch den Punkt rollen, ohne anzuhalten, nicht schneller als V (eine Kurve, ein Schienenende, ein Transformator unterwegs); an der Linie vom Punkt gehalten, durch den das letzte `pass` des Hinweises ging (eine Kette davon folgt Schienen und Bahnen) |
| `cross DX DZ X Y Z R` | über ein bewegliches Teil (eine Schaukel) zum Punkt: der Moment und die Länge des Stoßes entlang DX DZ werden auf Kopien des Laufs von hier gefunden (warten, stoßen, tragen lassen, abstoßen), der erste, nach dem die Kugel innerhalb R vom Punkt auf einem Boden ist, und dann gespielt |
| `hold X Y Z T` | die Kugel T Takte auf dem Punkt halten — still, wo gewartet werden muss (auf einem Hang, oder nach schneller Ankunft) |
| `await X Y Z QX QY QZ D T` | die Kugel an Ort und Stelle halten, bis der lose Körper oder das bewegliche Modulteil nächst X Y Z innerhalb D von QX QY QZ ist (höchstens T Takte) — eine Schaukel oder ein Aufzug im richtigen Moment |
| `ride X Y Z QX QY QZ D T` | die Kugel über dem Körper oder Modulteil nächst X Y Z halten, bis es innerhalb D von QX QY QZ ist (höchstens T Takte) |
| `replay FILE FROM TO` | die Eingaben eines Spielers in den Takten FROM..TO des Replays FILE (neben der Hinweisdatei), zur Ansicht des Testspielers gedreht; beginnt er, während ein Transformator die Kugel hält, startet er, wenn die Kugel losgelassen wird (FROM: der Verwandlungstakt des Spielers + 149), im Takt mit dem Spieler |

Richtungen sind Weltrichtungen (X nach Osten, Z nach Norden, wie das Level gebaut ist), ein Hinweis hängt also nicht
von der Drehung der Ansicht ab; die Pfeile werden in jedem Takt zur Ansicht gewählt. Eine Hinweisdatei ist nicht
vertrauenswürdig: ihre Größe (64 KiB), die Zahl der Hinweise (256) und Schritte (64 je Hinweis) und jede Zahl sind
begrenzt, ein Replay ist ein einfacher Dateiname daneben, und eine fehlerhafte Zeile weist die ganze Datei mit ihrer
Zeilennummer ab. `HINTS_TRACE=1` gibt die Kugel alle zehn Takte aus, solange ein Hinweis lenkt.

## Ihm beim Spielen zusehen

`--watch` (mit `--level N`, `--open DIR` oder `--sector N` und mit `--follow`, `--then`, `--plan`, `--hints` wie
bei `--autopilot`) lässt den Testspieler im Fenster des Spiels spielen: seine Eingaben steuern das Spiel in jedem
Tick statt der Tastatur. Seine Entscheidungen (Versuche auf Kopien des Laufs, Suchen) fallen zwischen den Bildern
auf einem eigenen Thread; dauert eine länger, steht das Bild und zeigt „thinking“. Ein Zurückgehen zu einem
gespeicherten Stand springt sichtbar dorthin, und die Zeile sagt, was misslang und wohin zurück. Der Stand oben
links: Tempo, Zeit, Sektor, Leben, wie weit die Kugel vom Rand ihres Bodens ist, wie oft er zurückging, was steuert
(die Linie des Spielers, die Wegsuche, ein Hinweis und sein Schritt, ein Manöver oder die Strahlsuche, wenn er
feststeckt), der erreichte Wegpunkt und der nächste Transformator. Die Tasten stehen in einem Kasten unten links in
der Sprache der Menüs (der eingeschaltete Zustand ist hervorgehoben):

| Taste | Was sie tut |
|---|---|
| Leertaste | anhalten, weiter |
| + / − | schneller, langsamer (¼× bis 8×; nur wie viele Ticks ein Bild: der Lauf bleibt derselbe) |
| F5 | selbst steuern: die Pfeile steuern, der Testspieler schaut zu; wieder F5 gibt ihm die Kugel zurück, wo sie gerade ist. Die Strecke wird neben den Hinweisen als `<level>-hand-<tick>.replay` gespeichert (der Lauf bis dahin, so spielt er genau nach), und eine Hinweiszeile, die sie abspielt (`at … : replay <Datei> FROM TO`), kommt in `<level>-hand.txt`; beide werden am Bildschirm genannt. In die Hinweisdatei des Levels kopiert, spielt der Testspieler sie dort beim nächsten Mal. Zu den Ständen davor geht er nicht mehr zurück |
| F6 | den Schritt des laufenden Hinweises überspringen |
| F7 | ein Bericht über diese Stelle (wie bei einem toten Lauf, nach `--report`) |
| F8 | eine Zone „hier nicht trödeln“ markieren (flow; siehe „Zonen“ unten): kurz getippt markiert die Taste den Weg der Kugel über die letzten 3 Sekunden bis zum Tippen; gehalten läuft die Zone weiter, solange die Taste unten ist (ab 3 Sekunden, bevor sie gedrückt wurde), und endet beim Loslassen |
| F4 | ebenso eine Zone „hier das Tempolimit halten und keine Kurven schneiden“ (careful) markieren |
| [ / ] | wie weit eine Markierung zurückreicht: 1 bis 10 Sekunden (3) |
| Rücktaste | die zuletzt markierte Zone zurücknehmen (während eine Taste gehalten wird: diese Markierung verwerfen) |
| F12 | die markierten Zonen in der Welt zeigen oder ausblenden, als durchscheinende Perlen entlang ihres Weges (flow grün, careful orange) |
| F1 | den Tastenkasten aus- oder einblenden |
| Esc | das Pausenmenü (Level verlassen, beenden) |

Am Ende, oder wenn das Fenster geschlossen wird, wird die Zusammenfassung des Laufs ausgegeben und der ganze Lauf,
auch die von Hand gesteuerten Strecken, unter `--record` gespeichert.

## Zonen

Der Zuschauer bringt dem Testspieler beim Zusehen (`--watch`) etwas bei: eine Taste markiert die Strecke, die die
Kugel gerade gerollt ist, als Zone, und der Testspieler hält sich von da an daran, in diesem Lauf und jedem späteren,
auch ohne Fenster. Eine Markierung reicht zurück (3 Sekunden, `[` und `]` ändern das), denn erst muss man bemerken,
dass die Kugel falsch rollt: F8 oder F4 kurz getippt markiert den Weg der Kugel über die letzten Sekunden bis zum
Tippen; gehalten läuft die Zone weiter, solange die Taste unten ist, und endet beim Loslassen, so bleibt nie eine
Zone offen. Angehalten (Leertaste) markiert ein Tippen die letzten Sekunden vor der Pause. Die gerade markierte
Strecke leuchtet ein paar Sekunden auf, eine Zone im Markieren ist zu sehen, während sie wächst, und die Statuszeile
nennt die Zone, in der die Kugel ist.

Die Zonen werden sofort geschrieben, eine Zeile je Zone, nach `<level>.zones` neben der Hinweisdatei des Levels (mit
`--hints hints/Level_01.txt`: `hints/Level_01.zones`), die jeder Lauf des Levels liest:

```
flow sector 1 r 4 : X Y Z ; X Y Z ; ...
careful sector 2 r 4 [speed V] : X Y Z ; ...
```

Die Punkte sind der Weg der Kugel durch die Strecke (ein Punkt: eine Stelle); die Zone ist alles innerhalb `r` von
diesem Weg in diesem Sektor.

- **flow** — hier nicht trödeln. Ein Hinweisschritt, der die Kugel anhält, läuft ohne Halt: ein `goto` wird
  durchrollt, ein `wait` oder `hold` ausgelassen, ein `pass` oder `press` langsamer als 5 mit 5 gefahren; aber nur,
  wenn auf einer Kopie des Laufs der Rest des Hinweises und eine Sekunde danach die Kugel behalten (ein Halt, den die
  Kugel braucht, bleibt). Die Steuerung zahlt für Rollen langsamer als 0,8 des Tempos des Bodens mehr, als ein
  Warten wert ist (eine Wahl, die die Kugel verliert, wird nie genommen).
- **careful** — hier das Tempolimit halten und keine Kurven schneiden. Nicht schneller als `speed` (4,5 ohne Angabe;
  zum Ändern die Zeile bearbeiten): die Steuerung zahlt für jede Einheit darüber viermal so viel, `pass`, `press`,
  `along` und `round` eines Hinweises werden darauf begrenzt, und was auch steuert (die Steuerung, ein Hinweis, die
  Linie des Spielers), die Pfeile in Laufrichtung der Kugel werden über dem Limit losgelassen und über dem 1,25-Fachen
  wird gebremst. Die Linie des Weges voraus (die Mitte, die die Wegsuche fand) wird gehalten: jede Einheit daneben
  kostet 2, neben der Mitte des Bodens dreimal so viel wie sonst. Wo das Level Tempo braucht (ein Anstieg, eine
  Lücke), bremst eine careful-Zone die Kugel auch: dort gehört keine hin.

Eine Zonendatei ist nicht vertrauenswürdig wie eine Hinweisdatei: ihre Größe (64 KiB), die Zonen (256) und Punkte
(256 je Zone) und jede Zahl sind begrenzt; eine fehlerhafte Zeile wird mit einem Vermerk ausgelassen, die anderen
Zonen gelten.

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

- Level 1: vom Start an geschafft in 6:12.27 (Hinweise für Rampe, Kiste, Schieber und Schienen).
  Allein ohne Lauf eines Spielers, nach dem Storyboard des Walkthroughs (`hints/Level_01.plan`, `.route`, `.txt`;
  die Hinweise zum Lauf des Inhabers heißen jetzt `Level_01-follow.txt`): 5:17.12, 9.1% der Zeit stehend
  (`hints/runs/Level_01-clean.replay`).
- Level 2: vom Start an geschafft in 6:13.27 (mit `--then Level_02-s5.replay`, dem Lauf des Inhabers ab Sektor 5).
  Allein ohne Lauf eines Spielers, nach dem Storyboard des Walkthroughs (`hints/Level_02.plan`, `.route`, `.txt`;
  die Hinweise zum Lauf des Inhabers heißen jetzt `Level_02-follow.txt`): 5:23.90, 5.9% der Zeit stehend
  (`hints/runs/Level_02-clean.replay`).
- Level 3: vom Start an geschafft in 7:45.48 (Hebel und Brücke in Sektor 4 wie beim Inhaber).
  Allein ohne Lauf eines Spielers, nach dem Storyboard des Walkthroughs (`hints/Level_03.plan`, `.route`, `.txt`;
  die Hinweise zum Lauf des Inhabers heißen jetzt `Level_03-follow.txt`): 5:58.97, 5.5% der Zeit stehend
  (`hints/runs/Level_03-clean.replay`).
- Level 4: vom Start an geschafft in 9:19.47 (`--follow` der Lauf des Inhabers, `--then` der ab Sektor 3).
- Level 5: vom Start an geschafft in 12:39.77 (Hinweise für beide Kisten und den Schieber).
- Level 6: vom Start an beendet in 8:01.22: Sektor 1 entlang der Linie des Inhabers, Sektoren 2–5 allein mit dem
  Plan und den Hinweisen nach dem Storyboard des Videos (`hints/Level_06.txt`: der wellige Stamm als Holz hin und als
  Stein schnell zurück, die Kisten heruntergerammt, der Stamm nach dem Sturz in Sektor 5).
- Level 7: vom Start an geschafft in 12:04.28, dem geschafften Lauf des Inhabers folgend.
- Level 8: vom Start an beendet, ganz ohne Lauf eines Spielers: allein durch den Plan aus dem Video
  (`hints/Level_08.plan`, mit Orten auf dem Weg) und die nach seinem Storyboard geschriebenen Hinweise
  (`hints/Level_08.txt`: Schienenpaare, auf der oberen lehnend gefahren, eine Spirale, eine Schaukel, die Zielscheiben,
  die die Brücken des Labyrinths senken, weggerammte Kisten).
- Level 9: vom Start an geschafft in 12:35.27, dem geschafften Lauf des Inhabers folgend (das Labyrinth der Schieber
  in Sektor 3 wird gelöst, wie der Inhaber es löste).
- Level 10: vom Start an geschafft in 15:31.00, dem geschafften Lauf des Inhabers folgend, mit `hints/Level_10.txt`.
- Level 11: vom Start an in 13:43.73 beendet, ganz ohne Lauf eines Spielers, allein nach dem Plan aus dem
  Walkthrough und den Hinweisen aus seinem Storyboard (`hints/Level_11.plan`, `.txt`, `.route`): die Bretter des
  Burg-Labyrinths umgestoßen, eine lose Steinkugel auf die Schienen gestoßen, die einen Schieber treiben, die
  Zielscheiben, die Kisten aus den Rinnen gestoßen, das Labyrinth von Sektor 4, die Ventilatoren und schwingenden
  Kisten von Sektor 5, und in Sektor 6 zwei lose Steinkugeln vom Gitter gestoßen (eine schiebt den Schieber zwischen
  den Wippen, eine öffnet eine Falltür) und die letzten Schienenpaare.
- Level 12–13: 12 in Sektor 2 (seine dünnen Schienen); 13 in Sektor 2, am Papier-Transformator vorbei und den
  Ventilator hinauf, am Balken, an dem die Ventilatoren 18_10–12 entlangblasen.
- Videos der beendeten Läufe: `scripts/video.sh`, außerhalb des Repositorys aufbewahrt.
- Unsere zehn Level (levels.md, „Unsere Level und wie man sie bearbeitet“), 2026-11-03: jedes vom Start an in drei
  Stilen geschafft, mit den Hinweisdateien, die ihre Beschreibungen schreiben (`assets/levels/<Level>/pilot/`).
