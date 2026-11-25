Das Orchestrator-Widget: Der Nutzer spricht mit dem Orchestrator, und der zeigt je Wunsch seinen Plan.

Beobachtung des Nutzers am 2026-11-26: „ich möchte das orchestrator widget haben. bitte einbauen". Der Kern plant Wünsche seit Runde 22 von `job-backend`.

Verlangt:

- Die Karte „Neues Ticket" wird zur Karte „Orchestrator" (`taik-orchestrator`): ein Feld für die Nachricht und darunter seine letzten fünf Pläne, neueste zuerst (`orchestratorFeed`).
- Ein Plan steht in einer Zeile (`planLine`): Dokument oder Sitzung auf welchem Repository, wer das Gate schreibt, wer arbeitet. Darunter der Grund. Ein Wunsch ohne Plan zeigt „Der Orchestrator plant."
- Ein Klick auf einen Plan öffnet das Ticket. Das Vorschlags-Ticket zeigt Plan und Grund.
- Nichts wird abgeschnitten.

Prüfung: `tools/factory-view-orchestrator/verify.mjs`, es übernimmt das Gate von `job-ansicht-wartegrund`.
