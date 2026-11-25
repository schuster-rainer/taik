Ein Rust-Kommandozeilenwerkzeug `taik-stats` bauen, das Entscheidungsprotokolle der Factory zusammenfasst.

Dies ist kein HTML-Auftrag. Liefere genau eine Rust-Quelldatei als Bündel. Die Antwort beginnt mit der Zeile `=== main.rs ===`, danach folgt nur der Quelltext. Keine Erklärung, keine Code-Zäune.

Das Gate übersetzt die Datei mit `rustc --edition 2021 -O -D warnings` ohne Abhängigkeiten und führt das Programm aus. Nur die Standardbibliothek. Kein `unsafe`, keine Prozesse, kein Netzwerk.

Aufruf: `taik-stats DATEI.run [DATEI.run ...]`

Ein Protokoll besteht aus Zeilen `schlüssel=wert` und endet mit `end`. Nur das erste `=` einer Zeile trennt Schlüssel und Wert. Ausgewertet werden die Schlüssel `decision`, `worker`, `worker_cost_usd` und `gate_ms`. Text in anderen Werten zählt nicht, auch wenn er wie ein Schlüssel aussieht.

Ausgabe auf stdout, genau diese sieben Zeilen:

```
records=5
accepted=3
rejected=1
other=1
cost_usd=1.150
gate_ms_max=13822
workers=claude-cli:2,codex-attached:1,local:1
```

- `records` ist die Zahl der Dateien. `accepted` und `rejected` zählen `decision`, alles andere ist `other`.
- `cost_usd` ist die Summe aller `worker_cost_usd` mit drei Nachkommastellen. Fehlt der Wert, zählt er als 0.
- `gate_ms_max` ist der größte ganzzahlige `gate_ms`-Wert. Fehlende oder nicht ganzzahlige Werte zählen als 0.
- `workers` listet jeden `worker` mit seiner Anzahl, nach Namen sortiert, mit Komma getrennt. Protokolle ohne `worker` fehlen in der Liste. Gibt es keinen, bleibt der Wert leer.

Fehler: Ohne Argumente endet das Programm mit Status 2 und schreibt nichts auf stdout. Ist eine Datei nicht lesbar, nennt es ihren Pfad auf stderr, schreibt nichts auf stdout und endet mit Status 2.
