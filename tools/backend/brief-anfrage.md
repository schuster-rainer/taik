Einen Wunsch des Nutzers in eigenen Worten als Ticket annehmen und Kommentare an Vorschlägen in die Unterhaltung führen.

Beobachtung am 2026-11-25: In der Ansicht lässt sich kein Ticket anlegen. Ein Auftrag entsteht nur über die Kommandozeile, ein GitHub-Issue oder eine Reflexion. Kommentare an einem Vorschlag schreibt die Ansicht nach `STATE/inbox`, das niemand liest.

Verlangt:

- `taik-seed request STATE < TEXT` hält den Wunsch als Protokoll `STATE/requests/ticket-NNN.run` fest und weckt den Dienst. Es startet nichts.
- Der Dienst leitet aus jedem offenen Wunsch einmal einen Vorschlag ab, mit seinem eigenen Worker, neben den laufenden Aufträgen. Lehnt der Worker ab, steht die Begründung in `requests/reflection-NAME.run`. Ein Fehler steht in `requests/NAME.failed`. Die Freigabe bleibt beim Nutzer.
- `taik-seed reply STATE NAME` nimmt auch den Namen eines wartenden Vorschlags an und schreibt die Nachricht in dessen Ordner.
- Bei der Freigabe ziehen die Nachrichten des Vorschlags in den Auftrag, bevor der Dienst ihn sieht. Der Worker der ersten Runde liest sie.

Grenze: Ein Wunsch, der eintrifft, während Aufträge laufen, wird erst im nächsten Durchgang des Dienstes abgeleitet.

Prüfung: `tools/backend/gate.sh`, dazu der neue Test `a_request_of_the_user_becomes_a_proposal_and_keeps_what_was_said` in `tests/proposal.rs`.
