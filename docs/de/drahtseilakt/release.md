# Releases und Pakete

Ein Release besteht aus drei Dingen, die sich unterschiedlich schnell ändern; sie werden getrennt gebaut und
ausgeliefert: ein Update des Programms baut nur die Binärpakete neu.

| Was | Ändert sich | Gebaut von | Veröffentlicht als |
|---|---|---|---|
| **Binärpakete**: das Programm | mit jeder Version | Workflow `Drahtseilakt release` (CI) | Release mit Tag `X.Y.Z` (Entwurf) |
| **Basisdaten**: `assets/` ohne die 13 ausgewählten Karten | selten | `scripts/pack-assets.sh` (Rechner des Besitzers) | Release mit Tag `data-<DATA_VERSION>` |
| **Levels**: die 13 ausgewählten Karten | selten | `scripts/pack-assets.sh` (Rechner des Besitzers) | dasselbe Release `data-<DATA_VERSION>` |

Daten und Levels liegen nicht in git („keine Binärdateien in git“) und lassen sich daher nicht in CI bauen; sie
werden auf dem Rechner des Besitzers aus dem lokalen `assets/` erzeugt und von Hand hochgeladen.

## Basisdaten

`assets/` ohne die ausgewählten Karten (`assets/levels/drahtseilakt/NN-*`) und ohne reine Entwicklungsdateien
(`mods-dev`, `own/make`): eigene Teile, Schriften, Klänge, Werksmods, das Handbuch, der Showcase-Level
(`00-Showcase`, `Parts_Test`) und die 13 Community-`.nmo`-Karten (`assets/levels/community`), an die das Spiel fest
gebunden ist. Etwa 93 MB. Nie eine Datei des originalen Ballance.

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

## Daten- und Level-Pakete erzeugen

```sh
scripts/pack-assets.sh          # OUT defaults to ../drahtseilakt-portable/bundles
```

Es erzeugt in OUT (`V` = `scripts/DATA_VERSION`):

| Datei | Wofür |
|---|---|
| `drahtseilakt-data-V.tar.gz` | die übrigen Systeme; CI lädt es für portable Archive, Installer, .app |
| `drahtseilakt-data_V-1_all.deb`, `drahtseilakt-data-V-1.noarch.rpm` | Linux, Basisdaten |
| `levels/NN-Name.zip` (13) | jedes System: nach `assets/levels/drahtseilakt/` entpacken |
| `drahtseilakt-levels_V-1_all.deb`, `drahtseilakt-levels-V-1.noarch.rpm` | Linux, alle 13 Karten (hängt vom Datenpaket ab) |
| `SHA256SUMS` | alles oben Genannte |

`.deb` und `.rpm` baut [nfpm](https://nfpm.goreleaser.com) 2.49.0, ein Werkzeug für beide; dasselbe Skript läuft so
auf dem Rechner des Besitzers (kein `rpmbuild` nötig) und in CI. `scripts/fetch-nfpm.sh` lädt es nach
`third_party/` (gitignoriert; nie in Systempfade) und prüft die im Skript festgeschriebene SHA-256 (aus der
`checksums.txt` des Releases). `scripts/pack-assets.sh --no-packages` überspringt die Pakete.

## Das Daten-Release hochladen

Vom Besitzer aus dem Checkout auszuführen; nichts hier läuft von selbst:

```sh
V=$(cat scripts/DATA_VERSION)
cd ../drahtseilakt-portable/bundles && sha256sum --check --quiet SHA256SUMS
gh release create "data-$V" --repo eugene-vereshchagin/drahtseilakt --draft --latest=false \
  --title "Drahtseilakt data $V" \
  --notes "Base data and the 13 picked maps for Drahtseilakt (data version $V). Programs: the releases X.Y.Z." \
  "drahtseilakt-data-$V.tar.gz" \
  "drahtseilakt-data_$V-1_all.deb" "drahtseilakt-data-$V-1.noarch.rpm" \
  "drahtseilakt-levels_$V-1_all.deb" "drahtseilakt-levels-$V-1.noarch.rpm" \
  levels/*.zip SHA256SUMS
```

Den Entwurf ansehen, dann veröffentlichen. Danach die CI auf die neuen Basisdaten zeigen lassen
(Repository-Variablen, einmal je Datenversion). In einem privaten Repository wird ein Release-Asset über seine
API-URL geladen (als Bearer geht `ASSETS_TOKEN`, sonst das eigene Token des Laufs):

```sh
V=$(cat scripts/DATA_VERSION)
URL=$(gh api "repos/eugene-vereshchagin/drahtseilakt/releases/tags/data-$V" \
  --jq ".assets[] | select(.name == \"drahtseilakt-data-$V.tar.gz\") | .url")
SUM=$(sha256sum "../drahtseilakt-portable/bundles/drahtseilakt-data-$V.tar.gz" | cut -d' ' -f1)
gh variable set ASSETS_BUNDLE_URL --repo eugene-vereshchagin/drahtseilakt --body "$URL"
gh variable set ASSETS_BUNDLE_SHA256 --repo eugene-vereshchagin/drahtseilakt --body "$SUM"
```

(Bei einem öffentlichen Release geht auch die normale Download-URL
`https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/data-$V/drahtseilakt-data-$V.tar.gz`.)
`scripts/ci-assets.sh` warnt, wenn die URL die Version aus `scripts/DATA_VERSION` nicht enthält.

## Das Binär-Release (CI)

Actions -> „Drahtseilakt release“ -> Run workflow (die Dateien sind Lauf-Artefakte) oder einen Tag `X.Y.Z` pushen:
Der Lauf erzeugt dann auch ein **Entwurfs**-Release mit den Dateien unten und `SHA256SUMS` (ohne Daten und Levels).
Die Eingabe `dev` fügt die Dev-Variante hinzu (nie in einem Release). Ein Push auf einen Branch startet nichts.

Mit `<V>` = Tag (außerhalb eines Tags `git-<sha>`) entstehen diese Dateien:

| Ziel | Dateien |
|---|---|
| Windows x86_64, arm64 | `drahtseilakt-<V>-windows-<arch>-setup.exe` (Installer), `drahtseilakt-<V>-windows-<arch>.zip` (portabel) |
| Linux x86_64, aarch64 | `drahtseilakt-<V>-linux-<arch>.tar.gz` (portabel), `drahtseilakt_<V>-1_<amd64\|arm64>.deb`, `drahtseilakt-<V>-1.<x86_64\|aarch64>.rpm` |
| Linux x86_64 DEV (Eingabe `dev`) | `drahtseilakt-<V>-linux-x86_64-dev.tar.gz`, `drahtseilakt-dev_<V>-1_amd64.deb` |
| macOS arm64, x86_64 | `drahtseilakt-<V>-macos-<arch>.dmg` (`.app` ohne Signatur), `drahtseilakt-<V>-macos-<arch>.tar.gz` (portabel) |
| FreeBSD x86_64 (experimentell) | `drahtseilakt-<V>-freebsd-x86_64.tar.gz` |

- **Portable Archive** und der **Windows-Installer** enthalten die Basisdaten und laufen daher eigenständig; die
  ausgewählten Karten sind die separaten `.zip`-Dateien aus dem Daten-Release. Das portable Archiv enthält auch die Toolbox.
- **Windows-Installer** (NSIS, auf Ubuntu aus dem portablen zip gebaut): standardmäßig pro Benutzer
  (`%LOCALAPPDATA%\Programs\Drahtseilakt`, ohne Administrator), ein Startmenü-Ordner, ein Deinstallationsprogramm unter
  „Apps“. Er installiert keine `portable.txt`; die Spielerdaten liegen daher im Profil, und eine Deinstallation berührt
  sie nie. Unsigniert: SmartScreen warnt.
- **Linux-Pakete**: das Programm in `/usr/bin/drahtseilakt` (Systembibliotheken: `libasound2`, `libudev1`), die Daten
  dort, wo das Spiel sie sucht, `/usr/share/drahtseilakt/assets` (das Programm findet sie über `/usr/bin/../share`,
  dann in `$XDG_DATA_DIRS`: `files.md`). Eine `.desktop`-Datei und das Icon (das Favicon der Site, skaliert von
  `tools/icon.py`, keine neue Datei in git). `drahtseilakt` hängt von `drahtseilakt-data (>= DATA_VERSION)` ab;
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
