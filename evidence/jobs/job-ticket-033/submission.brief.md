Die Orchestrator-Karte der Ansicht zeigt zuerst die offenen Einträge und hält abgelehnte und zurückgezogene Wünsche in einem zugeklappten Bereich zurück.

Dies ist eine Änderung am bestehenden Produkt ansicht. Ausgangspunkt ist der angenommene Quelltext der Ansicht. Geändert wird nur, was hier steht. Alles andere bleibt, wie es ist, einschließlich aller anderen Karten, der Reihenfolge der Karten, des Eingabefelds, der Stile und aller bestehenden Funktionen.

## Ausgangslage

Die Karte „Orchestrator" zeigt unter dem Eingabefeld bis zu fünf Einträge aus `orchestratorFeed(snapshot)`. Seit dort auch abgelehnte und zurückgezogene Wünsche erscheinen, verdrängen diese die offenen Einträge und schieben die Karte „Tickets" weit nach unten.

## Anforderungen

1. `orchestratorFeed` bleibt unverändert. Sie liefert weiter alle Einträge, neueste zuerst.

2. Im Block BOARD (zwischen `/*BOARD_BEGIN*/` und `/*BOARD_END*/`) gibt es die neue reine Funktion `feedParts(feed)`.
   - Eingabe ist die Liste, die `orchestratorFeed` liefert.
   - Rückgabe ist ein Objekt `{ open, done }`, beide Werte sind Arrays.
   - `done` enthält die Einträge, deren Zeile (`line`) genau „Abgelehnt" oder genau „Fehlgeschlagen" lautet.
   - `open` enthält alle anderen Einträge.
   - Die Einträge selbst werden unverändert übernommen (dieselben Objekte).
   - Die Reihenfolge innerhalb von `open` und innerhalb von `done` entspricht der Reihenfolge der Eingabe.
   - Bei einer leeren Liste sind `open` und `done` leere Arrays.
   - Die Funktion verändert ihre Eingabe nicht und hat keine Nebenwirkungen.

3. Darstellung der Karte „Orchestrator" unter dem Eingabefeld:
   - Zuerst stehen alle offenen Einträge, höchstens fünf, in der bisherigen Darstellung und Reihenfolge.
   - Gibt es erledigte Einträge, steht darunter ein aufklappbarer Bereich mit dem Titel „Abgelehnt und zurückgezogen". Hinter dem Titel steht die Anzahl aller erledigten Einträge in Klammern, zum Beispiel „Abgelehnt und zurückgezogen (3)".
   - Der Bereich ist zunächst zugeklappt.
   - Er merkt sich wie die anderen aufklappbaren Bereiche der Ansicht, ob er geöffnet wurde. Der Schlüssel dafür lautet `orchestrator/done`. Nutze denselben Mechanismus wie die vorhandenen aufklappbaren Bereiche.
   - Geöffnet zeigt er die erledigten Einträge, höchstens zehn, in derselben Darstellung wie bisher die Einträge der Karte.
   - Gibt es keine erledigten Einträge, erscheint der Bereich nicht.

4. Gibt es weder offene noch erledigte Einträge, steht unter dem Eingabefeld nichts.

## Hinweise

- Das Ergebnis bleibt ein HTML-Fragment mit Stil, Markup und Skript wie bisher, und der Block BOARD bleibt als solcher erhalten.
- Die Anzahl in Klammern zählt alle erledigten Einträge, auch wenn nur zehn angezeigt werden.
- Weitere Änderungen sind nicht erlaubt.
