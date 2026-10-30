# Drahtseilakt

[English](README.md) · [Русский](README.ru.md) · **Deutsch**

*Drahtseilakt* — die Nummer des Seiltänzers und jeder Balanceakt, bei dem ein Schritt zur Seite den Absturz
bedeutet. Ballance, 2004 vom kleinen deutschen Studio Cyparade gemacht, ist genau das: eine Kugel auf einer
Schiene über dem Abgrund. Der Name ist unsere Verbeugung vor dem Spiel und den Menschen, die es gemacht haben.

Drahtseilakt ist eine neue Engine für **Ballance** (Cyparade / Atari, 2004), von Grund auf in Rust geschrieben.
Sie spielt die Daten deiner eigenen Kopie des Spiels — die Originallevel genau so, wie sie sind, und
Community-Karten — unter Linux, Windows und macOS, in jeder Auflösung und jedem Seitenverhältnis bis 8K, mit
Level 13 und einem Menü für Community-Karten, später mit einem offenen, dokumentierten Levelformat und einem
Konverter dafür.

Das Projekt begann als Fork von [OpenBallance](https://github.com/emdzej/openballance) von emdzej, das die
originalen Virtools-Skripte auf einer eigenen Laufzeit ausführt; seine Historie und Reverse-Engineering-Notizen
bleiben erhalten, sein Code wird ersetzt (warum, steht in den [Entscheidungen](docs/de/drahtseilakt/decisions.md)).

Stand: in Arbeit — Phase 1 des [Fahrplans](docs/de/drahtseilakt/roadmap.md), der Leser für Virtools-Dateien.
Warum das Spiel das überhaupt brauchte: [Geschichte](docs/de/drahtseilakt/history.md).

## Bauen

Rust (stable) und cargo. Abhängigkeiten sind vendort; der Build braucht kein Netz. Die Anleitung folgt mit dem
ersten Code von Phase 1.

## Rechtliches

Drahtseilakt steht unter der GPL-3.0, wie OpenBallance, mit dem es begann. Es braucht deine eigene Kopie des
Spiels und liefert nie Daten von Ballance aus. (Solange dieses Repository privat ist, hält `levels/` eine
Sicherung der Originallevel für die Entwicklung; das öffentliche Repository enthält sie nicht.) Ballance ist
© 2004 Cyparade / Atari. Dieses Projekt steht in keiner Verbindung zu ihnen.
