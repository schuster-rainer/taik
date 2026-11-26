Einen Vorschlag ablehnen und einen Wunsch zurückziehen: `taik-seed decline STATE NAME`.

Von der Factory hervorgebracht. Wunsch `requests/ticket-022.run`, Gate von `pruefer` (Sonnet 5.5, lesend, mit Clippy), Arbeit von Opus 5.5, `job-ticket-022` in Runde 1 angenommen. Der Kandidat dieser Runde ist der Merge jenes Commits; von `claude-fable` stammt nur `cargo fmt`.

Verhalten: Die Begründung liegt einmalig als `declined.md` im Vorschlag oder als `NAME.withdrawn` beim Wunsch. Ein abgelehnter Vorschlag lässt sich nicht mehr freigeben, ein zurückgezogener Wunsch wird nicht mehr geplant und nimmt keine Antwort mehr an.

Das Gate des Auftrags prüfte Befehl, Sperren und Clippy. Der Teil mit dem Dienst wird in dieser Runde von der ganzen Testsuite geprüft (`tests/decline_service.rs`).

Prüfung: `tools/backend/gate.sh`.
