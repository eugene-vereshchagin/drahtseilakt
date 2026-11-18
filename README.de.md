# Drahtseilakt

[English](README.md) · [Русский](README.ru.md) · **Deutsch** · [简体中文](README.zh.md)

*Drahtseilakt* — die Nummer des Seiltänzers und jeder Balanceakt, bei dem ein Schritt zur Seite den Absturz
bedeutet. Ballance, 2004 vom kleinen deutschen Studio Cyparade gemacht, ist genau das: eine Kugel auf einer
Schiene über dem Abgrund. Der Name ist unsere Verbeugung vor dem Spiel und den Menschen, die es gemacht haben.

Drahtseilakt ist eine neue Engine für **Ballance** (Cyparade / Atari, 2004), von Grund auf in Rust geschrieben. Sie läuft
nativ unter Linux, Windows und macOS, in jeder Auflösung und jedem Seitenverhältnis bis 4K, mit eigener Oberfläche,
eigenen Teilen und Klängen, sodass man sofort losspielen kann. Sie spielt die Original-Levels aus Ihrer eigenen Kopie
des Spiels (einschließlich Level 13), Community-Karten (als `.nmo`-Dateien samt ihren Virtools-Skripten) und Levels im
eigenen offenen, dokumentierten [Levelformat](docs/de/drahtseilakt/open-format.md). Mit dem Spiel kommen 13 Community-Karten so,
wie ihre Autoren sie gemacht haben, und 13 weitere ausgewählte (das Drahtseilakt-Set, ein eigener Download), alle mit
Nennung ihrer Autoren ([CREDITS.md](CREDITS.md)). Vom Original wird nie etwas ausgeliefert; was Sie davon wollen, macht
die Toolbox aus Ihrer eigenen Kopie.

Das Projekt begann als Fork von [OpenBallance](https://github.com/emdzej/openballance) von emdzej. Seine Historie und die
Reverse-Engineering-Notizen (`docs/`) bleiben erhalten, die Physik ist nach Rust portiert; der Rest ist neuer Code (warum:
[Entscheidungen](docs/de/drahtseilakt/decisions.md)). Warum das Spiel überhaupt ein zweites Leben brauchte:
[Geschichte](docs/de/drahtseilakt/history.md).

**Stand:** in Arbeit, Version 0.1.0, noch nicht öffentlich veröffentlicht. Engine, Menüs, Ton, Physik, Replays, die Skripte
der Community-Karten, das offene Format und die Paketierung stehen; was bis zu einem Release fehlt, steht unter
„Kriterien für die Veröffentlichung“ im [Fahrplan](docs/de/drahtseilakt/roadmap.md). Ein Release macht nur der Besitzer; bis dahin sind
die Downloads unten Entwürfe oder CI-Builds.

## Herunterladen

Jedes Release enthält Installer und portable Archive für Windows, Linux und macOS (FreeBSD experimentell), `.deb`-/`.rpm`-Pakete
für Linux, und die [Toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox) steckt in den portablen
Archiven und Installern. Die 13 ausgewählten Karten sind ein eigener Download, der sich selten ändert. Die Dateinamen behalten die Version. Das neueste Release: `https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest`; ein Skript holt es über die API (`GET https://api.github.com/repos/eugene-vereshchagin/drahtseilakt/releases/latest`, dann die Datei aus `assets[]`, deren `name` z. B. zu `drahtseilakt-*-windows-x86_64-setup.exe` passt, und ihre `browser_download_url`). Welche Datei für wen ist und warum die Downloads so aufgeteilt sind:
[distribution.md](docs/de/drahtseilakt/distribution.md); die genaue Liste und die Installationszeilen:
[release.md](docs/de/drahtseilakt/release.md).

<!-- downloads:start -->
Version **0.1.0**, alle Dateien und Hinweise: [https://github.com/eugene-vereshchagin/drahtseilakt/releases/tag/0.1.0](https://github.com/eugene-vereshchagin/drahtseilakt/releases/tag/0.1.0).

| System | Download |
|---|---|
| Windows x64, Installer | [drahtseilakt-0.1.0-windows-x86_64-setup.exe](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-windows-x86_64-setup.exe) |
| Windows x64, portabel | [drahtseilakt-0.1.0-windows-x86_64.zip](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-windows-x86_64.zip) |
| Windows ARM, Installer | [drahtseilakt-0.1.0-windows-aarch64-setup.exe](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-windows-aarch64-setup.exe) |
| Windows ARM, portabel | [drahtseilakt-0.1.0-windows-aarch64.zip](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-windows-aarch64.zip) |
| macOS Apple Silicon (nicht signiert) | [drahtseilakt-0.1.0-macos-aarch64.dmg](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-macos-aarch64.dmg) |
| macOS Intel (nicht signiert) | [drahtseilakt-0.1.0-macos-x86_64.dmg](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-macos-x86_64.dmg) |
| macOS Apple Silicon, portabel | [drahtseilakt-0.1.0-macos-aarch64.tar.gz](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-macos-aarch64.tar.gz) |
| macOS Intel, portabel | [drahtseilakt-0.1.0-macos-x86_64.tar.gz](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-macos-x86_64.tar.gz) |
| Linux x64, portabel | [drahtseilakt-0.1.0-linux-x86_64.tar.gz](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-linux-x86_64.tar.gz) |
| Linux ARM, portabel | [drahtseilakt-0.1.0-linux-aarch64.tar.gz](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-linux-aarch64.tar.gz) |
| Debian/Ubuntu x64 (mit dem Datenpaket) | [drahtseilakt_0.1.0-1_amd64.deb](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt_0.1.0-1_amd64.deb) |
| Debian/Ubuntu ARM (mit dem Datenpaket) | [drahtseilakt_0.1.0-1_arm64.deb](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt_0.1.0-1_arm64.deb) |
| Debian/Ubuntu: das Datenpaket | [drahtseilakt-data_20261117-1_all.deb](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-data_20261117-1_all.deb) |
| Fedora/openSUSE x64 (mit dem Datenpaket) | [drahtseilakt-0.1.0-1.x86_64.rpm](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-1.x86_64.rpm) |
| Fedora/openSUSE ARM (mit dem Datenpaket) | [drahtseilakt-0.1.0-1.aarch64.rpm](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-1.aarch64.rpm) |
| Fedora/openSUSE: das Datenpaket | [drahtseilakt-data-20261117-1.noarch.rpm](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-data-20261117-1.noarch.rpm) |
| FreeBSD x64 (experimentell) | [drahtseilakt-0.1.0-freebsd-x86_64.tar.gz](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/drahtseilakt-0.1.0-freebsd-x86_64.tar.gz) |
| Prüfsummen | [SHA256SUMS](https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/0.1.0/SHA256SUMS) |

Die 13 ausgewählten Karten, ein eigener Download (etwa 1,1 GB, je Karte ein Zip, entpacken nach `assets/levels/drahtseilakt/` des Spiels): [data-20261117](https://github.com/eugene-vereshchagin/drahtseilakt/releases/tag/data-20261117).
<!-- downloads:end -->

## Bauen und Spielen

Rust (stable, 1.89 oder neuer) und cargo; unter Linux ein Vulkan-Treiber und die udev-Header für die Gamepads
(`libudev-dev` unter Debian/Ubuntu, `systemd-devel` unter Fedora). Der Build läuft offline: Die Abhängigkeiten sind
festgelegt und liegen im Repository.

```sh
cargo build --release
target/release/drahtseilakt
```

Aus einem Checkout findet das Spiel die Basisdaten in `assets/`. Es startet mit dem Hauptmenü; `--level N` spielt das
Original-Level N, `--map FILE.nmo` eine beliebige Leveldatei oder Community-Karte, `--open DIR` ein Level im offenen Format,
`--replay FILE` einen aufgezeichneten Lauf; `drahtseilakt --help` listet alles auf. Die Pfeiltasten rollen die Kugel relativ zur
Kamera, Shift+Links/Rechts drehen die Kamera, die Leertaste hebt sie an, F11 ist Vollbild, Esc pausiert, F1 öffnet das
Handbuch; Gamepads gehen auch, und Options → Controls belegt andere Tasten. Menüs, Modi und Optionen:
[Menüs](docs/de/drahtseilakt/menu.md); wo das Spiel seine Dateien ablegt, portabel oder installiert:
[Dateien](docs/de/drahtseilakt/files.md); wo Levels liegen und wie man sie macht und teilt:
[Levels](docs/de/drahtseilakt/levels.md), [Levels machen](docs/de/drahtseilakt/making-levels.md). Unter Windows öffnet der
Release-Build kein Konsolenfenster; Meldungen gehen in `drahtseilakt.log` im Datenordner.

Zum Starten braucht das Spiel keine Kopie von Ballance. Um die Original-Levels zu spielen, zeigen Sie der Toolbox Ihr
installiertes Spiel oder ein CD-Abbild (oder starten `drahtseilakt --import DIR`); dann erscheinen Levels, Klänge und
Aussehen des Originals im Spiel.

## Die Toolbox

`drahtseilakt-toolbox` ist ein eigenes Programm in einem eigenen Repository,
[eugene-vereshchagin/drahtseilakt-toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox), mit eigenen
Releases. Sie bereitet Levels auf Ihrem Rechner vor: Sie importiert Ihre Ballance-Kopie, remastert die Original-Levels
(rundere Formen, mehrfach größere Texturen; mit Real-ESRGAN, wenn installiert, sonst mit einfacher Skalierung),
konvertiert heruntergeladene Community-Karten und prüft, ob sie spielbar sind. Das Spiel spielt nur; es startet kein
anderes Programm. Um beide aus dem Quelltext zu bauen, klonen Sie die Toolbox neben dieses Repository (sie liest Levels
über die Crates dieses Repositorys). Für Spieler beschreibt sie das [Handbuch](docs/manual/de/toolbox.md).

## Dokumentation

Die Doku gibt es auf Englisch, Russisch, Deutsch und in vereinfachtem Chinesisch (`docs/`, `docs/ru/`, `docs/de/`,
`docs/zh/`), auch als mit VitePress gebaute Website (`docs/.vitepress`). Beginnen Sie mit dem
[Fahrplan](docs/de/drahtseilakt/roadmap.md) (der Plan und was „fertig“ heißt) und den
[Entscheidungen](docs/de/drahtseilakt/decisions.md) (warum). Das Handbuch für Spieler liegt in `docs/manual/<Sprache>/` und
wird mit dem Spiel ausgeliefert. Zum Testen gibt es einen fensterlosen Replay-Check, aufgezeichnete
Läufe, die die Tests abspielen, und Fuzzing jedes Parsers ([Fuzzing](docs/de/drahtseilakt/fuzzing.md)).
Regeln für Mitwirkende: [distribution.md](docs/de/drahtseilakt/distribution.md) und `CLAUDE.md`.

## Wer es gemacht hat

Den Code, die Dokumentation und die erzeugten Klänge und Musik schreibt **Claude** (das KI-Modell von Anthropic, in Claude
Code). **Veresk** besitzt das Projekt: die Idee, die Entscheidungen, den Plan, das Spielen und das Ohr, das sagt, was
stimmt. Die Commits tragen beide Namen.

## Lizenz und Danksagungen

Aus OpenBallance (GPL-3.0, emdzej und Mitwirkende) übernommene Dateien sagen das in ihren Köpfen; die Crate `ivp` ist der
Physik-Port. Der Dank an OpenBallance und emdzej bleibt mit der Historie dieses Repositorys bestehen.

Alles in einem Download, was nicht für Drahtseilakt geschrieben wurde, mit seiner Lizenz (der Upstream, Schriften,
Texturquellen, Klänge und Musik, die mitgelieferten Community-Levels und ihre Autoren, jede ins Programm kompilierte Crate),
steht in [THIRD-PARTY.md](THIRD-PARTY.md), das neben `LICENSE` liegt; die Karten und ihre Autoren stehen in
[CREDITS.md](CREDITS.md). `tools/third_party.py` erzeugt die Crate-Liste offline aus `Cargo.lock` neu, und ein Test schlägt
bei einer Abhängigkeit ohne GPL-3.0-kompatible Lizenz fehl.

Drahtseilakt steht unter der GPL-3.0, wie OpenBallance, aus dem es hervorging. Es liefert nie Daten von Ballance aus und
braucht zum Start keine Kopie des Spiels. (Solange dieses Repository privat ist, hält `levels/` eine Sicherung der
Original-Levels für die Entwicklung; das öffentliche Repository enthält sie nicht.) Die Community-Karten gehören ihren
Autoren; wenn ein Autor widerspricht, nehmen wir die Karte heraus. Ballance ist © 2004 Cyparade / Atari. Das Projekt steht
in keiner Verbindung zu ihnen.
