Das Ticket eines Vorschlags soll unter der Beschreibung einen zweiten aufklappbaren Bereich „Gate" zeigen, damit der Text der Prüfung vor der Freigabe lesbar ist.

## Änderung

Die App liefert für jeden Vorschlag das Feld `proposal.gate`. Es enthält den Text der Prüfung, also ein Shell-Skript oder ein Node-Modul. Solange die Prüfung noch nicht geschrieben ist, fehlt das Feld.

Im Ticket eines Vorschlags erscheint unter der Beschreibung ein zweiter aufklappbarer Bereich. Er wird nach demselben Muster gebaut wie der bestehende Bereich der Beschreibung.

### Kopfzeile

- Die Überschrift lautet „Gate", gefolgt von der Zeilenzahl in Klammern, zum Beispiel „Gate (89 Zeilen)".
- Die Zeilenzahl ist die Zahl der Zeilen von `proposal.gate`.
- Ein abschließender Zeilenumbruch am Textende erzeugt keine zusätzliche Zeile.
- Bei genau einer Zeile lautet der Text „Gate (1 Zeilen)". Es gibt keine Sonderform für den Singular.

### Zustand

- Der Bereich ist zunächst zugeklappt.
- Er merkt sich, ob der Nutzer ihn geöffnet hat, mit demselben Verfahren wie die Beschreibung. Dazu dient das Attribut `data-round`.
- Der Schlüssel lautet `proposal:NAME/gate`. `NAME` ist der Name des Vorschlags, genau wie beim Schlüssel der Beschreibung.
- Ein Neuzeichnen der Ansicht oder ein neuer Datenstand darf einen geöffneten Bereich nicht schließen und einen zugeklappten nicht öffnen.
- Der Zustand der Beschreibung und der Zustand des Gate-Bereichs sind voneinander unabhängig.

### Inhalt

Geöffnet zeigt der Bereich den vollständigen Text von `proposal.gate` in einem `pre`-Element. Das `pre`-Element hat dieselbe Darstellung wie das der Beschreibung:

- feste Zeichenbreite
- erhaltene Zeilenumbrüche
- begrenzte Höhe, bei Überlänge scrollbar
- nichts abgeschnitten
- kein Überlauf nach rechts: lange Zeilen werden umbrochen oder die Breite wird begrenzt, so wie es bei der Beschreibung bereits geschieht
- Der Text wird als reiner Text eingesetzt, nicht als HTML. Zeichen wie `<`, `&` und `$` erscheinen unverändert.

### Fehlendes Gate

Fehlt `proposal.gate` oder ist es nicht vorhanden, erscheint der Bereich gar nicht. Weder eine Kopfzeile noch ein leerer Rahmen wird gezeigt.

## Unverändert

Alles andere bleibt, wie es ist: Tickets, Crew, Auftragsverlauf, Ablauf und Werkstück, die Beschreibung des Vorschlags samt ihrem Verhalten, das bestehende Layout, der Stil und die Skriptlogik außerhalb dieser Änderung. Ändere nur, was für den neuen Bereich nötig ist.

## Ausgabe

Das Format des Produkts bleibt ein HTML-Fragment mit Stil, Markup und Skript, wie es die bisherige Ansicht ist.
