Im Werkstück-Feld ein Fragment als Quelltext zeigen statt als leere Hülle.

Du bekommst die installierte Ansicht als angenommenen Ausgangsstand. Liefere einen Patch. Ändere nur die Funktion `workpieceState` zwischen `/*WORKPIECE_BEGIN*/` und `/*WORKPIECE_END*/` und was unmittelbar von ihrer Regel abhängt. Alles andere bleibt Zeichen für Zeichen erhalten.

Beobachtung: Die Factory-Ansicht selbst ist ein Fragment, das mit einem `<style>`-Block beginnt. Als Werkstück eines Ansichts-Auftrags wurde sie im Modus „Ansicht" als Dokument gerendert und erschien als leere Hülle.

Neue Regel: Ein Text „ist ein Dokument" nur noch, wenn er nach führendem Leerraum mit `<!doctype` oder `<html` beginnt, Groß- und Kleinschreibung egal. Ein Text, der mit `<style` beginnt, ist kein Dokument. Er erscheint im Modus `view` als `'source'`, sowohl als Entwurf als auch als angenommenes Artefakt. Alle anderen Regeln von `workpieceState` bleiben, wie sie sind.

Gibt es an anderer Stelle der Ansicht eine eigene Prüfung, ob ein Text ein Dokument ist, die über das Rendern im Werkstück-Feld entscheidet, gilt dort dieselbe Regel.
