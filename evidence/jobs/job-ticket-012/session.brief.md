Der Befehl `taik-seed settle STATE job-NAME` übernimmt die angenommene Sitzung über dieselbe Funktion wie `adopt` und hält das Ergebnis im Auftragsordner fest.

## Ausgangslage

Nach einer angenommenen Agentensitzung muss man heute `taik-seed adopt STATE job-NAME` von Hand aufrufen. Die Funktion dafür ist `adopt::adopt` in `src/adopt.rs`. Sie liefert den Zweignamen `taik/NAME` oder einen Fehler. Der Befehl `adopt` steht in `src/main.rs`. Dort wird auch der Exit-Status festgelegt: `Ok(true)` ergibt 0 und ein Fehler ergibt 2.

## Teil 1: Befehl `settle`

`taik-seed settle STATE job-NAME` ruft `adopt::adopt` auf. Es gibt keinen zweiten Übernahmeweg und keine kopierte Logik. Der Auftragsordner ist `STATE/jobs/job-NAME`.

- **Übernahme gelingt:**
  - Der Befehl schreibt den Zweignamen (zum Beispiel `taik/demo`) als eine Zeile mit abschließendem Zeilenumbruch in `adopted.txt` im Auftragsordner.
  - Die Datei wird nur einmal angelegt. Existiert sie schon, bleibt sie unverändert.
  - Der Befehl gibt den Zweignamen auf stdout aus, eine Zeile, wie `adopt`.
  - Der Status ist 0.
- **Übernahme scheitert:**
  - Der Befehl schreibt die Fehlermeldung als genau eine nicht leere Zeile nach `adopt-failed.txt` im Auftragsordner. Die Datei darf überschrieben werden.
  - Enthält die Meldung Zeilenumbrüche, werden sie durch Leerzeichen ersetzt.
  - `adopted.txt` wird dabei nicht angelegt.
  - Der Status ist 2.
- **Der Auftragsordner hat keine Datei `session.repo`:** Der Befehl schreibt nichts und endet mit Status 0.
- **Unbekannter Auftrag** (kein Ordner `STATE/jobs/job-NAME`): Der Befehl endet mit Status 2. Er legt weder Ordner noch Dateien an.

Die Logik soll als eigene öffentliche Funktion vorliegen, zum Beispiel `adopt::settle(state, job_name)`. Der Befehl in `main.rs` und der Dienst (Teil 2) rufen beide diese Funktion auf. Trage `settle` auch in `USAGE` in `src/main.rs` ein.

## Teil 2: Dienst

Der Dienst in `src/service.rs` ruft nach einer angenommenen Runde eines Sitzungsauftrags dieselbe Funktion wie `settle` auf. Sitzungsaufträge erkennt man an `src/session.rs`.

- Ein Fehler dabei macht aus der angenommenen Runde keine abgelehnte.
- Ein Fehler löst keine weitere Runde aus.
- Aufträge, die keine Sitzungsaufträge sind, bleiben unberührt.

Dieser Teil braucht eine echte Sitzungsrunde. Das Gate des Auftrags prüft ihn nicht. Zeige ihn durch einen Test in `tests/` nach dem Vorbild von `tests/session.rs` mit einem Stellvertreter für den Harness. Der Test kann in `tests/session.rs` stehen oder in einer eigenen Datei. Das Gate ruft ihn nicht auf, er soll aber sauber kompilieren und bestehen.

## Tests und README

- Lege `tests/settle.rs` als eigenes Testziel an. Es nutzt ein echtes kleines Git-Repository und keine Sandbox, nach dem Vorbild von `tests/adopt.rs` mit `CARGO_BIN_EXE_taik-seed` und `tempfile`.
- Das Testziel deckt Teil 1 ab:
  - Erfolg mit `adopted.txt`.
  - Eine vorhandene `adopted.txt` bleibt unverändert.
  - Fehlschlag mit `adopt-failed.txt` und Status 2.
  - Auftrag ohne `session.repo`: nichts wird geschrieben, Status 0.
  - Unbekannter Auftrag: Status 2, nichts wird angelegt.
- Das Testziel muss mit `cargo test --offline --locked --test settle` bestehen und mindestens einen Test enthalten.
- Trage in `README.md` im Abschnitt „Commands“ die Zeile `taik-seed settle STATE job-NAME` ein, direkt unter der Zeile für `adopt`.

## Was bleibt

- `taik-seed adopt` verhält sich unverändert: Ausgabe, Status und Zweig bleiben gleich. `tests/adopt.rs` bleibt gültig.
- Die Sitzungs- und Übernahme-Logik wird nicht umgebaut. Alle anderen Befehle bleiben wie sie sind.
- Keine neuen Abhängigkeiten in `Cargo.toml`. Der Bau muss mit `cargo build --offline --locked` gehen.
- Das Seed-Dokument `TAIK-FACTORY-SEED-v0.4.md` bleibt unverändert.
