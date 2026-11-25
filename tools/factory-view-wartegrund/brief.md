Jedes wartende Ticket sagt, warum es wartet.

Beobachtung des Nutzers am 2026-11-25: „ich kann leider nicht erkennen, warum gewartet wird. wo steht in der factory die frage". Drei Tickets trugen das Kennzeichen „Frage", obwohl kein Agent gefragt hatte: Ihre letzte Runde war abgelehnt.

Verlangt:

- Eine abgelehnte Runde heißt auf der Tafel „Abgelehnt", ein Vorschlag „Freigabe". „Frage" steht nur dort, wo ein Agent gefragt hat.
- Jeder Eintrag unter „Wartet" trägt seinen Grund (`why`): die Frage des Agenten im Wortlaut, den nächsten Schritt aus der Analyse, sonst den Befund der abgelehnten Runde.
- Das Ticket beginnt mit „Wartet auf dich" und sagt, was die Factory von dir braucht.

Prüfung: `tools/factory-view-wartegrund/verify.mjs`. Gegenüber dem Gate von `job-ansicht-sitzung` erwarten zwei Vergleiche die neuen Kennzeichen, weil der Nutzer die alten als irreführend benannt hat.
