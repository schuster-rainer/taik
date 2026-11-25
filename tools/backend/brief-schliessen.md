Einen Auftrag mit Begründung schließen: `taik-seed close STATE job-NAME`.

Dies ist die erste Änderung am Kern, die die Factory selbst hervorgebracht hat. Wunsch `requests/ticket-003.run`, Plan des Orchestrators: Sitzung auf `taik`, Gate von Savant (für den ausgefallenen `grok-schnell`), Arbeit vom Standard-Worker Opus 5.5. Der Auftrag `job-ticket-003` wurde in Runde 4 von Savants Gate angenommen (Commit `cc12e02` in der Kopie). Die Runden 1 bis 3 scheiterten an der Sandbox und am Suchpfad des Dienstes, nicht an der Arbeit; der Agent hat zweimal im Ticket nachgefragt.

Der Kandidat dieser Runde ist der Merge jenes Commits in den Checkout. Von `claude-fable` stammt nur `cargo fmt` auf den übernommenen Dateien.

Verlangt und in `job-ticket-003` geprüft: Die Begründung kommt von der Standardeingabe (1 bis 2000 Bytes) und liegt danach einmalig als `closed.md` im Auftragsordner. Ein unbekannter Auftrag, eine leere oder zu lange Begründung und ein zweites Schließen enden mit Status 2. Ein geschlossener Auftrag bekommt keine Runde mehr, auch nicht nach einer Antwort.

Prüfung: `tools/backend/gate.sh`, mit dem neuen `tests/close.rs` des Agenten.
