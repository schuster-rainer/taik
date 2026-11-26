Die Reflexion der Factory legt ihr nächstes Vorhaben als Wunsch in STATE/requests an, statt propose direkt aufzurufen.

## Ziel

Heute ruft `proposal::reflect` (src/proposal.rs) nach einem fertigen Auftrag direkt `propose` auf, ohne Plan des Orchestrators. Die Wünsche des Nutzers gehen dagegen als `STATE/requests/ticket-NNN.run` durch den Orchestrator. Die Reflexion nimmt künftig denselben Weg wie diese Wünsche.

## Verhalten nach der Änderung

1. `reflect` ruft `propose` nicht mehr auf und ruft keinen Worker auf. Es nimmt das älteste offene Vorhaben der Factory. Das ist der Eintrag aus `open_intents` mit dem ältesten Änderungsdatum. Danach legt es daraus einen Wunsch an, mit derselben Funktion wie `taik-seed request` (`service::request`). Der Wunsch bekommt wie jeder Wunsch den nächsten freien Namen `ticket-NNN`.
   - Erste Zeile des Wunsches, also sein Titel: `Vorhaben der Factory: ` gefolgt von den ersten 80 Zeichen des Vorhabens. Gezählt werden Zeichen, nicht Bytes.
   - Der übrige Text des Wunsches ist das Vorhaben vollständig. Im Protokoll steht es wie bisher als `observation=`.
2. Das Protokoll dieses Wunsches trägt zusätzlich die Zeilen `origin=reflection` und `intent_from=<Pfad der Auswertung>`. Beide stehen vor der Schlusszeile `end`. Der Pfad ist derselbe, den `open_intents` für diese Auswertung liefert. Ein Wunsch des Nutzers über `taik-seed request` bekommt diese Zeilen nicht und bleibt Byte für Byte wie bisher.
3. `open_intents` betrachtet ein Vorhaben auch dann als aufgegriffen, wenn in `STATE/requests` ein Wunsch (`*.run`) mit `intent_from=<derselbe Pfad>` existiert. Die bisherige Erkennung über `reflection-*.run` in den Auftragsordnern bleibt.
4. Gibt es kein offenes Vorhaben, legt `reflect` nichts an und ruft keinen Worker auf. Das ist kein Fehler, der Exit-Status ist 0.
5. Die Grenze von drei in 24 Stunden bleibt. Sie zählt die jungen `reflection-*.run` (jünger als 24 Stunden) und die jungen Wünsche mit `origin=reflection` in `STATE/requests` zusammen. Bei drei oder mehr legt `reflect` nichts an.
6. `reflect` läuft auch nach einem fertigen Sitzungsauftrag, nicht nur nach einem Dokument-Auftrag. Die Bedingung bleibt: `TAIK_AUTO_PROPOSE` ist gesetzt. Der Aufruf gehört in `run_source_job` in src/service.rs, in den Zweig für Sitzungsaufträge. Ein Test dafür braucht die Sandbox. Er darf geschrieben werden, wird aber vom Gate nicht ausgeführt.

Aus dem Wunsch entsteht wie bisher ein Vorschlag, der auf die Freigabe des Nutzers wartet. Das übernimmt der Dienst über `derive_requests`. Das ändert sich nicht.

## Befehl

Neuer Befehl: `taik-seed reflect STATE job-NAME`. `job-NAME` ist der Ordnername unter `STATE/jobs`. Der Befehl tut dasselbe wie der Dienst nach einem fertigen Auftrag, aber ohne die Bedingung `TAIK_AUTO_PROPOSE`. Er gibt bei Erfolg Exit-Status 0 zurück, auch wenn nichts angelegt wurde. Er ist nur auf macOS vorhanden, wie die anderen Befehle dieser Art.

## Tests

- Neues Testziel `tests/reflect.rs` mit mindestens fünf Tests, ohne Worker und ohne Sandbox. Sie decken die Punkte 1 bis 5 ab und rufen `taik-seed reflect` auf. Die Datei beginnt wie die anderen Tests mit `#![cfg(target_os = "macos")]`.
- `cargo test --offline --locked --test reflect` muss grün sein.
- `cargo clippy --offline --locked --all-targets -- -D warnings` bleibt ohne Befund.
- Bestehende Tests, die das alte Verhalten von `reflect` prüfen, werden an das neue Verhalten angepasst und in der Zusammenfassung genannt.

## Was bleibt

- `taik-seed propose` und `proposal::propose` bleiben unverändert.
- `taik-seed request` und das Format der Wunsch-Protokolle des Nutzers bleiben unverändert.
- Der Dienst `derive_requests` und das Wunsch-Protokoll `reflection-ticket-NNN.run` in `STATE/requests` bleiben.
- Das Gate liest keine anderen Dateien und ändert keine Dateien außerhalb des Repositorys.
