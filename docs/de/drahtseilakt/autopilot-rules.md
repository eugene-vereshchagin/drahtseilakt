# Wie der Testspieler fährt: die Regeln

Die Läufe des Testspielers sind der Beweis, dass ein Level durchspielbar ist, und ihre Videos werden angesehen.
Darum fährt er wie ein sicherer, ruhiger Spieler: in der Mitte der Bahn, mit gleichmäßiger, mäßiger Geschwindigkeit,
ohne Ideallinien und ohne Zögern an den offensichtlichen Stellen. Langsamer wird er nur, wo das Level es verlangt
(eine enge Stelle, ein Timing). Diese Regeln sagen, wie; die Route jedes Levels (`hints/Level_NN.route`) sagt, wo.
Die Mechanik, auf der sie beruhen, steht in [autopilot.md](autopilot.md).

## Tempo

- **Reisegeschwindigkeit nach Boden.** Auf einer Schiene oder schmalen Bahn mäßig (6 Einheiten je Sekunde), auf
  breitem Boden mehr (bis 10, mit seiner Breite). Schneller kostet; langsamer als die Hälfte davon auch, wo nichts
  ein Warten verlangt.
- **Kein Anhalten und Anfahren.** Die Punkte eines Hinweises werden durchfahren (`pass`), nicht angefahren und dort
  angehalten (`goto`), außer der nächste Schritt braucht die Kugel still und an ihrem Platz: ein Anlauf auf eine
  Zielscheibe, der Beginn eines Rammstoßes, der Rand einer Schaukel.
- **Festgefahren ist kurz.** Nach zwei Sekunden ohne Vorankommen probiert der Testspieler seine Manöver. Folgt er dem
  Lauf eines Spielers, ist dessen Tempo das Maß (vier Sekunden, die bisherige Steuerung): der Spieler kann dort aus
  gutem Grund gewartet haben.

## Wo die Kugel nie stehen bleibt

- **Schienenenden und Schwellen.** Wo der Weg von einer Schiene auf einen Boden geht oder eine kleine Stufe hinauf,
  behält die Kugel ihren Schwung: kurz aufgehalten, setzt sie etwas zurück und nimmt sie mit Anlauf.
- **Transformatoren.** Unterwegs in Fahrt hinein; das Spiel hält die Kugel, während sie sich verwandelt, das ist kein
  Halt des Spielers. Am Rand eines Transformators aufgehalten, fährt die Kugel sofort hinein.
- **Lücken, über die der Weg geht.** Sofort mit Anlauf genommen, nicht nach Stehen an der Kante.
- **Geraden, Kurven, Säcke und schwingende Teile, wenn frei.** Kein Zögern; auf ein bewegliches Teil im Weg wird nur
  gewartet, solange es im Weg ist.

## Schienen und Stämme

- **Eine Kette von Punkten ist eine Linie.** Jedes `pass` hält die Kugel auf der Linie vom vorher durchfahrenen
  Punkt, so folgt eine entlang von Schienen oder einer schmalen Bahn geschriebene Kette ihnen und schneidet nicht ab.
- **Schienenpaare übereinander** (die Kugel rollt auf der unteren und lehnt an der oberen): `press` entlang mit einem
  Versatz zur oberen Schiene, zuerst genau am Anfang des Paars ausgerichtet; am Ende eines Paars schnell genug, um
  auf das nächste zu kommen.
- **Wellige Stämme.** Eine in einer Senke stehende Kugel kommt nicht heraus, eine Steinkugel am wenigsten: auf einem
  welligen Stamm hält die Kugel ihre Geschwindigkeit (`press` schnell), zwischen den flach darauf liegenden
  Transformatoren.

## Wo die Kugel hält oder wartet

- **Schaukeln und Aufzüge**: am Rand, still, bis zu dem auf Kopien des Laufs gefundenen Moment (`cross`).
- **Zielscheiben**: vor dem Anlauf genau ausrichten (`hold`), dann schnell in die Scheibe und nachdrücken (`press`,
  dann `push`): eine Kugel, die die Scheibe nur berührt, lässt die Brücke oben.
- **Kisten zum Herunterrammen**: ein Anlauf vom selben Ort für jeden Rammstoß, danach bremsen, damit die Kugel der
  Kiste nicht über die Kante folgt.
- **Ventilatoren**: eine Papierkugel wartet über dem Gitter nur, bis sie gehoben wird.

## Mechanismen

- **Bretter auf einem Scharnier** (Zugbrücken, P_Modul_25) werden aus kurzem Anlauf umgestoßen, mit Bremsen gleich nach
  dem Stoß, damit die Kugel dem Brett nicht in die Lücke folgt.
- **Schieber** (P_Modul_01) bewegt ein Stoß gegen ihre Scheibe, nicht Andrücken; reicht einer nicht, mehrere.
- **Eine lose Kugel als Schlüssel**: wo eine lose Kugel irgendwohin rollen muss, um einen Mechanismus auszulösen (die
  Scheibe eines Schiebers, eine Falltür), wird sie mit einem streifenden Stoß entlang ihrer Reihe dorthin gestoßen,
  die Kugel bremst sofort (Level 11: zwei Steinkugeln vom Gitter auf die Schienen darunter gestoßen).
- **Wippen** (P_Modul_30) werden vom tiefen Ende mit Anlauf überquert: aus dem Stand kommt eine Steinkugel nicht über
  die Achse.
- **Ein Hinweis beginnt, nachdem ein Transformator die Kugel losgelassen hat**, nicht während er sie hält: der Platz auf
  dem Weg wird nach dem Hinweis neu gefunden, und ein unter einem Hinweis passierter Transformator gilt nicht als
  passiert.
- **Ein passierter Sektor ist fort**: seine Objekte verlassen Physik und Bild, der Weg führt nie zurück hinein.

## Kugeln

- **Papier** über Ventilatoren und an Luftwänden entlang; leicht: für Hänge und durchhängende Platten braucht sie
  Anlauf.
- **Holz** für Schienen und die meisten Zielscheiben; schwer genug, um eine schnell getroffene und nachgedrückte
  Zielscheibe umzulegen.
- **Stein** zum Rammen von Kisten und Schieben von Kugeln; er zerbricht hängende Platten (P_Modul_29): er fährt nicht
  darüber, außer das Level will, dass er mit den Platten hindurchfällt (die Türme von Level 6).

## Zonen, die der Zuschauer markiert

Wo der Besitzer sieht, dass der Testspieler falsch fährt, markiert er die Strecke beim Zusehen (`--watch`, F8 und
F4; [autopilot.md](autopilot.md), „Zonen“), und die Markierung ist von da an eine Regel für diese Strecke, in
`<level>.zones` neben den Hinweisen:

- **flow** („hier nicht trödeln“): kein Halt, kein Warten, kein Langsamwerden, das die Kugel nicht braucht. Das
  `goto` eines Hinweises wird durchrollt, `wait` und `hold` ausgelassen, ein langsames `pass` oder `press` mit 5
  gefahren, jeweils nur, wenn eine Kopie des Laufs zeigt, dass die Kugel so bleibt; die Steuerung hält mindestens
  0,8 des Tempos des Bodens.
- **careful** („Tempolimit halten und keine Kurven schneiden“): nicht schneller als das Tempo der Zone (4,5 ohne
  Angabe), auf der Mittellinie des Weges, keine Ideallinie; was auch steuert, bleibt unter dem Limit.

Eine Zone des Zuschauers geht den Tempo-Regeln oben vor: sie ist, was der Besitzer gesehen hat.

## Die gelernte Tabelle: `hints/learned.txt`

Eine Zone lehrt den Testspieler auch, was für eine Stelle es war, für jedes Level, auch noch nicht geschaffte (12, 13,
unsere eigenen). Bei jeder Markierung liest er die Stelle entlang der markierten Strecke, jede Zehntelsekunde, nach
acht Merkmalen:

| Merkmal | Werte |
|---|---|
| surface | `floor`, `rail`, `off` (auf keinem Boden des Weges: ein Modulteil, in der Luft) |
| width | die des Bodens: `narrow` (unter 4), `medium` (unter 10), `wide` |
| border | die nächste Kante des Bodens innerhalb 8: `walls` (dort steht eine Wand), `open` (es geht hinunter), `far` |
| curve | die Drehung des Weges über die nächsten 8 Punkte: `straight` (unter 20°), `bend` (unter 60°), `sharp` |
| slope | sein Anstieg darüber: `flat`, `up`, `down` (mehr als 0,15) |
| ahead | das Erste darin: `gap`, `drop`, `rail-end`, `rail-start`, `sill`, `transformer` oder `none` |
| module | das nächste Modul innerhalb 8 (`P_Modul_25`, `P_Box`, ...) oder `none` |
| kind | `wood`, `stone`, `paper` |

Was über den größten Teil (60 %) der Strecke gleich war, wird eine Regel: die Oberfläche, die Kugel und die Werte,
die die Stelle besonders machen (kein gewöhnlicher mittlerer oder breiter Boden, gerade, eben, nichts voraus, kein
Modul); die gewöhnlichen bleiben weg, sie würden die Regel nur seltener gelten lassen. Eine Strecke ohne Besonderes
lehrt nichts (ihre Zone bleibt die des Levels). Die Regel kommt in `learned.txt` neben den Hinweisen, eine Datei für
alle Level, mit dem Ort, wo sie gelernt wurde, und was der Testspieler dort tat:

```
flow when surface=rail ahead=rail-end kind=wood   # Level_01 sector 2 (147.2 -2.1 -552.0) 2026-11-03: 3.0 s, stopping 40%, mostly hint, off the middle 0.0
careful speed 4.5 when surface=floor width=narrow curve=sharp kind=stone
```

Eine Regel gilt, wo jedes Merkmal, das sie nennt, diesen Wert hat; sie verlangt, was eine Zone ihrer Art verlangt
(flow, careful). Jeder Lauf liest die Tabelle und sieht jeden Takt auf die Stelle: wo die Kugel in keiner eigenen Zone
des Levels ist (die gehen vor) und Regeln gelten, flow, wenn eine davon flow ist, das niedrigste Limit der careful;
die Statuszeile nennt die genaueste geltende Regel (`learned: rail, rail-end ahead, wood → don't dawdle`), mit F12
auch die Stelle, wie die Regeln sie sehen (`here: surface=rail ...`). Eine Regel nennt mindestens zwei Merkmale,
eines davon besonders für die Stelle, damit keine fast überall gilt. Eine Markierung zurücknehmen (Rücktaste) nimmt
auch die Regel heraus, die sie lehrte. Die Tabelle ist einfacher Text: eine zu weite Regel wird mit einem weiteren
Merkmal enger gemacht oder gelöscht. Gelesen wie eine Hinweisdatei, ist sie nicht vertrauenswürdig: Größe (64 KiB),
Regeln (512) und Zahlen begrenzt, Werte aus den Listen oben, eine fehlerhafte Zeile mit Vermerk ausgelassen.
`AUTOPILOT_SITUATION=1` gibt die Stelle, wie die Regeln sie sehen, jede Sekunde eines Laufs aus.

Die Merkmale unterscheiden 2079 Arten von Stellen ohne das Modul (1701 auf Böden, 189 auf Schienen, 189 abseits),
jede davon noch mit jedem Modul in der Nähe.

## Die Route eines Levels: `hints/Level_NN.route`

Eine Datei je Level, für Menschen und Agenten (der Testspieler liest den Plan und die daraus gemachten Hinweise): für
jeden Sektor seine Abschnitte der Reihe nach, jeder mit der Kugel, mit der er gefahren wird, den Objekten der Reihe
nach, dem, was an jedem getan wird, und dem Tempo.

```
sector 1 wood                         # der Sektor und die Kugel, mit der er beginnt
  start → end platform x 166          pass, cruise
  rails x 122 south → Paper_01        pass into it → paper
  Modul_18_01 (fan)                   wait over the grate → lifted onto the rails x 80
  rails x 80 south → Wood_01          press at a push (a slope) → wood
  rails x 80 north, U over the fan    pass, cruise (a wood ball is not lifted)
  rails x 96 south → CP1              press, cruise; on over the sill onto the platform
```

Aktionen: **pass** (durch, im Tempo), **press** (entlang Schienen oder einer Geraden, an ihr gehalten), **round**
(eine Spirale), **hold** (still, ausgerichtet), **cross** (eine Schaukel, im gefundenen Moment), **ram** (Kisten, mit
Anlauf, bremsend), **target** (hold, dann press und push in die Scheibe), **wait** (nur über dem Gitter eines
Ventilators oder für ein bewegliches Teil).
