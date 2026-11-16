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
fährt in einen, der die Art des Wegs macht, wie eine neue Kugel). Früher an einem Fleck: ist ein Versuch von dort so weit zurückgegangen, wie die Speicherstände
reichen, wird das dritte weitere Stehenbleiben dort (innerhalb von 2 Einheiten) oder der sechste weitere Verlust der
Kugel (innerhalb von 12) hingenommen (was dort scheitert, entschied sich früher, als ein Zurückgehen reicht); bis dahin
gehen die Versuche weiter zurück, und ein hingenommener Fehlschlag löscht seinen Fleck (die nächste Kugel kommt mit
eigener Vorgeschichte dorthin). Die Hinweise können nach dem Zurückgehen wieder
auslösen, wie für eine neue Kugel. Ein Hinweis, dessen Kugel einen Schritt 3 oder mehr unter dessen Punkt beendet
(auf einen Boden darunter gefallen), ist vom Kurs ab: seine weiteren Schritte zählen nicht als Vorankommen, also
geht der Testspieler von dort zurück wie von einem Weg, der steht, statt den Hinweis vom falschen Boden zu Ende zu
spielen. Eine Kugel, die verloren geht, wie der Spieler dort seine verlor (innerhalb vier
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
at X Y Z r R [sector N] [kind wood|stone|paper|sticky] [retry N] : SCHRITT ; SCHRITT ; ...
```

`retry N` (1 bis 50) markiert einen Hinweis, dessen Ausgang an Unterschieden hängt, die zu klein sind, um Schritte
dagegen zu schreiben (eine Kiste auf eine andere gerammt, ein bewegtes Teil in seiner Phase getroffen): ein Spieler
versucht so eine Stelle noch einmal. Bevor der Hinweis auslöst, wird der Lauf gesichert; scheitert er (die Kugel
verloren, stehend oder vom Kurs ab, während er läuft, oder ein `check` nicht erfüllt), geht der Lauf zu dieser
Sicherung zurück und der Hinweis läuft erneut, variiert und geseedet (reproduzierbar): eine Startverzögerung bis 3
Takte je Versuch (höchstens 15) ohne Pfeile, `pass`- und `press`-Punkte bis 0.3 quer verschoben, Geschwindigkeiten
bis 6% schneller oder langsamer, Stöße bis 3 Takte länger oder kürzer, die Verstärkung eines `trace` bis 15% daneben und sein aufgezeichneter Weg quer verschoben wie die Punkte (ein geregelter `trace` zöge sonst jeden Versuch auf dieselbe Linie zurück)
(die volle Breite ab dem vierten Versuch). Jeder Versuch zählt gegen `--rewinds` (0: keiner), der behaltene Lauf
enthält nur den gelungenen Versuch, und nach N erneuten Versuchen wird das Scheitern hingenommen. Die
Zusammenfassung nennt die Versuche jedes solchen Hinweises (`retries: line L: N tries`). Als Schritt geschrieben tut `retry N`
dasselbe von diesem Schritt an (der Lauf gesichert, wenn der nächste Schritt beginnt; variiert werden die Schritte
danach, die Verzögerung kommt vor den ersten davon). Hinweise ohne die Marke laufen wie bisher.

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
| `shove X Y Z TX TY TZ T [R [V]]` | den losen Körper (oder das bewegliche Teil eines Moduls) nächst X Y Z nach TX TY TZ schieben: um ihn herum hinter ihn, R von seiner Mitte (3,5, die Größe einer Kugel; mehr für eine Kiste, höchstens 10), dann langsam hinein (höchstens T Takte), mit V nur, solange er langsamer als V ist (eine Kiste auf einem Gefälle rutscht von selbst weiter) — ein Brett an seinen Platz geschoben, eine Kugel dorthin gerollt, wo man sie braucht, eine Kiste unter eine Schiene gesetzt |
| `ram X Y Z DX DZ B V T [S]` | den losen Körper (oder das bewegliche Teil eines Moduls) nächst X Y Z entlang der Weltrichtung (DX, DZ) durch seine Mitte rammen (S Einheiten links davon, entlang des Stoßes gesehen, höchstens 3): zum Anlaufpunkt B Einheiten hinter dem Körper, wo er jetzt ist (2 bis 30; um ihn herum, wenn die Kugel vor oder neben ihm ist), oder auf die Linie durch ihn, mindestens halb so weit dahinter, dann durch ihn hindurch nicht schneller als V für T Ticks des Stoßes (oder bis die Kugel an ihm stehen bleibt), auf der Linie durch seine Mitte gehalten, während er sich bewegt: eine Kiste in ihre Nische gestoßen, wo auch immer Kugel und Kiste gerade sind, nicht nach festen Koordinaten |
| `press X Y Z SX SZ V T` | entlang der Geraden vom Ort, an dem der Schritt beginnt, zum Punkt rollen, seitlich an ihr gehalten (SX SZ: ein Versatz quer dazu, höchstens 3: auf welcher Seite eines Schienenpaars), solange langsamer als V; fertig am Punkt, dahinter oder nach T Takten — Schienen, auf der oberen lehnend gefahren, ein gleichmäßiger Anstieg |
| `pass X Y Z V [T]` | durch den Punkt rollen, ohne anzuhalten, nicht schneller als V (eine Kurve, ein Schienenende, ein Transformator unterwegs); an der Linie vom Punkt gehalten, durch den das letzte `pass` des Hinweises ging (eine Kette davon folgt Schienen und Bahnen) |
| `cross DX DZ X Y Z R` | über ein bewegliches Teil (eine Schaukel) zum Punkt: der Moment und die Länge des Stoßes entlang DX DZ werden auf Kopien des Laufs von hier gefunden (warten, stoßen, tragen lassen, abstoßen), der erste, nach dem die Kugel innerhalb R vom Punkt auf einem Boden ist, und dann gespielt |
| `hold X Y Z T` | die Kugel T Takte auf dem Punkt halten — still, wo gewartet werden muss (auf einem Hang, oder nach schneller Ankunft) |
| `along V T` | in die Richtung schieben, in die die Kugel rollt, nicht schneller als V, T Takte lang — Schienen, die sie führen |
| `round X Z R V T [Y]` | um die senkrechte Achse durch (X, Z) im Radius R mit der Geschwindigkeit V rollen (in eine Richtung, bei negativem V in die andere), T Takte lang oder bis die Kugel über Y ist — eine Schienenspirale hinauf |
| `await X Y Z QX QY QZ D T` | die Kugel an Ort und Stelle halten, bis der lose Körper oder das bewegliche Modulteil nächst X Y Z innerhalb D von QX QY QZ ist (höchstens T Takte) — eine Schaukel oder ein Aufzug im richtigen Moment |
| `ride X Y Z QX QY QZ D T [A]` | die Kugel über dem Körper oder Modulteil nächst X Y Z halten, bis es innerhalb D von QX QY QZ ist (höchstens T Takte); mit A um A Einheiten vor seiner Mitte in seiner Fahrtrichtung (dahinter, negativ; höchstens 4): vorn auf einer Kugel, die Schienen hinabrollt, nicht oben auf ihr |
| `replay FILE FROM TO` | die Eingaben eines Spielers in den Takten FROM..TO des Replays FILE (neben der Hinweisdatei), zur Ansicht des Testspielers gedreht; beginnt er, während ein Transformator die Kugel hält, startet er, wenn die Kugel losgelassen wird (FROM: der Verwandlungstakt des Spielers + 149), im Takt mit dem Spieler |
| `sync X Y Z [T]` | die Kugel auf dem Punkt halten, bis die losen Objekte und Module des Sektors eingesetzt sind (der Sektor eines Checkpoints beginnt 48 Takte danach; höchstens T, 600): der Zustand, mit dem ein Lauf `--sector N` beginnt |
| `until X Y Z N [T]` | die Kugel auf dem Punkt halten, bis die Uhr des Sektors (die Takte, seit seine Module in die Physik kamen: an seinem Checkpoint, bei einem Start mit `--sector N` oder neu nach einer verlorenen Kugel) N erreicht (höchstens T Takte, 1800): ein Rätsel im selben Augenblick des Sektors begonnen, seine beweglichen Dinge, wo sie waren, ob der Lauf am Checkpoint begann oder aus dem Sektor davor kam |
| `check X Y Z D [T]` | die Erfolgsbedingung des Hinweises: die Kugel jetzt oder, ohne Pfeile, binnen T Takten höchstens D vom Punkt; nicht erfüllt, ist der Hinweis gescheitert (einer mit `retry` wird erneut versucht, ein anderer endet einfach dort) |
| `retry N [W]` | von hier an wird der Hinweis bis zu N-mal erneut versucht, wenn er scheitert, wie `retry N` vor dem `:` es von seinem Anfang an tut: der Lauf wird gesichert, wenn der nächste Schritt beginnt (eine Strecke davor, die immer gelingt, wird nicht wiederholt); mit W werden seine `pass`- und `press`-Punkte (und die Linie eines `ram`) bis W statt 0.3 quer verschoben (höchstens 2): ein Rammen, dessen Ausgang davon abhängt, wo es den Körper trifft |
| `before N` | eine Erfolgsbedingung in der Zeit: die Uhr des Sektors (wie `until` sie zählt) nicht über N, wenn der Hinweis hierher kommt, sonst ist er gescheitert (ein Zug von Kugeln oder ein bewegtes Teil zu spät getroffen; das Protokoll gibt die Uhr aus) |
| `full STEP` | der Schritt (einer, der schiebt: nicht `turn`, `cross`, `trace` oder `wait`) schiebt so stark, wie die Pfeile der Ansicht, wie sie ist, erlauben, statt im Mittel mit dem Schub des Originals: auf einer 45-Grad-Karte zwei Pfeile entlang einer Weltachse (1,41-mal), einer diagonal (siehe „Modifizierte Originale“); auf den Originallevels dasselbe wie ohne |
| `trace FILE FROM TO [GAIN]` | dieselbe Strecke eines aufgezeichneten Laufs, aber geregelt: das Replay wird beim Laden der Hinweise einmal abgespielt, für Ort und Geschwindigkeit der Kugel in jedem Takt; danach sind in jedem Takt die Pfeile des Spielers die Vorsteuerung plus ein Zug (mal GAIN, 1 ohne Angabe, 0 keiner) zu Ort und Geschwindigkeit an der passenden Stelle (nach Fortschritt entlang des Laufs, nahe einem Takt pro Takt, so dass Wartezeiten Wartezeiten bleiben), in den Pfeilsatz umgesetzt, der dem in der aktuellen Ansicht am nächsten liegt (der Rest geht in den nächsten Takt). Mehr als 4 Einheiten neben dem Lauf lenkt es zum Weg des Laufs 30 Takte voraus, bis die Kugel wieder darauf ist, und gibt sie nach 10 s an den Autopiloten zurück. Anders als `replay` hält es, wenn die Kugel etwas anders ankommt (eine Kette von Sektoren: erst `sync`, dann `trace`) |

Richtungen sind Weltrichtungen (X nach Osten, Z nach Norden, wie das Level gebaut ist), ein Hinweis hängt also nicht
von der Drehung der Ansicht ab; die Pfeile werden in jedem Takt zur Ansicht gewählt. Eine Hinweisdatei ist nicht
vertrauenswürdig: ihre Größe (64 KiB), die Zahl der Hinweise (256) und Schritte (64 je Hinweis) und jede Zahl sind
begrenzt, ein Replay ist ein einfacher Dateiname daneben, und eine fehlerhafte Zeile weist die ganze Datei mit ihrer
Zeilennummer ab. `HINTS_TRACE=1` gibt die Kugel alle zehn Takte aus, solange ein Hinweis lenkt. Zum Vergleich zweier Läufe gibt `DRAHTSEILAKT_DUMP_STATE=S:T,...` (oder `@N`, ein Takt des Spiels) mit `--replay --headless` den ganzen Zustand der Simulation bei T auf der Uhr von Sektor S aus (die Takte, seit seine Module in die Physik kamen, die Uhr, auf die `until` wartet): die Kugel, den Zustand jedes Moduls und Auslösers, jeden losen Körper und Modulteil mit seinem Physikkern, den Zufallsgenerator der Physik.

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
| Rücktaste | die zuletzt in diesem Fenster markierte Zone zurücknehmen, mit der Regel, die sie lehrte (die Zeilen jeder Markierung tragen ihre id, `# mark=<id>`, so werden Zeilen einer anderen Sitzung nie entfernt); während eine Taste gehalten wird: diese Markierung verwerfen |
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
`--hints hints/Level_01.txt`: `hints/Level_01.zones`), die jeder Lauf des Levels liest. Mit `--hints FILE` werden die Markierungen in den Ordner dieser Datei geschrieben
(die gelernte Tabelle auch); ohne in `hints/` im Datenordner des Spiels. Eine Datei wird ganz oder gar nicht
geschrieben (eine neue Datei daneben, an ihren Platz umbenannt), nie durch einen Link; eine volle Zonendatei (256
Zonen) nimmt keine mehr, und der Bildschirm sagt es:

```
flow sector 1 r 4 : X Y Z ; X Y Z ; ...
careful sector 2 r 4 [speed V] : X Y Z ; ...
plain sector 2 r 8 : X Y Z ; ...
```

Die Punkte sind der Weg der Kugel durch die Strecke (ein Punkt: eine Stelle); die Zone ist alles innerhalb `r` von
diesem Weg in diesem Sektor.

- **flow** — hier nicht trödeln. Ein Hinweisschritt, der die Kugel anhält, läuft ohne Halt: ein `goto` wird
  durchrollt, ein `wait` oder `hold` ausgelassen, ein `pass` oder `press` langsamer als 5 mit 5 gefahren; aber nur,
  wenn auf einer Kopie des Laufs der Rest des Hinweises und eine Sekunde danach die Kugel behalten (ein Halt, den die
  Kugel braucht, bleibt). Die Steuerung zahlt für Rollen langsamer als 0,8 des Tempos des Bodens mehr, als ein
  Warten wert ist (eine Wahl, die die Kugel verliert, wird nie genommen), außer wo die Kugel Anlauf braucht (eine
  Lücke oder ein zu steiler Anstieg voraus, wie bei einer careful-Zone unten): dafür zurückzurollen und
  anzuhalten ist kein Trödeln, dort verlangt die Zone nichts (der Halt eines Hinweises bleibt auch).
- **careful** — hier das Tempolimit halten und keine Kurven schneiden. Nicht schneller als `speed` (4,5 ohne Angabe;
  zum Ändern die Zeile bearbeiten): die Steuerung zahlt für jede Einheit darüber viermal so viel, `pass`, `press`,
  `along` und `round` eines Hinweises werden darauf begrenzt, und was auch steuert (die Steuerung, ein Hinweis, die
  Linie des Spielers), die Pfeile in Laufrichtung der Kugel werden über dem Limit losgelassen und über dem 1,25-Fachen
  wird gebremst. Die Linie des Weges voraus (die Mitte, die die Wegsuche fand) wird gehalten: jede Einheit daneben
  kostet 2, neben der Mitte des Bodens dreimal so viel wie sonst. Wo die Physik den Schwung der Kugel braucht, wird
  das Limit losgelassen und nur die Linie gehalten: der Weg voraus (etwa 30 Einheiten) hat eine Lücke zum
  Überqueren oder einen Anstieg, steiler als die Kugel aus eigenem Schub schafft am Limit (bei 4,5: Stein
  12,6°, Holz 26°, Papier 31°; auf der Engine gemessen schafft Stein aus eigenem Schub nichts Steileres als 17°, Holz
  37°, Papier 46°),
  gleich voraus kommt eine Schwelle, oder der laufende Hinweis sagt Tempo (`press`, `along` oder `round` schneller
  als das Limit: ein Anstieg, Schienen, ein Rammstoß; oder ein `pass` schneller als es bis 25 Einheiten davor: sein
  Anlauf). Die Statuszeile sagt es (`[limit let go: a gap to cross ahead]`). Eine gelernte careful-Regel bremst
  ebenso und wird an denselben Stellen losgelassen.
- **plain** — hier keine gelernte Regel (von Hand geschrieben, `plain sector N r R : X Y Z ; ...`): der Testspieler
  fährt, wie er es selbst tut, wo eine auf einem anderen Level gelernte Regel die Kugel verlieren würde (auf Level 6
  schneidet „bend, wood: nicht trödeln“ das Warten auf einen Aufzug und das der Papierkugel an den Ventilatoren ab).

Jede Markierung lehrt den Testspieler auch, was für eine Stelle es war, für jedes Level: eine Regel der gelernten
Tabelle (`learned.txt` neben den Hinweisen; [autopilot-rules.md](autopilot-rules.md), „Die gelernte Tabelle“).

`AUTOPILOT_SITUATION=1` druckt außer der Stelle jede Sekunde jeden Wechsel dessen, was Zonen und Regeln verlangen,
mit dem Tick und dem Ort der Kugel (`zone t14528 (137.9 -6.8 -554.8) [learned: ... → careful, at most 4.5]`, `none`,
wenn nichts gilt), und warum ein Limit losgelassen ist.

Eine Zonendatei ist nicht vertrauenswürdig wie eine Hinweisdatei: ihre Größe (64 KiB), die Zonen (256) und Punkte
(256 je Zone) und jede Zahl sind begrenzt; eine fehlerhafte Zeile wird mit einem Vermerk ausgelassen, die anderen
Zonen gelten.

## Modifizierte Originale

Die Umbauten der Originallevels aus der Community (levels.md, „Modifizierte Originale“) haben die Geometrie des
Originals: dieselben Objekte an denselben Stellen. Die **unsichtbaren Karten** verstecken jedes Objekt, auch den
Platzhalter des Zielballons (das Spiel setzt den Ballon trotzdem dorthin, wie es die Initialisierung des Originals
tut); ihre Physik ist die des Originals, Tick für Tick, sodass die Läufe des Originals unverändert auf ihnen laufen.
Die **45-Grad-Karten** drehen die Wiedereinstiegspunkte um eine Achteldrehung: die Kamera blickt entlang einer
Diagonale der Welt, und die Pfeile schieben entlang der Achsen der Kamera.

Darum nimmt der Testspieler die Hinweise und Pläne des Originals, wie sie sind (ihre Orte und Richtungen sind die der
Welt), und gibt eine Richtung der Welt als die Pfeile der Kamera, wie sie gerade steht, samt ihrem Versatz gegen ganze
Vierteldrehungen: auf einer 45-Grad-Karte ist ein Schub entlang einer Weltachse zwei Pfeile. Zusammen gehaltene Pfeile addieren ihre
Schübe, dort wäre ein Schub entlang einer Achse also 1,41-mal so stark wie im Original und einer diagonal 0,71-mal:
die Schritte der Hinweise (Schübe, Anlehnen an Schienen, Anläufe und Rammstöße, ein Abschnitt `replay`) sind in der
Ansicht des Originals abgestimmt, darum geben sie auf einer solchen Karte im Mittel den Schub des Originals: in jedem
Tick die nächstliegenden Pfeile, plus was die Ticks davor übrig ließen (ein diagonaler Schub des Originals, 1,41, ist
mehr, als ein Pfeil entlang dieser Diagonale gibt: dort ein Pfeil). `cross` (auf Kopien des Laufs in der Ansicht, wie
sie ist, erprobt) und `trace` (mit eigenem Übertrag) bleiben, wie sie sind; auf den Originallevels ändert sich nichts.
Wo der Schub des Originals in einer solchen Ansicht zu wenig ist, wird ein Schritt `full` geschrieben (die eine Regel dafür,
auf Wunsch, so dass ohne sie abgestimmte Läufe bleiben, wie sie waren): er wird mit den eigenen Pfeilen der Ansicht
gegeben, so stark, wie sie schieben, zwei Pfeile entlang einer Weltachse (1,41-mal der Achsschub des Originals, so viel
wie sein diagonaler) und einer diagonal (dort gibt kein Satz mehr). Ein Rammstoß, ein Kick oder ein Aufstieg, den das
Original diagonal machte (1,41), wird entlang einer Achse mit `full` geschrieben, um diesen Schub zu haben.
Ein Replay eines Spielers
auf dem Originallevel (mit `--follow` und `--then` gefolgt oder als Abschnitt `replay` oder `trace` eines Hinweises)
wird auf dem Originallevel angesehen, der Datei der Kopie selbst, wenn die Karte dessen Geometrie hat (dieselben
Objekte an denselben Stellen, in beliebiger Reihenfolge); übernommen werden nur der Weg des Balls und seine Schübe in
der Welt, und die Tasten des Spielers werden in die Pfeile der Kamera der Karte übersetzt. Ein Umbau mit anderen
Objekten ist kein Umbau dieses Levels: ein solches Replay wird wie bisher abgelehnt.

Ein Replay des Originallevels läuft auch mit `--replay` auf einem Umbau, dessen Objekte zudem gleich gedreht sind
(eine unsichtbare Karte: dieselbe Physik und Kamera, also derselbe Lauf Tick für Tick; `--record` speichert ihn als
Replay der Karte). Auf einer 45-Grad-Karte würden die Tasten anders schieben, dort wird es abgelehnt.

Stand 2026-11-15 (Läufe in `hints/runs/modified/`): die unsichtbaren Karten, 12 von 13 vom Testspieler vom Start an
geschafft (Level 1, 11, 12, 13 allein nach Plan und Hinweisen, 4 und 9 den Läufen des Besitzers folgend, die übrigen dem
gespeicherten Lauf des Originals folgend; Level 8 bisher nur als gespeicherter Lauf des Originals, auf der Karte
abgespielt, `Level_08-original-run.replay`). Die 45-Grad-Karten: Level 1-7, 9 und 10 geschafft, dem gespeicherten
Lauf des Originals ohne Hinweise folgend (die offenen Schritte der Hinweise sind für die Kamera des Originals
geschrieben); Level 8 ebenso, 10:41.30. Mit den Hinweisen als Schub des Originals
(oben), 2026-11-15: Level 12 geschafft, 13:30.93 (`--follow` mit seinem gespeicherten Lauf, sein Plan und
`hints/Level_12-45.txt`: zwei Stellen vorsichtiger, die
Rückkehr auf die obere Reihe von Sektor 6 bei z 694 und die Ecke auf die Schiene x 521.3 in Sektor 7 langsamer und
weiter; Bremsen und Lenken sind dort diagonale Schübe, in dieser Ansicht schwächer). Level 11
geschafft, 14:44.20 mit 3 Leben (ebenso, `hints/Level_11-45.txt`: je zwei Rammstöße gegen die Kiste mehr in den
ersten beiden Kanälen von Sektor 3; auf den letzten Schienenpaaren von Sektor 6 lehnt das erste Paar mit 1.2 statt
0.8, und da die Kugel langsamer als im Original ans Ende des zweiten kommt, 7 bis 8 statt 9.7, weil das Anlehnen vom
Schub entlang der Schiene nimmt, und zu kurz auf der Kreuzung des dritten landet, setzen sie 8 Ticks Schub nach Osten
auf die Schienen des dritten, bevor sie zwischen sie zurückrollt). Level 13 (`hints/Level_13-45.txt`, 2026-11-16: `Level_13-full.txt`, Sektor 2 dort
zerlegt, wo der Lauf des Besitzers in dieser Ansicht anders geht: das Brett bei z 686, das seine Kugel im Vorbeirollen
umwirft, wird umgerammt; die Kugel hält an der Kreuzung der Serpentine bei z 721 2 Einheiten früher, so bleibt die
Kiste darunter und Stein 18 rollt unberührt darüber; die Kiste wird geregelt unter die Kreuzungen z 713 und 705
geschoben, jede Kreuzung abgewartet; auf dem Balken nach seinen Starttasten ein Schub nach Osten, damit die
Papierkugel vor dem Stein im Fach des zweiten Ventilators ist) schaffte damals die Sektoren 1 und 2 vom Start,
wenn er mit dem Plan des Levels läuft: `--autopilot --plan hints/Level_13.plan --hints hints/Level_13-45.txt`. Ohne
`--plan` ist der gefundene Weg ein anderer (2255 Punkte statt 2530), und die Kugel bleibt am Ende von Sektor 1 stehen;
alles, was der Lauf liest, liegt in `hints/` (weder Weg noch Wiederholungen noch die gelernte Tabelle kommen aus dem
Datenordner), und mit leerem Datenordner läuft er Tick für Tick gleich wie mit einem voller früherer Läufe. Am 2026-11-16 mit leerem Datenordner erneut geprüft:
der Lauf ist deterministisch (dasselbe Rezept zweimal, auch mit dem Programm jenes Merges, gibt eine byte-gleiche
Wiederholung); durch Sektor 2 kommt er nur durch Zurückgehen: die Papierkugel rollt hinter Stein 18 die Schienen hinab
statt auf ihm (die Kugel des Besitzers liegt vorn auf ihm) und fällt in die Lücke bei z 585, die der Besitzer mit 9
bis 12 überspringt, und der 13. Versuch (zurück auf 3:23, die geseedete Variation) schafft sie, CP2 um 3:44.88 mit 3
Leben. Mit `--rewinds 0` geht die Kugel dort verloren; ein schnellerer `full`-Schub, vor dem Stein oder nach seinem
Fall, sprang nicht hinüber. Die Sektoren 3, 4 und 5 gelingen, von ihren Kontrollpunkten begonnen, mit
`full`-Schritten (oben), wo der Schub des Besitzers diagonal war: in Sektor 3 die Kisten `full` entlang der Achsen
gerammt (Kiste 40 nach Osten direkt aus seinem Lauf, ohne Anlauf), der Papiertransformator und der Weg herum zur
Mündung des Kanals `full`, sodass die Kugel vor dem ersten Stein im Kanal ist (0:58.2, bei ihm 0:58.0); sein Lauf von
z -753 bis zum Steintransformator gefolgt (1:14.45, bei ihm 1:14.32), sodass die vier schwingenden Säcke stehen wie
bei ihm und dort kein Stein fällt; am Ende des Kanals sein Lauf, dann hält die Kugel still, während der letzte Stein
kommt: alle zehn Steine in der Schlange, CP3 um 2:44.93 (bei ihm 2:44.92). In Sektor 4 ist sein Kick nach Nordosten
vom Floß in den ersten Ventilator (x -134) in seinen zwei Teilen gegeben, `full` nach Norden, dann `full` nach Osten
(je 22 Ticks, danach sein Lauf): CP4 um 2:04.48 (bei ihm 2:04.18). In Sektor 5 sind der Schub auf die Spirale und
`round` hinauf `full`: das Level vom Kontrollpunkt in 1:54.17 ohne verlorene Kugel geschafft. Vom Start (2026-11-16) stirbt der Lauf in
Sektor 3 an Kiste 40: die Kugel kommt 0.2 bis 1 Einheit neben dem Weg des Starts am Kontrollpunkt zum Rammstoß (Kiste
38, von seinem Lauf mit Verstärkung 3 gerammt, wird etwas anders gestoßen), trifft die Kiste südlich ihrer Mitte, die
Kiste dreht sich und klemmt, die Kugel steht bei (-726.6, -219.4, -293.9); 30 Versuche zurück lösen sie nicht. Andere
Rammlinien, ein `pass`-Punkt davor, Stillstehen am Eingang des Sektors vor seinem Lauf und ein `shove` nach dem
Rammstoß brachen den Start an CP3 oder halfen nicht. Von Sektor 3 an in einem Zug geht Sektor 4 nach dem ersten
Ventilator verloren: das Floß (vier lose Kugeln) ist am selben Ort wie bei einem Start an CP4 (auf 0.1), aber die
Stelle des Trace in seinem Lauf hinkt 20 bis 30 Ticks nach, so kommt die Kugel 3 Einheiten nördlich an den zweiten
Ventilator (x -76, z -1143), wird zu wenig gehoben und fällt auf den unteren Boden, verloren bei x 50 bis 60. Auf dem
Floß fahren, bis es steht, wo seins stand, dann folgen, gibt beiden Starts denselben Zustand, verfehlt aber von dort
den Ventilator; mit `pass`-Schritten hineingerollt fällt die Kugel in die Lücke darunter. Vom Start ist Level 13 noch
nicht geschafft (Kiste 40 in Sektor 3 und der zweite Ventilator in Sektor 4 in einem Zug).
Später am 2026-11-16 wurde Sektor 4 auf das Floß synchronisiert, wie der Besitzer darauf wartet: nach jeder Landung hinter einem Ventilator hält die Kugel und wartet (`await`) auf die vordere Kugel des Floßes dort, wo sie war, als er losfuhr, dann wird sein Lauf weiter verfolgt (sein Stoß in den dritten Ventilator aufgeteilt in `full` nach Norden und Osten): ab CP3 CP4 in 2:04.22 (seiner 2:04.18). In einem Zug ab Sektor 3 endet das Warten innerhalb von 4 Ticks desselben Zustands, aber nach dem zweiten Ventilator landet die Kugel trotzdem zu tief. Für Kiste 40 scheiterten der neue Schritt `ram` (ein Körper mit Anlauf durch seine Mitte gerammt, nach seiner jetzigen Lage), ein Abgleich am Halt im Steinumwandler und ein Halt vor dem Stoß schon ab CP3 (die Kiste klemmt, oder die Papierkugel kommt eine Sekunde zu spät zu den Steinen); `ride` vor der Mitte von Stein 18 schafft Sektor 2 ab CP1 4 s schneller, vom Start aber nie die Lücke. Aus einem leeren Datenordner ist der Lauf wie zuvor: tot in Sektor 3 an Kiste 40.

Zustand für Zustand verglichen (2026-11-16, `DRAHTSEILAKT_DUMP_STATE`, siehe oben) unterscheiden sich der Lauf vom Start und
der Start an CP3 in Sektor 3 in: der Kugel (sie kommt rollend über den Checkpoint, ein Start setzt sie still hin), den
losen Körpern des Sektors und den Nähetests der Module um einen Takt (an einem Checkpoint kommen die losen Körper einen
Takt vor den Modulen, ein Start mit `--sector` setzt beide zugleich ein), dem Schwingen der vier Säcke (seine
Schrittzahl läuft vom Levelstart: 38 gegen 139, die andere Parität), dem gemeinsamen Zufallsgenerator der Physik und
dem Zähler des Schlaftests (wann ruhende Körper einfrieren, daher liegt Kiste 42 um 0,15 Einheiten und 1,7 Grad
anders) und der Drehlage der Kugel. Der Steintransformator macht die Kugel gleich (derselbe Ort, in Ruhe), und ein
darin begonnener trace beginnt, wenn die Kugel losgelassen wird: von dort stimmen beide Läufe 130 Takte lang auf 0,0002
überein, doch das Rammen von Kiste 38 (Kiste auf Kiste) macht daraus binnen 50 Takten 1 bis 2 Einheiten, und Kiste 40
klemmt im einen Lauf, im anderen nicht. Gleiche Zustände sind dort nicht zu erreichen (Drehlage und Zufallsgenerator
liegen nicht bei den Hinweisen); jede Variante ab dem Loslassen (trace ab Takt 2322 bis 2328, ein Halten bis zur Uhr
des Sektors danach) klemmt Kiste 40 auch ab CP3. Das Kistenrätsel braucht Schritte, deren Ausgang von so kleinen
Unterschieden nicht abhängt.

## Der Speedrun-Stil

`--style speedrun` (mit `--autopilot` oder `--watch`) lässt den Testspieler so schnell fahren, wie er kann, für
Speedrun-Videos; `--style calm`, der Standard, ist alles oben Beschriebene, und seine Läufe bleiben unverändert. Wo er
den Weg selbst findet, hält ein Speedrun ein weit höheres Tempo (11 auf einer Schiene, 9 und mehr auf einem Boden, bis
22 auf einem breiten, 16 abseits der Böden), meidet einen Rand nur näher als 0,8 und hält keine Mitte: er fährt an
Rändern entlang und schneidet Kurven; keine Vorsicht-Zone und keine gelernte Regel hält ihn zurück. Den Drang einer
Fluss-Zone zur Geschwindigkeit übernimmt er nicht: Maß bleibt, wie weit eine Wahl auf dem Weg voranbringt (sonst rollte
er lieber schnell einen Anstieg hinab als langsam hinauf). Die Halte eines Hinweises bleiben, wie sie geschrieben sind
(ein Warten, das ein Rätsel braucht: eine Kopie des Laufs beurteilt nur die Kugel), außer in den Fluss-Zonen des
Levels wie im ruhigen Stil. Folgt er dem Lauf eines Spielers, steht er nicht, wo der Spieler stand: ein Abschnitt von
30 Ticks oder mehr, in dem die Kugel des Spielers langsamer als 0,8 und innerhalb 1,5 vom Anfang ist, wird
übersprungen, wenn die Tasten des Spielers ab seinem Ende auf einer Kopie des Laufs die Kugel fünf Sekunden auf seiner
Linie halten (ein Warten auf einen Aufzug oder eine Schaukel besteht das nicht und bleibt). Ein Fehlschlag wird wie im
ruhigen Stil zurückgenommen: das Risiko kostet Versuche, nicht den behaltenen Lauf.

Eine **ruhige Zone** in der Zonendatei eines Levels (`calm sector N r R : X Y Z ; ...`, von Hand geschrieben) verlangt
dort vom Speedrun den ruhigen Stil und ändert am ruhigen Stil nichts: ein Rätsel, das das gleichmäßige Tempo schafft
und das des Speedruns nicht (eine Papierkugel vor einer Steinkugel, ein Labyrinth von Schiebern in der Phase des
Spielers). Dort werden auch die Stillstände des Spielers nicht übersprungen.

Für andere Versuche skaliert `AUTOPILOT_SPEEDRUN_PACE=F` das Tempo (0,3 bis 2), `AUTOPILOT_SPEEDRUN_NO_SKIP=1` behält
die Stillstände des Spielers, und `AUTOPILOT_SPEEDRUN_SKIP_ONLY=1` fährt das ruhige Tempo und überspringt nur sie. Die
schnellsten Läufe liegen in `hints/runs/speedrun/`.

Ein Level, dessen Speedrun Hinweise braucht, die der ruhige Lauf nicht nehmen darf, hat eine eigene Hinweisdatei (mit
`--hints`): `hints/Level_03-speedrun.txt` ist `Level_03.txt` und ein Hinweis mehr (Sektor 4: die Holzkugel sofort auf
die einzelne Schiene südlich des Kontrollpunkts geschickt, wo sie von selbst 8 s stand). Die Zonendatei bleibt die
daneben (`hints/Level_03.zones`), ihre ruhigen Zonen gelten nur für den Speedrun. 2026-11-16, Level 3, 5, 9 und 13:

- Level 3: 5:26.92, 5 Leben, keine Kugel verloren (ruhig 5:58.97), `--plan hints/Level_03.plan --hints
  hints/Level_03-speedrun.txt`. Die Wartezeiten auf den Schienen von Sektor 2 (die Kugel kommt in den Mulden an ihrem
  Fuß zur Ruhe) bleiben: kürzer fällt sie herunter. Kein Weg an den Verwandlern von Sektor 1 vorbei gefunden (Stein am
  Eingang der Gabelplattform nimmt den Südarm zum Papierverwandler nicht).
- Level 5: 11:29.95, 5 Leben, keine Kugel verloren (ruhig 12:39.77), `--follow hints/Level_05-owner.replay --hints
  hints/Level_05.txt` mit einer ruhigen Zone über Sektor 5 (`hints/Level_05.zones`; im Tempo des Speedruns fiel die
  Kugel dort bei (1113.7, -138, -47) bis zum Spielende herunter). Sektoren 1-4 im Speedrun-Tempo erreichen den letzten
  Kontrollpunkt bei 8:25 (ruhig 9:35).
- Level 9: noch kein schnellerer Lauf. Dem Lauf des Besitzers folgend liegt der Speedrun am dritten Kontrollpunkt 7 s
  vorn, fällt dann in Sektor 4 (Steinkugel, Zielscheiben, Schienen um die Schleife) immer wieder bei (423, -109, 568)
  herunter; eine ruhige Zone dort oder über den ganzen Sektor macht ihn langsamer als den ruhigen Lauf (16:55, zwei
  Kugeln verloren).
- Level 13: der Speedrun gleicht dem ruhigen Lauf (10:43.90): das Level fahren Hinweise und die nachverfolgten Läufe
  des Besitzers, seine langen Halte gehören den Rätseln selbst (eine Kiste unter der Serpentine, 8 s; die Steine, die
  die Papierkugel am Ende von Sektor 3 heben, 20 s).

Die Läufe von Level 3 und 5 spielen auf den unsichtbaren Karten Tick für Tick gleich.

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
  die Hinweise zum Lauf des Inhabers heißen jetzt `Level_01-follow.txt`): 5:17.12, 9.1% der Zeit stehend. Mit den
  Markierungen des Inhabers (`hints/Level_01.zones`, `hints/learned.txt`), 2026-11-03: 6:23.63, 9.7% der Ticks
  stehend (unter 0,3 pro Sekunde; der Lauf davor 9.7% so gezählt), näher als 1 am Rand 4.3% (4.5%), Tempo 4.8 ± 3.6
  (5.5 ± 4.0) (`hints/runs/Level_01-clean.replay`).
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
  Stein schnell zurück, die Kisten heruntergerammt, der Stamm nach dem Sturz in Sektor 5). Dieser Lauf kam weder mit
  der gelernten Tabelle (Spielende in Sektor 2) noch ohne sie (stecken in Sektor 5) wieder; 2026-11-15, mit
  plain-Zonen, wo die auf Level 1 gelernten Regeln das Warten der Sektoren 2 und 3 abschneiden
  (`hints/Level_06.zones`), und der Steinkugel in Sektor 5 von ihrem Umwandler auf die hängenden Platten geschoben
  (allein rollte sie vom Turm auf die Schienen über dem Stamm): 9:01.60, 4 Leben, steht 17,2% der Ticks (vorher
  16,5%), näher als 1 am Rand 5,3% (2,5%), Tempo 4,8 ± 4,5 (4,9 ± 4,4) (`hints/runs/Level_06.replay`).
- Level 7: vom Start an geschafft in 12:04.28, dem geschafften Lauf des Inhabers folgend.
- Level 8: vom Start an in 9:59.40 beendet, ganz ohne Lauf eines Spielers: allein durch den Plan aus dem Video
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
- Level 12: vom Start an in 12:35.68 beendet, ganz ohne Lauf eines Spielers, allein nach dem Plan und den Hinweisen
  aus dem Storyboard des Walkthroughs (`hints/Level_12.plan`, `.txt`, `.route`; `hints/runs/Level_12.replay`): die
  Serpentine der dünnen Schiene, das Ziel, das die Brücke in Sektor 3 senkt, die Rinne gekippter Würfel und ihre Lücke,
  die vier Aufzüge mit Käfig, die Ventilatortreppen, die in die Spiralen geschobenen Steinkugeln, die Kisten auf dem
  Buckel, die Halbröhren aus Stämmen. Stillstand 5,7 % der Ticks (1,4 % außerhalb von Transformatoren), näher als 1 am
  Rand 14,1 %, Geschwindigkeit 5,3 ± 3,8.
- Level 13: Sektor 1 nach den Hinweisen; Sektor 2 von seinem Anfang an nach den Hinweisen aus dem Lauf des Besitzers,
  seiner Linie folgend (`hints/Level_13.txt`, `hints/Level_13-s2.replay`), ohne Zurückgehen: die Bretter, die Kiste
  unter jeder Schienenkreuzung, wenn Steinkugel 18 kommt, der Tauchgang der Papierkugel, der Balken und der Schacht.
  Sektor 3: an den Steinen vorbei über die seitlichen Rinnen wie der Besitzer, die Kisten, Stein, Papier und Ventilator
  nach Hinweisen, allein weiter bis zur Grube vor CP3, wo die Papierkugel wartet, bis die Steine sie heben (noch nicht
  gelöst). Vom Levelstart an ist Sektor 2 noch nicht zuverlässig (die Stöße der Kiste hängen davon ab, woher die Kugel
  kommt); Sektoren 4 und 5 sind offen.
- Videos der beendeten Läufe: `scripts/video.sh`, außerhalb des Repositorys aufbewahrt.
- Unsere zehn Level (levels.md, „Unsere Level und wie man sie bearbeitet“), 2026-11-03: jedes vom Start an in drei
  Stilen geschafft, mit den Hinweisdateien, die ihre Beschreibungen schreiben (`assets/levels/<Level>/pilot/`).
