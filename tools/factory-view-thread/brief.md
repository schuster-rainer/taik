Die Unterhaltung eines Auftrags in seinem Ticket zeigen und führen.

Beobachtung des Nutzers am 2026-11-25: Ein Ticket hat eine Beschreibung, das ist der Auftrag. Darunter stehen die Antworten von Agent und Nutzer wie ein Chat. Muss der Nutzer antworten, bekommt er ein Signal und schreibt direkt ins Ticket. Der Kern hält die Unterhaltung seit Runde 17 von `job-backend` im Auftragsordner. Die Ansicht schrieb Antworten bisher nach `STATE/inbox`, das nichts las.

Verlangt:

- Ein geöffnetes Ticket zeigt die Beschreibung (aufklappbar), dann in der Reihenfolge des Geschehens die Nachrichten und die Runden mit Entscheidung, Befund des Gates und Kosten, dann die Wahl des Agenten und das Antwortfeld. Die Reihenfolge bestimmt die reine Funktion `ticketThread(job)` im Block `THREAD`.
- Eine Antwort an einen Auftrag schreibt die installierte Factory über `taik-seed reply` in den Auftragsordner. Nur Kommentare an Vorschläge, die noch keinen Auftragsordner haben, gehen weiter nach `STATE/inbox`.
- Ein Auftrag, dessen Worker gefragt hat, steht unter „Wartet" mit dem Kennzeichen „Frage". Ein Auftrag, auf dessen Antwort noch keine Runde gefolgt ist, steht unter „Läuft".
- Beginnt ein Ticket auf den Nutzer zu warten, meldet die Ansicht das einmal der App. Die App spielt einen Ton und meldet sich im Dock. Beim ersten Laden gibt es kein Signal.
- Ein geöffnetes Ticket darf höher werden als die geschlossene Liste.

Der ausführliche Auftragsverlauf mit Arbeitsschritten und Prüfprotokoll bleibt in dieser Runde als eigenes Feld darunter. Ihn ganz ins Ticket zu ziehen ändert die Navigation und den Prüfvertrag und ist der nächste Auftrag.
