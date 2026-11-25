Das Ticket-Board der Ansicht ordnen: eine Gruppe zur Zeit, Karten über die volle Breite, Titel als reiner Text.

Beobachtung des Nutzers am 2026-11-25 an einer Aufnahme: Das Board „sieht extrem krüppelig aus". Gemessen am installierten Stand: Die Karten waren so breit wie ihr Text (480 und 410 Pixel in einer Liste von 620), Titel begannen mit dem `#` der Auftragsüberschrift, jede Karte nannte ihre Kennung zweimal, drei gestapelte Gruppen schoben den Auftragsverlauf aus dem Fenster, und wartende Vorschläge standen zusätzlich in einem eigenen Feld.

Herkunft: Das Board kam mit Commit 269c87f direkt in `tools/factory-view.html`, nachdem `job-board-ansicht` nach vier abgelehnten Runden sein Budget ausgeschöpft hatte. Diese Runde stellt es unter ein fixiertes Gate.

Verlangt:

- Drei Reiter `Wartet`, `Läuft`, `Entschieden` mit ihrer Anzahl. Es ist immer genau eine Gruppe zu sehen. Ohne Wahl des Nutzers ist es die erste Gruppe, die etwas enthält. Die Entscheidung trifft die reine Funktion `boardTab(columns, chosen)` im Block `BOARD`.
- Eine Karte ist so breit wie ihre Liste. Erste Zeile: Titel links, Kennzeichen rechts. Zweite Zeile: Auftrag, Stand und Kosten mit Dezimalkomma.
- `ticketTitle(text)` entfernt führende `#` einer Markdown-Überschrift. Sie gilt für Karten, das Auftragsziel und die Rundentitel.
- Eine lange Gruppe rollt innerhalb des Boards. Das Board nimmt am Schreibtisch höchstens die halbe Fensterhöhe.
- Das Feld „Vorschläge der Factory" und der Knopf „Zum ersten wartenden Ticket" entfallen. Ein Vorschlag ist ein Ticket und wird in seiner geöffneten Karte freigegeben.
- Die Zuordnung der Tickets zu den Gruppen bleibt, wie `job-board-ansicht` sie festgelegt hat.

Daneben: Im Ablauf bleibt die Zeile unter jeder Station einzeilig, das Modell steht ohne Anbieterpräfix, und alle Beträge der Ansicht haben ein Dezimalkomma.
