Die Ansicht der Factory zeigt bei einem Sitzungsauftrag im Werkstück die Änderung der Sitzung statt des Dokuments eines fremden Auftrags.

Ändere ausschließlich das bestehende Produkt „ansicht" (HTML-Fragment mit Stil, Markup und Skript). Alles andere bleibt genau wie bisher: Layout, die übrigen Karten, alle bestehenden Namen und Funktionen sowie das Verhalten für Aufträge, die keine Sitzungsaufträge sind.

## Begriff Sitzungsauftrag

Ein Auftrag `job` ist ein Sitzungsauftrag, wenn mindestens eine seiner Runden in `round.fields` ein nicht leeres Feld `commit` hat. Ein Sitzungsauftrag hat kein Dokument, er ändert Dateien in einem Repository.

Die Änderung einer Runde steht in `job.changes[Name der Runde ohne ".run"]` als `{ file, added, removed, rejected, lines, more }`. `lines` sind die Zeilen des Patches. Jede beginnt mit "+", mit "-", mit zwei Leerzeichen oder ist die Zeile "···" (Lücke). `file` nennt die geänderten Dateien.

## Gewünschte Änderungen

1. `workpieceState(job, mode)` (im Block WORKPIECE_BEGIN/WORKPIECE_END) liefert für einen Sitzungsauftrag genau `{ show: 'session', unverified: false }`. Das gilt unabhängig von `mode` und davon, ob `job.preview` oder `job.draft` gesetzt ist. Für alle anderen Aufträge bleibt das Ergebnis genau wie bisher.

2. Neue reine Funktion `sessionChange(job)`, im selben Block WORKPIECE_BEGIN/WORKPIECE_END, neben `workpieceState` definiert. Eingabe: ein Auftrag. Ausgabe: für die letzte Runde (in der Reihenfolge von `job.rounds`), für die `job.changes[Name der Runde ohne ".run"]` existiert, das Objekt `{ round, change }` mit `round` = Name dieser Runde ohne ".run" und `change` = die Änderung dieser Runde. Hat keine Runde eine Änderung (oder fehlt `job.changes`), ist die Ausgabe `null`.

3. Ist das gewählte Ticket ein Sitzungsauftrag, zeigt die Karte „Werkstück" niemals das Dokument eines anderen Auftrags. Sie zeigt stattdessen die Änderung der letzten Runde, die eine Änderung hat (aus `sessionChange`):
   - in der Kopfzeile den Namen des Auftrags ohne das Präfix "job-", die Runde und die Zeile aus `changeLine(change)`,
   - darunter den Patch (`lines`) mit denselben Farben wie der Reiter „Diff": hinzugefügt, entfernt, unverändert und Lücke werden wie dort gefärbt. Nutze dieselben Klassen oder dieselbe Darstellung wie dort,
   - der Patch ist scrollbar und läuft nicht nach rechts über die Karte hinaus.
   
   Hat keine Runde des Auftrags eine Änderung, steht in der Karte der Text „Diese Sitzung hat noch nichts geändert."

4. Die Reiter „Ansicht", „Quelltext" und „Diff" sind bei einem Sitzungsauftrag ausgeblendet oder deaktiviert, weil es nur diese eine Darstellung gibt. Bei anderen Aufträgen funktionieren sie wie bisher.

5. Der Knopf „Ergebnis öffnen" im Ticket erscheint bei einem Sitzungsauftrag nicht. Bei anderen Aufträgen erscheint er wie bisher.

## Ausgabe

Liefere das vollständige geänderte HTML-Fragment im bisherigen Format des Produkts. Füge keine neuen Abhängigkeiten ein und benenne nichts um.
