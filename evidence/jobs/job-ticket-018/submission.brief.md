Die Karte „Orchestrator“ der Ansicht soll in orchestratorFeed auch abgelehnte und fehlgeschlagene Wünsche zeigen.

Ändere ausschließlich die Funktion orchestratorFeed und die Darstellung der Karte „Orchestrator“ im Produkt ansicht. Alles andere an der Ansicht bleibt unverändert, ebenso alle Einträge, die schon bisher erscheinen.

orchestratorFeed liegt weiter im Block zwischen /*BOARD_BEGIN*/ und /*BOARD_END*/, nimmt weiter data (mit data.proposals) entgegen und gibt weiter ein Array von Einträgen mit den Feldern key, title, line und reason zurück. Die Funktion planLine im selben Block bleibt unverändert.

Neues Verhalten von orchestratorFeed:

1. Ein Wunsch, dessen proposal.state den Wert "declined" oder "failed" hat, wird als Eintrag in die Rückgabe aufgenommen. Das gilt auch dann, wenn er zusätzlich einen Plan hat.
2. Die Einträge stehen in derselben Reihenfolge wie die übrigen, neueste zuerst. Sie werden nach demselben Kriterium einsortiert wie die bisherigen Einträge. Es gibt keine eigene Gruppe am Anfang oder am Ende.
3. Das Feld line lautet "Abgelehnt" bei declined und "Fehlgeschlagen" bei failed.
4. Das Feld reason ist der Text aus proposal.note. Fehlt proposal.note oder ist es leer (auch nur Leerraum), lautet reason "ohne Begründung".
5. Hat ein solcher Eintrag zusätzlich einen Plan, steht die Planzeile, die sonst in line stünde, nicht in line. Sie wird dem Grund vorangestellt, getrennt durch " · " (Leerzeichen, Mittelpunkt, Leerzeichen). Dann gilt reason = Planzeile + " · " + Grund. Der Grund ist dabei der Text aus proposal.note oder "ohne Begründung". line bleibt "Abgelehnt" bzw. "Fehlgeschlagen".
6. Einträge mit Plan, deren Zustand nicht declined oder failed ist, und Einträge im Zustand deriving bleiben genau so, wie sie bisher erzeugt werden.

Darstellung in der Karte: Die Zeile (line) eines Eintrags im Zustand declined oder failed wird in der Farbe der Ablehnungen gezeigt, also mit var(--bad). Die Zeilen der übrigen Einträge behalten ihre bisherige Farbe. Der Grund wird wie bei den übrigen Einträgen angezeigt. Nutze dafür die vorhandene Struktur, die Klassen und die Namen der Karte weiter, und erfinde keine neuen Felder.

Das Ergebnis ist die gesamte geänderte Ansicht als ein HTML-Fragment im bisherigen Format des Produkts.
