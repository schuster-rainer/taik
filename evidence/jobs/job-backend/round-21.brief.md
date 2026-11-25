Ein Auftrag als Agentensitzung: ein Agent mit Datei-Werkzeugen arbeitet in einer eigenen Kopie, behält seine Sitzung, und das Gate urteilt über seinen Commit.

Beobachtung des Nutzers am 2026-11-25: „ich will eine normale agentensession haben können", „runden, ja, resume ja", Dauer eines Arbeitsgangs „Keine Obergrenze". Dazu: „mir fehlt halt einfach was der agent gemacht hat … welche datei und was geändert wurde".

Messungen an der installierten Claude-CLI vor dem Bau: Fortsetzen einer Sitzung geht, der zweite Aufruf las 34.907 Tokens aus dem Cache. Ein lokaler Klon je Ticket trägt Git. Mit geschlossenem Home-Verzeichnis kann der Agent die Kopie lesen und schreiben, den Haupt-Checkout und die Auftragsordner nicht. Die Anmeldung der CLI braucht den Schlüsselbund; eine Shell in derselben Sandbox konnte in der Probe einen fremden Schlüsselbund-Eintrag auslesen. Deshalb hat der Agent keine Shell.

Verlangt:

- `taik-seed queue-session STATE NAME BRIEF.md GATE.sh [REPOSITORY]` reiht einen Sitzungsauftrag ein. Der Dienst erkennt ihn an `session.brief.md`.
- Die Kopie liegt in `STATE/work/NAME`, ein lokaler Klon ohne Rückweg zum Ursprung, oder ein leeres Repository.
- Ein Aufruf hat nur Read, Edit, Write, Glob und Grep. Er hat keine Zeitgrenze und eine Lease von 600 s ohne Ausgabe, weil ein Werkzeuglauf länger still sein kann als das Schreiben eines Dokuments.
- Runde 1 beginnt eine Sitzung, jede weitere setzt sie fort und bekommt nur das Neue: den Befund des Gates und die Antworten des Nutzers.
- Was der Agent sagt und welches Werkzeug er auf welche Datei anwendet, steht in `round-NN.dialog.jsonl` und in den Arbeitsschritten. Der Dialog bleibt lokal und ist von Git ausgenommen.
- Nach dem Aufruf legt die Factory einen Commit an und hält `round-NN.diff` fest. Das fixierte Gate prüft einen sauberen Checkout genau dieses Commits. Das Protokoll nennt Commit, Dateien und die Zusammenfassung des Agenten.
- Ändert der Commit einen Pfad aus `.taik/protected` des Ausgangsstands, wird er abgelehnt, ohne dass das Gate läuft.
- Eine Rückfrage des Agenten (`=== FRAGE ===`) wird zur Nachricht im Ticket, der Auftrag wartet.
- Eine Ablehnung führt zur nächsten Runde, bis zu sechs Runden und einer je Antwort. Diese Zahl ist eine Annahme des Autors.
- Die Kosten zum Listenpreis stehen als `session_listed_usd` im Protokoll und zählen nicht gegen das Tagesbudget, weil die Sitzung über das Abo läuft.

Prüfung: `tools/backend/gate.sh`, dazu der neue Test in `tests/session.rs` mit einem Stellvertreter des Harness.
