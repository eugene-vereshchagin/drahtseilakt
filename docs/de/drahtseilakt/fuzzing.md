# Fuzzing der Parser

Jeder Leser einer Datei, die das Spiel nicht selbst geschrieben hat — `.nmo`/`.cmo`-Level und Community-Karten, das
offene Levelformat, Bilder, Klänge, das CD-Abbild und sein InstallShield-Archiv, Replays, Hinweisdateien, die
`level.txt` der Levelordner, Klangthemen, die Einstellungs- und Highscore-Dateien — hat ein Fuzz-Ziel. Es gibt zwei
Wege, sie laufen zu lassen.

## Der stabile Fuzzer (läuft überall, offline)

`crates/fuzzkit` ist ein kleiner Mutations-Fuzzer ohne Abhängigkeiten. Jedes Ziel ist ein `#[ignore]`-Test neben den
übrigen Tests des Parsers; er mutiert einen Korpus von Startdateien (Bits kippen, interessante Werte, Einfügen, Löschen,
Kopieren, Zusammenfügen, Abschneiden, Längenfelder verschieben, bei Textformaten Zeilen, Wörter und Zahlen) mit einem
festen xorshift-Generator, so dass jede Eingabe aus ihrer Nummer wieder entsteht. Jede Eingabe läuft unter Aufsicht:
eine Panik wird abgefangen, eine Eingabe über der Frist oder über der Speichergrenze des Ziels beendet den Lauf, und ein
Abbruch (eine Speicheranforderung, die an der Adressraumgrenze scheitert, ein Stapelüberlauf) wird durch Wiederholen
des letzten Abschnitts wiedergefunden.

```sh
scripts/fuzz.sh                    # alle Ziele, je 600 s, 4 gleichzeitig
scripts/fuzz.sh 3600 2 nmo_file    # ein Ziel, eine Stunde
FUZZ_SECS=60 cargo test --release --offline -p level --test fuzz fuzz_nmo_file -- --ignored --nocapture
```

Ziele: `nmo_file`, `nmo_chunk`, `open_level` (Crate `level`); `image`, `level_txt`, `theme_txt`, `sound_table`
(`gamedata`); `disc_image`, `cue_sheet`, `iscab` (`cdimage`); `replay`, `hints`, `wav`, `ogg`, `highscore`,
`settings`, `level_sim` — ein Level geladen und eine Sekunde gespielt, womit eine fremde Karte über den Leser
hinaus in Physik und Regeln gelangt — und `module_sim` — eine Moduldatei anstelle einer, die ein Original-Level
benutzt (ein Levelordner darf eigene mitbringen), mit der Kopie des Spielers (das Spiel, `crates/game/src/fuzz_tests.rs`).

Startdateien sind kleine synthetische Dateien der Tests, die mitgelieferten Dateien des Spiels und — falls vorhanden —
die Dateien des Spielers (`OPENBALLANCE_DATA`, `OPENBALLANCE_MAPS`, die Original- und Community-Level des Profils), zur
Laufzeit gelesen und nie ins Repository geschrieben. `FUZZ_NO_REAL=1` lässt sie weg.

Einstellungen: `FUZZ_SECS` (Zeit je Ziel), `FUZZ_SEED` (welche Eingaben; `scripts/fuzz.sh` nimmt jedes Mal einen neuen),
`FUZZ_TIMEOUT_MS` (Frist einer Eingabe, 10 s), `FUZZ_RSS_MB` (Speichergrenze), `FUZZ_REPRO=<Datei>` (eine Eingabe
laufen lassen und sagen, was sie tut).

Funde landen in `fuzz/artifacts/<Ziel>/` (von git ignoriert): `crash-*`, `hang-*`, `oom-*` oder `abort-*`, jeweils
`.bin` mit einer `.txt`, die sagt, was geschah, und einer minimierten `.min.bin`. Eine Korrektur bekommt einen
Regressionstest neben dem Parser und eine synthetische Eingabe (nie Spieldaten) in `fuzz/regressions/<Ziel>/`, die
die gewöhnlichen Tests abspielen.

## cargo-fuzz (libFuzzer, nightly)

`fuzz/` ist ein cargo-fuzz-Crate außerhalb des Workspace (der Build des Spiels braucht es nie). Seine Ziele rufen
dieselben Funktionen auf wie die Tests des stabilen Fuzzers:

```sh
FUZZ_DUMP_CORPUS=1 cargo test --release --offline -p level --test fuzz fuzz_nmo_file -- --ignored  # Startdateien -> fuzz/corpus/
cd fuzz && cargo +nightly fuzz run nmo_file corpus/nmo_file
```

Das Spiel ist ein Binär-Crate; von seinen eigenen Parsern hat daher nur `replay` (der für sich steht) ein
cargo-fuzz-Ziel, die übrigen (`hints`, `wav`, `ogg`, `highscore`, `settings`, `level_sim`, `module_sim`) laufen auf dem stabilen Fuzzer.
