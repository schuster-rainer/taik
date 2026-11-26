Die Ansicht soll Rückfragen des Orchestrators zu einem Wunsch anzeigen und die Beantwortung ermöglichen.

Änderung am Produkt ansicht. Ausgangspunkt ist der angenommene Quelltext der Ansicht. Es ändert sich ausschließlich das Folgende; alles andere bleibt unverändert, auch alle Namen, Schnittstellen, Darstellungen und das Verhalten für Einträge in anderen Zuständen.

Hintergrund: Der Orchestrator kann jetzt zu einem Wunsch zurückfragen. Die App liefert so einen Wunsch in snapshot.proposals mit proposal.state "asks". Das Gespräch steht in proposal.messages. Das sind Einträge der Form { author: "agent" | "user", round, text }, die ältesten zuerst. Bei state "asks" ist die letzte Nachricht die Frage des Agenten.

1. boardTickets (Block BOARD_BEGIN/BOARD_END, boardTickets(data) liefert { open, run, decision })
- Ein Vorschlag mit state "asks" erscheint unter „Wartet" (Gruppe open).
- Er trägt das Kennzeichen „Frage" (Feld flag).
- Sein Grund (why) ist „Der Orchestrator fragt: " (mit Doppelpunkt und einem Leerzeichen), gefolgt vom Text der letzten Nachricht des Agenten.
- Sein Schlüssel (key) ist wie bei anderen Vorschlägen "proposal:" + Name.

2. orchestratorFeed (gleicher Block BOARD_BEGIN/BOARD_END, orchestratorFeed(data) liefert eine Liste mit { key, title, line, reason })
- Ein Vorschlag mit state "asks" erscheint in der Liste.
- Seine Zeile (line) ist „Der Orchestrator fragt."
- Sein Grund (reason) ist der Text der Frage, also der letzten Agent-Nachricht.

3. Ticket eines solchen Eintrags
- Das Ticket zeigt das Gespräch in der vorhandenen Darstellung der Nachrichten (Agent und Du), also so, wie die Ansicht Nachrichten von Vorschlägen schon darstellt.
- Darunter steht das Antwortfeld mit dem Knopf „Senden".
- Es gibt keinen Knopf „Freigeben".
- „Senden" verschickt die Antwort wie ein Kommentar an einem Vorschlag, und zwar über commentTicket mit { name, text }. name ist der Schlüssel ohne das Präfix "proposal:", text ist der eingegebene Text.

4. Karte „Ablauf" (Block FLOW_BEGIN/FLOW_END, flowStations(job, proposal) liefert die sechs Stationen mit name, actor, state, failed)
- Für einen Vorschlag mit state "asks" (flowStations(null, proposal)) ist die Station „Plan des Orchestrators" aktiv.
- „Wunsch" ist erledigt.
- Alle späteren Stationen sind offen.
- Einträge in anderen Zuständen behalten ihre bisherige Darstellung der Stationen.

Ausgabe: wie bisher bei diesem Produkt, das vollständige HTML-Fragment der Ansicht (Stil, Markup und Skript) in derselben Form wie der bisherige Quelltext, mit nur diesen Änderungen.
