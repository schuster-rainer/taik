Der Befehl `taik-seed assign STATE NAME AGENT` lässt den Nutzer vor der Freigabe festlegen, welcher Agent die Arbeit eines Vorschlags macht.

# Ausgangslage

Der Orchestrator legt im Plan eines Vorschlags (`STATE/proposals/NAME/plan.txt`, Zeile `work=`) fest, welcher Agent arbeitet. Der Nutzer kann das vor `taik-seed approve` nicht ändern. Relevanter Code: `src/proposal.rs` (`approve`, `decline`), `src/main.rs` (Befehlsverteilung, dort die Zweige `approve` und `decline`), `src/crew.rs` (`load`, `Harness`), `src/orchestrator.rs` (`Plan::read`, `STANDARD`).

# Gewünschtes Verhalten

1. `taik-seed assign STATE NAME AGENT`: AGENT ist `standard` oder der Name eines Agenten der Crew (`STATE/crew/AGENT.agent`, lesbar mit `crew::load`). Der Befehl schreibt den Namen als eine Zeile (mit Zeilenende) in die Datei `worker` im Ordner `STATE/proposals/NAME` und endet mit Status 0. Eine vorhandene Datei `worker` wird überschrieben, solange der Vorschlag wartet. Die Datei soll nie halb geschrieben sichtbar sein (wie bei den anderen Befehlen, z. B. temporäre Datei und Umbenennen).
2. `taik-seed approve`: Gibt es die Datei `worker`, gilt ihr Inhalt statt `work=` aus dem Plan. Der Auftrag bekommt diesen Agenten (Datei `agent` im Auftragsordner), bei `standard` keine Datei `agent`. `plan.txt` bleibt unverändert und wird wie bisher in den Auftragsordner kopiert. Zusätzlich liegt im Auftragsordner die Datei `worker` mit demselben Inhalt wie im Vorschlagsordner. Sie muss schon da sein, wenn der Auftrag für den Dienst sichtbar wird (also wie `agent` und `plan.txt` vor dem Veröffentlichen schreiben, für Dokument- und Sitzungsaufträge). Ohne Datei `worker` bleibt alles wie bisher.
3. Mit Status 2 und ohne irgendetwas zu schreiben (weder Datei noch Ordner) werden abgewiesen:
   - ein unbekannter oder ungültiger Vorschlagsname (gleiche Regel wie bei `approve`/`decline`, `worker_name`),
   - ein schon freigegebener oder abgelehnter Vorschlag (`approved.txt` oder `declined.md` vorhanden),
   - ein unbekannter oder ungültiger Agent,
   - bei einem Vorschlag, dessen Plan eine Sitzung ist (`kind=sitzung`): ein Agent, der nicht auf dem Claude-Harness läuft (`standard` ist erlaubt).
4. Der Agent, der laut Plan das Gate geschrieben hat (`gate=` in `plan.txt`), darf nicht als Arbeiter gesetzt werden: Status 2, nichts wird geschrieben.

# Tests und Doku

- Neues Testziel `tests/assign.rs` (wie `tests/decline.rs` aufgebaut, `#![cfg(target_os = "macos")]`, Binary über `CARGO_BIN_EXE_taik-seed`). Es kommt ohne Worker und ohne Sandbox aus: Vorschlag, Plan und Crew legt der Test selbst im State-Ordner an; für die Freigabe einer Sitzung dient ein echtes kleines Git-Repository. Abgedeckt sein sollen: Schreiben und Überschreiben, Freigabe mit Agent, mit `standard` und ohne Datei, alle Abweisungen aus Punkt 3 und 4 (Zustand danach unverändert).
- In `README.md` unter „Four commands“ (die Befehlsliste) eine Zeile, die `taik-seed assign STATE NAME AGENT` nennt und kurz erklärt.
- `cargo clippy --offline --locked --all-targets -- -D warnings` bleibt ohne Befund.

# Was bleibt

Bestehende Tests und Testdateien bleiben unverändert. Das Format von `plan.txt`, die Befehle `approve`, `decline` und alle anderen Befehle bleiben im bisherigen Verhalten, wenn keine Datei `worker` existiert. Keine neuen Abhängigkeiten.
