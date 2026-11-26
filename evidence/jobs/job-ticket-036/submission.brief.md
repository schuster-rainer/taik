Die Ansicht zeigt im Werkstück während einer laufenden Sitzung live, was der Agent gerade tut.

# Änderung am Produkt ansicht

Geändert wird nur das Werkstück (Block WORKPIECE und die Karte „Werkstück"). Alles andere bleibt, wie es ist: Tickets, Crew, Auftragsverlauf, Ablauf, alle bestehenden Funktionen, Namen, Ergebnisse und das Verhalten für Nicht-Sitzungsaufträge.

## Ausgangslage

Läuft die Runde eines Sitzungsauftrags (job.sourceActive ist wahr), zeigt das Werkstück bisher die Änderung der vorigen Runde oder „Diese Sitzung hat noch nichts geändert.". Die App liefert aber schon während der Runde `job.dialogs[Name der laufenden Runde]`. Das ist eine Liste in zeitlicher Reihenfolge mit Einträgen:
- `{ who: "agent" | "user", text }` für Gesagtes,
- `{ tool, target }` für einen Werkzeugaufruf (tool z. B. "Read", "Glob", "Grep", "Edit", "Write"; target die Datei oder das Suchmuster).

Der Name der laufenden Runde ist "round-" plus die zweistellige Nummer `job.rounds.length + 1` (z. B. "round-03").

## Laufende Sitzung

Ein Auftrag gilt als laufende Sitzung, wenn job.sourceActive wahr ist und `job.dialogs` für die laufende Runde einen Eintrag hat (mindestens ein Eintrag in dieser Liste). Das gilt auch, wenn noch keine Runde ein Feld commit hat.

## Anforderungen

1. Im Block WORKPIECE (zwischen /*WORKPIECE_BEGIN*/ und /*WORKPIECE_END*/) gibt es die reine Funktion `sessionLive(job)`; `workpieceState` und `sessionChange` bleiben im selben Block.
   - Für eine laufende Sitzung liefert sie `{ round, steps, last, read, changed, said }`:
     - `round`: Nummer der laufenden Runde (job.rounds.length + 1, als Zahl).
     - `steps`: Zahl der Einträge mit `tool` (Werkzeugaufrufe).
     - `last`: der letzte Werkzeugaufruf als Text „Werkzeug Ziel", z. B. „Edit src/main.rs", sonst eine leere Zeichenkette.
     - `read`: die verschiedenen Ziele der Aufrufe von Read, in der Reihenfolge ihres ersten Auftretens.
     - `changed`: die verschiedenen Ziele der Aufrufe von Edit und Write, in der Reihenfolge ihres ersten Auftretens (gemeinsam, ohne Doppelte).
     - `said`: Text des letzten Eintrags mit who "agent", sonst eine leere Zeichenkette.
   - Für jeden anderen Auftrag (auch sourceActive wahr ohne Eintrag für die laufende Runde, oder sourceActive falsch) und für einen fehlenden Auftrag (undefined/null) liefert sie `null`.
   - Sie verändert ihre Eingabe nicht.

2. `workpieceState(job, mode)` liefert für eine laufende Sitzung `{ show: 'live', unverified: true }`, unabhängig von mode. Alle anderen Ergebnisse bleiben wie bisher.

3. Bei show 'live' zeigt die Karte „Werkstück":
   - in der Kopfzeile den Namen des Auftrags ohne das Präfix "job-", den Text „Runde N läuft" und die Zahl der Schritte;
   - darunter: den letzten Schritt hervorgehoben; „Geändert" mit der Liste aus changed oder „noch nichts"; „Gelesen" mit der Liste aus read oder „noch nichts"; und, wenn said nicht leer ist, „Zuletzt gesagt" mit said.
   - Dateinamen stehen in code-Elementen.
   - Nichts wird abgeschnitten (kein Text-Overflow mit Auslassungspunkten), lange Listen sind scrollbar (begrenzte Höhe mit vertikalem Scrollen), es gibt keinen waagerechten Überlauf (lange Namen umbrechen).
   - Die Reiter „Ansicht", „Quelltext" und „Diff" sind dabei ausgeblendet oder deaktiviert.
   - Text aus Dialogen wird als Text eingesetzt, nicht als HTML.

4. Ist die Runde fertig (kein laufender Sitzungsauftrag mehr), gilt wieder das bisherige Verhalten für Sitzungsaufträge, einschließlich der Reiter.

## Beispiel

Job mit sourceActive wahr, rounds mit 2 Einträgen, dialogs["round-03"] = [{tool:"Read",target:"a.rs"},{tool:"Read",target:"a.rs"},{who:"agent",text:"Ich suche"},{tool:"Edit",target:"src/main.rs"}]. Ergebnis: `{ round: 3, steps: 3, last: "Edit src/main.rs", read: ["a.rs"], changed: ["src/main.rs"], said: "Ich suche" }`.
