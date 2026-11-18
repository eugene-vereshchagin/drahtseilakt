# Releases und Pakete

Ein Release besteht aus drei Dingen, die sich unterschiedlich schnell ändern; sie werden getrennt gebaut und
ausgeliefert: ein Update des Programms baut nur die Binärpakete neu.

| Was | Ändert sich | Gebaut von | Veröffentlicht als |
|---|---|---|---|
| **Binärpakete**: das Programm und die Basisdaten als `.deb`/`.rpm` | mit jeder Version | Workflow `Drahtseilakt release` (CI) | Release mit Tag `X.Y.Z` (Entwurf) |
| **Levels**: die 13 ausgewählten Karten | selten | `scripts/pack-assets.sh` (Rechner des Besitzers) | Release mit Tag `data-<DATA_VERSION>` |

**Die Basisdaten liegen in git** (Entscheidung des Besitzers 2026-11-17, decisions.md „Binärdateien in git: die
Basisdaten“); CI nimmt sie aus dem Checkout und lädt nichts herunter, die Pakete `drahtseilakt-data` (`.deb`/`.rpm`) werden
im Release-Workflow daraus gebaut. Nur die 13 ausgewählten Karten sind nicht in git (ein `level.bin` dort liegt über
GitHubs Grenze von 100 MB je Datei); sie entstehen auf dem Rechner des Besitzers aus dem lokalen `assets/` und werden von
Hand hochgeladen.

## Basisdaten

`assets/` ohne die ausgewählten Karten (`assets/levels/drahtseilakt/NN-*`, deren `level.txt` bleibt in git) und ohne
reine Entwicklungsdateien (`mods-dev`, `own/make`): unsere Teile und Texturen, Schriften, Klänge, Werksmods, der
Showcase-Level (`00-Showcase`, `Parts_Test`) und die 13 Community-`.nmo`-Karten (`assets/levels/community`), an die das Spiel
gebunden ist. Etwa 100 MB in git (355 Dateien, die größte 3,6 MB), 93 MB als Paket. Nie eine Datei des originalen Ballance
(beim Aufnehmen per Hash gegen eine Kopie des Originals geprüft). Nicht in git, weil CI es billig erzeugt: das Handbuch
(`assets/manual`, `tools/make_manual.py`, von `scripts/ci-assets.sh` ausgeführt). `.gitignore` ignoriert die Dateien der
ausgewählten Karten außer ihrer `level.txt` sowie die abgeleiteten `.dtx`, `level.source`, `*.remaster/`,
`textures.original/`.

## Levels (Karten)

Die 13 ausgewählten Karten, je eine `.zip` (29-204 MB, zusammen 1,1 GB), und dieselben Karten in einem Paket
`drahtseilakt-levels` (2,8 GB installiert, 1,1 GB gepackt). Jede Datei liegt unter GitHubs 2-GiB-Grenze; die Skripte
verweigern ein größeres Paket.

## Versionen

- `scripts/DATA_VERSION` ist ein Datum `JJJJMMTT` (z. B. `20261117`). Es ist die Version der Daten **und** die Version,
  die die Programmpakete verlangen: sie hängen von `drahtseilakt-data (>= DATA_VERSION)` ab.
- Erhöhen, wenn sich Daten oder Levels ändern (und committen), dann packen, dann hochladen. Braucht ein
  Programm-Release neuere Daten als das letzte Daten-Release, kommt zuerst ein Daten-Release.
- Die Programmversion ist der Tag `X.Y.Z`. Ein Workflow-Lauf ohne Tag ergibt `0~git<sha>` in den Paketen.
- Ein Daten-Release trägt den Tag `data-<DATA_VERSION>`; der Tag-Filter des Workflows (`[0-9]+.[0-9]+.[0-9]+`) passt
  nicht darauf, es wird also nichts gestartet.

## Level-Pakete erzeugen

```sh
scripts/pack-assets.sh          # OUT ist standardmäßig ../drahtseilakt-portable/bundles
```

Erzeugt in OUT (`V` = `scripts/DATA_VERSION`):

| Datei | Wofür |
|---|---|
| `levels/NN-Name.zip` (13) | jedes System: nach `assets/levels/drahtseilakt/` entpacken |
| `drahtseilakt-levels_V-1_all.deb`, `drahtseilakt-levels-V-1.noarch.rpm` | Linux, alle 13 Karten (hängt vom Datenpaket ab) |
| `SHA256SUMS` | alles oben Genannte |

`--with-data` erzeugt zusätzlich das alte `drahtseilakt-data-V.tar.gz` und Datenpakete aus dem lokalen `assets/`, zum
lokalen Prüfen der Pakete; sie werden nicht hochgeladen: CI baut `drahtseilakt-data` aus git
(`scripts/make-distro-packages.sh data --out DIR` aus dem Checkout; `--tar FILE` nimmt stattdessen ein Archiv).

`.deb` und `.rpm` erzeugt [nfpm](https://nfpm.goreleaser.com) 2.49.0, ein Werkzeug für beide, sodass dasselbe Skript auf dem
Rechner des Besitzers (kein `rpmbuild` nötig) und in CI läuft. `scripts/fetch-nfpm.sh` lädt es nach `third_party/` (in
gitignore; nie in einen Systempfad) und prüft die im Skript festgeschriebene SHA-256. `scripts/pack-assets.sh --no-packages`
überspringt die Pakete.

## Das Level-Release hochladen

Vom Besitzer aus dem Checkout ausgeführt; nichts davon läuft von selbst:

```sh
V=$(cat scripts/DATA_VERSION)
cd ../drahtseilakt-portable/bundles && sha256sum --check --quiet SHA256SUMS
gh release create "data-$V" --repo eugene-vereshchagin/drahtseilakt --draft --latest=false \
  --title "Drahtseilakt levels $V" \
  --notes "The 13 picked maps for Drahtseilakt (data version $V). Programs and base data: the releases X.Y.Z." \
  "drahtseilakt-levels_$V-1_all.deb" "drahtseilakt-levels-$V-1.noarch.rpm" \
  levels/*.zip SHA256SUMS
```

Den Entwurf ansehen, dann veröffentlichen. Sonst ist nichts einzustellen: CI braucht weder Variable noch Secret noch einen
Download der Daten.

## Das Binär-Release (CI)

Actions -> „Drahtseilakt release“ -> Run workflow (die Dateien sind Lauf-Artefakte) oder einen Tag `X.Y.Z` pushen:
Der Lauf erzeugt dann auch ein **Entwurfs**-Release mit den Dateien unten und `SHA256SUMS` (ohne die Levels).
Die Eingabe `dev` fügt die Dev-Variante hinzu (nie in einem Release). Ein Push auf einen Branch startet nichts.

**Release-Assets haben stabile Namen**: Beim Tag-Release wird die Version aus den Dateinamen geschnitten (bei den Datenpaketen auch die Datenversion: `drahtseilakt_amd64.deb`, `drahtseilakt.x86_64.rpm`, `drahtseilakt-data_all.deb`, `drahtseilakt-windows-x86_64-setup.exe`), sodass `https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest/download/<Name>` immer das neueste Release liefert; die Version bleibt im Tag, im Titel und in den Paket-Metadaten. Lauf-Artefakte behalten die Namen mit `<V>` wie unten. Welche Datei für wen: [distribution.md](distribution.md).

Mit `<V>` = Tag (außerhalb eines Tags `git-<sha>`) entstehen diese Dateien:

| Ziel | Dateien |
|---|---|
| Windows x86_64, aarch64 | `drahtseilakt-<V>-windows-<arch>-setup.exe` (Installer), `drahtseilakt-<V>-windows-<arch>.zip` (portabel) |
| Linux x86_64, aarch64 | `drahtseilakt-<V>-linux-<arch>.tar.gz` (portabel), `drahtseilakt_<V>-1_<amd64\|arm64>.deb`, `drahtseilakt-<V>-1.<x86_64\|aarch64>.rpm` |
| Linux (x86_64-Job, einmal) | `drahtseilakt-data_<DATA_VERSION>-1_all.deb`, `drahtseilakt-data-<DATA_VERSION>-1.noarch.rpm` |
| Linux x86_64 DEV (Eingabe `dev`) | `drahtseilakt-<V>-linux-x86_64-dev.tar.gz`, `drahtseilakt-dev_<V>-1_amd64.deb` |
| macOS arm64, x86_64 | `drahtseilakt-<V>-macos-<arch>.dmg` (`.app` ohne Signatur), `drahtseilakt-<V>-macos-<arch>.tar.gz` (portabel) |
| FreeBSD x86_64 (experimentell) | `drahtseilakt-<V>-freebsd-x86_64.tar.gz` |

- **Portable Archive** und der **Windows-Installer** enthalten die Basisdaten und laufen daher eigenständig; die
  ausgewählten Karten sind die separaten `.zip`-Dateien aus dem Daten-Release (`data-<DATA_VERSION>`). Das portable Archiv enthält auch die Toolbox.
- **Windows-Installer** (NSIS, auf Ubuntu aus dem portablen zip gebaut): standardmäßig pro Benutzer
  (`%LOCALAPPDATA%\Programs\Drahtseilakt`, ohne Administrator), ein Startmenü-Ordner, ein Deinstallationsprogramm unter
  „Apps“. Er installiert keine `portable.txt`; die Spielerdaten liegen daher im Profil, und eine Deinstallation berührt
  sie nie. Installer, Deinstaller und Verknüpfungen tragen unser Icon (`scripts/packaging/icon.ico`, neben dem Programm installiert). Unsigniert: SmartScreen warnt. **Noch nicht:** das Icon in `drahtseilakt.exe` selbst (Explorer, Taskleiste einer portablen Kopie): dafür fehlt eine Build-Abhängigkeit zum Einbetten von Ressourcen im Offline-Crate-Cache; das Fenster hat das Icon (zur Laufzeit gesetzt).
- **Eine portable Kopie** (`portable.txt` neben dem Programm) sucht ihre Assets nur im eigenen `assets/`, nie in einem Quellbaum, `/usr/share` oder `XDG_DATA_DIRS`.
- **Linux-Pakete**: das Programm in `/usr/bin/drahtseilakt` (hängt von `libasound2`, `libudev1` ab; das .rpm von den Sonames `libasound.so.2`, `libudev.so.1`, `libc.so.6(GLIBC_2.35)`, die Fedora und openSUSE beide liefern, anders als Paketnamen wie `systemd-libs`; empfiehlt die Fenstersystem- und Grafikbibliotheken, die das Spiel zur Laufzeit lädt: `libvulkan1`, `libxkbcommon-x11-0`, `libx11-6`, `libxcursor1`, `libxi6`, `libwayland-client0`), die Daten
  dort, wo das Spiel sie sucht, `/usr/share/drahtseilakt/assets` (das Programm findet sie über `/usr/bin/../share`,
  dann in `$XDG_DATA_DIRS`: `files.md`). Eine `.desktop`-Datei und das Icon (`scripts/packaging/icon-512.png`, unsere Holzkugel, dasselbe Bild wie auf der Website und im Fenster; 512 px und eine 256-px-Kopie über `tools/icon.py`). `drahtseilakt` hängt von `drahtseilakt-data (>= DATA_VERSION)` ab;
  `drahtseilakt-levels` ist optional. Die Toolbox ist nicht enthalten (nur portabel).
  `drahtseilakt-dev` stellt `drahtseilakt` bereit und kollidiert damit (nur `.deb`, aus der Eingabe `dev`).
- **macOS**: die `.app` hält die Daten in `Contents/Resources/assets`; nicht signiert oder notarisiert, der erste Start
  braucht Rechtsklick -> Öffnen.

Installation unter Debian/Ubuntu: `sudo apt install ./drahtseilakt-data_V-1_all.deb ./drahtseilakt_X.Y.Z-1_amd64.deb`
(und `./drahtseilakt-levels_V-1_all.deb`); Fedora: `sudo dnf install ./drahtseilakt-data-V-1.noarch.rpm ./drahtseilakt-X.Y.Z-1.x86_64.rpm`.

## Festschreiben und Rechte

Actions sind per vollem Commit-SHA festgeschrieben, die Rust-Toolchain per exakter Version, Crates per `Cargo.lock`,
nfpm und Real-ESRGAN per SHA-256; der Workflow hat `contents: read`, nur der Release-Job `contents: write`. NSIS
kommt aus dem Paketarchiv von Ubuntu 24.04 (von der Distribution signiert, ohne Hash; die Version steht im Log).
