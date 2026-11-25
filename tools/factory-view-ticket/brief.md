Ein Ticket zeigt seinen ganzen Auftrag, und die zweite Auftragsliste entfällt.

Beobachtung des Nutzers am 2026-11-25: „ticket system und auftrags verwaltung muss noch harmonisiert werden und ich sehe irgendwie noch nicht wirklich viel im ticket, abgeschnittene meldungen. Unvollständiger context." Die Ansicht zeigte einen Auftrag an drei Stellen: als aufgeklapptes Ticket in der Liste mit gekürzten Zeilen, als Karte „Auftragsziel" und als Karte „Auftragsverlauf" mit eigener Auswahlliste. Das offene Vorhaben aus `job-prozess-reflexion/summary-02.run` verlangt dasselbe.

Verlangt:

- Es gibt genau einen Ort für einen Auftrag: das Ticket `#taik-ticket`. Ein Klick auf einen Eintrag der Ticketliste wählt ihn aus, die Liste klappt nichts mehr auf.
- Das Ticket zeigt Titel, Kennung, Zustand, Agent und Kosten, die vollständige Beschreibung, dann in der Reihenfolge des Geschehens alle Nachrichten ungekürzt und jede Runde mit Entscheidung, vollem Befund des Gates, Arbeitsschritten und Prüfprotokoll, dann die Wahl des Agenten und das Antwortfeld.
- Die Auswahlliste `taik-live-job`, die Karte „Auftragsziel" und die Karte „Software-Auftrag" entfallen. Der Knopf zum Ergebnis sitzt im Ticket.
- Ein Vorschlag der Factory ist ebenfalls ein Ticket, mit Freigabe und Kommentar.
- Kein Text im Ticket wird abgeschnitten, auch nicht in der schmalen Ansicht.

Nicht Teil dieser Runde: ein Ticket in der Ansicht anlegen, Kommentare an Vorschlägen in die Unterhaltung führen.

Prüfung: `tools/factory-view-ticket/verify.mjs`, vor dem Kandidaten geschrieben. Die WebKit-Prüfung `tools/check-factory-design.js` wählt Aufträge jetzt über ihr Ticket statt über die Auswahlliste. Der vorige Stand der Prüfung liegt als `tools/check-factory-design.v2.js`.
