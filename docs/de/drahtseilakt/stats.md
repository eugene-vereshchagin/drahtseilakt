# In Zahlen

Wie Drahtseilakt entstand, gezählt am 2026-11-03 (`scripts/stats.py`). Jede Zeile Code, jede Seite dieser
Dokumentation und jeder erzeugte Klang wurde von Claude geschrieben (dem KI-Modell von Anthropic, in Claude Code).
Veresk, der das Projekt führt, hat nichts davon geschrieben: Er setzte die Ziele, traf die Entscheidungen, spielte,
hörte zu und sagte, was nicht stimmte.

## Zeit

| | |
|---|---|
| Beginn der Arbeit | 2026-10-30, auf dem Fork von OpenBallance (2026-10-01) |
| Kalenderzeit | 4 Tage |
| Arbeitszeit von Claude, Hauptsitzung | 34,6 Stunden (Pausen zwischen Schritten unter 15 Minuten) |
| Arbeitszeit von Claude, alle Agenten zusammen | 47,8 Stunden (bis zu sechs Agenten arbeiteten parallel) |

## Code

| | Zeilen |
|---|---|
| Rust, 6 Crates | 44.108 |
| — game (Fenster, Menü, Klang, Spielregeln, Autopilot) | 22.882 |
| — ivp (die Physik-Engine, portiert) | 14.080 |
| — gamedata, level, nmo, cdimage (Dateien, Formate, die CD) | 7.146 |
| Shader (WGSL) | 296 |
| Die Toolbox (eigenes Repository, Rust) | 3.874 |
| Klangsynthese (Python) | 1.234 |
| Tests | 249 |
| Commits | 243 im Spiel seit dem Fork, 19 in der Toolbox |

Was es ersetzt: den C-Code von OpenBallance, mit dem es begann, 27.144 Zeilen in 83 Dateien, nur noch als Referenz
behalten, bis er geht.

## Dokumentation

Vier Sprachen (Englisch, Russisch, Deutsch, vereinfachtes Chinesisch), je 11 Seiten, etwa 1.200 Zeilen pro Sprache.

## Klang

Das erzeugte Klangthema ist ganz aus Code gemacht: 51 Klänge und Musikstücke — jede Kugel auf jeder Oberfläche, die
Ventilatoren, der Donner, die Atmosphären, fünfzehn Phrasen gezupfter Saiten und die Musik des Ziels — synthetisiert
aus Messungen der Klänge des Originals, ohne ein einziges Sample davon.

## Tokens

| | |
|---|---|
| Modellaufrufe | 7.472 (Claude Opus 5.5: 6.368, Claude Haiku 5.5: 1.104) |
| Vom Modell geschriebene Tokens | 6,7 Millionen |
| Neu gelesene Tokens | 15,6 Tausend |
| Aus dem Prompt-Cache gelesene Tokens | 3,30 Milliarden |
| In den Prompt-Cache geschriebene Tokens | 25,0 Millionen |
| Alle verarbeiteten Tokens | 3,33 Milliarden |

Fast alles Gelesene ist derselbe Arbeitskontext, bei jedem Schritt neu gelesen: eine lange Sitzung über viel Code.
