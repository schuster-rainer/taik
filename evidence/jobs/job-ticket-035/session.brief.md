Das Programm auftragsbericht soll die Option `--json` bekommen, die den Bericht als ein JSON-Objekt ausgibt.

## Gewünschtes Verhalten

`auftragsbericht --json ORDNER` gibt statt der Tabellenzeilen genau ein JSON-Objekt auf der Standardausgabe aus, gefolgt von genau einem Zeilenende (`\n`). Das Objekt hat diese Form:

```
{ "auftraege": [ { "name": ..., "runden": ..., "entscheidung": ..., "kosten": ... }, ... ],
  "gesamt": { "auftraege": ..., "angenommen": ..., "kosten": ... } }
```

- `name`: Name des Auftragsordners (JSON-Zeichenkette).
- `runden`: Zahl der Runden (JSON-Zahl).
- `entscheidung`: letzte Entscheidung oder `"-"`, wenn es keine Runde gibt.
- `kosten`: JSON-Zahl mit höchstens zwei Nachkommastellen (z. B. 0.3, nicht 0.30000000000000004; 1 oder 1.0 sind beide in Ordnung).
- `gesamt.auftraege`: Zahl der ausgegebenen Aufträge.
- `gesamt.angenommen`: Zahl der ausgegebenen Aufträge mit letzter Entscheidung `accepted`.
- `gesamt.kosten`: Summe der Kosten, ebenfalls höchstens zwei Nachkommastellen.
- Die Aufträge sind nach Name sortiert, wie bisher. Bei leerem `ORDNER` ist `auftraege` eine leere Liste und `gesamt` enthält nur Nullen.
- Die Option lässt sich mit `--offen` verbinden, in beliebiger Reihenfolge (`--json --offen ORDNER` und `--offen --json ORDNER`). Dann enthält die Liste nur die Aufträge, deren letzte Entscheidung nicht `accepted` ist, und `gesamt` zählt nur diese.
- Zeichen in Namen und Entscheidungen, die JSON nicht roh erlaubt (Anführungszeichen, Rückstrich, Steuerzeichen unter U+0020), müssen korrekt maskiert werden, sodass die Ausgabe gültiges JSON ist und die Werte nach dem Parsen unverändert sind.
- Whitespace und Reihenfolge der Schlüssel im JSON sind frei.

## Unverändert zu lassen

- Ohne `--json` bleibt die Ausgabe byte-genau wie bisher, ebenso `--offen` allein.
- Rückgabewerte und Fehlermeldungen bleiben: 1 bei fehlendem oder unlesbarem ORDNER (auch mit `--json`, dann nichts auf der Standardausgabe), 2 bei falschem Aufruf (z. B. `--json` ohne ORDNER, unbekannte Option) mit der Zeile `Aufruf: auftragsbericht ORDNER`.
- Keine fremden Abhängigkeiten: `[dependencies]` in `Cargo.toml` bleibt leer, `Cargo.lock` enthält nur das Paket `auftragsbericht`. Das JSON wird von Hand erzeugt.
- Bestehende Tests bleiben gültig.

## Zusätzlich

- Ergänze in `README.md` die Option `--json` (Aufruf, Form der Ausgabe, Kombination mit `--offen`, Maskierung).
- Ergänze Tests (in `tests/cli.rs` und/oder als Unit-Tests in `src/lib.rs`) für `--json`, die Kombination mit `--offen` und die Maskierung.
- Bauen und Testen geht mit `cargo build --offline --locked`.
