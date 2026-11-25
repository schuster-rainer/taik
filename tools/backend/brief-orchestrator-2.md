Fällt der Autor des Gates aus, schreibt ein anderer Agent das Gate; ein Plan-Beleg ist kein Wunsch.

Beobachtung aus dem ersten echten Durchlauf am 2026-11-26 (`requests/ticket-002`): Der Orchestrator wählte `grok-schnell` für das Gate, der Aufruf endete nach 7,5 s mit Status 1, und der Wunsch blieb als Fehler liegen. Außerdem las der Dienst `ticket-002.plan.run` als eigenen Wunsch und meldete „invalid proposal name".

Verlangt:

- Scheitert der Aufruf des Gate-Autors, versucht der Dienst der Reihe nach die übrigen Agenten, die weder ausgefallen sind noch die Arbeit machen. Der Grund im Plan des Vorschlags nennt, wer ausfiel und wer einsprang.
- Dateien `ticket-NNN.*.run` gelten nicht als Wünsche.

Prüfung: `tools/backend/gate.sh`, dazu der neue Modultest `another_agent_steps_in_for_a_gate_author_that_fails`.
