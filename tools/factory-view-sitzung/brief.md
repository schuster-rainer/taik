Eine Agentensitzung im Ticket zeigen: Zusammenfassung, geänderte Dateien, Dialog.

Beobachtung des Nutzers am 2026-11-25: „können wir dort auch den agentendialog anzeigen? können wir ja auch im ticket mit ablegen", „mir fehlt halt einfach was der agent gemacht hat … welche datei und was geändert wurde … von highlevel zum detail". Der Kern führt Sitzungsaufträge seit Runde 21 von `job-backend` und legt je Runde Dialog, Patch und Zusammenfassung ab.

Verlangt:

- Eine Runde zeigt die Zusammenfassung des Agenten aus ihrem Protokoll.
- Die Änderungszeile nennt die Dateien des Commits, dahinter liegt der Patch je Datei.
- Aufklappbar steht der Dialog der Runde: was der Agent sagte, was der Nutzer sagte, welches Werkzeug auf welche Datei ging (`dialogLine`).
- Die App liest dafür `session.brief.md`, `round-NN.diff` und `round-NN.dialog.jsonl`.

Prüfung: `tools/factory-view-sitzung/verify.mjs`, es übernimmt das Gate von `job-ansicht-analyse`.
