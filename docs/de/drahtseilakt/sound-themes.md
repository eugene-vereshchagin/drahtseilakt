# Klangthemen

Jeder Klang und jedes Musikstück in Drahtseilakt kommt aus einem **Klangthema**: einem Ordner mit Dateien, benannt nach
den **Rollen**, die das Spiel abspielt (der Sturz, ein Kontrollpunkt, die Holzkugel rollt auf Stein, die Musik eines
Levels...). Das Spiel liefert zwei Themen mit, und jeder kann ein weiteres machen: andere Aufnahmen, eine andere Musik,
ein Remaster. Diese Seite ist für Autoren von Themen: was in den Ordner gehört, was jede Rolle ist, wie lang und laut
der Klang des Originals für sie ist und wie man ein Thema so weitergibt, dass andere es nutzen können.

## Wo Themen liegen

- **Mitgeliefert**, in `assets/sounds/`: *system* (frei lizenzierte Aufnahmen und Musik, das Thema, auf das alle anderen
  zurückgreifen) und *neuro* (für das Spiel synthetisierte Klänge und Musik, die Voreinstellung).
- **Die eigenen des Spielers** im Datenordner unter `sounds/<name>/`: `~/.local/share/drahtseilakt/sounds/` unter
  Linux, `%APPDATA%\drahtseilakt\sounds\` unter Windows, `~/Library/Application Support/drahtseilakt/sounds/` unter
  macOS. Ein Thema wird installiert, indem man seinen Ordner dorthin kopiert.

Der Name des Ordners ist der Schlüssel des Themas (den die Einstellungen speichern): Buchstaben, Ziffern, `_`, `-` und
`.`, nicht mit einem Punkt am Anfang, höchstens 64 Zeichen. Ein Ordner des Spielers mit dem Namen eines mitgelieferten
Themas (`system`, `neuro`) wird übergangen; `original` ist der Name, den die Toolbox dem Thema aus der eigenen
Ballance-Kopie des Spielers gibt. Höchstens 32 Themen werden genommen. Das Spiel sucht Themen beim Start: nach dem
Hinzufügen neu starten.

## Der Ordner

```
sounds/
  calm/
    theme.txt          Name, Autor, Lizenz, Ersatzthema
    CREDITS.md         wer jede Datei gemacht hat, unter welcher Lizenz (empfohlen)
    Sounds/            die Effekte: Misc_Fall.ogg, Hit_Wood_Wood.ogg, ...
      sounds.txt       optional: mehrere Dateien je Rolle (Pools)
    Music/             die Musik: Music_Atmo_1.ogg, Music_Theme_1_1.ogg, ...
      music.txt        optional: mehrere Dateien je Rolle (Pools) oder Dateien mit beliebigen Namen
```

Die Datei einer Rolle ist `Sounds/<rolle>.ogg` oder `.wav` (oder dasselbe in `Music/`; das Spiel sieht in beiden nach).
Ein Thema muss nicht jede Rolle füllen: was ihm fehlt, kommt aus seinem Ersatzthema (siehe `theme.txt`).

Gelesen werden nur gewöhnliche Dateien: Namen aus Buchstaben, Ziffern, `_`, `-` und `.`; Verknüpfungen (symbolische
Links, Junctions) werden nie verfolgt; andere Unterordner als `Sounds/` und `Music/` werden übergangen.

## theme.txt

Reiner Text in UTF-8, höchstens 4 KiB, ein `Schlüssel Wert` je Zeile; Zeilen mit `#` am Anfang sind Kommentare.

| Schlüssel | Bedeutung |
|---|---|
| `name.en`, `name.ru`, `name.de`, `name.zh` | Der Name des Themas in den Einstellungen je Sprache (höchstens 64 Zeichen). Eine Sprache ohne eigenen zeigt den englischen, sonst den Ordnernamen. |
| `author` | Wer es gemacht hat (für Menschen; das Spiel zeigt es noch nicht). |
| `licence` | Die Lizenz, z. B. `CC0 1.0` oder `CC-BY 4.0, je Datei in CREDITS.md` (für Menschen). |
| `fallback` | Das Thema, aus dem fehlende Rollen kommen: ein Themenschlüssel oder `none` für kein anderes (das Thema ist vollständig). Ohne Angabe *system*. |

```
# Calm: sanfte Aufnahmen für lange Level.
name.en Calm
name.ru Тихая
name.de Ruhig
name.zh 平静
author Jane Doe
licence CC-BY 4.0 (CREDITS.md)
fallback system
```

Die Reihenfolge, in der eine Rolle gesucht wird: das gewählte Thema, sein Ersatzthema, dann *system* (`fallback none`:
nur das gewählte). Alle Dateien einer Rolle kommen aus einem Thema: dem ersten, das die Rolle überhaupt hat. Ein Klang,
den keines hat, kommt aus den eigenen Klängen des Levels oder aus der Kopie des Spielers (nur Effekte); Musik nie.

## Rollen

Die Werte des Originals unten sind an seinen Dateien gemessen: Länge und integrierte Lautheit (EBU R 128, LUFS), wo
der Klang lang genug zum Messen ist; sehr kurze sind als kurz angegeben. Sie sind ein Anhalt, keine Regel: ein Thema
darf insgesamt lauter oder leiser sein, aber seine Klänge sollten etwa diese Verhältnisse zueinander behalten, denn das
Spiel mischt sie wie das Original (Lautstärke und Geschwindigkeit des Rollgeräuschs folgen etwa der Geschwindigkeit der
Kugel).

**Schleife** heißt, das Spiel spielt die Datei immer wieder: sie muss sich ohne Naht wiederholen (ihr Ende läuft in
ihren Anfang, etwa durch eine Überblendung). Alle anderen Klänge sind **einmalig**.

### Effekte (`Sounds/`)

| Rolle | Wann sie spielt | Art | Original |
|---|---|---|---|
| `Misc_StartLevel` | Ein Lauf beginnt | einmalig | 4,2 s, −18 LUFS |
| `Misc_Checkpoint` | Ein Kontrollpunkt ist erreicht | einmalig | 3,5 s, −16 LUFS |
| `Misc_Fall` | Die Kugel ist verloren | einmalig | 5,6 s, −18 LUFS |
| `Misc_Lightning` | Eine neue Kugel erscheint | einmalig | 3,5 s, −25 LUFS |
| `Misc_Trafo` | Ein Transformator wechselt die Kugel | einmalig | 3,1 s, −22 LUFS |
| `Extra_Life_Blob` | Das Extraleben wird berührt | einmalig | 0,3 s, kurz |
| `Misc_extraball` | Das Extraleben ist gewonnen (0,3 s nach `Extra_Life_Blob`) | einmalig | 0,9 s, kurz |
| `Extra_Start` | Extrapunkte werden freigesetzt | einmalig | 3,0 s, −28 LUFS |
| `Extra_Hit` | Ein Extrapunkt wird eingesammelt | einmalig | 0,5 s, −36 LUFS |
| `Misc_RopeTears` | Ein Seil reißt | einmalig | 0,2 s, kurz |
| `Misc_Ventilator` | Ein Ventilator bläst (Lautstärke nach Entfernung) | Schleife | 1,1 s, −23 LUFS |
| `Music_thunder` | Donner nach einem Blitz in Gewitterleveln (trotz des Namens ein Effekt: Effektlautstärke) | einmalig | 12 s, −18 LUFS |
| `Menu_click` | Eine Wahl im Menü | einmalig | 0,03 s, kurz |
| `Menu_load` | Ein Level wird geladen | einmalig | 5,2 s, −19 LUFS |
| `Roll_<kugel>_<boden>` | Die Kugel rollt (Lautstärke und Geschwindigkeit folgen ihrer Geschwindigkeit) | Schleife | 1–2 s, −27 bis −41 LUFS |
| `Hit_<kugel>_<boden>` | Die Kugel stößt an Boden oder Wand | einmalig | 0,1–0,3 s, kurz |

Die Kugeln: `Wood`, `Stone` und `Paper`; die Böden: `Stone`, `Wood`, `Metal`. Rollen: `Roll_Wood_Stone`,
`Roll_Wood_Wood`, `Roll_Wood_Metal`, `Roll_Stone_Stone`, `Roll_Stone_Wood`, `Roll_Stone_Metal` und `Roll_Paper` für die
Papierkugel auf jedem Boden. Stöße: `Hit_Wood_Stone`, `Hit_Wood_Wood`, `Hit_Wood_Metal`, `Hit_Stone_Stone`,
`Hit_Stone_Wood`, `Hit_Stone_Metal`, die Kuppeln `Hit_Wood_Dome` (2,0 s, −18 LUFS) und `Hit_Stone_Kuppel` (1,2 s,
−21 LUFS) und `Hit_Paper` für die Papierkugel. Die Rollschleifen des Originals von leise nach laut: Papier und Holz auf
Stein etwa −41, Holz auf Metall −37, Stein auf Stein −36, Stein auf Holz −29, Holz auf Holz −27, Stein auf Metall
−28 LUFS.

Ein Level kann eigene Effekte mitbringen (der Ordner `Sounds/` eines Community-Levels); sie kommen vor jedem Thema.

### Musik (`Music/`)

Die Musik hat eine eigene Lautstärkeeinstellung. Wie ein Level sie spielt (wie der Musikmanager des Originals):

| Rolle | Wann sie spielt | Art | Original |
|---|---|---|---|
| `Music_Atmo_1`, `_2`, `_3` | Die Atmosphäre: vom Beginn eines Laufs an eine von ihnen nach einer zufälligen Pause bis 15 s, immer wieder | einmalig | 26–28 s, −30 bis −34 LUFS, ein langsames An- und Abschwellen, ohne Rhythmus und Bass |
| `Music_Theme_<n>_1`, `_2`, `_3` | Das Thema des Levels (`n` aus der Leveltabelle, 1 bis 5 in den Original-Leveln): ab 7 s einer seiner Teile nach einer zufälligen Pause bis 50 s, immer wieder; endet nach dem letzten Kontrollpunkt | einmalig | 4–23 s, −13 bis −19 LUFS |
| `Music_EndCheckpoint` | Nach dem letzten Kontrollpunkt nahe dem Levelende, bis zum Ziel | Schleife | 14 s, −13 LUFS |
| `Music_Final` | Das Level ist geschafft | einmalig | 26,5 s, −16 LUFS |
| `Music_LastFinal` | Das letzte Level ist geschafft (statt `Music_Final`) | einmalig | 7,7 s, −17 LUFS |

Atmosphäre und Thema spielen gleichzeitig, jedes mit eigenen Pausen, daher sollte die Atmosphäre sehr leise und ruhig
sein. Die Thementeile des Originals sind kurze Phrasen; längere Stücke gehen auch (die von *system* sind ganze Stücke bis
100 s bei etwa −17 LUFS), aber jedes Stück liegt während eines Levels dekodiert im Speicher (etwa 35 MB für 100 s
Stereo), also besser unter etwa zwei Minuten. Höchstens 384 MB dekodierte
Musik werden zugleich gehalten; ein Stück darüber oder eines, das sich nicht dekodieren lässt, fällt für den Lauf aus
seinem Pool, und ein anderes spielt. `Music_Highscore` (der 2-s-Jingle des Originals) wird angenommen, aber
noch nicht gespielt.

## Pools: mehrere Dateien für eine Rolle

Eine Rolle kann mehrere Dateien haben, einen **Pool**: jedes Mal, wenn das Spiel die Rolle spielt, wählt es zufällig
eine davon, nie dieselbe zweimal hintereinander. Ein Pool wird durch die Tabellen `Music/music.txt` und
`Sounds/sounds.txt` angegeben, beide im selben Format:

```
# rolle datei [datei ...]
Music_Atmo_1 bowl.ogg flute.ogg
Music_Atmo_1 wind.ogg
Music_Theme_4_1 river.ogg
Music_Theme_4 garden.ogg dusk.ogg
Music_Final fanfare_a.ogg fanfare_b.ogg
```

- Eine Zeile je Rolle: zuerst die Rolle, dann ein oder mehrere Dateinamen, getrennt durch Leerzeichen oder Tabs. Eine
  Rolle auf mehreren Zeilen sammelt alle. Die alten Zeilen mit einer Datei (`Music_Theme_1_1 piece.ogg`) sind dasselbe
  Format.
- Die Datei mit dem Namen der Rolle (`Music/Music_Atmo_1.ogg`), falls vorhanden, ist die erste ihres Pools; die
  Tabellen fügen hinzu. Eine Tabelle nennt nur Dateien ihres eigenen Ordners: `music.txt` die in `Music/`,
  `sounds.txt` die in `Sounds/`.
- Atmosphäre und Levelthema wählen unter **allen** Dateien ihrer Teile zusammen: die Atmosphäre unter den Pools von
  `Music_Atmo_1`, `_2`, `_3` und der Gruppe `Music_Atmo`; Thema `n` unter denen von `Music_Theme_<n>_1` bis `_3` und der
  Gruppe `Music_Theme_<n>`. So kann ein Thema beliebig viele Atmosphären und Stücke haben, auf Teile verteilt oder
  nicht. Eine Datei in mehreren zählt einmal.
- `Music_EndCheckpoint`, `Music_Final` und `Music_LastFinal` wählen je Level eine aus ihrem Pool; jeder Effekt wählt
  bei jedem Abspielen eine aus seinem Pool (etwa mehrere Stoßklänge). Ein eigener Effekt des Levels ersetzt den Pool.
- Die Wahl der Musik folgt einem Startwert: eine Aufzeichnung (Replay) spielt ihre Musik jedes Mal in derselben
  Reihenfolge, ein neuer Lauf in einer neuen. Die Musik verändert das Spiel selbst nie.
- Rollennamen werden ohne Rücksicht auf Groß- und Kleinschreibung verglichen, Dateinamen genau.

Die Tabellen werden als nicht vertrauenswürdige Dateien gelesen (Themen werden von Fremden geteilt) und streng geprüft:

- höchstens 16 KiB und 512 Zeilen, in UTF-8 (eine Byte-Order-Mark am Anfang ist erlaubt); eine größere oder nicht-UTF-8-Tabelle wird ganz übergangen;
- Dateinamen sind einfache Namen mit `.ogg` oder `.wav` am Ende (Buchstaben, Ziffern, `_`, `-`, `.`, nicht mit einem
  Punkt am Anfang): keine Ordner, Pfade oder Laufwerksbuchstaben. Eine Zeile, die etwas anderes nennt, wird ganz
  übergangen;
- eine Rolle, die das Spiel nicht kennt, wird übergangen; höchstens 16 Dateien je Rolle und 256 Rollen;
- eine genannte, aber fehlende Datei (oder eine Verknüpfung) wird weggelassen.

Jede übergangene Zeile wird im Terminal gemeldet (Standardfehler), z. B.
`sound theme table .../Music/music.txt: line 3: My_Song is not a role the game plays: ignored`.

## Formate

- **Ogg Vorbis** (`.ogg`, bevorzugt) oder unkomprimiertes PCM-**WAV** (`.wav`, 8 oder 16 Bit); mono oder stereo; jede
  Abtastrate von 1 bis 192 kHz (44,1 kHz empfohlen; das Spiel rechnet um). Mono reicht für Effekte, Stereo passt zur
  Musik.
- Eine Datei hat höchstens 64 MB und dekodiert zu höchstens zehn Minuten. Eine defekte Datei wird zugunsten der des
  nächsten Themas übergangen.
- Ogg-Vorbis-Qualität 3–5 (`ffmpeg -c:a libvorbis -q:a 4`) genügt.
- Lautheit durch Messen angleichen, z. B. `ffmpeg -i file.ogg -af ebur128 -f null -` (das letzte `I:` ist die
  integrierte Lautheit). `prepare.py` von *system* (in `assets/sounds/system/`) zeigt eine vollständige Vorbereitung:
  Schnitt, Blenden, nahtlose Schleifen, Lautheitsangleich und Kodierung.

## Ein Thema testen

1. Den Ordner in `sounds/` des Datenordners legen und das Spiel starten.
2. In den erweiterten Modus wechseln und das Thema in der Zeile **Sounds** der erweiterten Einstellungsseite wählen (die
   Einstellung `sound-theme NAME` in `settings.txt`). Der klassische Modus hat dieselbe Wahl auf seiner Seite Sound (Zeile **Sound theme**).
3. Ein Level spielen und das Terminal auf Tabellenhinweise beobachten. Eine fehlende Rolle füllt das Ersatzthema, daher
   sollte ein Thema mit `fallback none` auf Lücken geprüft werden: alle Rollen der Tabellen oben.
4. Zum Anhören ohne zu spielen eine Aufzeichnung abspielen und die Mischung in eine Datei schreiben:
   `DRAHTSEILAKT_SOUND_FILE=mix.wav drahtseilakt --replay run.replay` (die Mischung geht nach `mix.wav` statt an die
   Lautsprecher).

## Lizenzen und Weitergabe

- Eigene Aufnahmen und Musik verwenden oder Werke unter **CC0** oder **CC-BY** (3.0 oder 4.0), damit jeder das Thema
  weitergeben kann. NonCommercial (NC) und NoDerivatives (ND) meiden: das Spiel schneidet und pegelt jede Datei.
- Eine `CREDITS.md` ins Thema legen: je Datei ihre Rolle, Titel des Werks, Autor, Lizenz, Quell-URL und was geändert
  wurde (geschnitten, geblendet, gepegelt, gefiltert). CC-BY verlangt diese Namensnennung; die `CREDITS.md` von *system*
  ist ein Beispiel.
- **Niemals Klänge oder Musik des Originalspiels** (oder eines anderen Spiels) aufnehmen, auch nicht bearbeitet. Die
  Klänge des Originals bleiben auf dem Rechner jedes Spielers: die Toolbox macht das Thema *original* lokal aus der
  eigenen Kopie des Spielers, und es wird nie geteilt.

## Die mitgelieferten Themen

- *system* (`assets/sounds/system/`): frei lizenzierte Effekte (Kenney, Autoren von freesound und OpenGameArt), acht
  Atmosphären in drei Teilen, achtzehn Levelstücke (Themen 1 bis 3 je drei, Themen 4 und 5 neun weitere in Pools) und
  die Zielmusik; gemacht von `prepare.py`; Autoren, Lizenzen und Änderungen in ihrer `CREDITS.md`.
- *neuro* (`assets/sounds/neuro/`): jede Rolle für das Spiel durch Code synthetisiert, GPL-3.0-only, `fallback none`.
