# Nachweise und Ergebnisse

## Auftragsannahme

- [Kurzanleitung aus dem ersten Auftrag an den Dienst](jobs/job-auftragsannahme/round-01.html)
- [Auftrag](jobs/job-auftragsannahme/round-01.brief.md), [unabhängiges Gate](jobs/job-auftragsannahme/submission.gate.mjs) und [Entscheidung](jobs/job-auftragsannahme/round-01.run)

## Selbstverbesserung der Suche

- [Doppelte Kandidaten entfernt](jobs/job-selbstverbesserung/round-01.run)
- [Ungültige Kandidaten vor der Erzeugung ausgeschlossen](jobs/job-selbstverbesserung/round-02.run)
- [Suchprozedur beibehalten](jobs/job-selbstverbesserung/round-03.run)

## Automatisch abgeleiteter Forschungsauftrag

- [Auftrag aus der gemessenen Sucharbeit](jobs/job-suchentscheidungen-wiederverwenden/request.run)
- [Unabhängige Prüfung und gespeicherte Suchentscheidungen](jobs/job-suchentscheidungen-wiederverwenden/round-01.run)
- [Gemessene Startprüfung und Worker-Timeout](jobs/job-suchtabelle-bei-bedarf/round-01.run) und [automatischer Vergleich der bedarfsgesteuerten Prüfung](jobs/job-suchtabelle-bei-bedarf/round-02.run)
- [Echter Kaltstart und Rückfall auf die reguläre Suche](jobs/job-suchtabelle-kaltstart/round-01.run)

## Tag-Prozedur

- [Erster akzeptierter Stand](jobs/job-tag-prozedur/01-seed.run)
- [Abgelehnter Kandidat](jobs/job-tag-prozedur/02-rejected.run)
- [Wiederaufnahme und Weiterentwicklung](jobs/job-tag-prozedur/03-extended.run)
- [Neuordnung der Prozedur](jobs/job-tag-prozedur/07-local-reorder.run)
- [Reduktion vor dem Sortieren](jobs/job-tag-prozedur/08-local-reduce.run)
- [Entscheidung zum Beibehalten](jobs/job-tag-prozedur/09-local-retain.run)
- [Erneute Verwendung](jobs/job-tag-prozedur/10-local-reuse.run)
- [Gegenprobe mit ungültiger Reihenfolge](jobs/job-tag-prozedur/11-invalid-order.run)

## Galaxy und Intelligence

- [Versuch 12 mit Auswertung](jobs/job-galaxy-intelligence/summary.run)
- [Gemessene Einzelläufe](jobs/job-galaxy-intelligence/results.json)

## Reflexion des Prozesses

- [Die Spec auf den eigenen Betrieb angewendet](jobs/job-prozess-reflexion/summary.run)

## Galaxy an Software-Aufträgen

- [Versuch mit und ohne wiederverwendetes Ergebnis, mit Auswertung](jobs/job-galaxy-quellcode/summary.run)
- [Gemeinsames Gate](jobs/job-galaxy-quellcode/gate.mjs), [Auftrag ohne](jobs/job-galaxy-quellcode/brief-ohne.md) und [mit Wiederverwendung](jobs/job-galaxy-quellcode/brief-mit.md)

## Supermarkt-Floorplanner

- [Akzeptierter Floorplanner mit Hell- und Dunkelmodus](jobs/job-floorplanner/round-04.html)
- [Auftrag](jobs/job-floorplanner/round-01.brief.md)
- [Abgelehnte Runde](jobs/job-floorplanner/round-01.run), [erster akzeptierter Stand](jobs/job-floorplanner/round-02.run) und [Prüfung von Rückgängig](jobs/job-floorplanner/round-03.run)
- [Freigabe des Hell- und Dunkelmodus](jobs/job-floorplanner/round-04.run)

## Gemeinsamer Rust-Einstieg

- [Entscheidung](jobs/job-rust-entrypoint/round-01.run) und [ausgeführte Prüfungen](jobs/job-rust-entrypoint/round-01.checks.log)
- [Angenommenes macOS-Binary `taik-seed`](jobs/job-rust-entrypoint/round-01-taik-seed)
- [Erneuerung für den Auftragsdienst](jobs/job-rust-entrypoint/round-02.run), [Prüfungen](jobs/job-rust-entrypoint/round-02.checks.log) und [installiertes macOS-Binary](jobs/job-rust-entrypoint/round-02-taik-seed)
- [Erneuerung für automatisch erzeugte Aufträge](jobs/job-rust-entrypoint/round-03.run), [Prüfungen](jobs/job-rust-entrypoint/round-03.checks.log) und [installiertes macOS-Binary](jobs/job-rust-entrypoint/round-03-taik-seed)
- [Erneuerung für bedarfsgesteuerte Prüfung und Live-Vorschau](jobs/job-rust-entrypoint/round-04.run) und [ausgeführte Prüfungen](jobs/job-rust-entrypoint/round-04.checks.log)
- [Rückkopplung bei eingereihten HTML-Aufträgen](jobs/job-rust-entrypoint/round-05.run) und [ausgeführte Prüfungen](jobs/job-rust-entrypoint/round-05.checks.log)
- [Automatischer Folgeauftrag aus einem freigegebenen Ergebnis](jobs/job-rust-entrypoint/round-06.run) und [ausgeführte Prüfungen](jobs/job-rust-entrypoint/round-06.checks.log)
- [Kaltstartprüfung als Folgeexperiment im Dienst](jobs/job-rust-entrypoint/round-07.run) und [ausgeführte Prüfungen](jobs/job-rust-entrypoint/round-07.checks.log)

- [Eingehängten Worker beim Namen nennen](jobs/job-rust-entrypoint/round-08.run)
- [Abgeschlossene Aufträge nicht erneut lesen, mit Messung](jobs/job-rust-entrypoint/round-09.run) und [Messwerte](jobs/job-rust-entrypoint/round-09.queue-latency.json)
- [Zweiter unbeaufsichtigter Worker](jobs/job-rust-entrypoint/round-10.run), [Ausfall vorher](jobs/job-worker-verfuegbarkeit/round-01.run) und [Annahme danach](jobs/job-worker-verfuegbarkeit-2/round-01.run)
- [Opus als führender Worker](jobs/job-rust-entrypoint/round-11.run) und [Zuschreibung bei Fehlschlag, Gate-Dauer](jobs/job-rust-entrypoint/round-12.run)

- [Promotion durch die installierte Factory eingeführt](jobs/job-rust-entrypoint/round-13.run)
- [Erste Entscheidung der Factory über ihren Nachfolger](jobs/job-backend/round-01.run) und [Prüfungen](jobs/job-backend/round-01.checks.log)
- [Tagesgrenze für Worker-Kosten](jobs/job-backend/round-02.run)
- [Startup-Reflexion über den eingestellten Worker](jobs/job-backend/round-03.run)
- [Drosselung des Gates unter launchd, gemessen](jobs/job-dienst-drosselung/summary.run)
- [Dauerprozess gegen Start bei Bedarf, gemessen](jobs/job-dienst-form/summary.run)

## Software, die kein HTML ist

- [Rust-Werkzeug `taik-stats`, vom Gate übersetzt und ausgeführt](jobs/job-taik-stats/round-03.run), [Auftrag](jobs/job-taik-stats/submission.brief.md), [Gate](jobs/job-taik-stats/submission.gate.mjs) und [Auswertung mit Gate-Fehler](jobs/job-taik-stats/observation.run)

## Vorschläge der Factory

- [Reflexion nach jedem Auftrag](jobs/job-backend/round-05.run) und [erste Reflexion im Dienst, mit Halt](jobs/job-worker-verfuegbarkeit-10/reflection-worker-verfuegbarkeit-10-next.run)
- [Vorschlagsweg eingeführt](jobs/job-backend/round-04.run)
- Freigegeben und angenommen: [Cargo-Projekt aus vier Dateien](jobs/job-mehrdatei-programm/round-01.run), [vorgeschlagener Auftrag](proposals/mehrdatei-programm/brief.md) und [Gate](proposals/mehrdatei-programm/verify.mjs), abgeleitet aus [dieser Beobachtung](jobs/job-taik-stats/observation.run), mit [Auswertung](jobs/job-mehrdatei-programm/observation.run)

## Snap als Kategorisierer

- [Sechs abgelehnte Runden klassifiziert](jobs/job-snap-kategorisierer/summary.run) und [Antworten](jobs/job-snap-kategorisierer/results.json)

## Savant als Worker

- [Zwei Aufträge mit Savant statt Opus, gleiche Gates](jobs/job-savant-worker/summary.run)
- [Interpreter-Auftrag, je zwei Läufe](jobs/job-savant-worker/summary-02.run) und [Qualität jenseits des Gates](jobs/job-savant-worker/summary-03.run)

## Marke

- [Markenblatt](jobs/job-markenblatt/round-01.html), [Auftrag](jobs/job-markenblatt/submission.brief.md) und [Entscheidung](jobs/job-markenblatt/round-01.run)

## Factory-Ansicht

- [Neumorphe Ansicht mit ausgezeichneten Prüfprotokollen](jobs/job-factory-neomorph/round-01.html), [Auftrag](jobs/job-factory-neomorph/round-01.brief.md) und [Entscheidung](jobs/job-factory-neomorph/round-01.run)
- [Bildmarke in der Kopfzeile](jobs/job-factory-neomorph/round-02.run) und [Aufnahme der installierten Ansicht](jobs/job-factory-neomorph/round-02.screenshot.png)
- [Ablauf einer Runde als Stationen](jobs/job-factory-neomorph/round-04.run) und [Vorschläge mit Freigabe im Dashboard](jobs/job-factory-ansicht-2/round-01.run)
- [Aufnahme des Kandidaten](jobs/job-factory-neomorph/round-01.screenshot.png), [WebKit-Prüfung](jobs/job-factory-neomorph/round-01.webkit.log) und [vorheriger Stand](jobs/job-factory-neomorph/rollback.factory-view.html)

Die Dateien eines Auftrags liegen in seinem Ordner unter `jobs/`. Dateien mit gleichem Rundennamen gehören zusammen. `.brief.md` beschreibt den Auftrag, `.progress` hält die Schritte fest und `.run` die Entscheidung. `artifact=` in der `.run` benennt das akzeptierte Ergebnis.

`origin.txt` verweist innerhalb von `jobs/` relativ auf die Quelle. Die ursprünglichen `.run`-Protokolle behalten die damals aufgezeichneten Pfade. Früheres `evidence/01-…` bis `evidence/11-…` liegt heute im Tag-Prozedur-Auftrag. Das frühere `evidence/12-galaxy-intelligence/` ist heute der Galaxy- und Intelligence-Auftrag.
