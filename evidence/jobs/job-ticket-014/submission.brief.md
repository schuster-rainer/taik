Die Ansicht zeigt im Ablauf und im Ticket, wer geplant, geprüft und gearbeitet hat und auf welchem Zweig das Ergebnis liegt.

Geändert wird nur das Produkt ansicht (ein HTML-Fragment mit Stil, Markup und Skript). Alles andere bleibt so, wie es ist: Aufbau, Karten, Texte, Stil und Verhalten für Aufträge ohne die neuen Felder.

Neue Daten: Die App liefert für jeden Auftrag zwei weitere optionale Felder.
- job.plan hat dieselbe Form wie proposal.plan: kind, repo, gate, work, reason. Das Feld fehlt, wenn es keinen Plan gibt.
- job.adopted ist der Name des Zweigs, auf dem das angenommene Ergebnis einer Sitzung im Ursprungs-Repository liegt, zum Beispiel "taik/ticket-013". Das Feld fehlt, wenn es keinen solchen Zweig gibt.

Gewünschte Änderungen:

1. Karte „Ablauf"
   - Hat ein Auftrag job.plan, nennt die Karte bei der Station „Gate" den Autor aus job.plan.gate und bei der Station „Arbeit" den Agenten aus job.plan.work.
   - Dem Namen wird jeweils ein @ vorangestellt, zum Beispiel „@codex".
   - Der Name „standard" wird als „Standard" ohne @ gezeigt.
   - Hat der Auftrag keinen Plan, bleibt die Karte wie bisher.

2. Ticket eines Auftrags mit job.plan
   - Das Ticket zeigt unter dem Stand denselben Block „Plan" und „Grund", den ein Vorschlags-Ticket schon zeigt.
   - Dafür wird dieselbe Darstellung (planLine und der Grund) verwendet, mit den Werten aus job.plan.
   - Ohne job.plan erscheint der Block nicht.

3. Ticket eines Auftrags mit job.adopted
   - Das Ticket zeigt im Stand eine zusätzliche Zeile mit der Bezeichnung „Ergebnis".
   - Der Text lautet „Zweig " gefolgt vom Zweignamen. Der Zweigname steht in einem code-Element und wird nicht abgeschnitten, also ohne Kürzung durch Auslassungspunkte, ohne Begrenzung der Länge und ohne verstecktes Überlaufen.
   - Ist job.plan.repo bekannt, folgt dahinter der Hinweis „im Repository " gefolgt von job.plan.repo. Ist das Repository unbekannt, entfällt der Hinweis.
   - Ohne job.adopted erscheint die Zeile nicht.

Regeln:
- Alle Werte aus den Daten werden als Text eingesetzt, nicht als HTML, damit Sonderzeichen wie < und & unverändert und sicher erscheinen.
- Die Ansicht muss weiter fehlerfrei laufen, wenn job.plan, job.adopted oder einzelne Felder darin fehlen.
- Das Ergebnis bleibt ein HTML-Fragment im bisherigen Format des Produkts. Der Ablauf der Freigabe (Layoutprüfung im Browser, Installation) bleibt unverändert.
