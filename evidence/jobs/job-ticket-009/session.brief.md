Der Orchestrator in taik soll Produkte, geschützte Pfade und ausgefallene Agenten kennen und seinen Auftragssatz mit `taik-seed orchestrator-prompt STATE REQUEST.run` sichtbar machen.

# Ziel

`src/orchestrator.rs` plant jeden Wunsch, kennt aber bisher drei Dinge nicht. Diese Dinge kommen hinzu, außerdem ein neuer Befehl, der den Auftragssatz zeigt. Nur macOS ist betroffen, das Modul `orchestrator` gibt es nur dort.

## 1. Befehl `orchestrator-prompt`

`taik-seed orchestrator-prompt STATE REQUEST.run` gibt genau den Text auf der Standardausgabe aus, den der Orchestrator für diesen Wunsch an seinen Worker schicken würde. Der Befehl ruft keinen Worker auf. Er endet mit Status 0 und schreibt keine Datei in STATE. `plan()` baut den Text mit derselben Funktion, es gibt keine zweite Kopie des Textes. Der Befehl steht auch im `USAGE`-Text in `src/main.rs`.

## 2. Produkte

Der Auftragssatz enthält einen Abschnitt, der mit der Zeile `PRODUCTS:` beginnt. Für jede gültige Datei `STATE/products/NAME.product` folgt eine Zeile `- NAME: <Text des Feldes about=>`. Ohne Produkte folgt die Zeile `(none)`. Für die Gültigkeit gelten dieselben Regeln wie in `products()` in `src/proposal.rs` (Name, `seed=`, `gate=`, Ende `end\n`). Nutze diese Funktion, statt sie zu kopieren. Der Auftragssatz sagt dem Worker, dass eine Änderung an einem gelisteten Produkt die Art `dokument` ist.

## 3. Geschützte Pfade

Unter jedem Repository in `REPOSITORIES:` steht eine eingerückte Zeile `  protected: ` (zwei Leerzeichen). Darauf folgen die nicht leeren Zeilen ohne `#` aus `.taik/protected` dieses Repositorys, getrennt durch Komma und Leerzeichen. Fehlt die Datei oder bleibt nichts übrig, lautet die Zeile `  protected: (none)`. Der Auftragssatz sagt, dass eine Sitzung diese Pfade nicht ändern darf.

## 4. Ausgefallene Agenten

Existiert `STATE/crew/NAME.failed` und ist die Datei jünger als sechs Stunden, endet die Zeile dieses Agenten in `AGENTS:` auf ` (failed recently)`. Eine ältere Datei bewirkt nichts.

Wählt der Worker für `Gate:` so einen Agenten, obwohl ein anderer, nicht ausgefallener Agent existiert, der nicht die Arbeit macht, ersetzt der Orchestrator ihn in `parse()` durch den ersten solchen Agenten in Reihenfolge der Crew. An den Grund wird ein Satz angehängt, der beide Namen mit vorangestelltem `@` nennt. Gibt es keinen solchen Agenten, bleibt die Wahl des Workers.

## 5. Ausfallmarke

Der Dienst schreibt `STATE/crew/NAME.failed` (eine Zeile mit der Fehlermeldung), wenn der Aufruf eines benannten Gate-Autors scheitert (siehe `derive_requests` in `src/service.rs` und `propose_planned` in `src/proposal.rs`). Er entfernt die Datei, wenn derselbe Agent später ein Gate liefert. Für den Standard-Worker (`standard`) wird keine solche Datei geschrieben.

## Tests

Das Gate führt genau diese Ziele aus, lege sie so an:

- Integrationstest `tests/orchestrator.rs` (mit `#![cfg(target_os = "macos")]`), der den Befehl `orchestrator-prompt` über `env!("CARGO_BIN_EXE_taik-seed")` ausführt und die Punkte 1 bis 4 (Anzeige) zeigt.
- Unit-Test mit dem Namen `a_failed_gate_agent_is_replaced` in `src/orchestrator.rs`: zeigt die Ersetzung aus Punkt 4.
- Unit-Test mit dem Namen `the_failed_marker_is_written_and_removed` (in einem beliebigen Modul von `src/`): zeigt Punkt 5 einschließlich des Standard-Workers.

# README

In `README.md` steht unter den Befehlen eine Zeile mit genau `taik-seed orchestrator-prompt STATE REQUEST.run` (in einem Codeblock, ggf. mit kurzer Erklärung).

# Muss unverändert bleiben

- Format von `.plan`-Dateien, `Plan::text`/`Plan::read` und das Verhalten von `parse()` in allen bisherigen Fällen.
- Bestehende Tests, besonders `a_plan_is_read_checked_and_kept` und `another_agent_steps_in_for_a_gate_author_that_fails`.
- `CONTRACT`, die bisherigen Zeilenformate `- NAME: about` unter `AGENTS:` und `- NAME: README-Auszug` unter `REPOSITORIES:`.
- `.taik/protected` und `TAIK-FACTORY-SEED-v0.4.md` (eingefroren).
