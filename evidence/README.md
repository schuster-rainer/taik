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

- [Abgleich der Architektur mit dem Ticket-Bild des Nutzers](jobs/job-prozess-reflexion/summary-02.run)
- [Die Spec auf den eigenen Betrieb angewendet](jobs/job-prozess-reflexion/summary.run)

## Galaxy an Software-Aufträgen

- [Versuch mit und ohne wiederverwendetes Ergebnis, mit Auswertung](jobs/job-galaxy-quellcode/summary.run)
- [Nachfolgeauftrag von Grund auf gegen aufbauend auf dem angenommenen Stand](jobs/job-galaxy-quellcode/summary-02.run)
- [Wiederholung über den Dienst mit Patch-Angebot](jobs/job-galaxy-quellcode/summary-03.run)
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
- [Patch-Format und aufbauende Aufträge](jobs/job-backend/round-07.run) und [Lease mit Heartbeat statt starrer Zeitgrenze](jobs/job-backend/round-08.run)
- [Nebenläufige Aufträge](jobs/job-backend/round-09.run), [formatneutraler Worker-Auftrag und sofort besetzte Plätze](jobs/job-backend/round-10.run) und [Messung am Dienst](jobs/job-dienst-durchsatz/summary.run)
- [Freigabeschritt durch den Dienst](jobs/job-backend/round-12.run)
- [Unterhaltung im Auftrag, Rückfrage des Workers, Preisliste](jobs/job-backend/round-17.run) und [erste Antwort, die einen Auftrag wieder öffnete](jobs/job-probe-pi-grok/round-02.run)
- [Benannte Agenten, Grok und Pi als Worker](jobs/job-backend/round-13.run), [Agent beim Einreihen](jobs/job-backend/round-14.run), [abgelehnte Runde wegen eines Zeittests](jobs/job-backend/round-15.run), [Zustandsverzeichnis richtig bestimmt](jobs/job-backend/round-16.run)
- Erste Runden benannter Agenten am Dienst: [Grok](jobs/job-probe-grok-schnell/round-01.run) und [Pi](jobs/job-probe-pi-grok/round-01.run)
- [Drosselung des Gates unter launchd, gemessen](jobs/job-dienst-drosselung/summary.run)
- [Dauerprozess gegen Start bei Bedarf, gemessen](jobs/job-dienst-form/summary.run)

## Software, die kein HTML ist

- [Rust-Werkzeug `taik-stats`, vom Gate übersetzt und ausgeführt](jobs/job-taik-stats/round-03.run), [Auftrag](jobs/job-taik-stats/submission.brief.md), [Gate](jobs/job-taik-stats/submission.gate.mjs) und [Auswertung mit Gate-Fehler](jobs/job-taik-stats/observation.run)

## Aufträge aus GitHub-Issues

- [Erstes Issue, vom Vorschlag bis zum geschlossenen Ticket](jobs/job-issue-1/round-01.run), [Anfrage](issues/1.run) und [Reflexion](issues/reflection-issue-1.run)

## Vorschläge der Factory

- [Offene Vorhaben und Produkte für die Reflexion](jobs/job-backend/round-18.run) und [erste Reflexion, die ein offenes Vorhaben aufgriff](jobs/job-prozess-reflexion/reflection-naechster-schritt.run)
- [Reflexion nach jedem Auftrag](jobs/job-backend/round-05.run) und [erste Reflexion im Dienst, mit Halt](jobs/job-worker-verfuegbarkeit-10/reflection-worker-verfuegbarkeit-10-next.run)
- [Vorschlagsweg eingeführt](jobs/job-backend/round-04.run)
- Freigegeben und angenommen: [Cargo-Projekt aus vier Dateien](jobs/job-mehrdatei-programm/round-01.run), [vorgeschlagener Auftrag](proposals/mehrdatei-programm/brief.md) und [Gate](proposals/mehrdatei-programm/verify.mjs), abgeleitet aus [dieser Beobachtung](jobs/job-taik-stats/observation.run), mit [Auswertung](jobs/job-mehrdatei-programm/observation.run)

## Snap als Kategorisierer

- [Sechs abgelehnte Runden klassifiziert](jobs/job-snap-kategorisierer/summary.run) und [Antworten](jobs/job-snap-kategorisierer/results.json)

## Savant als Worker

- [Savant als Worker des Dienstes](jobs/job-backend/round-06.run) und [erster Auftrag damit](jobs/job-savant-im-dienst/round-01.run)
- [Zwei Aufträge mit Savant statt Opus, gleiche Gates](jobs/job-savant-worker/summary.run)
- [Interpreter-Auftrag, je zwei Läufe](jobs/job-savant-worker/summary-02.run) und [Qualität jenseits des Gates](jobs/job-savant-worker/summary-03.run)

## Marke

- [Markenblatt](jobs/job-markenblatt/round-01.html), [Auftrag](jobs/job-markenblatt/submission.brief.md) und [Entscheidung](jobs/job-markenblatt/round-01.run)

## Factory-Ansicht

- [Neumorphe Ansicht mit ausgezeichneten Prüfprotokollen](jobs/job-factory-neomorph/round-01.html), [Auftrag](jobs/job-factory-neomorph/round-01.brief.md) und [Entscheidung](jobs/job-factory-neomorph/round-01.run)
- [Bildmarke in der Kopfzeile](jobs/job-factory-neomorph/round-02.run) und [Aufnahme der installierten Ansicht](jobs/job-factory-neomorph/round-02.screenshot.png)
- [Unterhaltung im Ticket](jobs/job-ansicht-gespraech/round-01.run), [Freigabe](jobs/job-ansicht-gespraech/release-round-01.run) und [Aufnahme](jobs/job-ansicht-gespraech/round-01.open-ticket.png)
- [Crew: Agenten und Abos in der Ansicht, Agent je Ticket](jobs/job-ansicht-crew/round-01.run), [Freigabe](jobs/job-ansicht-crew/release-round-01.run), Aufnahmen [Standard](jobs/job-ansicht-crew/round-01.default.png), [Formular](jobs/job-ansicht-crew/round-01.agent-form.png) und [Ticket](jobs/job-ansicht-crew/round-01.ticket-agent.png)
- [Ein Ticket mit dem ganzen Auftrag, die zweite Auftragsliste entfällt](jobs/job-ansicht-ticket/round-01.run), [Freigabe](jobs/job-ansicht-ticket/release-round-01.run), Aufnahmen [Ticket](jobs/job-ansicht-ticket/round-01.ticket.png) und [Verlauf](jobs/job-ansicht-ticket/round-01.verlauf.png)
- [Ticket in der Ansicht anlegen, Vorschlag mit Beschreibung und Unterhaltung, zusammengefasste Byte-Schritte](jobs/job-ansicht-anlegen/round-01.run), [Freigabe](jobs/job-ansicht-anlegen/release-round-01.run), [Aufnahme](jobs/job-ansicht-anlegen/round-01.neues-ticket.png), Kern dazu in [Runde 19 von job-backend](jobs/job-backend/round-19.run)
- [Der ganze Bestand in der Ticketliste, mit Suche; alte Aufträge öffnen sich mit Details und Verlauf](jobs/job-ansicht-bestand/round-01.run), [Freigabe](jobs/job-ansicht-bestand/release-round-01.run), [Aufnahme](jobs/job-ansicht-bestand/round-01.bestand.png)
- [Das Ticket von oben nach unten: Stand, Änderung je Runde, geänderte Zeilen](jobs/job-ansicht-stand/round-01.run), [Freigabe](jobs/job-ansicht-stand/release-round-01.run)
- [Analyse einer Ablehnung im Ticket](jobs/job-ansicht-analyse/round-01.run), [Freigabe](jobs/job-ansicht-analyse/release-round-01.run), Kern dazu in [Runde 20 von job-backend](jobs/job-backend/round-20.run)
- [Agentensitzung: eigene Kopie, Datei-Werkzeuge, fortgesetzte Sitzung, Urteil über den Commit](jobs/job-backend/round-21.run), erster echter Lauf in [job-probe-sitzung](jobs/job-probe-sitzung/round-01.run) mit [Patch](jobs/job-probe-sitzung/round-01.diff), Ansicht dazu in [job-ansicht-sitzung](jobs/job-ansicht-sitzung/round-02.run) (erste [Freigabe](jobs/job-ansicht-sitzung/release-round-01.run) an einem Timeout der Prüfung gescheitert, zweite [angenommen](jobs/job-ansicht-sitzung/release-round-02.run))
- [Jedes wartende Ticket sagt, warum es wartet](jobs/job-ansicht-wartegrund/round-01.run), [Freigabe](jobs/job-ansicht-wartegrund/release-round-01.run), [Aufnahme](jobs/job-ansicht-wartegrund/round-01.wartet.png)
- [Orchestrator: plant jeden Wunsch (Art, Gate-Autor, Arbeit) und zeigt seinen Plan](jobs/job-backend/round-22.run), [Ersatz für einen ausgefallenen Gate-Autor](jobs/job-backend/round-23.run), Widget in [job-ansicht-orchestrator](jobs/job-ansicht-orchestrator/round-01.run) mit [Freigabe](jobs/job-ansicht-orchestrator/release-round-01.run); Runde 2 [abgelehnt](jobs/job-ansicht-orchestrator/round-02.run), weil sie das Abschneiden mit verbotenen Stilregeln beheben wollte, Runde 3 [angenommen](jobs/job-ansicht-orchestrator/round-03.run) und [freigegeben](jobs/job-ansicht-orchestrator/release-round-03.run), [Aufnahme](jobs/job-ansicht-orchestrator/round-03.orchestrator.png)
- Von der Factory selbst hervorgebracht, geplant vom Orchestrator: [Auftrag schließen](jobs/job-ticket-003/round-04.run) (Sitzung von Opus, Gate von Savant, drei Ablehnungen durch Sandbox und Suchpfad, zwei Rückfragen des Agenten; im Kern als [Runde 27](jobs/job-backend/round-27.run)) und [Ablauf eines Tickets in der Ansicht, Orchestrator zuoberst](jobs/job-ticket-004/round-02.run) (Opus; erste [Freigabe abgelehnt](jobs/job-ticket-004/release-round-01.run), zweite [vom Dienst installiert](jobs/job-ticket-004/release-round-02.run), [Aufnahme](jobs/job-ticket-004/round-02.ablauf.png)). Dabei behoben: [Gate kann bauen](jobs/job-backend/round-24.run), [mktemp](jobs/job-backend/round-25.run), [Runden zählen](jobs/job-backend/round-26.run)
- [Schließen-Knopf und geschlossene Tickets in der Ansicht](jobs/job-ticket-006/round-05.run) (Opus, Gate des Produkts; vier Runden an einem verbotenen Attributnamen gescheitert, den der Befund nicht nannte, [Analyse der Factory](jobs/job-ticket-006/round-04.analysis.md) benannte ihn, Runde 5 [vom Dienst installiert](jobs/job-ticket-006/release-round-05.run)). Das Produkt-Gate nennt den gefundenen Namen seitdem im Befund. [Wunsch wird nicht mehr gekürzt](jobs/job-backend/round-28.run).
- Weitere Arbeiten der Factory am eigenen Kern, je Wunsch, Plan, Gate, Sitzung: [Ergebnis übernehmen](jobs/job-ticket-007/round-01.run) ([Runde 29](jobs/job-backend/round-29.run)); [Orchestrator kennt Produkte, Schutzliste und Ausfälle](jobs/job-ticket-009/round-01.run) ([Runde 31](jobs/job-backend/round-31.run)), nachdem derselbe Wunsch mit blind geschriebenem Gate [dreimal scheiterte und geschlossen wurde](jobs/job-ticket-008/closed.md); [Zweig nach Annahme selbst anlegen](jobs/job-ticket-012/round-01.run) ([Runde 33](jobs/job-backend/round-33.run)). Dazu von Hand: [lesender Gate-Autor](jobs/job-backend/round-30.run), [Antwort mit Vorsatz lesen](jobs/job-backend/round-32.run).
- [Ticket-Board geordnet: eine Gruppe zur Zeit, Karten über die volle Breite](jobs/job-ansicht-tickets/round-01.run), [Freigabe](jobs/job-ansicht-tickets/release-round-01.run), Aufnahmen [Standard](jobs/job-ansicht-tickets/round-01.default.png), [Entschieden in Hell](jobs/job-ansicht-tickets/round-01.decided-light.png) und [geöffnetes Ticket](jobs/job-ansicht-tickets/round-01.open-ticket.png)
- [Erster Board-Auftrag, viermal abgelehnt](jobs/job-board-ansicht/round-04.run)
- [Änderung der Ansicht ohne Agent bis zur Installation](jobs/job-ansicht-fragment/observation.run), mit [Runde](jobs/job-ansicht-fragment/round-01.run) und [Freigabe](jobs/job-ansicht-fragment/release-round-01.run)
- [Ein Werkstück-Feld und belebter Ablauf, von der Factory gebaut](jobs/job-ansicht-werkstueck/round-02.run), [Freigabe](jobs/job-ansicht-werkstueck/release-round-02.run), [Aufnahme](jobs/job-ansicht-werkstueck/round-02.screenshot.png) und [Auswertung](jobs/job-ansicht-werkstueck/observation.run)
- [Token- und Zeitfluss je Runde, von der Factory selbst als Patch gebaut](jobs/job-ansicht-tokenfluss/round-01.run), [Aufnahme](jobs/job-ansicht-tokenfluss/round-01.screenshot.png) und [Auswertung](jobs/job-ansicht-tokenfluss/observation.run)
- [Parallele Plätze einstellbar](jobs/job-factory-ansicht-2/round-04.run), im Dienst mit [Runde 11](jobs/job-backend/round-11.run)
- [Kosten je Auftrag und Diff beim Ergebnis](jobs/job-factory-ansicht-2/round-03.run)
- [Agentenanzeige über Hooks](jobs/job-factory-ansicht-2/round-02.run) und [Aufnahme](jobs/job-factory-ansicht-2/round-02.screenshot.png)
- [Ablauf einer Runde als Stationen](jobs/job-factory-neomorph/round-04.run) und [Vorschläge mit Freigabe im Dashboard](jobs/job-factory-ansicht-2/round-01.run)
- [Aufnahme des Kandidaten](jobs/job-factory-neomorph/round-01.screenshot.png), [WebKit-Prüfung](jobs/job-factory-neomorph/round-01.webkit.log) und [vorheriger Stand](jobs/job-factory-neomorph/rollback.factory-view.html)

Die Dateien eines Auftrags liegen in seinem Ordner unter `jobs/`. Dateien mit gleichem Rundennamen gehören zusammen. `.brief.md` beschreibt den Auftrag, `.progress` hält die Schritte fest und `.run` die Entscheidung. `artifact=` in der `.run` benennt das akzeptierte Ergebnis.

`origin.txt` verweist innerhalb von `jobs/` relativ auf die Quelle. Die ursprünglichen `.run`-Protokolle behalten die damals aufgezeichneten Pfade. Früheres `evidence/01-…` bis `evidence/11-…` liegt heute im Tag-Prozedur-Auftrag. Das frühere `evidence/12-galaxy-intelligence/` ist heute der Galaxy- und Intelligence-Auftrag.
