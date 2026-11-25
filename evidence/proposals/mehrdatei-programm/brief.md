Ein Rust-Kommandozeilenwerkzeug `taik-stats` als Cargo-Projekt aus mehreren Quelldateien bauen, das Entscheidungsprotokolle der Factory zusammenfasst.

Dies ist kein HTML-Auftrag. Liefere ein Bündel aus mehreren Dateien. Jede Datei beginnt mit einer eigenen Kopfzeile `=== pfad ===`. Danach folgt ihr Inhalt bis zur nächsten Kopfzeile. Die Antwort beginnt direkt mit einer Kopfzeile. Schreibe keine Erklärung und keine Code-Zäune. Die Antwort darf höchstens 96 KiB groß sein.

## Dateien

- `Cargo.toml`: Diese Datei ist Pflicht.
- `src/main.rs`: Diese Datei ist Pflicht.
- Mindestens eine weitere Rust-Datei unter `src/`, zum Beispiel `src/record.rs` oder `src/summary.rs`. Pfade bestehen aus Buchstaben, Ziffern und `_`, getrennt durch `/`, und enden auf `.rs`.
- Andere Dateien sind nicht erlaubt. Jeder Pfad darf nur einmal vorkommen.
- Jede weitere `.rs`-Datei muss wirklich gebraucht werden. Fehlt eine von ihnen, muss die Übersetzung von `src/main.rs` scheitern. Binde sie also über `mod` ein und verwende sie.

`Cargo.toml` enthält einen Abschnitt `[package]` mit `name = "taik-stats"`, `version = "0.1.0"` und `edition = "2021"`. Es gibt keine Abhängigkeiten. Ein Abschnitt `[dependencies]`, `[dev-dependencies]` oder `[build-dependencies]` darf höchstens leer vorkommen. Lege keinen `[[bin]]`-Abschnitt an. Das Binärprogramm heißt `taik-stats`.

## Übersetzung

Das Programm wird so übersetzt:

`rustc --edition 2021 -O -D warnings -F unsafe_code --crate-name taik_stats --crate-type bin src/main.rs`

Die Übersetzung muss ohne Warnungen gelingen. Verwende nur die Standardbibliothek und kein `unsafe`.

Ist `cargo` vorhanden, wird das Projekt zusätzlich mit `cargo build --offline` gebaut. Dabei gilt `RUSTFLAGS="-D warnings -F unsafe_code"`. Das so gebaute Programm muss sich genauso verhalten.

## Aufruf

`taik-stats DATEI.run [DATEI.run ...]`

Ein Protokoll besteht aus Zeilen der Form `schlüssel=wert` und endet mit der Zeile `end`. Nur das erste `=` einer Zeile trennt Schlüssel und Wert. Der Wert darf also selbst `=` enthalten.

Ausgewertet werden nur diese Schlüssel:

- `decision`
- `worker`
- `worker_cost_usd`
- `gate_ms`

Text in anderen Werten zählt nicht, auch wenn er wie ein Schlüssel aussieht. Ein Beispiel ist `note=decision=rejected worker=local`.

## Ausgabe

Die Ausgabe geht auf stdout und besteht aus genau sieben Zeilen. Jede Zeile endet mit einem Zeilenumbruch `\n`. Danach folgt nichts mehr. Beispiel:

```
records=5
accepted=3
rejected=1
other=1
cost_usd=1.150
gate_ms_max=13822
workers=claude-cli:2,codex-attached:1,local:1
```

Die Werte bedeuten:

- `records` ist die Zahl der übergebenen Dateien.
- `accepted` zählt die Protokolle mit `decision=accepted`. `rejected` zählt die Protokolle mit `decision=rejected`. Alle übrigen Protokolle zählen als `other`, auch solche ohne `decision`.
- `cost_usd` ist die Summe aller `worker_cost_usd`-Werte als Dezimalzahl mit genau drei Nachkommastellen. Ein fehlender oder nicht lesbarer Wert zählt als 0. Aus `0.1` und `0.2` wird zum Beispiel `0.300`.
- `gate_ms_max` ist der größte ganzzahlige `gate_ms`-Wert. Fehlende, leere oder nicht ganzzahlige Werte wie `99999.5` oder `abc` zählen als 0.
- `workers` listet jeden `worker`-Wert mit seiner Anzahl in der Form `name:anzahl`. Die Einträge sind nach Namen sortiert und durch Komma getrennt. Protokolle ohne `worker` fehlen in der Liste. Gibt es keinen Worker, bleibt der Wert leer, und die Zeile lautet `workers=`.

## Fehler

- Ohne Argumente endet das Programm mit Exit-Status 2. Es schreibt dann nichts auf stdout.
- Ist eine der Dateien nicht lesbar, etwa weil sie nicht existiert, nennt das Programm ihren Pfad auf stderr, genau so wie er übergeben wurde. Es schreibt dann nichts auf stdout und endet mit Exit-Status 2.
- Bei Erfolg ist der Exit-Status 0.
