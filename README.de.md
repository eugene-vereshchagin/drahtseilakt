# Drahtseilakt

[English](README.md) · [Русский](README.ru.md) · **Deutsch**

*Drahtseilakt* — die Nummer des Seiltänzers und jeder Balanceakt, bei dem ein Schritt zur Seite den Absturz
bedeutet. Ballance, 2004 vom kleinen deutschen Studio Cyparade gemacht, ist genau das: eine Kugel auf einer
Schiene über dem Abgrund. Der Name ist unsere Verbeugung vor dem Spiel und den Menschen, die es gemacht haben.

Drahtseilakt ist ein Fork von [OpenBallance](https://github.com/emdzej/openballance) von emdzej, einer
originalgetreuen Neuimplementierung von **Ballance** (Cyparade / Atari, 2004), die die eigenen Virtools-Skripte
des Originalspiels auf einer neuen Laufzeitumgebung ausführt, mit einem Port seiner Physik-Engine Ipion. Ziel
des Forks ist ein vollwertiges natives Spiel: ein echtes Fenster unter Linux, Windows und macOS, jede Auflösung
und jedes Seitenverhältnis bis 8K, eine wirklich runde Kugel, Level 13 und ein Menü für Community-Karten — und
später ein offenes, dokumentiertes Levelformat.

Stand: in Arbeit, keines der eigenen Ziele des Forks ist bisher erreicht. Siehe den
[Fahrplan](docs/de/drahtseilakt/roadmap.md) und die [Entscheidungen](docs/de/drahtseilakt/decisions.md) dahinter.

## Wie es funktioniert

Fast die gesamte Spiellogik von Ballance steckt in Virtools-Verhaltensgraphen in seinen `.cmo` / `.nmo`-Dateien.
Die Engine lädt diese Dateien in ein CK2-kompatibles Objektmodell und führt die Graphen aus; die Building
Blocks, die sie aufrufen, sind aus den Original-DLLs portiert, und die Physik ist ein Port der Ipion-Engine
(IVP) aus `physics_RT.dll`. Die Interna sind in [`docs/`](docs/) beschrieben (Dokumentationsseite des
Upstreams: https://openballance.emdzej.pl; bisher auf Englisch).

## Bauen

Derzeit ist der Build der des Upstreams (der native Backend ist Phase 1 des Fahrplans):

```sh
tools/fetch-gasm-sdk.sh
cmake -S . -B build-gasm -DOPENBALLANCE_PLATFORM=gasm -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_TOOLCHAIN_FILE=.deps/gasm-c-sdk/cmake/gasm-toolchain.cmake -DWASI_SDK_PREFIX="$PWD/.deps/wasi-sdk"
cmake --build build-gasm -j        # build-gasm/openballance.wasm

cmake -S . -B build && cmake --build build -j   # Engine-Bibliothek + Tests (brauchen Ihre Spieldaten)
```

## Rechtliches

Drahtseilakt steht unter der GPL-3.0, wie OpenBallance, von dem es abgeleitet ist. Es enthält weder Code noch
Assets von Ballance: Sie brauchen Ihre eigene Kopie des Spiels. Ballance © 2004 Cyparade / Atari. Dieses
Projekt steht in keiner Verbindung zu ihnen.
