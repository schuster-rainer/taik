Das Ticket erzählt von oben nach unten: Stand, Änderung je Runde, geänderte Zeilen.

Beobachtung des Nutzers am 2026-11-25: „mir fehlt halt einfach was der agent gemacht hat, was die offenen fragen oder was problem ist. und der byte .. bla bla writen. macht kein sinn. da würde ich gerne auch sehen können welche datei und was geändert wurde. also von highlevel zum detail. top down".

Verlangt:

- Oben im Ticket steht der Stand (`ticketState`): was zuletzt geschah, das Problem in den Worten des Gates, die offene Frage des Agenten, der nächste Schritt.
- Jede Runde nennt in einer Zeile, welche Datei sich wie stark geändert hat (`changeLine`). Bei einer Ablehnung gilt das für den abgelehnten Kandidaten.
- Hinter der Zeile liegen aufklappbar die geänderten Zeilen mit zwei Zeilen Umgebung.
- Die App berechnet die Änderung aus den vorhandenen Dateien des Auftragsordners gegen den zuvor angenommenen Stand. Neue Daten entstehen nicht.

Nicht Teil dieser Runde: Zwischentext und Überlegungen des Agenten. Die hält der Kern bisher nicht fest.

Prüfung: `tools/factory-view-stand/verify.mjs`. Ein Namenskonflikt im Gate (`round` doppelt deklariert) wurde behoben, nachdem der Kandidat existierte und bevor das Gate fixiert wurde. Die Behauptungen blieben unverändert.
