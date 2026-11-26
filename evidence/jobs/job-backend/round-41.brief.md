Der Orchestrator kann zu einem Wunsch zurückfragen.

Von der Factory hervorgebracht. Wunsch `requests/ticket-019.run`, Gate von `pruefer` (Sonnet 5.5, lesend, mit Clippy), Arbeit von Opus 5.5, `job-ticket-019` in Runde 1 angenommen. Der Kandidat dieser Runde ist der Merge jenes Commits; von `claude-fable` stammt nur `cargo fmt`.

Verhalten: Der Worker des Orchestrators darf statt eines Plans eine Zeile `Frage: …` liefern. Sie liegt als Nachricht im Gespräch `requests/NAME.talk/`, der Wunsch wartet. `taik-seed reply STATE ticket-NNN` schreibt die Antwort dorthin, danach plant der Dienst mit dem Gespräch im Auftragssatz. Höchstens drei Fragen je Wunsch.

Das Gate des Auftrags prüfte den Befehl `reply`, den Auftragssatz und Clippy. Der Teil mit dem Dienst braucht die Sandbox und wird in dieser Runde von der ganzen Testsuite geprüft (`tests/request_question.rs`).

Prüfung: `tools/backend/gate.sh`.
