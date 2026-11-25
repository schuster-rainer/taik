Das Rust-Werkzeug run-diff soll zwei .run-Protokolle vergleichen und je abweichendem Schlüssel eine Zeile ausgeben.

## Protokollformat

Eine `.run`-Datei wird als UTF-8-Text gelesen. Zeilen sind durch `\n` getrennt; ein `\r` direkt vor `\n` gehört zum Zeilenende und wird entfernt.

- Leere Zeilen werden ignoriert.
- Zeilen ohne `=` werden ignoriert; `end` ist kein Schlüssel.
- Eine Zeile wird am **ersten** `=` geteilt: Alles davor ist der Schlüssel, alles danach der Wert.
- Schlüssel und Wert werden nicht getrimmt. Ein Wert darf leer sein oder weitere `=` enthalten.
- Kommt ein Schlüssel mehrfach vor, gilt die **letzte** Zeile.

## Ausgabe

Verglichen werden die Schlüssel beider Dateien. Für jeden Schlüssel gilt:

- nur in A: `key: A=<wert>`
- nur in B: `key: B=<wert>`
- in beiden, aber mit unterschiedlichem Wert: `key: A=<wertA> B=<wertB>`
- in beiden mit gleichem Wert: keine Zeile

`key` und `wert` werden exakt so ausgegeben, wie sie geparst wurden, ohne Escaping. Die Zeilen werden aufsteigend nach Schlüssel sortiert, in der Ordnung von Rust `String` (lexikographisch über UTF-8-Bytes). Jede Ausgabezeile endet mit `\n`, auch die letzte.

## Status und stderr

- Keine Abweichung, beide Dateien lesbar: Status 0, stdout leer.
- Mindestens eine Abweichung, beide Dateien lesbar: Status 1, die Zeilen auf stdout.
- Mindestens eine Datei nicht lesbar: Status 2, stdout leer. Auf stderr steht für jede nicht lesbare Datei der übergebene Pfad in einer eigenen Zeile, in Argumentreihenfolge; sonst steht nichts auf stderr.
- Bei lesbaren Dateien ist stderr leer.

## Programm und Abgabe

Aufruf: `run-diff A.run B.run`.

Abgabe als Bündel aus genau einer Datei. Das Dokument beginnt mit der Zeile `=== main.rs ===`; danach folgt der vollständige Rust-Quelltext. Keine weiteren Dateien, kein Cargo-Projekt. Das Programm wird mit `rustc` aus `main.rs` gebaut. Es darf nur die Rust-Standardbibliothek verwenden: keine externen Crates, kein Netzwerk.
