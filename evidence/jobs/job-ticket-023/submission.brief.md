Die Ansicht soll Vorschläge ablehnen und Wünsche zurückziehen können, mit den Knöpfen „Ablehnen" und „Zurückziehen" und der reinen Funktion ticketActions(proposal).

# Änderung am Produkt ansicht

Geändert wird nur das Folgende. Alles andere an der Ansicht bleibt, wie es ist: Layout, Stil, übrige Knöpfe, Nachrichten, Anzeige von Tickets, Crew, Auftragsverlauf, Ablauf und Werkstück.

## Hintergrund

Die App nimmt jetzt die Nachricht `declineTicket` mit `{ name: "ticket-…", text: "Begründung" }` an. Ein abgelehnter Vorschlag und ein zurückgezogener Wunsch kommen danach mit `proposal.state` gleich `"declined"` und der Begründung in `proposal.note` zurück. Dafür ist an der Ansicht nichts zu ändern.

## Neue reine Funktion ticketActions(proposal)

Die Funktion steht im Block BOARD (zwischen `/*BOARD_BEGIN*/` und `/*BOARD_END*/`). Sie ist rein: Sie liest nur ihr Argument, hat keine Nebenwirkungen und braucht weder DOM noch andere Zustände. Sie liefert eine Liste von Bezeichnungen (Strings) in der Reihenfolge der Knöpfe:

- Vorschlag ohne `state` (kein `proposal.state`, also undefined, null oder leer): `["Freigeben", "Ablehnen", "Senden"]`
- `state` gleich `"asks"`: `["Zurückziehen", "Senden"]`
- jeder andere `state`: `[]`
- fehlender Wert (`undefined` oder `null` als `proposal`): `[]`

## Ticket und Knöpfe

1. Das Ticket baut seine Knöpfe aus der Liste von `ticketActions(proposal)`, in genau dieser Reihenfolge. „Freigeben" und „Senden" funktionieren wie bisher.
2. Ein Ticket eines Vorschlags, der auf Freigabe wartet (kein `proposal.state`), zeigt neben „Freigeben" und „Senden" den Knopf „Ablehnen".
3. Ein Ticket eines Wunsches, zu dem der Orchestrator fragt (`proposal.state` gleich `"asks"`), zeigt neben „Senden" den Knopf „Zurückziehen".
4. Die Knöpfe „Ablehnen" und „Zurückziehen" tragen das Attribut `data-ticket-decline`, dessen Wert der Schlüssel des Tickets ist (derselbe Schlüssel, den die Ticketknöpfe sonst zur Zuordnung nutzen, also mit dem Präfix `proposal:`, sofern das Ticket ihn so führt).

## Verhalten beim Klick

Beide Knöpfe nehmen den Text aus dem Antwortfeld des Tickets als Begründung.

- Ist das Feld leer (auch nur Leerraum), bekommt es den Fokus, und es wird nichts gesendet.
- Sonst rufen sie `window.webkit.messageHandlers.declineTicket.postMessage({ name, text })` auf. `name` ist der Ticketschlüssel ohne das Präfix `proposal:` (zum Beispiel `ticket-023`), `text` ist die Begründung. Danach wird der Entwurf des Feldes geleert, so wie es bei „Senden" mit dem Entwurf geschieht.

## Format

Wie bisher: ein HTML-Fragment mit Stil, Markup und Skript, vollständig ausgegeben. Der Block BOARD und seine Struktur bleiben erhalten, `ticketActions` kommt hinzu.
