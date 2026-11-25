Ein Orchestrator legt für jeden Wunsch fest, wie er ausgeführt wird, wer das Gate schreibt und wer arbeitet.

Beobachtung des Nutzers am 2026-11-26: „wir haben einen orchestrator. der soll die tickets verteilen. haben wir das nicht?" und „ich möchte das orchestrator widget haben. bitte einbauen". Bisher verteilt der Nutzer von Hand, Sitzungsaufträge entstehen nur über die Kommandozeile, und Gate und Arbeit kommen vom selben Worker.

Verlangt:

- Für jeden Wunsch (`requests/ticket-NNN.run`) fragt der Dienst zuerst den Standard-Worker als Orchestrator. Der kennt die Crew mit ihren Beschreibungen und die bekannten Repositories (`STATE/repos/NAME.repo`).
- Der Plan hat fünf Zeilen: Art (Dokument oder Sitzung), Repo, Gate, Arbeit, Grund. Er liegt als `requests/NAME.plan` beim Wunsch und als `plan.txt` beim Vorschlag.
- Gate und Arbeit gehen an zwei verschiedene Agenten, sobald zwei vorhanden sind. Eine Sitzung läuft auf dem Claude-Harness; eine andere Wahl fällt auf den Standard zurück. Beides steht dann im Grund.
- Das Gate schreibt der Agent, den der Plan nennt. Für eine Sitzung ist es ein Shell-Skript für das Repository, dessen Dateiliste und README der Autor sieht.
- Die Freigabe folgt dem Plan: ein Sitzungsauftrag auf dem Repository oder ein Dokument-Auftrag, jeweils mit dem Agenten für die Arbeit.
- Gibt der Orchestrator keinen gültigen Plan, läuft der Wunsch wie bisher als Dokument beim Standard, und der Grund sagt das.
- Der Orchestrator startet nichts. Die Freigabe bleibt beim Nutzer.

Prüfung: `tools/backend/gate.sh`, dazu der neue Test `the_orchestrator_plans_a_request_and_approval_follows_the_plan` und der Modultest des Plans. Der bestehende Test für Wünsche blieb unverändert.
