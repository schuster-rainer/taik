Ein Ticket in der Ansicht anlegen, und ein Vorschlag zeigt Beschreibung und Unterhaltung.

Beobachtung am 2026-11-25: Die Ansicht hat kein Feld für einen neuen Auftrag. Ein Vorschlags-Ticket zeigt nur seinen Titel, Kommentare daran liest niemand. Beim Öffnen passt das gezeigte Ticket nicht zur gezeigten Gruppe. Der Nutzer zeigte eine Liste von Arbeitsschritten, die fast nur aus „N Bytes Quellcode empfangen" besteht, und fragte, ob das eleganter geht.

Verlangt:

- Ein Feld „Neues Ticket". Der Text geht als `newTicket` an die App, die ihn mit `taik-seed request` festhält (Kern seit Runde 19 von `job-backend`).
- Ein Wunsch ohne Vorschlag steht unter „Läuft" als „Wird abgeleitet". Abgelehnt oder fehlgeschlagen steht er einen Tag unter „Entschieden", mit Begründung. Die Einordnung leistet `boardTickets`.
- Ein Vorschlags-Ticket zeigt seine Beschreibung und was dazu gesagt wurde. Freigeben und kommentieren kann man nur einen fertigen Vorschlag.
- Vor der ersten Wahl zeigt das Ticket den laufenden Auftrag, sonst den ersten Eintrag der gezeigten Gruppe (`firstTicket`).
- Aufeinanderfolgende Byte-Meldungen eines Workers erscheinen als ein Schritt mit ihrer Anzahl (`foldSteps` im Block `STEPS`).

Nicht Teil dieser Runde: Arbeitsschritte mit Dateinamen aus Werkzeug-Hooks. Die gibt es erst, wenn ein Agent mit Werkzeugen arbeitet.

Prüfung: `tools/factory-view-anlegen/verify.mjs`, vor dem Kandidaten geschrieben.
