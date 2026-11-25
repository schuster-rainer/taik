Die Ticketliste zeigt den ganzen Bestand, auch alte Aufträge mit Details und Verlauf.

Beobachtung des Nutzers am 2026-11-25: „wo ist jetzt meine ganze ticket übersicht?", „ich will alte auch sehen können", „und details, und den verlauf. du musst dafür keine neuen daten generieren." Die App lud nur die 12 zuletzt geänderten von 86 Auftragsordnern, die übrigen waren nicht erreichbar.

Verlangt:

- Die App liefert für jeden Auftragsordner eine leichte Zeile (`index`: Kennung, Titel, letzte Entscheidung, Runden). Die vollen Daten bleiben bei den neuesten Aufträgen.
- `boardTickets` reiht jeden nicht geladenen Auftrag unter „Entschieden" ein, jeden nur einmal, mit „Angenommen", „Abgelehnt" oder „Ohne Runde". Ein alter abgelehnter Auftrag wartet nicht auf den Nutzer.
- Ein Suchfeld `taik-board-search` grenzt die Tickets nach Titel und Kennung ein (`filterTickets`). Beim Tippen bleibt das Feld im Fokus.
- Ein Klick auf einen alten Auftrag meldet `showJob` an die App. Sie lädt ihn vollständig aus den vorhandenen Protokollen, und das Ticket zeigt Beschreibung, Runden und Verlauf wie bei jedem anderen.

Prüfung: `tools/factory-view-bestand/verify.mjs`, vor dem Kandidaten geschrieben.
