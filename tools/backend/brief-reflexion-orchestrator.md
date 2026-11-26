Die eigenen Vorhaben der Factory gehen über den Orchestrator.

Von der Factory hervorgebracht. Wunsch `requests/ticket-026.run`, Gate von `pruefer` (Sonnet 5.5, lesend), Arbeit von Opus 5.5, `job-ticket-026` in Runde 1 angenommen. Der Kandidat dieser Runde ist der Merge jenes Commits; von `claude-fable` stammt nur `cargo fmt`.

Verhalten: Nach einem fertigen Auftrag legt die Reflexion aus dem ältesten offenen Vorhaben einen Wunsch `ticket-NNN` an, mit `origin=reflection` und `intent_from`. Der Orchestrator plant ihn wie jeden Wunsch. Ohne offenes Vorhaben geschieht nichts. Die Grenze von drei in 24 Stunden bleibt. Die Reflexion läuft jetzt auch nach Sitzungsaufträgen. Die Freigabe bleibt beim Nutzer.

Prüfung: `tools/backend/gate.sh`.
