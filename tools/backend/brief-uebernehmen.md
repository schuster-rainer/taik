Das angenommene Ergebnis einer Sitzung als Zweig ins Ursprungs-Repository holen: `taik-seed adopt STATE job-NAME`.

Von der Factory hervorgebracht. Wunsch `requests/ticket-007.run`, Plan des Orchestrators: Sitzung auf `taik`, Gate von Savant (für den ausgefallenen `grok-schnell`), Arbeit von Opus 5.5. `job-ticket-007` wurde in Runde 1 von Savants Gate angenommen. Der Kandidat dieser Runde ist der Merge jenes Commits; von `claude-fable` stammt nur `cargo fmt`.

Verhalten, in `job-ticket-007` geprüft: Der Befehl legt im Ursprungs-Repository den Zweig `taik/NAME` auf den angenommenen Commit, wechselt keinen Zweig, ändert keine Arbeitsdateien und führt nichts zusammen. Abgewiesen mit Status 2: unbekannter Auftrag, Auftrag ohne `session.repo`, letzte Runde nicht angenommen, Zweig zeigt schon auf einen anderen Commit.

Prüfung: `tools/backend/gate.sh`, mit dem neuen `tests/adopt.rs` des Agenten.
