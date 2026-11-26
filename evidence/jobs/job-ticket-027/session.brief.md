Im Repository auftragsbericht entsteht ein Rust-Kommandozeilenprogramm, das die Auftragsordner einer Factory zusammenfasst.

## Projekt

- Cargo-Projekt, Paket- und Programmname `auftragsbericht`, Edition 2021, keine fremden Abhängigkeiten.
- `Cargo.lock` muss mit eingecheckt sein. Das Projekt muss mit `cargo build --offline --locked` bauen.
- Die vorhandene `README.md` wird ersetzt (siehe unten). Sonst bleibt nichts anderes bestehen, was geändert werden müsste.

## Aufruf

`auftragsbericht ORDNER`

ORDNER enthält Auftragsordner, deren Name mit `job-` beginnt. Ordner mit anderem Namen und Dateien direkt in ORDNER werden übergangen, auch wenn eine Datei mit `job-` beginnt.

## Protokolle

In einem Auftragsordner liegen Protokolle:
- Dateien `round-NN.run` (NN zwei Ziffern),
- beliebige weitere Dateien mit der Endung `.run`.

Ein Protokoll besteht aus Zeilen `schlüssel=wert`. Dateien ohne Endung `.run` zählen nirgends mit.

## Zählung je Auftrag

- **Runden**: Anzahl der Dateien `round-NN.run`.
- **Letzte Entscheidung**: Wert von `decision=` in der Runde mit der höchsten Nummer. Gibt es keine Runde, ist es `-`.
- **Kosten**: Summe aller Werte von `worker_cost_usd=` und `session_listed_usd=` über alle `.run`-Dateien des Auftragsordners. Ein Wert, der keine Zahl ist, zählt als 0.

## Ausgabe (Standardausgabe)

- Aufträge nach Name sortiert, je Auftrag eine Zeile mit vier durch Tabulator getrennten Feldern: Name, Runden, letzte Entscheidung, Kosten.
- Kosten mit genau zwei Nachkommastellen und Punkt als Dezimaltrenner.
- Danach als letzte Zeile vier durch Tabulator getrennte Felder: `gesamt`, Zahl der Aufträge, Zahl der Aufträge mit letzter Entscheidung `accepted`, Summe der Kosten (zwei Nachkommastellen).

Beispiel (Tabulatoren hier als Leerraum):

```
job-a	2	accepted	3.75
job-b	1	rejected	0.50
gesamt	2	1	4.25
```

## Fehler

- Ohne Argument oder mit mehr als einem Argument: `Aufruf: auftragsbericht ORDNER` auf der Fehlerausgabe, Status 2.
- ORDNER ist kein lesbarer Ordner (fehlt oder ist eine Datei): Meldung auf der Fehlerausgabe, Status 1.
- Leerer ORDNER: nur die Zeile `gesamt`, `0`, `0`, `0.00` (tabulatorgetrennt), Status 0.

## Tests und README

- Tests im Projekt, die `cargo test --offline --locked` erfolgreich ausführt. Es muss mindestens ein Test laufen, der die Zählung prüft.
- `README.md` auf Deutsch mit Zweck, Aufruf und einem Beispiel der Ausgabe. Sie nennt den Namen `auftragsbericht`.
