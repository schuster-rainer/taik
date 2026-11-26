Der Orchestrator kennt Produkte, geschützte Pfade und ausgefallene Agenten, und sein Auftragssatz ist einsehbar.

Von der Factory hervorgebracht. Wunsch `requests/ticket-009.run` (derselbe wie `ticket-008`). Plan des Orchestrators: Sitzung auf `taik`, Gate von `pruefer` (Sonnet 5.5, liest das Repository), Arbeit von Opus 5.5. `job-ticket-009` wurde in Runde 1 angenommen. Zum Vergleich: `job-ticket-008` mit einem blind geschriebenen Gate scheiterte dreimal und wurde geschlossen.

Der Kandidat dieser Runde ist der Merge des Zweigs `taik/ticket-009`, den `taik-seed adopt` angelegt hat. Von `claude-fable` stammt nur `cargo fmt`.

Verhalten, in `job-ticket-009` geprüft: `taik-seed orchestrator-prompt STATE REQUEST.run` druckt den Auftragssatz ohne Worker-Aufruf. Der Auftragssatz listet Produkte und je Repository die geschützten Pfade und markiert Agenten mit einer höchstens sechs Stunden alten Datei `crew/NAME.failed`. Ein so markierter Gate-Autor wird ersetzt. Der Dienst schreibt und entfernt diese Datei.

Prüfung: `tools/backend/gate.sh`, mit dem neuen `tests/orchestrator.rs` des Agenten.
