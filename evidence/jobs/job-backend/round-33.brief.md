Eine angenommene Sitzung legt ihren Zweig selbst an: `taik-seed settle STATE job-NAME`, vom Dienst nach der Annahme aufgerufen.

Von der Factory hervorgebracht. Wunsch `requests/ticket-012.run`. Die erste Fassung (`ticket-011`) lehnte der lesende Gate-Autor `pruefer` ab: Das Verhalten lasse sich nur über eine echte Sitzungsrunde prüfen, und das Gate könne keine Sandbox starten. Der Wunsch wurde in einen ohne Sandbox prüfbaren Befehl und den Aufruf im Dienst geteilt. Plan: Sitzung auf `taik`, Gate von `pruefer` (Sonnet 5.5), Arbeit von Opus 5.5. `job-ticket-012` wurde in Runde 1 angenommen.

Der Kandidat dieser Runde ist der Merge des Zweigs `taik/ticket-012`. Von `claude-fable` stammt nur `cargo fmt`. Diese Runde prüft mit der ganzen Testsuite auch den Teil, den das Gate des Auftrags nicht prüfen konnte: den Aufruf im Dienst (`tests/session.rs`).

Prüfung: `tools/backend/gate.sh`.
