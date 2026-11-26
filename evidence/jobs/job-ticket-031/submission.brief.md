Die Ansicht der Factory soll im Ticket eines wartenden Vorschlags eine Auswahl für den Arbeiter zeigen, die vor der Freigabe den Agenten wählen lässt.

Ändere das Produkt ansicht. Genau das Folgende ändert sich, alles andere bleibt, wie es ist. Das gilt auch für planLine(plan), das unverändert bleibt.

Neue Daten der App:
- Ein Vorschlag kann das Feld proposal.worker haben. Das ist der Agent, den der Nutzer statt des geplanten Arbeiters gewählt hat. Das Feld fehlt, solange nichts gewählt wurde.
- Die App nimmt die Nachricht assignWorker mit { name, agent } an. name ist der Name des Vorschlags, agent der Name eines Agenten oder "standard".
- Die Agenten der Crew liefert wie bisher crewOf(snapshot).agents, mit Einträgen { name, harness, … }.

1. Reine Funktion workerChoices(proposal, agents) im Block CREW (zwischen /*CREW_BEGIN*/ und /*CREW_END*/)
Sie liefert ein Array von Namen in dieser Reihenfolge: zuerst "standard", dann die Namen der Agenten in der gegebenen Reihenfolge. Folgende Regeln gelten:
- Der Agent, der das Gate geschrieben hat (proposal.plan.gate), ist nicht enthalten. Das gilt auch für "standard", falls proposal.plan.gate gleich "standard" ist.
- Ist proposal.plan.kind gleich "sitzung", sind nur "standard" und Agenten mit harness "claude" enthalten.
- Agenten mit gesetztem Feld error (ein wahrer Wert) sind nie enthalten.
- Hat der Vorschlag keinen Plan (proposal.plan fehlt) oder hat er ein Feld state (gesetzt, wahr), ist die Liste leer.
- Die Funktion verändert ihre Argumente nicht.

2. Reine Funktion plannedWorker(proposal) im selben Block CREW
Sie liefert proposal.worker, falls vorhanden, sonst proposal.plan.work, sonst "standard". Sie muss auch ohne plan funktionieren.

3. Auswahl im Ticket
Das Ticket eines wartenden Vorschlags mit Plan zeigt über dem Antwortfeld ein select-Element mit der Beschriftung „Arbeit macht". Es enthält diese Merkmale:
- Die Einträge sind die Namen aus workerChoices, in derselben Reihenfolge.
- Der Eintrag "standard" erscheint mit dem Text „Standard", alle anderen mit dem Text „@name", also mit vorangestelltem @.
- Der Wert (value) jedes Eintrags ist der Name aus workerChoices, also "standard" oder der Agentenname ohne @.
- Gewählt ist plannedWorker(proposal). Steht dieser Name nicht in der Liste, ist kein Eintrag besonders gewählt, der Standardverhalten des Browsers genügt.
- Das select-Element trägt das Attribut data-assign mit dem Namen des Vorschlags ohne das Präfix "proposal:".
- Ändert der Nutzer die Auswahl (Ereignis change), sendet die Ansicht window.webkit.messageHandlers.assignWorker.postMessage({ name, agent }). name ist der Name des Vorschlags ohne "proposal:", agent ist der gewählte value.
- Ist workerChoices leer, erscheint keine Auswahl und auch keine Beschriftung.
- Die Auswahl wird beim erneuten Zeichnen nicht doppelt erzeugt. Eine Eingabe im Antwortfeld darf durch die Auswahl nicht verloren gehen, soweit die Ansicht das bisher schon sicherstellt.

4. Anzeige des gewählten Arbeiters
Für einen Vorschlag mit proposal.worker zeigen die Zeile „Plan" im Ticket und die Station „Arbeit" in der Karte „Ablauf" diesen Agenten statt plan.work. Ohne proposal.worker bleibt die Anzeige wie bisher. planLine(plan) selbst bleibt unverändert. Die Ersetzung geschieht beim Aufrufer oder durch einen Plan, der eine Kopie mit work gleich proposal.worker erhält. Das Original darf nicht verändert werden.

Das Ergebnis bleibt ein HTML-Fragment mit Stil, Markup und Skript im bisherigen Aufbau. Die Funktionen workerChoices und plannedWorker müssen wie die übrigen Funktionen des Blocks CREW erreichbar sein, so wie dort bereits crewOf und planLine.
