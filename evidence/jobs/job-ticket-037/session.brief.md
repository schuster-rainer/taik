Das Programm auftragsbericht soll mit `--hilfe` einen Hilfetext ausgeben.

## Verhalten

- `auftragsbericht --hilfe` schreibt den Hilfetext auf die Standardausgabe und endet mit Status 0. Auf der Standardfehlerausgabe erscheint nichts.
- Der Hilfetext besteht aus genau vier Zeilen, jede mit Zeilenende:
  1. Die erste Zeile ist genau `Aufruf: auftragsbericht [--offen] [--json] ORDNER`.
  2. Danach folgt je eine Zeile für `--offen`, `--json` und `--hilfe`, in dieser Reihenfolge. Jede Zeile enthält den Optionsnamen und eine kurze deutsche Erklärung, was die Option tut.
- `--hilfe` gilt allein und zusammen mit beliebigen anderen Argumenten, in jeder Position. Dazu zählen `--offen`, `--json`, ein oder mehrere Ordner (auch nicht vorhandene) und unbekannte Optionen. Dann wird nur die Hilfe ausgegeben, mit Status 0 und ohne weitere Ausgabe. Es wird kein Ordner gelesen und kein Bericht erzeugt.
- Die Ausgabe ist in allen diesen Fällen identisch.

## Unverändert

- Die Fehlermeldung bei falschem Aufruf bleibt wie sie ist. Sie lautet `Aufruf: auftragsbericht ORDNER` auf der Standardfehlerausgabe, Status 2. Das gilt z. B. für den Aufruf ohne Argumente und für unbekannte Optionen ohne `--hilfe`.
- Alle übrigen Verhaltensweisen (Bericht, `--offen`, `--json`, Rückgabewerte 0/1/2) bleiben unverändert. Die vorhandenen Tests müssen weiter bestehen.

## Tests und README

- Ergänze in `tests/cli.rs` mindestens einen Test, dessen Name `hilfe` enthält. Er prüft den Hilfetext und den Status 0, außerdem `--hilfe` zusammen mit anderen Argumenten. Der Test muss mit `cargo test --offline --locked --test cli hilfe` laufen und bestehen.
- Ergänze das README: `--hilfe` im Abschnitt Aufruf und eine kurze Beschreibung der Hilfe. Den Hilfetext aus dem Abschnitt „Fehler und Rückgabewerte“ lässt du dabei unberührt.
