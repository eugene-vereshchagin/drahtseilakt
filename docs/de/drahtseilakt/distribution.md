# Wie Drahtseilakt ausgeliefert wird und wo was zu finden ist

Eine Seite für zwei Fragen: *Was lade ich herunter?* und *Wo in den Repositories liegt was?* Die Schritte zum Bauen
und Hochladen stehen in [release.md](release.md), die Gründe in [decisions.md](decisions.md) (unten verlinkt).

## Was in welchem Download steckt, und warum

Drahtseilakt besteht aus drei Teilen, die sich unterschiedlich schnell ändern und deshalb getrennt ausgeliefert werden:

- **Das Programm** (das Spiel, in den portablen Archiven und Installern auch die Toolbox): bei jeder Version neu.
- **Die Basisdaten**: unsere eigenen Teile, Texturen, Schriften, Klänge, Werksmods, das Schaufensterlevel `00-Showcase`,
  das versteckte `Parts_Test` und die 13 Community-`.nmo`-Karten. Sie liegen in git und ändern sich hin und wieder.
- **Die 13 ausgewählten Karten** (das Drahtseilakt-Set): selten. Je eine `.zip`, dazu ein Linux-Paket,
  `drahtseilakt-levels`.

Warum diese Aufteilung:

- **Die Grenzen von GitHub.** Eine Datei in git darf nicht über 100 MB sein, ein Release-Asset nicht über 2 GiB. Das
  `level.bin` einer ausgewählten Karte ist größer als 100 MB, deshalb gibt es diese Karten nur als Release-Assets
  (zusammen etwa 1,1 GB, jede Datei unter der Grenze); die Basisdaten (rund 100 MB, keine Datei über 3,6 MB) passen in git
  ([Binärdateien in git: die Basisdaten](decisions.md#binärdateien-in-git-die-basisdaten)).
- **Kleine Programm-Updates.** Die Levels ändern sich nicht mit dem Programm; eine neue Programmversion baut also nur die
  Binärpakete neu, und niemand lädt wieder ein Gigabyte. Die Programmversion ist der Tag `X.Y.Z`; die Daten haben ihre
  eigene, `scripts/DATA_VERSION` (ein Datum), und die Programmpakete verlangen `drahtseilakt-data >= DATA_VERSION`.
- **Eigene Inhalte als Standard; vom Original wird nie etwas ausgeliefert.** In den Downloads steckt nur, was wir gemacht
  haben oder was uns die Urheber zu liefern erlauben (genannt in `CREDITS.md`, `THIRD-PARTY.md`). Die Ressourcen des
  Originals entstehen auf dem Rechner des Spielers aus seiner eigenen Kopie
  ([Eigenes als Standard, das Original auf Wunsch des Spielers](decisions.md#eigenes-als-standard-das-original-auf-wunsch-des-spielers)).
  Das ist rechtliche Vorsicht und die Inhaltspolitik des Projekts zugleich.
- **Die Toolbox bereitet Levels vor, das Spiel spielt nur.** Die eigene Kopie importieren, remastern, heruntergeladene
  Karten konvertieren und der Upscaler Real-ESRGAN sind Aufgabe der Toolbox, eines eigenen Programms in einem eigenen
  Repository; das Spiel spielt, was in seinen Ordnern liegt
  ([Die Toolbox bereitet Level vor](decisions.md#die-toolbox-bereitet-level-vor-das-spiel-spielt-nur)). Darum enthalten
  die portablen Archive und Installer beide Programme, die Linux-Pakete nur das Spiel.
- **Das Spiel geht nie online.** Es lädt keine Levels, Updates oder Schriften nach und fragt nicht nach einer neuen Version;
  die Dateien holen Sie selbst von GitHub ([Das Spiel geht nie online](decisions.md#das-spiel-geht-nie-online)). Nur der
  Build darf herunterladen, festgelegt und per Prüfsumme geprüft.
- **Portabel oder installiert.** Eine portable Kopie (eine Datei `portable.txt` neben dem Programm) hält alles in ihrem
  Ordner und fasst sonst nichts an; eine installierte legt die Daten des Spielers in dessen Profil, sodass ein Deinstallieren
  sie nie löscht ([files.md](files.md)).
- **Der Dev-Build** ist für die Entwicklung, nicht für Spieler: ein Debug-Build mit eingeschaltetem God Mode
  (`drahtseilakt-dev`, `-dev` im Namen). Er entsteht nur auf Anfrage (Workflow-Eingabe `dev`) und ist nie Teil eines Releases.

Das einzige im Code fest verbundene Level ist das Schaufensterlevel `00-Showcase`; alle anderen Levels sind Daten in
Ordnern ([levels.md](levels.md)).

## Ich will X: lade Y herunter

Am einfachsten nimmt man die Links aus der Download-Tabelle der README.md (und ihrer Übersetzungen): Ein Workflow (`readme-downloads.yml`) schreibt sie bei jedem veröffentlichten Release mit direkten Links auf die Dateien des neuesten veröffentlichten Releases neu. Das neueste Release findet man außerdem über die GitHub-API, nicht über einen festen Link: `GET https://api.github.com/repos/eugene-vereshchagin/drahtseilakt/releases/latest`, dann aus `assets[]` die Datei wählen, deren `name` zu einem Muster wie `drahtseilakt-*-windows-x86_64-setup.exe` passt, und ihre `browser_download_url` verwenden. Die Seite <https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest> zeigt immer das neueste Release. Die Dateinamen behalten die Version (unten `<version>`; `<data-version>` bei den Datenpaketen), damit man sie lesen kann. Prüfen Sie einen Download gegen `SHA256SUMS` aus demselben Release.

| Ich will | Herunterladen |
|---|---|
| Unter Windows spielen, installiert (pro Benutzer, ohne Administrator, `%LOCALAPPDATA%\Programs\Drahtseilakt`) | `drahtseilakt-<version>-windows-x86_64-setup.exe` (ARM: `-aarch64-`) |
| Unter Windows spielen, ohne Installation | `drahtseilakt-<version>-windows-x86_64.zip` (ARM: `-aarch64`) |
| Unter Linux spielen, ohne Installation | `drahtseilakt-<version>-linux-x86_64.tar.gz` (ARM: `-aarch64`) |
| Pakete für Debian / Ubuntu | `drahtseilakt_<version>-1_amd64.deb` + `drahtseilakt-data_<data-version>-1_all.deb` (ARM: `drahtseilakt_<version>-1_arm64.deb`) |
| Pakete für Fedora / openSUSE | `drahtseilakt-<version>-1.x86_64.rpm` + `drahtseilakt-data-<data-version>-1.noarch.rpm` (ARM: `drahtseilakt-<version>-1.aarch64.rpm`) |
| Unter macOS spielen | `drahtseilakt-<version>-macos-aarch64.dmg` (Apple Silicon) oder `drahtseilakt-<version>-macos-x86_64.dmg` (Intel); unsigniert: beim ersten Start Rechtsklick, Öffnen. Die `.tar.gz` mit denselben Namen ist die portable Form |
| FreeBSD (experimentell) | `drahtseilakt-<version>-freebsd-x86_64.tar.gz` |
| Die Toolbox | in jedem portablen Archiv und Installer enthalten; oder in ihren eigenen Releases: [drahtseilakt-toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox/releases) |
| Die 13 ausgewählten Karten | das Daten-Release `data-<DATA_VERSION>`: eine `.zip` je Karte (nach `assets/levels/drahtseilakt/` entpacken), unter Linux `drahtseilakt-levels` (`.deb`/`.rpm`) |
| Die Original-Levels von Ballance | von uns nicht herunterladbar: die Toolbox importiert sie aus Ihrer eigenen Kopie |
| Den neuesten unveröffentlichten Code ausprobieren | selbst bauen ([README](https://github.com/eugene-vereshchagin/drahtseilakt/blob/main/README.de.md)) oder die Artefakte eines Workflow-Laufs nehmen (`git-<sha>`-Builds, Pakete `0~git<sha>`) |

Die portablen Archive und Installer enthalten die Basisdaten und laufen sofort; die Linux-Pakete brauchen das Datenpaket
neben dem Programm (`apt install ./drahtseilakt-data_*_all.deb ./drahtseilakt_*_amd64.deb`). Die genaue Dateiliste eines Releases
und die Installationszeilen stehen in [release.md](release.md).

## Wo was liegt

| Wo | Was |
|---|---|
| `eugene-vereshchagin/drahtseilakt` (dieses Repository) | das Spiel, die Engine-Crates (`crates/`: `nmo`, `level`, `gamedata`, die Physik `ivp`, `script`, `game`, ...), `assets/` (Basisdaten), `docs/`, `scripts/`, `tools/`, `hints/` (Autopilot-Pläne), `tests/` |
| `eugene-vereshchagin/drahtseilakt-toolbox` | die Toolbox, mit eigenen Releases und eigener CI; sie liest Levels über `crates/gamedata` und `crates/level` dieses Repositorys |
| `assets/` | was mit dem Spiel ausgeliefert wird: unsere Teile, Schriften, Klänge, Werksmods, `levels/community/` und `levels/drahtseilakt/` (dort nur die `level.txt` der ausgewählten Karten; ihre Dateien sind Release-Assets) |
| `docs/drahtseilakt/` | die Projektdoku (Fahrplan, Entscheidungen, Formate, Menüs, Mods); jede Seite auch in `docs/ru/`, `docs/de/`, `docs/zh/` |
| `docs/manual/<Sprache>/` | das Handbuch für Spieler; `tools/make_manual.py` baut es nach `assets/manual`, das nicht in git liegt: die CI erzeugt es (`scripts/ci-assets.sh`), und das Spiel öffnet es mit F1 |
| `scripts/` | das Verpacken des Releases (`ci-package.sh`, `make-distro-packages.sh`, `pack-assets.sh`, `DATA_VERSION`) |
| `.github/workflows/` | CI: Tests (`drahtseilakt.yml`) und Release (`drahtseilakt-release.yml`); beide laufen auf Anforderung |
| `docs/internals/`, `docs/*.md` im Wurzelverzeichnis | Notizen und Reverse-Engineering-Referenzen aus dem Upstream OpenBallance, als Referenz behalten |

## Regeln für Mitwirkende

- **Keine Original-Ressourcen**, nie: keine Datei, Textur, kein Klang und kein Modell von Ballance in git, in einem Paket,
  einem Test oder einer Fixture. Tests, die das Spiel brauchen, lesen es aus `OPENBALLANCE_DATA`; nichts in den Downloads
  stammt vom Original.
- **Festgelegte Abhängigkeiten**: Crates in `Cargo.lock` (vendored, der Build läuft offline), Actions per vollständigem
  Commit-SHA, Werkzeuge und heruntergeladene Binärdateien per Version und SHA-256. Eine Abhängigkeit kommt erst dazu, wenn
  geprüft ist, dass es sie gibt, dass sie gepflegt wird und eine GPL-3.0-kompatible Lizenz hat (`tools/third_party.py` und
  ein Test wachen darüber).
- **Vier Sprachen**: Eine Änderung an der Doku geht im selben Commit in Englisch, Russisch, Deutsch und vereinfachtem
  Chinesisch ein; eine neue Seite bekommt einen Seitenleisten-Eintrag in jeder Locale (`docs/.vitepress/config.ts`).
  Menütexte müssen in allen Sprachen passen.
- **Die CI läuft auf Anforderung** und bei Pull Requests, nicht bei jedem Push: Änderungen bündeln.
- **Kein Netzwerk im Spiel**; nur der Build darf herunterladen.
- **Releases nur durch den Besitzer**: Niemand setzt einen Tag, veröffentlicht oder lädt ein Daten-Release hoch ohne seinen
  ausdrücklichen Befehl. Ein Tag `X.Y.Z` erzeugt einen Entwurf; er veröffentlicht ihn.

Einzelheiten: `CLAUDE.md` im Wurzelverzeichnis des Repositorys.
