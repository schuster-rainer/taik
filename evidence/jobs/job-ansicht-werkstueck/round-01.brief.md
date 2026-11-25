Die rechte Seite der Factory-Ansicht zu einem einzigen Werkstück-Feld umbauen und den Ablauf sichtbar lebendig machen.

Du bekommst die installierte Ansicht als angenommenen Ausgangsstand. Liefere bevorzugt einen Patch. Was dieser Auftrag nicht nennt, bleibt Zeichen für Zeichen erhalten: die Stilregeln (Schatten nur über die vorhandenen `--nm-*`-Variablen, kein Glow, kein `filter`, kein `text-shadow`), die Blöcke `RUN_HIGHLIGHT` und `TOKEN_FLOW`, der Platzhalter `/*FACTORY_DATA*/null`, die linke Spalte und die Kopfzeile.

## 1. Ein Werkstück-Feld statt zwei Feldern

Der Nutzer fand das Feld „Zum Auftrag" redundant: Es wiederholte die Arbeitsschritte des Auftragsverlaufs und zeigte nur eine zweite Sicht auf dasselbe Werkstück.

- Entferne die ganze `<section>` mit „Zum Auftrag" samt `#taik-job-content`, `#taik-job-viewport`, `#taik-job-observation`, `#taik-job-label`, den Funktionen `renderJobPreview` und `renderJobObservation` und allen CSS-Regeln dazu. Diese IDs und der Text „Zum Auftrag" dürfen nirgends mehr vorkommen.
- Die rechte Spalte hat danach zwei Zeilen: `auto` für das Feld „Ablauf" und `minmax(0, 1fr)` für das Werkstück-Feld. In der schmalen Ansicht `auto minmax(480px, 75vh)`.
- Das bisherige Feld „Ergebnis" heißt „Werkstück". Es enthält `<taik-artifact-preview id="taik-result-viewport">` (angenommener Stand), `<taik-artifact-preview id="taik-draft-viewport" draft hidden>` (Entwurf, hierher verschoben), ein neues `<div id="taik-result-source" class="diff" hidden>` für Quelltext und das vorhandene `#taik-result-diff`. Es ist immer höchstens eines der vier sichtbar.
- Im Kopf stehen drei Knöpfe `<button type="button" data-workpiece-mode="view">Ansicht</button>`, `…="source">Quelltext</button>`, `…="diff">Diff</button>` (statt `data-result-mode`). Der gewählte ist `disabled`. Alle drei sind immer vorhanden.
- Das Werkstück gehört zum gewählten Auftrag. Hat er weder Entwurf noch angenommenes Artefakt, zeigt das Feld das jüngste angenommene Dokument eines anderen Auftrags.

Die Entscheidung, was zu sehen ist, trifft eine reine Funktion `workpieceState(job, mode)` zwischen `/*WORKPIECE_BEGIN*/` und `/*WORKPIECE_END*/`. Der Block braucht nichts außerhalb von sich. Sie gibt `{ show, unverified }` zurück:

- `job` fehlt oder hat weder nichtleeres `draft` noch `preview`: `show: 'empty'`, `unverified: false`.
- Ein Text „ist ein Dokument", wenn er nach führendem Leerraum mit `<!doctype`, `<html` oder `<style` beginnt, Groß- und Kleinschreibung egal.
- Mit nichtleerem `draft` ist `unverified` `true`. Modus `view`: `'draft'`, wenn der Entwurf ein Dokument ist, sonst `'source'`. Modus `source`: `'source'`. Modus `diff`: `'diff'`, wenn es ein `preview` gibt, sonst `'source'`.
- Ohne Entwurf ist `unverified` `false`. Modus `view`: `'accepted'`, wenn `preview` ein Dokument ist, sonst `'source'`. Modus `source`: `'source'`. Modus `diff`: `'diff'`, wenn `previous` eine Zeichenkette ist, sonst `'source'`.

Darstellung je `show`: `accepted` zeigt `#taik-result-viewport`. `draft` zeigt `#taik-draft-viewport` mit dem Entwurf (ohne Skripte, wie bisher). `source` zeigt in `#taik-result-source` den Entwurf oder sonst das angenommene Artefakt über die vorhandene Funktion `highlight` in `<pre class="source-code"><code>…</code></pre>` und folgt beim Entwurf dem Textende. `diff` zeigt in `#taik-result-diff` den Zeilen-Diff: bei einem Entwurf zwischen `preview` und `draft`, sonst zwischen `previous` und `preview`. `empty` zeigt einen Satz mit Klasse `sub`.

Wichtig: `#taik-result-viewport` wird weiter nur über `resultViewport.show(revision, html)` befüllt und bei einem Entwurf nur verborgen (`hidden`), nie neu geladen. Ist `unverified` wahr, steht im Kopf des Felds deutlich „Entwurf · ungeprüft" in der Farbe `var(--bad)`, sonst wie bisher Auftrag, „freigegeben" und Artefaktname.

## 2. Ablauf mit sichtbarer Aktivität

Der Nutzer will sehen, dass gearbeitet wird. Bewegung ja, Leuchten nein.

- Die aktive Station (`.flow-node.active`) pulsiert ruhig: eine Animation, die die Station leicht ein- und ausatmet (`transform: scale(1)` bis `scale(1.03)`), dazu der vorhandene Drehring.
- Über die Verbindung zur aktiven Station wandern drei kleine Punkte von links nach rechts (Pseudoelemente oder zusätzliche `<i>`-Elemente, 4px, Farbe `var(--active)`), zeitlich versetzt.
- Jede Station zeigt während einer Runde einen laufenden Wert: beim Worker die Zeichenzahl des Entwurfs (`job.draft.length`, mit schmalem Leerzeichen als Tausendertrenner und dem Wort „Zeichen"), beim Auftrag die laufende Zeit seit `job.sourceStartedAt`.
- Wird eine Station erledigt, läuft einmal eine kurze Animation, die ihre Operationszeile von `var(--quiet)` nach `var(--active)` überblendet.
- Der Statuspunkt in der Kopfzeile blinkt, solange ein Auftrag in Arbeit ist.
- Neue Zeilen in den Arbeitsschritten links erscheinen mit einer kurzen Einblendung von unten (`opacity` und `translateY`).

Insgesamt mindestens vier verschiedene `@keyframes`. Alle Animationen enden in einem `@media (prefers-reduced-motion: reduce)`-Block mit `animation: none`.
