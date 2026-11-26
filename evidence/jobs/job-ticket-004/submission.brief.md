Die Karte „Ablauf" der Ansicht soll für das gewählte Ticket den wirklichen Arbeitsablauf der Factory mit sechs Stationen, ihren Ausführenden und ihrem Zustand zeigen.

Du änderst das Produkt ansicht. Gehe von der angenommenen Quelle der Ansicht aus und übernimm alles wörtlich, was du nicht änderst.

Was sich ändert
- Die Karte „Ablauf" zeigt nicht mehr die fünf festen Stationen, sondern für das ausgewählte Ticket genau sechs Stationen in dieser Reihenfolge und mit diesen Bezeichnungen: „Wunsch", „Plan des Orchestrators", „Gate", „Freigabe", „Arbeit", „Urteil".
- Jede Station nennt ihren Ausführenden als @name:
  - Bei einem Vorschlag (proposal) stammen Gate-Autor und Arbeiter aus proposal.plan, Feld gate für die Station „Gate" und Feld work für die Station „Arbeit".
  - Bei einem Auftrag (job) stammt der Ausführende der Station „Gate" aus job.agent und der der Station „Arbeit" aus dem Feld worker der Runden des Auftrags.
  - Fehlt eine Angabe oder ist sie leer, steht dort „Standard". Das gilt auch für die übrigen Stationen, für die keine eigene Quelle genannt ist.
- Jede Station zeigt ihren Zustand als eines von drei Wörtern: „erledigt", „aktiv", „offen".
  - Es ist höchstens eine Station „aktiv".
  - Stationen vor der aktiven sind „erledigt", Stationen nach ihr sind „offen".
  - Ist das Ticket abgeschlossen oder abgelehnt, sind alle Stationen „erledigt".
- Jede Station zeigt Bezeichnung, @name und Zustand als lesbaren Text, zum Beispiel „Gate · @savant · erledigt". Die Reihenfolge der drei Angaben ist in allen Stationen gleich.
- Die Darstellung bleibt stilistisch bei der bestehenden Ansicht: gleiche Farben, Schriften, Abstände und Kartenform wie im Rest der Ansicht, gleiches Markup-Muster, gleiches Skript-Muster. Kein externes Netzwerk, keine externen Ressourcen.

Verhalten, das geprüft wird
- Wählt man ein Ticket mit proposal.plan = { gate: "savant", work: "standard" }, erscheint bei „Gate" @savant und bei „Arbeit" @standard.
- Wählt man ein Ticket mit proposal.plan = { gate: "standard", work: "savant" }, erscheint bei „Gate" @standard und bei „Arbeit" @savant.
- Fehlen gate oder work, erscheint an dieser Station „Standard". Ist gar kein Ausführender bekannt, erscheint an allen Stationen „Standard".
- Bei einem Auftrag mit job.agent = "orchestrator" und Runden mit worker = "savant" erscheint bei „Gate" @orchestrator und bei „Arbeit" @savant.
- Wechselt man zwischen zwei Tickets mit verschiedenen Angaben, ändert sich die Karte mit und zeigt keine Reste des vorigen Tickets.
- Zu jedem Zeitpunkt ist genau dann keine Station „aktiv", wenn das Ticket abgeschlossen oder abgelehnt ist; sonst ist genau eine Station „aktiv", alle davor „erledigt", alle dahinter „offen".
- Ein Ticket ohne jede Angabe zu Ausführenden zeigt sechs Stationen mit „Standard".

Ausgabe
- Gib die vollständige Ansicht als ein Dokument aus (Text, höchstens 512 KiB), so wie sie angenommen wurde; ein Bundle nur, wenn die Ansicht aus mehreren Dateien besteht.
- Alles andere bleibt, wie es ist: Tickets, Crew, Auftragsverlauf, Ablauf und Werkstück behalten ihre bisherige Funktion und ihr bisheriges Aussehen.
