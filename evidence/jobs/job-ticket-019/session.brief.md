Der Orchestrator darf bei einem unklaren Wunsch zurückfragen und plant erst weiter, wenn der Nutzer geantwortet hat.

# Auftrag

Im Repo `taik` plant der Orchestrator (`src/orchestrator.rs`) jeden Wunsch sofort. Ist ein Wunsch unklar, rät er, oder der Gate-Autor lehnt ab. Künftig soll am Wunsch ein Gespräch möglich sein. Dafür wird das vorhandene Modul `src/talk.rs` benutzt. Das Modul selbst bleibt unverändert nutzbar.

## Verhalten

1. **Antwort des Workers.** Statt der fünf Planzeilen darf der Worker genau eine Zeile `Frage: <eine Frage auf Deutsch>` liefern. Der Auftragssatz (`CONTRACT` oder die Regeln in `src/orchestrator.rs`) sagt ihm das und dass er nur fragen soll, wenn eine falsche Annahme den Auftrag wertlos machen würde.
2. **Ablage.** Das Gespräch eines Wunsches NAME liegt in `STATE/requests/NAME.talk/` als Nachrichten des Moduls talk (`message-NNN.agent.md` und `message-NNN.user.md`). Eine Frage wird dort als Nachricht des Agenten abgelegt. Ein Plan (`NAME.plan`) entsteht dann nicht.
3. **Warten.** Solange die letzte Nachricht im Gespräch vom Agenten stammt, plant der Dienst diesen Wunsch nicht erneut und leitet nichts ab (`derive_requests` in `src/service.rs`).
4. **Antworten.** `taik-seed reply STATE ticket-NNN` (Text von der Standardeingabe) tut Folgendes, wenn alle drei Bedingungen gelten:
   - Der Wunsch `STATE/requests/ticket-NNN.run` existiert.
   - Der Vorschlag `STATE/proposals/ticket-NNN` existiert nicht.
   - Die letzte Nachricht im Gespräch `STATE/requests/ticket-NNN.talk/` stammt vom Agenten.
   
   Dann schreibt `reply` die Antwort als Nachricht des Nutzers in dieses Gespräch, weckt den Dienst (wie bisher durch eine Änderung im Ordner `STATE/jobs`) und endet mit Status 0. In jedem anderen Fall bleibt das bisherige Verhalten von `reply` unverändert. Zum Beispiel endet `reply` mit Fehlerstatus, wenn die letzte Nachricht vom Nutzer stammt oder es keinen Wunsch gibt. Existiert ein Vorschlag mit `brief.md`, geht der Kommentar wie bisher in den Vorschlag.
5. **Weiter.** Nach einer Antwort plant der Dienst den Wunsch erneut. Der Auftragssatz enthält dann das bisherige Gespräch (`talk::prompt`). Nach der dritten Frage zu demselben Wunsch darf der Worker nicht mehr fragen. Eine vierte Frage wird wie eine ungültige Antwort behandelt (der vorhandene Rückfall auf den einfachen Plan).
6. **Anzeige.** `taik-seed orchestrator-prompt STATE REQUEST.run` zeigt das Gespräch mit an, wenn es eines gibt. Dabei ist `REQUEST.run` der Pfad `STATE/requests/ticket-NNN.run`, das Gespräch liegt in `STATE/requests/ticket-NNN.talk/`. Die Ausgabe enthält dann den Text von `talk::prompt`, also zum Beispiel die Zeilen `[you] <Frage>` und `[user] <Antwort>`. Gibt es kein Gespräch, bleibt die Ausgabe wie bisher und enthält kein `CONVERSATION ABOUT THIS WORK ORDER`.

## Tests

- Modultests für das Erkennen einer Frage und für die Grenze von drei Fragen.
- Ein eigenes Testziel `tests/request_talk.rs` für Punkt 4. Es kommt ohne Worker und ohne Sandbox aus: Es legt Wunsch und Gespräch selbst an und startet `taik-seed` über `env!("CARGO_BIN_EXE_taik-seed")`. Es muss mit `cargo test --offline --locked --test request_talk` laufen und mindestens einen Test enthalten, der besteht. Wie in `tests/proposal.rs` ist die Datei auf macOS beschränkt (`#![cfg(target_os = "macos")]`).
- Ein Test für die Punkte 2, 3 und 5 mit einem Stellvertreter für den Worker nach dem Vorbild von `tests/proposal.rs`. Er braucht die Sandbox. Er soll geschrieben werden, wird aber vom Gate nicht ausgeführt.

## Muss bleiben

- `cargo clippy --offline --locked --all-targets -- -D warnings` bleibt ohne Befund.
- Bestehende Tests bleiben unverändert.
- Das Verhalten von `reply` für `job-…`-Namen und für Vorschläge bleibt wie es ist.
- Die Seed-Spezifikation `TAIK-FACTORY-SEED-v0.4.md` bleibt unberührt.

Das Gate baut das Projekt, führt `taik-seed reply` und `taik-seed orchestrator-prompt` gegen selbst angelegte Zustandsordner aus, führt `tests/request_talk.rs` aus und lässt clippy laufen.
