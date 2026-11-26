Der Autor des Gates einer Sitzung liest das Repository, bevor er das Gate schreibt.

Beobachtung am 2026-11-26 in `job-ticket-008`: Drei Runden scheiterten an einem Gate, das gegen eine erfundene Ablage prüfte (`crew/NAME` ohne `.agent`, `state/repositories/NAME` statt `repos/NAME.repo`) und am Ende die ganze Testsuite rief, die in der Sandbox des Gates nicht laufen kann. Der Autor (Savant) hatte keine Werkzeuge und sah vom Repository nur die Dateiliste und den Anfang der README. Der Auftrag wurde mit `taik-seed close` geschlossen.

Verlangt:

- Ist der Autor eines Sitzungs-Gates der Standard-Worker oder ein Agent auf dem Claude-Harness, arbeitet er in einer Wegwerf-Kopie des Repositorys mit Read, Glob und Grep, ohne Schreibwerkzeuge, in derselben Sandbox wie eine Sitzung.
- Der Auftrag an den Autor sagt, dass das Gate keine weitere Sandbox starten kann, kein Netz hat, nur die Testziele des Auftrags rufen soll und den Grund eines Fehlschlags samt Ausgabe drucken muss.
- Der Orchestrator bevorzugt für das Gate einer Sitzung einen Agenten auf dem Claude-Harness, der nicht die Arbeit macht.
- Eine letzte Ergebniszeile ohne Zeilenende wird gelesen.

Prüfung: `tools/backend/gate.sh`. Der bestehende Test `the_orchestrator_plans_a_request_and_approval_follows_the_plan` läuft jetzt über den lesenden Autor.
